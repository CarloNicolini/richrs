mod cli;
mod export;
mod layout;
mod markdown;
mod pager;
mod render;
mod text;

use clap::{CommandFactory, Parser};
use cli::Args;
use std::{
    io::{self, IsTerminal, Read, Write},
    process::{Command, Stdio},
};
use text::*;

fn read_resource(path: &str) -> Result<(String, Option<String>), String> {
    if path == "-" {
        let mut data = String::new();
        io::stdin()
            .read_to_string(&mut data)
            .map_err(|e| format!("unable to read stdin: {e}"))?;
        return Ok((data, None));
    }
    if path.starts_with("https://") || path.starts_with("http://") {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .build();
        let agent: ureq::Agent = config.into();
        let mut response = agent
            .get(path)
            .call()
            .map_err(|e| format!("unable to fetch {path}: {e}"))?;
        let mime = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.split(';').next().unwrap_or("").to_string());
        let body = response
            .body_mut()
            .with_config()
            .limit(64 * 1024 * 1024)
            .read_to_string()
            .map_err(|e| format!("unable to read response: {e}"))?;
        return Ok((body, mime));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("unable to read {path}: {e}"))?;
    Ok((String::from_utf8_lossy(&bytes).into_owned(), None))
}
fn run(args: Args) -> Result<(), String> {
    if let Some(shell) = args.generate_completions {
        clap_complete::generate(shell, &mut Args::command(), "rich", &mut io::stdout());
        return Ok(());
    }
    if args.resource.is_empty() && !args.print && !args.rule {
        Args::parse_from(["rich", "--help"]);
        return Ok(());
    }
    if args.head == Some(0) || args.tail == Some(0) {
        return Err("head and tail must be at least 1".into());
    }
    // Preserve upstream's precedence when more than one format flag is passed.
    let selected = if args.print {
        "print"
    } else if args.syntax {
        "syntax"
    } else if args.json {
        "json"
    } else if args.markdown {
        "md"
    } else if args.rule {
        "rule"
    } else if args.inspect {
        "inspect"
    } else if args.csv {
        "csv"
    } else if args.rst {
        "rst"
    } else if args.ipynb {
        "ipynb"
    } else {
        "auto"
    };
    if selected == "inspect" {
        return Err("--inspect evaluates Python objects in upstream rich-cli and cannot run in this standalone Rust binary".into());
    }
    let padding = args.padding()?;
    let global_style = Style::parse(&args.style)?;
    let panel_style = Style::parse(&args.panel_style)?;
    let terminal_width = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .or_else(|| {
            std::env::var("COLUMNS")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
        })
        .unwrap_or(80)
        .clamp(1, 65535);
    let outer = args
        .width
        .map(usize::from)
        .unwrap_or(terminal_width)
        .min(args.max_width.map(usize::from).unwrap_or(usize::MAX));
    let inner = outer
        .saturating_sub(padding[1] + padding[3] + if args.panel != "none" { 4 } else { 0 })
        .max(1);
    let path = args
        .resource
        .split(['?', '#'])
        .next()
        .unwrap_or(&args.resource);
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let auto_syntax = selected == "auto"
        && !matches!(
            extension.as_str(),
            "md" | "json" | "csv" | "tsv" | "rst" | "ipynb"
        );
    let simple_source = matches!(selected, "syntax") || auto_syntax;
    let can_stream = simple_source
        && !args.resource.starts_with("http://")
        && !args.resource.starts_with("https://")
        && !args.line_numbers
        && args.style.is_empty()
        && args.panel == "none"
        && padding == [0; 4]
        && args.export_html.is_none()
        && args.export_svg.is_none()
        && !args.pager
        && !args.left
        && !args.right
        && !args.center;
    if can_stream {
        let color = args.force_terminal
            || (io::stdout().is_terminal()
                && std::env::var_os("NO_COLOR").is_none_or(|s| s.is_empty())
                && !std::env::var("TERM").is_ok_and(|s| s == "dumb"));
        let stdout = io::stdout();
        let mut writer = io::BufWriter::new(stdout.lock());
        let result = if args.resource == "-" {
            render::syntax_stream(
                io::BufReader::new(io::stdin().lock()),
                &mut writer,
                "",
                args.lexer.as_deref(),
                &args,
                inner,
                color,
            )
        } else {
            let file = std::fs::File::open(&args.resource)
                .map_err(|e| format!("unable to read {}: {e}", args.resource))?;
            render::syntax_stream(
                io::BufReader::new(file),
                &mut writer,
                path,
                args.lexer.as_deref(),
                &args,
                inner,
                color,
            )
        };
        if let Err(error) = result {
            if error.contains("Broken pipe") {
                return Ok(());
            }
            return Err(error);
        }
        return Ok(());
    }
    let mut doc = if matches!(selected, "print" | "rule") {
        let resource = if args.resource == "-" {
            read_resource("-")?.0
        } else {
            args.resource.clone()
        };
        let parsed = markup(&resource, args.emoji)?;
        if args.rule {
            let label = parsed.into_iter().flatten().collect();
            vec![layout::rule(
                &label,
                inner,
                &args.rule_char,
                &Style::parse(&args.rule_style)?,
                if args.text_left {
                    "left"
                } else if args.text_right {
                    "right"
                } else {
                    "center"
                },
            )?]
        } else {
            let mode = if args.text_left {
                "left"
            } else if args.text_right {
                "right"
            } else if args.text_center {
                "center"
            } else if args.text_full {
                "full"
            } else {
                "left"
            };
            let mut doc = vec![];
            for line in parsed {
                let wrapped = if args.soft {
                    vec![line]
                } else if args.no_wrap {
                    vec![truncate(&line, inner)]
                } else {
                    wrap(&line, inner, true)
                };
                let len = wrapped.len();
                for (i, line) in wrapped.into_iter().enumerate() {
                    doc.push(layout::justify(
                        &line,
                        inner,
                        if mode == "full" && i + 1 == len {
                            "left"
                        } else {
                            mode
                        },
                    ));
                }
            }
            doc
        }
    } else {
        let (data, mime) = read_resource(&args.resource)?;
        let format = if selected == "auto" {
            extension.as_str()
        } else {
            selected
        };
        match format {
            "json" => render::json(&data, &args, inner)?,
            "md" => markdown::render(&data, &args, inner)?,
            "csv" | "tsv" => render::csv(&data, path, &args, inner)?,
            "rst" => markdown::rst(&data, &args, inner)?,
            "ipynb" => render::notebook(&data, &args, inner)?,
            _ => {
                let lexer = args.lexer.as_deref().or(match mime.as_deref() {
                    Some("application/json") => Some("json"),
                    Some("text/html") => Some("html"),
                    Some("application/javascript") => Some("javascript"),
                    _ => None,
                });
                render::syntax(&data, path, lexer, &args, inner)?
            }
        }
    };
    if padding != [0; 4] {
        doc = layout::pad(
            doc,
            padding,
            args.expand || args.width.is_some(),
            outer.saturating_sub(if args.panel != "none" { 4 } else { 0 }),
        );
    }
    if args.panel != "none" {
        let title = markup(&args.title, args.emoji)?
            .into_iter()
            .flatten()
            .collect();
        let caption = markup(&args.caption, args.emoji)?
            .into_iter()
            .flatten()
            .collect();
        doc = layout::panel(
            doc,
            &args.panel,
            outer,
            args.expand || args.width.is_some(),
            &title,
            &caption,
            &panel_style,
        );
    }
    if !args.left && (args.center || args.right) {
        let natural = doc.iter().map(width).max().unwrap_or(0);
        let block = args.width.map(usize::from).unwrap_or(natural);
        let offset =
            terminal_width.saturating_sub(block) / if args.center && !args.right { 2 } else { 1 };
        for line in &mut doc {
            let mut row = spaces(offset);
            row.append(line);
            *line = row;
        }
    }
    for line in &mut doc {
        for cell in line {
            cell.style = global_style.overlay(&cell.style);
        }
    }
    if let Some(path) = &args.export_html {
        std::fs::write(path, export::html(&doc))
            .map_err(|e| format!("unable to export HTML: {e}"))?;
    }
    if let Some(path) = &args.export_svg {
        std::fs::write(path, export::svg(&doc))
            .map_err(|e| format!("unable to export SVG: {e}"))?;
    }
    let is_terminal = io::stdout().is_terminal();
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|s| !s.is_empty());
    let dumb = std::env::var("TERM").is_ok_and(|s| s == "dumb");
    let color = args.force_terminal || (is_terminal && !no_color && !dumb);
    let output = ansi_string(&doc, color, args.hyperlinks);
    if args.pager && is_terminal && std::env::var_os("PAGER").is_none() {
        pager::show(&doc, &args.resource, color, args.hyperlinks)
            .map_err(|e| format!("pager failed: {e}"))?;
    } else if args.pager && is_terminal {
        let pager = std::env::var("PAGER").map_err(|_| "PAGER is not valid UTF-8")?;
        let words = shlex::split(&pager)
            .filter(|w| !w.is_empty())
            .ok_or("invalid PAGER command")?;
        let mut child = Command::new(&words[0])
            .args(&words[1..])
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("unable to start pager '{}': {e}", words[0]))?;
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(output.as_bytes()) {
                if e.kind() != io::ErrorKind::BrokenPipe {
                    return Err(e.to_string());
                }
            }
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("pager exited with {status}"));
        }
    } else if let Err(e) = io::stdout().lock().write_all(output.as_bytes()) {
        if e.kind() != io::ErrorKind::BrokenPipe {
            return Err(e.to_string());
        }
    }
    Ok(())
}
fn main() {
    if let Err(error) = run(Args::parse()) {
        let _ = writeln!(io::stderr().lock(), "rich: {error}");
        std::process::exit(1);
    }
}

use crate::{cli::Args, layout, text::*};
use std::{
    collections::VecDeque,
    io::{BufRead, Write},
    sync::LazyLock,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, ThemeSet},
    parsing::SyntaxSet,
};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(|| {
    let mut themes = ThemeSet::load_defaults();
    for (name, data) in [
        ("monokai", include_str!("../assets/themes/monokai.tmTheme")),
        ("dracula", include_str!("../assets/themes/dracula.tmTheme")),
        ("default", include_str!("../assets/themes/default.tmTheme")),
        (
            "friendly",
            include_str!("../assets/themes/friendly.tmTheme"),
        ),
    ] {
        themes.themes.insert(
            name.into(),
            ThemeSet::load_from_reader(&mut std::io::Cursor::new(data))
                .expect("bundled syntax theme"),
        );
    }
    themes
});

/// Render source with memory bounded by the current line (or the requested tail).
/// This is used for plain files and stdin when post-processing does not require
/// retaining a full render tree.
pub fn syntax_stream<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
    path: &str,
    lexer: Option<&str>,
    args: &Args,
    limit: usize,
    color: bool,
) -> Result<(), String> {
    let name = lexer.map(|x| match x {
        "py" => "python",
        "js" => "javascript",
        "sh" | "bash" => "shell-unix-generic",
        "text" | "txt" => "Plain Text",
        _ => x,
    });
    let syntax = if let Some(name) = name {
        SYNTAXES
            .find_syntax_by_token(name)
            .or_else(|| {
                SYNTAXES
                    .syntaxes()
                    .iter()
                    .find(|s| s.name.eq_ignore_ascii_case(name))
            })
            .ok_or_else(|| format!("unknown lexer: {name}"))?
    } else {
        let path = path.split(['?', '#']).next().unwrap_or(path);
        std::path::Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .and_then(|s| SYNTAXES.find_syntax_by_extension(s))
            .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text())
    };
    let theme_name = match args.theme.as_str() {
        "ansi_dark" => "base16-ocean.dark",
        "ansi_light" => "InspiredGitHub",
        "solarized-dark" => "Solarized (dark)",
        "solarized-light" => "Solarized (light)",
        other => other,
    };
    let theme = THEMES
        .themes
        .get(theme_name)
        .ok_or_else(|| format!("unknown syntax theme: {}", args.theme))?;
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut bytes = Vec::with_capacity(8192);
    let mut line_number = 0usize;
    let mut tail: VecDeque<Vec<Line>> = VecDeque::new();
    let mut wrote = false;
    let mut trailing_newline = false;
    loop {
        bytes.clear();
        let count = reader
            .read_until(b'\n', &mut bytes)
            .map_err(|e| format!("unable to read source: {e}"))?;
        if count == 0 {
            break;
        }
        line_number += 1;
        trailing_newline = bytes.last() == Some(&b'\n');
        let raw = String::from_utf8_lossy(&bytes);
        let ranges = highlighter
            .highlight_line(&raw, &SYNTAXES)
            .map_err(|e| e.to_string())?;
        let mut cells = Vec::with_capacity(raw.len());
        for (style, text) in ranges {
            let style = Style {
                fg: Some((style.foreground.r, style.foreground.g, style.foreground.b)),
                bg: if args.theme.starts_with("ansi_") {
                    None
                } else {
                    Some((style.background.r, style.background.g, style.background.b))
                },
                bold: style.font_style.contains(FontStyle::BOLD),
                italic: style.font_style.contains(FontStyle::ITALIC),
                underline: style.font_style.contains(FontStyle::UNDERLINE),
                ..Style::default()
            };
            for ch in text.trim_end_matches(['\n', '\r']).chars() {
                if ch == '\t' {
                    cells.extend(styled(&" ".repeat(4 - width(&cells) % 4), &style));
                } else if !ch.is_control() {
                    cells.push(Cell {
                        ch,
                        style: style.clone(),
                    });
                }
            }
        }
        if args.guides {
            let leading = cells.iter().take_while(|c| c.ch == ' ').count();
            for n in (0..leading).step_by(4) {
                cells[n].ch = '│';
                cells[n].style = Style::parse("dim").unwrap();
            }
        }
        let mut rendered = vec![];
        if args.soft || args.no_wrap {
            rendered.push(if args.no_wrap {
                truncate(&cells, limit)
            } else {
                cells
            });
        } else {
            rendered = wrap(&cells, limit, false);
        }
        if let Some(count) = args.head {
            if line_number > count {
                break;
            }
            for line in rendered {
                write_line(&mut writer, line, color)?;
                wrote = true;
            }
        } else if let Some(count) = args.tail {
            tail.push_back(rendered);
            while tail.len() > count {
                tail.pop_front();
            }
        } else {
            for line in rendered {
                write_line(&mut writer, line, color)?;
                wrote = true;
            }
        }
    }
    if args.tail.is_none() && trailing_newline && args.head.is_none_or(|n| n > line_number) {
        write_line(&mut writer, vec![], color)?;
        wrote = true;
    }
    if args.tail.is_some() {
        for source_line in tail {
            for line in source_line {
                write_line(&mut writer, line, color)?;
                wrote = true;
            }
        }
    }
    if !wrote {
        writer.write_all(b"\n").map_err(|e| e.to_string())?;
    }
    writer.flush().map_err(|e| e.to_string())
}
fn write_line<W: Write>(writer: &mut W, line: Line, color: bool) -> Result<(), String> {
    writer
        .write_all(ansi_string(&vec![line], color, false).as_bytes())
        .map_err(|e| e.to_string())
}

pub fn syntax(
    data: &str,
    path: &str,
    lexer: Option<&str>,
    args: &Args,
    limit: usize,
) -> Result<Document, String> {
    let name = lexer.map(|x| match x {
        "py" => "python",
        "js" => "javascript",
        "sh" | "bash" => "shell-unix-generic",
        "text" | "txt" => "Plain Text",
        _ => x,
    });
    let syntax = if let Some(name) = name {
        SYNTAXES
            .find_syntax_by_token(name)
            .or_else(|| {
                SYNTAXES
                    .syntaxes()
                    .iter()
                    .find(|s| s.name.eq_ignore_ascii_case(name))
            })
            .ok_or_else(|| format!("unknown lexer: {name}"))?
    } else {
        let path = path.split(['?', '#']).next().unwrap_or(path);
        std::path::Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .and_then(|s| SYNTAXES.find_syntax_by_extension(s))
            .or_else(|| SYNTAXES.find_syntax_by_first_line(data.lines().next().unwrap_or("")))
            .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text())
    };
    let theme = match args.theme.as_str() {
        "ansi_dark" => "base16-ocean.dark",
        "ansi_light" => "InspiredGitHub",
        "solarized-dark" => "Solarized (dark)",
        "solarized-light" => "Solarized (light)",
        other => other,
    };
    let theme=THEMES.themes.get(theme).ok_or_else(||format!("unknown syntax theme '{}'; supported: ansi_dark, ansi_light, solarized-dark, solarized-light, {}",args.theme,THEMES.themes.keys().cloned().collect::<Vec<_>>().join(", ")))?;
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines: Vec<_> = data.split_inclusive('\n').collect();
    let actual_lines = lines.len();
    if data.ends_with('\n') && args.tail.is_none() {
        lines.push("");
    }
    let total = lines.len();
    let (start, end) = line_range(
        if args.tail.is_some() {
            actual_lines
        } else {
            total
        },
        args.head,
        args.tail,
    );
    let digits = total.max(1).to_string().len();
    let gutter = if args.line_numbers { digits + 3 } else { 0 };
    let mut result = vec![];
    for (i, line) in lines.iter().enumerate() {
        // Parse skipped lines too, so multiline comments/strings retain their state.
        let ranges = highlighter
            .highlight_line(line, &SYNTAXES)
            .map_err(|e| e.to_string())?;
        if i < start || i >= end {
            continue;
        }
        let mut cells = vec![];
        for (s, text) in ranges {
            let style = Style {
                fg: Some((s.foreground.r, s.foreground.g, s.foreground.b)),
                bg: if args.theme.starts_with("ansi_") {
                    None
                } else {
                    Some((s.background.r, s.background.g, s.background.b))
                },
                bold: s.font_style.contains(FontStyle::BOLD),
                italic: s.font_style.contains(FontStyle::ITALIC),
                underline: s.font_style.contains(FontStyle::UNDERLINE),
                ..Style::default()
            };
            for ch in text.trim_end_matches(['\n', '\r']).chars() {
                if ch == '\t' {
                    cells.extend(styled(&" ".repeat(4 - width(&cells) % 4), &style));
                } else if !ch.is_control() {
                    cells.push(Cell {
                        ch,
                        style: style.clone(),
                    });
                }
            }
        }
        if args.guides {
            let leading = cells.iter().take_while(|c| c.ch == ' ').count();
            for n in (0..leading).step_by(4) {
                cells[n].ch = '│';
                cells[n].style = Style::parse("dim").unwrap();
            }
        }
        let available = limit.saturating_sub(gutter).max(1);
        let wrapped = if args.no_wrap {
            vec![truncate(&cells, available)]
        } else {
            wrap(&cells, available, false)
        };
        for (part, line) in wrapped.into_iter().enumerate() {
            let mut row = if args.line_numbers {
                styled(
                    &format!(
                        "  {:>digits$} ",
                        if part == 0 {
                            (i + 1).to_string()
                        } else {
                            String::new()
                        }
                    ),
                    &Style::parse("dim").unwrap(),
                )
            } else {
                vec![]
            };
            row.extend(line);
            if !args.theme.starts_with("ansi_") {
                let background = theme.settings.background.map(|s| (s.r, s.g, s.b));
                let padding = styled(
                    &" ".repeat(limit.saturating_sub(width(&row))),
                    &Style {
                        bg: background,
                        ..Style::default()
                    },
                );
                row.extend(padding);
            }
            result.push(row);
        }
    }
    if result.is_empty() {
        result.push(vec![]);
    }
    Ok(result)
}
pub fn line_range(total: usize, head: Option<usize>, tail: Option<usize>) -> (usize, usize) {
    if let Some(n) = head {
        (0, n.min(total))
    } else if let Some(n) = tail {
        (total.saturating_sub(n), total)
    } else {
        (0, total)
    }
}
pub fn json(data: &str, args: &Args, limit: usize) -> Result<Document, String> {
    let value: serde_json::Value =
        serde_json::from_str(data).map_err(|e| format!("invalid JSON: {e}"))?;
    syntax(
        &serde_json::to_string_pretty(&value).unwrap(),
        "data.json",
        Some("json"),
        args,
        limit,
    )
}
pub fn csv(data: &str, path: &str, args: &Args, limit: usize) -> Result<Document, String> {
    let delimiter = if path.ends_with(".tsv") {
        b'\t'
    } else {
        // Score parsed records rather than raw delimiter counts, so quoted
        // commas inside semicolon- or tab-separated fields do not confuse detection.
        b",\t;|"
            .iter()
            .copied()
            .enumerate()
            .max_by_key(|(preference, delimiter)| {
                let mut reader = csv::ReaderBuilder::new()
                    .delimiter(*delimiter)
                    .has_headers(false)
                    .flexible(true)
                    .from_reader(data.as_bytes());
                let counts: Vec<usize> = reader
                    .records()
                    .take(20)
                    .filter_map(Result::ok)
                    .map(|r| r.len())
                    .collect();
                let mut frequencies = std::collections::HashMap::<usize, usize>::new();
                for count in &counts {
                    *frequencies.entry(*count).or_default() += 1;
                }
                let (columns, consistent) = frequencies
                    .into_iter()
                    .filter(|(cols, _)| *cols > 1)
                    .max_by_key(|(_, n)| *n)
                    .unwrap_or((0, 0));
                (
                    consistent * 100 / counts.len().max(1),
                    columns,
                    4 - preference,
                )
            })
            .map(|(_, delimiter)| delimiter)
            .unwrap_or(b',')
    };
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(data.as_bytes());
    let records = reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("invalid CSV: {e}"))?;
    if records.is_empty() {
        return Ok(vec![]);
    }
    let header = has_header(&records);
    let offset = usize::from(header);
    let total = records.len() - offset;
    let (start, end) = line_range(total, args.head, args.tail);
    let mut rows = vec![];
    if header {
        rows.push(records[0].iter().map(plain).collect());
    }
    for record in &records[offset + start..offset + end] {
        rows.push(record.iter().map(plain).collect());
    }
    let mut out = vec![];
    if !args.title.is_empty() {
        out.extend(markup(&args.title, args.emoji)?);
    }
    out.extend(layout::table(&rows, header, limit));
    if !args.caption.is_empty() {
        out.extend(markup(&args.caption, args.emoji)?);
    }
    Ok(out)
}
// Vote on stable numeric or string-length columns, as csv.Sniffer does.
fn has_header(records: &[csv::StringRecord]) -> bool {
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        Numeric,
        Length(usize),
        Unknown,
        Mixed,
    }
    fn kind(s: &str) -> Kind {
        if s.trim().parse::<f64>().is_ok() {
            Kind::Numeric
        } else {
            Kind::Length(s.chars().count())
        }
    }
    let first = &records[0];
    let mut kinds = vec![Kind::Unknown; first.len()];
    for row in records
        .iter()
        .skip(1)
        .take(21)
        .filter(|r| r.len() == first.len())
    {
        for (i, value) in row.iter().enumerate() {
            let next = kind(value);
            kinds[i] = match kinds[i] {
                Kind::Unknown => next,
                old if old == next => old,
                _ => Kind::Mixed,
            };
        }
    }
    let score: i32 = kinds
        .iter()
        .enumerate()
        .map(|(i, k)| match k {
            Kind::Unknown => 1,
            Kind::Mixed => 0,
            Kind::Numeric => {
                if kind(&first[i]) == Kind::Numeric {
                    -1
                } else {
                    1
                }
            }
            Kind::Length(n) => {
                if first[i].chars().count() == *n {
                    -1
                } else {
                    1
                }
            }
        })
        .sum();
    score > 0
}

pub fn notebook(data: &str, args: &Args, limit: usize) -> Result<Document, String> {
    let nb: serde_json::Value =
        serde_json::from_str(data).map_err(|e| format!("invalid notebook JSON: {e}"))?;
    let cells = nb["cells"]
        .as_array()
        .ok_or("notebook must have a cells array")?;
    let lang = args
        .lexer
        .as_deref()
        .or(nb
            .pointer("/metadata/kernelspec/language")
            .and_then(|v| v.as_str()))
        .unwrap_or("python");
    let source = |value: &serde_json::Value| -> String {
        if let Some(s) = value.as_str() {
            s.into()
        } else {
            value
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<String>())
                .unwrap_or_default()
        }
    };
    let mut out = vec![];
    for cell in cells {
        out.push(vec![]);
        let text = source(&cell["source"]);
        match cell["cell_type"].as_str() {
            Some("code") => {
                let count = cell["execution_count"]
                    .as_u64()
                    .map(|n| n.to_string())
                    .unwrap_or(" ".into());
                out.push(styled(
                    &format!("In [{count}]:"),
                    &Style::parse("green").unwrap(),
                ));
                let code = syntax(&text, "", Some(lang), args, limit.saturating_sub(4).max(1))?;
                out.extend(layout::panel(
                    code,
                    "rounded",
                    limit,
                    true,
                    &vec![],
                    &vec![],
                    &Style::parse("dim").unwrap(),
                ));
            }
            Some("markdown") => out.extend(crate::markdown::render(&text, args, limit)?),
            Some("raw") => out.extend(text.lines().map(plain)),
            other => return Err(format!("invalid notebook cell type: {other:?}")),
        }
        if let Some(outputs) = cell["outputs"].as_array() {
            for output in outputs {
                match output["output_type"].as_str() {
                    Some("stream") => out.extend(
                        source(&output["text"])
                            .lines()
                            .map(|l| plain(&strip_ansi(l))),
                    ),
                    Some("error") => {
                        if let Some(trace) = output["traceback"].as_array() {
                            for line in trace {
                                out.extend(source(line).lines().map(|l| plain(&strip_ansi(l))));
                            }
                        }
                    }
                    Some("execute_result") | Some("display_data") => {
                        if output["output_type"] == "execute_result" {
                            out.push(styled(
                                &format!(
                                    "Out[{}]:",
                                    output["execution_count"]
                                        .as_u64()
                                        .map(|n| n.to_string())
                                        .unwrap_or(" ".into())
                                ),
                                &Style::parse("red").unwrap(),
                            ));
                        }
                        if let Some(md) = output["data"].get("text/markdown") {
                            out.extend(crate::markdown::render(&source(md), args, limit)?);
                        } else {
                            out.extend(
                                source(&output["data"]["text/plain"])
                                    .lines()
                                    .map(|l| plain(&strip_ansi(l))),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(out)
}
fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            match chars.next() {
                Some('[') => {
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(c) = chars.next() {
                        if c == '\x07' {
                            break;
                        }
                        if c == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else if !ch.is_control() {
            out.push(ch);
        }
    }
    out
}

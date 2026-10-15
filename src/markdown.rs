use crate::{cli::Args, layout, render, text::*};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

pub fn render(data: &str, args: &Args, limit: usize) -> Result<Document, String> {
    let mut doc = vec![];
    let mut line: Line = vec![];
    let mut styles = vec![Style::default()];
    let mut lists: Vec<Option<u64>> = vec![];
    let mut prefix = String::new();
    let mut quote = 0usize;
    let mut code: Option<(String, String)> = None;
    let mut heading = None;
    let mut links: Vec<String> = vec![];
    let mut table_rows: Option<Vec<Vec<Line>>> = None;
    let mut row: Vec<Line> = vec![];
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    for event in Parser::new_ext(data, options) {
        if let Some((_, text)) = &mut code {
            match &event {
                Event::End(TagEnd::CodeBlock) => {}
                Event::Text(s) => {
                    text.push_str(s);
                    continue;
                }
                _ => continue,
            }
        }
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Heading { level, .. } => {
                    flush(&mut doc, &mut line, limit, &prefix, quote);
                    heading = Some(level as usize);
                    styles.push(Style::parse("bold").unwrap());
                }
                Tag::Emphasis => styles.push(
                    styles
                        .last()
                        .unwrap()
                        .overlay(&Style::parse("italic").unwrap()),
                ),
                Tag::Strong => styles.push(
                    styles
                        .last()
                        .unwrap()
                        .overlay(&Style::parse("bold").unwrap()),
                ),
                Tag::Strikethrough => styles.push(
                    styles
                        .last()
                        .unwrap()
                        .overlay(&Style::parse("strike").unwrap()),
                ),
                Tag::BlockQuote(_) => {
                    flush(&mut doc, &mut line, limit, &prefix, quote);
                    quote += 1;
                }
                Tag::List(start) => {
                    flush(&mut doc, &mut line, limit, &prefix, quote);
                    lists.push(start);
                }
                Tag::Item => {
                    flush(&mut doc, &mut line, limit, &prefix, quote);
                    prefix = " ".to_string() + &"  ".repeat(lists.len().saturating_sub(1));
                    if let Some(Some(n)) = lists.last_mut() {
                        prefix += &format!("{n}. ");
                        *n += 1;
                    } else {
                        prefix += "• ";
                    }
                }
                Tag::CodeBlock(kind) => {
                    flush(&mut doc, &mut line, limit, &prefix, quote);
                    let lang = match kind {
                        CodeBlockKind::Fenced(s) => {
                            s.split_whitespace().next().unwrap_or("text").to_string()
                        }
                        _ => "text".into(),
                    };
                    code = Some((lang, String::new()));
                }
                Tag::Link { dest_url, .. } => {
                    links.push(dest_url.to_string());
                    let mut s = Style::parse("underline cyan").unwrap();
                    if args.hyperlinks {
                        s.link = Some(dest_url.to_string());
                    }
                    styles.push(styles.last().unwrap().overlay(&s));
                }
                Tag::Image { dest_url, .. } => {
                    line.extend(styled("🖼 ", styles.last().unwrap()));
                    links.push(dest_url.to_string());
                }
                Tag::Table(_) => {
                    flush(&mut doc, &mut line, limit, &prefix, quote);
                    table_rows = Some(vec![]);
                }
                Tag::TableHead | Tag::TableRow => {
                    row.clear();
                }
                Tag::TableCell => {
                    line.clear();
                }
                Tag::FootnoteDefinition(label) => {
                    flush(&mut doc, &mut line, limit, &prefix, quote);
                    line.extend(plain(&format!("[{label}] ")));
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => {
                    flush(&mut doc, &mut line, limit, &prefix, quote);
                    if lists.is_empty() {
                        blank(&mut doc);
                    }
                }
                TagEnd::Heading(_) => {
                    styles.pop();
                    let level = heading.take().unwrap_or(2);
                    if level == 1 {
                        doc.extend(layout::panel(
                            vec![layout::align(&line, limit.saturating_sub(4), "center")],
                            "double",
                            limit,
                            true,
                            &vec![],
                            &vec![],
                            &Style::default(),
                        ));
                        line.clear();
                    } else {
                        let centered = layout::align(&line, limit, "center");
                        line.clear();
                        doc.push(centered);
                    }
                    blank(&mut doc);
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                    styles.pop();
                }
                TagEnd::BlockQuote(_) => {
                    flush(&mut doc, &mut line, limit, &prefix, quote);
                    quote = quote.saturating_sub(1);
                    blank(&mut doc);
                }
                TagEnd::Item => {
                    flush(&mut doc, &mut line, limit, &prefix, quote);
                    prefix.clear();
                }
                TagEnd::List(_) => {
                    lists.pop();
                    prefix.clear();
                    if lists.is_empty() {
                        blank(&mut doc);
                    }
                }
                TagEnd::CodeBlock => {
                    let (lang, text) = code.take().unwrap();
                    let rendered = render::syntax(
                        &text,
                        "",
                        Some(&lang),
                        args,
                        limit.saturating_sub(2).max(1),
                    )?;
                    for mut row in rendered {
                        let mut l = plain(" ");
                        l.append(&mut row);
                        doc.push(l);
                    }
                    blank(&mut doc);
                }
                TagEnd::Link => {
                    styles.pop();
                    if let Some(url) = links.pop()
                        && !args.hyperlinks
                    {
                        line.extend(plain(&format!(" ({url})")));
                    }
                }
                TagEnd::Image => {
                    if let Some(url) = links.pop() {
                        line.extend(plain(&format!(" ({url})")));
                    }
                }
                TagEnd::TableCell => {
                    row.push(std::mem::take(&mut line));
                }
                TagEnd::TableHead | TagEnd::TableRow => {
                    if let Some(rows) = &mut table_rows {
                        rows.push(std::mem::take(&mut row));
                    }
                }
                TagEnd::Table => {
                    doc.extend(layout::table(
                        &table_rows.take().unwrap_or_default(),
                        true,
                        limit,
                    ));
                    blank(&mut doc);
                }
                _ => {}
            },
            Event::Text(text) => line.extend(styled(&text, styles.last().unwrap())),
            Event::Code(text) => line.extend(styled(
                &text,
                &styles
                    .last()
                    .unwrap()
                    .overlay(&Style::parse("bold cyan").unwrap()),
            )),
            Event::SoftBreak => line.extend(plain(" ")),
            Event::HardBreak => flush(&mut doc, &mut line, limit, &prefix, quote),
            Event::Rule => {
                flush(&mut doc, &mut line, limit, &prefix, quote);
                doc.push(layout::rule(
                    &vec![],
                    limit,
                    "─",
                    &Style::default(),
                    "center",
                )?);
                blank(&mut doc);
            }
            Event::TaskListMarker(checked) => {
                line.extend(plain(if checked { "☑ " } else { "☐ " }))
            }
            Event::FootnoteReference(label) => line.extend(plain(&format!("[{label}]"))),
            // HTML is displayed literally; it is never executed in terminal or exports.
            Event::Html(text) | Event::InlineHtml(text) => line.extend(plain(&text)),
            _ => {}
        }
    }
    flush(&mut doc, &mut line, limit, &prefix, quote);
    while doc.last().is_some_and(Vec::is_empty) {
        doc.pop();
    }
    Ok(doc)
}
fn blank(doc: &mut Document) {
    if doc.last().is_some_and(|l| !l.is_empty()) {
        doc.push(vec![]);
    }
}
fn flush(doc: &mut Document, line: &mut Line, limit: usize, prefix: &str, quote: usize) {
    if line.is_empty() {
        return;
    }
    let lead = format!("{}{}", "│ ".repeat(quote), prefix);
    let indent = width(&plain(&lead));
    for (i, l) in wrap(line, limit.saturating_sub(indent).max(1), true)
        .into_iter()
        .enumerate()
    {
        let mut row = if i == 0 {
            plain(&lead)
        } else {
            let mut p = plain(&"│ ".repeat(quote));
            p.extend(spaces(width(&plain(prefix))));
            p
        };
        row.extend(l);
        row.extend(spaces(limit.saturating_sub(width(&row))));
        doc.push(row);
    }
    line.clear();
}

/// Translate common reStructuredText constructs into the Markdown renderer.
/// Arbitrary docutils directives are diagnosed; roles are retained as text.
pub fn rst(data: &str, args: &Args, limit: usize) -> Result<Document, String> {
    let lines: Vec<_> = data.lines().collect();
    let mut md = String::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(directive) = line.strip_prefix(".. ") {
            if let Some(lang) = directive
                .strip_prefix("code-block::")
                .or_else(|| directive.strip_prefix("code::"))
            {
                md += &format!("\n```{}\n", lang.trim());
                i += 1;
                while i < lines.len() && lines[i].trim().is_empty() {
                    i += 1;
                }
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].trim().is_empty())
                {
                    md += lines[i].strip_prefix("   ").unwrap_or(lines[i]);
                    md.push('\n');
                    i += 1;
                }
                md += "```\n";
                continue;
            } else if let Some(body) = directive
                .strip_prefix("note::")
                .or_else(|| directive.strip_prefix("warning::"))
            {
                md += &format!("> {body}\n");
                i += 1;
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].trim().is_empty())
                {
                    md += &format!("> {}\n", lines[i].trim());
                    i += 1;
                }
                continue;
            } else if directive.contains("::") {
                return Err(format!(
                    "unsupported reStructuredText directive: {directive}"
                ));
            } else {
                i += 1;
                continue;
            }
        }
        if i + 1 < lines.len() && !line.trim().is_empty() {
            let underline = lines[i + 1].trim();
            let marker = underline.chars().next().unwrap_or(' ');
            if ['=', '-', '~', '^', '"'].contains(&marker)
                && underline.chars().all(|c| c == marker)
                && underline.len() >= line.len()
            {
                md += &format!("{} {line}\n\n", if marker == '=' { "#" } else { "##" });
                i += 2;
                continue;
            }
        }
        if line.ends_with("::") {
            md += line.trim_end_matches(':');
            md += "\n\n```text\n";
            i += 1;
            while i < lines.len() && lines[i].is_empty() {
                i += 1;
            }
            while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                md += lines[i].strip_prefix("   ").unwrap_or(lines[i]);
                md.push('\n');
                i += 1;
            }
            md += "```\n";
            continue;
        }
        // RST inline literals use double backticks, Markdown uses single backticks.
        md += &line.replace("``", "`");
        md.push('\n');
        i += 1;
    }
    let rendered = render(&md, args, limit)?;
    let mut result: Document = vec![];
    for mut line in rendered {
        if line.is_empty()
            && result
                .last()
                .and_then(|l| l.last())
                .is_some_and(|c| c.ch == '╝')
        {
            continue;
        }
        while line.last().is_some_and(|c| c.ch == ' ') {
            line.pop();
        }
        result.push(line);
    }
    Ok(result)
}

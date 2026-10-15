use crate::text::*;

fn runs(line: &Line) -> Vec<(String, Style)> {
    let mut runs: Vec<(String, Style)> = vec![];
    for c in line {
        if let Some((s, style)) = runs.last_mut().filter(|(_, s)| *s == c.style) {
            let _ = style;
            s.push(c.ch);
        } else {
            runs.push((c.ch.to_string(), c.style.clone()));
        }
    }
    runs
}
pub fn html(doc: &Document) -> String {
    let mut out = String::from(
        "<!doctype html>\n<html lang=\"en\"><meta charset=\"utf-8\"><title>Rich output</title><body style=\"background:#101010;color:#eeeeee\"><pre style=\"font-family:monospace;line-height:1.25\">",
    );
    for line in doc {
        for (text, style) in runs(line) {
            let text = escape(&text);
            let span = format!("<span style=\"{}\">{text}</span>", style.css());
            if let Some(url) = style.link.filter(|u| {
                u.starts_with("https://") || u.starts_with("http://") || u.starts_with("mailto:")
            }) {
                out += &format!("<a href=\"{}\">{span}</a>", escape(&url));
            } else {
                out += &span;
            }
        }
        out.push('\n');
    }
    out + "</pre></body></html>\n"
}
pub fn svg(doc: &Document) -> String {
    let columns = doc.iter().map(width).max().unwrap_or(0);
    let w = columns * 9 + 32;
    let h = doc.len() * 20 + 32;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><title>Rich output</title><rect width=\"100%\" height=\"100%\" fill=\"#101010\"/>"
    );
    for (y, line) in doc.iter().enumerate() {
        let mut x = 16;
        for (text, style) in runs(line) {
            let cells = width(&plain(&text));
            let fg = if style.reverse {
                style.bg.unwrap_or((16, 16, 16))
            } else {
                style.fg.unwrap_or((238, 238, 238))
            };
            let bg = if style.reverse {
                style.fg.or(Some((238, 238, 238)))
            } else {
                style.bg
            };
            if let Some((r, g, b)) = bg {
                out += &format!(
                    "<rect x=\"{x}\" y=\"{}\" width=\"{}\" height=\"20\" fill=\"#{r:02x}{g:02x}{b:02x}\"/>",
                    16 + y * 20,
                    cells * 9
                );
            }
            out += &format!(
                "<text x=\"{x}\" y=\"{}\" xml:space=\"preserve\" font-family=\"monospace\" font-size=\"15\" fill=\"#{:02x}{:02x}{:02x}\" style=\"{}\">{}</text>",
                31 + y * 20,
                fg.0,
                fg.1,
                fg.2,
                style.css(),
                escape(&text)
            );
            x += cells * 9;
        }
    }
    out + "</svg>\n"
}

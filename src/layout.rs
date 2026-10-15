use crate::text::*;

pub fn align(line: &Line, size: usize, mode: &str) -> Line {
    let gap = size.saturating_sub(width(line));
    let left = match mode {
        "right" => gap,
        "center" => gap / 2,
        _ => 0,
    };
    let mut result = spaces(left);
    result.extend(line.clone());
    result
}
pub fn justify(line: &Line, size: usize, mode: &str) -> Line {
    if mode != "full" {
        let mut out = align(line, size, mode);
        if mode == "center" {
            out.extend(spaces(size.saturating_sub(width(&out))));
        }
        return out;
    }
    let gaps = line.iter().filter(|c| c.ch == ' ').count();
    if gaps == 0 {
        return line.clone();
    }
    let extra = size.saturating_sub(width(line));
    let mut n = 0;
    let mut out = vec![];
    for c in line {
        out.push(c.clone());
        if c.ch == ' ' {
            out.extend(spaces(extra / gaps + usize::from(n < extra % gaps)));
            n += 1;
        }
    }
    out
}
pub fn pad(doc: Document, padding: [usize; 4], expand: bool, limit: usize) -> Document {
    let [top, right, bottom, left] = padding;
    let content = if expand {
        limit.saturating_sub(left + right)
    } else {
        doc.iter().map(width).max().unwrap_or(0)
    };
    let mut result = vec![spaces(content + left + right); top];
    for mut line in doc {
        let mut row = spaces(left);
        let w = width(&line);
        row.append(&mut line);
        row.extend(spaces(content.saturating_sub(w) + right));
        result.push(row);
    }
    result.extend(vec![spaces(content + left + right); bottom]);
    result
}
pub fn panel(
    doc: Document,
    kind: &str,
    limit: usize,
    expand: bool,
    title: &Line,
    caption: &Line,
    style: &Style,
) -> Document {
    let chars = match kind {
        "ascii" | "ascii2" => ['+', '-', '+', '|', '+', '+'],
        "rounded" => ['╭', '─', '╮', '│', '╰', '╯'],
        "heavy" => ['┏', '━', '┓', '┃', '┗', '┛'],
        "double" => ['╔', '═', '╗', '║', '╚', '╝'],
        _ => ['┌', '─', '┐', '│', '└', '┘'],
    };
    let natural = doc
        .iter()
        .map(width)
        .max()
        .unwrap_or(0)
        .max(width(title) + 2)
        .max(width(caption) + 2)
        + 4;
    let size = if expand { limit } else { natural.min(limit) }.max(2);
    let border = |left: char, right: char, label: &Line| {
        let inner = size - 2;
        let mut line = styled(&left.to_string(), style);
        let label = truncate(label, inner.saturating_sub(2));
        if !label.is_empty() && inner >= 2 {
            let gap = inner - width(&label) - 2;
            line.extend(styled(&chars[1].to_string().repeat(gap / 2), style));
            line.extend(plain(" "));
            line.extend(label);
            line.extend(plain(" "));
            line.extend(styled(&chars[1].to_string().repeat(gap - gap / 2), style));
        } else {
            line.extend(styled(&chars[1].to_string().repeat(inner), style));
        }
        line.extend(styled(&right.to_string(), style));
        line
    };
    let mut result = vec![border(chars[0], chars[2], title)];
    for line in doc {
        let inner = size.saturating_sub(4).max(1);
        for line in wrap(&line, inner, false) {
            let mut row = styled(&chars[3].to_string(), style);
            if size >= 4 {
                row.extend(plain(" "));
                row.extend(truncate(&line, inner));
                row.extend(spaces(inner.saturating_sub(width(&line))));
                row.extend(plain(" "));
            } else {
                row.extend(truncate(&line, size - 2));
                row.extend(spaces((size - 2).saturating_sub(width(&line))));
            }
            row.extend(styled(&chars[3].to_string(), style));
            result.push(row);
        }
    }
    result.push(border(chars[4], chars[5], caption));
    result
}
pub fn rule(
    label: &Line,
    size: usize,
    characters: &str,
    style: &Style,
    alignment: &str,
) -> Result<Line, String> {
    let chars = plain(characters);
    let cw = width(&chars);
    if cw == 0 {
        return Err("rule characters must have positive display width".into());
    }
    let fill = |n: usize| {
        let mut l = vec![];
        for _ in 0..n / cw {
            l.extend(chars.clone());
        }
        l.extend(spaces(n % cw));
        for c in &mut l {
            c.style = style.clone();
        }
        l
    };
    let label = truncate(label, size.saturating_sub(4));
    if label.is_empty() {
        return Ok(fill(size));
    }
    let remaining = size.saturating_sub(width(&label) + 2);
    let left = match alignment {
        "left" => 0,
        "right" => remaining,
        _ => remaining / 2,
    };
    let mut l = fill(left);
    if left > 0 {
        l.extend(plain(" "));
    }
    l.extend(label);
    if left < remaining {
        l.extend(plain(" "));
    }
    let used = width(&l);
    l.extend(fill(size.saturating_sub(used)));
    Ok(l)
}

/// A wrapping table shared by CSV and Markdown renderers.
pub fn table(rows: &[Vec<Line>], header: bool, limit: usize) -> Document {
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    if cols == 0 {
        return vec![];
    }
    // Each column needs one content cell, two spaces, and a separator.
    if limit < cols * 4 + 1 {
        let mut out = vec![];
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                let mut l = plain(&format!("{}: ", i + 1));
                l.extend(cell.clone());
                out.extend(wrap(&l, limit, true));
            }
            out.push(vec![]);
        }
        return out;
    }
    let mut sizes = vec![1; cols];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            sizes[i] = sizes[i].max(width(cell));
        }
    }
    let available = limit - cols * 3 - 1;
    while sizes.iter().sum::<usize>() > available {
        let i = sizes.iter().enumerate().max_by_key(|(_, s)| *s).unwrap().0;
        sizes[i] -= 1;
    }
    let numeric: Vec<bool> = (0..cols)
        .map(|i| {
            rows.iter().skip(usize::from(header)).all(|row| {
                row.get(i).is_some_and(|cell| {
                    let value: String = cell.iter().map(|c| c.ch).collect();
                    value.is_empty() || value.parse::<f64>().is_ok()
                })
            })
        })
        .collect();
    let border_style = Style::parse("blue").unwrap();
    let border = |a: &str, b: &str, c: &str, h: &str| {
        let mut s = a.to_string();
        for (i, n) in sizes.iter().enumerate() {
            s += &h.repeat(n + 2);
            s += if i + 1 == cols { c } else { b };
        }
        styled(&s, &border_style)
    };
    let mut out = vec![if header {
        border("┏", "┳", "┓", "━")
    } else {
        border("┌", "┬", "┐", "─")
    }];
    for (r, row) in rows.iter().enumerate() {
        let cells: Vec<Document> = (0..cols)
            .map(|i| wrap(row.get(i).unwrap_or(&vec![]), sizes[i], true))
            .collect();
        let height = cells.iter().map(Vec::len).max().unwrap_or(1);
        for y in 0..height {
            let vertical = if r == 0 && header { "┃" } else { "│" };
            let mut line = styled(vertical, &border_style);
            for i in 0..cols {
                let cell = cells[i].get(y).cloned().unwrap_or_default();
                let mut cell = align(&cell, sizes[i], if numeric[i] { "right" } else { "left" });
                if numeric[i] || (r == 0 && header) {
                    let style =
                        Style::parse(if numeric[i] { "bold green" } else { "bold" }).unwrap();
                    for c in &mut cell {
                        c.style = style.overlay(&c.style);
                    }
                }
                line.extend(plain(" "));
                let w = width(&cell);
                line.extend(cell);
                line.extend(spaces(sizes[i].saturating_sub(w) + 1));
                line.extend(styled(vertical, &border_style));
            }
            out.push(line);
        }
        if r == 0 && header {
            out.push(border("┡", "╇", "┩", "━"));
        }
    }
    out.push(border("└", "┴", "┘", "─"));
    out
}

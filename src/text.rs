use std::{collections::HashMap, sync::LazyLock};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Style {
    // Explicitly disabled attributes must override inherited markup styles.
    pub clear: u16,
    pub fg: Option<(u8, u8, u8)>,
    pub bg: Option<(u8, u8, u8)>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub reverse: bool,
    pub blink: bool,
    pub hidden: bool,
    pub link: Option<String>,
}
impl Style {
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut style = Self::default();
        let mut background = false;
        let mut negate = false;
        let mut tokens = s.split_whitespace();
        while let Some(token) = tokens.next() {
            let enabled = !negate;
            match token {
                "none" | "default" | "reset" => {}
                "on" => {
                    background = true;
                    continue;
                }
                "not" => {
                    negate = true;
                    continue;
                }
                "bold" | "b" => style.bold = enabled,
                "dim" => style.dim = enabled,
                "italic" | "i" => style.italic = enabled,
                "underline" | "u" => style.underline = enabled,
                "strike" | "s" => style.strike = enabled,
                "reverse" => style.reverse = enabled,
                "blink" | "blink2" => style.blink = enabled,
                "conceal" => style.hidden = enabled,
                "link" => style.link = tokens.next().map(str::to_string),
                _ => {
                    let color =
                        color(token).ok_or_else(|| format!("unknown style or color: {token}"))?;
                    if background {
                        style.bg = Some(color);
                        background = false;
                    } else {
                        style.fg = Some(color);
                    }
                }
            }
            if negate {
                style.clear |= match token {
                    "bold" | "b" => 1,
                    "dim" => 2,
                    "italic" | "i" => 4,
                    "underline" | "u" => 8,
                    "strike" | "s" => 16,
                    "reverse" => 32,
                    "blink" | "blink2" => 64,
                    "conceal" => 128,
                    _ => return Err(format!("not cannot precede {token}")),
                };
            }
            negate = false;
        }
        if background || negate {
            return Err(format!("incomplete style: {s}"));
        }
        Ok(style)
    }
    pub fn overlay(&self, other: &Self) -> Self {
        let attr = |a: bool, b: bool, bit: u16| {
            if other.clear & bit != 0 {
                false
            } else {
                a || b
            }
        };
        Self {
            clear: (self.clear & !other.enabled_attributes()) | other.clear,
            fg: other.fg.or(self.fg),
            bg: other.bg.or(self.bg),
            bold: attr(self.bold, other.bold, 1),
            dim: attr(self.dim, other.dim, 2),
            italic: attr(self.italic, other.italic, 4),
            underline: attr(self.underline, other.underline, 8),
            strike: attr(self.strike, other.strike, 16),
            reverse: attr(self.reverse, other.reverse, 32),
            blink: attr(self.blink, other.blink, 64),
            hidden: attr(self.hidden, other.hidden, 128),
            link: other.link.clone().or_else(|| self.link.clone()),
        }
    }
    fn enabled_attributes(&self) -> u16 {
        u16::from(self.bold)
            | (u16::from(self.dim) << 1)
            | (u16::from(self.italic) << 2)
            | (u16::from(self.underline) << 3)
            | (u16::from(self.strike) << 4)
            | (u16::from(self.reverse) << 5)
            | (u16::from(self.blink) << 6)
            | (u16::from(self.hidden) << 7)
    }
    pub fn ansi(&self) -> String {
        let mut codes = Vec::new();
        for (enabled, code) in [
            (self.bold, "1"),
            (self.dim, "2"),
            (self.italic, "3"),
            (self.underline, "4"),
            (self.blink, "5"),
            (self.reverse, "7"),
            (self.hidden, "8"),
            (self.strike, "9"),
        ] {
            if enabled {
                codes.push(code.into());
            }
        }
        if let Some((r, g, b)) = self.fg {
            codes.push(format!("38;2;{r};{g};{b}"));
        }
        if let Some((r, g, b)) = self.bg {
            codes.push(format!("48;2;{r};{g};{b}"));
        }
        if codes.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", codes.join(";"))
        }
    }
    pub fn css(&self) -> String {
        let mut css = String::new();
        let (fg, bg) = if self.reverse {
            (
                self.bg.or(Some((16, 16, 16))),
                self.fg.or(Some((230, 230, 230))),
            )
        } else {
            (self.fg, self.bg)
        };
        if let Some((r, g, b)) = fg {
            css += &format!("color:#{r:02x}{g:02x}{b:02x};");
        }
        if let Some((r, g, b)) = bg {
            css += &format!("background-color:#{r:02x}{g:02x}{b:02x};");
        }
        if self.bold {
            css += "font-weight:bold;";
        }
        if self.italic {
            css += "font-style:italic;";
        }
        if self.dim {
            css += "opacity:0.5;";
        }
        if self.hidden {
            css += "visibility:hidden;";
        }
        if self.underline || self.strike {
            css += "text-decoration:";
            if self.underline {
                css += "underline ";
            }
            if self.strike {
                css += "line-through";
            }
            css += ";";
        }
        css
    }
}
static COLORS: LazyLock<HashMap<String, usize>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../assets/colors.json")).expect("bundled color table")
});
static EMOJI: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../assets/emoji.json")).expect("bundled emoji table")
});

fn color(name: &str) -> Option<(u8, u8, u8)> {
    let n = match name {
        "black" => 0,
        "red" => 1,
        "green" => 2,
        "yellow" => 3,
        "blue" => 4,
        "magenta" => 5,
        "cyan" => 6,
        "white" => 7,
        "bright_black" | "grey" | "gray" => 8,
        "bright_red" => 9,
        "bright_green" => 10,
        "bright_yellow" => 11,
        "bright_blue" => 12,
        "bright_magenta" => 13,
        "bright_cyan" => 14,
        "bright_white" => 15,
        "orange1" | "orange" => 214,
        "purple" => 129,
        "pink1" | "pink" => 218,
        _ => {
            if let Some(hex) = name.strip_prefix('#')
                && hex.len() == 6
            {
                let v = u32::from_str_radix(hex, 16).ok()?;
                return Some(((v >> 16) as u8, (v >> 8) as u8, v as u8));
            }
            if let Some(rgb) = name.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
                let v = rgb
                    .split(',')
                    .map(str::parse::<u8>)
                    .collect::<Result<Vec<_>, _>>()
                    .ok()?;
                return if v.len() == 3 {
                    Some((v[0], v[1], v[2]))
                } else {
                    None
                };
            }
            if let Some(n) = COLORS.get(name) {
                *n
            } else {
                name.strip_prefix("color(")?
                    .strip_suffix(')')?
                    .parse::<u8>()
                    .ok()? as usize
            }
        }
    };
    const BASE: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    Some(if n < 16 {
        BASE[n]
    } else if n < 232 {
        let n = n - 16;
        let component = |x: usize| if x == 0 { 0 } else { (55 + 40 * x) as u8 };
        (component(n / 36), component(n / 6 % 6), component(n % 6))
    } else {
        let x = (8 + 10 * (n - 232)) as u8;
        (x, x, x)
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}
pub type Line = Vec<Cell>;
pub type Document = Vec<Line>;
pub fn plain(s: &str) -> Line {
    styled(s, &Style::default())
}
pub fn styled(s: &str, style: &Style) -> Line {
    // Strip terminal controls from untrusted files/URLs. Newlines are handled by callers.
    s.chars()
        .filter(|c| !c.is_control())
        .map(|ch| Cell {
            ch,
            style: style.clone(),
        })
        .collect()
}
pub fn width(line: &Line) -> usize {
    if line.iter().all(|c| c.ch.is_ascii()) {
        return line.len();
    }
    let s: String = line.iter().map(|c| c.ch).collect();
    UnicodeWidthStr::width(s.as_str())
}
// Preserve grapheme clusters (combining marks, emoji ZWJ sequences and flags)
// when wrapping or cropping styled text.
fn clusters(line: &Line) -> Vec<(usize, usize, bool)> {
    if line.iter().all(|c| c.ch.is_ascii()) {
        return line
            .iter()
            .enumerate()
            .map(|(i, c)| (i + 1, 1, c.ch == ' '))
            .collect();
    }
    let s: String = line.iter().map(|c| c.ch).collect();
    let mut index = 0;
    s.graphemes(true)
        .map(|g| {
            index += g.chars().count();
            (index, UnicodeWidthStr::width(g), g == " ")
        })
        .collect()
}
pub fn spaces(n: usize) -> Line {
    plain(&" ".repeat(n))
}
pub fn plain_string(doc: &Document) -> String {
    let mut s = String::new();
    for line in doc {
        for c in line {
            s.push(c.ch);
        }
        s.push('\n');
    }
    s
}
pub fn ansi_string(doc: &Document, color: bool, links: bool) -> String {
    if !color {
        return plain_string(doc);
    }
    let mut s = String::new();
    for line in doc {
        let mut current = Style::default();
        for cell in line {
            if cell.style != current {
                if current.link.is_some() && links {
                    s += "\x1b]8;;\x1b\\";
                }
                if current != Style::default() {
                    s += "\x1b[0m";
                }
                s += &cell.style.ansi();
                if links && let Some(link) = &cell.style.link {
                    s += &format!("\x1b]8;;{}\x1b\\", safe_url(link));
                }
                current = cell.style.clone();
            }
            s.push(cell.ch);
        }
        if current.link.is_some() && links {
            s += "\x1b]8;;\x1b\\";
        }
        if current != Style::default() {
            s += "\x1b[0m";
        }
        s.push('\n');
    }
    s
}
pub fn safe_url(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn markup(input: &str, emoji: bool) -> Result<Document, String> {
    let mut text = input.to_string();
    if emoji {
        let mut out = String::new();
        let mut rest = text.as_str();
        while let Some(start) = rest.find(':') {
            out += &rest[..start];
            rest = &rest[start + 1..];
            if let Some(end) = rest.find(':')
                && let Some(e) = EMOJI.get(&rest[..end])
            {
                out += e.as_str();
                rest = &rest[end + 1..];
                continue;
            }
            out.push(':');
        }
        out += rest;
        text = out;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut stack: Vec<(String, Style)> = Vec::new();
    let mut doc = vec![vec![]];
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && chars.get(i + 1) == Some(&'[') {
            doc.last_mut()
                .unwrap()
                .extend(styled("[", &combined(&stack)));
            i += 2;
            continue;
        }
        if chars[i] == '['
            && let Some(end) = chars[i + 1..]
                .iter()
                .position(|c| *c == ']')
                .map(|n| n + i + 1)
        {
            let tag: String = chars[i + 1..end].iter().collect();
            if let Some(close) = tag.strip_prefix('/') {
                if close.is_empty() {
                    if stack.pop().is_none() {
                        return Err("closing markup tag has no opening tag".into());
                    }
                } else if let Some(pos) = stack.iter().rposition(|(name, _)| name == close) {
                    stack.remove(pos);
                } else {
                    return Err(format!(
                        "closing tag [/{close}] has no matching opening tag"
                    ));
                }
                i = end + 1;
                continue;
            } else if tag.starts_with(|c: char| c.is_alphabetic() || c == '#') {
                let parsed = if let Some(link) = tag.strip_prefix("link=") {
                    Ok(Style {
                        link: Some(link.into()),
                        ..Style::default()
                    })
                } else {
                    Style::parse(&tag)
                };
                stack.push((tag, parsed?));
                i = end + 1;
                continue;
            }
        }
        let ch = chars[i];
        if ch == '\n' {
            doc.push(vec![]);
        } else if ch == '\t' {
            doc.last_mut()
                .unwrap()
                .extend(styled("    ", &combined(&stack)));
        } else if !ch.is_control() {
            doc.last_mut().unwrap().push(Cell {
                ch,
                style: combined(&stack),
            });
        }
        i += 1;
    }
    Ok(doc)
}
fn combined(stack: &[(String, Style)]) -> Style {
    stack
        .iter()
        .fold(Style::default(), |s, (_, next)| s.overlay(next))
}

pub fn wrap(line: &Line, limit: usize, words: bool) -> Document {
    let limit = limit.max(1);
    if line.is_empty() {
        return vec![vec![]];
    }
    let groups = clusters(line);
    let mut result = vec![];
    let mut start = 0;
    while start < groups.len() {
        let mut end = start;
        let mut used = 0;
        let mut space = None;
        while end < groups.len() {
            let (_, w, is_space) = groups[end];
            if used + w > limit && end > start {
                break;
            }
            if is_space {
                space = Some((end, used));
            }
            used += w;
            end += 1;
            if used > limit {
                break;
            }
        }
        if end < groups.len()
            && words
            && !groups[end].2
            && let Some((s, w)) = space.filter(|(s, _)| *s > start)
        {
            end = s + usize::from(w < limit);
        }
        let first = if start == 0 { 0 } else { groups[start - 1].0 };
        let last = groups[end - 1].0;
        result.push(line[first..last].to_vec());
        start = end;
        if words {
            while start < groups.len() && groups[start].2 {
                start += 1;
            }
        }
    }
    result
}
pub fn truncate(line: &Line, limit: usize) -> Line {
    let mut used = 0;
    let mut end = 0;
    for (index, w, _) in clusters(line) {
        if used + w > limit {
            break;
        }
        used += w;
        end = index;
    }
    line[..end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_markup() {
        let d = markup("[red]a[b]界[/b]c[/red]", false).unwrap();
        assert_eq!(plain_string(&d), "a界c\n");
        assert!(d[0][1].style.bold);
        assert!(!d[0][2].style.bold);
        assert_eq!(width(&d[0]), 4);
    }
    #[test]
    fn negated_markup_attributes_override_parent() {
        let d = markup("[bold]a[not bold]b[/]c[/]", false).unwrap();
        assert!(d[0][0].style.bold);
        assert!(!d[0][1].style.bold);
        assert!(d[0][2].style.bold);
    }
    #[test]
    fn escaped_and_bad_markup() {
        assert_eq!(plain_string(&markup(r"\[red]", false).unwrap()), "[red]\n");
        assert!(markup("[/red]", false).is_err());
    }
    #[test]
    fn emoji_codes() {
        assert_eq!(plain_string(&markup(":rocket:", true).unwrap()), "🚀\n");
    }
    #[test]
    fn wrapping_wide() {
        let d = wrap(&plain("界界 a"), 4, true);
        assert_eq!(plain_string(&d), "界界\na\n");
    }
    #[test]
    fn emoji_graphemes_are_not_split() {
        let family = "👨‍👩‍👧‍👦";
        let line = plain(&format!("{family}🇮🇹e\u{301}"));
        assert_eq!(width(&line), 5);
        assert_eq!(
            plain_string(&vec![truncate(&line, 2)]),
            format!("{family}\n")
        );
        assert_eq!(wrap(&line, 2, false).len(), 3);
    }
    #[test]
    fn colors() {
        assert_eq!(
            Style::parse("bold #ff8800 on color(17)").unwrap().bg,
            Some((0, 0, 95))
        );
        assert!(Style::parse("nonsense").is_err());
    }
}

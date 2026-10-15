use crate::text::*;
use crossterm::{
    cursor,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseEventKind,
    },
    execute, queue,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

struct Screen;
impl Screen {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let screen = Self;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            cursor::Hide
        )?;
        Ok(screen)
    }
}
impl Drop for Screen {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            DisableMouseCapture,
            cursor::Show,
            LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    }
}
#[derive(Default)]
struct View {
    top: usize,
    horizontal: usize,
}
impl View {
    fn key(&mut self, code: KeyCode, modifiers: KeyModifiers, height: usize, total: usize) -> bool {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return false,
            KeyCode::Down | KeyCode::Char('j') => self.top = self.top.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.top = self.top.saturating_sub(1),
            KeyCode::PageDown | KeyCode::Char(' ') => self.top = self.top.saturating_add(height),
            KeyCode::PageUp => self.top = self.top.saturating_sub(height),
            KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.top = self.top.saturating_add(height / 2)
            }
            KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.top = self.top.saturating_sub(height / 2)
            }
            KeyCode::Home | KeyCode::Char('g') => self.top = 0,
            KeyCode::End | KeyCode::Char('G') => self.top = total.saturating_sub(height),
            KeyCode::Right | KeyCode::Char('l') => {
                self.horizontal = self.horizontal.saturating_add(4)
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.horizontal = self.horizontal.saturating_sub(4)
            }
            _ => {}
        }
        self.top = self.top.min(total.saturating_sub(height));
        true
    }
}
pub fn show(doc: &Document, title: &str, color: bool, links: bool) -> io::Result<()> {
    let _screen = Screen::enter()?;
    let mut view = View::default();
    let mut out = io::stdout();
    loop {
        let (w, h) = terminal::size()?;
        let height = h.saturating_sub(1) as usize;
        let columns = w.saturating_sub(1) as usize;
        view.top = view.top.min(doc.len().saturating_sub(height));
        queue!(out, cursor::MoveTo(0, 0), Clear(ClearType::All))?;
        for (y, line) in doc.iter().skip(view.top).take(height).enumerate() {
            let mut skipped = 0;
            let line: Line = line
                .iter()
                .skip_while(|c| {
                    let before = skipped;
                    skipped += unicode_width::UnicodeWidthChar::width(c.ch).unwrap_or(0);
                    before < view.horizontal
                })
                .cloned()
                .collect();
            let text = ansi_string(&vec![truncate(&line, columns)], color, links);
            queue!(out, cursor::MoveTo(0, y as u16))?;
            out.write_all(text.trim_end_matches('\n').as_bytes())?;
            if doc.len() > height && columns > 0 {
                let thumb = (view.top * height / doc.len()).min(height.saturating_sub(1));
                queue!(out, cursor::MoveTo(w.saturating_sub(1), y as u16))?;
                out.write_all(if y == thumb {
                    "█".as_bytes()
                } else {
                    "│".as_bytes()
                })?;
            }
        }
        let status = format!(
            " q quit · ↑↓/jk · PgUp/PgDn · Home/End  {}/{}  {} ",
            view.top + 1,
            doc.len(),
            title
        );
        let status = truncate(&plain(&status), w as usize);
        queue!(out, cursor::MoveTo(0, h.saturating_sub(1)))?;
        if color {
            out.write_all(b"\x1b[7m")?;
        }
        out.write_all(
            plain_string(&vec![status])
                .trim_end_matches('\n')
                .as_bytes(),
        )?;
        if color {
            out.write_all(b"\x1b[0m")?;
        }
        out.flush()?;
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if !view.key(key.code, key.modifiers, height, doc.len()) {
                    break;
                }
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => {
                    view.key(KeyCode::Down, KeyModifiers::NONE, height, doc.len());
                }
                MouseEventKind::ScrollUp => {
                    view.key(KeyCode::Up, KeyModifiers::NONE, height, doc.len());
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn navigation_clamps_and_quits() {
        let mut v = View::default();
        assert!(v.key(KeyCode::End, KeyModifiers::NONE, 20, 100));
        assert_eq!(v.top, 80);
        v.key(KeyCode::Down, KeyModifiers::NONE, 20, 100);
        assert_eq!(v.top, 80);
        v.key(KeyCode::Char('u'), KeyModifiers::CONTROL, 20, 100);
        assert_eq!(v.top, 70);
        v.key(KeyCode::Home, KeyModifiers::NONE, 20, 100);
        assert_eq!(v.top, 0);
        v.key(KeyCode::Up, KeyModifiers::NONE, 20, 100);
        assert_eq!(v.top, 0);
        assert!(!v.key(KeyCode::Char('q'), KeyModifiers::NONE, 20, 100));
    }
}

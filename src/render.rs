use std::io::{self, Write};

use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::execute;
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::config::Config;
use crate::document::{DocumentSet, LineView};
use crate::highlight::SyntaxEngine;
use crate::style::{StyledSpan, TextStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenSize {
    pub width: u16,
    pub height: u16,
}

pub struct TerminalSession {
    active: bool,
    alternate_screen: bool,
}

impl TerminalSession {
    pub fn enter(no_init: bool) -> Result<Self> {
        terminal::enable_raw_mode()?;
        if !no_init {
            execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        }
        Ok(Self {
            active: true,
            alternate_screen: !no_init,
        })
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.active {
            let _ = terminal::disable_raw_mode();
            if self.alternate_screen {
                let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
            }
        }
    }
}

pub fn size() -> Result<ScreenSize> {
    let (width, height) = terminal::size()?;
    Ok(ScreenSize { width, height })
}

pub fn render(
    out: &mut impl Write,
    docs: &DocumentSet,
    config: &Config,
    engine: &SyntaxEngine,
    top_line: usize,
    prompt: Option<&str>,
    status: &str,
) -> Result<()> {
    let screen = size()?;
    let content_height = screen
        .height
        .saturating_sub(if config.status_bar { 1 } else { 0 });
    let mut remaining_rows = content_height as usize;
    let mut global = top_line;
    let line_number_width = if config.line_numbers {
        docs.lines
            .iter()
            .filter(|line| !line.header)
            .map(|line| line.local_line + 1)
            .max()
            .unwrap_or(1)
            .to_string()
            .len()
    } else {
        0
    };

    queue_clear(out)?;

    while remaining_rows > 0 {
        if let Some(view) = docs.line(global) {
            let rows = render_line(
                out,
                docs,
                engine,
                config,
                &view,
                screen.width as usize,
                line_number_width,
            )?;
            remaining_rows = remaining_rows.saturating_sub(rows);
            global += 1;
        } else {
            break;
        }
    }

    if config.status_bar {
        render_status(
            out,
            screen,
            prompt.unwrap_or(status),
            status,
            top_line,
            docs,
            config,
        )?;
    }

    out.flush()?;
    Ok(())
}

fn queue_clear(out: &mut impl Write) -> Result<()> {
    execute!(out, MoveTo(0, 0), Clear(ClearType::All))?;
    Ok(())
}

fn render_status(
    out: &mut impl Write,
    screen: ScreenSize,
    prompt: &str,
    status: &str,
    top_line: usize,
    docs: &DocumentSet,
    config: &Config,
) -> Result<()> {
    let y = screen.height.saturating_sub(1);
    execute!(out, MoveTo(0, y), Clear(ClearType::CurrentLine))?;
    let current = docs.line(top_line);
    let mut left = String::new();
    if let Some(view) = current {
        let kind = if view.header { "header" } else { "line" };
        let total = docs.line_count().max(1);
        let percent = ((top_line + 1) * 100 / total).min(100);
        left = format!(
            "{} {} / {} [{}%] {}",
            docs.document(view.doc)
                .map(|d| d.name.as_str())
                .unwrap_or("<stdin>"),
            view.local_line + 1,
            docs.document(view.doc).map(|d| d.line_count()).unwrap_or(0),
            percent,
            kind
        );
    }
    let composed = if prompt.is_empty() {
        if status.is_empty() {
            left
        } else {
            format!("{} {}", left, status)
        }
    } else {
        prompt.to_string()
    };
    let mut line = composed;
    if line.is_empty() {
        line.push_str("xless");
    }
    let width = screen.width as usize;
    line = truncate_to_width(&line, width);
    write!(out, "\x1b[7m{:<width$}\x1b[0m", line, width = width)?;
    let _ = config;
    Ok(())
}

fn render_line(
    out: &mut impl Write,
    docs: &DocumentSet,
    engine: &SyntaxEngine,
    config: &Config,
    view: &LineView<'_>,
    width: usize,
    line_number_width: usize,
) -> Result<usize> {
    let mut spans = if view.header {
        vec![StyledSpan {
            text: view.text.to_string(),
            style: TextStyle {
                reverse: true,
                ..TextStyle::default()
            },
        }]
    } else if config.raw_control_chars {
        vec![StyledSpan {
            text: view.text.to_string(),
            style: TextStyle::default(),
        }]
    } else if view.bytes.contains(&0x1b) {
        engine.parse_ansi_line(&view.text)
    } else {
        engine.highlight_line(&docs.docs[view.doc].syntax, &view.text)
    };

    if config.line_numbers && !view.header {
        let prefix = format!(
            "{:>width$} ",
            view.local_line + 1,
            width = line_number_width
        );
        spans.insert(0, StyledSpan::plain(prefix));
    } else if view.header {
        spans.insert(0, StyledSpan::plain(String::new()));
    }

    let rows = paint_spans(out, &spans, width, config)?;
    if rows == 0 {
        writeln!(out)?;
        return Ok(1);
    }
    Ok(rows)
}

fn paint_spans(
    out: &mut impl Write,
    spans: &[StyledSpan],
    width: usize,
    config: &Config,
) -> Result<usize> {
    let mut current_style = TextStyle::default();
    let mut col = 0usize;
    let mut rows = 1usize;
    for span in spans {
        if span.style != current_style {
            if !current_style.eq(&TextStyle::default()) {
                write!(out, "\x1b[0m")?;
            }
            write!(out, "{}", span.style.to_ansi_prefix())?;
            current_style = span.style;
        }
        for ch in span.text.chars() {
            let rendered = render_char(ch, col, config);
            if rendered.is_empty() {
                continue;
            }
            let rendered_width = UnicodeWidthStr::width(rendered.as_str());
            if width > 0 && col + rendered_width > width {
                if config.chop_long_lines {
                    continue;
                }
                write!(out, "\x1b[0m\r\n")?;
                rows += 1;
                col = 0;
                if !current_style.eq(&TextStyle::default()) {
                    write!(out, "{}", current_style.to_ansi_prefix())?;
                }
            }
            write!(out, "{}", rendered)?;
            col += rendered_width;
        }
    }
    if !current_style.eq(&TextStyle::default()) {
        write!(out, "\x1b[0m")?;
    }
    Ok(rows)
}

fn render_char(ch: char, col: usize, config: &Config) -> String {
    if config.raw_control_chars {
        return ch.to_string();
    }
    if ch == '\t' {
        let spaces = config.tab_width.max(1) - (col % config.tab_width.max(1));
        return " ".repeat(spaces);
    }
    if ch.is_control() {
        return if ch == '\u{7f}' {
            "^?".to_string()
        } else {
            format!("^{}", ((ch as u32 & 0x1f) + 0x40) as u8 as char)
        };
    }
    ch.to_string()
}

fn truncate_to_width(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_owned();
    }

    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width > width {
            break;
        }
        out.push(ch);
        used += ch_width;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_without_breaking_utf8_boundaries() {
        assert_eq!(truncate_to_width("xéy", 2), "xé");
        assert_eq!(truncate_to_width("hello", 3), "hel");
    }
}

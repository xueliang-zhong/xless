use std::borrow::Cow;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use regex::bytes::Regex as BytesRegex;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::style::{Rgb, StyledSpan, TextStyle};

#[derive(Debug, Clone)]
pub enum SyntaxChoice {
    Plain,
    Named(String),
}

#[derive(Debug)]
pub struct SyntaxEngine {
    ps: &'static SyntaxSet,
    theme: Theme,
}

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

impl SyntaxEngine {
    pub fn new(theme_name: &str) -> Result<Self> {
        let ps = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
        let themes = THEME_SET.get_or_init(ThemeSet::load_defaults);
        let theme = themes
            .themes
            .get(theme_name)
            .cloned()
            .or_else(|| themes.themes.get("base16-ocean.dark").cloned())
            .context("loading syntect theme")?;
        Ok(Self { ps, theme })
    }

    pub fn detect(
        &self,
        path: &Option<std::path::PathBuf>,
        bytes: &[u8],
        language_hint: Option<&str>,
    ) -> SyntaxChoice {
        if let Some(language) = language_hint {
            return SyntaxChoice::Named(language.to_string());
        }
        if let Some(path) = path
            && let Some(syntax) = self.syntax_for_path(path, bytes)
        {
            return SyntaxChoice::Named(syntax.name.clone());
        }
        SyntaxChoice::Plain
    }

    fn syntax_for_path(&self, path: &Path, bytes: &[u8]) -> Option<&SyntaxReference> {
        self.ps
            .find_syntax_for_file(path)
            .ok()
            .flatten()
            .or_else(|| self.syntax_from_shebang(bytes))
    }

    fn syntax_from_shebang(&self, bytes: &[u8]) -> Option<&SyntaxReference> {
        let first_line = bytes.split(|b| *b == b'\n').next()?;
        let text = std::str::from_utf8(first_line).ok()?;
        if !text.starts_with("#!") {
            return None;
        }
        let shebang = text.trim_start_matches("#!");
        let candidate = shebang
            .split_whitespace()
            .next()
            .and_then(|p| Path::new(p).file_name())?
            .to_string_lossy()
            .to_string();
        self.ps.find_syntax_by_token(&candidate)
    }

    pub fn highlight_line(&self, choice: &SyntaxChoice, line: &str) -> Vec<StyledSpan> {
        match choice {
            SyntaxChoice::Plain => vec![StyledSpan::plain(line)],
            SyntaxChoice::Named(name) => {
                if let Some(syntax) = self.ps.find_syntax_by_name(name) {
                    self.highlight_with_syntax(syntax, line)
                } else {
                    vec![StyledSpan::plain(line)]
                }
            }
        }
    }

    fn highlight_with_syntax(&self, syntax: &SyntaxReference, line: &str) -> Vec<StyledSpan> {
        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut spans = Vec::new();
        if let Ok(ranges) = highlighter.highlight_line(line, self.ps) {
            for (style, text) in ranges {
                let mut span_style = TextStyle {
                    fg: Some(style.foreground.into()),
                    ..TextStyle::default()
                };
                if style
                    .font_style
                    .contains(syntect::highlighting::FontStyle::BOLD)
                {
                    span_style.bold = true;
                }
                if style
                    .font_style
                    .contains(syntect::highlighting::FontStyle::ITALIC)
                {
                    span_style.italic = true;
                }
                if style
                    .font_style
                    .contains(syntect::highlighting::FontStyle::UNDERLINE)
                {
                    span_style.underline = true;
                }
                spans.push(StyledSpan {
                    text: text.to_string(),
                    style: span_style,
                });
            }
            return spans;
        }
        vec![StyledSpan::plain(line)]
    }

    pub fn parse_ansi_line(&self, line: &str) -> Vec<StyledSpan> {
        let mut spans = Vec::new();
        let mut current = TextStyle::default();
        let mut buf = String::new();
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' {
                match chars.peek().copied() {
                    Some('[') => {
                        chars.next();
                        push_span(&mut spans, &mut buf, current);
                        let mut params = String::new();
                        let mut final_byte = None;
                        for next in chars.by_ref() {
                            if ('@'..='~').contains(&next) {
                                final_byte = Some(next);
                                break;
                            }
                            params.push(next);
                        }
                        if final_byte == Some('m') {
                            self.apply_sgr(&mut current, &params);
                        }
                    }
                    Some(']') | Some('P') | Some('_') | Some('^') | Some('X') => {
                        chars.next();
                        push_span(&mut spans, &mut buf, current);
                        skip_escape_string(&mut chars);
                    }
                    Some(_) => {
                        chars.next();
                        push_span(&mut spans, &mut buf, current);
                    }
                    None => {}
                }
            } else if ch.is_control() && ch != '\n' && ch != '\t' {
                push_span(&mut spans, &mut buf, current);
                spans.push(StyledSpan {
                    text: caret_repr(ch),
                    style: current,
                });
            } else {
                buf.push(ch);
            }
        }
        push_span(&mut spans, &mut buf, current);
        if spans.is_empty() {
            spans.push(StyledSpan::plain(String::new()));
        }
        spans
    }

    pub fn strip_ansi_sequences(bytes: &[u8]) -> Cow<'_, [u8]> {
        if !bytes.contains(&0x1b) {
            return Cow::Borrowed(bytes);
        }

        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0usize;
        while i < bytes.len() {
            if bytes[i] != 0x1b {
                out.push(bytes[i]);
                i += 1;
                continue;
            }

            i += 1;
            if i >= bytes.len() {
                break;
            }

            match bytes[i] {
                b'[' => {
                    i += 1;
                    while i < bytes.len() {
                        let byte = bytes[i];
                        i += 1;
                        if (0x40..=0x7e).contains(&byte) {
                            break;
                        }
                    }
                }
                b']' | b'P' | b'_' | b'^' | b'X' => {
                    i += 1;
                    while i < bytes.len() {
                        if bytes[i] == 0x07 {
                            i += 1;
                            break;
                        }
                        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                }
                _ => {
                    i += 1;
                }
            }
        }

        Cow::Owned(out)
    }

    fn apply_sgr(&self, current: &mut TextStyle, params: &str) {
        if params.is_empty() {
            *current = TextStyle::default();
            return;
        }

        let mut codes = params.split(';').peekable();
        while let Some(code) = codes.next() {
            let Ok(code) = code.parse::<u16>() else {
                continue;
            };
            match code {
                0 => *current = TextStyle::default(),
                1 => current.bold = true,
                2 => current.dim = true,
                3 => current.italic = true,
                4 => current.underline = true,
                7 => current.reverse = true,
                22 => {
                    current.bold = false;
                    current.dim = false;
                }
                23 => current.italic = false,
                24 => current.underline = false,
                27 => current.reverse = false,
                30..=37 => current.fg = Some(Rgb::from_ansi_index((code - 30) as u8)),
                39 => current.fg = None,
                40..=47 => current.bg = Some(Rgb::from_ansi_index((code - 40) as u8)),
                49 => current.bg = None,
                90..=97 => current.fg = Some(Rgb::from_ansi_index((code - 90 + 8) as u8)),
                100..=107 => current.bg = Some(Rgb::from_ansi_index((code - 100 + 8) as u8)),
                38 | 48 => {
                    let is_fg = code == 38;
                    let Some(mode) = codes.next() else {
                        break;
                    };
                    let Ok(mode) = mode.parse::<u16>() else {
                        continue;
                    };
                    match mode {
                        5 => {
                            let Some(index) = codes.next() else {
                                break;
                            };
                            if let Ok(index) = index.parse::<u8>() {
                                let color = Rgb::from_ansi_index(index);
                                if is_fg {
                                    current.fg = Some(color);
                                } else {
                                    current.bg = Some(color);
                                }
                            }
                        }
                        2 => {
                            let (Some(r), Some(g), Some(b)) =
                                (codes.next(), codes.next(), codes.next())
                            else {
                                break;
                            };
                            if let (Ok(r), Ok(g), Ok(b)) =
                                (r.parse::<u8>(), g.parse::<u8>(), b.parse::<u8>())
                            {
                                let color = Rgb::new(r, g, b);
                                if is_fg {
                                    current.fg = Some(color);
                                } else {
                                    current.bg = Some(color);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    pub fn search_regex(&self, pattern: &str, ignore_case: bool) -> Result<BytesRegex> {
        let builder = if ignore_case {
            format!("(?i){}", pattern)
        } else {
            pattern.to_string()
        };
        BytesRegex::new(&builder).context("compiling search pattern")
    }
}

fn push_span(spans: &mut Vec<StyledSpan>, buf: &mut String, style: TextStyle) {
    if !buf.is_empty() {
        spans.push(StyledSpan {
            text: std::mem::take(buf),
            style,
        });
    }
}

fn skip_escape_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    let mut saw_escape = false;
    for next in chars.by_ref() {
        if next == '\u{7}' {
            return;
        }
        if saw_escape && next == '\\' {
            return;
        }
        saw_escape = next == '\u{1b}';
    }
}

fn caret_repr(ch: char) -> String {
    if ch == '\u{7f}' {
        "^?".to_string()
    } else if ch.is_ascii_control() {
        format!("^{}", ((ch as u8 & 0x1f) + 0x40) as char)
    } else {
        format!("\\u{{{:x}}}", ch as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sgr_sequences() {
        let engine = SyntaxEngine::new("base16-ocean.dark").unwrap();
        let spans = engine.parse_ansi_line("\u{1b}[31mred\u{1b}[0m plain");
        assert!(spans.iter().any(|s| s.style.fg.is_some()));
        assert!(spans.iter().any(|s| s.text.contains("plain")));
    }

    #[test]
    fn parses_extended_colors_and_strips_escape_sequences() {
        let engine = SyntaxEngine::new("base16-ocean.dark").unwrap();
        let spans = engine.parse_ansi_line(
            "\u{1b}[38;5;196mred\u{1b}[0m \u{1b}[38;2;1;2;3mblue\u{1b}[0m \u{1b}]8;;https://example.com\u{7}link\u{1b}]8;;\u{7}",
        );
        assert_eq!(spans[0].text, "red");
        assert_eq!(spans[0].style.fg, Some(Rgb::from_ansi_index(196)));
        assert!(
            spans
                .iter()
                .any(|span| span.text == "blue" && span.style.fg == Some(Rgb::new(1, 2, 3)))
        );
        assert!(spans.iter().any(|span| span.text == "link"));
        assert!(
            spans
                .iter()
                .all(|span| !span.text.contains("https://example.com"))
        );
    }
}

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
        if let Some(path) = path {
            if let Some(syntax) = self.syntax_for_path(path, bytes) {
                return SyntaxChoice::Named(syntax.name.clone());
            }
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

    pub fn highlight_line(
        &self,
        choice: &SyntaxChoice,
        line: &str,
    ) -> Vec<StyledSpan> {
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
                let mut span_style = TextStyle::default();
                span_style.fg = Some(style.foreground.into());
                if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
                    span_style.bold = true;
                }
                if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
                    span_style.italic = true;
                }
                if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
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
            if ch == '\u{1b}' && chars.peek() == Some(&'[') {
                chars.next();
                if !buf.is_empty() {
                    spans.push(StyledSpan {
                        text: std::mem::take(&mut buf),
                        style: current,
                    });
                }
                let mut params = String::new();
                while let Some(next) = chars.next() {
                    if next == 'm' {
                        self.apply_sgr(&mut current, &params);
                        break;
                    }
                    if next.is_ascii_alphanumeric() || next == ';' {
                        params.push(next);
                    } else {
                        break;
                    }
                }
            } else if ch.is_control() && ch != '\n' && ch != '\t' {
                if !buf.is_empty() {
                    spans.push(StyledSpan {
                        text: std::mem::take(&mut buf),
                        style: current,
                    });
                }
                spans.push(StyledSpan {
                    text: caret_repr(ch),
                    style: current,
                });
            } else {
                buf.push(ch);
            }
        }
        if !buf.is_empty() {
            spans.push(StyledSpan { text: buf, style: current });
        }
        if spans.is_empty() {
            spans.push(StyledSpan::plain(String::new()));
        }
        spans
    }

    fn apply_sgr(&self, current: &mut TextStyle, params: &str) {
        if params.is_empty() || params == "0" {
            *current = TextStyle::default();
            return;
        }
        let mut it = params.split(';').peekable();
        while let Some(code) = it.next() {
            match code {
                "1" => current.bold = true,
                "2" => current.dim = true,
                "3" => current.italic = true,
                "4" => current.underline = true,
                "7" => current.reverse = true,
                "22" => {
                    current.bold = false;
                    current.dim = false;
                }
                "23" => current.italic = false,
                "24" => current.underline = false,
                "27" => current.reverse = false,
                "30" => current.fg = Some(Rgb { r: 0, g: 0, b: 0 }),
                "31" => current.fg = Some(Rgb { r: 205, g: 49, b: 49 }),
                "32" => current.fg = Some(Rgb { r: 13, g: 188, b: 121 }),
                "33" => current.fg = Some(Rgb { r: 229, g: 229, b: 16 }),
                "34" => current.fg = Some(Rgb { r: 36, g: 114, b: 200 }),
                "35" => current.fg = Some(Rgb { r: 188, g: 63, b: 188 }),
                "36" => current.fg = Some(Rgb { r: 17, g: 168, b: 205 }),
                "37" => current.fg = Some(Rgb { r: 229, g: 229, b: 229 }),
                "39" => current.fg = None,
                "40" => current.bg = Some(Rgb { r: 0, g: 0, b: 0 }),
                "41" => current.bg = Some(Rgb { r: 205, g: 49, b: 49 }),
                "42" => current.bg = Some(Rgb { r: 13, g: 188, b: 121 }),
                "43" => current.bg = Some(Rgb { r: 229, g: 229, b: 16 }),
                "44" => current.bg = Some(Rgb { r: 36, g: 114, b: 200 }),
                "45" => current.bg = Some(Rgb { r: 188, g: 63, b: 188 }),
                "46" => current.bg = Some(Rgb { r: 17, g: 168, b: 205 }),
                "47" => current.bg = Some(Rgb { r: 229, g: 229, b: 229 }),
                "49" => current.bg = None,
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

fn caret_repr(ch: char) -> String {
    let byte = ch as u32 as u8;
    if byte == 0x7f {
        "^?".to_string()
    } else {
        format!("^{}", ((byte & 0x1f) + 0x40) as char)
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
}

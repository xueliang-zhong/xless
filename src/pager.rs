use std::io;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;

use crate::config::Config;
use crate::document::DocumentSet;
use crate::highlight::SyntaxEngine;
use crate::render::{TerminalSession, render};
use unicode_width::UnicodeWidthChar;

pub struct Pager {
    config: Config,
    docs: DocumentSet,
    engine: SyntaxEngine,
    top_line: usize,
    prompt: PromptMode,
    status: String,
    last_search: Option<(String, bool)>,
    quit: bool,
}

#[derive(Debug, Clone)]
enum PromptMode {
    Normal,
    Search { input: String, backward: bool },
    Help,
}

impl Pager {
    pub fn new(config: Config, docs: DocumentSet) -> Result<Self> {
        let engine = SyntaxEngine::new(&config.theme)?;
        Ok(Self {
            config,
            docs,
            engine,
            top_line: 0,
            prompt: PromptMode::Normal,
            status: String::new(),
            last_search: None,
            quit: false,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let _term = TerminalSession::enter(self.config.no_init)?;
        let mut out = io::stdout();
        if self.config.quit_if_one_screen && self.fits_screen()? {
            render(
                &mut out,
                &self.docs,
                &self.config,
                &self.engine,
                self.top_line,
                None,
                "",
            )?;
            return Ok(());
        }
        let mut last_tick = Instant::now();
        loop {
            let prompt = match &self.prompt {
                PromptMode::Normal => None,
                PromptMode::Search { input, backward } => {
                    self.status = if *backward {
                        format!("?{}", input)
                    } else {
                        format!("/{}", input)
                    };
                    Some(self.status.as_str())
                }
                PromptMode::Help => Some(HELP_TEXT),
            };
            render(
                &mut out,
                &self.docs,
                &self.config,
                &self.engine,
                self.top_line,
                prompt,
                &self.status,
            )?;

            if self.quit {
                break;
            }

            let timeout = if self.config.follow {
                Duration::from_millis(150)
            } else {
                Duration::from_millis(50)
            };
            if event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key)?,
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            } else if self.config.follow && last_tick.elapsed() > Duration::from_millis(500) {
                self.reload_if_possible()?;
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    fn fits_screen(&self) -> Result<bool> {
        let (width, height) = terminal::size()?;
        let mut rows = 0usize;
        let limit = if self.config.status_bar {
            height.saturating_sub(1)
        } else {
            height
        } as usize;
        let mut global = 0usize;
        while global < self.docs.line_count() && rows <= limit {
            let view = self.docs.line(global).context("missing line")?;
            let line_rows = estimate_rows(
                &view.text,
                width as usize,
                self.config.chop_long_lines,
                self.config.tab_width,
            );
            rows += line_rows.max(1);
            if rows > limit {
                return Ok(false);
            }
            global += 1;
        }
        Ok(true)
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        let prompt = std::mem::replace(&mut self.prompt, PromptMode::Normal);
        match prompt {
            PromptMode::Normal => {
                self.handle_normal_key(key)?;
            }
            PromptMode::Search {
                mut input,
                backward,
            } => {
                let done = self.handle_search_key(key, &mut input, backward)?;
                if done {
                    self.prompt = PromptMode::Normal;
                } else {
                    self.prompt = PromptMode::Search { input, backward };
                }
            }
            PromptMode::Help => {
                self.prompt = PromptMode::Normal;
            }
        }
        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Enter => self.scroll(1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_up(1),
            KeyCode::Char('f') | KeyCode::PageDown | KeyCode::Char(' ') => self.page_down(),
            KeyCode::Char('b') | KeyCode::PageUp => self.page_up(),
            KeyCode::Char('d') => self.page_down_half(),
            KeyCode::Char('u') => self.page_up_half(),
            KeyCode::Char('g') => self.top_line = 0,
            KeyCode::Char('G') => self.bottom(),
            KeyCode::Char('/') => {
                self.prompt = PromptMode::Search {
                    input: String::new(),
                    backward: false,
                }
            }
            KeyCode::Char('?') => {
                self.prompt = PromptMode::Search {
                    input: String::new(),
                    backward: true,
                }
            }
            KeyCode::Char('n') => self.repeat_search(false)?,
            KeyCode::Char('N') => self.repeat_search(true)?,
            KeyCode::Char('h') => self.prompt = PromptMode::Help,
            KeyCode::Char('r') => self.reload_if_possible()?,
            KeyCode::Char('v') => self.open_in_editor()?,
            KeyCode::Char('F') => self.config.follow = !self.config.follow,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            _ => {}
        }
        Ok(())
    }

    fn handle_search_key(
        &mut self,
        key: KeyEvent,
        input: &mut String,
        backward: bool,
    ) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.status.clear();
                return Ok(true);
            }
            KeyCode::Enter => {
                let query = input.clone();
                self.perform_search(&query, backward)?;
                self.status.clear();
                return Ok(true);
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    input.push(c);
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn perform_search(&mut self, pattern: &str, backward: bool) -> Result<()> {
        if pattern.is_empty() {
            return Ok(());
        }
        let regex = self
            .engine
            .search_regex(pattern, self.config.ignore_case)
            .context("search")?;
        self.last_search = Some((pattern.to_string(), backward));
        if backward {
            for idx in (0..self.top_line).rev() {
                if let Some(view) = self.docs.line(idx) {
                    if regex.is_match(&view.bytes) {
                        self.top_line = idx;
                        return Ok(());
                    }
                }
            }
            if self.config.wrap_search {
                for idx in (self.top_line..self.docs.line_count()).rev() {
                    if let Some(view) = self.docs.line(idx) {
                        if regex.is_match(&view.bytes) {
                            self.top_line = idx;
                            return Ok(());
                        }
                    }
                }
            }
        } else {
            for idx in self.top_line + 1..self.docs.line_count() {
                if let Some(view) = self.docs.line(idx) {
                    if regex.is_match(&view.bytes) {
                        self.top_line = idx;
                        return Ok(());
                    }
                }
            }
            if self.config.wrap_search {
                for idx in 0..=self.top_line.min(self.docs.line_count().saturating_sub(1)) {
                    if let Some(view) = self.docs.line(idx) {
                        if regex.is_match(&view.bytes) {
                            self.top_line = idx;
                            return Ok(());
                        }
                    }
                }
            }
        }
        self.status = "pattern not found".to_string();
        Ok(())
    }

    fn repeat_search(&mut self, backward: bool) -> Result<()> {
        if let Some((pattern, last_backward)) = &self.last_search {
            let pattern = pattern.clone();
            let target_backward = if backward {
                !*last_backward
            } else {
                *last_backward
            };
            self.perform_search(&pattern, target_backward)?;
        }
        Ok(())
    }

    fn page_down(&mut self) {
        self.scroll_by_rows(true, 0.9);
    }

    fn page_up(&mut self) {
        self.scroll_by_rows(false, 0.9);
    }

    fn page_down_half(&mut self) {
        self.scroll_by_rows(true, 0.5);
    }

    fn page_up_half(&mut self) {
        self.scroll_by_rows(false, 0.5);
    }

    fn scroll(&mut self, lines: usize) {
        self.top_line = (self.top_line + lines).min(self.docs.line_count().saturating_sub(1));
    }

    fn scroll_up(&mut self, lines: usize) {
        self.top_line = self.top_line.saturating_sub(lines);
    }

    fn bottom(&mut self) {
        self.top_line = self.docs.line_count().saturating_sub(1);
    }

    fn scroll_by_rows(&mut self, forward: bool, fraction: f64) {
        let screen = terminal::size().unwrap_or((80, 24));
        let limit = if self.config.status_bar {
            screen.1.saturating_sub(1)
        } else {
            screen.1
        } as usize;
        let steps = ((limit as f64) * fraction).max(1.0) as usize;
        if forward {
            self.top_line = advance_lines(
                &self.docs,
                self.top_line,
                steps,
                screen.0 as usize,
                &self.config,
            );
        } else {
            self.top_line = rewind_lines(
                &self.docs,
                self.top_line,
                steps,
                screen.0 as usize,
                &self.config,
            );
        }
    }

    fn reload_if_possible(&mut self) -> Result<()> {
        let docs = self.docs.reloaded(&self.config)?;
        self.docs = docs;
        self.top_line = self.top_line.min(self.docs.line_count().saturating_sub(1));
        Ok(())
    }

    fn open_in_editor(&mut self) -> Result<()> {
        let current = self.docs.line(self.top_line).context("no current line")?;
        let doc = self.docs.document(current.doc).context("missing doc")?;
        let Some(path) = &doc.path else {
            self.status = "stdin cannot be edited".to_string();
            return Ok(());
        };
        terminal::disable_raw_mode()?;
        let _guard = RawModeGuard;
        let mut editor = command_from_string(&self.config.editor)?;
        editor.arg(format!("+{}", current.local_line + 1)).arg(path);
        let status = editor.status().context("launching editor")?;
        if !status.success() {
            self.status = format!("editor exited with {}", status);
        }
        Ok(())
    }
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::enable_raw_mode();
    }
}

fn command_from_string(cmd: &str) -> Result<Command> {
    let parts =
        shell_words::split(cmd).with_context(|| format!("parsing editor command {cmd:?}"))?;
    let mut parts = parts.into_iter();
    let mut command = Command::new(parts.next().unwrap_or_else(|| "vim".to_string()));
    for part in parts {
        command.arg(part);
    }
    Ok(command)
}

fn estimate_rows(text: &str, width: usize, chop: bool, tab_width: usize) -> usize {
    if width == 0 {
        return 1;
    }
    let mut rows = 1usize;
    let mut col = 0usize;
    for ch in text.chars() {
        let rendered = if ch == '\t' {
            let spaces = tab_width.max(1) - (col % tab_width.max(1));
            spaces
        } else if ch.is_control() {
            if ch == '\u{7f}' { 2 } else { 2 }
        } else {
            UnicodeWidthChar::width(ch).unwrap_or(1)
        };
        if col + rendered > width {
            if chop {
                continue;
            }
            rows += 1;
            col = 0;
        }
        col += rendered;
    }
    rows
}

fn advance_lines(
    docs: &DocumentSet,
    start: usize,
    steps: usize,
    width: usize,
    config: &Config,
) -> usize {
    let mut rows = 0usize;
    let mut idx = start;
    while idx < docs.line_count() && rows < steps {
        if let Some(view) = docs.line(idx) {
            rows += estimate_rows(&view.text, width, config.chop_long_lines, config.tab_width);
        }
        idx += 1;
    }
    idx.saturating_sub(1)
        .min(docs.line_count().saturating_sub(1))
}

fn rewind_lines(
    docs: &DocumentSet,
    start: usize,
    steps: usize,
    width: usize,
    config: &Config,
) -> usize {
    let mut rows = 0usize;
    let mut idx = start;
    while idx > 0 && rows < steps {
        idx -= 1;
        if let Some(view) = docs.line(idx) {
            rows += estimate_rows(&view.text, width, config.chop_long_lines, config.tab_width);
        }
    }
    idx
}

const HELP_TEXT: &str = "q quit  j/k scroll  f/b page  / search  n/N next/prev  v editor  r reload";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::document::DocumentSet;
    use tempfile::NamedTempFile;

    fn sample_set() -> DocumentSet {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "alpha\nbeta\ngamma\nbeta\n").unwrap();
        DocumentSet::from_paths(&[tmp.path().to_path_buf()], &Config::default()).unwrap()
    }

    #[test]
    fn search_repeats_follow_original_direction() {
        let mut pager = Pager::new(Config::default(), sample_set()).unwrap();
        pager.perform_search("beta", false).unwrap();
        assert_eq!(pager.top_line, 1);
        pager.repeat_search(false).unwrap();
        assert_eq!(pager.top_line, 3);
        pager.repeat_search(true).unwrap();
        assert_eq!(pager.top_line, 1);
    }

    #[test]
    fn backward_search_repeats_backward_first() {
        let mut pager = Pager::new(Config::default(), sample_set()).unwrap();
        pager.top_line = 3;
        pager.perform_search("beta", true).unwrap();
        assert_eq!(pager.top_line, 1);
        pager.repeat_search(false).unwrap();
        assert_eq!(pager.top_line, 3);
    }

    #[test]
    fn line_scrolling_moves_within_bounds() {
        let docs = sample_set();
        assert_eq!(advance_lines(&docs, 0, 1, 80, &Config::default()), 0);
        assert_eq!(rewind_lines(&docs, 1, 1, 80, &Config::default()), 0);
    }

    #[test]
    fn parses_editor_command_with_quoted_arguments() {
        let command = command_from_string("nvim -u 'NORC profile'").unwrap();
        assert_eq!(command.get_program(), "nvim");
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, vec!["-u", "NORC profile"]);
    }
}

use std::borrow::Cow;
use std::collections::HashMap;
use std::io;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;

use crate::config::Config;
use crate::document::DocumentSet;
use crate::highlight::SyntaxEngine;
use crate::render::{RenderContext, TerminalSession, render};
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupCommand {
    Pattern { pattern: String, backward: bool },
    Line(usize),
    Bottom,
    Follow,
}

pub struct Pager {
    config: Config,
    docs: DocumentSet,
    engine: SyntaxEngine,
    startup: Vec<StartupCommand>,
    top_line: usize,
    horizontal_offset: usize,
    count_buffer: String,
    prompt: PromptMode,
    status: String,
    last_search: Option<SearchState>,
    marks: HashMap<char, usize>,
    quit: bool,
}

#[derive(Debug)]
struct SearchState {
    pattern: String,
    backward: bool,
    regex: regex::bytes::Regex,
}

#[derive(Debug, Clone)]
enum PromptMode {
    Normal,
    Search { input: String, backward: bool },
    Mark { action: MarkAction },
    Command { input: String },
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkAction {
    SetFirst,
    SetLast,
    Jump,
}

impl Pager {
    pub fn new(config: Config, docs: DocumentSet, startup: Vec<StartupCommand>) -> Result<Self> {
        let engine = SyntaxEngine::new(&config.theme)?;
        Ok(Self {
            config,
            docs,
            engine,
            startup,
            top_line: 0,
            horizontal_offset: 0,
            count_buffer: String::new(),
            prompt: PromptMode::Normal,
            status: String::new(),
            last_search: None,
            marks: HashMap::new(),
            quit: false,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.apply_startup_commands()?;
        let _term = TerminalSession::enter(self.config.no_init)?;
        if self.config.quit_if_one_screen && self.fits_screen()? {
            let mut out = io::stdout();
            let ctx = RenderContext {
                docs: &self.docs,
                config: &self.config,
                engine: &self.engine,
                horizontal_offset: self.horizontal_offset,
            };
            render(&mut out, &ctx, self.top_line, None, "")?;
            return Ok(());
        }
        let mut out = io::stdout();
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
                PromptMode::Mark { action } => Some(match action {
                    MarkAction::SetFirst => "mark: set first",
                    MarkAction::SetLast => "mark: set last",
                    MarkAction::Jump => "mark: jump",
                }),
                PromptMode::Command { input } => Some(if input.is_empty() {
                    ":"
                } else {
                    input.as_str()
                }),
                PromptMode::Help => Some(HELP_TEXT),
            };
            let ctx = RenderContext {
                docs: &self.docs,
                config: &self.config,
                engine: &self.engine,
                horizontal_offset: self.horizontal_offset,
            };
            render(&mut out, &ctx, self.top_line, prompt, &self.status)?;

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

    fn apply_startup_commands(&mut self) -> Result<()> {
        let startup = std::mem::take(&mut self.startup);
        for command in startup {
            match command {
                StartupCommand::Pattern { pattern, backward } => {
                    self.perform_startup_search(&pattern, backward)?;
                }
                StartupCommand::Line(line) => {
                    self.top_line = line
                        .saturating_sub(1)
                        .min(self.docs.line_count().saturating_sub(1));
                }
                StartupCommand::Bottom => self.bottom(),
                StartupCommand::Follow => self.config.follow = true,
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
                self.config.raw_control_chars,
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
            PromptMode::Mark { action } => {
                let done = self.handle_mark_key(key, action)?;
                if done {
                    self.prompt = PromptMode::Normal;
                } else {
                    self.prompt = PromptMode::Mark { action };
                }
            }
            PromptMode::Command { mut input } => {
                let done = self.handle_command_key(key, &mut input)?;
                if done {
                    self.prompt = PromptMode::Normal;
                } else {
                    self.prompt = PromptMode::Command { input };
                }
            }
            PromptMode::Help => {
                self.prompt = PromptMode::Normal;
            }
        }
        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.modifiers.is_empty()
            && let KeyCode::Char(c) = key.code
            && c.is_ascii_digit()
        {
            if c != '0' || !self.count_buffer.is_empty() {
                self.count_buffer.push(c);
            }
            return Ok(());
        }
        let count = self.take_count();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll(count.unwrap_or(1))
            }
            KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_up(count.unwrap_or(1))
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll(count.unwrap_or(1))
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_up(count.unwrap_or(1))
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_down_count(count)
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_up_count(count)
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_down_half_count(count)
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_up_half_count(count)
            }
            KeyCode::Right => self.scroll_right_count(count),
            KeyCode::Left => self.scroll_left_count(count),
            KeyCode::Home => self.reset_horizontal_offset(),
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Enter => self.scroll(count.unwrap_or(1)),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_up(count.unwrap_or(1)),
            KeyCode::Char('f') | KeyCode::PageDown | KeyCode::Char(' ') => {
                self.page_down_count(count)
            }
            KeyCode::Char('b') | KeyCode::PageUp => self.page_up_count(count),
            KeyCode::Char('d') => self.page_down_half_count(count),
            KeyCode::Char('u') => self.page_up_half_count(count),
            KeyCode::Char('g') => {
                self.top_line = count
                    .map_or(0, |line| line.saturating_sub(1))
                    .min(self.docs.line_count().saturating_sub(1));
            }
            KeyCode::Char('G') => {
                if let Some(line) = count {
                    self.top_line = line
                        .saturating_sub(1)
                        .min(self.docs.line_count().saturating_sub(1));
                } else {
                    self.bottom();
                }
            }
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
            KeyCode::Char('m') => {
                self.prompt = PromptMode::Mark {
                    action: MarkAction::SetFirst,
                }
            }
            KeyCode::Char(':') => {
                self.prompt = PromptMode::Command {
                    input: String::new(),
                }
            }
            KeyCode::Char('M') => {
                self.prompt = PromptMode::Mark {
                    action: MarkAction::SetLast,
                }
            }
            KeyCode::Char('\'') => {
                self.prompt = PromptMode::Mark {
                    action: MarkAction::Jump,
                }
            }
            KeyCode::Char('n') => self.repeat_search(false)?,
            KeyCode::Char('N') => self.repeat_search(true)?,
            KeyCode::Char('h') => self.prompt = PromptMode::Help,
            KeyCode::Char('r') | KeyCode::Char('R') => self.reload_if_possible()?,
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

    fn handle_mark_key(&mut self, key: KeyEvent, action: MarkAction) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.status.clear();
                return Ok(true);
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(false);
                }
                match action {
                    MarkAction::SetFirst => self.set_mark(c, self.top_line),
                    MarkAction::SetLast => {
                        let line = self.visible_last_line().unwrap_or(self.top_line);
                        self.set_mark(c, line);
                    }
                    MarkAction::Jump => self.jump_to_mark(c),
                }
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_command_key(&mut self, key: KeyEvent, input: &mut String) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.status.clear();
                return Ok(true);
            }
            KeyCode::Enter => {
                let command = input.clone();
                self.execute_command(&command)?;
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
        self.perform_search_with_origin(pattern, backward, false)
    }

    fn perform_startup_search(&mut self, pattern: &str, backward: bool) -> Result<()> {
        self.perform_search_with_origin(pattern, backward, true)
    }

    fn perform_search_with_origin(
        &mut self,
        pattern: &str,
        backward: bool,
        include_current_line: bool,
    ) -> Result<()> {
        if pattern.is_empty() {
            return Ok(());
        }
        let regex = self
            .engine
            .search_regex(
                pattern,
                self.config.ignore_case,
                self.config.ignore_case_always,
            )
            .context("search")?;
        let found = self.locate_match(&regex, backward, include_current_line);
        if let Some(line) = found {
            self.top_line = line;
        } else {
            self.status = "pattern not found".to_string();
        }
        self.last_search = Some(SearchState {
            pattern: pattern.to_string(),
            backward,
            regex,
        });
        Ok(())
    }

    fn matches_search(&self, regex: &regex::bytes::Regex, bytes: &[u8]) -> bool {
        let bytes = if self.config.raw_control_chars {
            Cow::Borrowed(bytes)
        } else {
            SyntaxEngine::strip_ansi_sequences(bytes)
        };
        regex.is_match(bytes.as_ref())
    }

    fn locate_match(
        &self,
        regex: &regex::bytes::Regex,
        backward: bool,
        include_current_line: bool,
    ) -> Option<usize> {
        if backward {
            self.find_previous_match(regex, self.top_line, false)
                .or_else(|| {
                    if self.config.wrap_search {
                        self.find_previous_match(regex, self.docs.line_count(), true)
                    } else {
                        None
                    }
                })
        } else {
            self.find_forward_match(regex, self.top_line, include_current_line)
                .or_else(|| {
                    if self.config.wrap_search {
                        self.find_forward_match(regex, 0, true)
                    } else {
                        None
                    }
                })
        }
    }

    fn find_forward_match(
        &self,
        regex: &regex::bytes::Regex,
        start_line: usize,
        include_start: bool,
    ) -> Option<usize> {
        let start = if include_start {
            start_line
        } else {
            start_line.saturating_add(1)
        };
        for idx in start..self.docs.line_count() {
            if let Some(view) = self.docs.line(idx)
                && self.matches_search(regex, view.bytes.as_ref())
            {
                return Some(idx);
            }
        }
        None
    }

    fn find_previous_match(
        &self,
        regex: &regex::bytes::Regex,
        start_line: usize,
        include_start: bool,
    ) -> Option<usize> {
        if self.docs.line_count() == 0 {
            return None;
        }
        let mut idx = if include_start {
            start_line.min(self.docs.line_count().saturating_sub(1))
        } else if start_line == 0 {
            return None;
        } else {
            start_line - 1
        };
        loop {
            if let Some(view) = self.docs.line(idx)
                && self.matches_search(regex, view.bytes.as_ref())
            {
                return Some(idx);
            }
            if idx == 0 {
                break;
            }
            idx -= 1;
        }
        None
    }

    fn repeat_search(&mut self, backward: bool) -> Result<()> {
        if let Some(state) = self.last_search.take() {
            let target_backward = if backward {
                !state.backward
            } else {
                state.backward
            };
            let found = self.locate_match(&state.regex, target_backward, false);
            if let Some(line) = found {
                self.top_line = line;
            } else {
                self.status = "pattern not found".to_string();
            }
            self.last_search = Some(SearchState {
                pattern: state.pattern,
                backward: target_backward,
                regex: state.regex,
            });
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

    fn page_down_count(&mut self, count: Option<usize>) {
        if let Some(lines) = count {
            self.scroll(lines);
        } else {
            self.page_down();
        }
    }

    fn page_up_count(&mut self, count: Option<usize>) {
        if let Some(lines) = count {
            self.scroll_up(lines);
        } else {
            self.page_up();
        }
    }

    fn page_down_half_count(&mut self, count: Option<usize>) {
        if let Some(lines) = count {
            self.scroll(lines);
        } else {
            self.page_down_half();
        }
    }

    fn page_up_half_count(&mut self, count: Option<usize>) {
        if let Some(lines) = count {
            self.scroll_up(lines);
        } else {
            self.page_up_half();
        }
    }

    fn scroll_right_count(&mut self, count: Option<usize>) {
        let step = self.horizontal_step().saturating_mul(count.unwrap_or(1));
        self.scroll_right(step);
    }

    fn scroll_left_count(&mut self, count: Option<usize>) {
        let step = self.horizontal_step().saturating_mul(count.unwrap_or(1));
        self.scroll_left(step);
    }

    fn horizontal_step(&self) -> usize {
        let (width, _) = terminal::size().unwrap_or((80, 24));
        usize::max(1, width as usize / 2)
    }

    fn scroll_right(&mut self, cols: usize) {
        if self.config.chop_long_lines {
            self.horizontal_offset = self.horizontal_offset.saturating_add(cols);
        }
    }

    fn scroll_left(&mut self, cols: usize) {
        if self.config.chop_long_lines {
            self.horizontal_offset = self.horizontal_offset.saturating_sub(cols);
        }
    }

    fn reset_horizontal_offset(&mut self) {
        self.horizontal_offset = 0;
    }

    fn bottom(&mut self) {
        let screen = terminal::size().unwrap_or((80, 24));
        self.top_line = bottom_line_for_screen(
            &self.docs,
            screen.0 as usize,
            screen.1 as usize,
            &self.config,
        );
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

    fn set_mark(&mut self, mark: char, line: usize) {
        self.marks.insert(mark, line);
        self.status = format!("set mark {mark}");
    }

    fn jump_to_mark(&mut self, mark: char) {
        if let Some(line) = self.marks.get(&mark).copied() {
            self.top_line = line.min(self.docs.line_count().saturating_sub(1));
            self.status = format!("jumped to mark {mark}");
        } else {
            self.status = format!("mark {mark} not set");
        }
    }

    fn execute_command(&mut self, command: &str) -> Result<()> {
        let command = command.trim();
        if command.is_empty() {
            return Ok(());
        }

        match command {
            "n" => self.jump_to_adjacent_file(1),
            "p" => self.jump_to_adjacent_file(-1),
            "q" => self.quit = true,
            _ => {
                self.status = format!("unknown command :{command}");
            }
        }
        Ok(())
    }

    fn jump_to_adjacent_file(&mut self, delta: isize) {
        let Some(current_doc) = self.docs.document_index_at_line(self.top_line) else {
            self.status = "no current file".to_string();
            return;
        };

        let Some(next_doc) = current_doc.checked_add_signed(delta) else {
            self.status = if delta.is_negative() {
                "no previous file".to_string()
            } else {
                "no next file".to_string()
            };
            return;
        };

        let Some(target_line) = self.docs.first_visible_line_for_document(next_doc) else {
            self.status = if delta.is_negative() {
                "no previous file".to_string()
            } else {
                "no next file".to_string()
            };
            return;
        };

        self.top_line = target_line;
        let file_name = self
            .docs
            .document(next_doc)
            .map(|doc| doc.name.as_str())
            .unwrap_or("<unknown>");
        self.status = if delta.is_negative() {
            format!("previous file {file_name}")
        } else {
            format!("next file {file_name}")
        };
    }

    fn take_count(&mut self) -> Option<usize> {
        if self.count_buffer.is_empty() {
            return None;
        }

        let mut value = 0usize;
        for ch in self.count_buffer.chars() {
            let digit = ch.to_digit(10)? as usize;
            value = value.saturating_mul(10).saturating_add(digit);
        }
        self.count_buffer.clear();
        Some(value)
    }

    fn visible_last_line(&self) -> Option<usize> {
        let (width, height) = terminal::size().ok()?;
        Some(visible_last_line_for_screen(
            &self.docs,
            self.top_line,
            width as usize,
            height as usize,
            &self.config,
        ))
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

fn estimate_rows(
    text: &str,
    width: usize,
    chop: bool,
    tab_width: usize,
    raw_control_chars: bool,
) -> usize {
    if width == 0 {
        return 1;
    }
    let tab_width = tab_width.max(1);
    let mut rows = 1usize;
    let mut col = 0usize;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if !raw_control_chars && ch == '\u{1b}' {
            skip_ansi_escape(&mut chars);
            continue;
        }
        let rendered = if ch == '\t' {
            tab_width - (col % tab_width)
        } else if ch.is_control() {
            2
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

fn skip_ansi_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    match chars.peek().copied() {
        Some('[') => {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
        }
        Some(']') | Some('P') | Some('_') | Some('^') | Some('X') => {
            chars.next();
            let mut saw_escape = false;
            for next in chars.by_ref() {
                if next == '\u{7}' {
                    break;
                }
                if saw_escape && next == '\\' {
                    break;
                }
                saw_escape = next == '\u{1b}';
            }
        }
        Some(_) => {
            chars.next();
        }
        None => {}
    }
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
            rows += estimate_rows(
                &view.text,
                width,
                config.chop_long_lines,
                config.tab_width,
                config.raw_control_chars,
            );
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
            rows += estimate_rows(
                &view.text,
                width,
                config.chop_long_lines,
                config.tab_width,
                config.raw_control_chars,
            );
        }
    }
    idx
}

const HELP_TEXT: &str = "q quit  j/k scroll  f/b page  / search  n/N next/prev  :n/:p files  m/M mark  ' jump  v editor  r/R reload";

fn bottom_line_for_screen(
    docs: &DocumentSet,
    width: usize,
    height: usize,
    config: &Config,
) -> usize {
    if docs.line_count() == 0 {
        return 0;
    }

    let limit = if config.status_bar {
        height.saturating_sub(1)
    } else {
        height
    };
    if limit == 0 {
        return docs.line_count().saturating_sub(1);
    }

    let mut used_rows = 0usize;
    let mut start = docs.line_count().saturating_sub(1);
    for idx in (0..docs.line_count()).rev() {
        let Some(view) = docs.line(idx) else {
            continue;
        };
        let rows = estimate_rows(
            &view.text,
            width,
            config.chop_long_lines,
            config.tab_width,
            config.raw_control_chars,
        )
        .max(1);
        if used_rows + rows > limit {
            break;
        }
        used_rows += rows;
        start = idx;
        if used_rows == limit {
            break;
        }
    }
    start
}

fn visible_last_line_for_screen(
    docs: &DocumentSet,
    top_line: usize,
    width: usize,
    height: usize,
    config: &Config,
) -> usize {
    if docs.line_count() == 0 {
        return 0;
    }

    let limit = if config.status_bar {
        height.saturating_sub(1)
    } else {
        height
    };
    if limit == 0 {
        return top_line.min(docs.line_count().saturating_sub(1));
    }

    let mut used_rows = 0usize;
    let mut last = top_line.min(docs.line_count().saturating_sub(1));
    for idx in top_line.min(docs.line_count())..docs.line_count() {
        let Some(view) = docs.line(idx) else {
            continue;
        };
        let rows = estimate_rows(
            &view.text,
            width,
            config.chop_long_lines,
            config.tab_width,
            config.raw_control_chars,
        )
        .max(1);
        if used_rows + rows > limit {
            break;
        }
        used_rows += rows;
        last = idx;
        if used_rows == limit {
            break;
        }
    }
    last
}

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
        let mut pager = Pager::new(Config::default(), sample_set(), Vec::new()).unwrap();
        pager.perform_search("beta", false).unwrap();
        assert_eq!(pager.top_line, 1);
        pager.repeat_search(false).unwrap();
        assert_eq!(pager.top_line, 3);
        pager.repeat_search(true).unwrap();
        assert_eq!(pager.top_line, 1);
    }

    #[test]
    fn backward_search_repeats_backward_first() {
        let mut pager = Pager::new(Config::default(), sample_set(), Vec::new()).unwrap();
        pager.top_line = 3;
        pager.perform_search("beta", true).unwrap();
        assert_eq!(pager.top_line, 1);
        pager.repeat_search(false).unwrap();
        assert_eq!(pager.top_line, 3);
    }

    #[test]
    fn command_prompt_moves_between_files() {
        let tmp = tempfile::tempdir().unwrap();
        let first = tmp.path().join("first.rs");
        let second = tmp.path().join("second.rs");
        std::fs::write(&first, "alpha\n").unwrap();
        std::fs::write(&second, "beta\n").unwrap();
        let docs = DocumentSet::from_paths(&[first, second], &Config::default()).unwrap();
        let mut pager = Pager::new(Config::default(), docs, Vec::new()).unwrap();

        pager.execute_command("n").unwrap();
        assert_eq!(pager.top_line, 3);
        assert!(pager.status.contains("next file"));

        pager.execute_command("p").unwrap();
        assert_eq!(pager.top_line, 1);
        assert!(pager.status.contains("previous file"));
    }

    #[test]
    fn line_scrolling_moves_within_bounds() {
        let docs = sample_set();
        assert_eq!(advance_lines(&docs, 0, 1, 80, &Config::default()), 0);
        assert_eq!(rewind_lines(&docs, 1, 1, 80, &Config::default()), 0);
    }

    #[test]
    fn bottom_position_shows_last_screenful() {
        let docs = sample_set();
        assert_eq!(bottom_line_for_screen(&docs, 80, 3, &Config::default()), 2);
    }

    #[test]
    fn row_estimation_ignores_ansi_sequences_when_not_raw() {
        let text = "\u{1b}[31mab\u{1b}[0mcd";
        assert_eq!(estimate_rows(text, 3, false, 4, false), 2);
        assert!(estimate_rows(text, 3, false, 4, true) > 2);
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

    #[test]
    fn search_ignores_ansi_sequences_by_default() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "plain\nab\x1b[31mc\x1b[0md\n").unwrap();
        let docs =
            DocumentSet::from_paths(&[tmp.path().to_path_buf()], &Config::default()).unwrap();
        let mut pager = Pager::new(Config::default(), docs, Vec::new()).unwrap();
        pager.perform_search("abcd", false).unwrap();
        assert_eq!(pager.top_line, 1);
    }

    #[test]
    fn ignore_case_search_follows_less_case_rules() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "alpha\nbeta\n").unwrap();
        let docs =
            DocumentSet::from_paths(&[tmp.path().to_path_buf()], &Config::default()).unwrap();
        let config = Config {
            ignore_case: true,
            ..Config::default()
        };
        let mut pager = Pager::new(config, docs, Vec::new()).unwrap();
        pager.perform_search("BETA", false).unwrap();
        assert_eq!(pager.top_line, 0);
        assert_eq!(pager.status, "pattern not found");
    }

    #[test]
    fn ignore_case_always_keeps_matching_uppercase_patterns() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "alpha\nbeta\n").unwrap();
        let docs =
            DocumentSet::from_paths(&[tmp.path().to_path_buf()], &Config::default()).unwrap();
        let config = Config {
            ignore_case: true,
            ignore_case_always: true,
            ..Config::default()
        };
        let mut pager = Pager::new(config, docs, Vec::new()).unwrap();
        pager.perform_search("BETA", false).unwrap();
        assert_eq!(pager.top_line, 1);
    }

    #[test]
    fn startup_commands_apply_before_rendering() {
        let mut pager = Pager::new(
            Config::default(),
            sample_set(),
            vec![StartupCommand::Pattern {
                pattern: "alpha".to_string(),
                backward: false,
            }],
        )
        .unwrap();
        pager.apply_startup_commands().unwrap();
        assert_eq!(pager.top_line, 0);
        assert_eq!(
            pager
                .last_search
                .as_ref()
                .map(|state| (state.pattern.as_str(), state.backward)),
            Some(("alpha", false))
        );
    }

    #[test]
    fn startup_pattern_compilation_errors_surface_early() {
        let mut pager = Pager::new(
            Config::default(),
            sample_set(),
            vec![StartupCommand::Pattern {
                pattern: "(".to_string(),
                backward: false,
            }],
        )
        .unwrap();
        assert!(pager.apply_startup_commands().is_err());
    }

    #[test]
    fn startup_pattern_matches_the_first_line() {
        let mut pager = Pager::new(
            Config::default(),
            sample_set(),
            vec![StartupCommand::Pattern {
                pattern: "alpha".to_string(),
                backward: false,
            }],
        )
        .unwrap();
        pager.apply_startup_commands().unwrap();
        assert_eq!(pager.top_line, 0);
        assert!(pager.status.is_empty());
    }

    #[test]
    fn backward_search_wraps_to_last_match() {
        let config = Config {
            wrap_search: true,
            ..Config::default()
        };
        let mut pager = Pager::new(config, sample_set(), Vec::new()).unwrap();
        pager.perform_search("beta", true).unwrap();
        assert_eq!(pager.top_line, 3);
    }

    #[test]
    fn marks_jump_back_to_saved_line() {
        let mut pager = Pager::new(Config::default(), sample_set(), Vec::new()).unwrap();
        pager.top_line = 2;
        pager.set_mark('a', 2);
        pager.top_line = 0;
        pager.jump_to_mark('a');
        assert_eq!(pager.top_line, 2);
        assert_eq!(pager.status, "jumped to mark a");
    }

    #[test]
    fn missing_mark_sets_status() {
        let mut pager = Pager::new(Config::default(), sample_set(), Vec::new()).unwrap();
        pager.jump_to_mark('z');
        assert_eq!(pager.status, "mark z not set");
    }

    #[test]
    fn control_keys_move_by_single_lines() {
        let mut pager = Pager::new(Config::default(), sample_set(), Vec::new()).unwrap();
        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL))
            .unwrap();
        assert_eq!(pager.top_line, 1);
        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL))
            .unwrap();
        assert_eq!(pager.top_line, 0);
        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL))
            .unwrap();
        assert_eq!(pager.top_line, 1);
        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL))
            .unwrap();
        assert_eq!(pager.top_line, 0);
    }

    #[test]
    fn horizontal_scroll_only_changes_offset_in_chop_mode() {
        let mut pager = Pager::new(Config::default(), sample_set(), Vec::new()).unwrap();
        pager.config.chop_long_lines = true;
        pager.scroll_right(12);
        assert_eq!(pager.horizontal_offset, 12);
        pager.scroll_left(5);
        assert_eq!(pager.horizontal_offset, 7);
        pager.reset_horizontal_offset();
        assert_eq!(pager.horizontal_offset, 0);

        pager.config.chop_long_lines = false;
        pager.scroll_right(40);
        assert_eq!(pager.horizontal_offset, 0);
    }

    #[test]
    fn numeric_prefixes_apply_to_motion_commands() {
        let mut pager = Pager::new(Config::default(), sample_set(), Vec::new()).unwrap();
        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE))
            .unwrap();
        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE))
            .unwrap();
        assert_eq!(pager.top_line, 2);

        pager.top_line = 3;
        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE))
            .unwrap();
        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE))
            .unwrap();
        assert_eq!(pager.top_line, 1);
    }

    #[test]
    fn numeric_prefixes_jump_to_requested_lines() {
        let mut pager = Pager::new(Config::default(), sample_set(), Vec::new()).unwrap();
        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE))
            .unwrap();
        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))
            .unwrap();
        assert_eq!(pager.top_line, 1);

        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE))
            .unwrap();
        pager
            .handle_normal_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))
            .unwrap();
        assert_eq!(pager.top_line, 3);
    }

    #[test]
    fn startup_line_and_follow_commands_apply() {
        let mut pager = Pager::new(
            Config::default(),
            sample_set(),
            vec![StartupCommand::Follow, StartupCommand::Line(3)],
        )
        .unwrap();
        pager.apply_startup_commands().unwrap();
        assert!(pager.config.follow);
        assert_eq!(pager.top_line, 2);
    }

    #[test]
    fn mark_last_visible_line_uses_rendered_bottom_of_screen() {
        let docs = sample_set();
        let config = Config {
            status_bar: false,
            ..Config::default()
        };
        let line = visible_last_line_for_screen(&docs, 1, 80, 2, &config);
        assert_eq!(line, 2);
    }
}

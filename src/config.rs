use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::Args;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub line_numbers: bool,
    pub raw_control_chars: bool,
    pub chop_long_lines: bool,
    pub squeeze_blank_lines: bool,
    pub quit_if_one_screen: bool,
    pub no_init: bool,
    pub follow: bool,
    pub ignore_case: bool,
    pub wrap_search: bool,
    pub highlight: bool,
    pub status_bar: bool,
    pub tab_width: usize,
    pub language: Option<String>,
    pub theme: String,
    pub editor: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            line_numbers: false,
            raw_control_chars: false,
            chop_long_lines: false,
            squeeze_blank_lines: false,
            quit_if_one_screen: false,
            no_init: false,
            follow: false,
            ignore_case: false,
            wrap_search: true,
            highlight: true,
            status_bar: true,
            tab_width: 4,
            language: None,
            theme: "base16-ocean.dark".to_string(),
            editor: "vim".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ConfigFile {
    pub line_numbers: Option<bool>,
    pub raw_control_chars: Option<bool>,
    pub chop_long_lines: Option<bool>,
    pub squeeze_blank_lines: Option<bool>,
    pub quit_if_one_screen: Option<bool>,
    pub no_init: Option<bool>,
    pub follow: Option<bool>,
    pub ignore_case: Option<bool>,
    pub wrap_search: Option<bool>,
    pub highlight: Option<bool>,
    pub status_bar: Option<bool>,
    pub tab_width: Option<usize>,
    pub language: Option<String>,
    pub theme: Option<String>,
    pub editor: Option<String>,
}

impl Config {
    pub fn load(explicit_path: Option<&Path>) -> Result<Self> {
        let mut config = Self::default();
        let path = match explicit_path {
            Some(path) => Some(path.to_path_buf()),
            None => default_config_path(),
        };
        if let Some(path) = path
            && path.exists()
        {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("reading config {}", path.display()))?;
            let parsed: ConfigFile = toml::from_str(&text)
                .with_context(|| format!("parsing config {}", path.display()))?;
            config.merge_file(parsed);
        }
        Ok(config)
    }

    pub fn apply_args(&mut self, args: &Args) {
        if args.line_numbers {
            self.line_numbers = true;
        }
        if args.raw_control_chars {
            self.raw_control_chars = true;
        }
        if args.chop_long_lines {
            self.chop_long_lines = true;
        }
        if args.squeeze_blank_lines {
            self.squeeze_blank_lines = true;
        }
        if args.quit_if_one_screen {
            self.quit_if_one_screen = true;
        }
        if args.no_init {
            self.no_init = true;
        }
        if args.follow {
            self.follow = true;
        }
        if args.ignore_case {
            self.ignore_case = true;
        }
        if let Some(language) = &args.language {
            self.language = Some(language.clone());
        }
        if let Some(theme) = &args.theme {
            self.theme = theme.clone();
        }
        if args.highlight {
            self.highlight = true;
        }
        if args.no_highlight {
            self.highlight = false;
        }
    }

    fn merge_file(&mut self, file: ConfigFile) {
        if let Some(v) = file.line_numbers {
            self.line_numbers = v;
        }
        if let Some(v) = file.raw_control_chars {
            self.raw_control_chars = v;
        }
        if let Some(v) = file.chop_long_lines {
            self.chop_long_lines = v;
        }
        if let Some(v) = file.squeeze_blank_lines {
            self.squeeze_blank_lines = v;
        }
        if let Some(v) = file.quit_if_one_screen {
            self.quit_if_one_screen = v;
        }
        if let Some(v) = file.no_init {
            self.no_init = v;
        }
        if let Some(v) = file.follow {
            self.follow = v;
        }
        if let Some(v) = file.ignore_case {
            self.ignore_case = v;
        }
        if let Some(v) = file.wrap_search {
            self.wrap_search = v;
        }
        if let Some(v) = file.highlight {
            self.highlight = v;
        }
        if let Some(v) = file.status_bar {
            self.status_bar = v;
        }
        if let Some(v) = file.tab_width {
            self.tab_width = v.max(1);
        }
        if let Some(v) = file.language {
            self.language = Some(v);
        }
        if let Some(v) = file.theme {
            self.theme = v;
        }
        if let Some(v) = file.editor {
            self.editor = v;
        }
    }

    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).context("serializing config")
    }
}

fn default_config_path() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".xless").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_merges_config() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".xless");
        fs::create_dir_all(&config_dir).unwrap();
        let path = config_dir.join("config.toml");
        fs::write(
            &path,
            r#"
line_numbers = true
tab_width = 8
squeeze_blank_lines = true
theme = "InspiredGitHub"
editor = "nvim"
"#,
        )
        .unwrap();
        let loaded = Config::load(Some(&path)).unwrap();
        assert!(loaded.line_numbers);
        assert_eq!(loaded.tab_width, 8);
        assert!(loaded.squeeze_blank_lines);
        assert_eq!(loaded.theme, "InspiredGitHub");
        assert_eq!(loaded.editor, "nvim");
    }
}

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use crate::config::Config;
use crate::document::DocumentSet;
use crate::pager::{Pager, StartupCommand};

#[derive(Debug, Parser)]
#[command(
    name = "xless",
    version,
    about = "A fast, color-first pager written in Rust"
)]
pub struct Args {
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,

    #[arg(short = 'N', long = "line-numbers")]
    pub line_numbers: bool,

    #[arg(short = 'R', long = "raw-control-chars")]
    pub raw_control_chars: bool,

    #[arg(short = 'S', long = "chop-long-lines")]
    pub chop_long_lines: bool,

    #[arg(short = 's', long = "squeeze-blank-lines")]
    pub squeeze_blank_lines: bool,

    #[arg(short = 'F', long = "quit-if-one-screen")]
    pub quit_if_one_screen: bool,

    #[arg(short = 'X', long = "no-init")]
    pub no_init: bool,

    #[arg(short = 'f', long = "follow")]
    pub follow: bool,

    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    #[arg(short = 'p', long = "pattern")]
    pub pattern: Option<String>,

    #[arg(long = "language")]
    pub language: Option<String>,

    #[arg(long = "theme")]
    pub theme: Option<String>,

    #[arg(long = "highlight")]
    pub highlight: bool,

    #[arg(long = "no-highlight")]
    pub no_highlight: bool,

    #[arg(long = "config")]
    pub config_path: Option<PathBuf>,

    #[arg(long = "no-config")]
    pub no_config: bool,

    #[arg(long = "dump-config")]
    pub dump_config: bool,
}

pub fn run() -> Result<()> {
    let args = Args::parse();
    let mut config = if args.no_config {
        Config::default()
    } else {
        Config::load(args.config_path.as_deref()).context("failed to load xless configuration")?
    };
    config.apply_args(&args);

    if args.dump_config {
        print!("{}", config.to_toml()?);
        return Ok(());
    }

    let (mut startup, files) = split_startup_commands(args.files);
    if let Some(pattern) = args.pattern {
        startup.push(StartupCommand::Pattern {
            pattern,
            backward: false,
        });
    }

    let input = if files.is_empty() {
        if std::io::stdin().is_terminal() {
            anyhow::bail!("xless needs a file list or piped input");
        }
        DocumentSet::from_stdin(&config)?
    } else {
        DocumentSet::from_paths(&files, &config)?
    };

    let mut pager = Pager::new(config, input, startup)?;
    pager.run()?;
    Ok(())
}

fn split_startup_commands(mut files: Vec<PathBuf>) -> (Vec<StartupCommand>, Vec<PathBuf>) {
    let mut startup = Vec::new();
    let mut split_at = 0usize;
    while let Some(path) = files.get(split_at) {
        let Some(command) = parse_startup_command(path.as_path()) else {
            break;
        };
        startup.push(command);
        split_at += 1;
    }
    let files = files.split_off(split_at);
    (startup, files)
}

fn parse_startup_command(path: &Path) -> Option<StartupCommand> {
    let text = path.to_str()?;
    let rest = text.strip_prefix('+')?;
    match rest {
        "F" => Some(StartupCommand::Follow),
        "G" => Some(StartupCommand::Bottom),
        _ => {
            if let Some(pattern) = rest.strip_prefix('/') {
                Some(StartupCommand::Pattern {
                    pattern: pattern.to_string(),
                    backward: false,
                })
            } else if let Some(pattern) = rest.strip_prefix('?') {
                Some(StartupCommand::Pattern {
                    pattern: pattern.to_string(),
                    backward: true,
                })
            } else if !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit()) {
                let line = rest.parse::<usize>().ok()?;
                Some(StartupCommand::Line(line))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_leading_startup_commands_from_files() {
        let (startup, files) = split_startup_commands(vec![
            PathBuf::from("+42"),
            PathBuf::from("+/needle"),
            PathBuf::from("file.txt"),
        ]);

        assert_eq!(
            startup,
            vec![
                StartupCommand::Line(42),
                StartupCommand::Pattern {
                    pattern: "needle".to_string(),
                    backward: false,
                },
            ]
        );
        assert_eq!(files, vec![PathBuf::from("file.txt")]);
    }

    #[test]
    fn leaves_non_startup_plus_paths_alone() {
        let (startup, files) = split_startup_commands(vec![PathBuf::from("+notes.txt")]);

        assert!(startup.is_empty());
        assert_eq!(files, vec![PathBuf::from("+notes.txt")]);
    }
}

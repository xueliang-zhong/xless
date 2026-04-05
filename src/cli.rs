use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use crate::config::Config;
use crate::document::DocumentSet;
use crate::pager::Pager;

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

    let input = if args.files.is_empty() {
        if std::io::stdin().is_terminal() {
            anyhow::bail!("xless needs a file list or piped input");
        }
        DocumentSet::from_stdin(&config)?
    } else {
        DocumentSet::from_paths(&args.files, &config)?
    };

    let mut pager = Pager::new(config, input, args.pattern)?;
    pager.run()?;
    Ok(())
}

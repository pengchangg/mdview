use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "mdview",
    version,
    about = "Terminal Markdown viewer: scroll, search, outline, mermaid diagrams, live reload"
)]
pub struct Args {
    /// Markdown file or directory to preview (default: the current directory)
    pub file: Option<PathBuf>,

    /// Disable file watching (on by default: re-render when the file is saved)
    /// `Option<bool>` 不是 bool：没给 = None，才能让配置文件填；给了 = Some(true)。
    #[arg(long, num_args = 0, default_missing_value = "true")]
    pub no_watch: Option<bool>,

    /// Disable mouse capture (so the terminal can still select and copy text)
    #[arg(long, num_args = 0, default_missing_value = "true")]
    pub no_mouse: Option<bool>,

    /// Start with the outline sidebar open
    #[arg(long, num_args = 0, default_missing_value = "true")]
    pub outline: Option<bool>,

    /// Maximum body width in columns; extra terminal width becomes margin
    #[arg(long)]
    pub max_width: Option<u16>,

    // 默认值不在这里填：config::merge 统一按 CLI > 配置文件 > 内置默认 三路定下来。
    #[arg(long, value_enum)]
    pub mermaid: Option<MermaidMode>,

    #[arg(long, value_enum)]
    pub theme: Option<ThemeMode>,

    /// Print a shell completion script to stdout and exit (works without a terminal)
    #[arg(long, value_enum)]
    pub print_completion: Option<Completion>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Completion {
    Bash,
    Zsh,
    Fish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum MermaidMode {
    /// Draw it as box-drawing art
    Auto,
    /// Leave it as a plain code block
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ThemeMode {
    /// Follow NO_COLOR / TERM / COLORFGBG
    Auto,
    /// Force the dark palette
    Dark,
    /// Force the light palette
    Light,
}

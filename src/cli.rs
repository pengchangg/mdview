use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "mdview",
    version,
    about = "Terminal Markdown viewer: scroll, search, outline, mermaid diagrams, live reload"
)]
pub struct Args {
    /// Markdown file to preview
    pub file: PathBuf,

    /// Disable file watching (on by default: re-render when the file is saved)
    #[arg(long)]
    pub no_watch: bool,

    /// Disable mouse capture (so the terminal can still select and copy text)
    #[arg(long)]
    pub no_mouse: bool,

    /// Start with the outline sidebar open
    #[arg(long)]
    pub outline: bool,

    /// Maximum body width in columns; extra terminal width becomes margin
    #[arg(long)]
    pub max_width: Option<u16>,

    #[arg(long, value_enum, default_value_t = MermaidMode::Auto)]
    pub mermaid: MermaidMode,

    #[arg(long, value_enum, default_value_t = ThemeMode::Auto)]
    pub theme: ThemeMode,
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

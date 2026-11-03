use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "mdview",
    version,
    about = "终端里的 Markdown 预览器：滚动、搜索、大纲、mermaid、保存即刷新"
)]
pub struct Args {
    /// 要预览的 markdown 文件
    pub file: PathBuf,

    /// 关闭文件监听（默认开启，保存后自动重绘）
    #[arg(long)]
    pub no_watch: bool,

    /// 关闭鼠标捕获（关闭后可用终端原生选中复制）
    #[arg(long)]
    pub no_mouse: bool,

    /// 启动即展开大纲侧栏
    #[arg(long)]
    pub outline: bool,

    /// 正文最大列数，超出的宽度留白（窄栏阅读）
    #[arg(long)]
    pub max_width: Option<u16>,

    #[arg(long, value_enum, default_value_t = MermaidMode::Auto)]
    pub mermaid: MermaidMode,

    #[arg(long, value_enum, default_value_t = ThemeMode::Auto)]
    pub theme: ThemeMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum MermaidMode {
    /// 渲染成字符图形
    Auto,
    /// 原样当代码块显示
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ThemeMode {
    Auto,
    Dark,
    Light,
}

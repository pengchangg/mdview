//! mdview —— 终端 Markdown 预览器。
//!
//! 逻辑与终端 IO 分开：`app`/`render`/`search`/`outline`/`wrap`/`mermaid` 是纯逻辑（可单测），
//! `view`/`term`/`event` 才碰屏幕和按键。

pub mod app;
pub mod cli;
pub mod event;
pub mod mermaid;
pub mod render;
pub mod search;
pub mod term;
pub mod theme;
pub mod view;
pub mod watch;
pub mod wrap;

pub use app::App;
pub use cli::Args;
pub use render::{RenderOptions, RenderedDoc, to_doc};

/// 把 markdown 渲染成可直接绘制的行 —— 整个库的唯一入口。
pub fn preview(src: &str, width: u16, opts: RenderOptions) -> RenderedDoc {
    to_doc(src, width, opts)
}

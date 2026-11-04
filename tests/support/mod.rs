//! 集成测试共用的构造器：渲染用暗色主题、无 mermaid，保证快照稳定。

#![allow(dead_code)]

use mdview::cli::{Args, MermaidMode};
use mdview::render::{RenderOptions, RenderedDoc};
use mdview::theme::Theme;

/// 固定主题，不读环境变量，保证快照不随 CI 的 TERM/NO_COLOR 飘。
pub fn theme() -> Theme {
    Theme {
        light: false,
        color: true,
        ascii: false,
    }
}

pub fn opts() -> RenderOptions {
    RenderOptions {
        theme: theme(),
        mermaid: MermaidMode::Auto,
    }
}

pub fn render(src: &str, width: u16) -> RenderedDoc {
    mdview::to_doc(src, width, opts())
}

pub fn render_no_mermaid(src: &str, width: u16) -> RenderedDoc {
    let mut o = opts();
    o.mermaid = MermaidMode::Off;
    mdview::to_doc(src, width, o)
}

pub fn plain(doc: &RenderedDoc) -> String {
    doc.plain.join("\n")
}

/// 写出临时 md 文件（返回路径），供热更新测试用。
/// 每个用例一个目录：并行跑的用例不能共用同一个 t.md。
static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub fn temp_md(body: &str) -> std::path::PathBuf {
    let path = temp_dir().join("t.md");
    std::fs::write(&path, body).unwrap();
    path
}

/// 一个独立的空目录：多文件用例在里面摆 a.md / part1.md / part10.md。
pub fn temp_dir() -> std::path::PathBuf {
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("mdview-test-{}-{}", std::process::id(), n));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 默认值都是 `None`：测试直接给命令行参数，不当 clap 走过一遍，
/// `theme`/`mermaid` 要硬编码才不会被快照环境飘。
pub fn args(path: &std::path::Path) -> Args {
    Args {
        file: Some(path.to_path_buf()),
        print_completion: None,
        no_watch: Some(true),
        no_mouse: Some(true),
        outline: None,
        max_width: None,
        mermaid: Some(MermaidMode::Off),
        theme: Some(mdview::cli::ThemeMode::Dark),
    }
}

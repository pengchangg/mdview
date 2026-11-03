//! 大纲：行文本、缩进、选中项、截断，以及「选中项跟着阅读位置」。

#[path = "support/mod.rs"]
mod support;

use mdview::render::{Heading, outline_lines};
use support::{render, theme};

fn hs() -> Vec<Heading> {
    vec![
        Heading {
            level: 1,
            text: "总览".into(),
            line: 0,
        },
        Heading {
            level: 2,
            text: "安装".into(),
            line: 12,
        },
        Heading {
            level: 3,
            text: "Windows 上的坑".into(),
            line: 20,
        },
        Heading {
            level: 2,
            text: "这一栏的标题文字非常长需要被截断掉一部分".into(),
            line: 30,
        },
    ]
}

fn text(lines: &[ratatui::text::Line<'static>]) -> Vec<String> {
    lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
        .collect()
}

#[test]
fn one_line_per_heading_with_marker_on_selected() {
    let out = outline_lines(&hs(), 24, 1, theme());
    assert_eq!(out.len(), 4);
    let t = text(&out);
    assert!(
        t[0].starts_with("  ") && t[0].contains("总览"),
        "{:?}",
        t[0]
    );
    assert!(t[1].starts_with("▸"), "选中项要标出来: {:?}", t[1]);
    assert!(
        t[2].starts_with("  ") && t[2].contains("Windows"),
        "{:?}",
        t[2]
    );
}

#[test]
fn indent_follows_relative_level_not_absolute() {
    let out = outline_lines(&hs(), 24, 0, theme());
    let t = text(&out);
    let lead = |s: &str| s.len() - s.trim_start().len();
    assert!(lead(&t[1]) > lead(&t[0]), "h2 应比 h1 缩进");
    assert!(lead(&t[2]) > lead(&t[1]), "h3 应比 h2 缩进");
}

#[test]
fn long_titles_get_truncated_not_wrapped() {
    let out = outline_lines(&hs(), 16, 0, theme());
    assert_eq!(out.len(), 4, "截断不该多出行");
    for l in &out {
        assert!(l.width() <= 16, "超出面板宽度: {:?}", l.spans);
    }
    assert!(text(&out)[3].ends_with('…'), "该有截断省略号");
}

#[test]
fn empty_outline_renders_nothing_and_does_not_panic() {
    assert!(outline_lines(&[], 24, 0, theme()).is_empty());
}

#[test]
fn outline_lines_point_at_rendered_heading_rows() {
    let doc = render("# A\n\n正文\n\n## B\n\n更多\n\n### C\n", 40);
    assert_eq!(doc.headings.len(), 3);
    let out = outline_lines(&doc.headings, 24, 2, theme());
    assert_eq!(out.len(), 3);
    for h in &doc.headings {
        assert!(
            doc.plain_of(h.line).contains(&h.text),
            "大纲行指错了: {:?}",
            h
        );
    }
}

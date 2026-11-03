//! mermaid：切段、出图、失败降级、ASCII 模式。

#[path = "support/mod.rs"]
mod support;

use mdview::mermaid::{Seg, kind_label, render, split};
use support::{plain, render as render_doc, render_no_mermaid};

const FLOW: &str = "graph TD\n  A[开始] --> B{判断}\n  B -->|是| C[做]\n  B -->|否| D[不做]\n";

#[test]
fn splits_fenced_diagrams_out_of_the_markdown() {
    let md = format!("# 标题\n\n```mermaid\n{FLOW}\n```\n\n正文\n");
    let segs = split(&md);
    assert!(matches!(segs[0], Seg::Md(_)), "先应是 markdown");
    assert!(
        segs.iter()
            .any(|s| matches!(s, Seg::Diagram(c) if c.contains("graph TD"))),
        "{segs:?}"
    );
    assert!(
        segs.last()
            .is_some_and(|s| matches!(s, Seg::Md(m) if m.contains("正文")))
    );
}

#[test]
fn ordinary_code_blocks_stay_markdown() {
    let md = "```rust\nfn main() {}\n```\n";
    let segs = split(md);
    assert!(segs.iter().all(|s| matches!(s, Seg::Md(_))));
    assert!(
        matches!(&segs[0], Seg::Md(m) if m.contains("```")),
        "普通代码块的围栏得原样留给 markdown"
    );
    assert!(plain(&render_doc(md, 40)).contains("fn main"));
}

#[test]
fn closing_fence_does_not_leak_into_the_diagram() {
    // 闭合 ``` 一旦进了图，mermaid 解析器会把它当成一个节点画出来。
    let md = format!("# 标题\n\n```mermaid\n{FLOW}```\n\n正文\n");
    let segs = split(&md);
    let Seg::Diagram(d) = segs.iter().find(|s| matches!(s, Seg::Diagram(_))).unwrap() else {
        unreachable!()
    };
    assert!(!d.contains("```"), "闭合围栏不该进图: {d:?}");
    assert!(render(d, 60, false).is_some());
    assert!(render_doc(&md, 80).warnings.is_empty(), "正常图不该报告警");
}

/// mermaid-text 会把 CJK 标签排成「开 始」（补半角空格），比对标签时先去空白。
fn flat(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn flowchart_renders_as_text_art() {
    let pic = render(FLOW, 60, false).expect("flowchart 应能渲染");
    let t = flat(&pic);
    assert!(t.contains("开始") && t.contains("判断"), "{pic}");
    assert!(
        pic.contains("A") || pic.contains("─") || pic.contains("│"),
        "该有框线/箭头: {pic}"
    );
    assert!(
        pic.lines().all(|l| l.chars().count() <= 60 + 8),
        "宽度失控:\n{pic}"
    );
}

#[test]
fn sequence_and_pie_also_render() {
    let seq = "sequenceDiagram\n  甲->>乙: 你好\n  乙-->>甲: 收到\n";
    assert!(render(seq, 60, false).is_some(), "sequenceDiagram 应支持");
    let pie = "pie title 占比\n  \"A\" : 42\n  \"B\" : 58\n";
    assert!(render(pie, 60, false).is_some(), "pie 应支持");
}

#[test]
fn narrow_budget_compares_instead_of_overflowing() {
    let pic = render(FLOW, 28, false).expect("窄也该出图");
    assert!(
        pic.lines().all(|l| l.chars().count() <= 32),
        "压不到列宽预算:\n{pic}"
    );
}

#[test]
fn ascii_mode_avoids_box_drawing() {
    let pic = render(FLOW, 60, true).expect("ascii 也应出图");
    assert!(
        !pic.chars()
            .any(|c| ['─', '│', '┌', '┐', '└', '┘', '├', '┤'].contains(&c)),
        "{pic}"
    );
    assert!(flat(&pic).contains("开始"), "{pic}");
}

#[test]
fn broken_diagram_falls_back_to_the_code_block_with_a_warning() {
    let md = "前言\n\n```mermaid\n这根本不是一张图 @@@\n```\n\n后记\n";
    let doc = render_doc(md, 60);
    assert!(!doc.warnings.is_empty(), "该报渲染告警");
    let all = plain(&doc);
    assert!(
        all.contains("这根本不是一张图"),
        "失败要原样贴回代码块:\n{all}"
    );
    assert!(all.contains("前言") && all.contains("后记"), "上下文不能丢");
}

#[test]
fn off_mode_keeps_the_raw_block_and_warns_nothing() {
    let md = format!("```mermaid\n{FLOW}```\n");
    let doc = render_no_mermaid(&md, 60);
    assert!(doc.warnings.is_empty());
    assert!(plain(&doc).contains("graph TD"));
}

#[test]
fn kind_label_names_the_diagram_type() {
    assert!(kind_label(FLOW).to_lowercase().contains("graph"));
    assert!(kind_label("sequenceDiagram\n  A->>B: hi").contains("sequence"));
}

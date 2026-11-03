//! 渲染快照：整篇样例文档的可见文本 + 大纲行号。

#[path = "support/mod.rs"]
mod support;

use support::{plain, render};

const DOC: &str = r#"# 标题一

段落文字，带 **粗体**、`代码` 和 [链接](https://example.com)。

## 标题二

- 列表项甲
- 列表项乙

```rust
fn main() {}
```

> 引用一句话

| 列 A | 列 B |
| ---- | ---- |
| 1    | 2    |

### 标题三
"#;

#[test]
fn fixture_visible_text() {
    let doc = render(DOC, 60);
    insta::assert_snapshot!(plain(&doc));
}

#[test]
fn headings_point_at_their_own_lines() {
    let doc = render(DOC, 60);
    let names: Vec<(u8, &str)> = doc
        .headings
        .iter()
        .map(|h| (h.level, h.text.as_str()))
        .collect();
    assert_eq!(names, vec![(1, "标题一"), (2, "标题二"), (3, "标题三")]);
    for h in &doc.headings {
        assert_eq!(
            doc.plain_of(h.line).trim(),
            h.text,
            "标题 {} 指错行",
            h.text
        );
    }
}

#[test]
fn narrow_width_wraps_rather_than_truncates() {
    let wide = render(DOC, 20);
    let all = plain(&wide);
    assert!(all.contains("段落文字，带"), "正文不见了");
    assert!(all.contains("链接"), "行内链接被吃掉了");
    assert!(all.contains("fn main"), "代码块被吃掉了");
    assert!(wide.lines.iter().all(|l| l.width() <= 20), "有行超过 20 列");
}

#[test]
fn wide_and_narrow_keep_the_same_heading_order() {
    let a: Vec<String> = render(DOC, 120)
        .headings
        .iter()
        .map(|h| h.text.clone())
        .collect();
    let b: Vec<String> = render(DOC, 24)
        .headings
        .iter()
        .map(|h| h.text.clone())
        .collect();
    assert_eq!(a, b);
}

#[test]
fn no_color_and_ascii_still_render() {
    let mut o = support::opts();
    o.theme.color = false;
    o.theme.ascii = true;
    let doc = mdview::to_doc(DOC, 60, o);
    assert!(plain(&doc).contains("标题一"));
}

#[test]
fn empty_input_yields_empty_doc() {
    let doc = render("", 60);
    assert!(doc.lines.is_empty());
    assert!(doc.headings.is_empty());
}

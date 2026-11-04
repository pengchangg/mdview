//! 搜索：大小写、字节区间、正则、高亮不丢文本。

#[path = "support/mod.rs"]
mod support;

use mdview::search::{compile, find, highlight};
use ratatui::style::Style;
use ratatui::text::Line;
use support::render;

use ratatui::style::Color;

const HIT: Style = Style::new().bg(Color::Red);

/// 测试里懒得每行写 unwrap；编译失败的用例直接走 compile。
fn pat(q: &str) -> mdview::search::Pattern {
    compile(q).unwrap_or_else(|e| panic!("{q:?} 应该是合法搜索词: {e}"))
}

#[test]
fn matches_ignore_case_in_english() {
    // 空行分开：markdown 软换行会把相邻两行合并成一段。
    let doc = render("Hello World\n\nhello again\n", 40);
    let m = find(&doc.plain, &pat("HELLO"));
    assert_eq!(m.len(), 2, "两行都该命中");
    assert_eq!((m[0].line, m[1].line), (0, 2));
    assert_eq!(m[0].ranges, vec![(0, 5)]);
}

#[test]
fn matches_cjk_and_repeated_hits_on_one_line() {
    let doc = render("终端里的 Markdown 预览器，markdown 很好用\n", 60);
    let m = find(&doc.plain, &pat("markdown"));
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].ranges.len(), 2, "同一行两处命中");
    for (a, b) in &m[0].ranges {
        assert_eq!(
            &doc.plain[0][*a..*b].to_lowercase(),
            "markdown",
            "区间应正好框住命中"
        );
    }
}

#[test]
fn empty_or_absent_needle_matches_nothing() {
    let doc = render("# 标题\n\n正文\n", 40);
    assert!(find(&doc.plain, &pat("")).is_empty());
    assert!(find(&doc.plain, &pat("不存在")).is_empty());
}

#[test]
fn regex_alternation_and_class_work() {
    let doc = render("foo bar\n\nbaz foo\n\nqux\n", 40);
    let m = find(&doc.plain, &pat("foo|baz"));
    assert_eq!(m.len(), 2, "两行各一处命中");
    assert_eq!(m[0].ranges, vec![(0, 3)], "foo bar 只命中 foo");
    assert_eq!(m[1].ranges, vec![(0, 3), (4, 7)], "baz foo 两处都算");

    let m = find(&doc.plain, &pat("[fb]o+"));
    assert_eq!(m.len(), 2, "字符类 + 量词");
}

#[test]
fn case_flag_c_makes_it_case_sensitive() {
    let doc = render("Hello World\n\nhello again\n", 40);
    assert_eq!(find(&doc.plain, &pat("HELLO")).len(), 2, "默认忽略大小写");
    assert_eq!(
        find(&doc.plain, &pat(r"HELLO\C")).len(),
        0,
        "\\C 之后区分大小写"
    );
    assert_eq!(
        find(&doc.plain, &pat(r"HELLO\c")).len(),
        2,
        "\\c 再忽略回来"
    );
}

#[test]
fn escaped_case_flag_stays_literal() {
    // 打 `\\C` 找的是反斜杠 + C，不是大小写开关。
    let doc = render("```\nx\\Cy\n```\n", 40);
    assert_eq!(find(&doc.plain, &pat(r"\\C")).len(), 1);
}

#[test]
fn bad_regex_is_an_error_and_only_the_first_line_matters() {
    assert!(compile("(").is_err());
    let e = compile("a{2,1}").err().unwrap();
    assert!(!e.is_empty() && !e.contains('\n'), "状态栏只一行: {e:?}");
}

#[test]
fn zero_width_matches_are_dropped() {
    let doc = render("foo\n\nqux\n", 40);
    let m = find(&doc.plain, &pat("o*"));
    assert_eq!(m.len(), 1, "空匹配不该算命中");
    for (a, b) in &m[0].ranges {
        assert!(b > a, "区间非空");
    }
}

#[test]
fn highlight_keeps_all_text_and_styles_the_hit() {
    let line = Line::from("hello world");
    let out = highlight(&line, &[(6, 11)], HIT, None);
    assert_eq!(
        out.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>(),
        "hello world"
    );
    assert_eq!(out.spans.len(), 2);
    assert_eq!(out.spans[1].style.bg, Some(Color::Red));

    let cur = highlight(
        &line,
        &[(0, 5)],
        HIT,
        Some((0, Style::new().fg(Color::Red))),
    );
    assert_eq!(
        cur.spans[0].style.fg,
        Some(Color::Red),
        "当前命中用更强的样式"
    );
    assert_eq!(
        cur.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>(),
        "hello world"
    );
}

#[test]
fn highlight_survives_multi_span_and_cjk_lines() {
    let doc = render("见 **粗体** 与 `code`\n", 40);
    let hits = find(&doc.plain, &pat("粗体"));
    assert_eq!(hits.len(), 1);
    let out = highlight(&doc.lines[hits[0].line], &hits[0].ranges, HIT, None);
    let text: String = out.spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(text.contains("粗体"), "高亮不该丢字: {text}");
    assert!(out.spans.iter().any(|s| s.style.bg == Some(Color::Red)));
}

#[test]
fn plain_and_lines_line_up_one_to_one() {
    let doc = render("# 标题\n\n- 项目符号\n\n```\ncode\n```\n", 30);
    assert_eq!(doc.plain.len(), doc.lines.len());
    for (i, l) in doc.lines.iter().enumerate() {
        let rebuilt: String = l.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(rebuilt, doc.plain[i], "第 {i} 行的 plain 与 spans 不一致");
    }
}

#[test]
fn hit_on_a_wrapped_line_is_found_on_its_own_row() {
    let doc = render("一二三四五六七八", 4);
    assert!(doc.lines.len() >= 2, "应当被折行");
    let m = find(&doc.plain, &pat("五六"));
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].ranges, vec![(0, 6)], "命中在折后的第二行开头");
}

//! 搜索：大小写、字节区间、正则、高亮不丢文本、命中能跨软换行。

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
    let m = find(&doc.logical, &pat("HELLO"));
    assert_eq!(m.len(), 2, "两行都该命中");
    assert_eq!((m[0].line, m[1].line), (0, 2));
    assert_eq!(m[0].rows[0].ranges, vec![(0, 5)]);
}

#[test]
fn matches_cjk_and_repeated_hits_on_one_line() {
    let doc = render("终端里的 Markdown 预览器，markdown 很好用\n", 60);
    let m = find(&doc.logical, &pat("markdown"));
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].rows[0].ranges.len(), 2, "同一行两处命中");
    for (a, b) in &m[0].rows[0].ranges {
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
    assert!(find(&doc.logical, &pat("")).is_empty());
    assert!(find(&doc.logical, &pat("不存在")).is_empty());
}

#[test]
fn regex_alternation_and_class_work() {
    let doc = render("foo bar\n\nbaz foo\n\nqux\n", 40);
    let m = find(&doc.logical, &pat("foo|baz"));
    assert_eq!(m.len(), 2, "两行各一处命中");
    assert_eq!(m[0].rows[0].ranges, vec![(0, 3)], "foo bar 只命中 foo");
    assert_eq!(
        m[1].rows[0].ranges,
        vec![(0, 3), (4, 7)],
        "baz foo 两处都算"
    );

    let m = find(&doc.logical, &pat("[fb]o+"));
    assert_eq!(m.len(), 2, "字符类 + 量词");
}

#[test]
fn case_flag_c_makes_it_case_sensitive() {
    let doc = render("Hello World\n\nhello again\n", 40);
    assert_eq!(find(&doc.logical, &pat("HELLO")).len(), 2, "默认忽略大小写");
    assert_eq!(
        find(&doc.logical, &pat(r"HELLO\C")).len(),
        0,
        "\\C 之后区分大小写"
    );
    assert_eq!(
        find(&doc.logical, &pat(r"HELLO\c")).len(),
        2,
        "\\c 再忽略回来"
    );
}

#[test]
fn escaped_case_flag_stays_literal() {
    // 打 `\\C` 找的是反斜杠 + C，不是大小写开关。
    let doc = render("```\nx\\Cy\n```\n", 40);
    assert_eq!(find(&doc.logical, &pat(r"\\C")).len(), 1);
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
    let m = find(&doc.logical, &pat("o*"));
    assert_eq!(m.len(), 1, "空匹配不该算命中");
    for (a, b) in &m[0].rows[0].ranges {
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
    let hits = find(&doc.logical, &pat("粗体"));
    assert_eq!(hits.len(), 1);
    let out = highlight(&doc.lines[hits[0].line], &hits[0].rows[0].ranges, HIT, None);
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
    let m = find(&doc.logical, &pat("五六"));
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].rows[0].ranges, vec![(0, 6)], "命中在折后的第二行开头");
}

// ---- 跨软换行：段落被折开后，一段话照样搜得到，但只亮命中的那几行 ----

/// 段落里的软换行 + 硬折行都算：文本按逻辑行匹配，区间落回显示行。
#[test]
fn a_hit_spanning_a_soft_wrap_matches_once() {
    let doc = render("alpha bravo\ncharlie delta\n", 40);
    assert_eq!(doc.logical.len(), 1, "软换行并成一个逻辑行");
    assert_eq!(doc.plain[0], "alpha bravo charlie delta");
    let m = find(&doc.logical, &pat("bravo charlie"));
    assert_eq!(m.len(), 1, "跨软换行只算一次命中");
    assert_eq!(m[0].line, 0);
    assert_eq!(m[0].rows.len(), 1, "没被折开就还是这一行");
    let (a, b) = m[0].rows[0].ranges[0];
    assert_eq!(&doc.plain[0][a..b], "bravo charlie");
}

#[test]
fn a_hit_spanning_a_wrapped_row_lights_both_rows() {
    // 40 列宽，第一行放不下 "bravo charlie"，命中会被切成两段。
    let doc = render("alpha bravo\ncharlie delta\n", 12);
    assert!(doc.plain.len() >= 2, "应当被折行: {:?}", doc.plain);
    let m = find(&doc.logical, &pat("bravo charlie"));
    assert_eq!(m.len(), 1, "折开了也只算一次命中");
    assert!(m[0].rows.len() >= 2, "该跨两行: {:?}", m[0].rows);
    for rh in &m[0].rows {
        let text = &doc.plain[rh.row];
        for (a, b) in &rh.ranges {
            assert!(!text[*a..*b].is_empty(), "区间不该空");
            assert!(*b <= text.len(), "区间不该越过行尾");
        }
    }
    let got: String = m[0]
        .rows
        .iter()
        .flat_map(|rh| {
            rh.ranges
                .iter()
                .map(|&(a, b)| doc.plain[rh.row][a..b].to_string())
        })
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(got, "bravo charlie", "两段拼回去就是命中原文");
}

#[test]
fn a_long_paragraph_searched_across_its_rows() {
    let src = "The quick brown fox jumps over the lazy dog\nand keeps running\n";
    let doc = render(src, 30);
    assert_eq!(doc.plain.len(), 2, "{:?}", doc.plain);
    let m = find(&doc.logical, &pat("over the lazy"));
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].rows.len(), 2, "命中跨了折行处: {:?}", m[0].rows);
    // 高亮画出来不该丢字
    let first = m[0].rows[0].row;
    let out = highlight(&doc.lines[first], &m[0].rows[0].ranges, HIT, None);
    let text: String = out.spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(text, doc.plain[first]);
}

/// 代码块与表格按自己的行匹配，不该跟邻居粘成一段。
#[test]
fn code_and_table_stay_one_logical_line_each() {
    let doc = render(
        "para text\n\n```\ncode one\ncode two\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n",
        40,
    );
    assert!(
        find(&doc.logical, &pat("code one code two")).is_empty(),
        "代码块两行不该并成一句"
    );
    assert_eq!(find(&doc.logical, &pat("code two")).len(), 1);
    assert!(
        find(&doc.logical, &pat("1 2 |")).is_empty(),
        "表格单元格不该连着搜: {:?}",
        find(&doc.logical, &pat("1 2 |"))
    );
}

/// 逻辑行与显示行的对位：每个逻辑行的 spans 拼回去正好是它的文本。
#[test]
fn logical_spans_cover_their_rows_in_order() {
    let doc = render(
        "# 标题\n\n见 **粗体** 与 `code`，还有中文一句话要折行才放得下看看\n\n- 项目\n\n```\nlong code line that is definitely longer than the width limit here\n```\n",
        16,
    );
    let mut prev = 0usize;
    for l in &doc.logical {
        assert!(
            l.first_row >= prev && l.first_row + l.spans.len() <= doc.lines.len(),
            "行号越界: {l:?}"
        );
        assert_eq!(l.spans.windows(2).filter(|w| w[0].1 > w[1].0).count(), 0);
        for (i, &(s, e)) in l.spans.iter().enumerate() {
            assert!(s <= e && e <= l.text.len(), "区间越界: {l:?}");
            let row = l.first_row + i;
            assert_eq!(
                l.text[s..e].replace('\t', "    "),
                doc.plain[row],
                "第 {row} 行与其区间不一致"
            );
        }
        prev = l.first_row + l.spans.len();
    }
}

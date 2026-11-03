//! 子串搜索：按字节区间给命中处上色。
//!
//! 大小写不敏感；区间用字节偏移，因为 `plain` 就是把同一行的 span 内容拼起来的，
//! 偏移能直接映射回 span 边界，不需要再算字素宽度。

use ratatui::style::Style;
use ratatui::text::{Line, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub line: usize,
    pub ranges: Vec<(usize, usize)>,
}

/// 全量重扫：实测 1 MB 文档每按一个键 6.8 ms，不需要增量索引。
pub fn find(plain: &[String], needle: &str) -> Vec<Match> {
    if needle.is_empty() {
        return Vec::new();
    }
    let nl: Vec<char> = needle.to_lowercase().chars().collect();
    plain
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            let ranges = find_line(line, &nl);
            (!ranges.is_empty()).then_some(Match { line: i, ranges })
        })
        .collect()
}

fn find_line(hay: &str, nl: &[char]) -> Vec<(usize, usize)> {
    let hc: Vec<char> = hay.chars().collect();
    let mut offs = Vec::with_capacity(hc.len() + 1);
    let mut o = 0usize;
    for c in &hc {
        offs.push(o);
        o += c.len_utf8();
    }
    offs.push(o);

    let mut out = Vec::new();
    let mut i = 0;
    while i + nl.len() <= hc.len() {
        // shortcut: 只比较单字符小写，İ 这类一进多出的字符当不匹配处理。
        if hc[i..i + nl.len()]
            .iter()
            .zip(nl)
            .all(|(a, b)| a.to_lowercase().next() == Some(*b))
        {
            out.push((offs[i], offs[i + nl.len()]));
            i += nl.len();
        } else {
            i += 1;
        }
    }
    out
}

/// 给 `ranges` 处叠加样式；`current` 命中的那条区间用 `cur` 样式（只可能一条）。
pub fn highlight(
    line: &Line<'_>,
    ranges: &[(usize, usize)],
    hit: Style,
    current: Option<(usize, Style)>,
) -> Line<'static> {
    if ranges.is_empty() {
        return owned(line);
    }
    let mut spans = Vec::new();
    let mut off = 0usize;
    for sp in &line.spans {
        let (s, e) = (off, off + sp.content.len());
        off = e;
        let mut pos = s;
        for (i, &(a, b)) in ranges.iter().enumerate() {
            if b <= s || a >= e {
                continue;
            }
            let (a, b) = (a.max(s), b.min(e));
            if a > pos {
                spans.push(Span::styled(
                    sp.content[pos - s..a - s].to_string(),
                    sp.style,
                ));
            }
            let style = match current {
                Some((ci, st)) if ci == i => sp.style.patch(st),
                _ => sp.style.patch(hit),
            };
            spans.push(Span::styled(sp.content[a - s..b - s].to_string(), style));
            pos = b;
        }
        if pos < e {
            spans.push(Span::styled(sp.content[pos - s..].to_string(), sp.style));
        }
    }
    Line::from(spans)
}

fn owned(line: &Line<'_>) -> Line<'static> {
    Line::from(
        line.spans
            .iter()
            .map(|s| Span::styled(s.content.to_string(), s.style))
            .collect::<Vec<_>>(),
    )
}

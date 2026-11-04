//! 搜索：按字节区间给命中处上色。默认大小写不敏感，含元字符时按正则匹配。
//!
//! 区间用字节偏移，因为 `plain` 就是把同一行的 span 内容拼起来的，
//! 偏移能直接映射回 span 边界，不需要再算字素宽度。

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use regex::RegexBuilder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub line: usize,
    pub ranges: Vec<(usize, usize)>,
}

/// 编译好的搜索词。不含元字符就走原来的字面量扫描：省掉每按一键编译一次正则，
/// 也让纯文本搜索的行为和开销跟以前完全一样。
#[derive(Debug, Clone)]
pub enum Pattern {
    /// 已小写的字面量；空串就是「没有搜索词」。
    Lit(String),
    Re(regex::Regex),
}

const META: &[char] = &[
    '\\', '.', '+', '*', '?', '(', ')', '|', '[', ']', '{', '}', '^', '$',
];

/// 编译搜索词。正则语法按 [regex] crate 走：`a|b`、`(ab)+`、`\b` 直接写，不用像 vim
/// 那样给元字符反着加反斜杠。结尾的 `\c` / `\C` 是 vim 的大小写开关（默认忽略大小写，
/// `\C` 才区分）。出错只回一行文案，状态栏放得下。
pub fn compile(q: &str) -> Result<Pattern, String> {
    if q.is_empty() || !q.contains(META) {
        return Ok(Pattern::Lit(q.to_lowercase()));
    }
    let (body, ignore_case) = strip_case_flag(q);
    RegexBuilder::new(body)
        .case_insensitive(ignore_case)
        .build()
        .map(Pattern::Re)
        .map_err(|e| {
            let s = e.to_string();
            s.lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("invalid pattern")
                .to_string()
        })
}

/// 取走结尾的 `\c` / `\C`。它前面的反斜杠是偶数个时那些反斜杠已自行配对，
/// 末尾这个 c/C 是字面量字母（`\\c` 找的是反斜杠 + c），不当开关用。
fn strip_case_flag(q: &str) -> (&str, bool) {
    for flag in ['C', 'c'] {
        if let Some(rest) = q.strip_suffix(flag) {
            let slashes = rest.chars().rev().take_while(|c| *c == '\\').count();
            if slashes % 2 == 1 {
                // 末尾那个反斜杠归开关，得一起剔掉，不然正文以 `\` 结尾编译不过。
                return (&rest[..rest.len() - 1], flag == 'c');
            }
        }
    }
    (q, true)
}

/// 全量重扫：实测 1 MB 文档每按一个键 6.8 ms，不需要增量索引。
pub fn find(plain: &[String], pat: &Pattern) -> Vec<Match> {
    // 字面量先拆成 char，整篇扫描就不用每行重建一次了。
    let lit: Vec<char> = match pat {
        Pattern::Lit(nl) => nl.chars().collect(),
        Pattern::Re(_) => Vec::new(),
    };
    if matches!(pat, Pattern::Lit(nl) if nl.is_empty()) {
        return Vec::new();
    }
    plain
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            let ranges = match pat {
                Pattern::Lit(_) => find_line(line, &lit),
                // shortcut: 零宽命中（`a*` 匹配空串那种）丢掉，它画不出高亮；
                // 真要跟随空匹配得让 Match 支持空区间 + view 画光标。
                Pattern::Re(re) => re
                    .find_iter(line)
                    .filter(|m| !m.as_str().is_empty())
                    .map(|m| (m.start(), m.end()))
                    .collect(),
            };
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

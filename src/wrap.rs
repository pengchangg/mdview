//! 带样式的软换行。
//!
//! `tui-markdown` 只产出「逻辑行」，而 `Paragraph` 的 wrap 拿不到换行后的行号，
//! 搜索、大纲跳转、滚动上界都要按**可见行**计数，所以这里自己折一次，
//! 顺便把 span 全部转成 owned（`Line<'static>`）。

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// (grapheme, 样式, 显示宽度)
type Cell = (String, Style, usize);

pub fn wrap_lines(lines: &[Line<'_>], width: usize) -> Vec<Line<'static>> {
    lines.iter().flat_map(|l| wrap_line(l, width)).collect()
}

/// 把一条逻辑行折成若干条宽度 ≤ `width` 的可见行（`width == 0` 时原样返回）。
pub fn wrap_line(line: &Line<'_>, width: usize) -> Vec<Line<'static>> {
    if width == 0 || line.width() <= width {
        return vec![owned(line)];
    }

    // 行首缩进单独留着，只挂在第一行上。
    let mut cells = cells_of(line);
    let indent_end = cells
        .iter()
        .position(|c| c.0.trim() != "")
        .unwrap_or(cells.len());
    let indent: Vec<Cell> = cells.drain(..indent_end).collect();

    let mut out: Vec<Line<'static>> = Vec::new();
    let mut cur: Vec<Cell> = indent.clone();
    let mut cur_w: usize = indent.iter().map(|c| c.2).sum();
    let mut gap: Vec<Cell> = Vec::new(); // 词间空格，只在决定不换行时才落到当前行

    for tok in tokens(cells) {
        let tok_w: usize = tok.iter().map(|c| c.2).sum();
        let is_space = tok.first().is_some_and(|c| c.0 == " ");
        if is_space {
            gap = tok;
            continue;
        }
        let gap_w: usize = gap.iter().map(|c| c.2).sum();
        if cur_w + gap_w + tok_w > width {
            if cur_w > indent_w(&indent) {
                // 词内断行前先换行，行尾空格丢弃。
                out.push(line_from(std::mem::take(&mut cur)));
                cur_w = 0;
            }
            if tok_w > width {
                // 超长词（长 URL、连续中文）：按列宽硬切。
                let mut rest = tok;
                while !rest.is_empty() {
                    let take = take_to(&mut rest, width - cur_w.min(width));
                    cur.extend(take);
                    cur_w = cur.iter().map(|c| c.2).sum();
                    if cur_w >= width {
                        out.push(line_from(std::mem::take(&mut cur)));
                        cur_w = 0;
                    }
                }
            } else {
                cur.extend(tok);
                cur_w += tok_w;
            }
        } else {
            cur.append(&mut gap);
            cur_w += gap_w;
            cur.extend(tok);
            cur_w += tok_w;
        }
        gap = Vec::new();
    }
    let _ = &indent;
    if !cur.is_empty() || out.is_empty() {
        out.push(line_from(cur));
    }
    out
}

fn indent_w(indent: &[Cell]) -> usize {
    indent.iter().map(|c| c.2).sum()
}

/// 从头部取走最多 `w` 列宽的 cell。
fn take_to(rest: &mut Vec<Cell>, w: usize) -> Vec<Cell> {
    let mut out = Vec::new();
    let mut used = 0;
    while let Some(c) = rest.first() {
        if used + c.2 > w && !out.is_empty() {
            break;
        }
        used += c.2;
        out.push(rest.remove(0));
        if used >= w {
            break;
        }
    }
    out
}

/// 切成「空格串」与「非空格串」交替的 token。
fn tokens(cells: Vec<Cell>) -> Vec<Vec<Cell>> {
    let mut out: Vec<Vec<Cell>> = Vec::new();
    for c in cells {
        let same = out.last().is_some_and(|t| (t[0].0 == " ") == (c.0 == " "));
        if same {
            out.last_mut().unwrap().push(c);
        } else {
            out.push(vec![c]);
        }
    }
    out
}

fn cells_of(line: &Line<'_>) -> Vec<Cell> {
    line.spans
        .iter()
        .flat_map(|s| {
            s.content.graphemes(true).map(move |g| {
                let g: String = if g == "\t" {
                    "    ".to_string()
                } else {
                    g.to_string()
                };
                let w = UnicodeWidthStr::width(g.as_str());
                (g, s.style, w.max(1))
            })
        })
        .collect()
}

fn line_from(cells: Vec<Cell>) -> Line<'static> {
    Line::from(
        cells
            .into_iter()
            .map(|(g, style, _)| Span::styled(g, style))
            .collect::<Vec<_>>(),
    )
}

fn owned(line: &Line<'_>) -> Line<'static> {
    Line::from(
        line.spans
            .iter()
            .map(|s| Span::styled(s.content.to_string(), s.style))
            .collect::<Vec<_>>(),
    )
}

/// 按显示宽度截断，绝不切断字素。
pub fn truncate(s: &str, width: usize) -> String {
    if s.width() <= width {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for g in s.graphemes(true) {
        let gw = UnicodeWidthStr::width(g);
        if w + gw + 1 > width {
            out.push('…');
            return out;
        }
        w += gw;
        out.push_str(g);
    }
    out
}

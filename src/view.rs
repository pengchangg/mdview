//! 绘制：正文（已折行）+ 右侧大纲 + 底部状态栏 + 搜索/帮助浮层。
//!
//! 正文不按 Paragraph 的 wrap 折行 —— `doc.lines` 已经按 `text_width` 折好了，
//! 这里只切片可视行并叠加搜索高亮，滚动条手动画（省得一列宽度算不清）。

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, Focus};
use crate::search;
use crate::theme::Theme;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    if area.width < 20 || area.height < 5 {
        // 这么小画什么都是错位，只贴一句话；按键照常能用（q 退出）。
        app.body_rect = area;
        app.panel_rect = Rect::default();
        f.render_widget(
            Paragraph::new("Terminal too small - resize to at least 20x5 (q to quit)")
                .style(app.theme.warning()),
            area,
        );
        return;
    }
    let [main, status] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    let pw = app.panel_width();
    let (body, panel) = if pw > 0 {
        let p = Rect {
            x: main.x + main.width - pw,
            y: main.y,
            width: pw,
            height: main.height,
        };
        let b = Rect {
            x: main.x,
            y: main.y,
            width: main.width - pw - 1,
            height: main.height,
        };
        (b, p)
    } else {
        (main, Rect::default())
    };
    app.body_rect = body;
    app.panel_rect = panel;

    draw_body(f, app, body);
    if pw > 0 {
        draw_outline(f, app, panel);
    }
    draw_status(f, app, status);
    if app.help {
        draw_help(f, app, main);
    }
}

fn draw_body(f: &mut Frame, app: &App, body: Rect) {
    let th = &app.theme;
    let h = body.height as usize;
    let start = app.scroll;
    let end = start + h;

    // 一条命中可能跨几行：先摊成 (显示行, 命中下标)，同一行可能挂着好几条。
    let hits: Vec<(usize, usize)> = app
        .matches
        .iter()
        .enumerate()
        .flat_map(|(i, m)| m.rows.iter().map(move |rh| (rh.row, i)))
        .filter(|(row, _)| *row >= start && *row < end)
        .collect();
    let mut hit_idx = 0;

    let mut rows: Vec<Line<'static>> = Vec::with_capacity(h);
    for r in 0..h {
        let row = start + r;
        match app.doc.lines.get(row) {
            Some(line) => {
                let mut ranges: Vec<(usize, usize)> = Vec::new();
                let mut cur = None;
                while hits.get(hit_idx).is_some_and(|(r, _)| *r == row) {
                    let i = hits[hit_idx].1;
                    hit_idx += 1;
                    let Some(rh) = app.matches[i].rows.iter().find(|rh| rh.row == row) else {
                        continue;
                    };
                    cur = cur.or((i == app.idx).then_some(ranges.len()));
                    ranges.extend_from_slice(&rh.ranges);
                }
                rows.push(if ranges.is_empty() {
                    clone_line(line)
                } else {
                    search::highlight(
                        line,
                        &ranges,
                        th.search_hit(),
                        cur.map(|i| (i, th.search_current())),
                    )
                });
            }
            None => rows.push(Line::default()),
        }
    }
    f.render_widget(Paragraph::new(Text::from(rows)), body);
    draw_scrollbar(f, app, body, th);
}

fn clone_line(line: &Line<'_>) -> Line<'static> {
    Line::from(
        line.spans
            .iter()
            .map(|s| Span::styled(s.content.to_string(), s.style))
            .collect::<Vec<_>>(),
    )
}
/// 一列滚动条：轨道 `┆`、滑块 `▍`。内容不足一屏时不画。
fn draw_scrollbar(f: &mut Frame, app: &App, body: Rect, th: &Theme) {
    let total = app.doc.lines.len();
    let h = body.height as usize;
    if total <= h || body.width == 0 {
        return;
    }
    let x = body.x + app.text_width().min(body.width.saturating_sub(1));
    let thumb_h = ((h * h) as f64 / total as f64).ceil().clamp(1.0, h as f64) as usize;
    let travel = (h - thumb_h) as f64;
    let frac = app.scroll as f64 / (total - h).max(1) as f64;
    let top = (frac * travel).round() as usize;
    let buf = f.buffer_mut();
    for r in 0..h {
        let (sym, style) = if r >= top && r < top + thumb_h {
            ("▍", th.scroll_thumb())
        } else {
            ("┆", th.scroll_track())
        };
        if let Some(cell) = buf.cell_mut((x, body.y + r as u16)) {
            cell.set_symbol(sym);
            cell.set_style(style);
        }
    }
}

fn draw_outline(f: &mut Frame, app: &App, panel: Rect) {
    let th = &app.theme;
    let focused = app.focus == Focus::Outline;
    let inner_width = panel.width.saturating_sub(2).max(1) as usize;
    let rows = crate::render::outline_lines(&app.doc.headings, inner_width, app.sel, *th);

    let title = if focused {
        "Outline  (esc back)"
    } else {
        "Outline"
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if focused {
            th.outline_selected()
        } else {
            th.frame()
        })
        .title(title)
        .title_style(th.outline_current());
    let height = panel.height.saturating_sub(2) as usize;
    let top = app.sel.saturating_sub(height.saturating_sub(1));
    let window: Vec<Line<'static>> = rows.iter().skip(top).take(height).cloned().collect();

    f.render_widget(Paragraph::new(Text::from(window)), block.inner(panel));
    f.render_widget(block, panel);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let th = &app.theme;
    let total = app.doc.lines.len();
    let cur = (app.scroll + 1).min(total.max(1));
    let pct = if total == 0 {
        0
    } else {
        (100.0 * (app.scroll + app.viewport() / 2) as f64 / total as f64).clamp(0.0, 100.0) as u16
    };

    let mut spans = vec![
        Span::styled(
            match app.focus {
                Focus::Text if !app.editing => " normal ",
                Focus::Outline => " OUTLINE ",
                _ => " SEARCH ",
            },
            th.outline_current(),
        ),
        Span::styled(
            // 多文件时多一个序号：一眼知道在同目录第几个，] [ 能切。
            if app.files.len() > 1 {
                format!(
                    " {} · {}/{} ",
                    app.file_name(),
                    app.file_idx + 1,
                    app.files.len()
                )
            } else {
                format!(" {} ", app.file_name())
            },
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" line {cur}/{total} ({pct}%) "), th.dim()),
    ];
    if !app.doc.warnings.is_empty() {
        spans.push(Span::styled(
            format!(" ⚠ {} ", app.doc.warnings.len()),
            th.warning(),
        ));
    }
    if app.editing {
        spans.push(Span::styled(
            format!("/{}▏", app.query),
            th.search_current(),
        ));
        // 边打字边编译，写了一半的正则（`(`、`[`）很常见，把错因直接显示在搜索框后面。
        match &app.search_err {
            Some(e) => spans.push(Span::styled(format!(" {e} "), th.warning())),
            None => spans.push(Span::styled(
                format!(" {} hits ", app.matches.len()),
                th.dim(),
            )),
        }
    } else if !app.query.is_empty() {
        let hit = if app.matches.is_empty() {
            0
        } else {
            app.idx + 1
        };
        spans.push(Span::styled(
            format!(" /{} {hit}/{} ", app.query, app.matches.len()),
            th.search_hit(),
        ));
    }
    if !app.status.is_empty() && !app.editing {
        spans.push(Span::styled(format!(" {} ", app.status), th.dim()));
    }
    spans.push(Span::styled(" ? help  q quit ", th.dim()));

    f.buffer_mut().set_style(area, th.status_bar());
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let th = &app.theme;
    let w = 52u16.min(area.width.saturating_sub(2));
    let rows: Vec<&str> = HELP.lines().collect();
    let h = (rows.len() as u16 + 2).min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let rect = Rect {
        x,
        y,
        width: w,
        height: h,
    };

    f.buffer_mut().reset();
    let help = Style::default()
        .bg(if th.light {
            ratatui::style::Color::White
        } else {
            ratatui::style::Color::Black
        })
        .patch(th.text());
    {
        let buf = f.buffer_mut();
        for yy in rect.y..rect.bottom() {
            for xx in rect.x..rect.right() {
                if let Some(c) = buf.cell_mut((xx, yy)) {
                    c.set_symbol(" ");
                    c.set_style(help);
                }
            }
        }
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(th.frame())
        .title(" Keys ");
    let lines: Vec<Line<'static>> = rows
        .iter()
        .take((h as usize).saturating_sub(2))
        .map(|s| Line::from(Span::styled(s.to_string(), th.dim())))
        .collect();
    f.render_widget(Paragraph::new(Text::from(lines)), block.inner(rect));
    f.render_widget(block, rect);
}

const HELP: &str = "j k ↑↓ scroll       d/u half page    f/b page
g/G top / bottom    o outline        m mouse
] [ prev/next file  gx follow link
/ regex search      n/N next/prev    Enter keep
  a|b  (ab)+  \\b   \\c \\C case off/on
Tab focus outline   Enter jump       ? close
q / Esc / Ctrl-C quit";

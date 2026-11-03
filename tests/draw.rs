//! view::draw 上屏验证：viewport 切片、面板宽度、极小终端、搜索行不占正文高度。

#[path = "support/mod.rs"]
mod support;

use mdview::app::App;
use mdview::view;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::{TerminalOptions, Viewport};
use support::{args, temp_md, theme};

/// 画一帧，返回每行的可见文本（去掉补白空格，宽字符保留）。
fn draw(app: &mut App, w: u16, h: u16) -> Vec<String> {
    let mut term = Terminal::with_options(
        TestBackend::new(w, h),
        TerminalOptions {
            viewport: Viewport::Fullscreen,
        },
    )
    .unwrap();
    app.set_size(w, h);
    term.draw(|f| view::draw(f, app)).unwrap();
    let buf = term.backend().buffer();
    let (w, h) = (buf.area().width, buf.area().height);
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| {
                    buf.cell((x, y))
                        .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' '))
                })
                .collect::<String>()
                .replace(' ', "")
        })
        .collect()
}

fn app_for(body: &str) -> App {
    App::open(args(&temp_md(body)), theme(), (80, 24)).unwrap()
}

#[test]
fn viewport_starts_at_the_scroll_offset() {
    let body: String = (1..=2000).map(|i| format!("- 第 {i} 行\n")).collect();
    let mut app = app_for(&body);
    assert!(app.doc.lines.len() > 1000, "fixture 太短测不出切片");

    app.scroll = 500;
    let screen = draw(&mut app, 70, 20);
    assert!(
        screen[0].contains(&app.doc.plain_of(500).replace(' ', "")),
        "第一行应是渲染后的第 500 行：{:?} vs {:?}",
        screen[0],
        app.doc.plain_of(500)
    );
}

#[test]
fn scroll_never_walks_past_the_bottom() {
    let mut app = app_for("# 只有一点内容\n\n正文\n");
    app.goto_bottom();
    let screen = draw(&mut app, 70, 20);
    assert!(screen.last().unwrap().contains("正文") || screen.last().unwrap().contains('%'));
}

#[test]
fn outline_stays_readable_on_a_60_column_terminal() {
    let mut app = app_for("# 甲\n\n正文\n\n## 乙乙乙乙乙乙乙乙乙乙\n\n更多\n");
    app.outline_open = true;
    draw(&mut app, 60, 20);
    assert!(
        (24..=40).contains(&app.panel_rect.width),
        "面板宽 {}",
        app.panel_rect.width
    );
    assert!(
        app.body_rect.width >= 20,
        "正文只剩 {}",
        app.body_rect.width
    );
    assert_eq!(app.panel_rect.right(), 60, "面板贴右边");
    assert_eq!(
        app.body_rect.right() + 1,
        app.panel_rect.x,
        "正文与面板之间留一条分隔线"
    );
}

#[test]
fn search_line_borrows_the_status_row_not_the_body() {
    let mut app = app_for("# 标题\n\n搜索词 在这里\n");
    draw(&mut app, 60, 18);
    let body = app.body_rect;
    app.begin_search();
    app.push_query('搜');
    let screen = draw(&mut app, 60, 18);
    assert_eq!(app.body_rect, body, "输入搜索词不该改变正文高度");
    assert!(
        screen.last().unwrap().contains("搜"),
        "搜索词应画在状态栏那一行：{:?}",
        screen.last()
    );
}

#[test]
fn tiny_terminal_shows_a_hint_instead_of_crashing() {
    let mut app = app_for("# 标题\n\n正文\n");
    let screen = draw(&mut app, 10, 3);
    assert!(screen[0].contains("Terminal"), "{screen:?}");
}

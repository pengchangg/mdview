//! 按键、搜索导航、大纲跟随、尺寸变化 —— 全走 App 逻辑，不起终端。

#[path = "support/mod.rs"]
mod support;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
use mdview::app::Focus;
use support::{args, temp_md, theme};

const DOC: &str = "\
# 第一章

第一章的正文。

## 第二节

第二节正文。

### 第三节

第三节正文，长一点好滚动。
";

/// 重复几遍凑够能滚动的长度。
fn long_doc() -> String {
    DOC.repeat(8)
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn app(src: &str, size: (u16, u16)) -> mdview::App {
    let path = temp_md(src);
    mdview::App::open(args(&path), theme(), size).unwrap()
}

#[test]
fn scroll_keys_move_one_line_and_page() {
    let mut a = app(&long_doc(), (100, 20));
    assert_eq!(a.scroll, 0);
    a.on_key(key(KeyCode::Char('j')));
    assert_eq!(a.scroll, 1);
    a.on_key(key(KeyCode::Char('k')));
    a.on_key(key(KeyCode::Char('k')));
    assert_eq!(a.scroll, 0, "顶部不能再往上滚");
    a.on_key(key(KeyCode::Char('d')));
    assert_eq!(a.scroll, 9, "d = 半页 (viewport 19 / 2)");
    a.on_key(key(KeyCode::Char('G')));
    let bottom = a.scroll;
    assert!(bottom > 0);
    a.on_key(key(KeyCode::Char('G')));
    assert_eq!(a.scroll, bottom, "到底了再按 G 不动");
    a.on_key(key(KeyCode::Char('g')));
    assert_eq!(a.scroll, 0);
}

#[test]
fn arrow_keys_and_home_end_match_j_k_g_g() {
    let mut a = app(&long_doc(), (100, 20));
    a.on_key(key(KeyCode::Down));
    a.on_key(key(KeyCode::Down));
    a.on_key(key(KeyCode::Up));
    assert_eq!(a.scroll, 1);
    a.on_key(key(KeyCode::End));
    let e = a.scroll;
    a.on_key(key(KeyCode::Char('G')));
    assert_eq!(e, a.scroll);
    a.on_key(key(KeyCode::Home));
    assert_eq!(a.scroll, 0);
}

#[test]
fn ctrl_c_quits() {
    let mut a = app(&long_doc(), (100, 20));
    a.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(a.quit);
}

#[test]
fn release_events_are_ignored() {
    let mut a = app(&long_doc(), (100, 20));
    let mut k = key(KeyCode::Char('j'));
    k.kind = KeyEventKind::Release;
    a.on_key(k);
    assert_eq!(a.scroll, 0);
}

#[test]
fn unknown_keys_do_nothing() {
    let mut a = app(&long_doc(), (100, 20));
    for c in ['x', 'z', '1', '@'] {
        a.on_key(key(KeyCode::Char(c)));
    }
    assert_eq!(
        (a.scroll, a.quit, a.help, a.query.as_str()),
        (0, false, false, "")
    );
}

#[test]
fn search_typing_jumps_to_first_hit_at_or_after_view() {
    let mut a = app(&long_doc(), (100, 20));
    a.on_key(key(KeyCode::Char('/')));
    assert!(a.editing);
    for c in "第三节".chars() {
        a.on_key(key(KeyCode::Char(c)));
    }
    assert!(!a.matches.is_empty());
    let hit = a.matches[a.idx].line;
    assert!(hit >= 1, "第一个命中应在当前位置之后: {hit}");
    assert!(
        a.scroll <= hit && hit < a.scroll + a.viewport(),
        "应滚到命中行"
    );
}

#[test]
fn search_n_wraps_around_and_n_goes_back() {
    let mut a = app(&long_doc(), (100, 20));
    a.set_query("正文");
    let n = a.matches.len();
    assert!(n >= 3, "样例应至少有 3 处命中");
    a.on_key(key(KeyCode::Char('n')));
    assert_eq!(a.idx, 1);
    for _ in 0..n - 1 {
        a.on_key(key(KeyCode::Char('n')));
    }
    assert_eq!(a.idx, 0, "n 应该环绕回第一条");
    a.on_key(key(KeyCode::Char('N')));
    assert_eq!(a.idx, n - 1, "N 应该环绕到末条");
}

#[test]
fn search_without_match_reports_and_does_not_move() {
    let mut a = app(&long_doc(), (100, 20));
    a.on_key(key(KeyCode::Char('j')));
    let before = a.scroll;
    a.begin_search();
    for c in "zzzz".chars() {
        a.push_query(c);
    }
    a.end_search();
    assert!(a.matches.is_empty());
    assert_eq!(a.scroll, before, "没匹配就不该滚动");
    assert!(
        a.status.contains("no match"),
        "状态栏要说明没找到: {}",
        a.status
    );
}

#[test]
fn esc_clears_search_then_quits() {
    let mut a = app(&long_doc(), (100, 20));
    a.set_query("正文");
    a.on_key(key(KeyCode::Esc));
    assert!(a.query.is_empty() && !a.quit, "第一次 Esc 只清搜索词");
    a.on_key(key(KeyCode::Esc));
    assert!(a.quit, "没有搜索词时 Esc 退出");
}

#[test]
fn cancel_search_restores_scroll() {
    let mut a = app(&long_doc(), (100, 20));
    for _ in 0..12 {
        a.scroll_by(1);
    }
    let before = a.scroll;
    a.begin_search();
    for c in "第三节".chars() {
        a.push_query(c);
    }
    a.cancel_search();
    assert_eq!(a.scroll, before, "取消搜索应回到原位置");
    assert!(a.query.is_empty() && a.matches.is_empty());
}

#[test]
fn outline_opens_focused_at_current_position() {
    let mut a = app(&long_doc(), (100, 30));
    a.jump_line(a.headings()[2].line);
    let want = 2;
    a.on_key(key(KeyCode::Char('o')));
    assert!(a.outline_open);
    assert_eq!(a.focus, Focus::Outline);
    assert_eq!(a.sel, want, "选中项应跟着读到哪儿");
    assert!(a.text_width() < 100, "大纲占了正文宽度");
}

#[test]
fn outline_nav_and_enter_jumps() {
    let mut a = app(&long_doc(), (100, 30));
    a.on_key(key(KeyCode::Tab));
    assert_eq!(a.focus, Focus::Text, "大纲没开时 Tab 不转移焦点");
    a.toggle_outline(true);
    a.sel = 0;
    a.on_key(key(KeyCode::Char('j')));
    a.on_key(key(KeyCode::Char('j')));
    assert_eq!(a.sel, 2);
    let line = a.headings()[2].line;
    a.on_key(key(KeyCode::Enter));
    assert!(
        a.scroll <= line && line < a.scroll + a.viewport(),
        "跳转后目标行应在视口内"
    );
    a.on_key(key(KeyCode::Esc));
    assert_eq!(a.focus, Focus::Text, "Esc 从大纲回到正文");
}

#[test]
fn outline_stays_clamped_to_the_ends() {
    let mut a = app(&long_doc(), (100, 30));
    a.toggle_outline(true);
    for _ in 0..50 {
        a.outline_step(1);
    }
    assert_eq!(a.sel, a.headings().len() - 1);
    for _ in 0..50 {
        a.outline_step(-1);
    }
    assert_eq!(a.sel, 0);
}

#[test]
fn width_change_rerenders_but_height_change_does_not() {
    // 需要一段足够长的正文，变窄才会真的折出更多行。
    let long = "这是一段用来测试软换行的长文本，".repeat(12);
    let src = format!("{}\n{long}\n", long_doc());
    let mut a = app(&src, (100, 30));
    let wide = a.doc.lines.len();
    a.set_size(40, 30);
    let narrow = a.doc.lines.len();
    assert!(narrow > wide, "变窄应折出更多行: {wide} -> {narrow}");
    assert_eq!(a.text_width(), 39);
    let before = a.doc.lines.len();
    a.set_size(40, 10);
    assert_eq!(a.doc.lines.len(), before, "只改高度不该重渲染");
    assert!(a.scroll <= a.max_scroll(), "变矮之后滚动位置要夹住");
}

#[test]
fn max_width_caps_the_text_columns() {
    let mut a = app(&long_doc(), (200, 30));
    a.args.max_width = Some(60);
    a.rerender();
    assert_eq!(a.text_width(), 60);
    assert!(a.doc.lines.iter().all(|l| l.width() <= 60));
}

#[test]
fn reload_picks_up_new_content_and_keeps_scroll() {
    let mut a = app("# 原文件\n\n内容\n", (80, 20));
    let path = a.path.clone();
    a.scroll_by(3);
    let before = a.scroll;
    std::fs::write(&path, "# 原文件\n\n内容\n\n## 新加的\n\n更多文字\n").unwrap();
    a.reload();
    assert!(
        a.doc.headings.iter().any(|h| h.text == "新加的"),
        "热更新没生效"
    );
    assert_eq!(a.scroll, before, "重载不该把位置弹回顶部");
    assert!(a.status.contains("reloaded"), "{}", a.status);
}

#[test]
fn reload_of_unreadable_file_keeps_the_screen() {
    let mut a = app("# 好文件\n", (80, 20));
    let lines = a.doc.lines.len();
    a.path = "/nonexistent/路径/nope.md".into();
    a.reload();
    assert_eq!(a.doc.lines.len(), lines, "读不到文件时保持上一版");
    assert!(a.status.contains("read failed"), "{}", a.status);
}

#[test]
fn mouse_wheel_scrolls_the_text_area() {
    let mut a = app(&long_doc(), (100, 20));
    a.body_rect = ratatui::layout::Rect::new(0, 0, 100, 19);
    let wheel = |kind| MouseEvent {
        kind,
        column: 10,
        row: 5,
        modifiers: KeyModifiers::NONE,
    };
    a.on_mouse(wheel(MouseEventKind::ScrollDown));
    assert_eq!(a.scroll, 3);
    a.on_mouse(wheel(MouseEventKind::ScrollUp));
    assert_eq!(a.scroll, 0, "顶部滚轮不再为负");
}

#[test]
fn clicking_the_outline_selects_and_jumps() {
    let mut a = app(&long_doc(), (100, 30));
    a.toggle_outline(true);
    a.sel = 0;
    a.body_rect = ratatui::layout::Rect::new(0, 0, 60, 29);
    a.panel_rect = ratatui::layout::Rect::new(69, 0, 31, 29);
    a.on_mouse(MouseEvent {
        kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 75,
        row: 4,
        modifiers: KeyModifiers::NONE,
    });
    assert_eq!(a.focus, Focus::Outline);
    assert!(a.sel > 0, "点击应选中对应标题, sel={}", a.sel);
}

#[test]
fn help_overlay_closes_on_any_key() {
    let mut a = app(&long_doc(), (100, 20));
    a.on_key(key(KeyCode::Char('?')));
    assert!(a.help);
    a.on_key(key(KeyCode::Char('x')));
    assert!(!a.help);
    assert!(!a.quit);
}

#[test]
fn handle_applies_a_batch_of_events() {
    let mut a = app(&long_doc(), (100, 20));
    a.handle(vec![
        mdview::event::Ev::Key(key(KeyCode::Char('j'))),
        mdview::event::Ev::Key(key(KeyCode::Char('j'))),
        mdview::event::Ev::Resize(50, 20),
        mdview::event::Ev::Key(key(KeyCode::Char('j'))),
    ]);
    assert_eq!(a.text_width(), 49);
    assert_eq!(a.scroll, 3);
}

#[test]
fn bom_does_not_leak_into_the_first_heading() {
    let a = app("\u{feff}# 标题\n\n正文\n", (60, 12));
    assert_eq!(a.headings()[0].text, "标题", "BOM 得在读取时剔掉");
    assert!(!a.doc.plain_of(0).contains('\u{feff}'));
}

#[test]
fn ctrl_u_clears_the_search_box() {
    let mut a = app(&long_doc(), (100, 20));
    a.begin_search();
    a.push_query('第');
    a.push_query('三');
    a.on_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert!(a.query.is_empty(), "Ctrl-U 应清空搜索词");
    assert!(a.matches.is_empty());
    // 其余 Ctrl-* 不该被当成字符塞进词里。
    a.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
    assert!(a.query.is_empty());
}

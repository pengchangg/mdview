//! gx 跟随链接：锚点、兄弟文件、目录、找不到的目标、以及「URL 只报不打开」。

#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mdview::App;
use support::{args, temp_dir, theme};

fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

/// n 个独立段落，用来把标题和链接挤到视口外面。
fn pad(n: usize) -> String {
    (1..=n).map(|i| format!("padding line {i}\n\n")).collect()
}

/// 一个目录里放 a.md（正文由 body 给）+ 若干兄弟文件。
fn open_a(body: &str, extra: &[(&str, &str)]) -> (PathBuf, App) {
    let dir = temp_dir();
    let a = dir.join("a.md");
    std::fs::write(&a, body).unwrap();
    for (name, text) in extra {
        std::fs::write(dir.join(name), text).unwrap();
    }
    let app = App::open(args(&a), theme(), (100, 20)).unwrap();
    let a = a.canonicalize().unwrap_or(a);
    (a, app)
}

/// 把光标挪到第一条链接那一行，按下 g 再按下 x；返回那次的起始行。
fn follow(app: &mut App) -> usize {
    let line = app.doc.links.first().expect("文档里没有链接").line;
    app.scroll = line;
    app.on_key(key('g'));
    assert!(app.pending_g, "g 该留下前缀");
    app.on_key(key('x'));
    line
}

fn selected_heading(app: &App) -> String {
    app.headings()
        .get(app.sel)
        .map(|h| h.text.clone())
        .unwrap_or_default()
}

#[test]
fn links_are_recorded_with_their_rendered_line() {
    let (_, app) = open_a(&format!("{}See [the guide](other.md).\n", pad(3)), &[]);
    assert_eq!(app.doc.links.len(), 1);
    let link = &app.doc.links[0];
    assert_eq!(link.text, "the guide");
    assert_eq!(link.url, "other.md");
    assert!(app.doc.plain_of(link.line).contains("the guide"));
}

#[test]
fn an_anchor_jumps_to_its_heading() {
    let body = format!("{}# Target\n\n# Getting Started\n\ntail\n", pad(30));
    let (_, mut app) = open_a(&format!("{body}[jump](#target)\n"), &[]);
    follow(&mut app);
    assert_eq!(selected_heading(&app), "Target");
    let line = app.doc.headings[app.sel].line;
    assert!(
        app.scroll <= line && line < app.scroll + app.viewport(),
        "标题 {line} 得在视口里：scroll {}",
        app.scroll
    );
}

/// GitHub 的 slug 写法要能用：`#getting-started` 对上「Getting Started」。
#[test]
fn anchor_matching_is_slug_style() {
    let body = format!("{}# Getting Started\n\nmore\n", pad(30));
    let (_, mut app) = open_a(&format!("{body}[start](#getting-started)\n"), &[]);
    follow(&mut app);
    assert_eq!(selected_heading(&app), "Getting Started");
    assert!(app.status.is_empty(), "{}", app.status);
}

#[test]
fn a_missing_anchor_says_so_and_does_not_move() {
    let body = format!("{}# Only\n\nmore\n", pad(30));
    let (_, mut app) = open_a(&format!("{body}[nope](#nowhere)\n"), &[]);
    let line = follow(&mut app);
    assert!(app.status.contains("no heading"), "{}", app.status);
    assert_eq!(app.scroll, line, "找不到锚点就不该动");
}

#[test]
fn a_duplicated_anchor_takes_the_first_and_says_how_many() {
    let body = format!("{}# Dup\n\nmid\n\n# Dup\n\ntail\n", pad(30));
    let (_, mut app) = open_a(&format!("{body}[dup](#dup)\n"), &[]);
    follow(&mut app);
    assert_eq!(selected_heading(&app), "Dup");
    assert_eq!(app.doc.headings[app.sel].line, app.doc.headings[0].line);
    assert!(app.status.contains("2 headings"), "{}", app.status);
}

#[test]
fn a_relative_link_switches_file() {
    let extra = [("b.md", "# Beta\n\nbeta body\n")];
    let (a, mut app) = open_a(&format!("{}See [b](b.md).\n", pad(3)), &extra);
    assert_eq!(app.file_name(), "a.md");
    follow(&mut app);
    assert_eq!(app.file_name(), "b.md");
    assert_eq!(app.path.parent(), a.parent());
    assert!(app.doc.plain.join(" ").contains("beta body"));
    assert_eq!(app.scroll, 0, "换文件从头看");
}

#[test]
fn a_link_with_an_anchor_switches_then_jumps() {
    let b = format!("{}# Target\n\nend\n", pad(30));
    let extra = [("b.md", b.as_str())];
    let (_, mut app) = open_a(&format!("{}Go [deep](b.md#target).\n", pad(3)), &extra);
    follow(&mut app);
    assert_eq!(app.file_name(), "b.md");
    assert_eq!(selected_heading(&app), "Target");
    let line = app.doc.headings[app.sel].line;
    assert!(app.scroll <= line && line < app.scroll + app.viewport());
}

#[test]
fn a_link_into_a_subdirectory_swaps_in_its_listing() {
    let dir = temp_dir();
    std::fs::write(dir.join("a.md"), format!("{}Down [one](sub/).\n", pad(3))).unwrap();
    let sub = dir.join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("one.md"), "# One\n").unwrap();
    std::fs::write(sub.join("two.md"), "# Two\n").unwrap();
    let mut app = App::open(args(&dir.join("a.md")), theme(), (100, 20)).unwrap();
    follow(&mut app);
    assert_eq!(app.file_name(), "one.md");
    assert_eq!(app.files.len(), 2, "换目录就该换成那个目录的文件");
    assert!(Path::new(&app.path).starts_with(&sub));
}

#[test]
fn a_missing_file_is_reported_and_changes_nothing() {
    let (a, mut app) = open_a(&format!("{}See [gone](ghost.md).\n", pad(3)), &[]);
    follow(&mut app);
    assert!(app.status.contains("not found"), "{}", app.status);
    assert_eq!(app.path, a);
    assert_eq!(app.file_name(), "a.md");
}

/// 外部链接不去拉浏览器，只把地址报到状态栏。
#[test]
fn a_url_only_shows_up_in_the_status_bar() {
    let (a, mut app) = open_a(
        &format!("{}Visit [site](https://example.com/x).\n", pad(3)),
        &[],
    );
    follow(&mut app);
    assert!(app.status.contains("https://example.com/x"));
    assert!(app.status.contains("ctrl-click"));
    assert_eq!(app.path, a);
    assert!(app.scroll > 0, "gx 失败不该跳顶");
}

#[test]
fn no_link_under_the_cursor_is_reported() {
    let (_, mut app) = open_a(&format!("{}plain text, no links here\n", pad(3)), &[]);
    app.on_key(key('g'));
    app.on_key(key('x'));
    assert!(app.status.contains("no link at cursor"), "{}", app.status);
    assert_eq!(app.scroll, 0, "gx 失败不该顺手跳顶");
}

/// 前缀只吃 x：g 后面跟别的键，照旧是跳顶 + 那个键本身。
#[test]
fn any_key_after_g_is_still_a_jump_to_top() {
    let (_, mut app) = open_a(&format!("{}text\n", pad(60)), &[]);
    app.goto_bottom();
    app.on_key(key('g'));
    app.on_key(key('j'));
    assert_eq!(app.scroll, 1, "g 之后按 j = 跳顶再滚一行");
    assert!(!app.pending_g);
}

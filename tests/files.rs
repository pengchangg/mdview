//! 目录浏览：起始文件、自然序、`]` `[` 环绕、外部增删跟得上。

#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mdview::files;
use support::{args, temp_dir, theme};

fn names(files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect()
}

/// 建一个塞好文件的目录，返回目录和按写入顺序的文件名。
fn dir_with(files: &[(&str, &str)]) -> PathBuf {
    let dir = temp_dir();
    for (name, body) in files {
        std::fs::write(dir.join(name), format!("# {name}\n\n{body}\n")).unwrap();
    }
    dir
}

fn app_at(path: &Path) -> mdview::App {
    mdview::App::open(args(path), theme(), (80, 24)).unwrap()
}

fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

// ---- 目标解析 ----

#[test]
fn a_directory_starts_at_the_first_md() {
    let dir = dir_with(&[("b.md", "x"), ("a.md", "x")]);
    let (files, start) = files::resolve_targets(std::slice::from_ref(&dir), &dir).unwrap();
    assert_eq!(names(&files), ["a.md", "b.md"]);
    assert_eq!(start, 0);
}

#[test]
fn a_file_gets_its_siblings_and_starts_at_itself() {
    let dir = dir_with(&[("a.md", "a"), ("b.md", "b"), ("c.md", "c")]);
    let (files, start) = files::resolve_targets(&[dir.join("b.md")], &dir).unwrap();
    assert_eq!(names(&files), ["a.md", "b.md", "c.md"]);
    assert_eq!(start, 1);
}

/// 数字段按数值比：part2 在 part10 前面，不是字典序。
#[test]
fn numbers_sort_as_numbers() {
    let dir = dir_with(&[
        ("part10.md", "x"),
        ("part2.md", "x"),
        ("Part3.md", "x"),
        ("part1.md", "x"),
    ]);
    let (files, _) = files::resolve_targets(&[dir.join("part1.md")], &dir).unwrap();
    assert_eq!(
        names(&files),
        ["part1.md", "part2.md", "Part3.md", "part10.md"]
    );
}

#[test]
fn markdown_extension_and_case_are_both_accepted() {
    let dir = dir_with(&[
        ("a.MD", "x"),
        ("b.markdown", "x"),
        ("c.txt", "x"),
        ("sub.md.bak", "x"),
    ]);
    let listed = files::list_md(&dir);
    assert_eq!(names(&listed), ["a.MD", "b.markdown"]);
}

/// 非 .md 的文件也要能看：插进兄弟列表，起始就是它。
#[test]
fn a_non_md_file_still_opens() {
    let dir = dir_with(&[("a.md", "a"), ("b.md", "b")]);
    let note = dir.join("notes.txt");
    std::fs::write(&note, "not markdown\n").unwrap();
    let (files, start) = files::resolve_targets(std::slice::from_ref(&note), &dir).unwrap();
    assert_eq!(names(&files), ["a.md", "b.md", "notes.txt"]);
    assert_eq!(start, 2);
}

#[test]
fn an_empty_directory_is_an_error_that_names_it() {
    let dir = temp_dir();
    let err = files::resolve_targets(std::slice::from_ref(&dir), &dir).unwrap_err();
    assert!(
        err.to_string().contains("no .md file in"),
        "{}",
        err.to_string()
    );
}

/// 没有参数 = 当前目录；这里把 cwd 传成测试目录，行为同上。
#[test]
fn no_argument_means_the_current_directory() {
    let dir = dir_with(&[("only.md", "x")]);
    let (files, start) = files::resolve_targets(&[], &dir).unwrap();
    assert_eq!(names(&files), ["only.md"]);
    assert_eq!(start, 0);
}

// ---- 切换 ----

#[test]
fn bracket_keys_walk_the_siblings_and_wrap() {
    let dir = dir_with(&[("a.md", "alpha"), ("b.md", "beta"), ("c.md", "gamma")]);
    let mut app = app_at(&dir.join("b.md"));
    assert_eq!(app.files.len(), 3);
    assert_eq!(app.file_idx, 1);

    app.on_key(key(']'));
    assert_eq!(app.file_name(), "c.md");
    assert!(app.doc.plain.join(" ").contains("gamma"));
    app.on_key(key(']'));
    assert_eq!(app.file_name(), "a.md", "到底就该绕回第一个");
    app.on_key(key('['));
    assert_eq!(app.file_name(), "c.md", "[ 同样绕回末尾");
}

#[test]
fn switching_files_resets_scroll_and_search() {
    let dir = dir_with(&[("a.md", "alpha\n\nneedle\n"), ("b.md", "beta\n")]);
    let mut app = app_at(&dir.join("a.md"));
    app.set_query("needle");
    app.scroll = 2;
    app.on_key(key(']'));
    assert_eq!(app.file_name(), "b.md");
    assert!(
        app.query.is_empty() && app.matches.is_empty(),
        "搜索不该跟过去"
    );
    assert_eq!(app.scroll, 0);
}

#[test]
fn one_file_in_the_directory_makes_the_brackets_noops() {
    let dir = dir_with(&[("solo.md", "only")]);
    let mut app = app_at(&dir.join("solo.md"));
    app.on_key(key(']'));
    app.on_key(key('['));
    assert_eq!(app.file_name(), "solo.md");
    assert_eq!(app.files.len(), 1);
}

// ---- 外部改了目录 ----

#[test]
fn a_new_sibling_joins_the_list() {
    let dir = dir_with(&[("a.md", "a")]);
    let mut app = app_at(&dir.join("a.md"));
    std::fs::write(dir.join("z.md"), "# z\n").unwrap();
    app.refresh_listing();
    assert_eq!(names(&app.files), ["a.md", "z.md"]);
    assert_eq!(app.file_idx, 0, "当前文件插到哪儿要跟着算");
}

#[test]
fn a_deleted_sibling_leaves_the_index_right() {
    let dir = dir_with(&[("a.md", "a"), ("b.md", "b"), ("c.md", "c")]);
    let mut app = app_at(&dir.join("c.md"));
    std::fs::remove_file(dir.join("a.md")).unwrap();
    app.refresh_listing();
    assert_eq!(names(&app.files), ["b.md", "c.md"]);
    assert_eq!(app.file_idx, 1);
    app.on_key(key(']'));
    assert_eq!(app.file_name(), "b.md");
}

/// 当前文件被删掉：内容留在屏幕上，还能切去别的文件。
#[test]
fn deleting_the_open_file_keeps_the_screen() {
    let dir = dir_with(&[("a.md", "alpha"), ("b.md", "beta")]);
    let mut app = app_at(&dir.join("a.md"));
    std::fs::remove_file(dir.join("a.md")).unwrap();
    app.refresh_listing();
    assert_eq!(app.file_name(), "a.md");
    assert!(app.doc.plain.join("").contains("alpha"));
    app.on_key(key(']'));
    assert_eq!(app.file_name(), "b.md");
}

/// 改内容还是走原来的热更新路径，不该被列表刷新搅坏。
#[test]
fn a_file_changed_on_disk_still_reloads() {
    let dir = dir_with(&[("a.md", "old")]);
    let mut app = app_at(&dir.join("a.md"));
    std::fs::write(dir.join("a.md"), "# new\n\nheadline\n").unwrap();
    app.reload();
    assert!(app.doc.plain.join("").contains("headline"));
    assert!(app.status.contains("reloaded"), "{}", app.status);
}

//! 配置文件：认哪些键、怎么报错、跟命令行怎么排序。

#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};

use mdview::cli::{MermaidMode, ThemeMode};
use mdview::config::{self, Config};
use support::{args, temp_dir};

fn write_config(text: &str) -> PathBuf {
    let path = temp_dir().join("config.toml");
    std::fs::write(&path, text).unwrap();
    path
}

/// `args()` 把 theme/mermaid 钉成 Dark/Off（快照要稳），测配置文件就得先把它们放开。
fn bare(path: &Path) -> mdview::cli::Args {
    let mut a = args(path);
    a.theme = None;
    a.mermaid = None;
    a
}

fn loaded(text: &str) -> Config {
    config::read(&write_config(text)).unwrap_or_else(|e| panic!("should load: {e}"))
}

#[test]
fn empty_file_is_all_defaults() {
    let c = loaded("");
    assert_eq!(c.max_width, None);
    assert_eq!(c.theme, None);
    assert_eq!(c.no_watch, None);
}

#[test]
fn every_key_is_read() {
    let c = loaded(
        "max_width = 100\ntheme = \"light\"\nmermaid = \"off\"\noutline = true\nno_watch = true\nno_mouse = true\n",
    );
    assert_eq!(c.max_width, Some(100));
    assert_eq!(c.theme.as_deref(), Some("light"));
    assert_eq!(c.mermaid.as_deref(), Some("off"));
    assert_eq!(
        (c.outline, c.no_watch, c.no_mouse),
        (Some(true), Some(true), Some(true))
    );
}

/// 打错的键名必须报出来，还要带上是哪个文件 —— 静悄悄忽略等于让用户以为生效了。
#[test]
fn unknown_key_names_the_key_and_the_file() {
    let path = write_config("max_widht = 100\n");
    let err = config::read(&path).unwrap_err();
    assert!(err.contains("max_widht"), "{err}");
    assert!(err.contains(&path.display().to_string()), "{err}");
}

#[test]
fn bad_toml_names_the_file() {
    let path = write_config("max_width = \n");
    let err = config::read(&path).unwrap_err();
    assert!(err.contains(&path.display().to_string()), "{err}");
}

/// 类型写错也不能退默认值：`theme = 3` 说得清是哪一项。
#[test]
fn wrong_type_names_the_key() {
    let path = write_config("max_width = \"100\"\n");
    let err = config::read(&path).unwrap_err();
    assert!(err.contains("max_width") && err.contains("number"), "{err}");

    let path = write_config("no_watch = \"yes\"\n");
    let err = config::read(&path).unwrap_err();
    assert!(err.contains("no_watch"), "{err}");
}

#[test]
fn unreadable_file_reports_the_path() {
    let path = temp_dir().join("nope.toml");
    assert!(matches!(config::read(&path), Err(e) if e.contains("nope.toml")));
}

// ---- 优先级：命令行 > 配置文件 > 内置默认 ----

#[test]
fn config_file_fills_what_the_command_line_left_out() {
    let dir = temp_dir();
    let mut a = bare(&dir.join("t.md"));
    let notes = config::merge(
        &mut a,
        &Config {
            max_width: Some(100),
            theme: Some("light".into()),
            mermaid: Some("off".into()),
            outline: Some(true),
            no_watch: Some(true),
            no_mouse: Some(true),
        },
    );
    assert_eq!(a.max_width, Some(100));
    assert_eq!(a.theme, Some(ThemeMode::Light));
    assert_eq!(a.mermaid, Some(MermaidMode::Off));
    assert_eq!(
        (a.outline, a.no_watch, a.no_mouse),
        (Some(true), Some(true), Some(true))
    );
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn command_line_wins_over_the_file() {
    let mut a = args(&temp_dir().join("t.md"));
    a.max_width = Some(40);
    a.theme = Some(ThemeMode::Dark);
    a.mermaid = Some(MermaidMode::Auto);
    a.outline = Some(false);
    let notes = config::merge(
        &mut a,
        &Config {
            max_width: Some(100),
            theme: Some("light".into()),
            mermaid: Some("off".into()),
            outline: Some(true),
            ..Default::default()
        },
    );
    assert_eq!(a.max_width, Some(40));
    assert_eq!(a.theme, Some(ThemeMode::Dark));
    assert_eq!(a.mermaid, Some(MermaidMode::Auto));
    assert_eq!(a.outline, Some(false), "命令行显式关掉的不能被文件打开");
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn nothing_at_all_falls_back_to_builtins() {
    let mut a = bare(&temp_dir().join("t.md"));
    config::merge(&mut a, &Config::default());
    assert_eq!(a.theme, Some(ThemeMode::Auto));
    assert_eq!(a.mermaid, Some(MermaidMode::Auto));
    assert_eq!(a.outline, None);
}

/// 一位数的正文宽没法看，夹到能用的下限并说一句。
#[test]
fn tiny_max_width_is_clamped_with_a_note() {
    let mut a = args(&temp_dir().join("t.md"));
    let notes = config::merge(
        &mut a,
        &Config {
            max_width: Some(0),
            ..Default::default()
        },
    );
    assert_eq!(a.max_width, Some(20));
    assert_eq!(notes, vec!["max_width 0 is too small, using 20"]);
}

/// 认不出的枚举值不致命：退默认，但要讲明白退成了什么、能写什么。
#[test]
fn unknown_enum_value_falls_back_with_a_note() {
    let mut a = bare(&temp_dir().join("t.md"));
    let notes = config::merge(
        &mut a,
        &Config {
            theme: Some("solarized".into()),
            ..Default::default()
        },
    );
    assert_eq!(a.theme, Some(ThemeMode::Auto));
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(
        notes[0].contains("solarized") && notes[0].contains("auto"),
        "{notes:?}"
    );
    assert!(notes[0].contains("light"), "该列出能写哪些值");
}

/// 合法值只有 clap 的 ValueEnum 一个来源，两边不会写歪。
#[test]
fn enum_values_are_case_insensitive() {
    let mut a = bare(&temp_dir().join("t.md"));
    config::merge(
        &mut a,
        &Config {
            theme: Some("LIGHT".into()),
            mermaid: Some("Off".into()),
            ..Default::default()
        },
    );
    assert_eq!(a.theme, Some(ThemeMode::Light));
    assert_eq!(a.mermaid, Some(MermaidMode::Off));
}

// ---- 找文件 ----

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let owned: Vec<(String, String)> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |k: &str| {
        owned
            .iter()
            .find(|(name, _)| name == k)
            .map(|(_, v)| v.clone())
    }
}

#[test]
fn mdview_config_env_wins_but_the_file_must_exist() {
    let path = write_config("outline = true\n");
    let found = config::locate(env(&[("MDVIEW_CONFIG", path.to_str().unwrap())]))
        .unwrap()
        .unwrap();
    assert_eq!(found, path);

    let missing = temp_dir().join("gone.toml");
    let err = config::locate(env(&[("MDVIEW_CONFIG", missing.to_str().unwrap())])).unwrap_err();
    assert!(
        err.contains("MDVIEW_CONFIG") && err.contains("gone.toml"),
        "{err}"
    );
}

#[test]
fn xdg_home_wins_and_does_not_fall_back_to_dot_config() {
    let xdg = temp_dir();
    let home = temp_dir();
    let in_xdg = xdg.join("mdview");
    std::fs::create_dir_all(&in_xdg).unwrap();
    let path = in_xdg.join("config.toml");
    std::fs::write(&path, "outline = true\n").unwrap();
    // ~/.config/mdview/config.toml 也在，但 XDG 设了就不该看它。
    let dot = home.join(".config/mdview");
    std::fs::create_dir_all(&dot).unwrap();
    std::fs::write(dot.join("config.toml"), "outline = false\n").unwrap();

    let found = config::locate(env(&[
        ("XDG_CONFIG_HOME", xdg.to_str().unwrap()),
        ("HOME", home.to_str().unwrap()),
    ]))
    .unwrap()
    .unwrap();
    assert_eq!(found, path);
}

#[test]
fn home_dot_config_is_the_default() {
    let home = temp_dir();
    let dir = home.join(".config/mdview");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, "").unwrap();
    let found = config::locate(env(&[("HOME", home.to_str().unwrap())]))
        .unwrap()
        .unwrap();
    assert_eq!(found, path);
}

#[test]
fn no_config_anywhere_is_ok() {
    let empty = temp_dir();
    let found = config::locate(env(&[("HOME", empty.to_str().unwrap())])).unwrap();
    assert_eq!(found, None);
}

/// main 里那条链：locate → read → merge；这里只保证 read 的错能带出路径。
#[test]
fn bad_config_reports_the_path_it_read() {
    let path: PathBuf = write_config("[");
    let err = config::read(&path).unwrap_err();
    let shown: &Path = &path;
    assert!(err.contains(&shown.display().to_string()), "{err}");
}

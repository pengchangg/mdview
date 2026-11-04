//! 用户配置文件：只认 6 个键，别的一律报错，不静悄悄吞掉打错的键名。
//! 优先级 CLI > 配置文件 > 内置默认，所以 `Args` 里这些字段都是 `Option`：
//! clap 不再填默认值，填了就说明命令行给过。

use std::path::{Path, PathBuf};

use clap::ValueEnum;

use crate::cli::{Args, MermaidMode, ThemeMode};

/// 配置文件认的键（错误提示里列出来，打错字时一眼看见该写什么）。
pub const KEYS: [&str; 6] = [
    "max_width",
    "theme",
    "mermaid",
    "outline",
    "no_watch",
    "no_mouse",
];

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub max_width: Option<u16>,
    pub theme: Option<String>,
    pub mermaid: Option<String>,
    pub outline: Option<bool>,
    pub no_watch: Option<bool>,
    pub no_mouse: Option<bool>,
}

/// 配置文件在哪：`MDVIEW_CONFIG`（指哪儿算哪儿，不存在就报错）→
/// `$XDG_CONFIG_HOME/mdview/config.toml` → `$HOME/.config/mdview/config.toml`。
/// 环境变量由调用方注入（edition 2024 下测试不能 set_var），返回 None = 没有配置文件。
pub fn locate(env: impl Fn(&str) -> Option<String>) -> Result<Option<PathBuf>, String> {
    if let Some(p) = env("MDVIEW_CONFIG") {
        let path = PathBuf::from(&p);
        if !path.is_file() {
            return Err(format!(
                "MDVIEW_CONFIG points at {} but there is no such file",
                path.display()
            ));
        }
        return Ok(Some(path));
    }
    let name: PathBuf = ["mdview", "config.toml"].iter().collect();
    let base: Option<PathBuf> = match (env("XDG_CONFIG_HOME"), env("HOME")) {
        // XDG 设了就不退回 ~/.config，免得两份配置互相打架。
        (Some(xdg), _) if !xdg.is_empty() => Some(PathBuf::from(xdg)),
        (_, Some(home)) if !home.is_empty() => Some(PathBuf::from(home).join(".config")),
        _ => None,
    };
    Ok(base.map(|b| b.join(name)).filter(|p| p.is_file()))
}

/// 读配置文件；不存在 = 全默认。语法错、类型错、未知键都在这里带上路径报出去。
/// 手写取值而不 `#[derive(Deserialize)]`：只有 6 个扁平键，引 serde 派生不值。
pub fn read(path: &Path) -> Result<Config, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let table: toml_edit::DocumentMut = text
        .parse()
        .map_err(|e: toml_edit::TomlError| format!("invalid config in {}:\n{e}", path.display()))?;
    for (key, _) in table.iter() {
        if !KEYS.contains(&key) {
            return Err(format!("unknown key `{key}` in {}", path.display()));
        }
    }
    Ok(Config {
        max_width: number(&table, "max_width", path)?,
        theme: string(&table, "theme", path)?,
        mermaid: string(&table, "mermaid", path)?,
        outline: flag(&table, "outline", path)?,
        no_watch: flag(&table, "no_watch", path)?,
        no_mouse: flag(&table, "no_mouse", path)?,
    })
}

fn string(t: &toml_edit::DocumentMut, key: &str, path: &Path) -> Result<Option<String>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .map(str::to_owned)
            .map(Some)
            .ok_or_else(|| format!("`{key}` must be a string in {}", path.display())),
    }
}

fn flag(t: &toml_edit::DocumentMut, key: &str, path: &Path) -> Result<Option<bool>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_bool()
            .map(Some)
            .ok_or_else(|| format!("`{key}` must be true or false in {}", path.display())),
    }
}

/// 比 u16 还宽的终端不存在，夹到上限而不是报错。
fn number(t: &toml_edit::DocumentMut, key: &str, path: &Path) -> Result<Option<u16>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_integer()
            .map(|n| Some(n.clamp(0, i64::from(u16::MAX)) as u16))
            .ok_or_else(|| format!("`{key}` must be a whole number in {}", path.display())),
    }
}

/// 配置文件填进命令行没给的键，返回要显示给用户的一句话（`theme`/`mermaid` 一定被填成具体值）。
pub fn merge(args: &mut Args, cfg: &Config) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(w) = cfg.max_width {
        // 0 会让正文一列宽，没意义；下限跟极窄终端的兜底一致，取 20。
        let clamped = w.max(20);
        if clamped != w {
            notes.push(format!("max_width {w} is too small, using {clamped}"));
        }
        args.max_width = args.max_width.or(Some(clamped));
    }
    args.theme = Some(pick(
        args.theme,
        cfg.theme.as_deref(),
        "theme",
        ThemeMode::Auto,
        &mut notes,
    ));
    args.mermaid = Some(pick(
        args.mermaid,
        cfg.mermaid.as_deref(),
        "mermaid",
        MermaidMode::Auto,
        &mut notes,
    ));
    args.outline = args.outline.or(cfg.outline);
    args.no_watch = args.no_watch.or(cfg.no_watch);
    args.no_mouse = args.no_mouse.or(cfg.no_mouse);
    notes
}

/// 枚举值只有一个合法写法来源：clap 的 `ValueEnum`，命令行和配置文件共用。
/// 配置文件里的值认不出来不致命 —— 退到内置默认，但要说明白退回了什么、能写什么。
fn pick<T: ValueEnum + Copy + PartialEq>(
    cli: Option<T>,
    file: Option<&str>,
    key: &str,
    default: T,
    notes: &mut Vec<String>,
) -> T {
    if let Some(from_cli) = cli {
        return from_cli;
    }
    let Some(raw) = file else { return default };
    if let Some(v) = variant_of::<T>(raw) {
        return v;
    }
    notes.push(format!(
        "unknown {key} \"{raw}\", using {} (want: {})",
        default
            .to_possible_value()
            .map_or_else(String::new, |p| p.get_name().to_owned()),
        wanted::<T>()
    ));
    default
}

fn variant_of<T: ValueEnum + Copy>(raw: &str) -> Option<T> {
    T::value_variants()
        .iter()
        .find(|v| {
            v.to_possible_value()
                .is_some_and(|p| p.get_name().eq_ignore_ascii_case(raw))
        })
        .copied()
}

fn wanted<T: ValueEnum + Copy>() -> String {
    T::value_variants()
        .iter()
        .filter_map(|v| v.to_possible_value())
        .map(|p| p.get_name().to_owned())
        .collect::<Vec<_>>()
        .join(", ")
}

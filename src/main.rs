use std::io::{IsTerminal, Stdout, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::Duration;

use clap::{CommandFactory, Parser};
use color_eyre::eyre::{bail, eyre};
use crossterm::terminal;
use mdview::{
    app::App,
    cli::{Args, Completion},
    config, event, term,
    theme::Theme,
    view, watch,
};
use ratatui::{Terminal, backend::CrosstermBackend};

fn main() -> color_eyre::Result<()> {
    let mut args = Args::parse();

    // 补全要在终端检测之前打：`mdview --print-completion zsh > ~/.zsh/...` 时 stdout 不是 tty。
    if let Some(shell) = args.print_completion {
        let mut cmd = Args::command();
        let kind = match shell {
            Completion::Bash => clap_complete::Shell::Bash,
            Completion::Zsh => clap_complete::Shell::Zsh,
            Completion::Fish => clap_complete::Shell::Fish,
        };
        clap_complete::generate(kind, &mut cmd, "mdview", &mut std::io::stdout());
        std::io::stdout().flush()?;
        return Ok(());
    }

    // 配置文件：MDVIEW_CONFIG → $XDG_CONFIG_HOME → ~/.config；只填命令行没给的键。
    let mut notes = match config::locate(|k| std::env::var(k).ok()).map_err(|e| eyre!("{e}"))? {
        Some(path) => config::merge(&mut args, &config::read(&path).map_err(|e| eyre!("{e}"))?),
        None => config::merge(&mut args, &config::Config::default()),
    };

    // 重定向到文件时没有终端可画，报一句话比吐一片转义码有用。
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!(
            "mdview needs an interactive terminal; use cat/less to read {}",
            args.file
                .map_or(String::from("."), |f| f.display().to_string())
        );
    }

    let size = terminal::size()?;
    let theme = Theme::detect(args.theme.unwrap_or(mdview::cli::ThemeMode::Auto));

    term::install_panic_hook();
    color_eyre::install()?;
    let guard = term::TermGuard::enter(!args.no_mouse.unwrap_or(false))?;

    let result = run(&args, theme, size, &mut notes);
    drop(guard); // 先把终端恢复正常，再打印可能的错误
    result
}

fn run(
    args: &Args,
    theme: Theme,
    size: (u16, u16),
    notes: &mut Vec<String>,
) -> color_eyre::Result<()> {
    let mut terminal: Terminal<CrosstermBackend<Stdout>> =
        Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    terminal.clear()?;
    let mut app = App::open(args.clone(), theme, size)?;

    let idle = if args.no_watch.unwrap_or(false) {
        Duration::from_millis(80)
    } else {
        event::TICK
    };
    let mut rx = spawn_watcher(&app, args, notes);
    app.status = notes.join("  "); // 配置文件的提醒（宽上限被夹过等）只在启动时看一眼
    let mut watch_dir = watch_dir_of(&app);

    loop {
        terminal.draw(|f| view::draw(f, &mut app))?;
        if app.quit {
            break;
        }
        // gx 跳到别的目录时监听要跟过去：换掉 rx，旧监听线程发不出去就自己退出并 drop 掉 watcher。
        let dir = watch_dir_of(&app);
        if dir != watch_dir {
            watch_dir = dir;
            rx = spawn_watcher(&app, args, notes);
        }
        let evs = event::collect(&rx, idle)?;
        if !evs.is_empty() {
            app.handle(evs);
        }
    }
    Ok(())
}

fn spawn_watcher(app: &App, args: &Args, notes: &mut Vec<String>) -> Receiver<()> {
    if args.no_watch.unwrap_or(false) {
        return std::sync::mpsc::channel::<()>().1;
    }
    match watch::spawn(app.path.clone()) {
        Ok(rx) => rx,
        Err(e) => {
            notes.push(format!("watcher unavailable: {e}"));
            std::sync::mpsc::channel::<()>().1
        }
    }
}

/// 监听的是文件所在目录（编辑器 rename-swap），所以换目录才算「换了监听对象」。
fn watch_dir_of(app: &App) -> PathBuf {
    app.path
        .parent()
        .map_or_else(|| Path::new(".").to_path_buf(), Path::to_path_buf)
}

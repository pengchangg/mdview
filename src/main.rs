use std::io::{IsTerminal, Stdout};
use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::bail;
use crossterm::terminal;
use mdview::{app::App, cli::Args, event, term, theme::Theme, view, watch};
use ratatui::{Terminal, backend::CrosstermBackend};

fn main() -> color_eyre::Result<()> {
    let args = Args::parse();

    // 重定向到文件时没有终端可画，报一句话比吐一片转义码有用。
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!(
            "mdview 需要交互终端；直接看原文请用 cat/less {}",
            args.file.display()
        );
    }
    if args.file.is_dir() {
        bail!("{} 是目录，v1 只预览单个 .md 文件", args.file.display());
    }

    let size = terminal::size()?;
    let theme = Theme::detect(args.theme);

    term::install_panic_hook();
    color_eyre::install()?;
    let guard = term::TermGuard::enter(!args.no_mouse)?;

    let result = run(&args, theme, size);
    drop(guard); // 先把终端恢复正常，再打印可能的错误
    result
}

fn run(args: &Args, theme: Theme, size: (u16, u16)) -> color_eyre::Result<()> {
    let mut terminal: Terminal<CrosstermBackend<Stdout>> =
        Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    terminal.clear()?;
    let mut app = App::open(args.clone(), theme, size)?;

    let rx = if args.no_watch {
        std::sync::mpsc::channel::<()>().1
    } else {
        match watch::spawn(app.path.clone()) {
            Ok(rx) => rx,
            Err(e) => {
                app.status = format!("监听不可用：{e}");
                std::sync::mpsc::channel::<()>().1
            }
        }
    };
    let idle = if args.no_watch {
        Duration::from_millis(80)
    } else {
        event::TICK
    };

    loop {
        terminal.draw(|f| view::draw(f, &mut app))?;
        if app.quit {
            break;
        }
        let evs = event::collect(&rx, idle)?;
        if !evs.is_empty() {
            app.handle(evs);
        }
    }
    Ok(())
}

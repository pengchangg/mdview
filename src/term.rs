//! 终端状态守卫：进入/退出备用屏、raw mode、鼠标捕获。
//!
//! panic 与正常退出走同一条 `restore()`，保证不留 raw mode、不泄漏鼠标捕获、光标可见。

use color_eyre::Result;
use crossterm::{
    cursor::{RestorePosition, SavePosition, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

pub struct TermGuard {
    pub mouse: bool,
}

pub fn restore(mouse: bool) {
    let _ = terminal::disable_raw_mode();
    let mut out = std::io::stdout();
    let _ = execute!(out, LeaveAlternateScreen);
    if mouse {
        let _ = execute!(out, DisableMouseCapture);
    }
    let _ = execute!(out, RestorePosition, Show);
}

impl TermGuard {
    pub fn enter(mouse: bool) -> Result<Self> {
        let mut out = std::io::stdout();
        execute!(out, SavePosition, EnterAlternateScreen)?;
        if mouse {
            execute!(out, EnableMouseCapture)?;
        }
        terminal::enable_raw_mode()?;
        Ok(Self { mouse })
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        restore(self.mouse);
    }
}

/// 运行中开关鼠标捕获（`m` 键）：关掉后终端可以自己选中文本复制。
pub fn set_mouse(on: bool) {
    let mut out = std::io::stdout();
    if on {
        let _ = execute!(out, EnableMouseCapture);
    } else {
        let _ = execute!(out, DisableMouseCapture);
    }
}

/// 装 panic hook：先把终端恢复干净，再打印报告。
pub fn install_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore(true);
        hook(info);
    }));
}

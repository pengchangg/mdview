//! 单一事件循环：键盘 + 文件监听，一次收齐再处理。
//!
//! 不引 tokio：监听线程只做「往 channel 投一个信号」这一件事，
//! 主循环用 `poll` 超时把两边合并，同一次 tick 内的多次 Resize/变更会合并成一次重绘。

use crossterm::event::{self, Event};
use std::sync::mpsc::Receiver;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum Ev {
    Key(event::KeyEvent),
    Mouse(event::MouseEvent),
    Resize(u16, u16),
    FileChanged,
    Focus(bool),
}

pub const TICK: Duration = Duration::from_millis(250);

/// 阻塞到第一个事件（或超时），然后把已经排好的事件一次取空。
pub fn collect(file_rx: &Receiver<()>, timeout: Duration) -> color_eyre::Result<Vec<Ev>> {
    let mut evs = Vec::new();
    if event::poll(timeout)? {
        push(&mut evs, event::read()?);
        while event::poll(Duration::ZERO)? {
            push(&mut evs, event::read()?);
        }
    }
    while file_rx.try_recv().is_ok() {
        if !evs.contains(&Ev::FileChanged) {
            evs.push(Ev::FileChanged);
        }
    }
    Ok(evs)
}

fn push(evs: &mut Vec<Ev>, ev: Event) {
    match ev {
        Event::Key(k) => evs.push(Ev::Key(k)),
        Event::Mouse(m) => evs.push(Ev::Mouse(m)),
        Event::Resize(w, h) => {
            if let Some(last) = evs.iter_mut().rev().find(|e| matches!(e, Ev::Resize(..))) {
                *last = Ev::Resize(w, h);
            } else {
                evs.push(Ev::Resize(w, h));
            }
        }
        Event::FocusLost => evs.push(Ev::Focus(false)),
        Event::FocusGained => evs.push(Ev::Focus(true)),
        _ => {}
    }
}

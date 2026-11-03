//! 文件监听：静默 250ms 后投一次「该重载了」。
//!
//! 监听**父目录**而不是文件本身 —— 编辑器的保存大多是写临时文件再 rename，
//! 直接监听文件会在第一次保存后就失联。

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::Duration;

use notify::{Event, RecursiveMode, Watcher};

pub fn spawn(target: PathBuf) -> color_eyre::Result<Receiver<()>> {
    let dir = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let name = target.file_name().map(|n| n.to_string_lossy().into_owned());

    let (tx_raw, rx_raw) = std::sync::mpsc::channel::<()>();
    let (tx, rx) = std::sync::mpsc::channel::<()>();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        let ev = match res {
            Ok(e) => e,
            Err(_) => return,
        };
        let hit = ev.paths.iter().any(|p| {
            p == &target || p.file_name().map(|n| n.to_string_lossy().into_owned()) == name
        });
        if hit {
            let _ = tx_raw.send(());
        }
    })?;
    watcher.watch(&dir, RecursiveMode::NonRecursive)?;

    std::thread::spawn(move || {
        while rx_raw.recv().is_ok() {
            // 把抖动合并掉：一直收到事件就继续等，安静 250ms 才通知一次。
            while rx_raw.recv_timeout(Duration::from_millis(250)).is_ok() {}
            if tx.send(()).is_err() {
                break;
            }
        }
        drop(watcher);
    });
    Ok(rx)
}

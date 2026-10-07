//! 文件监听：外部修改当前 md 时通知前端（提示 reload）
//!
//! 打开文件时 watch 其所在目录，过滤到该文件的增改删事件，emit 'file-changed'。
//! 保存后 note_saved 设一个短暂忽略窗口，避免自己写盘触发误报。

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

pub struct FileWatcher {
    inner: Mutex<Option<RecommendedWatcher>>,
    current: Mutex<Option<String>>,
    ignore_until: Mutex<u128>,
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            current: Mutex::new(None),
            ignore_until: Mutex::new(0),
        }
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[derive(Clone, Serialize)]
struct FileChanged {
    path: String,
}

/// 开始监听 path（其父目录，过滤到该文件）。
pub fn start_watch(app: &AppHandle, path: String) -> Result<(), String> {
    let state = app.state::<FileWatcher>();
    *state.current.lock().unwrap() = Some(path.clone());

    let app2 = app.clone();
    let path2 = path.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        let ev = match res {
            Ok(e) => e,
            Err(_) => return,
        };
        if !ev
            .paths
            .iter()
            .any(|p| p.to_string_lossy() == path2)
        {
            return;
        }
        let relevant = matches!(ev.kind, EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_));
        if !relevant {
            return;
        }
        let st = app2.state::<FileWatcher>();
        if now_ms() < *st.ignore_until.lock().unwrap() {
            return; // 刚保存，忽略自己触发的写
        }
        let _ = app2.emit("file-changed", FileChanged { path: path2.clone() });
    })
    .map_err(|e| e.to_string())?;

    let p = std::path::Path::new(&path);
    let dir = p.parent().map(|d| if d.as_os_str().is_empty() { p } else { d }).unwrap_or(p);
    watcher
        .watch(dir, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    *state.inner.lock().unwrap() = Some(watcher);
    Ok(())
}

/// 停止监听。
pub fn stop_watch(app: &AppHandle) {
    let state = app.state::<FileWatcher>();
    *state.inner.lock().unwrap() = None;
    *state.current.lock().unwrap() = None;
}

/// 标记"刚保存"，1.5s 内忽略 file-changed。
pub fn note_saved(app: &AppHandle) {
    let state = app.state::<FileWatcher>();
    *state.ignore_until.lock().unwrap() = now_ms() + 1500;
}

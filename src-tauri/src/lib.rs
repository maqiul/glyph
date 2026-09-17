// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Glyph —— Tauri 2 入口
//!
//! 模块：
//! - `commands`：所有 IPC commands（前端 invoke 入口）
//! - `markdown`：Markdown 读取 / 编码检测 / 大纲抽取
//! - `screenshot`：屏幕捕获（xcap）
//! - `ocr` / `ocr_cloud`：本地（oar-ocr）/ 云端（百度）文字识别
//! - `error`：统一错误类型
//!
//! 全局能力：托盘常驻 + 全局快捷键（Ctrl+Shift+G 唤起）+ 关窗收进托盘。

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{Builder as GsBuilder, GlobalShortcutExt, ShortcutState};

pub mod commands;
pub mod error;
pub mod markdown;
pub mod ocr;
pub mod ocr_cloud;
pub mod screenshot;

/// 全局唤起快捷键（app 在后台/最小化时把它带回前台）
const GLOBAL_SHORTCUT: &str = "CmdOrCtrl+Shift+G";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(
            GsBuilder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.unminimize();
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::get_app_info,
            commands::read_markdown_file,
            commands::write_markdown_file,
            commands::create_markdown_file,
            commands::extract_outline,
            commands::capture_screens,
            commands::get_cached_capture,
            commands::ocr_recognize_file,
            commands::ocr_recognize_base64,
            commands::ocr_recognize_cloud,
            commands::write_file_base64,
        ])
        .setup(|app| {
            log::info!("Glyph 启动 v{}", env!("CARGO_PKG_VERSION"));

            // 全局快捷键注册
            if let Err(e) = app.global_shortcut().register(GLOBAL_SHORTCUT) {
                log::warn!("注册全局快捷键 {GLOBAL_SHORTCUT} 失败: {e}");
            }

            // 关闭主窗时收进托盘而非退出
            if let Some(w) = app.get_webview_window("main") {
                let win = w.clone();
                w.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });
            }

            // 系统托盘
            let show_i = MenuItem::with_id(app, "show", "显示 Glyph", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Glyph application");
}
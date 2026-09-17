// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Glyph —— Tauri 2 入口
//!
//! 模块：
//! - `commands`：所有 IPC commands（前端 invoke 入口）
//! - `markdown`：Markdown 解析 / 代码高亮 / 大纲抽取
//! - `error`：统一错误类型

pub mod commands;
pub mod error;
pub mod markdown;
pub mod ocr;
pub mod screenshot;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::get_app_info,
            commands::read_markdown_file,
            commands::write_markdown_file,
            commands::create_markdown_file,
            commands::extract_outline,
            commands::capture_screens,
            commands::ocr_recognize_file,
            commands::ocr_recognize_base64,
            commands::write_file_base64,
        ])
        .setup(|_app| {
            log::info!("Glyph 启动 v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Glyph application");
}
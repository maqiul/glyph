//! Tauri IPC commands —— 前端 invoke 的入口
//!
//! 所有公开的 command 都集中在这里，方便审计权限边界。

use std::fs as std_fs;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use tauri::command;

use crate::error::GlyphError;
use crate::markdown::{extract_headings, render_file, RenderOptions2, RenderResult};

/// 写入结果
#[derive(Debug, Serialize)]
pub struct WriteResult {
    pub bytes: usize,
    pub encoding: String,
}

/// 应用信息（启动时前端调一次，校验版本 / 平台）
#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub tauri_version: String,
    pub platform: String,
}

#[command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        tauri_version: tauri::VERSION.to_string(),
        platform: std::env::consts::OS.to_string(),
    }
}

/// 读取一个 markdown 文件 → 返回原始文本 + 元数据 + 大纲。
/// **前端**负责 Markdown → HTML 渲染（markdown-it + highlight.js）。
#[command]
pub async fn read_markdown_file(path: String) -> Result<RenderResult, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || render_file(&path, RenderOptions2::default()))
        .await
        .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 写入文本到 markdown 文件（编辑保存用）。
/// 写入用 UTF-8（无 BOM）。如果原文件是 GB18030 / Latin1 等非 UTF-8 编码，
/// 会以 UTF-8 写回（提示用户）。
#[command]
pub async fn write_markdown_file(path: String, content: String) -> Result<WriteResult, GlyphError> {
    let bytes = content.as_bytes().to_vec();
    let bytes_len = bytes.len();
    tauri::async_runtime::spawn_blocking(move || -> Result<WriteResult, GlyphError> {
        let p = Path::new(&path);
        if let Some(parent) = p.parent() {
            std_fs::create_dir_all(parent).map_err(GlyphError::from)?;
        }
        std_fs::write(p, &bytes).map_err(GlyphError::from)?;
        Ok(WriteResult {
            bytes: bytes_len,
            encoding: "UTF-8".to_string(),
        })
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 健康检查 —— 前端启动时打 call，验证后端可达
#[command]
pub fn ping() -> String {
    "pong".to_string()
}

/// 抽取纯文本大纲（前端粘贴文本时用）
#[command]
pub fn extract_outline(text: String) -> Vec<crate::markdown::Heading> {
    extract_headings(&text)
}

/// 捕获所有显示器 → 返回每屏 PNG(base64)。截图工具用。
#[command]
pub async fn capture_screens() -> Result<Vec<crate::screenshot::CaptureResult>, GlyphError> {
    tauri::async_runtime::spawn_blocking(crate::screenshot::capture_all)
        .await
        .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 读取最近一次捕获的缓存（overlay 用，避免 overlay 自我截屏导致白屏）。
#[command]
pub fn get_cached_capture() -> Result<Vec<crate::screenshot::CaptureResult>, GlyphError> {
    crate::screenshot::cached()
}

/// OCR 识别图片文件（路径）。首次调用会触发模型下载 + 初始化，较慢。
#[command]
pub async fn ocr_recognize_file(path: String) -> Result<String, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::ocr::recognize_file(std::path::Path::new(&path))
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// OCR 识别 base64 图片（截图/剪贴板）。
#[command]
pub async fn ocr_recognize_base64(image_base64: String) -> Result<String, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || crate::ocr::recognize_base64(&image_base64))
        .await
        .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 把 base64 数据写入指定路径（截图保存用）。自动创建父目录。
#[command]
pub async fn write_file_base64(path: String, data_base64: String) -> Result<(), GlyphError> {
    tauri::async_runtime::spawn_blocking(move || -> Result<(), GlyphError> {
        let bytes = STANDARD
            .decode(&data_base64)
            .map_err(|e| GlyphError::Internal(format!("base64: {e}")))?;
        let p = Path::new(&path);
        if let Some(parent) = p.parent() {
            std_fs::create_dir_all(parent).map_err(GlyphError::from)?;
        }
        std_fs::write(p, &bytes).map_err(GlyphError::from)?;
        Ok(())
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 云端 OCR 识别（provider: baidu/ali/tencent；key 由前端本地配置传入）。
#[command]
pub async fn ocr_recognize_cloud(
    path: String,
    provider: String,
    api_key: String,
    secret_key: String,
) -> Result<String, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::ocr_cloud::recognize_cloud(
            std::path::Path::new(&path),
            &provider,
            &api_key,
            &secret_key,
        )
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 新建空 markdown 文件
#[command]
pub async fn create_markdown_file(path: String) -> Result<(), GlyphError> {
    tauri::async_runtime::spawn_blocking(move || -> Result<(), GlyphError> {
        let p = Path::new(&path);
        if let Some(parent) = p.parent() {
            std_fs::create_dir_all(parent).map_err(GlyphError::from)?;
        }
        std_fs::write(p, b"# Untitled\n\n").map_err(GlyphError::from)?;
        Ok(())
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}
//! Tauri IPC commands —— 前端 invoke 的入口
//!
//! 所有公开的 command 都集中在这里，方便审计权限边界。

use std::fs as std_fs;
use std::path::Path;
use std::borrow::Cow;

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

/// 读取本地图片 → `data:<mime>;base64,...`。
/// Markdown 预览里的相对/绝对本地图片无法在 webview 直接加载（页面源是 tauri://localhost），
/// 前端解析出绝对路径后调用本命令，以内联 data URL 显示。与截图/OCR 的 base64 方案一致。
#[command]
pub async fn read_image_data_url(path: String) -> Result<String, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || -> Result<String, GlyphError> {
        const MAX_IMAGE_BYTES: u64 = 25 * 1024 * 1024; // 25MB
        let p = Path::new(&path);
        let meta = std_fs::metadata(p).map_err(GlyphError::from)?;
        if meta.len() > MAX_IMAGE_BYTES {
            return Err(GlyphError::Internal(format!(
                "图片过大（{} 字节，上限 25MB）",
                meta.len()
            )));
        }
        let bytes = std_fs::read(p).map_err(GlyphError::from)?;
        let mime = mime_from_ext(p);
        Ok(format!("data:{mime};base64,{}", STANDARD.encode(&bytes)))
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 按扩展名猜图片 MIME；未知返回 application/octet-stream。
fn mime_from_ext(p: &Path) -> &'static str {
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" | "jpe" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "tif" | "tiff" => "image/tiff",
        _ => "application/octet-stream",
    }
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

/// 把图片（PNG/JPEG base64）复制到系统剪贴板。
#[command]
pub async fn copy_image_to_clipboard(image_base64: String) -> Result<(), GlyphError> {
    tauri::async_runtime::spawn_blocking(move || -> Result<(), GlyphError> {
        let bytes = STANDARD
            .decode(&image_base64)
            .map_err(|e| GlyphError::Internal(format!("base64: {e}")))?;
        let img = image::load_from_memory(&bytes)
            .map_err(|e| GlyphError::Internal(format!("图片解码失败: {e}")))?
            .to_rgba8();
        let (w, h) = (img.width() as usize, img.height() as usize);
        let mut ctx =
            arboard::Clipboard::new().map_err(|e| GlyphError::Internal(format!("剪贴板: {e}")))?;
        ctx.set_image(arboard::ImageData {
            width: w,
            height: h,
            bytes: Cow::Owned(img.into_raw()),
        })
        .map_err(|e| GlyphError::Internal(format!("复制失败: {e}")))?;
        Ok(())
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 检测文件编码（chardetng + BOM 嗅探），返回编码名。
#[command]
pub async fn detect_encoding(path: String) -> Result<String, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || crate::encoding::detect(Path::new(&path)))
        .await
        .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 转换文件编码（from 留空 = 自动检测），写入 out，返回 "源 → 目标"。
#[command]
pub async fn convert_file_encoding(
    path: String,
    from: String,
    to: String,
    out: String,
) -> Result<String, GlyphError> {
    let (src, dst) = tauri::async_runtime::spawn_blocking(move || {
        crate::encoding::convert(Path::new(&path), &from, &to, Path::new(&out))
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))??;
    Ok(format!("{src} → {dst}"))
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

/// 云端 OCR 识别 base64 图片（截图联动用）。
#[command]
pub async fn ocr_recognize_cloud_base64(
    image_base64: String,
    provider: String,
    api_key: String,
    secret_key: String,
) -> Result<String, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::ocr_cloud::recognize_cloud_base64(&image_base64, &provider, &api_key, &secret_key)
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

// ---- PDF 操作 ----

/// 合并多个 PDF
#[command]
pub async fn pdf_merge(paths: Vec<String>, out: String) -> Result<(), GlyphError> {
    tauri::async_runtime::spawn_blocking(move || crate::pdf::merge(&paths, std::path::Path::new(&out)))
        .await
        .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 逐页拆分，返回生成的文件路径
#[command]
pub async fn pdf_split(path: String, out_dir: String) -> Result<Vec<String>, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::pdf::split(std::path::Path::new(&path), std::path::Path::new(&out_dir))
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 旋转指定页
#[command]
pub async fn pdf_rotate(path: String, pages: String, angle: i32, out: String) -> Result<(), GlyphError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::pdf::rotate(std::path::Path::new(&path), &pages, angle, std::path::Path::new(&out))
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 删除指定页
#[command]
pub async fn pdf_delete(path: String, pages: String, out: String) -> Result<(), GlyphError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::pdf::delete(std::path::Path::new(&path), &pages, std::path::Path::new(&out))
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 读取 PDF 页数
#[command]
pub fn pdf_page_count(path: String) -> Result<u32, GlyphError> {
    crate::pdf::page_count(std::path::Path::new(&path))
}

/// 提取 PDF 文字
#[command]
pub async fn pdf_extract_text(path: String) -> Result<String, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::pdf::extract_text(std::path::Path::new(&path))
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 提取指定页为新 PDF
#[command]
pub async fn pdf_extract_pages(path: String, pages: String, out: String) -> Result<(), GlyphError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::pdf::extract(
            std::path::Path::new(&path),
            &pages,
            std::path::Path::new(&out),
        )
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// 按范围拆分（ranges 如 "1-3,4-8"）
#[command]
pub async fn pdf_split_ranges(
    path: String,
    ranges: String,
    out_dir: String,
) -> Result<Vec<String>, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::pdf::split_ranges(
            std::path::Path::new(&path),
            &ranges,
            std::path::Path::new(&out_dir),
        )
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

// ---- 通用 HTTP 请求（Postman 式）----

/// 发送任意 HTTP 请求（Rust 侧 ureq 代理，绕开 webview CORS）。
/// headers 为键值对列表；4xx/5xx 也作为正常响应回传，仅传输层错误返回 Err。
#[command]
pub async fn http_request(
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: String,
    timeout_secs: u64,
) -> Result<crate::http::HttpResult, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::http::request(&method, &url, &headers, &body, timeout_secs)
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}

/// PDF 转图片（逐页渲染为 PNG/JPEG，pages 空 = 全部）
#[command]
pub async fn pdf_to_images(
    path: String,
    pages: String,
    dpi: u32,
    format: String,
    out_dir: String,
) -> Result<Vec<String>, GlyphError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::pdf::to_images(
            std::path::Path::new(&path),
            &pages,
            dpi,
            &format,
            std::path::Path::new(&out_dir),
        )
    })
    .await
    .map_err(|e| GlyphError::Internal(format!("join: {e}")))?
}
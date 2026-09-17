//! 屏幕捕获模块
//!
//! 用 xcap 做跨平台捕获（Windows / macOS / Linux-X11；Linux Wayland 走 portal）。
//! 第一版：捕获所有显示器，编码为 PNG(base64) 返回前端预览。
//! 后续：透明置顶 overlay 框选 + 裁剪。

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use std::sync::Mutex;
use xcap::Monitor;

use crate::error::GlyphError;

/// 单个显示器的捕获结果
#[derive(Debug, Clone, Serialize)]
pub struct CaptureResult {
    pub index: usize,
    pub name: String,
    /// 显示器在虚拟桌面坐标系中的原点（多屏拼接裁剪/overlay 定位用）
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// PNG 图片的 base64（不含 data: 前缀）
    pub png_base64: String,
}

/// 最近一次捕获缓存。overlay 窗口读取它，而不是自己再截一次
/// （否则 overlay 会把"自己盖在屏幕上"的白底截进去，导致白屏）。
static LAST_CAPTURE: Mutex<Option<Vec<CaptureResult>>> = Mutex::new(None);

/// 捕获所有显示器，并缓存结果供 overlay 读取。
pub fn capture_all() -> Result<Vec<CaptureResult>, GlyphError> {
    let monitors = Monitor::all().map_err(|e| GlyphError::Internal(format!("monitor enum: {e}")))?;
    let mut out = Vec::new();

    for (i, m) in monitors.iter().enumerate() {
        let img = m
            .capture_image()
            .map_err(|e| GlyphError::Internal(format!("capture monitor {i}: {e}")))?;
        let w = img.width();
        let h = img.height();

        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| GlyphError::Internal(format!("png encode: {e}")))?;

        out.push(CaptureResult {
            index: i,
            name: m.friendly_name().unwrap_or_else(|_| format!("Monitor {i}")),
            x: m.x().unwrap_or(0),
            y: m.y().unwrap_or(0),
            width: w,
            height: h,
            png_base64: STANDARD.encode(&buf),
        });
    }

    if let Ok(mut g) = LAST_CAPTURE.lock() {
        *g = Some(out.clone());
    }
    Ok(out)
}

/// 读取最近一次捕获的缓存（overlay 用）。
pub fn cached() -> Result<Vec<CaptureResult>, GlyphError> {
    LAST_CAPTURE
        .lock()
        .map_err(|_| GlyphError::Internal("capture cache lock poisoned".into()))?
        .clone()
        .ok_or_else(|| GlyphError::NotFound("无缓存截图，请先捕获".into()))
}
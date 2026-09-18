//! macOS 原生 OCR（Vision 框架）
//!
//! 用系统自带的 `VNRecognizeTextRequest` 做文字识别，替代 ONNX Runtime：
//! - 支持 Intel(x86_64) + Apple 芯片(arm64)，可离线、无需下载模型；
//! - 语言：简体中文(zh-Hans) + 英文(en-US)，accurate 模式；
//! - 实际识别在 `ocr_vision.m`（由 build.rs 用 cc 编译并链接 Vision 框架）中完成。
//!
//! 对外 API 与非 macOS 的 `ocr.rs` 保持一致，供 `commands.rs` 无差别调用。

use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar};
use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine as _};

use crate::error::GlyphError;

extern "C" {
    /// 识别图片字节，成功返回换行拼接的 UTF-8 文本（需 `glyph_ocr_free` 释放）；
    /// 失败返回 NULL，并把错误信息写入 `out_error`（同样需释放）。
    fn glyph_ocr_recognize(
        data: *const c_uchar,
        len: usize,
        out_error: *mut *mut c_char,
    ) -> *mut c_char;

    fn glyph_ocr_free(ptr: *mut c_char);
}

fn recognize_bytes(bytes: &[u8]) -> Result<String, GlyphError> {
    let mut err: *mut c_char = std::ptr::null_mut();
    let out = unsafe { glyph_ocr_recognize(bytes.as_ptr(), bytes.len(), &mut err) };
    if out.is_null() {
        let msg = if !err.is_null() {
            let s = unsafe { CStr::from_ptr(err) }.to_string_lossy().into_owned();
            unsafe { glyph_ocr_free(err) };
            s
        } else {
            "Vision OCR 失败".to_string()
        };
        return Err(GlyphError::Internal(format!("OCR 识别失败: {msg}")));
    }
    let text = unsafe { CStr::from_ptr(out) }.to_string_lossy().into_owned();
    unsafe { glyph_ocr_free(out) };
    Ok(text)
}

/// 识别一个图片文件（png/jpg/bmp/webp），返回拼接的文本。
pub fn recognize_file(path: &Path) -> Result<String, GlyphError> {
    let bytes = std::fs::read(path).map_err(GlyphError::from)?;
    recognize_bytes(&bytes)
}

/// 识别 base64 图片（截图/剪贴板），无需落临时文件。
pub fn recognize_base64(image_base64: &str) -> Result<String, GlyphError> {
    let bytes = STANDARD
        .decode(image_base64)
        .map_err(|e| GlyphError::Internal(format!("base64 decode: {e}")))?;
    recognize_bytes(&bytes)
}

//! 文件编码检测与转换（chardetng + encoding_rs，纯 Rust）
//!
//! - `detect`：嗅探 BOM / chardetng 统计检测，返回编码名
//! - `convert`：按来源编码（可自动检测）解码，再以目标编码重新写盘

use std::fs;
use std::path::Path;

use crate::error::GlyphError;

fn read_bytes(path: &Path) -> Result<Vec<u8>, GlyphError> {
    fs::read(path).map_err(GlyphError::from)
}

/// 检测字节流编码（优先 BOM，其次 chardetng 统计）
fn detect_bytes(bytes: &[u8]) -> String {
    if bytes.starts_with(b"\xef\xbb\xbf") {
        return "UTF-8 (BOM)".to_string();
    }
    if bytes.starts_with(b"\xff\xfe") {
        return "UTF-16LE".to_string();
    }
    if bytes.starts_with(b"\xfe\xff") {
        return "UTF-16BE".to_string();
    }
    let mut det = chardetng::EncodingDetector::new();
    let _ = det.feed(bytes, true);
    det.guess(None, true).name().to_string()
}

/// 检测文件编码，返回编码名（如 UTF-8 / GB18030 / Big5 / ISO-8859-1）
pub fn detect(path: &Path) -> Result<String, GlyphError> {
    Ok(detect_bytes(&read_bytes(path)?))
}

/// 转换文件编码。
/// - `from`：源编码标签，留空 = 自动检测
/// - `to`：目标编码标签（UTF-8 / GBK / GB18030 / Big5 / Shift_JIS / EUC-KR / ISO-8859-1 / UTF-16LE ...）
/// - 写入 `out`，返回 `(源编码, 目标编码)`
pub fn convert(
    path: &Path,
    from: &str,
    to: &str,
    out: &Path,
) -> Result<(String, String), GlyphError> {
    let bytes = read_bytes(path)?;
    let from_trim = from.trim();

    let (src_name, src_enc): (String, &'static encoding_rs::Encoding) = if from_trim.is_empty() {
        let mut det = chardetng::EncodingDetector::new();
        let _ = det.feed(&bytes, true);
        let enc = det.guess(None, true);
        (enc.name().to_string(), enc)
    } else {
        let enc = encoding_rs::Encoding::for_label(from_trim.as_bytes())
            .ok_or_else(|| GlyphError::Internal(format!("未知源编码: {from_trim}")))?;
        (from_trim.to_string(), enc)
    };

    let (text, _) = src_enc.decode_with_bom_removal(&bytes);

    let dst_enc = encoding_rs::Encoding::for_label(to.trim().as_bytes())
        .ok_or_else(|| GlyphError::Internal(format!("未知目标编码: {to}")))?;
    let (encoded, dst_used, _) = dst_enc.encode(&text);

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(GlyphError::from)?;
    }
    fs::write(out, &encoded).map_err(GlyphError::from)?;
    Ok((src_name, dst_used.name().to_string()))
}

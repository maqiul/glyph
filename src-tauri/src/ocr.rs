//! 本地 OCR 模块（PP-OCR via oar-ocr + ONNX Runtime）
//!
//! 模型：中文 PP-OCRv5 mobile（det + rec + 字典），auto-download 首次运行时从
//! ModelScope 拉取并缓存。识别 pipeline（检测/裁剪/CTC 解码）由 oar-ocr 封装。
//! OAROCR 实例用 OnceLock 懒加载单例（首次 build 会触发模型下载，较慢）。

use std::path::Path;
use std::sync::OnceLock;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use oar_ocr::oarocr::OAROCRBuilder;
use oar_ocr::utils::load_image;

use crate::error::GlyphError;

const DET_MODEL: &str = "pp-ocrv5_mobile_det.onnx";
const REC_MODEL: &str = "pp-ocrv5_mobile_rec.onnx";
const DICT: &str = "ppocrv5_dict.txt";

type OcrInstance = oar_ocr::oarocr::OAROCR;

static OCR: OnceLock<Result<OcrInstance, String>> = OnceLock::new();

fn ocr_instance() -> Result<&'static OcrInstance, GlyphError> {
    OCR.get_or_init(|| {
        OAROCRBuilder::new(DET_MODEL, REC_MODEL, DICT)
            .build()
            .map_err(|e| format!("{e}"))
    })
    .as_ref()
    .map_err(|e| GlyphError::Internal(format!("OCR 初始化失败: {e}")))
}

/// 识别一个图片文件（png/jpg/bmp/webp），返回拼接的文本。
pub fn recognize_file(path: &Path) -> Result<String, GlyphError> {
    let ocr = ocr_instance()?;
    let image = load_image(path).map_err(|e| GlyphError::Internal(format!("load image: {e}")))?;
    let results = ocr
        .predict(vec![image])
        .map_err(|e| GlyphError::Internal(format!("predict: {e}")))?;

    let mut lines: Vec<String> = Vec::new();
    for result in results {
        for region in result.text_regions {
            if let Some(text) = region.text {
                let t = text.trim();
                if !t.is_empty() {
                    lines.push(t.to_string());
                }
            }
        }
    }
    Ok(lines.join("\n"))
}

/// 识别 base64 图片（截图/剪贴板），先落临时文件再走 recognize_file。
pub fn recognize_base64(png_base64: &str) -> Result<String, GlyphError> {
    let bytes = STANDARD
        .decode(png_base64)
        .map_err(|e| GlyphError::Internal(format!("base64 decode: {e}")))?;
    let tmp = std::env::temp_dir().join(format!("glyph_ocr_{}.png", nanos()));
    std::fs::write(&tmp, &bytes).map_err(GlyphError::from)?;
    let out = recognize_file(&tmp);
    let _ = std::fs::remove_file(&tmp);
    out
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}
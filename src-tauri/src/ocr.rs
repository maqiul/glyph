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

#[cfg(test)]
mod tests {
    use super::*;

    /// 端到端验证：建 OCR 引擎（首次触发模型 auto-download）+ 推理一张图。
    /// 不校验文字正确性（无真图），只证明"下载→加载→检测→识别"链路通、不 panic。
    #[test]
    fn ocr_engine_builds_and_predicts() {
        let ocr = OAROCRBuilder::new(DET_MODEL, REC_MODEL, DICT)
            .build()
            .expect("build OCR (downloads models on first run)");

        let tmp = std::env::temp_dir().join("glyph_ocr_test.png");
        let img = image::DynamicImage::new_rgb8(220, 80);
        img.save(&tmp).expect("save test img");

        let loaded = load_image(&tmp).expect("load test img");
        let results = ocr.predict(vec![loaded]).expect("predict");
        let _ = results; // 纯色图无文字，结果为空属正常
        let _ = std::fs::remove_file(&tmp);
    }

    /// 真实图片识别质量验证：图路径由环境变量 GLYPH_OCR_TEST_IMG 传入。
    #[test]
    #[ignore = "需真实图片，手动跑"]
    fn ocr_real_image_debug() {
        let path = std::env::var("GLYPH_OCR_TEST_IMG").unwrap_or_default();
        if path.is_empty() {
            eprintln!("GLYPH_OCR_TEST_IMG not set, skip");
            return;
        }
        let ocr = OAROCRBuilder::new(DET_MODEL, REC_MODEL, DICT)
            .build()
            .expect("build OCR");
        let img = load_image(Path::new(&path)).expect("load image");
        let results = ocr.predict(vec![img]).expect("predict");
        let mut count = 0;
        for r in results {
            for reg in r.text_regions {
                if let Some(t) = reg.text {
                    count += 1;
                    println!("OCR> {}", t);
                }
            }
        }
        println!("OCR total lines: {}", count);
    }
}
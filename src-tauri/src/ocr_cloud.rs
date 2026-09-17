//! 云端 OCR 模块（可插拔服务商）
//!
//! 当前已接：百度智能云「通用文字识别（高精度版 accurate_basic）」，token 认证。
//! 阿里 / 腾讯：架构预留，适配器待补（需各自签名，且需你的 key 实测）。
//! API Key / Secret 由前端从本地配置传入，仅在本机使用。

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::Value;
use std::path::Path;

use crate::error::GlyphError;

pub fn recognize_cloud(
    path: &Path,
    provider: &str,
    api_key: &str,
    secret_key: &str,
) -> Result<String, GlyphError> {
    if api_key.trim().is_empty() || secret_key.trim().is_empty() {
        return Err(GlyphError::Internal("云端 OCR 未配置 API Key / Secret Key".into()));
    }
    match provider {
        "baidu" => baidu_ocr(path, api_key, secret_key),
        "ali" | "tencent" => Err(GlyphError::Internal(
            "该服务商适配器即将支持（当前已接百度）".into(),
        )),
        other => Err(GlyphError::Internal(format!("未知云端服务商: {other}"))),
    }
}

fn read_image_base64(path: &Path) -> Result<String, GlyphError> {
    let bytes = std::fs::read(path).map_err(GlyphError::from)?;
    Ok(STANDARD.encode(&bytes))
}

fn baidu_token(api_key: &str, secret_key: &str) -> Result<String, GlyphError> {
    let url = format!(
        "https://aip.baidubce.com/oauth/2.0/token?grant_type=client_credentials&client_id={}&client_secret={}",
        api_key, secret_key
    );
    let resp: Value = ureq::get(&url)
        .call()
        .map_err(|e| GlyphError::Internal(format!("百度 token 请求失败: {e}")))?
        .into_json()
        .map_err(|e| GlyphError::Internal(format!("百度 token 解析失败: {e}")))?;
    resp.get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| GlyphError::Internal(format!("百度取 token 失败: {resp}")))
}

fn baidu_ocr(path: &Path, api_key: &str, secret_key: &str) -> Result<String, GlyphError> {
    let token = baidu_token(api_key, secret_key)?;
    let b64 = read_image_base64(path)?;
    let url = format!(
        "https://aip.baidubce.com/rest/2.0/ocr/v1/accurate_basic?access_token={}",
        token
    );
    let resp: Value = ureq::post(&url)
        .send_form(&[("image", b64.as_str())])
        .map_err(|e| GlyphError::Internal(format!("百度 OCR 请求失败: {e}")))?
        .into_json()
        .map_err(|e| GlyphError::Internal(format!("百度 OCR 解析失败: {e}")))?;

    if resp.get("error_code").is_some() {
        let code = resp.get("error_code").map(|v| v.to_string()).unwrap_or_default();
        let msg = resp
            .get("error_msg")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        return Err(GlyphError::Internal(format!("百度 OCR 错误 {code}: {msg}")));
    }

    let mut lines = Vec::new();
    if let Some(arr) = resp.get("words_result").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(w) = item.get("words").and_then(|v| v.as_str()) {
                let t = w.trim();
                if !t.is_empty() {
                    lines.push(t.to_string());
                }
            }
        }
    }
    Ok(lines.join("\n"))
}

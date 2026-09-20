//! 通用 HTTP 客户端（Postman 式）：由 Rust 侧用 ureq 发请求，绕开 webview 的 CORS 限制。
//! 4xx/5xx 也作为正常响应回传（便于查看错误响应体），仅传输层错误（DNS/TLS/超时）返回 Err。

use std::time::{Duration, Instant};

use serde::Serialize;

use crate::error::GlyphError;

/// 响应体最多回传给前端的字节数（超出截断，避免超大 IPC 负载拖垮前端）。
const MAX_BODY: usize = 1_000_000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResult {
    pub status: u16,
    pub status_text: String,
    pub ok: bool,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub truncated: bool,
    pub duration_ms: u64,
    pub size_bytes: usize,
}

fn build_agent(timeout_secs: u64) -> ureq::Agent {
    let t = Duration::from_secs(timeout_secs.max(1).min(600));
    ureq::AgentBuilder::new()
        .timeout_connect(t)
        .timeout_read(t)
        .timeout_write(t)
        .redirects(20)
        .build()
}

/// 收集响应头：ureq 2.x 的 Response 用 headers_names() + all(name) 枚举。
fn collect_headers(resp: &ureq::Response) -> Vec<(String, String)> {
    resp.headers_names()
        .into_iter()
        .map(|name| {
            let vals = resp.all(&name);
            (name, vals.join(", "))
        })
        .collect()
}

/// 把 Response（无论 2xx 还是 4xx/5xx）统一转成前端结果。
fn finish(resp: ureq::Response, start: Instant) -> Result<HttpResult, GlyphError> {
    let duration_ms = start.elapsed().as_millis() as u64;
    let status = resp.status();
    let status_text = resp.status_text().to_string();
    let headers = collect_headers(&resp);
    let text = resp
        .into_string()
        .map_err(|e| GlyphError::Internal(format!("读取响应失败: {e}")))?;
    let size_bytes = text.len();
    let truncated = size_bytes > MAX_BODY;
    let body = if truncated {
        // 按字节截断，回退到最近的合法 UTF-8 字符边界，避免 panic。
        let mut end = MAX_BODY;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        text[..end].to_string()
    } else {
        text
    };
    Ok(HttpResult {
        status,
        status_text,
        ok: (200..400).contains(&status),
        headers,
        body,
        truncated,
        duration_ms,
        size_bytes,
    })
}

pub fn request(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: &str,
    timeout_secs: u64,
) -> Result<HttpResult, GlyphError> {
    let url = url.trim();
    if url.is_empty() {
        return Err(GlyphError::Internal("URL 不能为空".into()));
    }
    let m = method.trim().to_uppercase();
    if m.is_empty() {
        return Err(GlyphError::Internal("方法不能为空".into()));
    }
    let agent = build_agent(timeout_secs);
    // 用通用 request 覆盖全部方法（含 OPTIONS 等无专用 builder 的）。
    let mut req = agent.request(&m, url);
    for (k, v) in headers {
        let k = k.trim();
        if k.is_empty() {
            continue;
        }
        req = req.set(k, v);
    }

    let start = Instant::now();
    // 有 body 用 send_bytes（不强制 Content-Type，尊重用户自定义头），否则裸调用。
    let result = if body.is_empty() {
        req.call()
    } else {
        req.send_bytes(body.as_bytes())
    };
    // ureq 2.x：>=400 归入 Error::Status，仍可拿到 Response；传输层错误才是真失败。
    let resp = match result {
        Ok(r) => r,
        Err(ureq::Error::Status(_code, resp)) => resp,
        Err(ureq::Error::Transport(t)) => {
            return Err(GlyphError::Internal(format!("请求失败: {t}")))
        }
    };
    finish(resp, start)
}

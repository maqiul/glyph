//! Markdown 文件读取模块
//!
//! 负责把磁盘上的 .md 文件读取、检测编码、抽取大纲。
//! **设计决策**：渲染全部放前端做（markdown-it + highlight.js），
//! 后端只做 IO/编码检测/大纲抽取。这样代码高亮、主题切换、动效
//! 都在前端更灵活，也避免打包 syntect assets 进入二进制。

use std::fs as std_fs;
use std::path::Path;

use crate::error::GlyphError;

/// 渲染选项（保留向后兼容；当前仅 gfm 影响大纲抽取）
#[derive(Debug, Clone, Copy)]
pub struct RenderOptions2 {
    pub gfm: bool,
}

impl Default for RenderOptions2 {
    fn default() -> Self {
        Self { gfm: true }
    }
}

/// 单个文件读取结果（文本 + 元数据 + 大纲，前端自渲染）
#[derive(Debug, Clone, serde::Serialize)]
pub struct RenderResult {
    /// 原始 markdown 文本（UTF-8）
    pub content: String,
    /// 标题（第一行 H1 或文件名）
    pub title: String,
    /// 字节数
    pub bytes: usize,
    /// 检测到的文本编码
    pub encoding: String,
    /// H1–H3 大纲
    pub headings: Vec<Heading>,
}

/// 大纲项（id 由前端从渲染产物里抽取，确保跟实际 HTML 一致）
#[derive(Debug, Clone, serde::Serialize)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    /// 暂留空字符串 —— 前端拿到 HTML 后扫真实 id 填回
    pub id: String,
}

/// 读取一个 markdown 文件，返回文本 + 元信息 + 大纲。
pub fn render_file<P: AsRef<Path>>(
    path: P,
    _options: RenderOptions2,
) -> Result<RenderResult, GlyphError> {
    let path = path.as_ref();
    let bytes = std_fs::read(path).map_err(|e| GlyphError::Io(format!("{}: {}", path.display(), e)))?;

    let (text, encoding) = decode(&bytes);

    let title = extract_title(&text).unwrap_or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string()
    });

    let headings = extract_headings(&text);

    Ok(RenderResult {
        content: text,
        title,
        bytes: bytes.len(),
        encoding,
        headings,
    })
}

fn extract_title(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim_start_matches('#').trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

pub fn extract_headings(text: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let trimmed = line.trim_start();
        let level = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=3).contains(&level) && trimmed.starts_with('#') {
            let rest = trimmed[level..].trim_end();
            if !rest.is_empty() {
                let id = slugify(rest);
                out.push(Heading {
                    level: level as u8,
                    text: rest.to_string(),
                    id,
                });
            }
        }
    }
    out
}

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => c.to_ascii_lowercase(),
            ' ' | '\t' | '\n' | '\r' => '-',
            _ => '-',
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn decode(bytes: &[u8]) -> (String, String) {
    if bytes.starts_with(b"\xef\xbb\xbf") {
        return (
            String::from_utf8_lossy(&bytes[3..]).into_owned(),
            "UTF-8 (BOM)".to_string(),
        );
    }
    if bytes.starts_with(b"\xff\xfe") {
        let (cow, _, _) = encoding_rs::UTF_16LE.decode(bytes);
        return (cow.into(), "UTF-16LE".to_string());
    }
    if bytes.starts_with(b"\xfe\xff") {
        let (cow, _, _) = encoding_rs::UTF_16BE.decode(bytes);
        return (cow.into(), "UTF-16BE".to_string());
    }

    let guess = chardetng::EncodingDetector::new();
    let enc = guess.guess(None, true);
    let (cow, _, had_errors) = enc.decode(bytes);
    if had_errors {
        return (String::from_utf8_lossy(bytes).into_owned(), "UTF-8 (lossy)".to_string());
    }
    (cow.into(), enc.name().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title() {
        assert_eq!(extract_title("# Hello\n"), Some("Hello".to_string()));
        assert_eq!(extract_title("## Sub\n"), Some("Sub".to_string()));
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn test_headings() {
        let md = "# H1\n## H2\n### H3\n#### H4\n";
        let h = extract_headings(md);
        assert_eq!(h.len(), 3);
        assert_eq!(h[0].level, 1);
        assert_eq!(h[2].level, 3);
    }

    #[test]
    fn test_headings_skip_fenced_code() {
        let md = "# Real\n```bash\n# not a heading\n```\n## Inside";
        let h = extract_headings(md);
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn test_decode_utf8_bom() {
        let bytes = b"\xef\xbb\xbf# Hello";
        let (s, enc) = decode(bytes);
        assert_eq!(s, "# Hello");
        assert!(enc.contains("BOM"));
    }
}
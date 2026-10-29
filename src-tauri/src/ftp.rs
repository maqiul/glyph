//! FTP 客户端模块（suppaftp，纯 Rust）
//!
//! 全局单会话（Mutex<Option<FtpStream>>）。支持连接/列目录/下载/上传。
//! 删除/重命名/建目录/站点书签作为后续增强。

use std::sync::Mutex;

use serde::Serialize;
use suppaftp::types::FileType;
use suppaftp::{FtpStream, Mode};
use tauri::State;

use crate::error::GlyphError;

pub struct FtpSession {
    pub stream: Mutex<Option<FtpStream>>,
}

impl FtpSession {
    pub fn new() -> Self {
        Self {
            stream: Mutex::new(None),
        }
    }
}

#[derive(Serialize)]
pub struct FtpEntry {
    pub name: String,
    pub size: u64,
    pub modified: String,
    pub is_dir: bool,
}

fn e<E: std::fmt::Debug>(x: E) -> GlyphError {
    GlyphError::Internal(format!("{x:?}"))
}

pub fn connect(
    sess: &State<FtpSession>,
    host: String,
    port: u16,
    user: String,
    pass: String,
    passive: bool,
) -> Result<(), GlyphError> {
    let mut stream = FtpStream::connect((host.as_str(), port)).map_err(e)?;
    stream.login(&user, &pass).map_err(e)?;
    // 被动(PASV)对防火墙/NAT 友好，是绝大多数场景的正确选择；主动(PORT)少数老服务器需要
    stream.set_mode(if passive { Mode::Passive } else { Mode::Active });
    let _ = stream.transfer_type(FileType::Binary);
    *sess.stream.lock().unwrap() = Some(stream);
    Ok(())
}

pub fn delete(sess: &State<FtpSession>, path: String) -> Result<(), GlyphError> {
    let mut guard = sess.stream.lock().unwrap();
    let stream = guard
        .as_mut()
        .ok_or_else(|| GlyphError::Internal("未连接 FTP".into()))?;
    stream.rm(&path).map_err(e)?;
    Ok(())
}

pub fn rename(sess: &State<FtpSession>, from: String, to: String) -> Result<(), GlyphError> {
    let mut guard = sess.stream.lock().unwrap();
    let stream = guard
        .as_mut()
        .ok_or_else(|| GlyphError::Internal("未连接 FTP".into()))?;
    stream.rename(&from, &to).map_err(e)?;
    Ok(())
}

pub fn mkdir(sess: &State<FtpSession>, path: String) -> Result<(), GlyphError> {
    let mut guard = sess.stream.lock().unwrap();
    let stream = guard
        .as_mut()
        .ok_or_else(|| GlyphError::Internal("未连接 FTP".into()))?;
    stream.mkdir(&path).map_err(e)?;
    Ok(())
}

pub fn disconnect(sess: &State<FtpSession>) {
    if let Some(mut s) = sess.stream.lock().unwrap().take() {
        let _ = s.quit();
    }
}

pub fn is_connected(sess: &State<FtpSession>) -> bool {
    sess.stream.lock().unwrap().is_some()
}

pub fn list(sess: &State<FtpSession>, path: String) -> Result<Vec<FtpEntry>, GlyphError> {
    let mut guard = sess.stream.lock().unwrap();
    let stream = guard
        .as_mut()
        .ok_or_else(|| GlyphError::Internal("未连接 FTP".into()))?;
    stream.cwd(&path).map_err(e)?;
    let lines = stream.list(None).map_err(e)?;
    Ok(lines.iter().filter_map(|l| parse_list_line(l)).collect())
}

pub fn download(
    sess: &State<FtpSession>,
    remote: String,
    local: String,
) -> Result<(), GlyphError> {
    let mut guard = sess.stream.lock().unwrap();
    let stream = guard
        .as_mut()
        .ok_or_else(|| GlyphError::Internal("未连接 FTP".into()))?;
    let mut buf = stream.retr_as_buffer(&remote).map_err(e)?;
    let bytes = buf.into_inner();
    std::fs::write(&local, bytes).map_err(GlyphError::from)?;
    Ok(())
}

pub fn upload(sess: &State<FtpSession>, local: String, remote: String) -> Result<(), GlyphError> {
    let mut guard = sess.stream.lock().unwrap();
    let stream = guard
        .as_mut()
        .ok_or_else(|| GlyphError::Internal("未连接 FTP".into()))?;
    let mut f = std::fs::File::open(&local).map_err(GlyphError::from)?;
    stream.put_file(&remote, &mut f).map_err(e)?;
    Ok(())
}

/// 解析 Unix `ls -l` 行。字段不足或 . / .. 返回 None。
fn parse_list_line(line: &str) -> Option<FtpEntry> {
    let cols: Vec<&str> = line.split_whitespace().collect();
    if cols.len() < 9 {
        return None;
    }
    let is_dir = cols[0].starts_with('d');
    let size: u64 = cols[4].parse().unwrap_or(0);
    let modified = format!("{} {} {}", cols[5], cols[6], cols[7]);
    // 文件名可能含空格：取第 9 段起的剩余
    let name = line.splitn(9, char::is_whitespace).nth(8)?.trim().to_string();
    if name == "." || name == ".." {
        return None;
    }
    Some(FtpEntry {
        name,
        size,
        modified,
        is_dir,
    })
}

//! SFTP 客户端模块（ssh2 / libssh2）
//!
//! 与 FTP 分离的全局会话。支持连接/列目录/上传/下载/删除/重命名/建目录。

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use ssh2::{OpenFlags, OpenType, Session};
use tauri::State;

use crate::error::GlyphError;

pub struct SftpSession {
    pub session: Mutex<Option<Session>>,
}

impl SftpSession {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
        }
    }
}

#[derive(Serialize)]
pub struct SftpEntry {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub mtime: u64,
}

fn e<E: std::fmt::Debug>(x: E) -> GlyphError {
    GlyphError::Internal(format!("{x:?}"))
}

pub fn connect(
    sess: &State<SftpSession>,
    host: String,
    port: u16,
    user: String,
    pass: String,
) -> Result<(), GlyphError> {
    let tcp = TcpStream::connect((host.as_str(), port)).map_err(e)?;
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = tcp.set_write_timeout(Some(Duration::from_secs(30)));
    let mut session = Session::new().map_err(e)?;
    session.set_tcp_stream(tcp);
    session.handshake().map_err(e)?;
    session.userauth_password(&user, &pass).map_err(e)?;
    if !session.authenticated() {
        return Err(GlyphError::Internal("SFTP 认证失败".into()));
    }
    *sess.session.lock().unwrap() = Some(session);
    Ok(())
}

pub fn disconnect(sess: &State<SftpSession>) {
    // 直接 drop session 关闭连接；ssh2 disconnect 签名易错，省略
    let _ = sess.session.lock().unwrap().take();
}

pub fn is_connected(sess: &State<SftpSession>) -> bool {
    sess.session.lock().unwrap().is_some()
}

pub fn list(sess: &State<SftpSession>, path: String) -> Result<Vec<SftpEntry>, GlyphError> {
    let guard = sess.session.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| GlyphError::Internal("未连接 SFTP".into()))?;
    let sftp = session.sftp().map_err(e)?;
    let entries = sftp.readdir(&path).map_err(e)?;
    let mut out = Vec::new();
    for (name, stat) in entries {
        let n = name.to_string_lossy().to_string();
        if n == "." || n == ".." {
            continue;
        }
        out.push(SftpEntry {
            name: n,
            size: stat.size.unwrap_or(0),
            is_dir: stat.is_dir(),
            mtime: stat.mtime.unwrap_or(0),
        });
    }
    Ok(out)
}

pub fn download(
    sess: &State<SftpSession>,
    remote: String,
    local: String,
) -> Result<(), GlyphError> {
    let guard = sess.session.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| GlyphError::Internal("未连接 SFTP".into()))?;
    let sftp = session.sftp().map_err(e)?;
    let mut rf = sftp.open(&remote).map_err(e)?;
    let mut data = Vec::new();
    rf.read_to_end(&mut data).map_err(e)?;
    std::fs::write(&local, data).map_err(GlyphError::from)?;
    Ok(())
}

pub fn upload(sess: &State<SftpSession>, local: String, remote: String) -> Result<(), GlyphError> {
    let data = std::fs::read(&local).map_err(GlyphError::from)?;
    let guard = sess.session.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| GlyphError::Internal("未连接 SFTP".into()))?;
    let sftp = session.sftp().map_err(e)?;
    let mut wf = sftp
        .open_mode(
            &remote,
            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
            0o644,
            OpenType::File,
        )
        .map_err(e)?;
    wf.write_all(&data).map_err(e)?;
    Ok(())
}

pub fn delete(sess: &State<SftpSession>, path: String) -> Result<(), GlyphError> {
    let guard = sess.session.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| GlyphError::Internal("未连接 SFTP".into()))?;
    let sftp = session.sftp().map_err(e)?;
    let p = Path::new(&path);
    if sftp.unlink(p).is_err() {
        sftp.rmdir(p).map_err(e)?;
    }
    Ok(())
}

pub fn rename(sess: &State<SftpSession>, from: String, to: String) -> Result<(), GlyphError> {
    let guard = sess.session.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| GlyphError::Internal("未连接 SFTP".into()))?;
    let sftp = session.sftp().map_err(e)?;
    sftp.rename(Path::new(&from), Path::new(&to), None).map_err(e)?;
    Ok(())
}

pub fn mkdir(sess: &State<SftpSession>, path: String) -> Result<(), GlyphError> {
    let guard = sess.session.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| GlyphError::Internal("未连接 SFTP".into()))?;
    let sftp = session.sftp().map_err(e)?;
    sftp.mkdir(Path::new(&path), 0o755).map_err(e)?;
    Ok(())
}

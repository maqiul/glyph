//! SFTP 客户端模块（ssh2 / libssh2）
//!
//! 与 FTP 分离的全局会话。支持连接/列目录/上传/下载/删除/重命名/建目录。

use std::net::TcpStream;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use ssh2::Session;
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
    pub mtime: i64,
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
    if let Some(s) = sess.session.lock().unwrap().take() {
        let _ = s.disconnect(None);
    }
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
    let dir = sftp.opendir(&path).map_err(e)?;
    let mut out = Vec::new();
    for (name, stat) in dir.by_ref() {
        let n = String::from_utf8_lossy(&name).to_string();
        if n == "." || n == ".." {
            continue;
        }
        out.push(SftpEntry {
            name: n,
            size: stat.stat.size.unwrap_or(0),
            is_dir: stat.is_dir(),
            mtime: stat.stat.mtime.unwrap_or(0),
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
    std::io::Read::read_to_end(&mut rf, &mut data).map_err(e)?;
    std::fs::write(&local, data).map_err(GlyphError::from)?;
    Ok(())
}

pub fn upload(sess: &State<SftpSession>, local: String, remote: String) -> Result<(), GlyphError> {
    let guard = sess.session.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| GlyphError::Internal("未连接 SFTP".into()))?;
    let sftp = session.sftp().map_err(e)?;
    let data = std::fs::read(&local).map_err(GlyphError::from)?;
    let mut wf = sftp.open(&remote).map_err(e)?;
    wf.create()
        .write_all(&data)
        .map_err(|x| GlyphError::Internal(format!("{x:?}")))?;
    Ok(())
}

pub fn delete(sess: &State<SftpSession>, path: String) -> Result<(), GlyphError> {
    let guard = sess.session.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| GlyphError::Internal("未连接 SFTP".into()))?;
    let sftp = session.sftp().map_err(e)?;
    // 先试文件删除，失败再试目录删除
    if sftp.remove_file(&path).is_err() {
        sftp.rmdir(&path).map_err(e)?;
    }
    Ok(())
}

pub fn rename(sess: &State<SftpSession>, from: String, to: String) -> Result<(), GlyphError> {
    let guard = sess.session.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| GlyphError::Internal("未连接 SFTP".into()))?;
    let sftp = session.sftp().map_err(e)?;
    sftp.rename(&from, &to, None).map_err(e)?;
    Ok(())
}

pub fn mkdir(sess: &State<SftpSession>, path: String) -> Result<(), GlyphError> {
    let guard = sess.session.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| GlyphError::Internal("未连接 SFTP".into()))?;
    let sftp = session.sftp().map_err(e)?;
    sftp.mkdir(&path, 0o755).map_err(e)?;
    Ok(())
}

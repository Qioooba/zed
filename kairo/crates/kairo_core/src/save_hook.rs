//! Post-save byte-fidelity hook.
//!
//! Zed buffers are UTF-8. For legacy GBK files we let Zed write its UTF-8
//! bytes first, then immediately recode the file on disk through the Kairo
//! agent (strict decode + atomic write). The buffer stays clean and the bytes
//! on disk remain correct for the legacy JSP/Tomcat runtime.

use std::{
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
    sync::OnceLock,
};

use anyhow::{Context as _, Result, bail};
use gpui::{App, Task};

static CONFIG: OnceLock<(String, String)> = OnceLock::new();

/// Called by kairo_core::init once the agent is available.
pub fn configure(agent_base_url: String, target_encoding: String) {
    let _ = CONFIG.set((agent_base_url, target_encoding));
}

/// Wrap a buffer save so non-UTF-8 legacy files are recoded after Zed writes.
pub fn after_save(save: Task<Result<()>>, path: Option<PathBuf>, cx: &mut App) -> Task<Result<()>> {
    cx.spawn(async move |_cx| {
        save.await?;
        let Some(path) = path else { return Ok(()) };
        let Some((base_url, encoding)) = CONFIG.get() else {
            return Ok(());
        };
        if encoding.eq_ignore_ascii_case("utf-8") || encoding.eq_ignore_ascii_case("utf8") {
            return Ok(());
        }
        recode_file_sync(base_url, encoding, &path)
    })
}

fn recode_file_sync(base_url: &str, encoding: &str, path: &std::path::Path) -> Result<()> {
    let temp = path.with_extension(format!(
        "{}.kairo-tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("txt")
    ));
    let original = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    std::fs::write(&temp, &original)?;

    let authority = base_url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let authority = authority
        .split('/')
        .next()
        .context("invalid agent base url")?;
    let mut stream = TcpStream::connect(authority)
        .with_context(|| format!("connecting to Kairo agent at {authority}"))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;

    let payload = serde_json::json!({
        "requestId": "kairo_post_save_recode",
        "payload": {
            "file": temp.to_string_lossy().replace('\\', "/"),
            "from": "utf-8",
            "to": encoding,
        }
    });
    let body = serde_json::to_vec(&payload)?;
    let request = format!(
        "POST /api/v1/encoding/recode HTTP/1.1\r\nHost: {authority}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(request.as_bytes())?;
    stream.write_all(&body)?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let response = String::from_utf8_lossy(&response);
    if !response.contains("\"ok\":true") {
        bail!(
            "agent rejected post-save recode: {}",
            response.chars().take(300).collect::<String>()
        );
    }
    let recoded = std::fs::read(&temp)?;
    std::fs::write(path, recoded)?;
    let _ = std::fs::remove_file(&temp);
    Ok(())
}

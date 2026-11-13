//! 个人云多后端端到端加密（E2EE）真实推拉与 12 词跨端恢复引擎。
//! 支持：
//! 1. `icloud_drive`：本机 / Apple iCloud Drive 容器目录隔离写入 `ops/{device_id}/{batch_seq:06}.enc`
//! 2. `webdav`：坚果云 / 群晖 NAS / Alist 真实 `MKCOL` + `PUT` + `GET`
//! 3. `byo_server`：私有节点或 `qi-cloud` `/v1/sync/ops/push` & `/v1/sync/ops/pull`
//! 4. `local_only`：单文件 `.qivault` 加密归档包导入导出

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::crypto_sync::{
    EncryptedBatchEnvelope, decrypt_sync_batch, derive_master_key, encrypt_sync_batch,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncExecutionReport {
    pub sync_mode: String,
    pub target_location: String,
    pub pushed_batches: usize,
    pub pulled_batches: usize,
    pub merged_records: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalEncryptedArchive {
    pub format: String,
    pub exported_at: String,
    pub envelope: EncryptedBatchEnvelope,
}

pub fn resolve_icloud_vault_dir() -> PathBuf {
    if let Ok(explicit) = std::env::var("QI_ICLOUD_SYNC_DIR") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if std::env::var("QI_DB_PATH").ok().as_deref() == Some(":memory:") {
        return std::env::temp_dir().join(format!("qi_memory_icloud_{}", std::process::id()));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home_pb = PathBuf::from(&home);
        let cloud_docs = home_pb
            .join("Library")
            .join("Mobile Documents")
            .join("com~apple~CloudDocs");
        if cloud_docs.is_dir() {
            return cloud_docs.join("QiVault");
        }
        return home_pb.join(".qi").join("icloud_vault");
    }
    PathBuf::from(".qi_icloud_vault")
}

#[cfg(target_os = "macos")]
fn is_macos_dataless_cloud_stub(path: &Path) -> bool {
    use std::os::macos::fs::MetadataExt;
    const SF_DATALESS: u32 = 0x4000_0000;
    fs::symlink_metadata(path)
        .map(|m| (m.st_flags() & SF_DATALESS) != 0)
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn is_macos_dataless_cloud_stub(_path: &Path) -> bool {
    false
}

/// 将单设备密文分片写入本地/iCloud Drive 个人目录（不覆写其他设备目录）
pub fn push_envelope_to_dir(base_dir: &Path, env: &EncryptedBatchEnvelope) -> Result<PathBuf, String> {
    let full = base_dir.join(&env.path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建同步目录失败: {e}"))?;
    }
    let json_str = serde_json::to_string_pretty(env).map_err(|e| e.to_string())?;
    fs::write(&full, json_str).map_err(|e| format!("写入 iCloud 分片失败: {e}"))?;
    Ok(full)
}

/// 从 iCloud Drive 目录扫描并收集所有（或除自身外的）设备分片
pub fn pull_envelopes_from_dir(
    base_dir: &Path,
    exclude_device: Option<&str>,
) -> Result<Vec<EncryptedBatchEnvelope>, String> {
    let ops_dir = base_dir.join("ops");
    if !ops_dir.exists() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    let entries = fs::read_dir(&ops_dir).map_err(|e| format!("读取 ops 目录失败: {e}"))?;
    for dev_entry in entries.flatten() {
        let dev_path = dev_entry.path();
        if !dev_path.is_dir() {
            continue;
        }
        let dev_name = dev_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if let Some(ex) = exclude_device
            && dev_name == ex
        {
            continue;
        }
        let mut files: Vec<PathBuf> = fs::read_dir(&dev_path)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("enc"))
            .collect();
        files.sort();
        for fp in files {
            if is_macos_dataless_cloud_stub(&fp) {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&fp)
                && let Ok(env) = serde_json::from_str::<EncryptedBatchEnvelope>(&content)
            {
                found.push(env);
            }
        }
    }
    Ok(found)
}

pub fn push_envelope_to_byo(
    base_url: &str,
    auth_token: &str,
    env: &EncryptedBatchEnvelope,
) -> Result<(), String> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() || auth_token.trim().is_empty() {
        return Err("请先配置 BYO-Server / qi-cloud 地址并完成登录凭证获取".to_owned());
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout_read(Duration::from_secs(10))
        .build();
    let endpoint = format!("{base}/v1/sync/ops/push");
    let resp = agent
        .post(&endpoint)
        .set("Authorization", &format!("Bearer {}", auth_token.trim()))
        .set("Content-Type", "application/json")
        .send_json(env)
        .map_err(|e| format!("推送 E2EE 分片失败: {e}"))?;
    if resp.status() >= 300 {
        return Err(format!("同步节点返回状态 {}", resp.status()));
    }
    Ok(())
}

pub fn pull_envelopes_from_byo(
    base_url: &str,
    auth_token: &str,
    after_id: i64,
    exclude_device: Option<&str>,
) -> Result<(Vec<EncryptedBatchEnvelope>, i64), String> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() || auth_token.trim().is_empty() {
        return Err("请先配置 BYO-Server / qi-cloud 地址并完成登录".to_owned());
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout_read(Duration::from_secs(10))
        .build();
    let ex = exclude_device.unwrap_or("");
    let url = format!("{base}/v1/sync/ops/pull?after_id={after_id}&exclude_device={ex}");
    let val: serde_json::Value = agent
        .get(&url)
        .set("Authorization", &format!("Bearer {}", auth_token.trim()))
        .call()
        .map_err(|e| format!("拉取 E2EE 分片失败: {e}"))?
        .into_json()
        .map_err(|e| format!("解析拉取响应失败: {e}"))?;

    let cursor_id = val["cursor_id"].as_i64().unwrap_or(after_id);
    let arr = val["envelopes"].as_array().cloned().unwrap_or_default();
    let mut list = Vec::new();
    for item in arr {
        if let Ok(env) = serde_json::from_value::<EncryptedBatchEnvelope>(item) {
            list.push(env);
        }
    }
    Ok((list, cursor_id))
}

pub fn push_envelope_to_webdav(
    webdav_root: &str,
    username: &str,
    password: &str,
    env: &EncryptedBatchEnvelope,
) -> Result<(), String> {
    let root = webdav_root.trim().trim_end_matches('/');
    if root.is_empty() {
        return Err("WebDAV 网盘地址不能为空".to_owned());
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout_read(Duration::from_secs(10))
        .build();

    let auth_header = basic_auth_header(username, password);
    for dir_url in [
        format!("{root}/ops"),
        format!("{root}/ops/{}", env.device_id),
    ] {
        let mut req = agent.request("MKCOL", &dir_url);
        if let Some(ref h) = auth_header {
            req = req.set("Authorization", h);
        }
        let _ = req.call();
    }

    let put_url = format!("{root}/{}", env.path);
    let body = serde_json::to_string_pretty(env).map_err(|e| e.to_string())?;
    let mut put_req = agent
        .put(&put_url)
        .set("Content-Type", "application/json");
    if let Some(ref h) = auth_header {
        put_req = put_req.set("Authorization", h);
    }
    put_req
        .send_string(&body)
        .map_err(|e| format!("WebDAV 上传分片失败 ({put_url}): {e}"))?;

    // 追加更新轻量设备清单方便跨端枚举
    let manifest_url = format!("{root}/ops/manifest_{}.json", env.device_id);
    let mut m_req = agent
        .put(&manifest_url)
        .set("Content-Type", "application/json");
    if let Some(ref h) = auth_header {
        m_req = m_req.set("Authorization", h);
    }
    let _ = m_req.send_json(json!({
        "device_id": env.device_id,
        "latest_seq": env.batch_seq,
        "latest_path": env.path
    }));

    Ok(())
}

/// 从 WebDAV PROPFIND 多状态 XML 中提取全部 `<href>` 路径（兼容 `<D:href>` / `<d:href>` / `<href>`）。
pub fn parse_webdav_propfind_hrefs(xml: &str) -> Vec<String> {
    let lower = xml.to_ascii_lowercase();
    let mut hrefs = Vec::new();
    let mut cursor = 0usize;

    while let Some(rel_start) = lower[cursor..].find("href>") {
        let tag_start = cursor + rel_start;
        // 确保紧邻 '<' 或 ':'（例如 `<href>` 或 `<d:href>`），避免匹配 `</href>`
        let is_open_tag = tag_start > 0
            && (lower.as_bytes()[tag_start - 1] == b'<'
                || lower.as_bytes()[tag_start - 1] == b':');
        let content_start = tag_start + "href>".len();
        if !is_open_tag {
            cursor = content_start;
            continue;
        }
        // 检查前面确实是 '<' 而不是 '</'
        let prefix_slice = &lower[..tag_start];
        if let Some(open_lt) = prefix_slice.rfind('<') {
            if lower[open_lt..tag_start].contains('/') {
                cursor = content_start;
                continue;
            }
        }
        if let Some(rel_end) = lower[content_start..].find("</") {
            let val = xml[content_start..content_start + rel_end].trim();
            if !val.is_empty() && !hrefs.iter().any(|h| h == val) {
                hrefs.push(val.to_owned());
            }
            cursor = content_start + rel_end + 2;
        } else {
            break;
        }
    }
    hrefs
}

fn resolve_webdav_href_url(endpoint_root: &str, href: &str) -> String {
    let trimmed_href = href.trim();
    if trimmed_href.starts_with("http://") || trimmed_href.starts_with("https://") {
        return trimmed_href.to_owned();
    }
    let root = endpoint_root.trim().trim_end_matches('/');
    if trimmed_href.starts_with('/') {
        // 提取 scheme://authority
        if let Some(scheme_pos) = root.find("://") {
            let after_scheme = &root[scheme_pos + 3..];
            let authority_end = after_scheme
                .find('/')
                .map(|idx| scheme_pos + 3 + idx)
                .unwrap_or(root.len());
            let origin = &root[..authority_end];
            return format!("{origin}{trimmed_href}");
        }
    }
    format!("{root}/{}", trimmed_href.trim_start_matches('/'))
}

/// 通过 HTTP `PROPFIND` + `GET`（含 Basic Auth）从 WebDAV 拉取所有 `.enc` 加密分片。
pub fn pull_envelopes_from_webdav(
    endpoint_url: &str,
    username: &str,
    password: &str,
) -> Result<Vec<(String, String)>, String> {
    let root = endpoint_url.trim().trim_end_matches('/');
    if root.is_empty() {
        return Err("WebDAV 网盘地址不能为空".to_owned());
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout_read(Duration::from_secs(10))
        .build();
    let auth_header = basic_auth_header(username, password);

    let propfind_body = r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:"><d:prop><d:displayname/><d:resourcetype/></d:prop></d:propfind>"#;
    let ops_target = if root.ends_with("/ops") {
        format!("{root}/")
    } else {
        format!("{root}/ops/")
    };

    let mut prop_req = agent
        .request("PROPFIND", &ops_target)
        .set("Depth", "2")
        .set("Content-Type", "application/xml; charset=utf-8");
    if let Some(ref h) = auth_header {
        prop_req = prop_req.set("Authorization", h);
    }

    let xml_resp = match prop_req.send_string(propfind_body) {
        Ok(resp) => resp
            .into_string()
            .map_err(|e| format!("读取 WebDAV PROPFIND 响应失败: {e}"))?,
        Err(_) => {
            // 若服务器不支持 /ops/ 路径或直接指向目录，回退请求 root
            let mut fallback_req = agent
                .request("PROPFIND", root)
                .set("Depth", "2")
                .set("Content-Type", "application/xml; charset=utf-8");
            if let Some(ref h) = auth_header {
                fallback_req = fallback_req.set("Authorization", h);
            }
            fallback_req
                .send_string(propfind_body)
                .map_err(|e| format!("WebDAV PROPFIND 请求失败 ({root}): {e}"))?
                .into_string()
                .map_err(|e| format!("读取 WebDAV PROPFIND 响应失败: {e}"))?
        }
    };

    let enc_hrefs: Vec<String> = parse_webdav_propfind_hrefs(&xml_resp)
        .into_iter()
        .filter(|h| h.trim_end_matches('/').ends_with(".enc"))
        .collect();

    let mut out = Vec::new();
    for href in enc_hrefs {
        let file_url = resolve_webdav_href_url(root, &href);
        let mut get_req = agent.get(&file_url);
        if let Some(ref h) = auth_header {
            get_req = get_req.set("Authorization", h);
        }
        let body = get_req
            .call()
            .map_err(|e| format!("WebDAV 下载分片失败 ({file_url}): {e}"))?
            .into_string()
            .map_err(|e| format!("读取 WebDAV 分片内容失败 ({file_url}): {e}"))?;
        out.push((href, body));
    }

    Ok(out)
}

fn basic_auth_header(username: &str, password: &str) -> Option<String> {
    if username.trim().is_empty() && password.trim().is_empty() {
        return None;
    }
    let raw = format!("{}:{}", username.trim(), password.trim());
    Some(format!("Basic {}", base64_encode(raw.as_bytes())))
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn build_local_archive_json(
    phrase: &str,
    device_id: &str,
    seq: u64,
    jsonl_payload: &str,
) -> Result<String, String> {
    let key = derive_master_key(phrase)?;
    let envelope = encrypt_sync_batch(&key, device_id, seq, jsonl_payload)?;
    let archive = LocalEncryptedArchive {
        format: "qi-encrypted-vault-v2".to_owned(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        envelope,
    };
    serde_json::to_string_pretty(&archive).map_err(|e| e.to_string())
}

pub fn decrypt_local_archive_json(phrase: &str, archive_json: &str) -> Result<String, String> {
    let key = derive_master_key(phrase)?;
    if let Ok(archive) = serde_json::from_str::<LocalEncryptedArchive>(archive_json) {
        return decrypt_sync_batch(&key, &archive.envelope);
    }
    let env: EncryptedBatchEnvelope =
        serde_json::from_str(archive_json).map_err(|_| "无法识别的 .qivault 加密归档格式".to_owned())?;
    decrypt_sync_batch(&key, &env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn webdav_propfind_xml_and_http_pull_with_basic_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{addr}/dav/qi-vault");

        let key = derive_master_key("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about").unwrap();
        let env = encrypt_sync_batch(&key, "dev_peer", 1, r#"{"kind":"metric","date":"2026-11-13","metric":"steps","source":"garmin","value":8200,"unit":"步"}"#).unwrap();
        let env_json = serde_json::to_string(&env).unwrap();
        let env_json_clone = env_json.clone();

        std::thread::spawn(move || {
            for stream_res in listener.incoming().take(2) {
                let Ok(mut stream) = stream_res else {
                    break;
                };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let mut req_bytes = Vec::new();
                let mut chunk = [0u8; 2048];
                while let Ok(n) = stream.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    req_bytes.extend_from_slice(&chunk[..n]);
                    if let Some(hdr_end) = req_bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let body_start = hdr_end + 4;
                        let hdrs = String::from_utf8_lossy(&req_bytes[..hdr_end]);
                        let content_len = hdrs
                            .lines()
                            .find_map(|line| {
                                let (k, v) = line.split_once(':')?;
                                k.trim()
                                    .eq_ignore_ascii_case("content-length")
                                    .then(|| v.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                        if req_bytes.len() >= body_start + content_len {
                            break;
                        }
                    }
                }
                let req = String::from_utf8_lossy(&req_bytes);
                assert!(
                    req.contains("Authorization: Basic ZGF2X3VzZXI6ZGF2X3Bhc3M="),
                    "missing basic auth in request: {req}"
                );
                let first_line = req.lines().next().unwrap_or("");
                if first_line.starts_with("PROPFIND ") {
                    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response><D:href>/dav/qi-vault/ops/</D:href></D:response>
  <D:response><D:href>/dav/qi-vault/ops/dev_peer/000001.enc</D:href></D:response>
</D:multistatus>"#;
                    let resp = format!(
                        "HTTP/1.1 207 Multi-Status\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        xml.len(),
                        xml
                    );
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.flush();
                    let _ = stream.shutdown(std::net::Shutdown::Write);
                } else if first_line.starts_with("GET /dav/qi-vault/ops/dev_peer/000001.enc") {
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        env_json_clone.len(),
                        env_json_clone
                    );
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.flush();
                    let _ = stream.shutdown(std::net::Shutdown::Write);
                }
            }
        });

        let pulled = pull_envelopes_from_webdav(&base_url, "dav_user", "dav_pass").unwrap();
        assert_eq!(pulled.len(), 1);
        assert_eq!(pulled[0].0, "/dav/qi-vault/ops/dev_peer/000001.enc");
        let decoded_env: EncryptedBatchEnvelope = serde_json::from_str(&pulled[0].1).unwrap();
        let plain = decrypt_sync_batch(&key, &decoded_env).unwrap();
        assert!(plain.contains("8200"));
    }
}

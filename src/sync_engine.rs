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

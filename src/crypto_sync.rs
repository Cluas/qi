//! E2EE 个人云增量同步编码与 12 词保险箱恢复码。
//! 采用设备目录隔离的 `ops/{device_id}/{batch_seq}.enc` 不可变分片，
//! 彻底规避裸同步 `.sqlite` 文件造成的跨端并发锁损坏。

use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 64 词东方生活风轻量助记词表（确保无拼写歧义、跨端易抄录）
const MNEMONIC_WORDS: [&str; 64] = [
    "qing", "zhu", "song", "lan", "yun", "xi", "feng", "shan", "shui", "yue", "chen", "zhao",
    "mian", "qi", "jing", "he", "run", "wen", "cha", "yuan", "lin", "shi", "gu", "quan",
    "bai", "cang", "bi", "dan", "mo", "ya", "su", "jian", "ping", "an", "tai", "ning", "le",
    "xin", "yi", "shen", "mai", "xiu", "yang", "chong", "sheng", "chun", "xia", "qiu", "dong",
    "lu", "shuang", "xue", "yan", "he2", "lu2", "ming", "guang", "zhe", "dao", "yuan2", "fan",
    "ting", "yuan3", "zhi",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedBatchEnvelope {
    pub protocol: String,
    pub cipher: String,
    pub path: String,
    pub device_id: String,
    pub batch_seq: u64,
    pub nonce_hex: String,
    pub ciphertext_hex: String,
    pub payload_bytes: usize,
}

/// 生成 12 词个人健康保险箱恢复码
pub fn generate_recovery_phrase() -> String {
    let mut entropy = [0u8; 16];
    let _ = getrandom(&mut entropy);
    (0..12)
        .map(|i| {
            let idx = (entropy[i] as usize) % MNEMONIC_WORDS.len();
            MNEMONIC_WORDS[idx]
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// 从 12 词助记恢复码确定性派生 256-bit AEAD 主密钥
pub fn derive_master_key(phrase: &str) -> Result<[u8; 32], String> {
    let normalized = phrase
        .split_whitespace()
        .map(|w| w.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.split_whitespace().count() < 8 {
        return Err("恢复码至少需要 8–12 个助记词".to_owned());
    }
    let mut hasher = Sha256::new();
    hasher.update(b"qi-personal-vault-v1:");
    hasher.update(normalized.as_bytes());
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    Ok(key)
}

/// 将单设备增量 JSONL 加密封装为 `ops/{device_id}/{batch_seq}.enc`
pub fn encrypt_sync_batch(
    key: &[u8; 32],
    device_id: &str,
    batch_seq: u64,
    jsonl_payload: &str,
) -> Result<EncryptedBatchEnvelope, String> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let mut nonce_bytes = [0u8; 24];
    getrandom(&mut nonce_bytes).map_err(|e| format!("随机数生成失败: {e}"))?;
    let nonce = XNonce::from_slice(&nonce_bytes);
    let path = format!("ops/{device_id}/{batch_seq:06}.enc");

    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: jsonl_payload.as_bytes(),
                aad: path.as_bytes(),
            },
        )
        .map_err(|e| format!("AEAD 加密失败: {e}"))?;

    Ok(EncryptedBatchEnvelope {
        protocol: "open-health-sync-v2".to_owned(),
        cipher: "XChaCha20-Poly1305".to_owned(),
        path,
        device_id: device_id.to_owned(),
        batch_seq,
        nonce_hex: hex::encode(nonce_bytes),
        ciphertext_hex: hex::encode(ciphertext),
        payload_bytes: jsonl_payload.len(),
    })
}

/// 解密并校验远端个人云拉取的加密增量分片
pub fn decrypt_sync_batch(
    key: &[u8; 32],
    envelope: &EncryptedBatchEnvelope,
) -> Result<String, String> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce_bytes = hex::decode(&envelope.nonce_hex).map_err(|_| "Nonce 格式非法".to_owned())?;
    if nonce_bytes.len() != 24 {
        return Err("XChaCha20 Nonce 长度必须为 24 字节".to_owned());
    }
    let ciphertext =
        hex::decode(&envelope.ciphertext_hex).map_err(|_| "密文十六进制格式非法".to_owned())?;
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(&nonce_bytes),
            Payload {
                msg: &ciphertext,
                aad: envelope.path.as_bytes(),
            },
        )
        .map_err(|_| "密钥不匹配或分片已被篡改".to_owned())?;
    String::from_utf8(plaintext).map_err(|_| "明文 UTF-8 编码损坏".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_e2ee_sync_batch() {
        let phrase = generate_recovery_phrase();
        assert_eq!(phrase.split_whitespace().count(), 12);
        let key = derive_master_key(&phrase).unwrap();
        let payload = r#"{"date":"2026-11-13","metric":"hrv_rmssd_ms","value":56.4}"#;

        let env = encrypt_sync_batch(&key, "iphone-16-pro", 7, payload).unwrap();
        assert_eq!(env.path, "ops/iphone-16-pro/000007.enc");
        assert!(!env.ciphertext_hex.contains("hrv_rmssd_ms"));

        let dec = decrypt_sync_batch(&key, &env).unwrap();
        assert_eq!(dec, payload);

        let wrong_key = derive_master_key("wrong phrase words one two three four five six").unwrap();
        assert!(decrypt_sync_batch(&wrong_key, &env).is_err());
    }
}

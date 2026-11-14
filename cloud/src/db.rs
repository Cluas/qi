//! 极简 Control Plane SQLite 存储层：
//! 严守「零健康明文库」原则 —— 仅保存账号令牌哈希、月度 ZDR 免费配额计数、
//! E2EE 密文信封、Ed25519 OSET 离线权益状态与 1-byte 同栖共居姿态。

use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
};
use chrono::{Datelike, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use getrandom::getrandom;
use rusqlite::{Connection, OptionalExtension, params};
use rustls_pki_types::{CertificateDer, SignatureVerificationAlgorithm, TrustAnchor, UnixTime};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CONTROL_PLANE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS accounts (
    account_id TEXT PRIMARY KEY,
    handle TEXT UNIQUE NOT NULL,
    token_hash TEXT NOT NULL,
    auth_verifier_hash TEXT NOT NULL DEFAULT '',
    plan_tier TEXT NOT NULL DEFAULT 'free',
    created_at TEXT NOT NULL,
    entitlement_expires_epoch INTEGER NOT NULL DEFAULT 0,
    guardian_sponsor INTEGER NOT NULL DEFAULT 0,
    hearth_gift_credits INTEGER NOT NULL DEFAULT 0,
    companion_loyalty_days INTEGER NOT NULL DEFAULT 0,
    pair_code TEXT NOT NULL DEFAULT '',
    partner_account_id TEXT NOT NULL DEFAULT '',
    co_mood INTEGER NOT NULL DEFAULT 0,
    co_outfit_id TEXT NOT NULL DEFAULT 'warm_robe',
    co_care_action TEXT NOT NULL DEFAULT '',
    co_updated_at INTEGER NOT NULL DEFAULT 0,
    bonsai_dew INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS monthly_quotas (
    account_id TEXT NOT NULL,
    year_month TEXT NOT NULL,
    meal_scans INTEGER NOT NULL DEFAULT 0,
    lab_parses INTEGER NOT NULL DEFAULT 0,
    briefings INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (account_id, year_month)
);

CREATE TABLE IF NOT EXISTS e2ee_sync_envelopes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    batch_seq INTEGER NOT NULL,
    path TEXT NOT NULL,
    protocol TEXT NOT NULL,
    cipher TEXT NOT NULL,
    nonce_hex TEXT NOT NULL,
    ciphertext_hex TEXT NOT NULL,
    payload_bytes INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(account_id, device_id, batch_seq)
);

CREATE TABLE IF NOT EXISTS apple_iap_transactions (
    transaction_id TEXT PRIMARY KEY,
    original_transaction_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    purchase_type TEXT NOT NULL,
    tier_granted TEXT NOT NULL,
    environment TEXT NOT NULL DEFAULT 'Sandbox',
    seats_credited INTEGER NOT NULL DEFAULT 0,
    community_gift_code TEXT NOT NULL DEFAULT '',
    expires_epoch INTEGER NOT NULL DEFAULT 0,
    jws_sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_apple_iap_original_tx
    ON apple_iap_transactions(original_transaction_id);
"#;

const ACCOUNT_COLUMN_MIGRATIONS: &[(&str, &str)] = &[
    ("auth_verifier_hash", "TEXT NOT NULL DEFAULT ''"),
    ("entitlement_expires_epoch", "INTEGER NOT NULL DEFAULT 0"),
    ("guardian_sponsor", "INTEGER NOT NULL DEFAULT 0"),
    ("hearth_gift_credits", "INTEGER NOT NULL DEFAULT 0"),
    ("companion_loyalty_days", "INTEGER NOT NULL DEFAULT 0"),
    ("pair_code", "TEXT NOT NULL DEFAULT ''"),
    ("partner_account_id", "TEXT NOT NULL DEFAULT ''"),
    ("co_mood", "INTEGER NOT NULL DEFAULT 0"),
    ("co_outfit_id", "TEXT NOT NULL DEFAULT 'warm_robe'"),
    ("co_care_action", "TEXT NOT NULL DEFAULT ''"),
    ("co_updated_at", "INTEGER NOT NULL DEFAULT 0"),
    ("bonsai_dew", "INTEGER NOT NULL DEFAULT 0"),
];

pub const FREE_LIMIT_MEAL_SCANS: i64 = 3;
pub const FREE_LIMIT_LAB_PARSES: i64 = 1;
pub const FREE_LIMIT_BRIEFINGS: i64 = 30;

pub const MAX_SYNC_ENVELOPE_BYTES: usize = 262_144;
pub const MAX_SYNC_CIPHERTEXT_HEX_LEN: usize = 524_288;
pub const MAX_ACCOUNT_SYNC_TOTAL_BYTES: i64 = 32 * 1024 * 1024;

pub const EXPECTED_IOS_BUNDLE_ID: &str = "eu.org.cluas.qi";
pub const HEARTH_GIFT_PASS_PRODUCT_ID: &str = "com.qi.hearth_gift_pass";
pub const HEARTH_GIFT_PASS_TITLE_ZH: &str = "窗台添柴礼赠券 · 赠予社区漂流席位";
pub const HEARTH_GIFT_PASS_TITLE_EN: &str = "Hearth Gift Pass · Gift 1 Community Seat";

/// Apple Root CA - G3 DER 编码（SHA-256: 63:34:3A:BF:B8:9A:6A:03:EB:B5:7E:9B:3F:5F:A7:BE:7C:4F:5C:75:6F:30:17:B3:A8:C4:88:C3:65:3E:91:79）
pub const APPLE_ROOT_CA_G3_DER: &[u8] = &[
    0x30, 0x82, 0x02, 0x43, 0x30, 0x82, 0x01, 0xc9, 0xa0, 0x03, 0x02, 0x01,
    0x02, 0x02, 0x08, 0x2d, 0xc5, 0xfc, 0x88, 0xd2, 0xc5, 0x4b, 0x95, 0x30,
    0x0a, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x03, 0x30,
    0x67, 0x31, 0x1b, 0x30, 0x19, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x12,
    0x41, 0x70, 0x70, 0x6c, 0x65, 0x20, 0x52, 0x6f, 0x6f, 0x74, 0x20, 0x43,
    0x41, 0x20, 0x2d, 0x20, 0x47, 0x33, 0x31, 0x26, 0x30, 0x24, 0x06, 0x03,
    0x55, 0x04, 0x0b, 0x0c, 0x1d, 0x41, 0x70, 0x70, 0x6c, 0x65, 0x20, 0x43,
    0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74, 0x69, 0x6f, 0x6e,
    0x20, 0x41, 0x75, 0x74, 0x68, 0x6f, 0x72, 0x69, 0x74, 0x79, 0x31, 0x13,
    0x30, 0x11, 0x06, 0x03, 0x55, 0x04, 0x0a, 0x0c, 0x0a, 0x41, 0x70, 0x70,
    0x6c, 0x65, 0x20, 0x49, 0x6e, 0x63, 0x2e, 0x31, 0x0b, 0x30, 0x09, 0x06,
    0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55, 0x53, 0x30, 0x1e, 0x17, 0x0d,
    0x31, 0x34, 0x30, 0x34, 0x33, 0x30, 0x31, 0x38, 0x31, 0x39, 0x30, 0x36,
    0x5a, 0x17, 0x0d, 0x33, 0x39, 0x30, 0x34, 0x33, 0x30, 0x31, 0x38, 0x31,
    0x39, 0x30, 0x36, 0x5a, 0x30, 0x67, 0x31, 0x1b, 0x30, 0x19, 0x06, 0x03,
    0x55, 0x04, 0x03, 0x0c, 0x12, 0x41, 0x70, 0x70, 0x6c, 0x65, 0x20, 0x52,
    0x6f, 0x6f, 0x74, 0x20, 0x43, 0x41, 0x20, 0x2d, 0x20, 0x47, 0x33, 0x31,
    0x26, 0x30, 0x24, 0x06, 0x03, 0x55, 0x04, 0x0b, 0x0c, 0x1d, 0x41, 0x70,
    0x70, 0x6c, 0x65, 0x20, 0x43, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63,
    0x61, 0x74, 0x69, 0x6f, 0x6e, 0x20, 0x41, 0x75, 0x74, 0x68, 0x6f, 0x72,
    0x69, 0x74, 0x79, 0x31, 0x13, 0x30, 0x11, 0x06, 0x03, 0x55, 0x04, 0x0a,
    0x0c, 0x0a, 0x41, 0x70, 0x70, 0x6c, 0x65, 0x20, 0x49, 0x6e, 0x63, 0x2e,
    0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55,
    0x53, 0x30, 0x76, 0x30, 0x10, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d,
    0x02, 0x01, 0x06, 0x05, 0x2b, 0x81, 0x04, 0x00, 0x22, 0x03, 0x62, 0x00,
    0x04, 0x98, 0xe9, 0x2f, 0x3d, 0x40, 0x72, 0xa4, 0xed, 0x93, 0x22, 0x72,
    0x81, 0x13, 0x1c, 0xdd, 0x10, 0x95, 0xf1, 0xc5, 0xa3, 0x4e, 0x71, 0xdc,
    0x14, 0x16, 0xd9, 0x0e, 0xe5, 0xa6, 0x05, 0x2a, 0x77, 0x64, 0x7b, 0x5f,
    0x4e, 0x38, 0xd3, 0xbb, 0x1c, 0x44, 0xb5, 0x7f, 0xf5, 0x1f, 0xb6, 0x32,
    0x62, 0x5d, 0xc9, 0xe9, 0x84, 0x5b, 0x4f, 0x30, 0x4f, 0x11, 0x5a, 0x00,
    0xfd, 0x58, 0x58, 0x0c, 0xa5, 0xf5, 0x0f, 0x2c, 0x4d, 0x07, 0x47, 0x13,
    0x75, 0xda, 0x97, 0x97, 0x97, 0x6f, 0x31, 0x5c, 0xed, 0x2b, 0x9d, 0x7b,
    0x20, 0x3b, 0xd8, 0xb9, 0x54, 0xd9, 0x5e, 0x99, 0xa4, 0x3a, 0x51, 0x0a,
    0x31, 0xa3, 0x42, 0x30, 0x40, 0x30, 0x1d, 0x06, 0x03, 0x55, 0x1d, 0x0e,
    0x04, 0x16, 0x04, 0x14, 0xbb, 0xb0, 0xde, 0xa1, 0x58, 0x33, 0x88, 0x9a,
    0xa4, 0x8a, 0x99, 0xde, 0xbe, 0xbd, 0xeb, 0xaf, 0xda, 0xcb, 0x24, 0xab,
    0x30, 0x0f, 0x06, 0x03, 0x55, 0x1d, 0x13, 0x01, 0x01, 0xff, 0x04, 0x05,
    0x30, 0x03, 0x01, 0x01, 0xff, 0x30, 0x0e, 0x06, 0x03, 0x55, 0x1d, 0x0f,
    0x01, 0x01, 0xff, 0x04, 0x04, 0x03, 0x02, 0x01, 0x06, 0x30, 0x0a, 0x06,
    0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x03, 0x03, 0x68, 0x00,
    0x30, 0x65, 0x02, 0x31, 0x00, 0x83, 0xe9, 0xc1, 0xc4, 0x16, 0x5e, 0x1a,
    0x5d, 0x34, 0x18, 0xd9, 0xed, 0xef, 0xf4, 0x6c, 0x0e, 0x00, 0x46, 0x4b,
    0xb8, 0xdf, 0xb2, 0x46, 0x11, 0xc5, 0x0f, 0xfd, 0xe6, 0x7a, 0x8c, 0xa1,
    0xa6, 0x6b, 0xce, 0xc2, 0x03, 0xd4, 0x9c, 0xf5, 0x93, 0xc6, 0x74, 0xb8,
    0x6a, 0xdf, 0xaa, 0x23, 0x15, 0x02, 0x30, 0x6d, 0x66, 0x8a, 0x10, 0xca,
    0xd4, 0x0d, 0xd4, 0x4f, 0xcd, 0x8d, 0x43, 0x3e, 0xb4, 0x8a, 0x63, 0xa5,
    0x33, 0x6e, 0xe3, 0x6d, 0xda, 0x17, 0xb7, 0x64, 0x1f, 0xc8, 0x53, 0x26,
    0xf9, 0x88, 0x62, 0x74, 0x39, 0x0b, 0x17, 0x5b, 0xcb, 0x51, 0xa8, 0x0c,
    0xe8, 0x18, 0x03, 0xe7, 0xa2, 0xb2, 0x28,
];

/// 与客户端 `src/entitlements.rs` 对齐的默认签名种子（生产环境通过 `QI_CLOUD_SIGNING_SEED_HEX` 覆盖）。
pub const DEFAULT_ATELIER_SIGNING_SEED: [u8; 32] = [
    0x51, 0x69, 0x41, 0x74, 0x65, 0x6c, 0x69, 0x65,
    0x72, 0x32, 0x30, 0x32, 0x36, 0x5f, 0x45, 0x64,
    0x32, 0x35, 0x35, 0x31, 0x39, 0x5f, 0x4f, 0x53,
    0x45, 0x54, 0x5f, 0x56, 0x34, 0x4b, 0x65, 0x79,
];
pub const OFFLINE_GRACE_SECONDS: i64 = 7 * 86_400;
pub const LOYALTY_MIN_CHECKIN_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionTier {
    Free,
    ProMonthly,
    Seasonal,
    ProAnnual,
    Duo,
    Family,
    LifetimeLocal,
}

impl SubscriptionTier {
    pub fn from_str(raw: &str) -> Self {
        Self::from_str_checked(raw).unwrap_or(Self::Free)
    }

    pub fn from_str_checked(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "free" => Some(Self::Free),
            "pro_monthly" | "monthly" => Some(Self::ProMonthly),
            "seasonal" | "solar_season" => Some(Self::Seasonal),
            "pro_annual" | "annual" | "pro" => Some(Self::ProAnnual),
            "duo" => Some(Self::Duo),
            "family" => Some(Self::Family),
            "lifetime_local" | "lifetime" => Some(Self::LifetimeLocal),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::ProMonthly => "pro_monthly",
            Self::Seasonal => "seasonal",
            Self::ProAnnual => "pro_annual",
            Self::Duo => "duo",
            Self::Family => "family",
            Self::LifetimeLocal => "lifetime_local",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Free => "素心清欢 · Free",
            Self::ProMonthly => "知气月雅 · 月卡",
            Self::Seasonal => "岁时节气季卡",
            Self::ProAnnual => "知气雅集 · 年卡",
            Self::Duo => "双人同窗 · Duo",
            Self::Family => "四时家书 · Family",
            Self::LifetimeLocal => "买断本地 · 终身版",
        }
    }

    pub fn is_paid(&self) -> bool {
        !matches!(self, Self::Free)
    }

    pub fn default_seats(&self) -> u8 {
        match self {
            Self::Duo => 2,
            Self::Family => 4,
            _ => 1,
        }
    }

    pub fn default_duration_seconds(&self) -> i64 {
        match self {
            Self::Free => 0,
            Self::ProMonthly => 30 * 86_400,
            Self::Seasonal => 92 * 86_400,
            Self::ProAnnual | Self::Duo | Self::Family => 365 * 86_400,
            Self::LifetimeLocal => 100 * 365 * 86_400,
        }
    }

    pub fn default_features(&self) -> Vec<String> {
        let mut base: Vec<&str> = vec![
            "raw_biometrics",
            "single_lab_parse",
            "jsonl_export",
            "recovery_priority",
            "basic_local_ai",
            "world_switcher_free",
            "free_solar_track",
            "week1_companion_letter",
            "sticker_quota_5",
            "custom_pet_1",
        ];
        if self.is_paid() {
            base.extend([
                "atelier_seasonal_track",
                "weekly_letters_archive",
                "unlimited_stickers",
                "ai_sticker_purine",
                "multi_lag_radar",
                "custom_washout_n_of_1",
                "wrist_widget_studio",
                "poster_custom_seals",
                "four_seasons_scroll",
                "preset_slots_unlimited",
                "keepsake_slot_3",
                "co_sanctuary_host",
                "webdav_bidirectional",
                "custom_pets_3",
                "custom_pet_ex_libris",
                "pet_weekly_letters",
            ]);
            if !matches!(self, Self::LifetimeLocal) {
                base.push("cloud_e2ee_sync");
            }
            if matches!(
                self,
                Self::ProAnnual | Self::Duo | Self::Family | Self::LifetimeLocal
            ) {
                base.push("multi_profile_labs");
            }
            if matches!(self, Self::Family | Self::ProAnnual | Self::LifetimeLocal) {
                base.push("parents_peace_letter");
            }
        }
        base.into_iter().map(ToString::to_string).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsetClaims {
    pub ver: u8,
    pub iss: String,
    pub sub: String,
    pub tier: String,
    pub issued_at: i64,
    pub refresh_after: i64,
    pub exp_epoch: i64,
    pub grace_until: i64,
    pub features: Vec<String>,
    pub seats: u8,
    pub guardian_sponsor: bool,
    pub companion_loyalty_eligible: bool,
    pub nonce: String,
}

pub fn env_production_security_mode() -> bool {
    std::env::var("QI_PRODUCTION_MODE").ok().as_deref() == Some("1")
        || (!cfg!(debug_assertions) && !cfg!(feature = "mock-billing"))
}

pub fn validate_production_signing_key(
    production_mode: bool,
    explicit_seed: Option<[u8; 32]>,
) -> Result<Option<[u8; 32]>, String> {
    if let Some(seed) = explicit_seed {
        if production_mode && seed == DEFAULT_ATELIER_SIGNING_SEED {
            return Err(
                "生产模式禁止使用公开默认签名种子 DEFAULT_ATELIER_SIGNING_SEED".to_owned(),
            );
        }
        return Ok(Some(seed));
    }
    match std::env::var("QI_CLOUD_SIGNING_SEED_HEX") {
        Ok(raw) if !raw.trim().is_empty() => {
            let bytes = hex::decode(raw.trim())
                .map_err(|_| "QI_CLOUD_SIGNING_SEED_HEX 必须为合法 Hex 编码".to_owned())?;
            if bytes.len() != 32 {
                return Err("QI_CLOUD_SIGNING_SEED_HEX 解码后必须严格为 32 字节".to_owned());
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            if production_mode && arr == DEFAULT_ATELIER_SIGNING_SEED {
                return Err(
                    "生产模式禁止使用公开默认签名种子 DEFAULT_ATELIER_SIGNING_SEED".to_owned(),
                );
            }
            Ok(Some(arr))
        }
        _ => {
            if production_mode {
                Err("生产模式必须配置独立 32 字节 Hex 私钥 QI_CLOUD_SIGNING_SEED_HEX".to_owned())
            } else {
                Ok(None)
            }
        }
    }
}

pub fn default_signing_key() -> SigningKey {
    if let Ok(hex_seed) = std::env::var("QI_CLOUD_SIGNING_SEED_HEX") {
        if let Ok(bytes) = hex::decode(hex_seed.trim()) {
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                return SigningKey::from_bytes(&arr);
            }
        }
    }
    SigningKey::from_bytes(&DEFAULT_ATELIER_SIGNING_SEED)
}

pub fn default_verifying_key() -> VerifyingKey {
    if let Ok(hex_pub) = std::env::var("QI_ENTITLEMENT_PUBKEY_HEX") {
        if let Ok(bytes) = hex::decode(hex_pub.trim()) {
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                if let Ok(vk) = VerifyingKey::from_bytes(&arr) {
                    return vk;
                }
            }
        }
    }
    default_signing_key().verifying_key()
}

pub fn mint_oset_token_with_key(
    subject: &str,
    tier: SubscriptionTier,
    now_epoch: i64,
    exp_epoch: i64,
    guardian_sponsor: bool,
    companion_loyalty_days: i64,
    signing_key: &SigningKey,
) -> Result<String, String> {
    if !tier.is_paid() && !guardian_sponsor {
        return Ok(String::new());
    }
    let effective_exp = if !tier.is_paid() && exp_epoch <= 0 {
        now_epoch.saturating_add(100 * 365 * 86_400)
    } else {
        exp_epoch
    };
    let grace_until = effective_exp.saturating_add(OFFLINE_GRACE_SECONDS);
    let mut salt = [0u8; 8];
    let _ = getrandom(&mut salt);
    let mut hasher = Sha256::new();
    hasher.update(subject.as_bytes());
    hasher.update(tier.as_str().as_bytes());
    hasher.update(now_epoch.to_le_bytes());
    hasher.update(salt);
    let nonce = hex::encode(&hasher.finalize()[..8]);

    let claims = OsetClaims {
        ver: 1,
        iss: "qi-atelier-auth".to_owned(),
        sub: subject.to_owned(),
        tier: tier.as_str().to_owned(),
        issued_at: now_epoch,
        refresh_after: now_epoch.saturating_add(7 * 86_400),
        exp_epoch: effective_exp,
        grace_until,
        features: tier.default_features(),
        seats: tier.default_seats(),
        guardian_sponsor,
        companion_loyalty_eligible: companion_loyalty_days >= LOYALTY_MIN_CHECKIN_DAYS,
        nonce,
    };

    let json = serde_json::to_vec(&claims).map_err(|e| format!("OSET claims 序列化失败: {e}"))?;
    let b64_claims = URL_SAFE_NO_PAD.encode(json);
    let signing_input = format!("qient1.{b64_claims}");
    let sig: Signature = signing_key.sign(signing_input.as_bytes());
    let b64_sig = URL_SAFE_NO_PAD.encode(sig.to_bytes());
    Ok(format!("{signing_input}.{b64_sig}"))
}

pub fn mint_oset_token(
    subject: &str,
    tier: SubscriptionTier,
    now_epoch: i64,
    exp_epoch: i64,
    guardian_sponsor: bool,
    companion_loyalty_days: i64,
) -> Result<String, String> {
    let signing_key = default_signing_key();
    mint_oset_token_with_key(
        subject,
        tier,
        now_epoch,
        exp_epoch,
        guardian_sponsor,
        companion_loyalty_days,
        &signing_key,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountIdentity {
    pub account_id: String,
    pub handle: String,
    pub plan_tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementsSnapshot {
    pub account_id: String,
    pub handle: String,
    pub plan_tier: String,
    pub tier_display: String,
    pub year_month: String,
    pub month: String,
    pub meal_scans_used: i64,
    pub meal_scans_remaining: i64,
    pub lab_parses_used: i64,
    pub lab_parses_remaining: i64,
    pub briefings_used: i64,
    pub briefings_remaining: i64,
    pub icloud_sync_free: bool,
    pub cross_platform_webdav_unlocked: bool,
    pub webdav_unlocked: bool,
    pub local_ollama_unlocked: bool,
    pub entitlement_token: String,
    pub entitlement_expires_epoch: i64,
    pub guardian_sponsor: bool,
    pub hearth_gift_credits: i64,
    pub hearth_gift_pass_title_zh: String,
    pub hearth_gift_pass_title_en: String,
    pub companion_loyalty_days: i64,
    pub seats: u8,
    pub pair_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoSanctuaryParty {
    pub mood: u8,
    pub mood_label: String,
    pub outfit_id: String,
    pub care_action: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoSanctuaryStatus {
    pub paired: bool,
    pub pair_code: String,
    pub bonsai_dew: i64,
    #[serde(rename = "self")]
    pub self_state: CoSanctuaryParty,
    pub partner: Option<CoSanctuaryParty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEnvelopeInput {
    pub device_id: String,
    pub batch_seq: u64,
    pub path: String,
    pub protocol: String,
    pub cipher: String,
    pub nonce_hex: String,
    pub ciphertext_hex: String,
    pub payload_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSyncEnvelope {
    pub id: i64,
    pub device_id: String,
    pub batch_seq: u64,
    pub path: String,
    pub protocol: String,
    pub cipher: String,
    pub nonce_hex: String,
    pub ciphertext_hex: String,
    pub payload_bytes: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionReceipt {
    pub account_id: String,
    pub deleted_accounts: usize,
    pub deleted_quota_rows: usize,
    pub deleted_e2ee_envelopes: usize,
    pub deleted_apple_transactions: usize,
    pub wiped_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppleVerifyRequest {
    #[serde(alias = "signed_transaction_info", alias = "jws")]
    pub jws_representation: Option<String>,
    #[serde(alias = "productId")]
    pub product_id: Option<String>,
    #[serde(alias = "transactionId")]
    pub transaction_id: Option<String>,
    #[serde(alias = "originalTransactionId")]
    pub original_transaction_id: Option<String>,
    pub environment: Option<String>,
    #[serde(alias = "bundleId")]
    pub bundle_id: Option<String>,
    #[serde(alias = "expiresDate")]
    pub expires_date_epoch: Option<i64>,
    pub quantity: Option<i64>,
    pub companion_loyalty_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleVerifyResponse {
    pub verified: bool,
    pub verification_mode: String,
    pub environment: String,
    pub product_id: String,
    pub purchase_type: String,
    pub product_display_zh: String,
    pub product_display_en: String,
    pub transaction_id: String,
    pub original_transaction_id: String,
    pub idempotent_replay: bool,
    pub entitlement_token: String,
    pub guardian_receipt_token: Option<String>,
    pub plan_tier: String,
    pub seats_credited: i64,
    pub hearth_gift_credits_total: i64,
    pub hearth_gift_pass_title_zh: String,
    pub hearth_gift_pass_title_en: String,
    pub community_gift_code: Option<String>,
    pub entitlements: EntitlementsSnapshot,
}

#[derive(Debug, Clone)]
pub enum AppleVerifyError {
    BadRequest(&'static str, String),
    Conflict(&'static str, String),
    Internal(String),
}

#[derive(Debug, Clone)]
pub enum ControlPlaneError {
    Unauthorized(&'static str, String),
    Forbidden(&'static str, String),
    BadRequest(&'static str, String),
    PayloadTooLarge(&'static str, String),
    Internal(String),
}

pub struct ControlPlaneDb {
    conn: Mutex<Connection>,
    production_mode: bool,
    signing_seed: Option<[u8; 32]>,
    test_apple_root_der: Option<Vec<u8>>,
}

fn current_year_month() -> String {
    let now = Utc::now();
    format!("{:04}-{:02}", now.year(), now.month())
}

fn random_hex(bytes_len: usize) -> String {
    let mut buf = vec![0u8; bytes_len];
    let _ = getrandom(&mut buf);
    hex::encode(buf)
}

fn hash_token(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"qi-cloud-bearer-v1:");
    h.update(token.as_bytes());
    hex::encode(h.finalize())
}

fn hash_auth_proof(proof: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"qi-cloud-auth-proof-v1:");
    h.update(proof.trim().as_bytes());
    hex::encode(h.finalize())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn mood_label(mood: u8) -> &'static str {
    match mood {
        0 => "SUNNY_PRIME",
        1 => "BREEZY_STEADY",
        _ => "COZY_RECOVER",
    }
}

fn generate_pair_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut raw = [0u8; 6];
    let _ = getrandom(&mut raw);
    raw.iter()
        .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
        .collect()
}

fn ensure_migrations(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(accounts)")
        .map_err(|e| e.to_string())?;
    let existing: HashSet<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|e| e.to_string())?;
    for (col_name, col_def) in ACCOUNT_COLUMN_MIGRATIONS {
        if !existing.contains(*col_name) {
            let sql = format!("ALTER TABLE accounts ADD COLUMN {col_name} {col_def}");
            conn.execute(&sql, []).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApplePurchaseKind {
    Subscription(SubscriptionTier),
    NonConsumable(SubscriptionTier),
    ConsumableHearthGiftPass,
}

#[derive(Debug, Clone, Copy)]
struct AppleProductSpec {
    product_id: &'static str,
    purchase_type: &'static str,
    kind: ApplePurchaseKind,
    display_zh: &'static str,
    display_en: &'static str,
}

impl AppleProductSpec {
    fn from_product_id(raw: &str) -> Option<Self> {
        match raw.trim() {
            "com.qi.atelier.pro_monthly" => Some(Self {
                product_id: "com.qi.atelier.pro_monthly",
                purchase_type: "auto_renewable_subscription",
                kind: ApplePurchaseKind::Subscription(SubscriptionTier::ProMonthly),
                display_zh: "知气月雅 · 月卡",
                display_en: "Atelier Pro Monthly",
            }),
            "com.qi.atelier.seasonal" => Some(Self {
                product_id: "com.qi.atelier.seasonal",
                purchase_type: "auto_renewable_subscription",
                kind: ApplePurchaseKind::Subscription(SubscriptionTier::Seasonal),
                display_zh: "岁时节气季卡",
                display_en: "Atelier Seasonal",
            }),
            "com.qi.atelier.pro_annual" => Some(Self {
                product_id: "com.qi.atelier.pro_annual",
                purchase_type: "auto_renewable_subscription",
                kind: ApplePurchaseKind::Subscription(SubscriptionTier::ProAnnual),
                display_zh: "知气雅集 · 年卡",
                display_en: "Atelier Pro Yearly",
            }),
            "com.qi.atelier.duo" => Some(Self {
                product_id: "com.qi.atelier.duo",
                purchase_type: "auto_renewable_subscription",
                kind: ApplePurchaseKind::Subscription(SubscriptionTier::Duo),
                display_zh: "双人同窗 · Duo",
                display_en: "Atelier Duo (2 Seats)",
            }),
            "com.qi.atelier.family" => Some(Self {
                product_id: "com.qi.atelier.family",
                purchase_type: "auto_renewable_subscription",
                kind: ApplePurchaseKind::Subscription(SubscriptionTier::Family),
                display_zh: "四时家书 · Family",
                display_en: "Atelier Family (4 Seats)",
            }),
            "com.qi.atelier.lifetime_local" => Some(Self {
                product_id: "com.qi.atelier.lifetime_local",
                purchase_type: "non_consumable",
                kind: ApplePurchaseKind::NonConsumable(SubscriptionTier::LifetimeLocal),
                display_zh: "买断本地 · 终身版",
                display_en: "Atelier Lifetime Local",
            }),
            HEARTH_GIFT_PASS_PRODUCT_ID => Some(Self {
                product_id: HEARTH_GIFT_PASS_PRODUCT_ID,
                purchase_type: "consumable",
                kind: ApplePurchaseKind::ConsumableHearthGiftPass,
                display_zh: HEARTH_GIFT_PASS_TITLE_ZH,
                display_en: HEARTH_GIFT_PASS_TITLE_EN,
            }),
            _ => None,
        }
    }
}

struct VerifiedAppleFields {
    spec: AppleProductSpec,
    transaction_id: String,
    original_transaction_id: String,
    environment: String,
    verification_mode: String,
    expires_epoch: i64,
    quantity: i64,
    jws_sha256: String,
}

fn decode_b64url_flexible(segment: &str) -> Result<Vec<u8>, String> {
    let clean = segment.trim();
    if clean.is_empty() {
        return Err("JWS 分段不能为空".to_owned());
    }
    URL_SAFE_NO_PAD
        .decode(clean)
        .or_else(|_| URL_SAFE.decode(clean))
        .map_err(|_| "JWS Base64URL 解码失败".to_owned())
}

fn json_string_or_number(val: &serde_json::Value) -> Option<String> {
    match val {
        serde_json::Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_owned())
            }
        }
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn json_epoch_seconds(val: &serde_json::Value) -> Option<i64> {
    let raw = match val {
        serde_json::Value::Number(n) => n.as_i64()?,
        serde_json::Value::String(s) => s.trim().parse::<i64>().ok()?,
        _ => return None,
    };
    Some(normalize_epoch_seconds(raw))
}

fn normalize_epoch_seconds(raw: i64) -> i64 {
    if raw >= 10_000_000_000 {
        raw / 1000
    } else {
        raw
    }
}

fn valid_tx_identifier(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

fn normalize_apple_env(raw: &str) -> Result<&'static str, AppleVerifyError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "production" | "prod" => Ok("Production"),
        "sandbox" => Ok("Sandbox"),
        "xcode" => Ok("Xcode"),
        _ => Err(AppleVerifyError::BadRequest(
            "invalid_environment",
            "environment 仅支持 Production / Sandbox / Xcode".to_owned(),
        )),
    }
}

fn apple_sandbox_verifying_key() -> VerifyingKey {
    if let Ok(hex_pub) = std::env::var("QI_APPLE_SANDBOX_VERIFY_KEY_HEX") {
        if let Ok(bytes) = hex::decode(hex_pub.trim()) {
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                if let Ok(vk) = VerifyingKey::from_bytes(&arr) {
                    return vk;
                }
            }
        }
    }
    default_verifying_key()
}

struct AppleStoreKitEku;

impl webpki::ExtendedKeyUsageValidator for AppleStoreKitEku {
    fn validate(&self, iter: webpki::KeyPurposeIdIter<'_, '_>) -> Result<(), webpki::Error> {
        for item in iter {
            let id = item?;
            let oid = id.to_decoded_oid();
            if oid.as_slice() == webpki::KeyUsage::SERVER_AUTH_REPR
                || oid.as_slice() == webpki::KeyUsage::CLIENT_AUTH_REPR
            {
                return Err(webpki::Error::ExtensionValueInvalid);
            }
        }
        Ok(())
    }
}

fn encode_asn1_unsigned_integer(bytes: &[u8]) -> Vec<u8> {
    let mut s = bytes;
    while s.len() > 1 && s[0] == 0 {
        s = &s[1..];
    }
    let needs_zero_pad = s[0] & 0x80 != 0;
    let len = s.len() + usize::from(needs_zero_pad);
    let mut out = Vec::with_capacity(2 + len);
    out.push(0x02);
    out.push(len as u8);
    if needs_zero_pad {
        out.push(0x00);
    }
    out.extend_from_slice(s);
    out
}

fn p1363_to_asn1_der_ecdsa(raw_sig: &[u8]) -> Result<Vec<u8>, AppleVerifyError> {
    if raw_sig.len() != 64 {
        return Err(AppleVerifyError::BadRequest(
            "invalid_jws_signature",
            "ES256 JWS 签名长度必须为 64 字节 P1363 格式".to_owned(),
        ));
    }
    let (r, s) = raw_sig.split_at(32);
    if r.iter().all(|&b| b == 0) || s.iter().all(|&b| b == 0) {
        return Err(AppleVerifyError::BadRequest(
            "invalid_jws_signature",
            "ES256 JWS 签名分量不可全为零".to_owned(),
        ));
    }
    let enc_r = encode_asn1_unsigned_integer(r);
    let enc_s = encode_asn1_unsigned_integer(s);
    let seq_len = enc_r.len() + enc_s.len();
    let mut der = Vec::with_capacity(2 + seq_len);
    der.push(0x30);
    der.push(seq_len as u8);
    der.extend_from_slice(&enc_r);
    der.extend_from_slice(&enc_s);
    Ok(der)
}

fn verify_apple_es256_jws(
    header: &serde_json::Value,
    signing_input: &str,
    sig_bytes: &[u8],
    now_epoch: i64,
    strict_prod: bool,
    extra_root_ca_der: Option<&[u8]>,
) -> Result<(), AppleVerifyError> {
    let x5c_arr = header
        .get("x5c")
        .and_then(|v| v.as_array())
        .filter(|arr| !arr.is_empty())
        .ok_or_else(|| {
            AppleVerifyError::BadRequest(
                "missing_x5c_chain",
                "ES256 JWS 必须携带非空 Apple x5c 证书链".to_owned(),
            )
        })?;
    if x5c_arr.len() > 8 {
        return Err(AppleVerifyError::BadRequest(
            "invalid_x5c_chain",
            "x5c 证书链深度超过上限".to_owned(),
        ));
    }

    let mut cert_ders = Vec::with_capacity(x5c_arr.len());
    for item in x5c_arr {
        let b64 = item.as_str().map(str::trim).filter(|s| !s.is_empty()).ok_or_else(|| {
            AppleVerifyError::BadRequest(
                "invalid_x5c_chain",
                "x5c 数组元素必须为非空 Base64 证书字符串".to_owned(),
            )
        })?;
        let der = STANDARD
            .decode(b64)
            .or_else(|_| STANDARD_NO_PAD.decode(b64))
            .map_err(|_| {
                AppleVerifyError::BadRequest(
                    "invalid_x5c_chain",
                    "x5c 证书 Base64 解码失败".to_owned(),
                )
            })?;
        cert_ders.push(der);
    }

    let apple_root_der = CertificateDer::from(APPLE_ROOT_CA_G3_DER);
    let mut trust_anchors: Vec<TrustAnchor<'_>> = Vec::with_capacity(2);
    let apple_anchor = webpki::anchor_from_trusted_cert(&apple_root_der).map_err(|e| {
        AppleVerifyError::Internal(format!("内置 Apple Root CA - G3 锚点解析失败: {e:?}"))
    })?;
    trust_anchors.push(apple_anchor);

    let extra_holder: Option<CertificateDer<'_>> = if !strict_prod {
        extra_root_ca_der.map(CertificateDer::from)
    } else {
        None
    };
    if let Some(ref extra_der) = extra_holder {
        let anchor = webpki::anchor_from_trusted_cert(extra_der).map_err(|e| {
            AppleVerifyError::BadRequest(
                "invalid_test_root_ca",
                format!("测试根证书锚点解析失败: {e:?}"),
            )
        })?;
        trust_anchors.push(anchor);
    }

    let leaf_der = CertificateDer::from(cert_ders[0].as_slice());
    let intermediates: Vec<CertificateDer<'_>> = cert_ders[1..]
        .iter()
        .map(|d| CertificateDer::from(d.as_slice()))
        .collect();

    let leaf_cert = webpki::EndEntityCert::try_from(&leaf_der).map_err(|e| {
        AppleVerifyError::BadRequest(
            "invalid_x5c_leaf",
            format!("x5c 叶证书 X.509 DER 解析失败: {e:?}"),
        )
    })?;

    let supported_sig_algs: &[&dyn SignatureVerificationAlgorithm] = &[
        webpki::ring::ECDSA_P256_SHA256,
        webpki::ring::ECDSA_P256_SHA384,
        webpki::ring::ECDSA_P384_SHA256,
        webpki::ring::ECDSA_P384_SHA384,
        webpki::ring::RSA_PKCS1_2048_8192_SHA256,
        webpki::ring::RSA_PKCS1_2048_8192_SHA384,
        webpki::ring::RSA_PKCS1_2048_8192_SHA512,
    ];
    let verify_time = UnixTime::since_unix_epoch(Duration::from_secs(now_epoch.max(0) as u64));

    leaf_cert
        .verify_for_usage(
            supported_sig_algs,
            &trust_anchors,
            &intermediates,
            verify_time,
            AppleStoreKitEku,
            None,
            None,
        )
        .map_err(|e| {
            AppleVerifyError::BadRequest(
                "x5c_chain_untrusted",
                format!("Apple x5c 证书链未通过可信根校验: {e:?}"),
            )
        })?;

    let der_sig = p1363_to_asn1_der_ecdsa(sig_bytes)?;
    leaf_cert
        .verify_signature(
            webpki::ring::ECDSA_P256_SHA256,
            signing_input.as_bytes(),
            &der_sig,
        )
        .map_err(|_| {
            AppleVerifyError::BadRequest(
                "jws_signature_mismatch",
                "ES256 JWS 签名密码学校验未通过".to_owned(),
            )
        })
}

fn parse_and_verify_apple_payload(
    req: &AppleVerifyRequest,
    now_epoch: i64,
    production_mode: bool,
    extra_root_ca_der: Option<&[u8]>,
) -> Result<VerifiedAppleFields, AppleVerifyError> {
    let env_force_prod = std::env::var("QI_APPLE_ENV").as_deref() == Ok("production");
    let strict_prod = production_mode || env_force_prod;

    let jws_opt = req
        .jws_representation
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let (
        product_id,
        transaction_id,
        original_transaction_id,
        bundle_id,
        environment,
        expires_epoch_opt,
        quantity,
        verification_mode,
        jws_sha256,
    ) = if let Some(jws) = jws_opt {
        let parts: Vec<&str> = jws.split('.').collect();
        if parts.len() != 3 {
            return Err(AppleVerifyError::BadRequest(
                "invalid_jws_format",
                "StoreKit 2 JWS 必须包含 3 段 Base64URL".to_owned(),
            ));
        }

        let header_bytes = decode_b64url_flexible(parts[0])
            .map_err(|m| AppleVerifyError::BadRequest("invalid_jws_header", m))?;
        let payload_bytes = decode_b64url_flexible(parts[1])
            .map_err(|m| AppleVerifyError::BadRequest("invalid_jws_payload", m))?;
        let sig_bytes = decode_b64url_flexible(parts[2])
            .map_err(|m| AppleVerifyError::BadRequest("invalid_jws_signature", m))?;

        let header: serde_json::Value = serde_json::from_slice(&header_bytes)
            .map_err(|_| AppleVerifyError::BadRequest("invalid_jws_header_json", "JWS Header 非合法 JSON".to_owned()))?;
        let alg = header
            .get("alg")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("");
        if alg.is_empty() || alg.eq_ignore_ascii_case("none") {
            return Err(AppleVerifyError::BadRequest(
                "invalid_jws_alg",
                "拒绝无签名或 alg=none 的 JWS 载荷".to_owned(),
            ));
        }

        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let verification_mode = match alg {
            "EdDSA" => {
                if strict_prod {
                    return Err(AppleVerifyError::BadRequest(
                        "unsupported_jws_alg",
                        "生产模式禁止 EdDSA，仅允许 Apple ES256 x5c 签名".to_owned(),
                    ));
                }
                let sig = Signature::from_slice(&sig_bytes).map_err(|_| {
                    AppleVerifyError::BadRequest(
                        "invalid_jws_signature",
                        "EdDSA JWS 签名长度非法".to_owned(),
                    )
                })?;
                let vk = apple_sandbox_verifying_key();
                vk.verify_strict(signing_input.as_bytes(), &sig).map_err(|_| {
                    AppleVerifyError::BadRequest(
                        "jws_signature_mismatch",
                        "EdDSA JWS 签名校验未通过".to_owned(),
                    )
                })?;
                "storekit2_jws_eddsa".to_owned()
            }
            "ES256" => {
                verify_apple_es256_jws(
                    &header,
                    &signing_input,
                    &sig_bytes,
                    now_epoch,
                    strict_prod,
                    extra_root_ca_der,
                )?;
                "storekit2_jws_es256".to_owned()
            }
            _ => {
                return Err(AppleVerifyError::BadRequest(
                    "unsupported_jws_alg",
                    format!("不支持的 JWS 算法: {alg}"),
                ));
            }
        };

        let claims: serde_json::Value = serde_json::from_slice(&payload_bytes)
            .map_err(|_| AppleVerifyError::BadRequest("invalid_jws_payload_json", "JWS Payload 非合法 JSON".to_owned()))?;
        if !claims.is_object() {
            return Err(AppleVerifyError::BadRequest(
                "invalid_jws_payload_json",
                "JWS Payload 必须为 JSON 对象".to_owned(),
            ));
        }

        if let Some(rev) = claims.get("revocationDate") {
            if !rev.is_null() {
                return Err(AppleVerifyError::BadRequest(
                    "transaction_revoked",
                    "该 Apple 交易已被退款或撤销".to_owned(),
                ));
            }
        }

        let jws_product_id = claims
            .get("productId")
            .or_else(|| claims.get("product_id"))
            .and_then(json_string_or_number)
            .ok_or_else(|| {
                AppleVerifyError::BadRequest("missing_product_id", "JWS 缺少 productId".to_owned())
            })?;
        let jws_tx_id = claims
            .get("transactionId")
            .or_else(|| claims.get("transaction_id"))
            .and_then(json_string_or_number)
            .ok_or_else(|| {
                AppleVerifyError::BadRequest(
                    "missing_transaction_id",
                    "JWS 缺少 transactionId".to_owned(),
                )
            })?;
        let jws_orig_id = claims
            .get("originalTransactionId")
            .or_else(|| claims.get("original_transaction_id"))
            .and_then(json_string_or_number)
            .unwrap_or_else(|| jws_tx_id.clone());

        if let Some(req_pid) = req.product_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            if req_pid != jws_product_id {
                return Err(AppleVerifyError::BadRequest(
                    "payload_mismatch",
                    "请求 product_id 与 JWS 签名载荷不一致".to_owned(),
                ));
            }
        }
        if let Some(req_tid) = req.transaction_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            if req_tid != jws_tx_id {
                return Err(AppleVerifyError::BadRequest(
                    "payload_mismatch",
                    "请求 transaction_id 与 JWS 签名载荷不一致".to_owned(),
                ));
            }
        }
        if let Some(req_oid) = req
            .original_transaction_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if req_oid != jws_orig_id {
                return Err(AppleVerifyError::BadRequest(
                    "payload_mismatch",
                    "请求 original_transaction_id 与 JWS 签名载荷不一致".to_owned(),
                ));
            }
        }

        let jws_bundle = claims
            .get("bundleId")
            .or_else(|| claims.get("bundle_id"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);
        if let (Some(req_b), Some(j_b)) = (
            req.bundle_id.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            jws_bundle.as_deref(),
        ) {
            if req_b != j_b {
                return Err(AppleVerifyError::BadRequest(
                    "payload_mismatch",
                    "请求 bundle_id 与 JWS 签名载荷不一致".to_owned(),
                ));
            }
        }

        let raw_env = claims
            .get("environment")
            .and_then(|v| v.as_str())
            .unwrap_or("Sandbox");
        let norm_env = normalize_apple_env(raw_env)?.to_owned();
        if strict_prod && norm_env != "Production" {
            return Err(AppleVerifyError::BadRequest(
                "sandbox_rejected_in_production",
                "生产模式拒绝 Sandbox / Xcode Apple 交易凭据".to_owned(),
            ));
        }
        if alg == "EdDSA" && norm_env == "Production" {
            return Err(AppleVerifyError::BadRequest(
                "eddsa_rejected_for_production",
                "EdDSA 测试签名不可用于 Production 环境交易".to_owned(),
            ));
        }

        let expires_epoch = claims
            .get("expiresDate")
            .or_else(|| claims.get("expires_date_epoch"))
            .and_then(json_epoch_seconds)
            .or_else(|| {
                if !strict_prod {
                    req.expires_date_epoch.map(normalize_epoch_seconds)
                } else {
                    None
                }
            });

        let quantity = claims
            .get("quantity")
            .and_then(json_epoch_seconds)
            .or(req.quantity)
            .unwrap_or(1);

        let mut h = Sha256::new();
        h.update(jws.as_bytes());
        let jws_hash = hex::encode(h.finalize());

        (
            jws_product_id,
            jws_tx_id,
            jws_orig_id,
            jws_bundle,
            norm_env,
            expires_epoch,
            quantity,
            verification_mode,
            jws_hash,
        )
    } else {
        if strict_prod {
            return Err(AppleVerifyError::BadRequest(
                "missing_jws_representation",
                "生产环境必须提供 StoreKit 2 ES256 JWS 交易签名".to_owned(),
            ));
        }
        let raw_env = req.environment.as_deref().unwrap_or("");
        let norm_env = normalize_apple_env(raw_env)?;
        if norm_env == "Production" {
            return Err(AppleVerifyError::BadRequest(
                "missing_jws_representation",
                "Production 环境验证必须提供 jws_representation".to_owned(),
            ));
        }

        let pid = req
            .product_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                AppleVerifyError::BadRequest("missing_product_id", "缺少 product_id".to_owned())
            })?
            .to_owned();
        let tid = req
            .transaction_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                AppleVerifyError::BadRequest(
                    "missing_transaction_id",
                    "缺少 transaction_id".to_owned(),
                )
            })?
            .to_owned();
        let oid = req
            .original_transaction_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                AppleVerifyError::BadRequest(
                    "missing_original_transaction_id",
                    "缺少 original_transaction_id".to_owned(),
                )
            })?
            .to_owned();

        let mut h = Sha256::new();
        h.update(format!("sandbox:{norm_env}:{tid}:{oid}:{pid}").as_bytes());
        let sandbox_hash = hex::encode(h.finalize());

        (
            pid,
            tid,
            oid,
            req.bundle_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned),
            norm_env.to_owned(),
            req.expires_date_epoch.map(normalize_epoch_seconds),
            req.quantity.unwrap_or(1),
            "storekit2_sandbox_payload".to_owned(),
            sandbox_hash,
        )
    };

    if bundle_id.as_deref() != Some(EXPECTED_IOS_BUNDLE_ID) {
        return Err(AppleVerifyError::BadRequest(
            "invalid_bundle_id",
            format!("bundle_id 缺失或不匹配，期望 {EXPECTED_IOS_BUNDLE_ID}"),
        ));
    }

    if !valid_tx_identifier(&transaction_id) || !valid_tx_identifier(&original_transaction_id) {
        return Err(AppleVerifyError::BadRequest(
            "invalid_transaction_id",
            "transaction_id 或 original_transaction_id 格式非法".to_owned(),
        ));
    }

    let spec = AppleProductSpec::from_product_id(&product_id).ok_or_else(|| {
        AppleVerifyError::BadRequest(
            "unknown_product_id",
            format!("未知的 Apple 商品标识: {product_id}"),
        )
    })?;

    if !(1..=20).contains(&quantity) {
        return Err(AppleVerifyError::BadRequest(
            "invalid_quantity",
            "quantity 必须在 1..=20 之间".to_owned(),
        ));
    }

    let resolved_expires_epoch = match spec.kind {
        ApplePurchaseKind::Subscription(tier) => {
            if let Some(exp) = expires_epoch_opt {
                if exp <= now_epoch {
                    return Err(AppleVerifyError::BadRequest(
                        "subscription_expired",
                        "该 Apple 订阅周期已过期，未签发新通行证".to_owned(),
                    ));
                }
                exp
            } else {
                now_epoch.saturating_add(tier.default_duration_seconds())
            }
        }
        ApplePurchaseKind::NonConsumable(tier) => {
            now_epoch.saturating_add(tier.default_duration_seconds())
        }
        ApplePurchaseKind::ConsumableHearthGiftPass => 0,
    };

    Ok(VerifiedAppleFields {
        spec,
        transaction_id,
        original_transaction_id,
        environment,
        verification_mode,
        expires_epoch: resolved_expires_epoch,
        quantity,
        jws_sha256,
    })
}

fn resolve_effective_account_tier(
    plan_tier: &str,
    stored_exp: i64,
    now_epoch: i64,
) -> (SubscriptionTier, i64, bool) {
    let raw_tier = SubscriptionTier::from_str(plan_tier);
    if !raw_tier.is_paid() {
        return (SubscriptionTier::Free, 0, false);
    }
    if raw_tier == SubscriptionTier::LifetimeLocal {
        let exp = if stored_exp > 0 {
            stored_exp
        } else {
            now_epoch.saturating_add(raw_tier.default_duration_seconds())
        };
        return (raw_tier, exp, false);
    }
    if stored_exp > now_epoch {
        return (raw_tier, stored_exp, false);
    }
    (SubscriptionTier::Free, 0, stored_exp > 0)
}

fn fetch_effective_account_tier_locked(
    conn: &Connection,
    account_id: &str,
    now_epoch: i64,
) -> Result<(SubscriptionTier, i64), String> {
    let row: Option<(String, i64)> = conn
        .query_row(
            "SELECT plan_tier, entitlement_expires_epoch FROM accounts WHERE account_id = ?1",
            params![account_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((plan_tier, stored_exp)) = row else {
        return Ok((SubscriptionTier::Free, 0));
    };
    let (tier, exp, _) = resolve_effective_account_tier(&plan_tier, stored_exp, now_epoch);
    Ok((tier, exp))
}

impl ControlPlaneDb {
    fn from_connection_with_config(
        conn: Connection,
        production_mode: bool,
        signing_seed: Option<[u8; 32]>,
        test_apple_root_der: Option<Vec<u8>>,
    ) -> Result<Self, String> {
        let validated_seed = validate_production_signing_key(production_mode, signing_seed)?;
        conn.execute_batch(CONTROL_PLANE_SCHEMA)
            .map_err(|e| e.to_string())?;
        ensure_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            production_mode,
            signing_seed: validated_seed,
            test_apple_root_der,
        })
    }

    pub fn open_memory() -> Result<Self, String> {
        Self::open_memory_with_config(env_production_security_mode(), None, None)
    }

    pub fn open_memory_with_production_mode(production_mode: bool) -> Result<Self, String> {
        Self::open_memory_with_config(production_mode, None, None)
    }

    pub fn open_memory_with_config(
        production_mode: bool,
        signing_seed: Option<[u8; 32]>,
        test_apple_root_der: Option<Vec<u8>>,
    ) -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        Self::from_connection_with_config(conn, production_mode, signing_seed, test_apple_root_der)
    }

    pub fn with_test_apple_root(mut self, root_der: Vec<u8>) -> Self {
        self.test_apple_root_der = Some(root_der);
        self
    }

    pub fn open_file(path: &Path) -> Result<Self, String> {
        Self::open_file_with_config(path, env_production_security_mode(), None, None)
    }

    pub fn open_file_with_config(
        path: &Path,
        production_mode: bool,
        signing_seed: Option<[u8; 32]>,
        test_apple_root_der: Option<Vec<u8>>,
    ) -> Result<Self, String> {
        // 先校验生产私钥再创建文件，避免无私钥时在磁盘留下空 DB
        let validated_seed = validate_production_signing_key(production_mode, signing_seed)?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let _ = conn.execute_batch("PRAGMA journal_mode = WAL;");
        Self::from_connection_with_config(conn, production_mode, validated_seed, test_apple_root_der)
    }

    pub fn is_production_mode(&self) -> bool {
        self.production_mode
    }

    pub fn signing_key(&self) -> SigningKey {
        if let Some(seed) = self.signing_seed {
            SigningKey::from_bytes(&seed)
        } else {
            default_signing_key()
        }
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        if let Some(seed) = self.signing_seed {
            SigningKey::from_bytes(&seed).verifying_key()
        } else {
            default_verifying_key()
        }
    }

    /// 首次注册创建账号；已有 handle 必须携带当前有效 Bearer Token 或注册凭据 auth_proof_hex 才允许轮换 Token
    pub fn create_or_rotate_session_with_auth(
        &self,
        handle: &str,
        bearer_token: Option<&str>,
        auth_proof_hex: Option<&str>,
    ) -> Result<(AccountIdentity, String, EntitlementsSnapshot), ControlPlaneError> {
        let clean_handle = handle.trim().to_owned();
        if clean_handle.is_empty() {
            return Err(ControlPlaneError::BadRequest(
                "invalid_handle",
                "账号标识不能为空".to_owned(),
            ));
        }
        let submitted_proof_hash = auth_proof_hex
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(hash_auth_proof);

        let guard = self
            .conn
            .lock()
            .map_err(|_| ControlPlaneError::Internal("DB 锁异常".to_owned()))?;
        let existing: Option<(String, String, String, String)> = guard
            .query_row(
                "SELECT account_id, plan_tier, token_hash, auth_verifier_hash FROM accounts WHERE handle = ?1",
                params![clean_handle],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;

        let raw_token = format!("qitok_{}", random_hex(24));
        let tok_hash = hash_token(&raw_token);
        let now = Utc::now().to_rfc3339();

        let (account_id, plan_tier) = match existing {
            Some((id, tier, stored_token_hash, stored_verifier_hash)) => {
                let bearer_ok = bearer_token
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|raw| {
                        constant_time_eq(
                            hash_token(raw).as_bytes(),
                            stored_token_hash.as_bytes(),
                        )
                    })
                    .unwrap_or(false);
                let proof_ok = if !stored_verifier_hash.is_empty() {
                    submitted_proof_hash
                        .as_deref()
                        .map(|h| constant_time_eq(h.as_bytes(), stored_verifier_hash.as_bytes()))
                        .unwrap_or(false)
                } else {
                    false
                };
                if !(bearer_ok || proof_ok) {
                    return Err(ControlPlaneError::Unauthorized(
                        "handle_already_claimed",
                        "该 handle 已被注册，需要当前有效 Bearer Token 或注册凭据方可轮换会话"
                            .to_owned(),
                    ));
                }
                guard
                    .execute(
                        "UPDATE accounts SET token_hash = ?1 WHERE account_id = ?2",
                        params![tok_hash, id],
                    )
                    .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
                (id, tier)
            }
            None => {
                let new_id = format!("acc_{}", random_hex(8));
                let verifier_hash = submitted_proof_hash.unwrap_or_default();
                guard
                    .execute(
                        "INSERT INTO accounts (account_id, handle, token_hash, auth_verifier_hash, plan_tier, created_at)
                         VALUES (?1, ?2, ?3, ?4, 'free', ?5)",
                        params![new_id, clean_handle, tok_hash, verifier_hash, now],
                    )
                    .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
                (new_id, "free".to_owned())
            }
        };

        let identity = AccountIdentity {
            account_id: account_id.clone(),
            handle: clean_handle,
            plan_tier,
        };
        let sk = self.signing_key();
        let ent = Self::load_entitlements_locked(&guard, &identity, &sk)
            .map_err(ControlPlaneError::Internal)?;
        Ok((identity, raw_token, ent))
    }

    /// 兼容旧签名的会话创建接口（新 handle 注册成功；已存在 handle 无凭据会被拒绝）
    pub fn create_or_rotate_session(
        &self,
        handle: &str,
    ) -> Result<(AccountIdentity, String, EntitlementsSnapshot), String> {
        self.create_or_rotate_session_with_auth(handle, None, None)
            .map_err(|e| match e {
                ControlPlaneError::Unauthorized(_, msg)
                | ControlPlaneError::Forbidden(_, msg)
                | ControlPlaneError::BadRequest(_, msg)
                | ControlPlaneError::PayloadTooLarge(_, msg)
                | ControlPlaneError::Internal(msg) => msg,
            })
    }

    pub fn authenticate(&self, raw_token: &str) -> Result<AccountIdentity, String> {
        let tok_hash = hash_token(raw_token.trim());
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        guard
            .query_row(
                "SELECT account_id, handle, plan_tier FROM accounts WHERE token_hash = ?1",
                params![tok_hash],
                |r| {
                    Ok(AccountIdentity {
                        account_id: r.get(0)?,
                        handle: r.get(1)?,
                        plan_tier: r.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "无效或已注销的登录凭证".to_owned())
    }

    fn load_entitlements_locked(
        conn: &Connection,
        id: &AccountIdentity,
        signing_key: &SigningKey,
    ) -> Result<EntitlementsSnapshot, String> {
        let (raw_plan_tier, stored_exp, guardian_int, hearth_gift_credits, loyalty_days, pair_code): (
            String,
            i64,
            i64,
            i64,
            i64,
            String,
        ) = conn
            .query_row(
                "SELECT plan_tier, entitlement_expires_epoch, guardian_sponsor, hearth_gift_credits, companion_loyalty_days, pair_code
                 FROM accounts WHERE account_id = ?1",
                params![id.account_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| (id.plan_tier.clone(), 0, 0, 0, 0, String::new()));

        let ym = current_year_month();
        let (meal, lab, brief): (i64, i64, i64) = conn
            .query_row(
                "SELECT meal_scans, lab_parses, briefings FROM monthly_quotas
                 WHERE account_id = ?1 AND year_month = ?2",
                params![id.account_id, ym],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or((0, 0, 0));

        let now_epoch = Utc::now().timestamp();
        let (tier_enum, exp_epoch, _) =
            resolve_effective_account_tier(&raw_plan_tier, stored_exp, now_epoch);
        let is_paid = tier_enum.is_paid();
        let effective_plan_tier = if is_paid {
            raw_plan_tier
        } else {
            "free".to_owned()
        };
        let guardian_sponsor = guardian_int != 0;
        let entitlement_token = if is_paid {
            mint_oset_token_with_key(
                &id.account_id,
                tier_enum,
                now_epoch,
                exp_epoch,
                guardian_sponsor,
                loyalty_days,
                signing_key,
            )?
        } else {
            String::new()
        };

        Ok(EntitlementsSnapshot {
            account_id: id.account_id.clone(),
            handle: id.handle.clone(),
            plan_tier: effective_plan_tier,
            tier_display: tier_enum.display_name().to_owned(),
            year_month: ym.clone(),
            month: ym,
            meal_scans_used: meal,
            meal_scans_remaining: if is_paid {
                -1
            } else {
                (FREE_LIMIT_MEAL_SCANS - meal).max(0)
            },
            lab_parses_used: lab,
            lab_parses_remaining: if is_paid {
                -1
            } else {
                (FREE_LIMIT_LAB_PARSES - lab).max(0)
            },
            briefings_used: brief,
            briefings_remaining: if is_paid {
                -1
            } else {
                (FREE_LIMIT_BRIEFINGS - brief).max(0)
            },
            icloud_sync_free: true,
            cross_platform_webdav_unlocked: is_paid,
            webdav_unlocked: is_paid,
            local_ollama_unlocked: is_paid,
            entitlement_token,
            entitlement_expires_epoch: exp_epoch,
            guardian_sponsor,
            hearth_gift_credits,
            hearth_gift_pass_title_zh: HEARTH_GIFT_PASS_TITLE_ZH.to_owned(),
            hearth_gift_pass_title_en: HEARTH_GIFT_PASS_TITLE_EN.to_owned(),
            companion_loyalty_days: loyalty_days,
            seats: tier_enum.default_seats(),
            pair_code,
        })
    }

    pub fn get_entitlements(&self, id: &AccountIdentity) -> Result<EntitlementsSnapshot, String> {
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let sk = self.signing_key();
        Self::load_entitlements_locked(&guard, id, &sk)
    }

    pub fn set_plan_tier(
        &self,
        id: &AccountIdentity,
        new_tier: &str,
    ) -> Result<EntitlementsSnapshot, String> {
        let stored_tier = if new_tier == "pro" {
            "pro"
        } else {
            SubscriptionTier::from_str(new_tier).as_str()
        };
        self.upgrade_plan(id, stored_tier, None, None)
    }

    pub fn upgrade_plan(
        &self,
        id: &AccountIdentity,
        stored_tier: &str,
        guardian_sponsor: Option<bool>,
        companion_loyalty_days: Option<i64>,
    ) -> Result<EntitlementsSnapshot, String> {
        let tier_enum = SubscriptionTier::from_str(stored_tier);
        let now_epoch = Utc::now().timestamp();
        let exp_epoch = if tier_enum.is_paid() {
            now_epoch.saturating_add(tier_enum.default_duration_seconds())
        } else {
            0
        };

        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        guard
            .execute(
                "UPDATE accounts SET plan_tier = ?1, entitlement_expires_epoch = ?2 WHERE account_id = ?3",
                params![stored_tier, exp_epoch, id.account_id],
            )
            .map_err(|e| e.to_string())?;
        if let Some(sponsor) = guardian_sponsor {
            guard
                .execute(
                    "UPDATE accounts SET guardian_sponsor = ?1 WHERE account_id = ?2",
                    params![if sponsor { 1i64 } else { 0i64 }, id.account_id],
                )
                .map_err(|e| e.to_string())?;
        }
        if let Some(days) = companion_loyalty_days {
            guard
                .execute(
                    "UPDATE accounts SET companion_loyalty_days = ?1 WHERE account_id = ?2",
                    params![days.max(0), id.account_id],
                )
                .map_err(|e| e.to_string())?;
        }

        let mut updated = id.clone();
        updated.plan_tier = stored_tier.to_owned();
        let sk = self.signing_key();
        Self::load_entitlements_locked(&guard, &updated, &sk)
    }

    pub fn refresh_entitlements(
        &self,
        id: &AccountIdentity,
    ) -> Result<EntitlementsSnapshot, String> {
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let (plan_tier, stored_exp): (String, i64) = guard
            .query_row(
                "SELECT plan_tier, entitlement_expires_epoch FROM accounts WHERE account_id = ?1",
                params![id.account_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        let now_epoch = Utc::now().timestamp();
        let (effective_tier, _, expired_downgraded) =
            resolve_effective_account_tier(&plan_tier, stored_exp, now_epoch);
        if expired_downgraded || (!effective_tier.is_paid() && (plan_tier != "free" || stored_exp != 0)) {
            guard
                .execute(
                    "UPDATE accounts SET plan_tier = 'free', entitlement_expires_epoch = 0 WHERE account_id = ?1",
                    params![id.account_id],
                )
                .map_err(|e| e.to_string())?;
        }
        let sk = self.signing_key();
        Self::load_entitlements_locked(&guard, id, &sk)
    }

    fn load_cosanctuary_status_locked(
        conn: &Connection,
        account_id: &str,
    ) -> Result<CoSanctuaryStatus, String> {
        let (pair_code, partner_id, co_mood, co_outfit_id, co_care_action, co_updated_at, bonsai_dew): (
            String,
            String,
            i64,
            String,
            String,
            i64,
            i64,
        ) = conn
            .query_row(
                "SELECT pair_code, partner_account_id, co_mood, co_outfit_id, co_care_action, co_updated_at, bonsai_dew
                 FROM accounts WHERE account_id = ?1",
                params![account_id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                    ))
                },
            )
            .map_err(|e| e.to_string())?;

        let self_mood = (co_mood.clamp(0, 2)) as u8;
        let self_state = CoSanctuaryParty {
            mood: self_mood,
            mood_label: mood_label(self_mood).to_owned(),
            outfit_id: co_outfit_id,
            care_action: co_care_action,
            updated_at: co_updated_at,
        };

        let partner = if partner_id.trim().is_empty() {
            None
        } else {
            conn.query_row(
                "SELECT co_mood, co_outfit_id, co_care_action, co_updated_at
                 FROM accounts WHERE account_id = ?1",
                params![partner_id],
                |r| {
                    let m = (r.get::<_, i64>(0)?.clamp(0, 2)) as u8;
                    Ok(CoSanctuaryParty {
                        mood: m,
                        mood_label: mood_label(m).to_owned(),
                        outfit_id: r.get(1)?,
                        care_action: r.get(2)?,
                        updated_at: r.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?
        };

        Ok(CoSanctuaryStatus {
            paired: partner.is_some(),
            pair_code,
            bonsai_dew,
            self_state,
            partner,
        })
    }

    pub fn cosanctuary_pair(
        &self,
        account_id: &str,
        requested_code: Option<&str>,
    ) -> Result<CoSanctuaryStatus, ControlPlaneError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| ControlPlaneError::Internal("DB 锁异常".to_owned()))?;
        let now_epoch = Utc::now().timestamp();
        let trimmed_code = requested_code
            .map(|s| s.trim().to_ascii_uppercase())
            .unwrap_or_default();

        if trimmed_code.is_empty() {
            let (self_tier, _) = fetch_effective_account_tier_locked(&guard, account_id, now_epoch)
                .map_err(ControlPlaneError::Internal)?;
            if !self_tier
                .default_features()
                .iter()
                .any(|f| f == "co_sanctuary_host")
            {
                return Err(ControlPlaneError::Forbidden(
                    "cosanctuary_host_requires_paid_plan",
                    "创建双人同栖房主配对码需要有效付费会员（含 Duo / Family / Pro / Seasonal / Lifetime）"
                        .to_owned(),
                ));
            }
            let existing_code: String = guard
                .query_row(
                    "SELECT pair_code FROM accounts WHERE account_id = ?1",
                    params![account_id],
                    |r| r.get(0),
                )
                .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
            if existing_code.is_empty() {
                for _ in 0..8 {
                    let candidate = generate_pair_code();
                    let clash: Option<String> = guard
                        .query_row(
                            "SELECT account_id FROM accounts WHERE pair_code = ?1",
                            params![candidate],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
                    if clash.is_none() {
                        guard
                            .execute(
                                "UPDATE accounts SET pair_code = ?1 WHERE account_id = ?2",
                                params![candidate, account_id],
                            )
                            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
                        break;
                    }
                }
            }
            return Self::load_cosanctuary_status_locked(&guard, account_id)
                .map_err(ControlPlaneError::Internal);
        }

        let partner_row: Option<(String, String)> = guard
            .query_row(
                "SELECT account_id, partner_account_id FROM accounts WHERE pair_code = ?1 AND pair_code != ''",
                params![trimmed_code],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;

        let Some((partner_id, partner_existing_partner)) = partner_row else {
            return Err(ControlPlaneError::BadRequest(
                "invalid_pair_code",
                "配对码无效或不存在".to_owned(),
            ));
        };
        if partner_id == account_id {
            return Err(ControlPlaneError::BadRequest(
                "cannot_pair_self",
                "不能与自己配对".to_owned(),
            ));
        }

        let (self_tier, _) = fetch_effective_account_tier_locked(&guard, account_id, now_epoch)
            .map_err(ControlPlaneError::Internal)?;
        let (partner_tier, _) =
            fetch_effective_account_tier_locked(&guard, &partner_id, now_epoch)
                .map_err(ControlPlaneError::Internal)?;
        let host_active = self_tier
            .default_features()
            .iter()
            .any(|f| f == "co_sanctuary_host")
            || partner_tier
                .default_features()
                .iter()
                .any(|f| f == "co_sanctuary_host");
        if !host_active {
            return Err(ControlPlaneError::Forbidden(
                "cosanctuary_requires_active_host",
                "同栖空间需要配对双方中至少一方持有有效付费会员".to_owned(),
            ));
        }

        let self_existing_partner: String = guard
            .query_row(
                "SELECT partner_account_id FROM accounts WHERE account_id = ?1",
                params![account_id],
                |r| r.get(0),
            )
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        if !self_existing_partner.is_empty() && self_existing_partner != partner_id {
            return Err(ControlPlaneError::BadRequest(
                "already_paired_with_other",
                "当前账号已与其他伙伴配对".to_owned(),
            ));
        }
        if !partner_existing_partner.is_empty() && partner_existing_partner != account_id {
            return Err(ControlPlaneError::BadRequest(
                "pair_code_already_bound",
                "该配对码已与其他伙伴绑定".to_owned(),
            ));
        }

        let tx = guard
            .transaction()
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        tx.execute(
            "UPDATE accounts SET partner_account_id = ?1, pair_code = ?2 WHERE account_id = ?3",
            params![account_id, trimmed_code, partner_id],
        )
        .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        tx.execute(
            "UPDATE accounts SET partner_account_id = ?1, pair_code = ?2 WHERE account_id = ?3",
            params![partner_id, trimmed_code, account_id],
        )
        .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        tx.commit()
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;

        Self::load_cosanctuary_status_locked(&guard, account_id)
            .map_err(ControlPlaneError::Internal)
    }

    pub fn cosanctuary_set_mood(
        &self,
        account_id: &str,
        mood: i64,
        outfit_id: Option<&str>,
    ) -> Result<CoSanctuaryStatus, ControlPlaneError> {
        if !(0..=2).contains(&mood) {
            return Err(ControlPlaneError::BadRequest(
                "invalid_mood",
                "mood 必须为 0 (SUNNY_PRIME)、1 (BREEZY_STEADY) 或 2 (COZY_RECOVER)".to_owned(),
            ));
        }
        let now = Utc::now().timestamp();
        let guard = self
            .conn
            .lock()
            .map_err(|_| ControlPlaneError::Internal("DB 锁异常".to_owned()))?;
        let partner_id: String = guard
            .query_row(
                "SELECT partner_account_id FROM accounts WHERE account_id = ?1",
                params![account_id],
                |r| r.get(0),
            )
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        let (self_tier, _) = fetch_effective_account_tier_locked(&guard, account_id, now)
            .map_err(ControlPlaneError::Internal)?;
        let partner_active = if !partner_id.trim().is_empty() {
            let (pt, _) = fetch_effective_account_tier_locked(&guard, &partner_id, now)
                .map_err(ControlPlaneError::Internal)?;
            pt.default_features()
                .iter()
                .any(|f| f == "co_sanctuary_host")
        } else {
            false
        };
        let host_active = self_tier
            .default_features()
            .iter()
            .any(|f| f == "co_sanctuary_host")
            || partner_active;
        if !host_active {
            return Err(ControlPlaneError::Forbidden(
                "cosanctuary_requires_active_host",
                "同栖空间状态同步需要自己或已配对伙伴持有有效付费会员".to_owned(),
            ));
        }

        if let Some(raw_outfit) = outfit_id {
            let clean = raw_outfit.trim();
            if clean.is_empty()
                || clean.len() > 32
                || !clean
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            {
                return Err(ControlPlaneError::BadRequest(
                    "invalid_outfit_id",
                    "outfit_id 格式非法".to_owned(),
                ));
            }
            guard
                .execute(
                    "UPDATE accounts SET co_mood = ?1, co_outfit_id = ?2, co_updated_at = ?3 WHERE account_id = ?4",
                    params![mood, clean, now, account_id],
                )
                .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        } else {
            guard
                .execute(
                    "UPDATE accounts SET co_mood = ?1, co_updated_at = ?2 WHERE account_id = ?3",
                    params![mood, now, account_id],
                )
                .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        }
        Self::load_cosanctuary_status_locked(&guard, account_id)
            .map_err(ControlPlaneError::Internal)
    }

    pub fn cosanctuary_send_care(
        &self,
        account_id: &str,
        care_action: &str,
    ) -> Result<CoSanctuaryStatus, ControlPlaneError> {
        let clean_action = care_action.trim();
        if !matches!(clean_action, "warm_tea" | "tuck_blanket" | "paw_stamp") {
            return Err(ControlPlaneError::BadRequest(
                "invalid_care_action",
                "care_action 仅支持 warm_tea / tuck_blanket / paw_stamp".to_owned(),
            ));
        }
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| ControlPlaneError::Internal("DB 锁异常".to_owned()))?;
        let partner_id: String = guard
            .query_row(
                "SELECT partner_account_id FROM accounts WHERE account_id = ?1",
                params![account_id],
                |r| r.get(0),
            )
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        if partner_id.trim().is_empty() {
            return Err(ControlPlaneError::BadRequest(
                "cosanctuary_not_paired",
                "尚未与伙伴配对，无法投递同栖关照".to_owned(),
            ));
        }

        let now = Utc::now().timestamp();
        let (self_tier, _) = fetch_effective_account_tier_locked(&guard, account_id, now)
            .map_err(ControlPlaneError::Internal)?;
        let (partner_tier, _) = fetch_effective_account_tier_locked(&guard, &partner_id, now)
            .map_err(ControlPlaneError::Internal)?;
        let host_active = self_tier
            .default_features()
            .iter()
            .any(|f| f == "co_sanctuary_host")
            || partner_tier
                .default_features()
                .iter()
                .any(|f| f == "co_sanctuary_host");
        if !host_active {
            return Err(ControlPlaneError::Forbidden(
                "cosanctuary_requires_active_host",
                "投递同栖关照需要自己或伙伴持有有效付费会员".to_owned(),
            ));
        }

        let tx = guard
            .transaction()
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        let updated = tx
            .execute(
                "UPDATE accounts SET co_care_action = ?1, co_updated_at = ?2, bonsai_dew = bonsai_dew + 1 WHERE account_id = ?3",
                params![clean_action, now, partner_id],
            )
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        if updated == 0 {
            return Err(ControlPlaneError::BadRequest(
                "partner_not_found",
                "伙伴账号不存在或已注销".to_owned(),
            ));
        }
        tx.execute(
            "UPDATE accounts SET bonsai_dew = bonsai_dew + 1 WHERE account_id = ?1",
            params![account_id],
        )
        .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        tx.commit()
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;

        Self::load_cosanctuary_status_locked(&guard, account_id)
            .map_err(ControlPlaneError::Internal)
    }

    pub fn cosanctuary_status(&self, account_id: &str) -> Result<CoSanctuaryStatus, String> {
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        Self::load_cosanctuary_status_locked(&guard, account_id)
    }

    /// 原子扣减当月 ZDR 额度；Free / 已过期账号超限或调用 atelier_synthesis 返回 Ok(false)
    pub fn try_consume_zdr_quota(&self, id: &AccountIdentity, task: &str) -> Result<bool, String> {
        let ym = current_year_month();
        let now_epoch = Utc::now().timestamp();
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let (effective_tier, _) =
            fetch_effective_account_tier_locked(&guard, &id.account_id, now_epoch)?;
        let is_active_paid = effective_tier.is_paid();

        if task == "atelier_synthesis" && !is_active_paid {
            return Ok(false);
        }

        guard
            .execute(
                "INSERT OR IGNORE INTO monthly_quotas (account_id, year_month, meal_scans, lab_parses, briefings)
                 VALUES (?1, ?2, 0, 0, 0)",
                params![id.account_id, ym],
            )
            .map_err(|e| e.to_string())?;

        let (meal, lab, brief): (i64, i64, i64) = guard
            .query_row(
                "SELECT meal_scans, lab_parses, briefings FROM monthly_quotas
                 WHERE account_id = ?1 AND year_month = ?2",
                params![id.account_id, ym],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|e| e.to_string())?;

        if !is_active_paid {
            let allowed = match task {
                "intake" => meal < FREE_LIMIT_MEAL_SCANS,
                "lab" => lab < FREE_LIMIT_LAB_PARSES,
                _ => brief < FREE_LIMIT_BRIEFINGS,
            };
            if !allowed {
                return Ok(false);
            }
        }

        let sql = match task {
            "intake" => {
                "UPDATE monthly_quotas SET meal_scans = meal_scans + 1 WHERE account_id = ?1 AND year_month = ?2"
            }
            "lab" => {
                "UPDATE monthly_quotas SET lab_parses = lab_parses + 1 WHERE account_id = ?1 AND year_month = ?2"
            }
            _ => {
                "UPDATE monthly_quotas SET briefings = briefings + 1 WHERE account_id = ?1 AND year_month = ?2"
            }
        };
        guard
            .execute(sql, params![id.account_id, ym])
            .map_err(|e| e.to_string())?;
        Ok(true)
    }

    /// 推送 E2EE 密文增量信封（强制校准云同步权益、拒绝明文与超限大包）
    pub fn push_sync_envelope(
        &self,
        account_id: &str,
        env: &SyncEnvelopeInput,
    ) -> Result<i64, ControlPlaneError> {
        let now_epoch = Utc::now().timestamp();
        let guard = self
            .conn
            .lock()
            .map_err(|_| ControlPlaneError::Internal("DB 锁异常".to_owned()))?;
        let (effective_tier, _) =
            fetch_effective_account_tier_locked(&guard, account_id, now_epoch)
                .map_err(ControlPlaneError::Internal)?;
        if !effective_tier
            .default_features()
            .iter()
            .any(|f| f == "cloud_e2ee_sync")
        {
            return Err(ControlPlaneError::Forbidden(
                "cloud_e2ee_sync_requires_subscription",
                "云端 E2EE 多端同步仅对有效云端订阅开放（Free 与 LifetimeLocal 不包含云存储配额）"
                    .to_owned(),
            ));
        }

        if env.cipher != "XChaCha20-Poly1305" {
            return Err(ControlPlaneError::BadRequest(
                "invalid_e2ee_cipher",
                "拒绝接收：仅接受 XChaCha20-Poly1305 端到端加密分片".to_owned(),
            ));
        }
        if !env.path.starts_with("ops/") {
            return Err(ControlPlaneError::BadRequest(
                "invalid_sync_path",
                "分片路径必须位于设备隔离前缀 ops/ 下".to_owned(),
            ));
        }
        if env.payload_bytes > MAX_SYNC_ENVELOPE_BYTES
            || env.ciphertext_hex.len() > MAX_SYNC_CIPHERTEXT_HEX_LEN
        {
            return Err(ControlPlaneError::PayloadTooLarge(
                "sync_envelope_too_large",
                format!(
                    "单个 E2EE 分片不得超过 {} 字节（256 KiB）",
                    MAX_SYNC_ENVELOPE_BYTES
                ),
            ));
        }
        let nonce = hex::decode(&env.nonce_hex).map_err(|_| {
            ControlPlaneError::BadRequest("invalid_nonce_hex", "nonce_hex 非法".to_owned())
        })?;
        if nonce.len() != 24 {
            return Err(ControlPlaneError::BadRequest(
                "invalid_nonce_length",
                "XChaCha20 nonce 长度必须为 24 字节".to_owned(),
            ));
        }
        let cipher_bytes = hex::decode(&env.ciphertext_hex).map_err(|_| {
            ControlPlaneError::BadRequest(
                "invalid_ciphertext_hex",
                "ciphertext_hex 非法".to_owned(),
            )
        })?;
        if cipher_bytes.is_empty() {
            return Err(ControlPlaneError::BadRequest(
                "empty_ciphertext",
                "密文不能为空".to_owned(),
            ));
        }
        if cipher_bytes.len() > MAX_SYNC_ENVELOPE_BYTES {
            return Err(ControlPlaneError::PayloadTooLarge(
                "sync_envelope_too_large",
                format!(
                    "解码后密文不得超过 {} 字节（256 KiB）",
                    MAX_SYNC_ENVELOPE_BYTES
                ),
            ));
        }

        let existing_total_bytes: i64 = guard
            .query_row(
                "SELECT COALESCE(SUM(MAX(payload_bytes, LENGTH(ciphertext_hex) / 2)), 0)
                 FROM e2ee_sync_envelopes WHERE account_id = ?1",
                params![account_id],
                |r| r.get(0),
            )
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        let new_bytes = env.payload_bytes.max(cipher_bytes.len());
        if existing_total_bytes
            .max(0)
            .saturating_add(new_bytes as i64)
            > MAX_ACCOUNT_SYNC_TOTAL_BYTES as i64
        {
            return Err(ControlPlaneError::PayloadTooLarge(
                "sync_account_quota_exceeded",
                format!(
                    "账号 E2EE 云同步总容量已达上限 {} 字节（32 MiB）",
                    MAX_ACCOUNT_SYNC_TOTAL_BYTES
                ),
            ));
        }

        let now = Utc::now().to_rfc3339();
        guard
            .execute(
                "INSERT INTO e2ee_sync_envelopes
                 (account_id, device_id, batch_seq, path, protocol, cipher, nonce_hex, ciphertext_hex, payload_bytes, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(account_id, device_id, batch_seq) DO NOTHING",
                params![
                    account_id,
                    env.device_id,
                    env.batch_seq as i64,
                    env.path,
                    env.protocol,
                    env.cipher,
                    env.nonce_hex,
                    env.ciphertext_hex,
                    env.payload_bytes as i64,
                    now
                ],
            )
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;

        let row_id: i64 = guard
            .query_row(
                "SELECT id FROM e2ee_sync_envelopes
                 WHERE account_id = ?1 AND device_id = ?2 AND batch_seq = ?3",
                params![account_id, env.device_id, env.batch_seq as i64],
                |r| r.get(0),
            )
            .map_err(|e| ControlPlaneError::Internal(e.to_string()))?;
        Ok(row_id)
    }

    pub fn pull_sync_envelopes(
        &self,
        account_id: &str,
        after_id: i64,
        exclude_device: Option<&str>,
    ) -> Result<(Vec<StoredSyncEnvelope>, i64), String> {
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let mut stmt = guard
            .prepare(
                "SELECT id, device_id, batch_seq, path, protocol, cipher, nonce_hex, ciphertext_hex, payload_bytes, created_at
                 FROM e2ee_sync_envelopes
                 WHERE account_id = ?1 AND id > ?2
                 ORDER BY id ASC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![account_id, after_id], |r| {
                Ok(StoredSyncEnvelope {
                    id: r.get(0)?,
                    device_id: r.get(1)?,
                    batch_seq: r.get::<_, i64>(2)? as u64,
                    path: r.get(3)?,
                    protocol: r.get(4)?,
                    cipher: r.get(5)?,
                    nonce_hex: r.get(6)?,
                    ciphertext_hex: r.get(7)?,
                    payload_bytes: r.get::<_, i64>(8)? as usize,
                    created_at: r.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let latest_id = rows.last().map(|r| r.id).unwrap_or(after_id);
        let filtered = match exclude_device {
            Some(ex) if !ex.trim().is_empty() => rows
                .into_iter()
                .filter(|r| r.device_id != ex)
                .collect(),
            _ => rows,
        };
        Ok((filtered, latest_id))
    }

    /// 校验 StoreKit 2 JWS / Sandbox 交易凭据并幂等落库（支持订阅签发 OSET 与窗台添柴礼赠券席位累加）
    pub fn verify_apple_purchase(
        &self,
        identity: &AccountIdentity,
        req: &AppleVerifyRequest,
    ) -> Result<AppleVerifyResponse, AppleVerifyError> {
        let now_epoch = Utc::now().timestamp();
        let verified = parse_and_verify_apple_payload(
            req,
            now_epoch,
            self.production_mode,
            self.test_apple_root_der.as_deref(),
        )?;
        let sk = self.signing_key();

        let mut guard = self
            .conn
            .lock()
            .map_err(|_| AppleVerifyError::Internal("DB 锁异常".to_owned()))?;
        let tx = guard
            .transaction()
            .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;

        let existing_tx: Option<(String, String, i64, String, i64)> = tx
            .query_row(
                "SELECT account_id, product_id, seats_credited, community_gift_code, expires_epoch
                 FROM apple_iap_transactions WHERE transaction_id = ?1",
                params![verified.transaction_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()
            .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;

        if let Some((owner_id, existing_pid, _existing_seats, existing_code, existing_exp)) = existing_tx {
            if owner_id != identity.account_id {
                return Err(AppleVerifyError::Conflict(
                    "transaction_owned_by_another_account",
                    "该 Apple 交易凭据已绑定其他账号".to_owned(),
                ));
            }
            if existing_pid != verified.spec.product_id {
                return Err(AppleVerifyError::BadRequest(
                    "transaction_product_mismatch",
                    "同一 transaction_id 不可变更为其他商品".to_owned(),
                ));
            }

            // 恢复购买幂等重放：如为订阅/买断且原账户处于 free 或更早到期，确保状态同步但不重复入账
            match verified.spec.kind {
                ApplePurchaseKind::Subscription(tier)
                | ApplePurchaseKind::NonConsumable(tier) => {
                    let effective_exp = existing_exp.max(verified.expires_epoch);
                    let current_row: Option<(String, i64)> = tx
                        .query_row(
                            "SELECT plan_tier, entitlement_expires_epoch FROM accounts WHERE account_id = ?1",
                            params![identity.account_id],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .optional()
                        .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;
                    if let Some((cur_tier, cur_exp)) = current_row {
                        if cur_tier == "free" || cur_exp < effective_exp {
                            tx.execute(
                                "UPDATE accounts SET plan_tier = ?1, entitlement_expires_epoch = ?2 WHERE account_id = ?3",
                                params![tier.as_str(), effective_exp, identity.account_id],
                            )
                            .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;
                        }
                    }
                }
                ApplePurchaseKind::ConsumableHearthGiftPass => {}
            }

            tx.commit()
                .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;
            let ent = Self::load_entitlements_locked(&guard, identity, &sk)
                .map_err(AppleVerifyError::Internal)?;
            let guardian_receipt_token =
                if matches!(verified.spec.kind, ApplePurchaseKind::ConsumableHearthGiftPass) {
                    Some(
                        mint_oset_token_with_key(
                            &identity.account_id,
                            SubscriptionTier::Free,
                            now_epoch,
                            0,
                            true,
                            ent.companion_loyalty_days,
                            &sk,
                        )
                        .map_err(AppleVerifyError::Internal)?,
                    )
                } else {
                    None
                };
            let code_opt = if existing_code.is_empty() {
                None
            } else {
                Some(existing_code)
            };
            return Ok(AppleVerifyResponse {
                verified: true,
                verification_mode: verified.verification_mode,
                environment: verified.environment,
                product_id: verified.spec.product_id.to_owned(),
                purchase_type: verified.spec.purchase_type.to_owned(),
                product_display_zh: verified.spec.display_zh.to_owned(),
                product_display_en: verified.spec.display_en.to_owned(),
                transaction_id: verified.transaction_id,
                original_transaction_id: verified.original_transaction_id,
                idempotent_replay: true,
                entitlement_token: ent.entitlement_token.clone(),
                guardian_receipt_token,
                plan_tier: ent.plan_tier.clone(),
                seats_credited: 0,
                hearth_gift_credits_total: ent.hearth_gift_credits,
                hearth_gift_pass_title_zh: HEARTH_GIFT_PASS_TITLE_ZH.to_owned(),
                hearth_gift_pass_title_en: HEARTH_GIFT_PASS_TITLE_EN.to_owned(),
                community_gift_code: code_opt,
                entitlements: ent,
            });
        }

        // 校验 original_transaction_id 归属权（防止跨账号重放订阅线）
        let other_owner: Option<String> = tx
            .query_row(
                "SELECT account_id FROM apple_iap_transactions
                 WHERE original_transaction_id = ?1 AND account_id != ?2
                 LIMIT 1",
                params![verified.original_transaction_id, identity.account_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;
        if other_owner.is_some() {
            return Err(AppleVerifyError::Conflict(
                "transaction_owned_by_another_account",
                "该 Apple 原始交易链已绑定其他账号".to_owned(),
            ));
        }

        let now_rfc3339 = Utc::now().to_rfc3339();
        let (tier_granted, seats_credited, community_gift_code) = match verified.spec.kind {
            ApplePurchaseKind::Subscription(tier) | ApplePurchaseKind::NonConsumable(tier) => {
                let (cur_tier, cur_exp): (String, i64) = tx
                    .query_row(
                        "SELECT plan_tier, entitlement_expires_epoch FROM accounts WHERE account_id = ?1",
                        params![identity.account_id],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;
                let next_exp = if cur_tier == tier.as_str() {
                    cur_exp.max(verified.expires_epoch)
                } else {
                    verified.expires_epoch
                };
                tx.execute(
                    "UPDATE accounts SET plan_tier = ?1, entitlement_expires_epoch = ?2 WHERE account_id = ?3",
                    params![tier.as_str(), next_exp, identity.account_id],
                )
                .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;
                (tier.as_str().to_owned(), 0i64, String::new())
            }
            ApplePurchaseKind::ConsumableHearthGiftPass => {
                let code = format!(
                    "QI-SEAT-{}",
                    &verified.jws_sha256[..10].to_ascii_uppercase()
                );
                tx.execute(
                    "UPDATE accounts SET hearth_gift_credits = hearth_gift_credits + ?1, guardian_sponsor = 1 WHERE account_id = ?2",
                    params![verified.quantity, identity.account_id],
                )
                .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;
                (
                    "hearth_gift_pass".to_owned(),
                    verified.quantity,
                    code,
                )
            }
        };

        if !self.production_mode {
            if let Some(days) = req.companion_loyalty_days {
                tx.execute(
                    "UPDATE accounts SET companion_loyalty_days = ?1 WHERE account_id = ?2",
                    params![days.max(0), identity.account_id],
                )
                .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;
            }
        }

        tx.execute(
            "INSERT INTO apple_iap_transactions
             (transaction_id, original_transaction_id, account_id, product_id, purchase_type,
              tier_granted, environment, seats_credited, community_gift_code, expires_epoch,
              jws_sha256, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                verified.transaction_id,
                verified.original_transaction_id,
                identity.account_id,
                verified.spec.product_id,
                verified.spec.purchase_type,
                tier_granted,
                verified.environment,
                seats_credited,
                community_gift_code,
                verified.expires_epoch,
                verified.jws_sha256,
                now_rfc3339,
            ],
        )
        .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;

        tx.commit()
            .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;

        let ent = Self::load_entitlements_locked(&guard, identity, &sk)
            .map_err(AppleVerifyError::Internal)?;
        let guardian_receipt_token =
            if matches!(verified.spec.kind, ApplePurchaseKind::ConsumableHearthGiftPass) {
                Some(
                    mint_oset_token_with_key(
                        &identity.account_id,
                        SubscriptionTier::Free,
                        now_epoch,
                        0,
                        true,
                        ent.companion_loyalty_days,
                        &sk,
                    )
                    .map_err(AppleVerifyError::Internal)?,
                )
            } else {
                None
            };
        let code_opt = if community_gift_code.is_empty() {
            None
        } else {
            Some(community_gift_code)
        };
        Ok(AppleVerifyResponse {
            verified: true,
            verification_mode: verified.verification_mode,
            environment: verified.environment,
            product_id: verified.spec.product_id.to_owned(),
            purchase_type: verified.spec.purchase_type.to_owned(),
            product_display_zh: verified.spec.display_zh.to_owned(),
            product_display_en: verified.spec.display_en.to_owned(),
            transaction_id: verified.transaction_id,
            original_transaction_id: verified.original_transaction_id,
            idempotent_replay: false,
            entitlement_token: ent.entitlement_token.clone(),
            guardian_receipt_token,
            plan_tier: ent.plan_tier.clone(),
            seats_credited,
            hearth_gift_credits_total: ent.hearth_gift_credits,
            hearth_gift_pass_title_zh: HEARTH_GIFT_PASS_TITLE_ZH.to_owned(),
            hearth_gift_pass_title_en: HEARTH_GIFT_PASS_TITLE_EN.to_owned(),
            community_gift_code: code_opt,
            entitlements: ent,
        })
    }

    pub fn count_apple_transactions_for_account(&self, account_id: &str) -> Result<usize, String> {
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let count: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM apple_iap_transactions WHERE account_id = ?1",
                params![account_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(count as usize)
    }

    /// Apple 5.1.1(v) 账号与全量密文分片事务性物理销毁
    pub fn delete_account_physical(&self, account_id: &str) -> Result<DeletionReceipt, String> {
        let mut guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let tx = guard.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE accounts SET partner_account_id = '' WHERE partner_account_id = ?1",
            params![account_id],
        )
        .map_err(|e| e.to_string())?;
        let deleted_apple_transactions = tx
            .execute(
                "DELETE FROM apple_iap_transactions WHERE account_id = ?1",
                params![account_id],
            )
            .map_err(|e| e.to_string())?;
        let deleted_e2ee_envelopes = tx
            .execute(
                "DELETE FROM e2ee_sync_envelopes WHERE account_id = ?1",
                params![account_id],
            )
            .map_err(|e| e.to_string())?;
        let deleted_quota_rows = tx
            .execute(
                "DELETE FROM monthly_quotas WHERE account_id = ?1",
                params![account_id],
            )
            .map_err(|e| e.to_string())?;
        let deleted_accounts = tx
            .execute(
                "DELETE FROM accounts WHERE account_id = ?1",
                params![account_id],
            )
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;

        Ok(DeletionReceipt {
            account_id: account_id.to_owned(),
            deleted_accounts,
            deleted_quota_rows,
            deleted_e2ee_envelopes,
            deleted_apple_transactions,
            wiped_at: Utc::now().to_rfc3339(),
        })
    }

    /// 供审计测试读取数据库表清单与密文表行数，验证无健康明文表
    pub fn table_names(&self) -> Result<Vec<String>, String> {
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let mut stmt = guard
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .map_err(|e| e.to_string())?;
        let names = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(names)
    }
}

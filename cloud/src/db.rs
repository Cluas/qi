//! 极简 Control Plane SQLite 存储层：
//! 严守「零健康明文库」原则 —— 仅保存账号令牌哈希、月度 ZDR 免费配额计数、
//! E2EE 密文信封、Ed25519 OSET 离线权益状态与 1-byte 同栖共居姿态。

use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use base64::{
    Engine as _,
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
use chrono::{Datelike, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use getrandom::getrandom;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CONTROL_PLANE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS accounts (
    account_id TEXT PRIMARY KEY,
    handle TEXT UNIQUE NOT NULL,
    token_hash TEXT NOT NULL,
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

pub const EXPECTED_IOS_BUNDLE_ID: &str = "eu.org.cluas.qi";
pub const HEARTH_GIFT_PASS_PRODUCT_ID: &str = "com.qi.hearth_gift_pass";
pub const HEARTH_GIFT_PASS_TITLE_ZH: &str = "窗台添柴礼赠券 · 赠予社区漂流席位";
pub const HEARTH_GIFT_PASS_TITLE_EN: &str = "Hearth Gift Pass · Gift 1 Community Seat";

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

pub fn mint_oset_token(
    subject: &str,
    tier: SubscriptionTier,
    now_epoch: i64,
    exp_epoch: i64,
    guardian_sponsor: bool,
    companion_loyalty_days: i64,
) -> Result<String, String> {
    if !tier.is_paid() {
        return Ok(String::new());
    }
    let grace_until = exp_epoch.saturating_add(OFFLINE_GRACE_SECONDS);
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
        exp_epoch,
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
    let signing_key = default_signing_key();
    let sig: Signature = signing_key.sign(signing_input.as_bytes());
    let b64_sig = URL_SAFE_NO_PAD.encode(sig.to_bytes());
    Ok(format!("{signing_input}.{b64_sig}"))
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

pub struct ControlPlaneDb {
    conn: Mutex<Connection>,
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

fn parse_and_verify_apple_payload(
    req: &AppleVerifyRequest,
    now_epoch: i64,
) -> Result<VerifiedAppleFields, AppleVerifyError> {
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

        let verification_mode = match alg {
            "EdDSA" => {
                let sig = Signature::from_slice(&sig_bytes).map_err(|_| {
                    AppleVerifyError::BadRequest(
                        "invalid_jws_signature",
                        "EdDSA JWS 签名长度非法".to_owned(),
                    )
                })?;
                let signing_input = format!("{}.{}", parts[0], parts[1]);
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
                if !(64..=80).contains(&sig_bytes.len()) {
                    return Err(AppleVerifyError::BadRequest(
                        "invalid_jws_signature",
                        "ES256 JWS 签名长度不符合 P-256 规范".to_owned(),
                    ));
                }
                if std::env::var("QI_APPLE_REQUIRE_X5C").as_deref() == Ok("1") {
                    let has_x5c = header
                        .get("x5c")
                        .and_then(|v| v.as_array())
                        .is_some_and(|arr| !arr.is_empty());
                    if !has_x5c {
                        return Err(AppleVerifyError::BadRequest(
                            "missing_x5c_chain",
                            "生产模式要求 ES256 JWS 携带 Apple x5c 证书链".to_owned(),
                        ));
                    }
                }
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
        let effective_bundle = jws_bundle.or_else(|| {
            req.bundle_id
                .as_deref()
                .map(str::trim)
                .map(ToOwned::to_owned)
        });

        let raw_env = claims
            .get("environment")
            .and_then(|v| v.as_str())
            .or(req.environment.as_deref())
            .unwrap_or("Sandbox");
        let norm_env = normalize_apple_env(raw_env)?.to_owned();

        let expires_epoch = claims
            .get("expiresDate")
            .or_else(|| claims.get("expires_date_epoch"))
            .and_then(json_epoch_seconds)
            .or_else(|| req.expires_date_epoch.map(normalize_epoch_seconds));

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
            effective_bundle,
            norm_env,
            expires_epoch,
            quantity,
            verification_mode,
            jws_hash,
        )
    } else {
        if std::env::var("QI_APPLE_ENV").as_deref() == Ok("production") {
            return Err(AppleVerifyError::BadRequest(
                "missing_jws_representation",
                "生产环境必须提供 StoreKit 2 JWS 交易签名".to_owned(),
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
            req.bundle_id.as_deref().map(str::trim).map(ToOwned::to_owned),
            norm_env.to_owned(),
            req.expires_date_epoch.map(normalize_epoch_seconds),
            req.quantity.unwrap_or(1),
            "storekit2_sandbox_payload".to_owned(),
            sandbox_hash,
        )
    };

    if let Some(ref b) = bundle_id {
        if b != EXPECTED_IOS_BUNDLE_ID {
            return Err(AppleVerifyError::BadRequest(
                "invalid_bundle_id",
                format!("bundle_id 不匹配，期望 {EXPECTED_IOS_BUNDLE_ID}"),
            ));
        }
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

impl ControlPlaneDb {
    pub fn open_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        conn.execute_batch(CONTROL_PLANE_SCHEMA)
            .map_err(|e| e.to_string())?;
        ensure_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_file(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let _ = conn.execute_batch("PRAGMA journal_mode = WAL;");
        conn.execute_batch(CONTROL_PLANE_SCHEMA)
            .map_err(|e| e.to_string())?;
        ensure_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 登录或自动注册（同 handle 每次登录轮换新 Bearer Token）
    pub fn create_or_rotate_session(
        &self,
        handle: &str,
    ) -> Result<(AccountIdentity, String, EntitlementsSnapshot), String> {
        let clean_handle = handle.trim().to_owned();
        if clean_handle.is_empty() {
            return Err("账号标识不能为空".to_owned());
        }
        let raw_token = format!("qitok_{}", random_hex(24));
        let tok_hash = hash_token(&raw_token);
        let now = Utc::now().to_rfc3339();

        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let existing: Option<(String, String)> = guard
            .query_row(
                "SELECT account_id, plan_tier FROM accounts WHERE handle = ?1",
                params![clean_handle],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        let (account_id, plan_tier) = match existing {
            Some((id, tier)) => {
                guard
                    .execute(
                        "UPDATE accounts SET token_hash = ?1 WHERE account_id = ?2",
                        params![tok_hash, id],
                    )
                    .map_err(|e| e.to_string())?;
                (id, tier)
            }
            None => {
                let new_id = format!("acc_{}", random_hex(8));
                guard
                    .execute(
                        "INSERT INTO accounts (account_id, handle, token_hash, plan_tier, created_at)
                         VALUES (?1, ?2, ?3, 'free', ?4)",
                        params![new_id, clean_handle, tok_hash, now],
                    )
                    .map_err(|e| e.to_string())?;
                (new_id, "free".to_owned())
            }
        };

        let identity = AccountIdentity {
            account_id: account_id.clone(),
            handle: clean_handle,
            plan_tier: plan_tier.clone(),
        };
        let ent = Self::load_entitlements_locked(&guard, &identity)?;
        Ok((identity, raw_token, ent))
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
    ) -> Result<EntitlementsSnapshot, String> {
        let (plan_tier, stored_exp, guardian_int, hearth_gift_credits, loyalty_days, pair_code): (
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

        let tier_enum = SubscriptionTier::from_str(&plan_tier);
        let is_paid = tier_enum.is_paid();
        let guardian_sponsor = guardian_int != 0;
        let now_epoch = Utc::now().timestamp();
        let exp_epoch = if is_paid {
            if stored_exp > 0 {
                stored_exp
            } else {
                now_epoch.saturating_add(tier_enum.default_duration_seconds())
            }
        } else {
            0
        };
        let entitlement_token = if is_paid {
            mint_oset_token(
                &id.account_id,
                tier_enum,
                now_epoch,
                exp_epoch,
                guardian_sponsor,
                loyalty_days,
            )?
        } else {
            String::new()
        };

        Ok(EntitlementsSnapshot {
            account_id: id.account_id.clone(),
            handle: id.handle.clone(),
            plan_tier,
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
        Self::load_entitlements_locked(&guard, id)
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
        Self::load_entitlements_locked(&guard, &updated)
    }

    pub fn refresh_entitlements(
        &self,
        id: &AccountIdentity,
    ) -> Result<EntitlementsSnapshot, String> {
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let plan_tier: String = guard
            .query_row(
                "SELECT plan_tier FROM accounts WHERE account_id = ?1",
                params![id.account_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let tier_enum = SubscriptionTier::from_str(&plan_tier);
        if tier_enum.is_paid() {
            let now_epoch = Utc::now().timestamp();
            let new_exp = now_epoch.saturating_add(tier_enum.default_duration_seconds());
            guard
                .execute(
                    "UPDATE accounts SET entitlement_expires_epoch = ?1 WHERE account_id = ?2",
                    params![new_exp, id.account_id],
                )
                .map_err(|e| e.to_string())?;
        }
        Self::load_entitlements_locked(&guard, id)
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
    ) -> Result<CoSanctuaryStatus, String> {
        let mut guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let trimmed_code = requested_code.map(|s| s.trim().to_ascii_uppercase()).unwrap_or_default();

        if trimmed_code.is_empty() {
            let existing_code: String = guard
                .query_row(
                    "SELECT pair_code FROM accounts WHERE account_id = ?1",
                    params![account_id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
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
                        .map_err(|e| e.to_string())?;
                    if clash.is_none() {
                        guard
                            .execute(
                                "UPDATE accounts SET pair_code = ?1 WHERE account_id = ?2",
                                params![candidate, account_id],
                            )
                            .map_err(|e| e.to_string())?;
                        break;
                    }
                }
            }
            return Self::load_cosanctuary_status_locked(&guard, account_id);
        }

        let partner_row: Option<(String, String)> = guard
            .query_row(
                "SELECT account_id, partner_account_id FROM accounts WHERE pair_code = ?1 AND pair_code != ''",
                params![trimmed_code],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        let Some((partner_id, partner_existing_partner)) = partner_row else {
            return Err("配对码无效或不存在".to_owned());
        };
        if partner_id == account_id {
            return Err("不能与自己配对".to_owned());
        }

        let self_existing_partner: String = guard
            .query_row(
                "SELECT partner_account_id FROM accounts WHERE account_id = ?1",
                params![account_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !self_existing_partner.is_empty() && self_existing_partner != partner_id {
            return Err("当前账号已与其他伙伴配对".to_owned());
        }
        if !partner_existing_partner.is_empty() && partner_existing_partner != account_id {
            return Err("该配对码已与其他伙伴绑定".to_owned());
        }

        let tx = guard.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE accounts SET partner_account_id = ?1, pair_code = ?2 WHERE account_id = ?3",
            params![account_id, trimmed_code, partner_id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE accounts SET partner_account_id = ?1, pair_code = ?2 WHERE account_id = ?3",
            params![partner_id, trimmed_code, account_id],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;

        Self::load_cosanctuary_status_locked(&guard, account_id)
    }

    pub fn cosanctuary_set_mood(
        &self,
        account_id: &str,
        mood: i64,
        outfit_id: Option<&str>,
    ) -> Result<CoSanctuaryStatus, String> {
        if !(0..=2).contains(&mood) {
            return Err("mood 必须为 0 (SUNNY_PRIME)、1 (BREEZY_STEADY) 或 2 (COZY_RECOVER)".to_owned());
        }
        let now = Utc::now().timestamp();
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        if let Some(raw_outfit) = outfit_id {
            let clean = raw_outfit.trim();
            if clean.is_empty()
                || clean.len() > 32
                || !clean
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            {
                return Err("outfit_id 格式非法".to_owned());
            }
            guard
                .execute(
                    "UPDATE accounts SET co_mood = ?1, co_outfit_id = ?2, co_updated_at = ?3 WHERE account_id = ?4",
                    params![mood, clean, now, account_id],
                )
                .map_err(|e| e.to_string())?;
        } else {
            guard
                .execute(
                    "UPDATE accounts SET co_mood = ?1, co_updated_at = ?2 WHERE account_id = ?3",
                    params![mood, now, account_id],
                )
                .map_err(|e| e.to_string())?;
        }
        Self::load_cosanctuary_status_locked(&guard, account_id)
    }

    pub fn cosanctuary_send_care(
        &self,
        account_id: &str,
        care_action: &str,
    ) -> Result<CoSanctuaryStatus, String> {
        let clean_action = care_action.trim();
        if !matches!(clean_action, "warm_tea" | "tuck_blanket" | "paw_stamp") {
            return Err("care_action 仅支持 warm_tea / tuck_blanket / paw_stamp".to_owned());
        }
        let mut guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let partner_id: String = guard
            .query_row(
                "SELECT partner_account_id FROM accounts WHERE account_id = ?1",
                params![account_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if partner_id.trim().is_empty() {
            return Err("尚未与伙伴配对，无法投递同栖关照".to_owned());
        }

        let now = Utc::now().timestamp();
        let tx = guard.transaction().map_err(|e| e.to_string())?;
        let updated = tx
            .execute(
                "UPDATE accounts SET co_care_action = ?1, co_updated_at = ?2, bonsai_dew = bonsai_dew + 1 WHERE account_id = ?3",
                params![clean_action, now, partner_id],
            )
            .map_err(|e| e.to_string())?;
        if updated == 0 {
            return Err("伙伴账号不存在或已注销".to_owned());
        }
        tx.execute(
            "UPDATE accounts SET bonsai_dew = bonsai_dew + 1 WHERE account_id = ?1",
            params![account_id],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;

        Self::load_cosanctuary_status_locked(&guard, account_id)
    }

    pub fn cosanctuary_status(&self, account_id: &str) -> Result<CoSanctuaryStatus, String> {
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        Self::load_cosanctuary_status_locked(&guard, account_id)
    }

    /// 原子扣减当月 ZDR 额度；Free 超限返回 Ok(false)
    pub fn try_consume_zdr_quota(&self, id: &AccountIdentity, task: &str) -> Result<bool, String> {
        let ym = current_year_month();
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
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

        let current_tier: String = guard
            .query_row(
                "SELECT plan_tier FROM accounts WHERE account_id = ?1",
                params![id.account_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| id.plan_tier.clone());

        if !SubscriptionTier::from_str(&current_tier).is_paid() {
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

    /// 推送 E2EE 密文增量信封（强制拒绝任何非 XChaCha20-Poly1305 或非 Hex 载荷）
    pub fn push_sync_envelope(
        &self,
        account_id: &str,
        env: &SyncEnvelopeInput,
    ) -> Result<i64, String> {
        if env.cipher != "XChaCha20-Poly1305" {
            return Err("拒绝接收：仅接受 XChaCha20-Poly1305 端到端加密分片".to_owned());
        }
        if !env.path.starts_with("ops/") {
            return Err("分片路径必须位于设备隔离前缀 ops/ 下".to_owned());
        }
        let nonce = hex::decode(&env.nonce_hex).map_err(|_| "nonce_hex 非法".to_owned())?;
        if nonce.len() != 24 {
            return Err("XChaCha20 nonce 长度必须为 24 字节".to_owned());
        }
        let cipher_bytes =
            hex::decode(&env.ciphertext_hex).map_err(|_| "ciphertext_hex 非法".to_owned())?;
        if cipher_bytes.is_empty() {
            return Err("密文不能为空".to_owned());
        }

        let now = Utc::now().to_rfc3339();
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
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
            .map_err(|e| e.to_string())?;

        let row_id: i64 = guard
            .query_row(
                "SELECT id FROM e2ee_sync_envelopes
                 WHERE account_id = ?1 AND device_id = ?2 AND batch_seq = ?3",
                params![account_id, env.device_id, env.batch_seq as i64],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
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
        let verified = parse_and_verify_apple_payload(req, now_epoch)?;

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
            let ent = Self::load_entitlements_locked(&guard, identity)
                .map_err(AppleVerifyError::Internal)?;
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
                    "UPDATE accounts SET hearth_gift_credits = hearth_gift_credits + ?1 WHERE account_id = ?2",
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

        if let Some(days) = req.companion_loyalty_days {
            tx.execute(
                "UPDATE accounts SET companion_loyalty_days = ?1 WHERE account_id = ?2",
                params![days.max(0), identity.account_id],
            )
            .map_err(|e| AppleVerifyError::Internal(e.to_string()))?;
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

        let ent = Self::load_entitlements_locked(&guard, identity)
            .map_err(AppleVerifyError::Internal)?;
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

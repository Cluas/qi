use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Development/sandbox Ed25519 seed used when `QI_ENTITLEMENT_PUBKEY_HEX` is not overridden.
/// In production cloud deployments, override with `QI_CLOUD_SIGNING_SEED_HEX` + `QI_ENTITLEMENT_PUBKEY_HEX`.
pub const DEFAULT_ATELIER_SIGNING_SEED: [u8; 32] = [
    0x51, 0x69, 0x41, 0x74, 0x65, 0x6c, 0x69, 0x65,
    0x72, 0x32, 0x30, 0x32, 0x36, 0x5f, 0x45, 0x64,
    0x32, 0x35, 0x35, 0x31, 0x39, 0x5f, 0x4f, 0x53,
    0x45, 0x54, 0x5f, 0x56, 0x34, 0x4b, 0x65, 0x79,
];

pub const OFFLINE_GRACE_SECONDS: i64 = 7 * 86_400;
pub const CLOCK_SKEW_TOLERANCE_SECONDS: i64 = 300;
pub const LOYALTY_MIN_CHECKIN_DAYS: i64 = 30;
pub const DEFAULT_LOCALE: &str = "zh-CN";
pub const FREE_CUSTOM_PET_QUOTA: usize = 1;
pub const PRO_CUSTOM_PET_QUOTA: usize = 3;
pub const HEARTH_GIFT_PASS_PRODUCT_ID: &str = "com.qi.hearth_gift_pass";

pub fn normalize_locale(raw: &str) -> &'static str {
    let trimmed = raw.trim().to_ascii_lowercase();
    if trimmed == "en" || trimmed.starts_with("en-") || trimmed.starts_with("en_") {
        "en"
    } else {
        "zh-CN"
    }
}

pub fn is_en_locale(locale: &str) -> bool {
    normalize_locale(locale) == "en"
}

pub fn tier_from_apple_product_id(product_id: &str) -> Option<SubscriptionTier> {
    let lower = product_id.trim().to_ascii_lowercase();
    if lower.is_empty() || lower.contains("hearth_gift_pass") || lower.contains("gift_pass") {
        return None;
    }
    if lower.contains("lifetime") {
        Some(SubscriptionTier::LifetimeLocal)
    } else if lower.contains("family") {
        Some(SubscriptionTier::Family)
    } else if lower.contains("duo") {
        Some(SubscriptionTier::Duo)
    } else if lower.contains("annual") || lower.contains("yearly") {
        Some(SubscriptionTier::ProAnnual)
    } else if lower.contains("seasonal") || lower.contains("season") {
        Some(SubscriptionTier::Seasonal)
    } else if lower.contains("monthly") || lower.contains("month") {
        Some(SubscriptionTier::ProMonthly)
    } else {
        None
    }
}

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
        match raw.trim().to_ascii_lowercase().as_str() {
            "pro_monthly" | "monthly" => Self::ProMonthly,
            "seasonal" | "solar_season" => Self::Seasonal,
            "pro_annual" | "annual" | "pro" => Self::ProAnnual,
            "duo" => Self::Duo,
            "family" => Self::Family,
            "lifetime_local" | "lifetime" => Self::LifetimeLocal,
            _ => Self::Free,
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
        self.display_name_localized(DEFAULT_LOCALE)
    }

    pub fn display_name_localized(&self, locale: &str) -> &'static str {
        if is_en_locale(locale) {
            match self {
                Self::Free => "Qi Free",
                Self::ProMonthly => "Qi Atelier Monthly",
                Self::Seasonal => "Solar Season Pass",
                Self::ProAnnual => "Qi Atelier Annual",
                Self::Duo => "Co-Window Duo",
                Self::Family => "Family Peace Letters",
                Self::LifetimeLocal => "Lifetime Local",
            }
        } else {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineEntitlementState {
    FreeLocal,
    ActiveAtelier,
    OfflineGrace,
    ExpiredReadOnlyVault,
}

impl OfflineEntitlementState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FreeLocal => "free_local",
            Self::ActiveAtelier => "active_atelier",
            Self::OfflineGrace => "offline_grace",
            Self::ExpiredReadOnlyVault => "expired_read_only_vault",
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementVerificationResult {
    pub valid_signature: bool,
    pub tier: String,
    pub tier_display: String,
    pub state: OfflineEntitlementState,
    pub is_atelier_unlocked: bool,
    pub seats: u8,
    pub guardian_sponsor: bool,
    pub companion_loyalty_eligible: bool,
    pub exp_epoch: i64,
    pub grace_until: i64,
    pub grace_days_remaining: i64,
    pub clock_rollback_detected: bool,
    pub effective_now_epoch: i64,
    pub new_max_seen_epoch: i64,
    pub features: Vec<String>,
    pub can_read_history_vault: bool,
    pub can_export_vault: bool,
    pub status_banner: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingPlanDescriptor {
    pub tier: String,
    pub title: String,
    pub subtitle: String,
    pub price_cny: i64,
    pub original_price_cny: Option<i64>,
    pub billing_cycle: String,
    pub seats: u8,
    pub highlights: Vec<String>,
    pub badge: String,
    pub loyalty_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialTimelineStep {
    pub day_label: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementCatalog {
    pub plans: Vec<PricingPlanDescriptor>,
    pub guardian_addon_title: String,
    pub guardian_addon_price_cny: i64,
    pub guardian_addon_desc: String,
    pub transparent_timeline: Vec<TrialTimelineStep>,
    pub ethical_promises: Vec<String>,
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

pub fn sign_oset_with_key(claims: &OsetClaims, signing_key: &SigningKey) -> Result<String, String> {
    let json = serde_json::to_vec(claims).map_err(|e| format!("claims serialize failed: {e}"))?;
    let b64_claims = URL_SAFE_NO_PAD.encode(json);
    let signing_input = format!("qient1.{b64_claims}");
    let sig: Signature = signing_key.sign(signing_input.as_bytes());
    let b64_sig = URL_SAFE_NO_PAD.encode(sig.to_bytes());
    Ok(format!("{signing_input}.{b64_sig}"))
}

pub fn sign_oset(claims: &OsetClaims) -> Result<String, String> {
    sign_oset_with_key(claims, &default_signing_key())
}

pub fn mint_oset_for_tier(
    subject: &str,
    tier: SubscriptionTier,
    now_epoch: i64,
    guardian_sponsor: bool,
    companion_loyalty_eligible: bool,
) -> Result<(OsetClaims, String), String> {
    let duration = tier.default_duration_seconds();
    let exp_epoch = if tier == SubscriptionTier::Free {
        now_epoch
    } else {
        now_epoch.saturating_add(duration)
    };
    let grace_until = if tier == SubscriptionTier::Free {
        now_epoch
    } else {
        exp_epoch.saturating_add(OFFLINE_GRACE_SECONDS)
    };

    let mut hasher = Sha256::new();
    hasher.update(subject.as_bytes());
    hasher.update(tier.as_str().as_bytes());
    hasher.update(now_epoch.to_le_bytes());
    let nonce = hex::encode(&hasher.finalize()[..8]);

    let claims = OsetClaims {
        ver: 1,
        iss: "qi-atelier-auth".to_string(),
        sub: subject.to_string(),
        tier: tier.as_str().to_string(),
        issued_at: now_epoch,
        refresh_after: now_epoch.saturating_add(7 * 86_400),
        exp_epoch,
        grace_until,
        features: tier.default_features(),
        seats: tier.default_seats(),
        guardian_sponsor,
        companion_loyalty_eligible,
        nonce,
    };
    let token = sign_oset(&claims)?;
    Ok((claims, token))
}

pub fn mint_sandbox_oset(
    subject: &str,
    tier_raw: &str,
    simulated_state: &str,
    guardian_sponsor: bool,
    loyalty_days: i64,
    now_epoch: i64,
) -> Result<String, String> {
    let tier = SubscriptionTier::from_str(tier_raw);
    let loyalty = loyalty_days >= LOYALTY_MIN_CHECKIN_DAYS;
    if tier == SubscriptionTier::Free {
        return Ok(String::new());
    }
    let (mut claims, _) = mint_oset_for_tier(subject, tier, now_epoch, guardian_sponsor, loyalty)?;
    match simulated_state.trim().to_ascii_lowercase().as_str() {
        "offline_grace" | "grace" => {
            claims.exp_epoch = now_epoch.saturating_sub(2 * 86_400);
            claims.grace_until = now_epoch.saturating_add(5 * 86_400);
        }
        "expired_read_only_vault" | "expired" => {
            claims.exp_epoch = now_epoch.saturating_sub(14 * 86_400);
            claims.grace_until = now_epoch.saturating_sub(7 * 86_400);
        }
        _ => {}
    }
    sign_oset(&claims)
}

pub fn verify_oset_with_clock(
    token: &str,
    now_epoch: i64,
    max_seen_epoch: i64,
    custom_pubkey_hex: Option<&str>,
) -> EntitlementVerificationResult {
    verify_oset_with_clock_localized(
        token,
        now_epoch,
        max_seen_epoch,
        custom_pubkey_hex,
        DEFAULT_LOCALE,
    )
}

pub fn verify_oset_with_clock_localized(
    token: &str,
    now_epoch: i64,
    max_seen_epoch: i64,
    custom_pubkey_hex: Option<&str>,
    locale: &str,
) -> EntitlementVerificationResult {
    let en = is_en_locale(locale);
    let new_max = max_seen_epoch.max(now_epoch);
    let free_result = |reason: &str, rollback: bool| EntitlementVerificationResult {
        valid_signature: false,
        tier: "free".to_string(),
        tier_display: SubscriptionTier::Free
            .display_name_localized(locale)
            .to_string(),
        state: OfflineEntitlementState::FreeLocal,
        is_atelier_unlocked: false,
        seats: 1,
        guardian_sponsor: false,
        companion_loyalty_eligible: false,
        exp_epoch: 0,
        grace_until: 0,
        grace_days_remaining: 0,
        clock_rollback_detected: rollback,
        effective_now_epoch: new_max,
        new_max_seen_epoch: new_max,
        features: SubscriptionTier::Free.default_features(),
        can_read_history_vault: true,
        can_export_vault: true,
        status_banner: if en {
            "All core rhythm logs, single-lab parsing, 1 custom pet, and local AI briefings stay free forever.".to_string()
        } else {
            "素心清欢 · 基础记录、本地 AI 晨报与 1 位自家灵宠永久免费".to_string()
        },
        reason: reason.to_string(),
    };

    let trimmed = token.trim();
    if trimmed.is_empty() {
        return free_result("no_token", false);
    }

    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() != 3 || parts[0] != "qient1" {
        return free_result("invalid_token_format", false);
    }

    let vk = if let Some(hex_str) = custom_pubkey_hex {
        match hex::decode(hex_str.trim()) {
            Ok(bytes) if bytes.len() == 32 => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                match VerifyingKey::from_bytes(&arr) {
                    Ok(v) => v,
                    Err(_) => return free_result("invalid_pubkey", false),
                }
            }
            _ => return free_result("invalid_pubkey_hex", false),
        }
    } else {
        default_verifying_key()
    };

    let sig_bytes = match URL_SAFE_NO_PAD.decode(parts[2]) {
        Ok(b) => b,
        Err(_) => return free_result("invalid_signature_b64", false),
    };
    let sig = match Signature::from_slice(&sig_bytes) {
        Ok(s) => s,
        Err(_) => return free_result("invalid_signature_len", false),
    };
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    if vk.verify(signing_input.as_bytes(), &sig).is_err() {
        return free_result("signature_verification_failed", false);
    }

    let claims_bytes = match URL_SAFE_NO_PAD.decode(parts[1]) {
        Ok(b) => b,
        Err(_) => return free_result("invalid_claims_b64", false),
    };
    let claims: OsetClaims = match serde_json::from_slice(&claims_bytes) {
        Ok(c) => c,
        Err(_) => return free_result("invalid_claims_json", false),
    };

    let tier_enum = SubscriptionTier::from_str(&claims.tier);
    if tier_enum == SubscriptionTier::Free {
        return free_result("free_tier_claim", false);
    }

    let clock_rollback = now_epoch.saturating_add(CLOCK_SKEW_TOLERANCE_SECONDS) < max_seen_epoch;
    let effective_now = if clock_rollback { max_seen_epoch } else { now_epoch };

    let (state, unlocked, banner, reason) = if clock_rollback {
        if effective_now <= claims.grace_until {
            (
                OfflineEntitlementState::OfflineGrace,
                true,
                if en {
                    "System clock rollback detected. Switched to offline grace mode; your saved history and custom pets remain untouched.".to_string()
                } else {
                    "检测到系统时钟回拨，已切换至离线宽限模式（历史资产与自家灵宠不受任何影响）".to_string()
                },
                "clock_rollback_grace",
            )
        } else {
            (
                OfflineEntitlementState::ExpiredReadOnlyVault,
                false,
                if en {
                    "Atelier pass expired. All saved keepsakes, stickers, custom pets, labs, and letters remain readable and exportable forever.".to_string()
                } else {
                    "雅集凭证已过期 · 所有历史风物、贴纸、自家灵宠、化验与家书永久可读可导出".to_string()
                },
                "clock_rollback_expired",
            )
        }
    } else if tier_enum == SubscriptionTier::LifetimeLocal || effective_now <= claims.exp_epoch {
        (
            OfflineEntitlementState::ActiveAtelier,
            true,
            if en {
                format!(
                    "{} · Offline cryptographic pass active",
                    tier_enum.display_name_localized("en")
                )
            } else {
                format!("{} · 离线密码学凭证有效", tier_enum.display_name())
            },
            "active_verified",
        )
    } else if effective_now <= claims.grace_until {
        let days_left = ((claims.grace_until - effective_now).max(0) + 86_399) / 86_400;
        (
            OfflineEntitlementState::OfflineGrace,
            true,
            if en {
                format!(
                    "Offline grace period · {} days left (offline logging and export remain available)",
                    days_left
                )
            } else {
                format!("离线宽限期 · 剩余 {} 天（断网亦可安心记录与导出）", days_left)
            },
            "offline_grace_window",
        )
    } else {
        (
            OfflineEntitlementState::ExpiredReadOnlyVault,
            false,
            if en {
                "Atelier expired · Read-only vault active. All saved stickers, custom pets, weekly letters, and lab archives remain yours forever.".to_string()
            } else {
                "雅集已到期 · 已开启「只读保险箱」，既往全部贴纸、自家灵宠、周信与化验档案永久为您保留".to_string()
            },
            "expired_read_only_vault",
        )
    };

    let grace_days_remaining = if state == OfflineEntitlementState::OfflineGrace {
        ((claims.grace_until - effective_now).max(0) + 86_399) / 86_400
    } else {
        0
    };

    let mut effective_features = SubscriptionTier::Free.default_features();
    if unlocked {
        for f in &claims.features {
            if !effective_features.contains(f) {
                effective_features.push(f.clone());
            }
        }
    }

    EntitlementVerificationResult {
        valid_signature: true,
        tier: tier_enum.as_str().to_string(),
        tier_display: tier_enum.display_name_localized(locale).to_string(),
        state,
        is_atelier_unlocked: unlocked,
        seats: claims.seats.max(1),
        guardian_sponsor: claims.guardian_sponsor,
        companion_loyalty_eligible: claims.companion_loyalty_eligible,
        exp_epoch: claims.exp_epoch,
        grace_until: claims.grace_until,
        grace_days_remaining,
        clock_rollback_detected: clock_rollback,
        effective_now_epoch: effective_now,
        new_max_seen_epoch: new_max,
        features: effective_features,
        can_read_history_vault: true,
        can_export_vault: true,
        status_banner: banner,
        reason: reason.to_string(),
    }
}

pub fn has_feature(verification: &EntitlementVerificationResult, feature: &str) -> bool {
    // Ethical red line #1 & #5: raw biometrics, single lab parse, JSONL/.qivault export,
    // recovery mode, 1 custom pet forever, and historical vault reading/exporting are ALWAYS unlocked.
    if matches!(
        feature,
        "raw_biometrics"
            | "single_lab_parse"
            | "jsonl_export"
            | "recovery_priority"
            | "basic_local_ai"
            | "can_read_history_vault"
            | "can_export_vault"
            | "free_solar_track"
            | "week1_companion_letter"
            | "custom_pet_1"
    ) {
        return true;
    }
    verification.is_atelier_unlocked && verification.features.iter().any(|f| f == feature)
}

pub fn custom_pet_quota(verification: &EntitlementVerificationResult) -> usize {
    if has_feature(verification, "custom_pets_3") {
        PRO_CUSTOM_PET_QUOTA
    } else {
        FREE_CUSTOM_PET_QUOTA
    }
}

pub fn build_entitlement_catalog(companion_loyalty_eligible: bool) -> EntitlementCatalog {
    build_entitlement_catalog_localized(companion_loyalty_eligible, DEFAULT_LOCALE)
}

pub fn build_entitlement_catalog_localized(
    companion_loyalty_eligible: bool,
    locale: &str,
) -> EntitlementCatalog {
    let en = is_en_locale(locale);
    let annual_price = if companion_loyalty_eligible { 98 } else { 118 };
    if en {
        let plans = vec![
            PricingPlanDescriptor {
                tier: "free".to_string(),
                title: "Qi Free · Always Free".to_string(),
                subtitle: "Core health rights stay unlocked: lab OCR, local Ollama briefings, double dew on rest days, 1 custom pet forever, JSONL export".to_string(),
                price_cny: 0,
                original_price_cny: None,
                billing_cycle: "Free forever".to_string(),
                seats: 1,
                highlights: vec![
                    "Full raw biometrics, unlimited single-person lab parsing, and 100% plain export".to_string(),
                    "Local AI (Ollama) 90-character morning briefing and Recovery Mode double dew stay free".to_string(),
                    "1 custom pet forever, week-1 companion letter, 5 monthly food stickers, and Solar Free track".to_string(),
                ],
                badge: "Health baseline · Never locked".to_string(),
                loyalty_applied: false,
            },
            PricingPlanDescriptor {
                tier: "pro_monthly".to_string(),
                title: "Qi Atelier Monthly".to_string(),
                subtitle: "Flexible monthly access with transparent renewal and zero dark patterns".to_string(),
                price_cny: 18,
                original_price_cny: None,
                billing_cycle: "30 days".to_string(),
                seats: 1,
                highlights: vec![
                    "All 24 solar term scenes, seasonal attire, custom seal inks, and 3 shelf display slots".to_string(),
                    "Up to 3 custom pets, custom paw Ex Libris stamps, and weekly pet first-person letters".to_string(),
                    "Unlimited food stickers, purine/sodium cues, and 90-day multi-lag correlation radar".to_string(),
                ],
                badge: "Flexible monthly".to_string(),
                loyalty_applied: false,
            },
            PricingPlanDescriptor {
                tier: "seasonal".to_string(),
                title: "Solar Season Pass".to_string(),
                subtitle: "Walk smoothly through an entire three-month solar season".to_string(),
                price_cny: 38,
                original_price_cny: Some(54),
                billing_cycle: "1 season (3 months)".to_string(),
                seats: 1,
                highlights: vec![
                    "Unlock all 6 Atelier seasonal keepsakes and solar term Ex Libris stamps for the season".to_string(),
                    "Four Seasons handscroll, up to 3 custom pets, and bidirectional WebDAV sync".to_string(),
                    "Custom N-of-1 experiment washout periods to uncover personal rhythm drivers".to_string(),
                ],
                badge: "Seasonal pick".to_string(),
                loyalty_applied: false,
            },
            PricingPlanDescriptor {
                tier: "pro_annual".to_string(),
                title: "Qi Atelier Annual".to_string(),
                subtitle: if companion_loyalty_eligible {
                    "30-day companion loyalty applied: ¥20 off, under ¥0.27 per day".to_string()
                } else {
                    "Full year of 24 solar terms, 52 companion letters, and 12 monthly calendar mats".to_string()
                },
                price_cny: annual_price,
                original_price_cny: Some(148),
                billing_cycle: "365 days".to_string(),
                seats: 1,
                highlights: vec![
                    "All 24 solar Atelier keepsakes, 52 companion & pet letters, and Archive Museum Shelf".to_string(),
                    "Multi-profile %ULN lab normalization plus automatic 90-day recheck reminders".to_string(),
                    "Large-print Family Peace Letters, Co-Window hosting, and 3 custom pets with Ex Libris".to_string(),
                ],
                badge: if companion_loyalty_eligible {
                    "30-day loyalty ¥98".to_string()
                } else {
                    "Most popular · Best value".to_string()
                },
                loyalty_applied: companion_loyalty_eligible,
            },
            PricingPlanDescriptor {
                tier: "duo".to_string(),
                title: "Co-Window Duo".to_string(),
                subtitle: "Two private vaults sharing only a 1-byte mood status and a shared window osmanthus".to_string(),
                price_cny: 168,
                original_price_cny: Some(236),
                billing_cycle: "365 days · 2 seats".to_string(),
                seats: 2,
                highlights: vec![
                    "Includes 2 full Qi Atelier Annual seats (¥84/person/year)".to_string(),
                    "Send warm tea, blankets, or gentle paw taps across windows with zero health metric exposure".to_string(),
                    "Grow a shared window osmanthus bonsai together from gentle daily rhythm completions".to_string(),
                ],
                badge: "Partners & close friends".to_string(),
                loyalty_applied: false,
            },
            PricingPlanDescriptor {
                tier: "family".to_string(),
                title: "Family Peace Letters".to_string(),
                subtitle: "Up to 4 family seats: turn gentle daily balance into reassuring large-print letters for parents".to_string(),
                price_cny: 228,
                original_price_cny: Some(472),
                billing_cycle: "365 days · 4 seats".to_string(),
                seats: 4,
                highlights: vec![
                    "Includes 4 isolated vaults plus multi-profile %ULN lab overview".to_string(),
                    "Generate warm large-print Family Peace Letters with one click".to_string(),
                    "Shared family seasonal museum access, unlimited stickers, and E2EE cloud sync".to_string(),
                ],
                badge: "Family peace · 4 seats".to_string(),
                loyalty_applied: false,
            },
            PricingPlanDescriptor {
                tier: "lifetime_local".to_string(),
                title: "Lifetime Local".to_string(),
                subtitle: "One-time purchase for the full local Atelier engine (BYO WebDAV sync free forever; excludes hosted cloud)".to_string(),
                price_cny: 368,
                original_price_cny: None,
                billing_cycle: "One-time purchase".to_string(),
                seats: 1,
                highlights: vec![
                    "Offline Ed25519 lifetime local license with zero renewal server dependency".to_string(),
                    "All solar term artwork, sticker filters, 90-day correlation radar, 3 custom pets, and WebDAV sync".to_string(),
                    "Your local .qivault archive continues running at full speed even offline for years".to_string(),
                ],
                badge: "One-time favorite".to_string(),
                loyalty_applied: false,
            },
        ];

        return EntitlementCatalog {
            plans,
            guardian_addon_title: "Hearth Gift Pass · Gift 1 Community Seat".to_string(),
            guardian_addon_price_cny: 12,
            guardian_addon_desc: "Optional ¥12 consumable pass: unlocks your Hearth Keeper ceramic badge and issues 1 transferable quarterly Family Peace Letter community seat.".to_string(),
            transparent_timeline: vec![
                TrialTimelineStep {
                    day_label: "Day 1 · Tonight".to_string(),
                    title: "Full Atelier seats open right away".to_string(),
                    detail: "Your offline Ed25519 pass writes directly into the local vault with clear renewal terms.".to_string(),
                },
                TrialTimelineStep {
                    day_label: "Day 4 · Gentle check-in".to_string(),
                    title: "Xiao Man sends your first rhythm recap".to_string(),
                    detail: "Review your collected dew drops, food stickers, and pet moments—cancel anytime with zero friction.".to_string(),
                },
                TrialTimelineStep {
                    day_label: "Day 7 · No surprise charges".to_string(),
                    title: "Clear renewal & read-only vault guarantee".to_string(),
                    detail: "If a subscription ends, your vault enters read-only retention: all pets, scrolls, stickers, and labs remain yours forever.".to_string(),
                },
            ],
            ethical_promises: vec![
                "Promise 1: Raw metrics, single-person lab parsing, 1 custom pet, and .qivault export never lock behind paywalls.".to_string(),
                "Promise 2: Recovery Mode stays free forever (+3 dew) with zero streak-revive traps.".to_string(),
                "Promise 3: Co-window sharing exchanges only a 1-byte mood code—never raw biometric or lab data.".to_string(),
            ],
        };
    }

    let plans = vec![
        PricingPlanDescriptor {
            tier: "free".to_string(),
            title: "素心清欢 · 永久免费".to_string(),
            subtitle: "核心健康权永久不锁：化验 OCR、Ollama 本地晨报、1 位自家灵宠、静养双倍露、JSONL 导出".to_string(),
            price_cny: 0,
            original_price_cny: None,
            billing_cycle: "永久免费".to_string(),
            seats: 1,
            highlights: vec![
                "完整生理数据 + 无限单人化验单解析 + 100% 明文导出".to_string(),
                "本地大模型 (Ollama) 90字晨间简报与「静养模式」双倍露永久免费".to_string(),
                "1 位自家灵宠永久驻窗 + 第 1 封伴侣周信 + 每月 5 张贴纸 + 节气清欢轨".to_string(),
            ],
            badge: "健康底线 · 永不涨价".to_string(),
            loyalty_applied: false,
        },
        PricingPlanDescriptor {
            tier: "pro_monthly".to_string(),
            title: "知气月雅 · 按月轻享".to_string(),
            subtitle: "按月随心开启雅集全功能，清晰透明可随时管理续期".to_string(),
            price_cny: 18,
            original_price_cny: None,
            billing_cycle: "30 天".to_string(),
            seats: 1,
            highlights: vec![
                "24 节气动态昼夜书房 + 「雅集轨」限定服饰、印泥与 3 个珍藏陈列位".to_string(),
                "至多 3 位自家灵宠、定制爪印藏书票 + 每周灵宠第一人称手写周信".to_string(),
                "无限饮食手账贴纸 + 嘌呤/钠提示 + 90 天多滞后归因雷达".to_string(),
            ],
            badge: "按月灵活".to_string(),
            loyalty_applied: false,
        },
        PricingPlanDescriptor {
            tier: "seasonal".to_string(),
            title: "岁时节气季卡".to_string(),
            subtitle: "陪你完整度过春/夏/秋/冬一整个节气季（3个月）".to_string(),
            price_cny: 38,
            original_price_cny: Some(54),
            billing_cycle: "一季 (3 个月)".to_string(),
            seats: 1,
            highlights: vec![
                "解锁当季全部 6 个节气「雅集轨」风物、节气藏书票与灵宠周信".to_string(),
                "四时身体风物长卷 + 至多 3 位灵宠驻窗 + WebDAV 双向加密同步".to_string(),
                "自定义 N-of-1 实验洗脱期，拿回生活归因权".to_string(),
            ],
            badge: "应季首选".to_string(),
            loyalty_applied: false,
        },
        PricingPlanDescriptor {
            tier: "pro_annual".to_string(),
            title: "知气雅集 · 年卡".to_string(),
            subtitle: if companion_loyalty_eligible {
                "已触发「连续陪伴 30 天」礼遇价：直降 ¥20，每天不到 ¥0.27".to_string()
            } else {
                "一年 24 节气、52 封周信、12 张节气月历垫全收录".to_string()
            },
            price_cny: annual_price,
            original_price_cny: Some(148),
            billing_cycle: "365 天".to_string(),
            seats: 1,
            highlights: vec![
                "解锁全部 24 节气雅集风物、52 封周信 + 「往期节气博物架」".to_string(),
                "多亲友化验档案 %ULN 归一化对比 + 90 天复查自动提醒 + 3 位灵宠与爪印印记".to_string(),
                "一键生成E2EE大字版《气韵平安家书》+ 「同窗共养」串门房主权".to_string(),
            ],
            badge: if companion_loyalty_eligible {
                "30天陪伴礼遇价 ¥98".to_string()
            } else {
                "最受欢迎 · 性价比之选".to_string()
            },
            loyalty_applied: companion_loyalty_eligible,
        },
        PricingPlanDescriptor {
            tier: "duo".to_string(),
            title: "双人同窗 · Duo".to_string(),
            subtitle: "两人各享独立私密 Vault，只同步 1 字节心情枚举与合种窗台金桂".to_string(),
            price_cny: 168,
            original_price_cny: Some(236),
            billing_cycle: "365 天 · 2 席".to_string(),
            seats: 2,
            highlights: vec![
                "含 2 个完整知气雅集年卡席位（均摊 ¥84/人/年）".to_string(),
                "双人互相串门递热茶/盖薄被/按爪印 + 零健康数据泄漏".to_string(),
                "合养「窗台岁时金桂盆栽」：双方温和达标共同抽出新枝".to_string(),
            ],
            badge: "情侣/挚友最爱".to_string(),
            loyalty_applied: false,
        },
        PricingPlanDescriptor {
            tier: "family".to_string(),
            title: "四时家书 · Family".to_string(),
            subtitle: "至多 4 位家人共享：把自己的温和气色变成爸妈安心的读报".to_string(),
            price_cny: 228,
            original_price_cny: Some(472),
            billing_cycle: "365 天 · 4 席".to_string(),
            seats: 4,
            highlights: vec![
                "含 4 个独立 Vault + 多亲友化验 %ULN 纵览".to_string(),
                "一键生成大字暖心《气韵平安家书》发给父母或子女".to_string(),
                "全家人共享节气博物架、无限贴纸、灵宠周信与端到端云同步".to_string(),
            ],
            badge: "全家安心 · 4 席".to_string(),
            loyalty_applied: false,
        },
        PricingPlanDescriptor {
            tier: "lifetime_local".to_string(),
            title: "买断本地 · 终身版".to_string(),
            subtitle: "一次买断本地全套雅集与计算引擎（不含托管云流量，WebDAV 自有云永久免费）".to_string(),
            price_cny: 368,
            original_price_cny: None,
            billing_cycle: "永久买断".to_string(),
            seats: 1,
            highlights: vec![
                "离线 Ed25519 终身本地授权，永不依赖任何续费服务器".to_string(),
                "本地全部节气美术、3 位自家灵宠、90天归因雷达与 WebDAV 双向同步".to_string(),
                "未来即使断网十年，本地 .qivault 保险箱仍全速运转".to_string(),
            ],
            badge: "买断党挚爱".to_string(),
            loyalty_applied: false,
        },
    ];

    EntitlementCatalog {
        plans,
        guardian_addon_title: "窗台添柴礼赠券 · 赠予社区漂流席位".to_string(),
        guardian_addon_price_cny: 12,
        guardian_addon_desc: "可选 +¥12 消耗型礼赠券：点亮「窗台添柴人」宋瓷徽章，并购买 1 份可兑现的季度《气韵平安家书》社区漂流席位赠券，可转赠亲友或流转社区漂流窗台。".to_string(),
        transparent_timeline: vec![
            TrialTimelineStep {
                day_label: "第 1 天 · 今晚".to_string(),
                title: "全功能雅集即刻落座".to_string(),
                detail: "离线 Ed25519 通行证立刻写入本地保险箱，订阅周期与续费方式透明可控。".to_string(),
            },
            TrialTimelineStep {
                day_label: "第 4 天 · 温柔回望".to_string(),
                title: "小满给你寄出第一张试炼总结".to_string(),
                detail: "看看这几天攒了几颗晨露、做了几张食光贴纸，不喜欢随时停下。".to_string(),
            },
            TrialTimelineStep {
                day_label: "第 7 天 · 只读护航".to_string(),
                title: "到期绝不锁死任何数据".to_string(),
                detail: "到期后若不续费，自动进入只读保险箱：既往画卷、贴纸、自家灵宠与化验永久留存，绝不删档或隐藏。".to_string(),
            },
        ],
        ethical_promises: vec![
            "红线 1：原始健康指标、单人化验单、1 位自家灵宠与 .qivault 导出永远免费且不锁门".to_string(),
            "红线 2：「安心静养」生病模式永久免费 (+3 💧)，绝不卖连续打卡复活卡".to_string(),
            "红线 3：不设暗扣连包陷阱；双人/家庭串门零隐私裸露（仅 1 字节心情枚举）".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oset_ed25519_sign_verify_and_tamper_rejection() {
        let now = 1_760_000_000;
        let (_claims, token) =
            mint_oset_for_tier("usr_tester", SubscriptionTier::ProAnnual, now, true, true)
                .expect("mint");
        let res = verify_oset_with_clock(&token, now + 60, now, None);
        assert!(res.valid_signature);
        assert_eq!(res.state, OfflineEntitlementState::ActiveAtelier);
        assert!(res.is_atelier_unlocked);
        assert!(res.guardian_sponsor);
        assert!(res.companion_loyalty_eligible);
        assert!(has_feature(&res, "multi_lag_radar"));

        // Tamper claims payload -> must reject cleanly and fall back to FreeLocal
        let mut parts: Vec<String> = token.split('.').map(ToString::to_string).collect();
        parts[1].push('A');
        let tampered = parts.join(".");
        let bad = verify_oset_with_clock(&tampered, now + 60, now, None);
        assert!(!bad.valid_signature);
        assert_eq!(bad.state, OfflineEntitlementState::FreeLocal);
        assert!(!bad.is_atelier_unlocked);
        // Vault export stays allowed even after invalid token
        assert!(has_feature(&bad, "jsonl_export"));
        assert!(has_feature(&bad, "recovery_priority"));
    }

    #[test]
    fn test_offline_grace_expired_vault_and_clock_rollback() {
        let now = 1_760_000_000;
        let (_claims, token) =
            mint_oset_for_tier("usr_tester", SubscriptionTier::ProMonthly, now, false, false)
                .expect("mint");

        // 2 days after 30-day expiry -> inside 7-day OfflineGrace
        let exp = now + 30 * 86_400;
        let grace = verify_oset_with_clock(&token, exp + 2 * 86_400, now, None);
        assert_eq!(grace.state, OfflineEntitlementState::OfflineGrace);
        assert!(grace.is_atelier_unlocked);
        assert_eq!(grace.grace_days_remaining, 5);

        // 10 days after expiry -> ExpiredReadOnlyVault (past archive ALWAYS readable/exportable)
        let expired = verify_oset_with_clock(&token, exp + 10 * 86_400, exp + 2 * 86_400, None);
        assert_eq!(expired.state, OfflineEntitlementState::ExpiredReadOnlyVault);
        assert!(!expired.is_atelier_unlocked);
        assert!(expired.can_read_history_vault);
        assert!(expired.can_export_vault);
        assert!(has_feature(&expired, "single_lab_parse"));
        assert!(has_feature(&expired, "recovery_priority"));

        // Clock rollback attempt (setting system clock back to `now` when max_seen is `exp + 10 days`)
        let rollback = verify_oset_with_clock(&token, now, exp + 10 * 86_400, None);
        assert!(rollback.clock_rollback_detected);
        assert_eq!(
            rollback.state,
            OfflineEntitlementState::ExpiredReadOnlyVault
        );
        assert!(!rollback.is_atelier_unlocked);
    }

    #[test]
    fn test_v5_bilingual_catalog_custom_pet_quota_and_commercial_gift_pass() {
        assert_eq!(normalize_locale(""), "zh-CN");
        assert_eq!(normalize_locale("zh-Hans-CN"), "zh-CN");
        assert_eq!(normalize_locale("en-US"), "en");
        assert_eq!(normalize_locale("en"), "en");

        let free = verify_oset_with_clock_localized("", 1_760_000_000, 0, None, "en");
        assert_eq!(custom_pet_quota(&free), FREE_CUSTOM_PET_QUOTA);
        assert!(has_feature(&free, "custom_pet_1"));
        assert!(!has_feature(&free, "pet_weekly_letters"));
        assert_eq!(free.tier_display, "Qi Free");

        let now = 1_760_000_000;
        let (_claims, token) =
            mint_oset_for_tier("usr_pet", SubscriptionTier::ProAnnual, now, false, true)
                .expect("mint");
        let pro_en = verify_oset_with_clock_localized(&token, now + 100, now, None, "en");
        assert_eq!(custom_pet_quota(&pro_en), PRO_CUSTOM_PET_QUOTA);
        assert!(has_feature(&pro_en, "custom_pet_ex_libris"));
        assert!(has_feature(&pro_en, "pet_weekly_letters"));
        assert!(!pro_en.status_banner.contains("知气 · Qi"));

        for loc in ["zh-CN", "en"] {
            let cat = build_entitlement_catalog_localized(true, loc);
            assert!(
                cat.guardian_addon_title.contains("窗台添柴礼赠券")
                    || cat.guardian_addon_title.contains("Hearth Gift Pass")
            );
            for banned in ["公益", "慈善", "捐", "charity", "donat"] {
                assert!(
                    !cat.guardian_addon_title.to_lowercase().contains(banned)
                        && !cat.guardian_addon_desc.to_lowercase().contains(banned),
                    "found banned term {banned} in locale {loc}"
                );
            }
        }
    }
}

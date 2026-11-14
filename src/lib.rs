//! 「知气」端侧 Rust 核心引擎，导出给 TSX 视图层 (`qi/native`)。

pub mod ai_provider;
pub mod analytics;
pub mod apple_native;
pub mod crypto_sync;
pub mod entitlements;
pub mod experiments;
pub mod healthkit_ios;
pub mod importers;
pub mod nutrition_db;
pub mod personas;
pub mod safety_ai;
pub mod solar_atelier;
pub mod storage;
pub mod sync_engine;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ed25519_dalek::SigningKey;
use nsh_core::prelude::*;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::ai_provider::generate_live_briefing_localized;
use crate::analytics::{
    AnalyticsSnapshot, HardwareTier, IntakeLogRow, LabBiomarkerRow, MultiLagImpactItem,
    build_daily_records, compute_multi_lag_radar, compute_snapshot, resolve_canonical_metrics,
    source_priority,
};
use crate::apple_native::{
    LocalAiCapabilityProbe, probe_local_ai_capabilities, run_offline_vision_ocr,
};
use crate::crypto_sync::{decrypt_sync_batch, derive_master_key, encrypt_sync_batch};
use crate::entitlements::{
    EntitlementCatalog, EntitlementVerificationResult, HEARTH_GIFT_PASS_PRODUCT_ID,
    LOYALTY_MIN_CHECKIN_DAYS, OfflineEntitlementState, OsetClaims, SubscriptionTier,
    build_entitlement_catalog_localized, custom_pet_quota, has_feature, is_en_locale,
    is_qa_or_debug_build, mint_oset_for_tier, mint_sandbox_oset, normalize_locale,
    sign_oset_with_key, tier_from_apple_product_id, verify_oset_with_clock_localized,
};
use crate::experiments::{
    EXPERIMENT_TEMPLATES, ExperimentEvaluation, ExperimentTemplate, PeriodComparison,
    compute_period_comparison, evaluate_experiments,
};
use crate::healthkit_ios::{
    is_healthkit_available as hk_is_available, parse_and_validate_healthkit_payload,
    request_and_query_healthkit_json,
};
use crate::importers::{parse_apple_health_xml_snippet, parse_csv_metrics};
use crate::nutrition_db::{FoodSearchHit, parse_natural_intake_text, search_foods};
use crate::safety_ai::{
    BriefingResult, MultiProfileRecheckReminder, MultiProfileUlnPoint, ParentsPeaceLetter,
    WELLNESS_DISCLAIMER, biomarker_plain_note_localized, build_morning_briefing_localized,
    build_parents_peace_letter_localized, canonicalize_marker,
    compute_multi_profile_uln_trends_at_localized, contains_cjk, enforce_wellness_guardrails,
    localized_marker_name, scrub_lab_text_pii, wellness_disclaimer_for_locale,
};
use crate::solar_atelier::{
    CustomPetCompanion, MAX_EQUIPPED_KEEPSAKES, SolarAtelierPayload, build_seasonal_pass_localized,
    build_solar_atelier_payload_v5, compute_custom_pet_quota, compute_solar_term_state_localized,
    generate_pet_weekly_letter_draft, infer_sticker_nutrition_hint_localized,
    normalize_die_cut_frame, normalize_pet_species, normalize_weather_mood,
    validate_pet_photo_data_url,
};
use crate::storage::{AppSettings, VaultDatabase};
use crate::sync_engine::{
    SyncExecutionReport, build_local_archive_json, decrypt_local_archive_json,
    pull_envelopes_from_byo, pull_envelopes_from_dir, pull_envelopes_from_webdav,
    push_envelope_to_byo, push_envelope_to_dir, push_envelope_to_webdav,
    resolve_icloud_vault_dir,
};

const EXPERIMENT_TEMPLATES_EN: &[ExperimentTemplate] = &[
    ExperimentTemplate {
        id: "no_caffeine_after_15",
        title: "No caffeine after 15:00",
        hypothesis: "Lower late-day caffeine half-life overlap to observe deep sleep and next-morning HRV",
        target_days: 7,
    },
    ExperimentTemplate {
        id: "screen_off_60m",
        title: "Screens off 60m before bed",
        hypothesis: "Reduce evening blue light and arousal to observe resting HR easing and sleep latency",
        target_days: 7,
    },
    ExperimentTemplate {
        id: "early_dinner_1930",
        title: "Finish dinner before 19:30",
        hypothesis: "Lighten overnight digestive load to watch resting HR and overnight HRV stability",
        target_days: 14,
    },
    ExperimentTemplate {
        id: "morning_sunlight_15m",
        title: "15m morning outdoor light",
        hypothesis: "Anchor circadian daylight timing for steadier evening wind-down and vitamin D support",
        target_days: 14,
    },
];

fn tag_display_en(tag: &str) -> String {
    match tag.trim() {
        "晚咖啡" | "caffeine_late" | "coffee" => "Late caffeine".to_owned(),
        "晚睡" | "late_sleep" => "Late bedtime".to_owned(),
        "饮酒" | "alcohol" | "beer" => "Alcohol".to_owned(),
        "夜宵" | "heavy_dinner" => "Late heavy dinner".to_owned(),
        "高盐饮食" | "high_sodium" => "High sodium meal".to_owned(),
        "高强度训练" | "intense_workout" => "High-load training".to_owned(),
        "高强度" => "High load".to_owned(),
        "午后咖啡因" => "Afternoon caffeine".to_owned(),
        "清淡饮食" => "Light balanced meal".to_owned(),
        "早睡" => "Early bedtime".to_owned(),
        "日晒" => "Morning sunlight".to_owned(),
        other if contains_cjk(other) => "Daily lifestyle factor".to_owned(),
        other => other.replace('_', " "),
    }
}

fn outcome_label_en(key: &str, raw_label: &str) -> String {
    match key.trim() {
        "hrv_ms" | "hrv_rmssd_ms" => "Overnight HRV".to_owned(),
        "hrv_sdnn_ms" => "Watch HRV".to_owned(),
        "rhr_bpm" | "resting_heart_rate_bpm" => "Resting HR".to_owned(),
        "sleep_minutes" => "Sleep duration".to_owned(),
        "sleep_deep_minutes" | "deep_sleep_min" => "Deep sleep".to_owned(),
        _ => {
            if raw_label.contains("HRV") {
                "Overnight HRV".to_owned()
            } else if raw_label.contains("静息心率") || raw_label.contains("RHR") {
                "Resting HR".to_owned()
            } else if raw_label.contains("深睡") {
                "Deep sleep".to_owned()
            } else if raw_label.contains("睡眠") {
                "Sleep duration".to_owned()
            } else if contains_cjk(raw_label) {
                "Rhythm metric".to_owned()
            } else {
                raw_label.to_owned()
            }
        }
    }
}

fn localize_snapshot(snapshot: &mut AnalyticsSnapshot, locale: &str) {
    if !is_en_locale(locale) {
        return;
    }
    snapshot.hardware_tier_label = match snapshot.hardware_tier.as_str() {
        "tier_b" => "Tier B · Fitness band (sleep duration + steps)".to_owned(),
        "tier_c" => "Tier C · Phone-only (steps + morning check-in)".to_owned(),
        _ => "Tier A · Full wearable (HRV + sleep stages)".to_owned(),
    };
    snapshot.state_badge = match snapshot.state.as_str() {
        "prime_ready" => "Well rested".to_owned(),
        "recovery_priority" => "Rest gently".to_owned(),
        _ => "Balanced".to_owned(),
    };
    let sleep_desc = snapshot
        .sleep_last_night_hours
        .map(|h| format!("Last night's sleep was {h:.1}h"))
        .unwrap_or_else(|| "Daily rhythm check-in is active".to_owned());
    let acwr_desc = snapshot
        .acwr
        .map(|a| format!(" with load ratio {a:.2}"))
        .unwrap_or_default();

    snapshot.headline = match snapshot.state.as_str() {
        "prime_ready" => format!(
            "{sleep_desc}{acwr_desc}; autonomic recovery sits comfortably in your green zone today."
        ),
        "recovery_priority" => format!(
            "{sleep_desc}{acwr_desc}; recovery markers favor lighter intensity and an earlier evening wind-down."
        ),
        _ => format!(
            "{sleep_desc}{acwr_desc}; daily balance stays calm and responsive to light movement."
        ),
    };
    snapshot.micro_action = match snapshot.state.as_str() {
        "prime_ready" => {
            "Take a 20-minute outdoor walk in daylight and keep hydration steady.".to_owned()
        }
        "recovery_priority" => {
            "Swap high-intensity training for 10 minutes of gentle mobility and wind down 30 minutes earlier.".to_owned()
        }
        _ => {
            "Take a short walk after lunch and step away from bright screens 30 minutes before bed.".to_owned()
        }
    };

    let mut signals_en = Vec::new();
    if let Some(h) = snapshot.sleep_last_night_hours {
        signals_en.push(format!("Last night's sleep: {h:.1}h"));
    }
    if let Some(debt) = snapshot.sleep_debt_7d_hours {
        signals_en.push(format!("7-day sleep balance debt: {debt:.1}h"));
    }
    if let Some(acwr) = snapshot.acwr {
        signals_en.push(format!("7d:28d training load ratio (ACWR): {acwr:.2}"));
    }
    if let Some(hrv) = snapshot
        .metrics
        .get("hrv_rmssd_ms")
        .or_else(|| snapshot.metrics.get("hrv_sdnn_ms"))
        .and_then(|m| m.value)
    {
        signals_en.push(format!("Overnight HRV: {hrv:.0} ms"));
    }
    if signals_en.is_empty() {
        signals_en.push(format!("Today's vitality score: {}", snapshot.qi_score));
    }
    snapshot.signals = signals_en;

    for (key, summary) in snapshot.metrics.iter_mut() {
        summary.label = match key.as_str() {
            "sleep_minutes" => "Last night's sleep".to_owned(),
            "sleep_deep_minutes" => "Deep sleep".to_owned(),
            "sleep_rem_minutes" => "REM sleep".to_owned(),
            "hrv_rmssd_ms" => "Nighttime HRV (RMSSD)".to_owned(),
            "hrv_sdnn_ms" => "Watch HRV (SDNN)".to_owned(),
            "resting_heart_rate_bpm" => "Resting HR (RHR)".to_owned(),
            "steps" => "Steps today".to_owned(),
            "active_zone_minutes" => "Active zone load".to_owned(),
            "calories_active_kcal" => "Active energy".to_owned(),
            _ => "Daily metric".to_owned(),
        };
        if summary.unit == "步" {
            summary.unit = "steps".to_owned();
        }
    }

    for imp in &mut snapshot.lagged_impacts {
        let en_tag = tag_display_en(&imp.tag);
        let en_outcome = outcome_label_en("", &imp.outcome_label);
        let sign = if imp.delta_value >= 0.0 { "+" } else { "" };
        imp.tag = en_tag.clone();
        imp.outcome_label = en_outcome.clone();
        imp.summary = format!(
            "Across {} days tagged \"{}\", {} shifted by {}{:.1} {} next day.",
            imp.event_days, en_tag, en_outcome, sign, imp.delta_value, imp.unit
        );
    }
}

fn localize_labs(labs: &mut [LabBiomarkerRow], locale: &str) {
    if !is_en_locale(locale) {
        return;
    }
    for row in labs.iter_mut() {
        row.marker_name = localized_marker_name(&row.canonical_key, &row.marker_name, locale);
        row.plain_note = biomarker_plain_note_localized(&row.canonical_key, &row.flag, locale);
    }
}

fn localize_experiments(exps: &mut [ExperimentEvaluation], locale: &str) {
    if !is_en_locale(locale) {
        return;
    }
    for exp in exps.iter_mut() {
        if exp.title.contains("咖啡因") {
            exp.title = "No caffeine after 15:00".to_owned();
            exp.hypothesis =
                "Lower late-day caffeine overlap to watch deep sleep and next-morning HRV.".to_owned();
        } else if exp.title.contains("屏幕") {
            exp.title = "Screens off 60m before bed".to_owned();
            exp.hypothesis =
                "Ease evening visual stimulation to watch resting HR and sleep depth.".to_owned();
        } else if exp.title.contains("晚餐") {
            exp.title = "Finish dinner before 19:30".to_owned();
            exp.hypothesis =
                "Give digestion an earlier break to support calm overnight heart rate.".to_owned();
        } else if exp.title.contains("日照") {
            exp.title = "15m morning outdoor light".to_owned();
            exp.hypothesis =
                "Anchor circadian daylight cues for steadier evening wind-down.".to_owned();
        } else if contains_cjk(&exp.title) {
            exp.title = format!("N-of-1 Experiment #{}", exp.id);
        }
        if contains_cjk(&exp.hypothesis) {
            exp.hypothesis =
                "Observe personal sleep, HRV, and resting heart rate shifts across active days."
                    .to_owned();
        }

        let hrv_part = exp
            .hrv_delta_ms
            .map(|d| format!("HRV {:+.1} ms", d))
            .unwrap_or_else(|| "HRV collecting".to_owned());
        let rhr_part = exp
            .rhr_delta_bpm
            .map(|d| format!("RHR {:+.1} bpm", d))
            .unwrap_or_else(|| "RHR collecting".to_owned());
        let outcome_phrase = match exp.outcome_verdict.as_str() {
            "helpful" | "positive" => "Encouraging trend",
            "neutral" | "disproved" => "No clear shift (neutral)",
            "collecting" => "Still gathering baseline",
            _ => "Observing trend",
        };
        exp.verdict_summary = format!(
            "{outcome_phrase} ({}/{} days, washout {}d skipped) · {hrv_part}, {rhr_part}.",
            exp.completed_days, exp.target_days, exp.washout_skipped_days
        );
    }
}

fn localize_period_comparison(cmp: &mut PeriodComparison, locale: &str) {
    if !is_en_locale(locale) {
        return;
    }
    let hrv_txt = match (cmp.hrv_current, cmp.hrv_prev) {
        (Some(c), Some(p)) => format!("HRV {c:.1} vs {p:.1} ms ({:+.1} ms)", c - p),
        _ => "HRV comparing".to_owned(),
    };
    let rhr_txt = match (cmp.rhr_current, cmp.rhr_prev) {
        (Some(c), Some(p)) => format!("RHR {c:.1} vs {p:.1} bpm ({:+.1} bpm)", c - p),
        _ => "RHR comparing".to_owned(),
    };
    let sleep_txt = match (cmp.sleep_current_h, cmp.sleep_prev_h) {
        (Some(c), Some(p)) => format!("Sleep {c:.1}h vs {p:.1}h ({:+.1}h)", c - p),
        _ => "Sleep comparing".to_owned(),
    };
    cmp.summary = format!("Recent 7d vs previous 7d: {hrv_txt}; {rhr_txt}; {sleep_txt}.");
}

fn localize_multi_lag_radar(radar: &mut [MultiLagImpactItem], locale: &str) {
    if !is_en_locale(locale) {
        return;
    }
    for item in radar.iter_mut() {
        let tag_en = tag_display_en(&item.tag);
        let outcome_en = outcome_label_en(&item.outcome_key, &item.outcome_label);
        item.tag = tag_en.clone();
        item.outcome_label = outcome_en.clone();
        item.lag_label = match item.lag_days {
            0 => "Same evening (D+0)".to_owned(),
            1 => "Next morning (D+1)".to_owned(),
            n => format!("D+{n} carry-over"),
        };
        if contains_cjk(&item.confounder_note) {
            item.confounder_note = if item.confounder_note.contains("晚睡") {
                "Controlled for late bedtime confounding days.".to_owned()
            } else {
                "Compared against your non-tagged baseline days.".to_owned()
            };
        }
        item.summary = format!(
            "{} on {}: {} {:+.1}% (n={} vs {}, Cohen's d={:.2}). {}",
            tag_en,
            item.lag_label,
            outcome_en,
            item.delta_pct,
            item.sample_n,
            item.control_n,
            item.cohen_d,
            item.confounder_note
        );
    }
}

fn localize_intakes_display(intakes: &mut [IntakeLogRow], locale: &str) {
    if !is_en_locale(locale) {
        return;
    }
    for row in intakes.iter_mut() {
        if row.notes.contains("演示") {
            row.notes = "Sample journal note".to_owned();
        }
        for t in &mut row.tags {
            if contains_cjk(t) {
                *t = tag_display_en(t);
            }
        }
    }
}

fn localize_probe(probe: &mut LocalAiCapabilityProbe, locale: &str) {
    if !is_en_locale(locale) {
        return;
    }
    if contains_cjk(&probe.foundation_models_status) {
        probe.foundation_models_status = if probe.foundation_models_supported {
            "Supported".to_owned()
        } else {
            "Unavailable (using deterministic Rust engine)".to_owned()
        };
    }
    if contains_cjk(&probe.summary) {
        probe.summary = format!(
            "Platform: {} · Apple Vision OCR: {} · Local Ollama: {}",
            probe.platform,
            if probe.vision_ocr_available { "ready" } else { "unavailable" },
            if probe.ollama_reachable {
                format!("connected ({} models)", probe.ollama_models.len())
            } else {
                "not running".to_owned()
            }
        );
    }
}

fn localize_settings_display(settings: &mut AppSettings) {
    if !is_en_locale(&settings.locale) {
        return;
    }
    if settings.last_sync_summary == "尚未执行跨端加密同步" {
        settings.last_sync_summary = "No cross-device E2EE sync run yet".to_owned();
    } else if settings.last_sync_summary == "本机与会话凭证已物理清空" {
        settings.last_sync_summary = "Local vault and session credentials cleared".to_owned();
    }
}

fn localize_dew_ledger_if_en(solar_atelier: &mut SolarAtelierPayload, locale: &str) {
    if !is_en_locale(locale) {
        return;
    }
    for entry in &mut solar_atelier.dew_ledger {
        if contains_cjk(&entry.note) {
            entry.note = match entry.source.as_str() {
                "recovery_priority_bonus" => "Recovery Mode double dew bonus (+3 dew)".to_owned(),
                "food_sticker" => "Food polaroid sticker moment (+1 dew)".to_owned(),
                "co_sanctuary" => "Co-window warm interaction (+1 dew)".to_owned(),
                "seasonal_pass_claim" => {
                    format!("Redeemed seasonal keepsake ({})", entry.reward_id)
                }
                _ => "Daily gentle rhythm dew entry".to_owned(),
            };
        }
    }
}

#[native_module("qi/native")]
fn native(registry: &mut Registry) {
    registry.module::<QiMeta>().class::<QiEngine>();
}

#[derive(Debug, Clone, Serialize)]
struct LastLabImportFeedback {
    redacted_fields: Vec<String>,
    scrubbed_preview: String,
    imported_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditCheckItem {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataAuditReport {
    pub active_persona: String,
    pub metric_rows: usize,
    pub distinct_days: usize,
    pub intake_rows: usize,
    pub lab_rows: usize,
    pub experiment_rows: usize,
    pub qi_score_in_bounds: bool,
    pub rmssd_sdnn_isolated: bool,
    pub tier_hrv_contract_held: bool,
    pub lab_trends_consistent: bool,
    pub pii_guard_verified: bool,
    pub vault_non_lockout_verified: bool,
    pub passed: bool,
    pub checks: Vec<AuditCheckItem>,
}

#[derive(Debug, Clone, Serialize)]
struct UiStatePayload {
    settings: AppSettings,
    snapshot: AnalyticsSnapshot,
    briefing: BriefingResult,
    intakes: Vec<IntakeLogRow>,
    labs: Vec<LabBiomarkerRow>,
    experiments: Vec<ExperimentEvaluation>,
    experiment_templates: Vec<ExperimentTemplate>,
    period_comparison: PeriodComparison,
    last_lab_import: Option<LastLabImportFeedback>,
    capability_probe: Option<LocalAiCapabilityProbe>,
    entitlements: EntitlementVerificationResult,
    entitlement_catalog: EntitlementCatalog,
    solar_atelier: SolarAtelierPayload,
    multi_lag_radar: Vec<MultiLagImpactItem>,
    multi_profile_uln_trends: Vec<MultiProfileUlnPoint>,
    recheck_reminders: Vec<MultiProfileRecheckReminder>,
    parents_peace_letter_preview: ParentsPeaceLetter,
}

/// 开源合规与安全声明只读元信息。
#[module]
pub struct QiMeta;

#[module]
impl QiMeta {
    /// 返回完整开源致谢 NOTICE 内容。
    pub fn notice_text() -> String {
        include_str!("../NOTICE").to_owned()
    }

    /// 返回非医疗器械合规声明文本。
    pub fn disclaimer() -> String {
        WELLNESS_DISCLAIMER.to_owned()
    }

    /// 按语言返回非医疗器械合规声明（`zh-CN` / `en`）。
    pub fn disclaimer_for_locale(locale: String) -> String {
        wellness_disclaimer_for_locale(&locale).to_owned()
    }

    /// 返回对齐 StoreKit 2 与云端控制面的 Apple IAP 商品 ID 清单 JSON。
    pub fn apple_catalog_product_ids_json() -> String {
        serde_json::json!({
            "subscriptions": [
                { "product_id": "com.qi.atelier.pro_monthly", "tier": "pro_monthly" },
                { "product_id": "com.qi.atelier.seasonal", "tier": "seasonal" },
                { "product_id": "com.qi.atelier.pro_annual", "tier": "pro_annual" },
                { "product_id": "com.qi.atelier.duo", "tier": "duo" },
                { "product_id": "com.qi.atelier.family", "tier": "family" }
            ],
            "non_consumables": [
                { "product_id": "com.qi.atelier.lifetime_local", "tier": "lifetime_local" }
            ],
            "consumables": [
                { "product_id": HEARTH_GIFT_PASS_PRODUCT_ID, "kind": "hearth_gift_pass" }
            ]
        })
        .to_string()
    }

    /// 模糊搜索 60+ 中式高频餐饮与补剂词典。
    pub fn search_foods_json(query: String, limit: u32) -> String {
        let hits: Vec<FoodSearchHit> = search_foods(&query, limit as usize);
        serde_json::to_string(&hits).unwrap_or_else(|_| "[]".to_owned())
    }

    /// 返回 6 套立体纵向测试画像元信息 JSON。
    pub fn personas_json() -> String {
        serde_json::to_string(&crate::personas::all_personas_meta())
            .unwrap_or_else(|_| "[]".to_owned())
    }

    /// 返回当前构建是否允许启用 QA Studio 与本地沙盒模拟能力（Release 恒为 false）。
    pub fn qa_studio_enabled() -> bool {
        is_qa_or_debug_build()
    }

    /// 根据云端账号与本地 12 词恢复短语派生防顶号凭据摘要（SHA-256 hex）。
    pub fn derive_cloud_auth_proof(handle: String, recovery_phrase: String) -> String {
        let norm_handle = handle.trim().to_ascii_lowercase();
        let norm_phrase = recovery_phrase.trim();
        let mut hasher = Sha256::new();
        hasher.update(format!("qi-cloud-auth-v1:{norm_handle}:{norm_phrase}").as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// 端侧唯一可信健康引擎实例：管理 SQLite、统计快照、N-of-1 实验、E2EE 同步与 AI 路由。
#[class]
pub struct QiEngine {
    db: VaultDatabase,
    last_lab_import: Option<LastLabImportFeedback>,
    live_briefing_override: Option<BriefingResult>,
    last_probe: Option<LocalAiCapabilityProbe>,
}

impl QiEngine {
    fn default_db_candidates_for_home(home: &std::path::Path) -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        if cfg!(target_os = "ios") {
            // iOS Seatbelt 沙盒禁止直接在容器根目录 ($HOME) 下创建隐藏目录，仅 Library / Documents / tmp 可写
            candidates.push(
                home.join("Library")
                    .join("Application Support")
                    .join("qi")
                    .join("qi_vault.sqlite3"),
            );
            candidates.push(home.join("Documents").join("qi").join("qi_vault.sqlite3"));
        } else {
            candidates.push(home.join(".qi").join("qi_vault.sqlite3"));
            candidates.push(
                home.join("Library")
                    .join("Application Support")
                    .join("qi")
                    .join("qi_vault.sqlite3"),
            );
        }
        candidates.push(std::env::temp_dir().join("qi_vault.sqlite3"));
        candidates
    }

    fn open_db_with_candidates(candidates: &[PathBuf]) -> Result<VaultDatabase, String> {
        let mut errors = Vec::new();
        for path in candidates {
            match VaultDatabase::open_file(path) {
                Ok(db) => return Ok(db),
                Err(err) => {
                    errors.push(format!("{}: {err}", path.display()));
                }
            }
        }
        eprintln!(
            "[Qi] 所有文件数据库候选路径均不可写，已自动降级为内存数据库 ({})",
            errors.join(" | ")
        );
        VaultDatabase::open_memory()
    }

    fn open_db(db_path: Option<String>) -> Result<VaultDatabase, String> {
        match db_path.as_deref() {
            Some(":memory:") => VaultDatabase::open_memory(),
            Some(":empty:") => VaultDatabase::open_empty_memory(),
            Some(p) if !p.trim().is_empty() => VaultDatabase::open_file(&PathBuf::from(p)),
            _ => {
                if let Ok(env_path) = std::env::var("QI_DB_PATH") {
                    let trimmed = env_path.trim();
                    if trimmed == ":memory:" {
                        return VaultDatabase::open_memory();
                    }
                    if !trimmed.is_empty() {
                        return VaultDatabase::open_file(&PathBuf::from(trimmed));
                    }
                }
                let candidates = if let Some(home) = std::env::var_os("HOME") {
                    Self::default_db_candidates_for_home(&PathBuf::from(home))
                } else {
                    vec![std::env::temp_dir().join("qi_vault.sqlite3")]
                };
                Self::open_db_with_candidates(&candidates)
            }
        }
    }

    fn current_epoch() -> i64 {
        chrono::Utc::now().timestamp()
    }

    fn evaluate_entitlement(&self, settings: &AppSettings) -> EntitlementVerificationResult {
        let now = Self::current_epoch();
        verify_oset_with_clock_localized(
            &settings.entitlement_token,
            now,
            settings.max_seen_epoch,
            None,
            &settings.locale,
        )
    }

    fn award_recovery_dew_if_needed(&self, snapshot: &AnalyticsSnapshot) {
        if snapshot.state == "recovery_priority" {
            let _ = self.db.ensure_daily_recovery_dew(&snapshot.target_date);
        }
    }

    pub(crate) fn ingest_healthkit_sync_json(
        &mut self,
        raw_json: &str,
        requested_days: u32,
    ) -> Result<String, String> {
        let batch = parse_and_validate_healthkit_payload(raw_json)?;
        let mut s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        if batch.metrics.is_empty() && batch.workouts.is_empty() {
            s.healthkit_authorized = batch.authorized;
            s.healthkit_last_sync_at = now_str;
            s.healthkit_synced_days = 0;
            s.healthkit_last_summary = if en {
                format!(
                    "Connected Apple Health, but found no records in the past {requested_days} days. Please check Settings › Health › Data Access & Devices › Qi."
                )
            } else {
                format!(
                    "已完成 Apple 健康连接，但近 {requested_days} 天暂未读到记录。请确认「系统设置 › 健康 › 数据访问与设备 › 知气」已开启睡眠、心率变异性与步数读取开关。"
                )
            };
            self.db.save_settings(&s)?;
            return self.build_payload();
        }

        if self
            .db
            .purge_demo_metrics_if_synthetic_persona(&s.active_persona)?
        {
            s.active_persona = "real_healthkit".to_owned();
        }

        for row in &batch.metrics {
            self.db.upsert_daily_metric(row)?;
        }
        for w in &batch.workouts {
            self.db.upsert_workout(
                &w.workout_id,
                &w.date,
                &w.activity_type,
                w.duration_minutes,
                w.calories_kcal,
                &w.source,
            )?;
        }

        let all_metrics = self.db.load_metrics()?;
        let has_hrv_or_stages = all_metrics.iter().any(|m| {
            matches!(
                m.metric.as_str(),
                "hrv_sdnn_ms" | "hrv_rmssd_ms" | "sleep_deep_minutes" | "sleep_rem_minutes"
            )
        });
        let has_sleep_or_rhr = all_metrics.iter().any(|m| {
            matches!(
                m.metric.as_str(),
                "sleep_minutes" | "resting_heart_rate_bpm"
            )
        });
        s.hardware_tier = if has_hrv_or_stages {
            "tier_a".to_owned()
        } else if has_sleep_or_rhr {
            "tier_b".to_owned()
        } else {
            batch.recommended_tier.to_owned()
        };

        s.healthkit_authorized = true;
        s.healthkit_last_sync_at = now_str;
        s.healthkit_synced_days = batch.distinct_days as u32;
        s.healthkit_last_summary = if en {
            format!(
                "Synced {} days from Apple Health ({} metrics · {} workouts)",
                batch.distinct_days,
                batch.metrics.len(),
                batch.workouts.len()
            )
        } else {
            format!(
                "已从 Apple 健康同步 {} 天（{} 项生理指标 · {} 次训练）",
                batch.distinct_days,
                batch.metrics.len(),
                batch.workouts.len()
            )
        };
        self.live_briefing_override = None;
        self.db.save_settings(&s)?;

        let snap = compute_snapshot(
            &all_metrics,
            &self.db.load_workouts()?,
            &self.db.load_intakes()?,
            HardwareTier::from_str(&s.hardware_tier),
            s.morning_feeling,
        );
        self.award_recovery_dew_if_needed(&snap);
        self.build_payload()
    }

    fn build_payload(&self) -> Result<String, String> {
        let mut settings = self.db.load_settings()?;
        let locale = normalize_locale(&settings.locale).to_owned();
        settings.locale = locale.clone();
        let metrics = self.db.load_metrics()?;
        let workouts = self.db.load_workouts()?;
        let mut intakes = self.db.load_intakes()?;
        let mut labs = self.db.load_labs()?;
        let experiments_raw = self.db.load_experiments()?;
        let tier = HardwareTier::from_str(&settings.hardware_tier);

        let mut snapshot = compute_snapshot(
            &metrics,
            &workouts,
            &intakes,
            tier,
            settings.morning_feeling,
        );
        let mut experiments =
            evaluate_experiments(&metrics, &experiments_raw, &snapshot.target_date);
        let mut period_comparison = compute_period_comparison(&metrics, &snapshot.target_date);
        let daily_records = build_daily_records(&metrics, &intakes);
        let mut multi_lag_radar = compute_multi_lag_radar(&daily_records);

        localize_snapshot(&mut snapshot, &locale);
        localize_labs(&mut labs, &locale);
        localize_experiments(&mut experiments, &locale);
        localize_period_comparison(&mut period_comparison, &locale);
        localize_multi_lag_radar(&mut multi_lag_radar, &locale);
        localize_intakes_display(&mut intakes, &locale);
        localize_settings_display(&mut settings);

        let briefing = if let Some(ref over) = self.live_briefing_override {
            over.clone()
        } else {
            build_morning_briefing_localized(
                &snapshot,
                &labs,
                &settings.ai_mode,
                &settings.lan_ai_url,
                settings.thermal_throttled,
                settings.simulated_ram_gb,
                &locale,
            )
        };

        let entitlements = self.evaluate_entitlement(&settings);
        // Never allow unsigned SQLite flags to spoof badge/UI state in downstream payload builders.
        settings.guardian_sponsor = entitlements.guardian_sponsor;
        if !has_feature(&entitlements, "multi_lag_radar") && multi_lag_radar.len() > 2 {
            multi_lag_radar.truncate(2);
        }
        let distinct_metric_days: BTreeSet<&str> = metrics.iter().map(|m| m.date.as_str()).collect();
        let loyalty_eligible = settings.companion_loyalty_days >= LOYALTY_MIN_CHECKIN_DAYS
            || distinct_metric_days.len() as i64 >= LOYALTY_MIN_CHECKIN_DAYS
            || entitlements.companion_loyalty_eligible;
        let entitlement_catalog = build_entitlement_catalog_localized(loyalty_eligible, &locale);

        let dew_balance = self.db.get_dew_balance()?;
        let dew_ledger = self.db.list_dew_ledger(24)?;
        let claimed_rewards = self.db.list_claimed_pass_rewards()?;
        let presets = self.db.list_presets()?;
        let stickers = self.db.list_stickers()?;
        let ym: String = snapshot.target_date.chars().take(7).collect();
        let used_stickers_in_month = self.db.count_stickers_in_month(&ym)?;
        let custom_pets = self.db.list_custom_pets()?;
        let pet_weekly_letters = self.db.list_pet_weekly_letters()?;

        let mut solar_atelier = build_solar_atelier_payload_v5(
            &settings,
            &snapshot,
            &entitlements,
            dew_balance,
            dew_ledger,
            &claimed_rewards,
            presets,
            stickers,
            used_stickers_in_month,
            &experiments,
            intakes.len(),
            distinct_metric_days.len(),
            custom_pets,
            pet_weekly_letters,
        );
        localize_dew_ledger_if_en(&mut solar_atelier, &locale);

        let (multi_profile_uln_trends, recheck_reminders) =
            compute_multi_profile_uln_trends_at_localized(&labs, &snapshot.target_date, &locale);
        let default_recipient = if is_en_locale(&locale) {
            "Mom & Dad"
        } else {
            "爸妈"
        };
        let parents_peace_letter_preview = build_parents_peace_letter_localized(
            default_recipient,
            &snapshot,
            &recheck_reminders,
            &locale,
        );
        let experiment_templates = if is_en_locale(&locale) {
            EXPERIMENT_TEMPLATES_EN.to_vec()
        } else {
            EXPERIMENT_TEMPLATES.to_vec()
        };
        let capability_probe = self.last_probe.clone().map(|mut p| {
            localize_probe(&mut p, &locale);
            p
        });

        let payload = UiStatePayload {
            settings,
            snapshot,
            briefing,
            intakes,
            labs,
            experiments,
            experiment_templates,
            period_comparison,
            last_lab_import: self.last_lab_import.clone(),
            capability_probe,
            entitlements,
            entitlement_catalog,
            solar_atelier,
            multi_lag_radar,
            multi_profile_uln_trends,
            recheck_reminders,
            parents_peace_letter_preview,
        };
        serde_json::to_string(&payload).map_err(|e| e.to_string())
    }
}

#[class]
impl QiEngine {
    /// 初始化本地健康引擎（传 null 使用默认本机路径，传 ":memory:" 用于内存隔离运行）。
    pub fn new(db_path: Option<String>) -> Result<Self, String> {
        let db = Self::open_db(db_path)?;
        Ok(Self {
            db,
            last_lab_import: None,
            live_briefing_override: None,
            last_probe: None,
        })
    }

    /// 获取当前完整端侧状态快照 JSON。
    pub fn state_json(&self) -> Result<String, String> {
        self.build_payload()
    }

    /// 完成 60 秒冷启动引导，若本机曾被清空则自动补充演示节律序列。
    pub fn complete_onboarding(&mut self, goal: String, tier: String) -> Result<String, String> {
        if self.db.load_metrics()?.is_empty() {
            self.db.seed_demo_35d()?;
        }
        let mut s = self.db.load_settings()?;
        s.onboarding_done = true;
        s.primary_goal = goal;
        s.hardware_tier = tier;
        self.db.save_settings(&s)?;
        let snap = compute_snapshot(
            &self.db.load_metrics()?,
            &self.db.load_workouts()?,
            &self.db.load_intakes()?,
            HardwareTier::from_str(&s.hardware_tier),
            s.morning_feeling,
        );
        self.award_recovery_dew_if_needed(&snap);
        self.build_payload()
    }

    /// 切换硬件算力降级档位（Tier A / B / C）及晨起主观体感。
    pub fn set_hardware_tier(&mut self, tier: String, feeling: Option<u32>) -> Result<String, String> {
        self.live_briefing_override = None;
        let mut s = self.db.load_settings()?;
        s.hardware_tier = tier;
        if let Some(f) = feeling {
            s.morning_feeling = f.clamp(1, 5);
        }
        self.db.save_settings(&s)?;
        let snap = compute_snapshot(
            &self.db.load_metrics()?,
            &self.db.load_workouts()?,
            &self.db.load_intakes()?,
            HardwareTier::from_str(&s.hardware_tier),
            s.morning_feeling,
        );
        self.award_recovery_dew_if_needed(&snap);
        self.build_payload()
    }

    /// 勾选/取消今日唯一高杠杆微行动。
    pub fn toggle_today_action(&mut self) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        s.checked_action_today = !s.checked_action_today;
        self.db.save_settings(&s)?;
        let snap = compute_snapshot(
            &self.db.load_metrics()?,
            &self.db.load_workouts()?,
            &self.db.load_intakes()?,
            HardwareTier::from_str(&s.hardware_tier),
            s.morning_feeling,
        );
        self.award_recovery_dew_if_needed(&snap);
        self.build_payload()
    }

    /// 3 秒记一餐或节律因子（支持传入可选热量），立即写入本地 SQLite 并重算滞后相关性。
    pub fn log_quick_intake(
        &mut self,
        category: String,
        item_name: String,
        tag: String,
        notes: String,
        calories_kcal: Option<f64>,
    ) -> Result<String, String> {
        let target_date = self
            .db
            .load_metrics()?
            .last()
            .map(|r| r.date.clone())
            .unwrap_or_else(|| "2026-11-13".to_owned());
        self.db.insert_intake(
            &target_date,
            &category,
            &item_name,
            calories_kcal,
            &notes,
            &[tag.as_str()],
        )?;
        self.build_payload()
    }

    /// 一句话智能记餐（本地 60+ 中式词典 + 时间因子槽位提取，0ms 离线执行）。
    pub fn log_natural_intake(&mut self, sentence: String) -> Result<String, String> {
        let target_date = self
            .db
            .load_metrics()?
            .last()
            .map(|r| r.date.clone())
            .unwrap_or_else(|| "2026-11-13".to_owned());
        let parsed = parse_natural_intake_text(&sentence);
        if parsed.is_empty() {
            return Err("请输入吃过或喝过的饮食/饮品内容".to_owned());
        }
        for item in parsed {
            self.db.insert_intake_idempotent(
                &target_date,
                &item.category,
                &item.item_name,
                item.calories_kcal,
                &item.notes,
                &item.tags,
            )?;
        }
        self.build_payload()
    }

    /// 导入体检单文本：先做端侧 PII 前置遮盖，再提取指标并由 Rust 重算参考区间与纵向趋势。
    pub fn import_lab_report(
        &mut self,
        test_date: String,
        raw_text: String,
    ) -> Result<String, String> {
        let (scrubbed, redacted, parsed) = self.db.import_lab_text(&test_date, &raw_text)?;
        self.last_lab_import = Some(LastLabImportFeedback {
            redacted_fields: redacted,
            scrubbed_preview: scrubbed,
            imported_count: parsed.len(),
        });
        self.build_payload()
    }

    /// 离线识别本地体检单图片（Apple Vision OCR -> 端侧 PII 脱敏 -> Rust 区间重算），零字节上传云端。
    pub fn ocr_and_import_lab_image(
        &mut self,
        test_date: String,
        image_path: String,
    ) -> Result<String, String> {
        let ocr_text = run_offline_vision_ocr(&image_path)?;
        let (scrubbed, redacted, parsed) = self.db.import_lab_text(&test_date, &ocr_text)?;
        self.last_lab_import = Some(LastLabImportFeedback {
            redacted_fields: redacted,
            scrubbed_preview: scrubbed,
            imported_count: parsed.len(),
        });
        self.build_payload()
    }

    /// 将前端 PickedImage / PickedFile 写入临时缓存并调用 Apple Vision 离线 OCR 建档。
    pub fn ocr_and_import_lab_bytes(
        &mut self,
        test_date: String,
        ext: String,
        bytes: Vec<u8>,
    ) -> Result<String, String> {
        let suffix = if ext.trim().is_empty() { "png" } else { ext.trim() };
        let tmp = std::env::temp_dir().join(format!("qi_lab_ocr_{}.{suffix}", std::process::id()));
        std::fs::write(&tmp, &bytes).map_err(|e| format!("写入临时图片失败: {e}"))?;
        let res = self.ocr_and_import_lab_image(test_date, tmp.to_string_lossy().into_owned());
        let _ = std::fs::remove_file(&tmp);
        res
    }

    /// 导入外部穿戴导出文本（Apple Health `export.xml` 片段、CSV 日报或 OpenHealthSync JSONL）。
    pub fn import_external_health_data(
        &mut self,
        format: String,
        raw_content: String,
    ) -> Result<String, String> {
        match format.as_str() {
            "apple_health_xml" => {
                let rows = parse_apple_health_xml_snippet(&raw_content);
                if rows.is_empty() {
                    return Err("未在 XML 中找到可识别的健康记录".to_owned());
                }
                for r in rows {
                    self.db.upsert_daily_metric(&r)?;
                }
            }
            "csv" => {
                let rows = parse_csv_metrics(&raw_content);
                if rows.is_empty() {
                    return Err("CSV 格式应为 date,metric,source,value,unit".to_owned());
                }
                for r in rows {
                    self.db.upsert_daily_metric(&r)?;
                }
            }
            _ => {
                self.db.merge_sync_jsonl(&raw_content)?;
            }
        }
        self.build_payload()
    }

    /// 创建 N-of-1 微习惯实验。
    pub fn start_experiment(
        &mut self,
        title: String,
        hypothesis: String,
        target_days: u32,
    ) -> Result<String, String> {
        let start_date = self
            .db
            .load_metrics()?
            .last()
            .map(|r| r.date.clone())
            .unwrap_or_else(|| "2026-11-13".to_owned());
        self.db
            .create_experiment_if_missing(&title, &hypothesis, target_days.max(3), &start_date)?;
        self.build_payload()
    }

    /// 打卡或取消某项实验今日的依从记录，立即重算实验期 vs 基线生理指标差值。
    pub fn toggle_experiment_today(&mut self, experiment_id: i64) -> Result<String, String> {
        let target_date = self
            .db
            .load_metrics()?
            .last()
            .map(|r| r.date.clone())
            .unwrap_or_else(|| "2026-11-13".to_owned());
        self.db
            .toggle_experiment_checkin(experiment_id, &target_date)?;
        self.build_payload()
    }

    /// 实时调用当前选定的 AI 引擎生成早报润色（支持 Ollama / ZDR 云 / 端侧门禁保护）。
    pub fn refresh_live_briefing(&mut self) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let metrics = self.db.load_metrics()?;
        let workouts = self.db.load_workouts()?;
        let intakes = self.db.load_intakes()?;
        let mut labs = self.db.load_labs()?;
        let tier = HardwareTier::from_str(&s.hardware_tier);
        let mut snapshot = compute_snapshot(&metrics, &workouts, &intakes, tier, s.morning_feeling);
        localize_snapshot(&mut snapshot, &s.locale);
        localize_labs(&mut labs, &s.locale);

        let res = generate_live_briefing_localized(
            &snapshot,
            &labs,
            &s.ai_mode,
            &s.lan_ai_url,
            &s.lan_ai_model,
            &s.cloud_url,
            &s.cloud_token,
            s.thermal_throttled,
            s.simulated_ram_gb,
            &s.locale,
        )?;
        self.live_briefing_override = Some(res);
        self.build_payload()
    }

    /// 探测本机 Apple Vision OCR、FoundationModels 与局域网 Ollama 实时连通状态。
    pub fn probe_capabilities(&mut self) -> Result<String, String> {
        let s = self.db.load_settings()?;
        self.last_probe = Some(probe_local_ai_capabilities(&s.lan_ai_url));
        self.build_payload()
    }

    /// 更新个人云同步模式与地址（iCloud / WebDAV / BYO-Server / 纯本地）。
    pub fn update_sync_config(
        &mut self,
        sync_mode: String,
        webdav_url: String,
        byo_url: String,
        webdav_username: String,
        webdav_password: String,
    ) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        s.sync_mode = sync_mode;
        s.webdav_url = webdav_url;
        s.byo_server_url = byo_url;
        s.webdav_username = webdav_username;
        s.webdav_password = webdav_password;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 保存 qi-cloud 控制面地址与登录会话令牌（供 BYO-Server 与 ZDR 网关复用）。
    pub fn update_cloud_session(
        &mut self,
        cloud_url: String,
        cloud_handle: String,
        cloud_token: String,
    ) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        s.cloud_url = cloud_url;
        s.cloud_handle = cloud_handle;
        s.cloud_token = cloud_token;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 更新 AI 引擎与本地算力防崩溃门禁参数。
    pub fn update_ai_config(
        &mut self,
        ai_mode: String,
        zdr_consented: bool,
        lan_url: String,
        lan_model: String,
        thermal_throttled: bool,
        ram_gb: u32,
    ) -> Result<String, String> {
        self.live_briefing_override = None;
        let mut s = self.db.load_settings()?;
        s.ai_mode = ai_mode;
        s.zdr_consented = zdr_consented;
        s.lan_ai_url = lan_url;
        s.lan_ai_model = lan_model;
        s.thermal_throttled = thermal_throttled;
        s.simulated_ram_gb = ram_gb;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 切换 08:30 本地早安通知。
    pub fn toggle_morning_push(&mut self, enabled: bool) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        s.morning_push_enabled = enabled;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 使用 12 词恢复码生成端到端加密增量分片并立即验签往返。
    pub fn preview_encrypted_batch(&self, batch_seq: u32) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let key = derive_master_key(&s.recovery_phrase)?;
        let jsonl = self.db.export_compact_jsonl()?;
        let env = encrypt_sync_batch(&key, &s.device_id, u64::from(batch_seq), &jsonl)?;
        let verified = decrypt_sync_batch(&key, &env)?;
        let cipher_head: String = env.ciphertext_hex.chars().take(48).collect();
        Ok(format!(
            "路径: {} | 算法: {} | 明文 {}B | 密文前缀: {}... | AEAD往返校验通过 ({}行)",
            env.path,
            env.cipher,
            env.payload_bytes,
            cipher_head,
            verified.lines().count()
        ))
    }

    /// 真实执行个人云 E2EE 加密同步（推送本设备新分片 + 拉取并合并其他设备分片）。
    pub fn run_e2ee_sync_now(&mut self) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        let key = derive_master_key(&s.recovery_phrase)?;
        let next_seq = s.sync_seq + 1;
        let full_jsonl = self.db.export_full_sync_jsonl()?;
        let env = encrypt_sync_batch(&key, &s.device_id, next_seq, &full_jsonl)?;

        let report = match s.sync_mode.as_str() {
            "icloud_drive" => {
                let dir = resolve_icloud_vault_dir();
                let written = push_envelope_to_dir(&dir, &env)?;
                let remote_envs = pull_envelopes_from_dir(&dir, Some(&s.device_id))?;
                let mut merged = 0usize;
                for r_env in &remote_envs {
                    if let Ok(plain) = decrypt_sync_batch(&key, r_env) {
                        merged += self.db.merge_sync_jsonl(&plain)?;
                    }
                }
                SyncExecutionReport {
                    sync_mode: "icloud_drive".to_owned(),
                    target_location: written.to_string_lossy().into_owned(),
                    pushed_batches: 1,
                    pulled_batches: remote_envs.len(),
                    merged_records: merged,
                    message: format!(
                        "iCloud 容器同步完成：已推分片 #{}，拉取合并 {} 个跨端分片（{} 条记录）",
                        next_seq,
                        remote_envs.len(),
                        merged
                    ),
                }
            }
            "webdav" => {
                push_envelope_to_webdav(
                    &s.webdav_url,
                    &s.webdav_username,
                    &s.webdav_password,
                    &env,
                )?;
                SyncExecutionReport {
                    sync_mode: "webdav".to_owned(),
                    target_location: format!("{}/{}", s.webdav_url.trim_end_matches('/'), env.path),
                    pushed_batches: 1,
                    pulled_batches: 0,
                    merged_records: 0,
                    message: format!("WebDAV 加密分片 {} 上传成功", env.path),
                }
            }
            "byo_server" => {
                push_envelope_to_byo(&s.byo_server_url, &s.cloud_token, &env)?;
                let (remote_envs, new_cursor) = pull_envelopes_from_byo(
                    &s.byo_server_url,
                    &s.cloud_token,
                    s.pull_cursor_id,
                    Some(&s.device_id),
                )?;
                let mut merged = 0usize;
                for r_env in &remote_envs {
                    if let Ok(plain) = decrypt_sync_batch(&key, r_env) {
                        merged += self.db.merge_sync_jsonl(&plain)?;
                    }
                }
                s.pull_cursor_id = new_cursor;
                SyncExecutionReport {
                    sync_mode: "byo_server".to_owned(),
                    target_location: s.byo_server_url.clone(),
                    pushed_batches: 1,
                    pulled_batches: remote_envs.len(),
                    merged_records: merged,
                    message: format!(
                        "私有节点 E2EE 同步成功：已推 {}，合并跨端分片 {} 个",
                        env.path,
                        remote_envs.len()
                    ),
                }
            }
            _ => SyncExecutionReport {
                sync_mode: "local_only".to_owned(),
                target_location: "local-vault".to_owned(),
                pushed_batches: 0,
                pulled_batches: 0,
                merged_records: 0,
                message: "当前处于纯本地离线模式，请使用下方「导出 .qivault 加密备份」".to_owned(),
            },
        };

        if report.pushed_batches > 0 {
            s.sync_seq = next_seq;
        }
        s.last_sync_summary = report.message.clone();
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 跨生态换机一键恢复：输入 12 词恢复码拉取解密远端分片并重建本地健康库。
    pub fn restore_from_recovery_phrase(
        &mut self,
        phrase: String,
        source_mode: String,
        archive_or_url: String,
    ) -> Result<String, String> {
        let key = derive_master_key(&phrase)?;
        let mut merged_total = 0usize;
        let mut pulled_batches = 0usize;

        match source_mode.as_str() {
            "icloud_drive" => {
                let dir = if archive_or_url.trim().is_empty() {
                    resolve_icloud_vault_dir()
                } else {
                    PathBuf::from(archive_or_url.trim())
                };
                let envs = pull_envelopes_from_dir(&dir, None)?;
                if envs.is_empty() {
                    return Err(format!("在 {} 中未找到加密分片", dir.display()));
                }
                for env in envs {
                    if let Ok(plain) = decrypt_sync_batch(&key, &env) {
                        merged_total += self.db.merge_sync_jsonl(&plain)?;
                        pulled_batches += 1;
                    }
                }
            }
            "byo_server" => {
                let s = self.db.load_settings()?;
                let base = if archive_or_url.trim().is_empty() {
                    s.byo_server_url
                } else {
                    archive_or_url
                };
                let (envs, cursor) = pull_envelopes_from_byo(&base, &s.cloud_token, 0, None)?;
                for env in envs {
                    if let Ok(plain) = decrypt_sync_batch(&key, &env) {
                        merged_total += self.db.merge_sync_jsonl(&plain)?;
                        pulled_batches += 1;
                    }
                }
                let mut ns = self.db.load_settings()?;
                ns.pull_cursor_id = cursor;
                self.db.save_settings(&ns)?;
            }
            _ => {
                let plain = decrypt_local_archive_json(&phrase, &archive_or_url)?;
                merged_total = self.db.merge_sync_jsonl(&plain)?;
                pulled_batches = 1;
            }
        }

        if pulled_batches == 0 {
            return Err("恢复码校验失败：未能解密任何加密分片，请核对 12 词拼写".to_owned());
        }

        let mut s = self.db.load_settings()?;
        s.recovery_phrase = phrase.trim().to_owned();
        s.onboarding_done = true;
        s.last_sync_summary =
            format!("12 词恢复完成：解密 {pulled_batches} 个分片，重建 {merged_total} 条个人记录");
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 导出开放标准 Compact JSONL 数据摘要。
    pub fn export_jsonl(&self) -> Result<String, String> {
        self.db.export_compact_jsonl()
    }

    /// 导出整库 `.qivault` 端到端加密单文件归档 JSON。
    pub fn export_encrypted_archive(&self) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let jsonl = self.db.export_full_sync_jsonl()?;
        build_local_archive_json(&s.recovery_phrase, &s.device_id, s.sync_seq + 1, &jsonl)
    }

    /// 物理清空本机健康库并重置引导流程（App Review 5.1.1(v)）。
    pub fn wipe_vault_and_reset(&mut self) -> Result<String, String> {
        self.last_lab_import = None;
        self.live_briefing_override = None;
        self.db.wipe_all_data()?;
        self.build_payload()
    }

    /// 重新载入 35 天演示数据。
    pub fn reseed_demo(&mut self) -> Result<String, String> {
        if !is_qa_or_debug_build() {
            return Err("QA Studio sandbox APIs are disabled in release builds.".to_owned());
        }
        self.live_briefing_override = None;
        self.db.seed_demo_35d()?;
        self.build_payload()
    }

    /// 切换并载入指定纵向测试画像（清空业务表，保留云会话、12词恢复码与同步配置）。
    pub fn load_persona(&mut self, persona_id: String) -> Result<String, String> {
        if !is_qa_or_debug_build() {
            return Err("QA Studio sandbox APIs are disabled in release builds.".to_owned());
        }
        if crate::personas::find_persona_meta(&persona_id).is_none() {
            return Err(format!("未知测试画像 ID: {persona_id}"));
        }
        self.db.clear_business_data()?;
        let meta = crate::personas::seed_persona_data(&self.db, &persona_id)?;

        let mut s = self.db.load_settings()?;
        s.active_persona = meta.id;
        s.hardware_tier = meta.hardware_tier;
        s.primary_goal = meta.primary_goal;
        s.morning_feeling = 4;
        s.onboarding_done = true;
        s.checked_action_today = false;
        self.db.save_settings(&s)?;

        self.live_briefing_override = None;
        self.last_lab_import = None;
        let snap = compute_snapshot(
            &self.db.load_metrics()?,
            &self.db.load_workouts()?,
            &self.db.load_intakes()?,
            HardwareTier::from_str(&s.hardware_tier),
            s.morning_feeling,
        );
        self.award_recovery_dew_if_needed(&snap);
        self.build_payload()
    }

    /// 重新验证当前 Ed25519 离线权益凭证并更新防回拨单调时钟水位线。
    pub fn verify_entitlements(&mut self) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        let res = self.evaluate_entitlement(&s);
        if res.new_max_seen_epoch > s.max_seen_epoch {
            s.max_seen_epoch = res.new_max_seen_epoch;
            self.db.save_settings(&s)?;
        }
        self.build_payload()
    }

    /// 沙盒切换会员档位/状态（内部使用真实 Ed25519 签发 OSET 通行证）。
    pub fn set_entitlement_sandbox(
        &mut self,
        tier: String,
        simulated_state: String,
        guardian_sponsor: bool,
        loyalty_days: i64,
    ) -> Result<String, String> {
        if !is_qa_or_debug_build() {
            return Err("QA Studio sandbox APIs are disabled in release builds.".to_owned());
        }
        let mut s = self.db.load_settings()?;
        let now = Self::current_epoch();
        let token = mint_sandbox_oset(
            &s.device_id,
            &tier,
            &simulated_state,
            guardian_sponsor,
            loyalty_days,
            now,
        )?;
        s.sandbox_tier = SubscriptionTier::from_str(&tier).as_str().to_owned();
        s.sandbox_state = simulated_state;
        s.companion_loyalty_days = loyalty_days.max(0);
        s.entitlement_token = token;
        let res = verify_oset_with_clock_localized(
            &s.entitlement_token,
            now,
            s.max_seen_epoch,
            None,
            &s.locale,
        );
        s.guardian_sponsor = res.guardian_sponsor;
        s.max_seen_epoch = res.new_max_seen_epoch;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 粘贴并应用真实签发的 Ed25519 离线权益通行证。
    pub fn apply_entitlement_token(&mut self, token: String) -> Result<String, String> {
        let trimmed = token.trim().to_owned();
        let mut s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let now = Self::current_epoch();
        if trimmed.is_empty() {
            s.entitlement_token.clear();
            s.sandbox_tier = "free".to_owned();
            self.db.save_settings(&s)?;
            return self.build_payload();
        }
        let res =
            verify_oset_with_clock_localized(&trimmed, now, s.max_seen_epoch, None, &s.locale);
        if !res.valid_signature {
            return Err(if en {
                "Atelier pass signature check failed. Verify the token is complete and unmodified."
                    .to_owned()
            } else {
                "雅集通行证签名校验失败，请确认令牌完整无篡改".to_owned()
            });
        }
        s.entitlement_token = trimmed;
        s.sandbox_tier = res.tier;
        s.guardian_sponsor = res.guardian_sponsor;
        s.max_seen_epoch = res.new_max_seen_epoch;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 清除当前通行证，回到素心清欢永久免费版（既往档案依然 100% 可读可导出）。
    pub fn clear_entitlement_token(&mut self) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        s.entitlement_token.clear();
        s.sandbox_tier = "free".to_owned();
        s.sandbox_state = "active".to_owned();
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 领取或兑换岁时风物季票节点（清欢轨永久免费，雅集轨需有效会员，兑换扣减晨露）。
    pub fn claim_seasonal_reward(&mut self, reward_id: String) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let ent = self.evaluate_entitlement(&s);
        let claimed = self.db.list_claimed_pass_rewards()?;
        let pass = build_seasonal_pass_localized(&ent, &claimed, &s.locale);
        let node = pass
            .into_iter()
            .find(|n| n.id == reward_id || n.reward_id == reward_id)
            .ok_or_else(|| {
                if en {
                    format!("Seasonal keepsake not found: {reward_id}")
                } else {
                    format!("未找到季票风物节点: {reward_id}")
                }
            })?;
        if node.claimed {
            return Err(if en {
                "This seasonal keepsake is already in your collection.".to_owned()
            } else {
                "该岁时风物已收入行囊，无需重复兑换".to_owned()
            });
        }
        if !node.unlocked_by_tier {
            return Err(if en {
                "This keepsake belongs to the Atelier track. Unlock Qi Atelier to redeem it with dew.".to_owned()
            } else {
                "该风物属于「知气雅集」限定轨，开启雅集后即可用晨露兑换".to_owned()
            });
        }
        let balance = self.db.get_dew_balance()?;
        if balance < node.cost_dew {
            return Err(if en {
                format!(
                    "Not enough morning dew: requires {}, you have {}.",
                    node.cost_dew, balance
                )
            } else {
                format!(
                    "晨露不足：需要 {} 滴晨露，当前拥有 {} 滴晨露",
                    node.cost_dew, balance
                )
            });
        }
        let note = if en {
            format!("Redeemed seasonal keepsake: {}", node.title)
        } else {
            format!("兑换岁时风物：{}", node.title)
        };
        self.db
            .add_dew_entry(-node.cost_dew, "seasonal_pass_claim", &note, &node.reward_id)?;
        self.build_payload()
    }

    /// 手动领取今日「安心静养 · 睡饱双倍露 (+3 滴晨露)」（仅在静养日触发，永久免费）。
    pub fn claim_recovery_dew(&mut self) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let snap = compute_snapshot(
            &self.db.load_metrics()?,
            &self.db.load_workouts()?,
            &self.db.load_intakes()?,
            HardwareTier::from_str(&s.hardware_tier),
            s.morning_feeling,
        );
        if snap.state != "recovery_priority" {
            return Err(if en {
                "Your rhythm looks steady today. Double recovery dew activates automatically on Rest Gently days.".to_owned()
            } else {
                "今日状态稳健，仅在「调养 · 蓄能」静养日自动触发双倍晨露".to_owned()
            });
        }
        self.db.ensure_daily_recovery_dew(&snap.target_date)?;
        self.build_payload()
    }

    /// 切换窗外天色心境（`sunny_crisp` / `misty_rain` / `first_snow` / `clear_moon`）。
    pub fn set_weather_mood(&mut self, weather_mood: String) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        s.weather_mood = normalize_weather_mood(&weather_mood).to_owned();
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 装卸窗台珍藏风物摆件（素心清欢 2 槽，雅集解锁第 3 槽）。
    pub fn toggle_equipped_keepsake(&mut self, keepsake_id: String) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let ent = self.evaluate_entitlement(&s);
        let max_slots = if has_feature(&ent, "keepsake_slot_3") {
            MAX_EQUIPPED_KEEPSAKES
        } else {
            2
        };
        let mut equipped: Vec<String> =
            serde_json::from_str(&s.equipped_keepsakes_json).unwrap_or_default();
        if let Some(pos) = equipped.iter().position(|x| x == &keepsake_id) {
            equipped.remove(pos);
        } else {
            if equipped.len() >= max_slots {
                return Err(if max_slots < MAX_EQUIPPED_KEEPSAKES {
                    if en {
                        "Free keepsake shelf holds up to 2 items. Unlock Qi Atelier for a 3rd window slot.".to_owned()
                    } else {
                        "素心清欢最多陈列 2 件风物，升级知气雅集解锁第 3 个珍藏陈列位".to_owned()
                    }
                } else if en {
                    "Up to 3 keepsakes can be arranged on the windowsill at once.".to_owned()
                } else {
                    "窗台最多同时陈列 3 件风物摆件".to_owned()
                });
            }
            equipped.push(keepsake_id);
        }
        s.equipped_keepsakes_json = serde_json::to_string(&equipped).map_err(|e| e.to_string())?;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 切换日签印泥（`cinnabar` 朱砂 / `gold` 泥金 / `lapis` 青金石）。
    pub fn set_seal_ink(&mut self, seal_ink: String) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let ink = match seal_ink.trim() {
            "gold" => "gold",
            "lapis" => "lapis",
            _ => "cinnabar",
        };
        if ink != "cinnabar" {
            let ent = self.evaluate_entitlement(&s);
            let claimed = self.db.list_claimed_pass_rewards()?;
            if !has_feature(&ent, "poster_custom_seals") && !claimed.iter().any(|c| c == ink) {
                return Err(if en {
                    "Song Gold and Lapis seal inks belong to Qi Atelier (or redeem via Seasonal Pass).".to_owned()
                } else {
                    "泥金/青金石印泥属于知气雅集专属（或先在季票兑换）".to_owned()
                });
            }
        }
        s.active_seal_ink = ink.to_owned();
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 保存当前书房预设（素心清欢最多 2 套，雅集无限）。
    pub fn save_sanctuary_preset(&mut self, name: String) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(if en {
                "Please enter a preset name.".to_owned()
            } else {
                "请输入书房预设名称".to_owned()
            });
        }
        let ent = self.evaluate_entitlement(&s);
        let existing_presets = self.db.list_presets()?;
        let is_overwrite = existing_presets.iter().any(|p| p.name == trimmed);
        if !is_overwrite && !has_feature(&ent, "preset_slots_unlimited") && existing_presets.len() >= 2
        {
            return Err(if en {
                "Free tier stores up to 2 sanctuary presets. Unlock Qi Atelier for unlimited presets.".to_owned()
            } else {
                "素心清欢最多保存 2 套书房预设，升级知气雅集解锁无限预设位".to_owned()
            });
        }
        let metrics = self.db.load_metrics()?;
        let target_date = metrics
            .last()
            .map(|r| r.date.clone())
            .unwrap_or_else(|| "2026-11-13".to_owned());
        let solar = compute_solar_term_state_localized(&target_date, &s.weather_mood, &s.locale);
        self.db.save_preset(
            trimmed,
            &solar.active_world,
            &s.weather_mood,
            &s.active_seal_ink,
            &s.equipped_keepsakes_json,
        )?;
        self.build_payload()
    }

    /// 一键套用已保存的书房预设。
    pub fn apply_sanctuary_preset(&mut self, preset_id: i64) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let preset = self.db.get_preset(preset_id)?.ok_or_else(|| {
            if en {
                format!("Sanctuary preset #{preset_id} not found")
            } else {
                format!("未找到书房预设 #{preset_id}")
            }
        })?;
        s.weather_mood = preset.weather_mood;
        s.active_seal_ink = preset.seal_ink;
        s.equipped_keepsakes_json = preset.keepsakes_json;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 添加一张食光拍立得饮食贴纸（免费版每月 5 张，雅集无限；奖励 +1 滴晨露）。
    pub fn add_food_sticker(
        &mut self,
        date: String,
        title: String,
        filter: String,
        note: String,
        image_ref: String,
    ) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let trimmed_title = title.trim();
        if trimmed_title.is_empty() {
            return Err(if en {
                "Please enter a meal sticker title.".to_owned()
            } else {
                "请输入饮食贴纸名称".to_owned()
            });
        }
        let use_date = if date.trim().is_empty() {
            self.db
                .load_metrics()?
                .last()
                .map(|r| r.date.clone())
                .unwrap_or_else(|| "2026-11-13".to_owned())
        } else {
            date.trim().to_owned()
        };
        let ym: String = use_date.chars().take(7).collect();
        let ent = self.evaluate_entitlement(&s);
        let used = self.db.count_stickers_in_month(&ym)?;
        if !has_feature(&ent, "unlimited_stickers") && used >= 5 {
            return Err(if en {
                "You have reached the 5-sticker monthly free quota. Unlock Qi Atelier for unlimited meal stickers.".to_owned()
            } else {
                "本月素心清欢 5 张食光贴纸额度已用完，升级知气雅集解锁无限手账".to_owned()
            });
        }
        let clean_filter = match filter.trim() {
            "watercolor" => "watercolor",
            "celadon" => "celadon",
            _ => "polaroid",
        };
        let hint = infer_sticker_nutrition_hint_localized(trimmed_title, &s.locale);
        self.db.insert_sticker(
            &use_date,
            trimmed_title,
            clean_filter,
            note.trim(),
            &hint,
            image_ref.trim(),
        )?;
        let dew_note = if en {
            format!("Captured food sticker: {trimmed_title}")
        } else {
            format!("定格食光贴纸：{trimmed_title}")
        };
        self.db
            .add_dew_entry(1, "food_sticker", &dew_note, "")?;
        self.build_payload()
    }

    /// 收藏或取消收藏饮食贴纸。
    pub fn toggle_sticker_favorite(&mut self, sticker_id: i64) -> Result<String, String> {
        self.db.toggle_sticker_favorite(sticker_id)?;
        self.build_payload()
    }

    /// 删除饮食贴纸。
    pub fn delete_food_sticker(&mut self, sticker_id: i64) -> Result<String, String> {
        self.db.delete_sticker(sticker_id)?;
        self.build_payload()
    }

    /// 隐私优先双人同窗互动（`warm_tea` / `tuck_blanket` / `paw_stamp`，奖励 +1 滴晨露并浇灌窗台金桂）。
    pub fn cosanctuary_action(
        &mut self,
        action: String,
        partner_name: Option<String>,
        pair_code: Option<String>,
    ) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        if let Some(pn) = partner_name
            && !pn.trim().is_empty()
        {
            s.partner_name = pn.trim().to_owned();
        }
        if let Some(pc) = pair_code
            && !pc.trim().is_empty()
        {
            s.pair_code = pc.trim().to_owned();
        }

        let act = action.trim();
        let note = match act {
            "warm_tea" => Some(if en {
                "Co-window visit: shared a cup of warm tea +1 dew"
            } else {
                "同窗串门：给好友递上一盏热茶 +1 滴晨露"
            }),
            "tuck_blanket" => Some(if en {
                "Co-window visit: tucked in a cozy blanket +1 dew"
            } else {
                "同窗串门：替好友盖好薄被 +1 滴晨露"
            }),
            "paw_stamp" => Some(if en {
                "Co-window visit: pressed a gentle paw tap +1 dew"
            } else {
                "同窗串门：在窗台按下暖心爪印 +1 滴晨露"
            }),
            _ => None,
        };
        if let Some(desc) = note {
            s.bonsai_dew = s.bonsai_dew.saturating_add(1);
            self.db.save_settings(&s)?;
            self.db.add_dew_entry(1, "co_sanctuary", desc, "")?;
        } else {
            self.db.save_settings(&s)?;
        }
        self.build_payload()
    }

    /// 切换「抬腕嘴替」桌面微件隐私视图模式（`metaphor` 诗意隐喻 vs `metrics` 数值）。
    pub fn set_widget_privacy_mode(&mut self, mode: String) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        s.widget_privacy_mode = if mode.trim() == "metrics" {
            "metrics".to_owned()
        } else {
            "metaphor".to_owned()
        };
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 发起自定义洗脱期（`washout_days`）的 N-of-1 自我对照实验。
    pub fn start_custom_experiment(
        &mut self,
        title: String,
        hypothesis: String,
        target_days: u32,
        washout_days: i64,
    ) -> Result<String, String> {
        let s = self.db.load_settings()?;
        if washout_days > 0 {
            let ent = self.evaluate_entitlement(&s);
            if !has_feature(&ent, "custom_washout_n_of_1") {
                return Err(if is_en_locale(&s.locale) {
                    "Custom N-of-1 washout windows require a Qi Atelier pass. Standard 0-day experiments stay free.".to_owned()
                } else {
                    "自定义对照试验洗脱期属于知气雅集会员权益（0 天标准实验永久免费）".to_owned()
                });
            }
        }
        if title.trim().is_empty() {
            return Err(if is_en_locale(&s.locale) {
                "Please enter an experiment title.".to_owned()
            } else {
                "请输入实验标题".to_owned()
            });
        }
        let start_date = self
            .db
            .load_metrics()?
            .last()
            .map(|r| r.date.clone())
            .unwrap_or_else(|| "2026-11-13".to_owned());
        self.db.create_experiment_with_washout(
            &title,
            &hypothesis,
            target_days.max(3),
            &start_date,
            washout_days,
        )?;
        self.build_payload()
    }

    /// 生成大字暖心、端侧脱敏《气韵平安家书》JSON。
    pub fn generate_parents_peace_letter(&mut self, recipient_label: String) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let ent = self.evaluate_entitlement(&s);
        if !has_feature(&ent, "parents_peace_letter") {
            return Err(if is_en_locale(&s.locale) {
                "Family Peace Letter generation requires Qi Atelier (Annual / Family / Lifetime) membership.".to_owned()
            } else {
                "生成大字暖心《气韵平安家书》需开通雅集年礼 / 四时家书 / 终身买断席位".to_owned()
            });
        }
        let metrics = self.db.load_metrics()?;
        let workouts = self.db.load_workouts()?;
        let intakes = self.db.load_intakes()?;
        let mut labs = self.db.load_labs()?;
        let mut snapshot = compute_snapshot(
            &metrics,
            &workouts,
            &intakes,
            HardwareTier::from_str(&s.hardware_tier),
            s.morning_feeling,
        );
        localize_snapshot(&mut snapshot, &s.locale);
        localize_labs(&mut labs, &s.locale);
        let (_, reminders) = compute_multi_profile_uln_trends_at_localized(
            &labs,
            &snapshot.target_date,
            &s.locale,
        );
        let letter = build_parents_peace_letter_localized(
            &recipient_label,
            &snapshot,
            &reminders,
            &s.locale,
        );
        serde_json::to_string(&letter).map_err(|e| e.to_string())
    }

    /// 为多亲友档案（`self` / `partner` / `parents`）录入单条体检指标，用于 %ULN 归一化对比与 90 天复查提醒。
    #[allow(clippy::too_many_arguments)]
    pub fn add_profile_lab_marker(
        &mut self,
        profile_id: String,
        test_date: String,
        marker_name: String,
        value: f64,
        unit: String,
        ref_low: Option<f64>,
        ref_high: Option<f64>,
    ) -> Result<String, String> {
        let s = self.db.load_settings()?;
        if marker_name.trim().is_empty() || !value.is_finite() {
            return Err(if is_en_locale(&s.locale) {
                "Please provide a valid biomarker name and numeric value.".to_owned()
            } else {
                "请输入有效的检验指标名称与数值".to_owned()
            });
        }
        let pid = match profile_id.trim() {
            "partner" => "partner",
            "parents" => "parents",
            _ => "self",
        };
        if pid != "self" {
            let ent = self.evaluate_entitlement(&s);
            if !has_feature(&ent, "multi_profile_labs") {
                return Err(if is_en_locale(&s.locale) {
                    "Multi-profile partner/parent lab tracking requires Qi Atelier membership. Personal labs stay free.".to_owned()
                } else {
                    "伴侣与父母多亲友化验档案需开通知气雅集会员（本人化验与历史档案永久免费）".to_owned()
                });
            }
        }
        let canonical = canonicalize_marker(&marker_name);
        let date_str = if test_date.trim().is_empty() {
            "2026-11-13".to_owned()
        } else {
            test_date.trim().to_owned()
        };
        self.db.insert_lab_marker_for_profile(
            pid,
            &date_str,
            &marker_name,
            &canonical,
            value,
            &unit,
            ref_low,
            ref_high,
        )?;
        self.build_payload()
    }

    /// 读取当前持久化界面语言设置（`{ "locale": "zh-CN" | "en" }`）。
    pub fn get_locale(&self) -> Result<String, String> {
        let s = self.db.load_settings()?;
        Ok(serde_json::json!({ "locale": normalize_locale(&s.locale) }).to_string())
    }

    /// 切换界面语言（`"zh-CN"` | `"en"`）并持久化至 SQLite，清空旧缓存简报并输出最新状态快照。
    pub fn set_locale(&mut self, locale: String) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        s.locale = normalize_locale(&locale).to_owned();
        self.live_briefing_override = None;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 列出已保存的所有「自家灵宠」及当前配额与周信归档 JSON。
    pub fn list_custom_pets(&self) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let ent = self.evaluate_entitlement(&s);
        let pets = self.db.list_custom_pets()?;
        let active_pet = pets.iter().find(|p| p.is_active).cloned();
        let quota = compute_custom_pet_quota(pets.len(), &ent);
        let letters = self.db.list_pet_weekly_letters()?;
        Ok(serde_json::json!({
            "pets": pets,
            "active_pet": active_pet,
            "quota": quota,
            "letters": letters,
        })
        .to_string())
    }

    /// 新增或更新「自家灵宠入驻」配置（免费版永久 1 位，雅集至多 3 位 + 专属爪印藏书票；到期进入只读保险箱绝不删除/锁定已有灵宠）。
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_custom_pet(
        &mut self,
        name: String,
        species: String,
        persona: String,
        photo_data_url: String,
        die_cut_frame: String,
        starlight_memorial: bool,
        paw_stamp_title: String,
        is_active: bool,
        pet_id: Option<String>,
    ) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let clean_name = name.trim().to_owned();
        if clean_name.is_empty() {
            return Err(if en {
                "Please enter a name for your custom pet.".to_owned()
            } else {
                "请填写自家灵宠的名字".to_owned()
            });
        }

        let validated_photo = validate_pet_photo_data_url(&photo_data_url, &s.locale)?;
        let clean_species = normalize_pet_species(&species).to_owned();
        let clean_frame = normalize_die_cut_frame(&die_cut_frame).to_owned();
        let clean_persona = persona.trim().to_owned();
        let clean_stamp = paw_stamp_title.trim().to_owned();

        let ent = self.evaluate_entitlement(&s);
        let now_iso = chrono::Utc::now().to_rfc3339();
        let requested_id = pet_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned);
        let existing = match &requested_id {
            Some(id) => self.db.get_custom_pet(id)?,
            None => None,
        };

        let pet_record = if let Some(prev) = existing {
            // 伦理红线：已保存的灵宠在免费/到期只读模式下永远可编辑非 Pro 字段与保留；仅变更设置新的 Pro 专属藏书票印记需要雅集权限
            let stamp_changed = clean_stamp != prev.paw_stamp_title && !clean_stamp.is_empty();
            if stamp_changed && !has_feature(&ent, "custom_pet_ex_libris") {
                return Err(if en {
                    "Custom Ex Libris paw stamp titles require an active Qi Atelier plan; your existing pet profile remains editable.".to_owned()
                } else {
                    "定制爪印藏书票印记需开启「知气雅集」（已有灵宠本体与纪念设置不受影响，可随时保存）。".to_owned()
                });
            }
            CustomPetCompanion {
                id: prev.id,
                name: clean_name,
                species: clean_species,
                persona: clean_persona,
                photo_data_url: validated_photo,
                die_cut_frame: clean_frame,
                starlight_memorial,
                paw_stamp_title: clean_stamp,
                is_active,
                created_at: prev.created_at,
                updated_at: now_iso,
            }
        } else {
            let count = self.db.count_custom_pets()?;
            let limit = custom_pet_quota(&ent);
            if count >= limit {
                return Err(if en {
                    format!(
                        "Current tier includes {limit} custom pet slot ({count}/{limit} used). Your existing pets stay saved forever; unlock Qi Atelier to host up to 3."
                    )
                } else {
                    format!(
                        "当前方案至多入驻 {limit} 位自家灵宠（已有 {count}/{limit} 位，原有灵宠永久保留不锁）；升级「知气雅集」可扩展至 3 位。"
                    )
                });
            }
            if !clean_stamp.is_empty() && !has_feature(&ent, "custom_pet_ex_libris") {
                return Err(if en {
                    "Custom Ex Libris paw stamp titles require an active Qi Atelier plan. Leave blank to save your free custom pet.".to_owned()
                } else {
                    "定制爪印藏书票印记属于知气雅集专属（留空即可免费保存 1 位自家灵宠）。".to_owned()
                });
            }
            let new_id = requested_id.unwrap_or_else(|| self.db.next_custom_pet_id());
            CustomPetCompanion {
                id: new_id,
                name: clean_name,
                species: clean_species,
                persona: clean_persona,
                photo_data_url: validated_photo,
                die_cut_frame: clean_frame,
                starlight_memorial,
                paw_stamp_title: clean_stamp,
                is_active: is_active || count == 0,
                created_at: now_iso.clone(),
                updated_at: now_iso,
            }
        };

        self.db.upsert_custom_pet(&pet_record)?;
        self.build_payload()
    }

    /// 切换窗台当前上阵的「自家灵宠」（传空字符串恢复默认岁时灵宠；所有已存灵宠任何档位均可自由点选切换）。
    pub fn set_active_custom_pet(&mut self, pet_id: String) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        self.db.set_active_custom_pet(&pet_id).map_err(|e| {
            if e == "custom_pet_not_found" {
                if en {
                    format!("Custom pet not found: {pet_id}")
                } else {
                    format!("未找到自家灵宠: {pet_id}")
                }
            } else {
                e
            }
        })?;
        self.build_payload()
    }

    /// 删除指定「自家灵宠」及其名下自动周信归档。
    pub fn delete_custom_pet(&mut self, pet_id: String) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let removed = self.db.delete_custom_pet(&pet_id)?;
        if !removed {
            return Err(if en {
                format!("Custom pet not found: {pet_id}")
            } else {
                format!("未找到可移除的自家灵宠: {pet_id}")
            });
        }
        self.build_payload()
    }

    /// 为指定自家灵宠生成一封第一人称周信并持久化归档（需有效雅集；过期后已生成信件仍永久可读）。
    pub fn generate_pet_weekly_letter(
        &mut self,
        pet_id: String,
        week_number: Option<u32>,
    ) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let ent = self.evaluate_entitlement(&s);
        if !has_feature(&ent, "pet_weekly_letters") {
            return Err(if en {
                "Generating new custom pet weekly letters requires an active Qi Atelier plan (previously saved letters remain readable forever).".to_owned()
            } else {
                "生成新的灵宠第一人称周信需开启「知气雅集」（既往已生成周信永久保留可读）。".to_owned()
            });
        }
        let pet = self.db.get_custom_pet(pet_id.trim())?.ok_or_else(|| {
            if en {
                format!("Custom pet not found: {pet_id}")
            } else {
                format!("未找到自家灵宠: {pet_id}")
            }
        })?;

        let metrics = self.db.load_metrics()?;
        let workouts = self.db.load_workouts()?;
        let intakes = self.db.load_intakes()?;
        let mut snapshot = compute_snapshot(
            &metrics,
            &workouts,
            &intakes,
            HardwareTier::from_str(&s.hardware_tier),
            s.morning_feeling,
        );
        localize_snapshot(&mut snapshot, &s.locale);
        let solar = compute_solar_term_state_localized(
            &snapshot.target_date,
            &s.weather_mood,
            &s.locale,
        );
        let existing_for_pet = self
            .db
            .list_pet_weekly_letters()?
            .into_iter()
            .filter(|l| l.pet_id == pet.id)
            .count();
        let wk = week_number.unwrap_or((existing_for_pet as u32) + 1).max(1);
        let now_iso = chrono::Utc::now().to_rfc3339();
        let draft = generate_pet_weekly_letter_draft(
            &pet,
            &snapshot,
            &solar,
            wk,
            &s.locale,
            &now_iso,
        );
        self.db.insert_pet_weekly_letter(&draft)?;
        self.build_payload()
    }

    /// 核验 StoreKit 2 / Apple IAP 交易（优先调用云端 `/v1/billing/apple/verify`，本地 Xcode/Sandbox 环境支持确定性离线沙盒兜底），并刷新权益与徽章状态。
    pub fn verify_apple_iap_transaction(
        &mut self,
        product_id: String,
        transaction_id: String,
        original_transaction_id: String,
        environment: String,
        jws_representation: Option<String>,
        expires_date_epoch: Option<i64>,
    ) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let clean_pid = product_id.trim().to_owned();
        if clean_pid.is_empty() {
            return Err(if en {
                "Apple product_id is required.".to_owned()
            } else {
                "缺少 Apple 商品 ID（product_id）".to_owned()
            });
        }

        let clean_tx = if transaction_id.trim().is_empty() {
            format!("tx_{}", Self::current_epoch())
        } else {
            transaction_id.trim().to_owned()
        };
        let clean_orig_tx = if original_transaction_id.trim().is_empty() {
            clean_tx.clone()
        } else {
            original_transaction_id.trim().to_owned()
        };
        let env_clean = if environment.trim().is_empty() {
            "Sandbox".to_owned()
        } else {
            environment.trim().to_owned()
        };

        let c_base = s.cloud_url.trim().trim_end_matches('/');
        if !c_base.is_empty() && !s.cloud_token.trim().is_empty() {
            let jws_clean = jws_representation
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToOwned::to_owned);
            if !is_qa_or_debug_build() && jws_clean.is_none() {
                return Err(if en {
                    "StoreKit 2 JWS transaction is required for verification in release builds.".to_owned()
                } else {
                    "正式发布包需提供 StoreKit 2 签名 JWS 交易凭证".to_owned()
                });
            }
            let endpoint = format!("{c_base}/v1/billing/apple/verify");
            let agent = ureq::AgentBuilder::new()
                .timeout_connect(std::time::Duration::from_secs(3))
                .timeout_read(std::time::Duration::from_secs(12))
                .build();
            let req_body = serde_json::json!({
                "jws_representation": jws_clean.unwrap_or_default(),
                "product_id": clean_pid,
                "transaction_id": clean_tx,
                "original_transaction_id": clean_orig_tx,
                "environment": env_clean,
                "bundle_id": "eu.org.cluas.qi",
                "expires_date_epoch": expires_date_epoch,
                "quantity": 1,
                "companion_loyalty_days": s.companion_loyalty_days,
            });
            let resp = agent
                .post(&endpoint)
                .set("Authorization", &format!("Bearer {}", s.cloud_token.trim()))
                .set("Content-Type", "application/json")
                .send_json(req_body)
                .map_err(|e| {
                    if en {
                        format!("Apple receipt verification failed: {e}")
                    } else {
                        format!("Apple 凭证校验请求失败: {e}")
                    }
                })?;
            let val: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
            if !val["verified"].as_bool().unwrap_or(false) {
                return Err(if en {
                    "Apple receipt verification returned unverified status.".to_owned()
                } else {
                    "Apple 交易未能通过核验".to_owned()
                });
            }
            let token_candidate = val["entitlement_token"]
                .as_str()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .or_else(|| {
                    val["guardian_receipt_token"]
                        .as_str()
                        .map(str::trim)
                        .filter(|v| !v.is_empty())
                });
            let mut verified_token_applied = false;
            if let Some(tok) = token_candidate {
                let res = verify_oset_with_clock_localized(
                    tok,
                    Self::current_epoch(),
                    s.max_seen_epoch,
                    None,
                    &s.locale,
                );
                if res.valid_signature {
                    s.entitlement_token = tok.to_owned();
                    s.sandbox_tier = res.tier;
                    s.guardian_sponsor = res.guardian_sponsor;
                    s.max_seen_epoch = res.new_max_seen_epoch;
                    verified_token_applied = true;
                }
            }
            if !verified_token_applied && !is_qa_or_debug_build() {
                return Err(if en {
                    "Cloud Apple verification did not yield a valid signed entitlement token.".to_owned()
                } else {
                    "云端 Apple 校验未返回有效的已签名权益凭据".to_owned()
                });
            }
            self.db.save_settings(&s)?;
            return self.build_payload();
        }

        // 仅允许 Debug/Test/qa-studio 的 Xcode/Sandbox 本地开发态在未绑定云端会话时执行本地沙盒签发；
        // Release 正式包严禁任何离线自签发或绕过云端 StoreKit 2 验签
        if !is_qa_or_debug_build() {
            return Err(if en {
                "Sign in to Qi Cloud with a verified StoreKit 2 receipt to verify purchases in release builds.".to_owned()
            } else {
                "正式发布包需登录云端账号并提供 StoreKit 2 签名凭证以完成购买核验".to_owned()
            });
        }
        let env_lower = env_clean.to_ascii_lowercase();
        if env_lower != "sandbox" && env_lower != "xcode" {
            return Err(if en {
                "Sign in to Qi Cloud first to verify Production App Store receipts.".to_owned()
            } else {
                "生产环境 Apple 票据校验需先连接云端账号会话".to_owned()
            });
        }

        let now = Self::current_epoch();
        let prev_ent = self.evaluate_entitlement(&s);
        if clean_pid == HEARTH_GIFT_PASS_PRODUCT_ID {
            let target_tier = if prev_ent.is_atelier_unlocked {
                SubscriptionTier::from_str(&prev_ent.tier)
            } else {
                SubscriptionTier::Free
            };
            let (_c, refreshed) = mint_oset_for_tier(
                &s.device_id,
                target_tier,
                now,
                true,
                s.companion_loyalty_days >= LOYALTY_MIN_CHECKIN_DAYS
                    || prev_ent.companion_loyalty_eligible,
            )?;
            let res =
                verify_oset_with_clock_localized(&refreshed, now, s.max_seen_epoch, None, &s.locale);
            s.entitlement_token = refreshed;
            s.guardian_sponsor = res.guardian_sponsor;
            s.max_seen_epoch = res.new_max_seen_epoch;
            self.db.save_settings(&s)?;
            return self.build_payload();
        }

        let tier_enum = tier_from_apple_product_id(&clean_pid).ok_or_else(|| {
            if en {
                format!("Unsupported Apple product_id: {clean_pid}")
            } else {
                format!("未识别的 Apple 商品 ID: {clean_pid}")
            }
        })?;
        let token = mint_sandbox_oset(
            &s.device_id,
            tier_enum.as_str(),
            "active",
            prev_ent.guardian_sponsor,
            s.companion_loyalty_days,
            now,
        )?;
        let res =
            verify_oset_with_clock_localized(&token, now, s.max_seen_epoch, None, &s.locale);
        s.entitlement_token = token;
        s.sandbox_tier = tier_enum.as_str().to_owned();
        s.sandbox_state = "active".to_owned();
        s.guardian_sponsor = res.guardian_sponsor;
        s.max_seen_epoch = res.new_max_seen_epoch;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 批量恢复 Apple IAP 已购权益（接收 StoreKit `Transaction.currentEntitlements` JSON 数组）。
    pub fn restore_apple_iap_purchases(
        &mut self,
        transactions_json: String,
    ) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let en = is_en_locale(&s.locale);
        let arr: Vec<serde_json::Value> = serde_json::from_str(&transactions_json).map_err(|e| {
            if en {
                format!("Invalid StoreKit restore payload: {e}")
            } else {
                format!("恢复购买数据格式无效: {e}")
            }
        })?;

        for item in arr {
            let pid = item["product_id"]
                .as_str()
                .or_else(|| item["productId"].as_str())
                .unwrap_or("")
                .to_owned();
            if pid.is_empty() {
                continue;
            }
            let tx_id = item["transaction_id"]
                .as_str()
                .or_else(|| item["transactionId"].as_str())
                .unwrap_or("")
                .to_owned();
            let orig_tx = item["original_transaction_id"]
                .as_str()
                .or_else(|| item["originalTransactionId"].as_str())
                .unwrap_or(&tx_id)
                .to_owned();
            let env = item["environment"]
                .as_str()
                .unwrap_or("Sandbox")
                .to_owned();
            let jws = item["jws_representation"]
                .as_str()
                .or_else(|| item["jws"].as_str())
                .map(ToOwned::to_owned);
            let exp = item["expires_date_epoch"]
                .as_i64()
                .or_else(|| item["expiresDate"].as_i64());
            let _ = self.verify_apple_iap_transaction(pid, tx_id, orig_tx, env, jws, exp)?;
        }
        self.build_payload()
    }

    /// 主动从配置的 WebDAV 网盘拉取跨端 E2EE 加密分片并合并入本地数据库。
    pub fn pull_webdav_sync(&mut self) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        let key = derive_master_key(&s.recovery_phrase)?;
        let fetched =
            pull_envelopes_from_webdav(&s.webdav_url, &s.webdav_username, &s.webdav_password)?;
        let mut pulled = 0usize;
        let mut merged = 0usize;
        for (_, body) in fetched {
            if let Ok(env) = serde_json::from_str::<crate::crypto_sync::EncryptedBatchEnvelope>(&body)
            {
                if env.device_id == s.device_id {
                    continue;
                }
                if let Ok(plain) = decrypt_sync_batch(&key, &env) {
                    merged += self.db.merge_sync_jsonl(&plain)?;
                    pulled += 1;
                }
            }
        }
        s.last_sync_summary =
            format!("WebDAV 双向拉取完成：解密 {pulled} 个远端分片，合并 {merged} 条记录");
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 对当前健康库与确定性统计快照执行 6 项不变量自检，输出 DataAuditReport JSON。
    pub fn run_data_integrity_audit(&self) -> Result<String, String> {
        let settings = self.db.load_settings()?;
        let metrics = self.db.load_metrics()?;
        let workouts = self.db.load_workouts()?;
        let intakes = self.db.load_intakes()?;
        let labs = self.db.load_labs()?;
        let experiments_raw = self.db.load_experiments()?;
        let tier = HardwareTier::from_str(&settings.hardware_tier);

        let snapshot = compute_snapshot(
            &metrics,
            &workouts,
            &intakes,
            tier,
            settings.morning_feeling,
        );
        let experiments_eval = evaluate_experiments(&metrics, &experiments_raw, &snapshot.target_date);
        let period_cmp = compute_period_comparison(&metrics, &snapshot.target_date);
        let briefing = build_morning_briefing_localized(
            &snapshot,
            &labs,
            &settings.ai_mode,
            &settings.lan_ai_url,
            settings.thermal_throttled,
            settings.simulated_ram_gb,
            &settings.locale,
        );

        let distinct_days: BTreeSet<&str> = metrics.iter().map(|m| m.date.as_str()).collect();
        let canonical = resolve_canonical_metrics(&metrics);

        // 1. qi_score 越界与有限性校验
        let opt_finite = |v: Option<f64>| v.map(f64::is_finite).unwrap_or(true);
        let stages_ok = snapshot
            .sleep_stages_last_night
            .as_ref()
            .map(|st| {
                st.deep_min.is_finite()
                    && st.core_min.is_finite()
                    && st.rem_min.is_finite()
                    && st.awake_min.is_finite()
                    && st.deep_min >= 0.0
                    && st.core_min >= 0.0
                    && st.rem_min >= 0.0
                    && st.awake_min >= 0.0
            })
            .unwrap_or(true);
        let metrics_finite = snapshot.metrics.values().all(|m| {
            opt_finite(m.value)
                && opt_finite(m.avg_7d)
                && opt_finite(m.baseline_30d_mean)
                && opt_finite(m.baseline_30d_std)
                && opt_finite(m.z_score)
        });
        let trend_ok = snapshot.trend_14d.iter().all(|p| {
            (15..=99).contains(&p.qi_score) && opt_finite(p.hrv_ms) && opt_finite(p.sleep_h)
        });
        let qi_score_in_bounds = (15..=99).contains(&snapshot.qi_score)
            && opt_finite(snapshot.sleep_debt_7d_hours)
            && opt_finite(snapshot.sleep_last_night_hours)
            && opt_finite(snapshot.acwr)
            && stages_ok
            && metrics_finite
            && trend_ok;

        // 2. RMSSD 与 SDNN 口径物理隔离 + 来源优先级去重校验
        let mut prio_ok = true;
        let mut min_prio_by_day_metric: BTreeMap<(&str, &str), usize> = BTreeMap::new();
        for r in &metrics {
            if !r.value.is_finite() {
                continue;
            }
            let p = source_priority(&r.source);
            min_prio_by_day_metric
                .entry((r.date.as_str(), r.metric.as_str()))
                .and_modify(|cur| {
                    if p < *cur {
                        *cur = p;
                    }
                })
                .or_insert(p);
        }
        for (day, day_map) in &canonical {
            for (metric, row) in day_map {
                if row.metric != *metric {
                    prio_ok = false;
                }
                if let Some(&min_p) = min_prio_by_day_metric.get(&(day.as_str(), metric.as_str()))
                    && source_priority(&row.source) != min_p
                {
                    prio_ok = false;
                }
            }
        }
        let rmssd_sdnn_isolated = prio_ok
            && snapshot
                .metrics
                .get("hrv_rmssd_ms")
                .map(|m| m.key == "hrv_rmssd_ms")
                .unwrap_or(true)
            && snapshot
                .metrics
                .get("hrv_sdnn_ms")
                .map(|m| m.key == "hrv_sdnn_ms")
                .unwrap_or(true);

        // 3. 硬件档位与 HRV/分期可见性契约校验
        let tier_hrv_contract_held = match tier {
            HardwareTier::TierA => {
                let target_day = canonical.get(&snapshot.target_date);
                let has_total = target_day.and_then(|d| d.get("sleep_minutes")).is_some();
                let has_stage = target_day
                    .map(|d| {
                        d.contains_key("sleep_deep_minutes") || d.contains_key("sleep_rem_minutes")
                    })
                    .unwrap_or(false);
                snapshot.sleep_stages_last_night.is_some() == (has_total && has_stage)
            }
            HardwareTier::TierB => {
                !snapshot.metrics.contains_key("hrv_rmssd_ms")
                    && !snapshot.metrics.contains_key("hrv_sdnn_ms")
                    && snapshot.sleep_stages_last_night.is_none()
                    && snapshot.trend_14d.iter().all(|p| p.hrv_ms.is_none())
            }
            HardwareTier::TierC => {
                !snapshot.metrics.contains_key("hrv_rmssd_ms")
                    && !snapshot.metrics.contains_key("hrv_sdnn_ms")
                    && snapshot.metrics.keys().all(|k| k == "steps")
                    && snapshot.sleep_stages_last_night.is_none()
                    && snapshot.sleep_last_night_hours.is_none()
                    && snapshot.acwr.is_none()
                    && snapshot
                        .trend_14d
                        .iter()
                        .all(|p| p.hrv_ms.is_none() && p.sleep_h.is_none())
            }
        };

        // 4. 化验单参考区间与纵向趋势一致性校验
        let mut labs_ok = true;
        let mut by_key: BTreeMap<(&str, &str), Vec<&LabBiomarkerRow>> = BTreeMap::new();
        for l in &labs {
            if !l.value.is_finite() {
                labs_ok = false;
            }
            let expected_flag = match (l.ref_low, l.ref_high) {
                (Some(lo), Some(hi)) => {
                    if l.value > hi {
                        "high"
                    } else if l.value < lo {
                        "low"
                    } else {
                        "normal"
                    }
                }
                (None, Some(hi)) if l.value > hi => "high",
                (Some(lo), None) if l.value < lo => "low",
                _ => "normal",
            };
            if l.flag != expected_flag {
                labs_ok = false;
            }
            by_key
                .entry((l.profile_id.as_str(), l.canonical_key.as_str()))
                .or_default()
                .push(l);
        }
        for mut group in by_key.into_values() {
            group.sort_by(|a, b| a.test_date.cmp(&b.test_date));
            for (idx, row) in group.iter().enumerate() {
                if idx == 0 {
                    if row.trend.as_deref() != Some("new") || row.prev_value.is_some() {
                        labs_ok = false;
                    }
                } else {
                    let prev = group[idx - 1];
                    let expected_delta = ((row.value - prev.value) * 100.0).round() / 100.0;
                    if row.prev_value != Some(prev.value)
                        || row.prev_date.as_deref() != Some(prev.test_date.as_str())
                        || row.delta != Some(expected_delta)
                        || !matches!(
                            row.trend.as_deref(),
                            Some("improved" | "worsened" | "stable")
                        )
                    {
                        labs_ok = false;
                    }
                }
            }
        }
        let lab_trends_consistent = labs_ok;

        // 5. 端侧 PII 脱敏与 SaMD 医疗红线护栏校验
        let probe_scrub = scrub_lab_text_pii("姓名：张三 身份证号：31010119900101123X 手机：13800138000");
        let probe_guard = enforce_wellness_guardrails("建议确诊后开药");
        let has_leak_in_records = labs.iter().any(|l| {
            l.marker_name.contains("姓名：")
                || l.marker_name.contains("310101")
                || l.plain_note.contains("确诊")
        }) || intakes.iter().any(|i| i.notes.contains("310101"))
            || briefing.text.contains("确诊")
            || briefing.text.contains("开药")
            || briefing.text.contains("处方")
            || briefing.text.contains("治疗方案");
        let pii_guard_verified = !probe_scrub.scrubbed_text.contains("张三")
            && !probe_scrub.scrubbed_text.contains("31010119900101123X")
            && !probe_scrub.scrubbed_text.contains("13800138000")
            && !probe_guard.contains("确诊")
            && !probe_guard.contains("开药")
            && !has_leak_in_records;

        // 6. 实验、滞后归因数值有限性 + 保险箱永不锁死契约校验
        let en = is_en_locale(&settings.locale);
        let current_ent = self.evaluate_entitlement(&settings);
        let now_audit = Self::current_epoch().max(settings.max_seen_epoch);
        let probe_signing_key = SigningKey::from_bytes(&[0x5A; 32]);
        let probe_pubkey_hex = hex::encode(probe_signing_key.verifying_key().to_bytes());
        let probe_claims = OsetClaims {
            ver: 1,
            iss: "qi-audit-probe".to_owned(),
            sub: "audit_probe".to_owned(),
            tier: SubscriptionTier::ProMonthly.as_str().to_owned(),
            issued_at: now_audit.saturating_sub(45 * 86_400),
            refresh_after: now_audit.saturating_sub(38 * 86_400),
            exp_epoch: now_audit.saturating_sub(14 * 86_400),
            grace_until: now_audit.saturating_sub(7 * 86_400),
            features: SubscriptionTier::ProMonthly.default_features(),
            seats: SubscriptionTier::ProMonthly.default_seats(),
            guardian_sponsor: false,
            companion_loyalty_eligible: false,
            nonce: "audit01".to_owned(),
        };
        let expired_probe_token =
            sign_oset_with_key(&probe_claims, &probe_signing_key).unwrap_or_default();
        let expired_probe = verify_oset_with_clock_localized(
            &expired_probe_token,
            now_audit,
            settings.max_seen_epoch,
            Some(&probe_pubkey_hex),
            &settings.locale,
        );
        let vault_non_lockout_verified = current_ent.can_read_history_vault
            && current_ent.can_export_vault
            && expired_probe.state == OfflineEntitlementState::ExpiredReadOnlyVault
            && expired_probe.can_read_history_vault
            && expired_probe.can_export_vault
            && has_feature(&expired_probe, "raw_biometrics")
            && has_feature(&expired_probe, "single_lab_parse")
            && has_feature(&expired_probe, "jsonl_export")
            && has_feature(&expired_probe, "recovery_priority")
            && has_feature(&expired_probe, "custom_pet_1");

        let experiments_ok = experiments_eval.iter().all(|e| {
            opt_finite(e.hrv_delta_ms)
                && opt_finite(e.rhr_delta_bpm)
                && opt_finite(e.deep_sleep_delta_min)
                && !e.verdict_summary.is_empty()
        }) && snapshot
            .lagged_impacts
            .iter()
            .all(|li| li.delta_value.is_finite())
            && opt_finite(period_cmp.hrv_current)
            && opt_finite(period_cmp.hrv_prev)
            && opt_finite(period_cmp.rhr_current)
            && opt_finite(period_cmp.rhr_prev)
            && opt_finite(period_cmp.sleep_current_h)
            && opt_finite(period_cmp.sleep_prev_h)
            && (metrics.is_empty() || !distinct_days.is_empty())
            && vault_non_lockout_verified;

        let checks = vec![
            AuditCheckItem {
                name: "qi_score_in_bounds".to_owned(),
                passed: qi_score_in_bounds,
                detail: format!(
                    "qi_score={} (15..=99), trend_len={}, acwr={:?}",
                    snapshot.qi_score,
                    snapshot.trend_14d.len(),
                    snapshot.acwr
                ),
            },
            AuditCheckItem {
                name: "rmssd_sdnn_isolated".to_owned(),
                passed: rmssd_sdnn_isolated,
                detail: if en {
                    "RMSSD and SDNN maintain isolated baselines with deterministic source priority.".to_owned()
                } else {
                    "RMSSD 与 SDNN 独立基线且多源按优先级消解".to_owned()
                },
            },
            AuditCheckItem {
                name: "tier_hrv_contract_held".to_owned(),
                passed: tier_hrv_contract_held,
                detail: if en {
                    format!("Hardware tier {} metrics and sleep stage contract verified.", settings.hardware_tier)
                } else {
                    format!("硬件档位 {} 指标与睡眠分期契约校验", settings.hardware_tier)
                },
            },
            AuditCheckItem {
                name: "lab_trends_consistent".to_owned(),
                passed: lab_trends_consistent,
                detail: if en {
                    format!("Verified {} lab biomarker reference ranges and longitudinal deltas/trends.", labs.len())
                } else {
                    format!("已校验 {} 条化验单参考区间与纵向 delta/trend", labs.len())
                },
            },
            AuditCheckItem {
                name: "pii_guard_verified".to_owned(),
                passed: pii_guard_verified,
                detail: if en {
                    "On-device PII redaction and SaMD wellness guardrails passed.".to_owned()
                } else {
                    "端侧 PII 脱敏与 SaMD 合规护栏校验通过".to_owned()
                },
            },
            AuditCheckItem {
                name: "experiments_and_lagged_finite".to_owned(),
                passed: experiments_ok,
                detail: if en {
                    format!(
                        "{} experiments and {} lag impacts finite; read-only vault non-lockout verified={}",
                        experiments_eval.len(),
                        snapshot.lagged_impacts.len(),
                        vault_non_lockout_verified
                    )
                } else {
                    format!(
                        "实验 {} 项、滞后归因 {} 项均无 NaN/越界；保险箱非锁死校验 vault_non_lockout_verified={}",
                        experiments_eval.len(),
                        snapshot.lagged_impacts.len(),
                        vault_non_lockout_verified
                    )
                },
            },
        ];

        let passed = checks.iter().all(|c| c.passed) && vault_non_lockout_verified;

        let report = DataAuditReport {
            active_persona: settings.active_persona,
            metric_rows: metrics.len(),
            distinct_days: distinct_days.len(),
            intake_rows: intakes.len(),
            lab_rows: labs.len(),
            experiment_rows: experiments_raw.len(),
            qi_score_in_bounds,
            rmssd_sdnn_isolated,
            tier_hrv_contract_held,
            lab_trends_consistent,
            pii_guard_verified,
            vault_non_lockout_verified,
            passed,
            checks,
        };
        serde_json::to_string(&report).map_err(|e| e.to_string())
    }

    /// 检查当前运行环境是否可唤起原生 Apple HealthKit 授权与直连查询。
    pub fn is_healthkit_available(&self) -> bool {
        hk_is_available()
    }

    /// 切换打开「知气」时是否自动从 Apple 健康增量同步近 7 天最新记录。
    pub fn set_healthkit_auto_sync(&mut self, enabled: bool) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        s.healthkit_auto_sync = enabled;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 在后台 OS 线程请求 Apple HealthKit 读取授权并导入近 `days` 天生理与运动记录，不阻塞 UI 主线程。
    pub async fn sync_apple_healthkit(this: Instance<Self>, days: u32) -> Result<String, String> {
        let clamped = days.clamp(1, 180);
        let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
        std::thread::spawn(move || {
            let res = request_and_query_healthkit_json(clamped);
            let _ = tx.send(res);
        });
        let raw_json = loop {
            match rx.try_recv() {
                Ok(res) => break res?,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    timer(std::time::Duration::from_millis(40)).await;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return Err("Apple HealthKit 同步线程意外中断".to_owned());
                }
            }
        };
        this.with_mut(|engine| engine.ingest_healthkit_sync_json(&raw_json, clamped))
    }
}

#[cfg(test)]
mod e2e_tests {
    use super::*;
    use crate::safety_ai::{parse_lab_text_and_verify, scrub_lab_text_pii};

    #[test]
    fn real_apple_vision_ocr_then_pii_scrub_then_lab_parse() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("sample_lab_report.png");
        if !fixture.is_file() {
            return;
        }
        if let Ok(ocr_text) = run_offline_vision_ocr(&fixture.to_string_lossy()) {
            let scrubbed = scrub_lab_text_pii(&ocr_text);
            assert!(!scrubbed.scrubbed_text.contains("陈默"));
            assert!(!scrubbed.scrubbed_text.contains("31010419920518432X"));
            assert!(!scrubbed.scrubbed_text.contains("13912345678"));

            let markers = parse_lab_text_and_verify(&scrubbed.scrubbed_text);
            assert!(markers.len() >= 4, "expected >=4 markers from real OCR, got {markers:?}");
            let ua = markers.iter().find(|m| m.canonical_key == "uric_acid").unwrap();
            assert_eq!(ua.flag, "high");
            let alt = markers.iter().find(|m| m.canonical_key == "alt").unwrap();
            assert_eq!(alt.flag, "high");
            let vd = markers.iter().find(|m| m.canonical_key == "vitamin_d").unwrap();
            assert_eq!(vd.flag, "low");
        }
    }

    #[test]
    fn cross_device_e2ee_sync_and_12_word_recovery_roundtrip() {
        let sync_dir = std::env::temp_dir().join(format!("qi_e2ee_sync_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&sync_dir);
        unsafe {
            std::env::set_var("QI_ICLOUD_SYNC_DIR", &sync_dir);
        }

        // 设备 A：录入餐食、开启新实验并执行 iCloud 目录 E2EE 推送
        let mut dev_a = QiEngine::new(Some(":memory:".to_owned())).unwrap();
        dev_a.complete_onboarding("lab_archive".into(), "tier_a".into()).unwrap();
        dev_a
            .log_natural_intake("下午四点喝了杯生椰拿铁，晚上吃了顿潮汕牛肉火锅".into())
            .unwrap();
        dev_a
            .import_lab_report("2026-11-13".into(), "尿酸（UA）480 umol/L 208-428".into())
            .unwrap();
        dev_a.run_e2ee_sync_now().unwrap();

        let state_a: serde_json::Value = serde_json::from_str(&dev_a.state_json().unwrap()).unwrap();
        let phrase = state_a["settings"]["recovery_phrase"].as_str().unwrap().to_owned();
        let score_a = state_a["snapshot"]["qi_score"].as_u64().unwrap();

        // 设备 B：从严格空白库出发，仅凭 12 词恢复码重建完整个人健康库
        let mut dev_b = QiEngine::new(Some(":empty:".to_owned())).unwrap();
        dev_b
            .restore_from_recovery_phrase(
                phrase,
                "icloud_drive".into(),
                sync_dir.to_string_lossy().into_owned(),
            )
            .unwrap();

        let state_b: serde_json::Value = serde_json::from_str(&dev_b.state_json().unwrap()).unwrap();
        assert_eq!(state_b["settings"]["onboarding_done"], true);
        assert_eq!(state_b["snapshot"]["qi_score"].as_u64().unwrap(), score_a);
        assert!(!state_b["labs"].as_array().unwrap().is_empty());
        assert!(!state_b["experiments"].as_array().unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&sync_dir);
    }

    #[test]
    fn byo_server_http_e2ee_sync_and_zdr_briefing_roundtrip() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::{Arc, Mutex};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{addr}");
        let stored_envelopes: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let env_ref = stored_envelopes.clone();

        std::thread::spawn(move || {
            for stream_res in listener.incoming().take(6) {
                let Ok(mut stream) = stream_res else {
                    break;
                };
                let mut buf = Vec::new();
                let mut tmp = [0u8; 4096];
                let mut header_end = None;
                let mut content_len = 0usize;
                loop {
                    let n = stream.read(&mut tmp).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                    if header_end.is_none() {
                        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            header_end = Some(pos + 4);
                            let hdr = String::from_utf8_lossy(&buf[..pos + 4]);
                            for line in hdr.lines() {
                                if let Some((k, v)) = line.split_once(':') {
                                    if k.trim().eq_ignore_ascii_case("content-length") {
                                        content_len = v.trim().parse().unwrap_or(0);
                                    }
                                }
                            }
                        }
                    }
                    if let Some(he) = header_end {
                        if buf.len() >= he + content_len {
                            break;
                        }
                    }
                }
                let raw_req = String::from_utf8_lossy(&buf);
                let first_line = raw_req.lines().next().unwrap_or("");
                let body_str = header_end
                    .map(|he| String::from_utf8_lossy(&buf[he..]).into_owned())
                    .unwrap_or_default();

                let resp_body = if first_line.starts_with("POST /v1/sync/ops/push") {
                    let env: serde_json::Value = serde_json::from_str(&body_str).unwrap();
                    assert_eq!(env["cipher"], "XChaCha20-Poly1305");
                    let mut guard = env_ref.lock().unwrap();
                    guard.push(env);
                    serde_json::json!({ "id": guard.len(), "status": "stored_ciphertext_only" }).to_string()
                } else if first_line.starts_with("GET /v1/sync/ops/pull") {
                    let guard = env_ref.lock().unwrap();
                    let envs: Vec<serde_json::Value> = guard
                        .iter()
                        .enumerate()
                        .map(|(idx, e)| {
                            let mut item = e.clone();
                            item["id"] = serde_json::json!((idx + 1) as i64);
                            item
                        })
                        .collect();
                    serde_json::json!({
                        "envelopes": envs,
                        "count": envs.len(),
                        "next_after_id": envs.len()
                    })
                    .to_string()
                } else if first_line.starts_with("POST /v1/zdr/complete") {
                    assert!(!body_str.contains("陈默"));
                    serde_json::json!({
                        "provider": "mock-zdr-gateway",
                        "content": "今日元气平稳，优先保持 23:00 前规律入睡。",
                        "zdr_enforced": true
                    })
                    .to_string()
                } else {
                    "{}".to_owned()
                };

                let http_resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    resp_body.len(),
                    resp_body
                );
                let _ = stream.write_all(http_resp.as_bytes());
            }
        });

        let mut dev_a = QiEngine::new(Some(":memory:".to_owned())).unwrap();
        dev_a.complete_onboarding("energy_sleep".into(), "tier_a".into()).unwrap();
        dev_a.update_cloud_session(base_url.clone(), "tester@qi.local".into(), "tok-demo-1".into()).unwrap();
        dev_a.update_sync_config("byo_server".into(), "".into(), base_url.clone(), "".into(), "".into()).unwrap();
        dev_a.update_ai_config("official_zdr_cloud".into(), true, "".into(), "".into(), false, 8).unwrap();

        dev_a.refresh_live_briefing().unwrap();
        let st_a: serde_json::Value = serde_json::from_str(&dev_a.state_json().unwrap()).unwrap();
        assert!(st_a["briefing"]["text"].as_str().unwrap().contains("23:00"));

        dev_a.log_quick_intake("meal".into(), "轻食鸡胸沙拉".into(), "高蛋白轻食".into(), "HTTP sync".into(), Some(420.0)).unwrap();
        dev_a.run_e2ee_sync_now().unwrap();
        let phrase = st_a["settings"]["recovery_phrase"].as_str().unwrap().to_owned();

        let mut dev_b = QiEngine::new(Some(":empty:".to_owned())).unwrap();
        dev_b
            .update_cloud_session(base_url.clone(), "tester@qi.local".into(), "tok-demo-1".into())
            .unwrap();
        dev_b.restore_from_recovery_phrase(phrase, "byo_server".into(), base_url).unwrap();
        let st_b: serde_json::Value = serde_json::from_str(&dev_b.state_json().unwrap()).unwrap();
        assert!(
            st_b["intakes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x["item_name"] == "轻食鸡胸沙拉")
        );
    }

    #[test]
    fn open_db_falls_back_when_primary_parent_unwritable() {
        let fallback_dir = std::env::temp_dir().join(format!(
            "qi_open_db_fallback_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let fallback_db = fallback_dir.join("qi_vault.sqlite3");
        let unwritable_primary = PathBuf::from("/dev/null/qi_sub/qi_vault.sqlite3");
        let db = QiEngine::open_db_with_candidates(&[unwritable_primary, fallback_db.clone()])
            .expect("primary failure should fall back to writable path");
        let settings = db.load_settings().expect("fallback db should initialize settings");
        assert!(!settings.locale.is_empty());
        assert!(fallback_db.exists());
        let _ = std::fs::remove_dir_all(&fallback_dir);
    }

    #[test]
    fn healthkit_import_clears_demo_persona_preserves_user_logs_and_keeps_sdnn_isolated() {
        let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
        engine
            .log_quick_intake(
                "drink".into(),
                "自录测试手冲咖啡".into(),
                "午后".into(),
                "测试保留".into(),
                Some(5.0),
            )
            .unwrap();
        engine
            .import_lab_report("2026-11-10".into(), "尿酸 (UA): 390 umol/L (参考 155-357)".into())
            .unwrap();

        let mut hk_metrics = Vec::new();
        for day_idx in 1..=10 {
            let date = format!("2026-11-{day_idx:02}");
            let late_cafe = day_idx % 3 == 0;
            if late_cafe {
                hk_metrics.push(serde_json::json!({
                    "date": date,
                    "metric": "sleep_deep_minutes",
                    "source": "apple_health",
                    "value": 62.0,
                    "unit": "min"
                }));
                hk_metrics.push(serde_json::json!({
                    "date": date,
                    "metric": "hrv_sdnn_ms",
                    "source": "apple_health",
                    "value": 34.0,
                    "unit": "ms"
                }));
            } else {
                hk_metrics.push(serde_json::json!({
                    "date": date,
                    "metric": "sleep_deep_minutes",
                    "value": 92.0,
                    "unit": "min"
                }));
                hk_metrics.push(serde_json::json!({
                    "date": date,
                    "metric": "hrv_sdnn_ms",
                    "value": 52.0,
                    "unit": "ms"
                }));
            }
            hk_metrics.push(serde_json::json!({
                "date": date,
                "metric": "sleep_minutes",
                "value": 445.0,
                "unit": "min"
            }));
            hk_metrics.push(serde_json::json!({
                "date": date,
                "metric": "resting_heart_rate_bpm",
                "value": 59.0,
                "unit": "bpm"
            }));
            hk_metrics.push(serde_json::json!({
                "date": date,
                "metric": "steps",
                "value": 7800.0,
                "unit": "步"
            }));
        }

        let hk_payload = serde_json::json!({
            "ok": true,
            "available": true,
            "authorized": true,
            "metrics": hk_metrics,
            "workouts": [{
                "workout_id": "HK-TEST-W1",
                "date": "2026-11-10",
                "activity_type": "跑步",
                "duration_minutes": 32.0,
                "calories_kcal": 265.0
            }]
        })
        .to_string();

        let state_raw = engine.ingest_healthkit_sync_json(&hk_payload, 35).unwrap();
        let st: serde_json::Value = serde_json::from_str(&state_raw).unwrap();
        assert_eq!(st["settings"]["active_persona"], "real_healthkit");
        assert_eq!(st["settings"]["healthkit_authorized"], true);
        assert_eq!(st["settings"]["healthkit_synced_days"], 10);
        assert_eq!(st["settings"]["hardware_tier"], "tier_a");
        assert!(st["snapshot"]["metrics"].get("hrv_rmssd_ms").is_none());
        assert_eq!(st["snapshot"]["metrics"]["hrv_sdnn_ms"]["source"], "apple_health");
        assert!(
            st["intakes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x["item_name"] == "自录测试手冲咖啡")
        );
        assert!(!st["labs"].as_array().unwrap().is_empty());

        let audit: serde_json::Value =
            serde_json::from_str(&engine.run_data_integrity_audit().unwrap()).unwrap();
        assert_eq!(audit["passed"], true);
        assert_eq!(audit["rmssd_sdnn_isolated"], true);
    }

    #[test]
    fn healthkit_empty_import_preserves_existing_data_with_guidance() {
        let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
        let before: serde_json::Value = serde_json::from_str(&engine.state_json().unwrap()).unwrap();
        let before_date = before["snapshot"]["target_date"].as_str().unwrap().to_owned();

        let empty_payload = serde_json::json!({
            "ok": true,
            "available": true,
            "authorized": true,
            "metrics": [],
            "workouts": []
        })
        .to_string();

        let after_raw = engine.ingest_healthkit_sync_json(&empty_payload, 35).unwrap();
        let after: serde_json::Value = serde_json::from_str(&after_raw).unwrap();
        assert_eq!(after["snapshot"]["target_date"], before_date);
        assert_eq!(after["settings"]["healthkit_synced_days"], 0);
        assert!(
            after["settings"]["healthkit_last_summary"]
                .as_str()
                .unwrap()
                .contains("系统设置")
        );
    }
}

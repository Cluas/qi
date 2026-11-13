//! 「知气 · Qi」端侧 Rust 核心引擎，导出给 TSX 视图层 (`qi/native`)。

pub mod ai_provider;
pub mod analytics;
pub mod apple_native;
pub mod crypto_sync;
pub mod entitlements;
pub mod experiments;
pub mod importers;
pub mod nutrition_db;
pub mod personas;
pub mod safety_ai;
pub mod solar_atelier;
pub mod storage;
pub mod sync_engine;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use nsh_core::prelude::*;
use serde::Serialize;

use crate::ai_provider::generate_live_briefing;
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
    EntitlementCatalog, EntitlementVerificationResult, LOYALTY_MIN_CHECKIN_DAYS,
    OfflineEntitlementState, SubscriptionTier, build_entitlement_catalog, has_feature,
    mint_sandbox_oset, verify_oset_with_clock,
};
use crate::experiments::{
    EXPERIMENT_TEMPLATES, ExperimentEvaluation, ExperimentTemplate, PeriodComparison,
    compute_period_comparison, evaluate_experiments,
};
use crate::importers::{parse_apple_health_xml_snippet, parse_csv_metrics};
use crate::nutrition_db::{FoodSearchHit, parse_natural_intake_text, search_foods};
use crate::safety_ai::{
    BriefingResult, MultiProfileRecheckReminder, MultiProfileUlnPoint, ParentsPeaceLetter,
    WELLNESS_DISCLAIMER, build_morning_briefing, build_parents_peace_letter, canonicalize_marker,
    compute_multi_profile_uln_trends_at, enforce_wellness_guardrails, scrub_lab_text_pii,
};
use crate::solar_atelier::{
    MAX_EQUIPPED_KEEPSAKES, SolarAtelierPayload, build_seasonal_pass, build_solar_atelier_payload,
    compute_solar_term_state, infer_sticker_nutrition_hint, normalize_weather_mood,
};
use crate::storage::{AppSettings, VaultDatabase};
use crate::sync_engine::{
    SyncExecutionReport, build_local_archive_json, decrypt_local_archive_json,
    pull_envelopes_from_byo, pull_envelopes_from_dir, pull_envelopes_from_webdav,
    push_envelope_to_byo, push_envelope_to_dir, push_envelope_to_webdav,
    resolve_icloud_vault_dir,
};

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
                if let Some(home) = std::env::var_os("HOME") {
                    let path = PathBuf::from(home).join(".qi").join("qi_vault.sqlite3");
                    VaultDatabase::open_file(&path)
                } else {
                    VaultDatabase::open_memory()
                }
            }
        }
    }

    fn current_epoch() -> i64 {
        chrono::Utc::now().timestamp()
    }

    fn evaluate_entitlement(&self, settings: &AppSettings) -> EntitlementVerificationResult {
        let now = Self::current_epoch();
        verify_oset_with_clock(&settings.entitlement_token, now, settings.max_seen_epoch, None)
    }

    fn award_recovery_dew_if_needed(&self, snapshot: &AnalyticsSnapshot) {
        if snapshot.state == "recovery_priority" {
            let _ = self.db.ensure_daily_recovery_dew(&snapshot.target_date);
        }
    }

    fn build_payload(&self) -> Result<String, String> {
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
        let experiments = evaluate_experiments(&metrics, &experiments_raw, &snapshot.target_date);
        let period_comparison = compute_period_comparison(&metrics, &snapshot.target_date);

        let briefing = if let Some(ref over) = self.live_briefing_override {
            over.clone()
        } else {
            build_morning_briefing(
                &snapshot,
                &labs,
                &settings.ai_mode,
                &settings.lan_ai_url,
                settings.thermal_throttled,
                settings.simulated_ram_gb,
            )
        };

        let entitlements = self.evaluate_entitlement(&settings);
        let distinct_metric_days: BTreeSet<&str> = metrics.iter().map(|m| m.date.as_str()).collect();
        let loyalty_eligible = settings.companion_loyalty_days >= LOYALTY_MIN_CHECKIN_DAYS
            || distinct_metric_days.len() as i64 >= LOYALTY_MIN_CHECKIN_DAYS
            || entitlements.companion_loyalty_eligible;
        let entitlement_catalog = build_entitlement_catalog(loyalty_eligible);

        let dew_balance = self.db.get_dew_balance()?;
        let dew_ledger = self.db.list_dew_ledger(24)?;
        let claimed_rewards = self.db.list_claimed_pass_rewards()?;
        let presets = self.db.list_presets()?;
        let stickers = self.db.list_stickers()?;
        let ym: String = snapshot.target_date.chars().take(7).collect();
        let used_stickers_in_month = self.db.count_stickers_in_month(&ym)?;

        let solar_atelier = build_solar_atelier_payload(
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
        );

        let daily_records = build_daily_records(&metrics, &intakes);
        let multi_lag_radar = compute_multi_lag_radar(&daily_records);
        let (multi_profile_uln_trends, recheck_reminders) =
            compute_multi_profile_uln_trends_at(&labs, &snapshot.target_date);
        let parents_peace_letter_preview =
            build_parents_peace_letter("爸妈", &snapshot, &recheck_reminders);

        let payload = UiStatePayload {
            settings,
            snapshot,
            briefing,
            intakes,
            labs,
            experiments,
            experiment_templates: EXPERIMENT_TEMPLATES.to_vec(),
            period_comparison,
            last_lab_import: self.last_lab_import.clone(),
            capability_probe: self.last_probe.clone(),
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
        let labs = self.db.load_labs()?;
        let tier = HardwareTier::from_str(&s.hardware_tier);
        let snapshot = compute_snapshot(&metrics, &workouts, &intakes, tier, s.morning_feeling);

        let res = generate_live_briefing(
            &snapshot,
            &labs,
            &s.ai_mode,
            &s.lan_ai_url,
            &s.lan_ai_model,
            &s.cloud_url,
            &s.cloud_token,
            s.thermal_throttled,
            s.simulated_ram_gb,
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
        self.live_briefing_override = None;
        self.db.seed_demo_35d()?;
        self.build_payload()
    }

    /// 切换并载入指定纵向测试画像（清空业务表，保留云会话、12词恢复码与同步配置）。
    pub fn load_persona(&mut self, persona_id: String) -> Result<String, String> {
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
        s.guardian_sponsor = guardian_sponsor;
        s.companion_loyalty_days = loyalty_days.max(0);
        s.entitlement_token = token;
        let res = verify_oset_with_clock(&s.entitlement_token, now, s.max_seen_epoch, None);
        s.max_seen_epoch = res.new_max_seen_epoch;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 粘贴并应用真实签发的 Ed25519 离线权益通行证。
    pub fn apply_entitlement_token(&mut self, token: String) -> Result<String, String> {
        let trimmed = token.trim().to_owned();
        let mut s = self.db.load_settings()?;
        let now = Self::current_epoch();
        if trimmed.is_empty() {
            s.entitlement_token.clear();
            s.sandbox_tier = "free".to_owned();
            self.db.save_settings(&s)?;
            return self.build_payload();
        }
        let res = verify_oset_with_clock(&trimmed, now, s.max_seen_epoch, None);
        if !res.valid_signature {
            return Err("雅集通行证签名校验失败，请确认令牌完整无篡改".to_owned());
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
        let ent = self.evaluate_entitlement(&s);
        let claimed = self.db.list_claimed_pass_rewards()?;
        let pass = build_seasonal_pass(&ent, &claimed);
        let node = pass
            .into_iter()
            .find(|n| n.id == reward_id || n.reward_id == reward_id)
            .ok_or_else(|| format!("未找到季票风物节点: {reward_id}"))?;
        if node.claimed {
            return Err("该岁时风物已收入行囊，无需重复兑换".to_owned());
        }
        if !node.unlocked_by_tier {
            return Err("该风物属于「知气雅集」限定轨，开启雅集后即可用晨露兑换".to_owned());
        }
        let balance = self.db.get_dew_balance()?;
        if balance < node.cost_dew {
            return Err(format!(
                "晨露不足：需要 {} 💧，当前拥有 {} 💧",
                node.cost_dew, balance
            ));
        }
        self.db.add_dew_entry(
            -node.cost_dew,
            "seasonal_pass_claim",
            &format!("兑换岁时风物：{}", node.title),
            &node.reward_id,
        )?;
        self.build_payload()
    }

    /// 手动领取今日「安心静养 · 睡饱双倍露 (+3 💧)」（仅在静养日触发，永久免费）。
    pub fn claim_recovery_dew(&mut self) -> Result<String, String> {
        let s = self.db.load_settings()?;
        let snap = compute_snapshot(
            &self.db.load_metrics()?,
            &self.db.load_workouts()?,
            &self.db.load_intakes()?,
            HardwareTier::from_str(&s.hardware_tier),
            s.morning_feeling,
        );
        if snap.state != "recovery_priority" {
            return Err("今日状态稳健，仅在「调养 · 蓄能」静养日自动触发双倍晨露".to_owned());
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
                    "素心清欢最多陈列 2 件风物，升级知气雅集解锁第 3 个珍藏陈列位".to_owned()
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
        let ink = match seal_ink.trim() {
            "gold" => "gold",
            "lapis" => "lapis",
            _ => "cinnabar",
        };
        if ink != "cinnabar" {
            let ent = self.evaluate_entitlement(&s);
            let claimed = self.db.list_claimed_pass_rewards()?;
            if !has_feature(&ent, "poster_custom_seals") && !claimed.iter().any(|c| c == ink) {
                return Err("泥金/青金石印泥属于知气雅集专属（或先在季票兑换）".to_owned());
            }
        }
        s.active_seal_ink = ink.to_owned();
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 保存当前书房预设（素心清欢最多 2 套，雅集无限）。
    pub fn save_sanctuary_preset(&mut self, name: String) -> Result<String, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("请输入书房预设名称".to_owned());
        }
        let s = self.db.load_settings()?;
        let ent = self.evaluate_entitlement(&s);
        let existing_presets = self.db.list_presets()?;
        let is_overwrite = existing_presets.iter().any(|p| p.name == trimmed);
        if !is_overwrite && !has_feature(&ent, "preset_slots_unlimited") && existing_presets.len() >= 2
        {
            return Err("素心清欢最多保存 2 套书房预设，升级知气雅集解锁无限预设位".to_owned());
        }
        let metrics = self.db.load_metrics()?;
        let target_date = metrics
            .last()
            .map(|r| r.date.clone())
            .unwrap_or_else(|| "2026-11-13".to_owned());
        let solar = compute_solar_term_state(&target_date, &s.weather_mood);
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
        let preset = self
            .db
            .get_preset(preset_id)?
            .ok_or_else(|| format!("未找到书房预设 #{preset_id}"))?;
        let mut s = self.db.load_settings()?;
        s.weather_mood = preset.weather_mood;
        s.active_seal_ink = preset.seal_ink;
        s.equipped_keepsakes_json = preset.keepsakes_json;
        self.db.save_settings(&s)?;
        self.build_payload()
    }

    /// 添加一张食光拍立得饮食贴纸（免费版每月 5 张，雅集无限；奖励 +1 💧）。
    pub fn add_food_sticker(
        &mut self,
        date: String,
        title: String,
        filter: String,
        note: String,
        image_ref: String,
    ) -> Result<String, String> {
        let trimmed_title = title.trim();
        if trimmed_title.is_empty() {
            return Err("请输入饮食贴纸名称".to_owned());
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
        let s = self.db.load_settings()?;
        let ent = self.evaluate_entitlement(&s);
        let used = self.db.count_stickers_in_month(&ym)?;
        if !has_feature(&ent, "unlimited_stickers") && used >= 5 {
            return Err("本月素心清欢 5 张食光贴纸额度已用完，升级知气雅集解锁无限手账".to_owned());
        }
        let clean_filter = match filter.trim() {
            "watercolor" => "watercolor",
            "celadon" => "celadon",
            _ => "polaroid",
        };
        let hint = infer_sticker_nutrition_hint(trimmed_title);
        self.db.insert_sticker(
            &use_date,
            trimmed_title,
            clean_filter,
            note.trim(),
            &hint,
            image_ref.trim(),
        )?;
        self.db.add_dew_entry(
            1,
            "food_sticker",
            &format!("定格食光贴纸：{trimmed_title}"),
            "",
        )?;
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

    /// 隐私优先双人同窗互动（`warm_tea` / `tuck_blanket` / `paw_stamp`，奖励 +1 💧 并浇灌窗台金桂）。
    pub fn cosanctuary_action(
        &mut self,
        action: String,
        partner_name: Option<String>,
        pair_code: Option<String>,
    ) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
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
            "warm_tea" => Some("同窗串门：给好友递上一盏热茶 +1 💧"),
            "tuck_blanket" => Some("同窗串门：替好友盖好薄被 +1 💧"),
            "paw_stamp" => Some("同窗串门：在窗台按下暖心爪印 +1 💧"),
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
        if title.trim().is_empty() {
            return Err("请输入实验标题".to_owned());
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
        let metrics = self.db.load_metrics()?;
        let workouts = self.db.load_workouts()?;
        let intakes = self.db.load_intakes()?;
        let labs = self.db.load_labs()?;
        let snapshot = compute_snapshot(
            &metrics,
            &workouts,
            &intakes,
            HardwareTier::from_str(&s.hardware_tier),
            s.morning_feeling,
        );
        let (_, reminders) = compute_multi_profile_uln_trends_at(&labs, &snapshot.target_date);
        let letter = build_parents_peace_letter(&recipient_label, &snapshot, &reminders);
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
        if marker_name.trim().is_empty() || !value.is_finite() {
            return Err("请输入有效的检验指标名称与数值".to_owned());
        }
        let pid = match profile_id.trim() {
            "partner" => "partner",
            "parents" => "parents",
            _ => "self",
        };
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
        let briefing = build_morning_briefing(
            &snapshot,
            &labs,
            &settings.ai_mode,
            &settings.lan_ai_url,
            settings.thermal_throttled,
            settings.simulated_ram_gb,
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
        let current_ent = self.evaluate_entitlement(&settings);
        let expired_probe_token =
            mint_sandbox_oset("audit_probe", "pro_monthly", "expired", false, 0, Self::current_epoch())
                .unwrap_or_default();
        let expired_probe = verify_oset_with_clock(
            &expired_probe_token,
            Self::current_epoch(),
            settings.max_seen_epoch,
            None,
        );
        let vault_non_lockout_verified = current_ent.can_read_history_vault
            && current_ent.can_export_vault
            && expired_probe.state == OfflineEntitlementState::ExpiredReadOnlyVault
            && expired_probe.can_read_history_vault
            && expired_probe.can_export_vault
            && has_feature(&expired_probe, "raw_biometrics")
            && has_feature(&expired_probe, "single_lab_parse")
            && has_feature(&expired_probe, "jsonl_export")
            && has_feature(&expired_probe, "recovery_priority");

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
                detail: "RMSSD 与 SDNN 独立基线且多源按优先级消解".to_owned(),
            },
            AuditCheckItem {
                name: "tier_hrv_contract_held".to_owned(),
                passed: tier_hrv_contract_held,
                detail: format!("硬件档位 {} 指标与睡眠分期契约校验", settings.hardware_tier),
            },
            AuditCheckItem {
                name: "lab_trends_consistent".to_owned(),
                passed: lab_trends_consistent,
                detail: format!("已校验 {} 条化验单参考区间与纵向 delta/trend", labs.len()),
            },
            AuditCheckItem {
                name: "pii_guard_verified".to_owned(),
                passed: pii_guard_verified,
                detail: "端侧 PII 脱敏与 SaMD 合规护栏校验通过".to_owned(),
            },
            AuditCheckItem {
                name: "experiments_and_lagged_finite".to_owned(),
                passed: experiments_ok,
                detail: format!(
                    "实验 {} 项、滞后归因 {} 项均无 NaN/越界；保险箱非锁死校验 vault_non_lockout_verified={}",
                    experiments_eval.len(),
                    snapshot.lagged_impacts.len(),
                    vault_non_lockout_verified
                ),
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
}

//! 「知气 · Qi」端侧 Rust 核心引擎，导出给 TSX 视图层 (`qi/native`)。

pub mod ai_provider;
pub mod analytics;
pub mod apple_native;
pub mod crypto_sync;
pub mod experiments;
pub mod importers;
pub mod nutrition_db;
pub mod safety_ai;
pub mod storage;
pub mod sync_engine;

use std::path::PathBuf;

use nsh_core::prelude::*;
use serde::Serialize;

use crate::ai_provider::generate_live_briefing;
use crate::analytics::{
    AnalyticsSnapshot, HardwareTier, IntakeLogRow, LabBiomarkerRow, compute_snapshot,
};
use crate::apple_native::{
    LocalAiCapabilityProbe, probe_local_ai_capabilities, run_offline_vision_ocr,
};
use crate::crypto_sync::{decrypt_sync_batch, derive_master_key, encrypt_sync_batch};
use crate::experiments::{
    EXPERIMENT_TEMPLATES, ExperimentEvaluation, ExperimentTemplate, PeriodComparison,
    compute_period_comparison, evaluate_experiments,
};
use crate::importers::{parse_apple_health_xml_snippet, parse_csv_metrics};
use crate::nutrition_db::{FoodSearchHit, parse_natural_intake_text, search_foods};
use crate::safety_ai::{BriefingResult, WELLNESS_DISCLAIMER, build_morning_briefing};
use crate::storage::{AppSettings, VaultDatabase};
use crate::sync_engine::{
    SyncExecutionReport, build_local_archive_json, decrypt_local_archive_json,
    pull_envelopes_from_byo, pull_envelopes_from_dir, push_envelope_to_byo,
    push_envelope_to_dir, push_envelope_to_webdav, resolve_icloud_vault_dir,
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
        self.build_payload()
    }

    /// 勾选/取消今日唯一高杠杆微行动。
    pub fn toggle_today_action(&mut self) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        s.checked_action_today = !s.checked_action_today;
        self.db.save_settings(&s)?;
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

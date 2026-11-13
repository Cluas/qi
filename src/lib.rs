//! 「知气 · Qi」端侧 Rust 核心引擎，导出给 TSX 视图层 (`qi/native`)。

pub mod analytics;
pub mod crypto_sync;
pub mod safety_ai;
pub mod storage;

use std::path::PathBuf;

use nsh_core::prelude::*;
use serde::Serialize;

use crate::analytics::{AnalyticsSnapshot, HardwareTier, IntakeLogRow, LabBiomarkerRow, compute_snapshot};
use crate::crypto_sync::{decrypt_sync_batch, derive_master_key, encrypt_sync_batch};
use crate::safety_ai::{BriefingResult, WELLNESS_DISCLAIMER, build_morning_briefing};
use crate::storage::{AppSettings, VaultDatabase};

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
    last_lab_import: Option<LastLabImportFeedback>,
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
}

/// 端侧唯一可信健康引擎实例：管理 SQLCipher/SQLite、统计快照、加密同步与 AI 路由。
#[class]
pub struct QiEngine {
    db: VaultDatabase,
    last_lab_import: Option<LastLabImportFeedback>,
}

impl QiEngine {
    fn open_db(db_path: Option<String>) -> Result<VaultDatabase, String> {
        match db_path.as_deref() {
            Some(":memory:") => VaultDatabase::open_memory(),
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
        let tier = HardwareTier::from_str(&settings.hardware_tier);

        let snapshot = compute_snapshot(
            &metrics,
            &workouts,
            &intakes,
            tier,
            settings.morning_feeling,
        );
        let briefing = build_morning_briefing(
            &snapshot,
            &labs,
            &settings.ai_mode,
            &settings.lan_ai_url,
            settings.thermal_throttled,
            settings.simulated_ram_gb,
        );

        let payload = UiStatePayload {
            settings,
            snapshot,
            briefing,
            intakes,
            labs,
            last_lab_import: self.last_lab_import.clone(),
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

    /// 3 秒记一餐或节律因子，立即写入本地 SQLite 并重算滞后相关性。
    pub fn log_quick_intake(
        &mut self,
        category: String,
        item_name: String,
        tag: String,
        notes: String,
    ) -> Result<String, String> {
        let target_date = self
            .db
            .load_metrics()?
            .last()
            .map(|r| r.date.clone())
            .unwrap_or_else(|| "2026-11-13".to_owned());
        self.db
            .insert_intake(&target_date, &category, &item_name, None, &notes, &[tag.as_str()])?;
        self.build_payload()
    }

    /// 导入体检单文本：先做端侧 PII 前置遮盖，再提取指标并由 Rust 重算参考区间。
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

    /// 更新个人云同步模式与地址（iCloud / WebDAV / BYO-Server / 纯本地）。
    pub fn update_sync_config(
        &mut self,
        sync_mode: String,
        webdav_url: String,
        byo_url: String,
    ) -> Result<String, String> {
        let mut s = self.db.load_settings()?;
        s.sync_mode = sync_mode;
        s.webdav_url = webdav_url;
        s.byo_server_url = byo_url;
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
        let env = encrypt_sync_batch(&key, "qi-mobile-primary", u64::from(batch_seq), &jsonl)?;
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

    /// 导出开放标准 Compact JSONL 数据摘要。
    pub fn export_jsonl(&self) -> Result<String, String> {
        self.db.export_compact_jsonl()
    }

    /// 物理清空本机健康库并重置引导流程（App Review 5.1.1(v)）。
    pub fn wipe_vault_and_reset(&mut self) -> Result<String, String> {
        self.last_lab_import = None;
        self.db.wipe_all_data()?;
        self.build_payload()
    }

    /// 重新载入 35 天演示数据。
    pub fn reseed_demo(&mut self) -> Result<String, String> {
        self.db.seed_demo_35d()?;
        self.build_payload()
    }
}

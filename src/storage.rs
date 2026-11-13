//! 本地优先 SQLite（rusqlite bundled）存储层、35 天节律演示集、N-of-1 实验与跨端增量同步合并。

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{Duration, NaiveDate};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::analytics::{DailyMetricRow, IntakeLogRow, LabBiomarkerRow, WorkoutRow};
use crate::crypto_sync::{generate_device_id, generate_recovery_phrase};
use crate::experiments::ExperimentRow;
use crate::safety_ai::{
    ParsedLabMarkerDraft, enrich_lab_longitudinal_trends, parse_lab_text_and_verify,
    scrub_lab_text_pii,
};

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS daily_metrics_by_source (
    date TEXT NOT NULL,
    metric TEXT NOT NULL,
    source TEXT NOT NULL,
    value REAL NOT NULL,
    unit TEXT NOT NULL DEFAULT '',
    revision_ms INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (date, metric, source)
);

CREATE TABLE IF NOT EXISTS workouts (
    workout_id TEXT PRIMARY KEY,
    date TEXT NOT NULL,
    activity_type TEXT NOT NULL,
    duration_minutes REAL NOT NULL,
    calories_kcal REAL,
    source TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS intake_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    category TEXT NOT NULL,
    item_name TEXT NOT NULL,
    calories_kcal REAL,
    notes TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS lab_biomarkers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    test_date TEXT NOT NULL,
    marker_name TEXT NOT NULL,
    canonical_key TEXT NOT NULL,
    value REAL NOT NULL,
    unit TEXT NOT NULL DEFAULT '',
    ref_low REAL,
    ref_high REAL,
    flag TEXT NOT NULL DEFAULT 'normal',
    plain_note TEXT NOT NULL DEFAULT '',
    UNIQUE(test_date, canonical_key)
);

CREATE TABLE IF NOT EXISTS experiments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    hypothesis TEXT NOT NULL,
    target_days INTEGER NOT NULL DEFAULT 7,
    start_date TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    UNIQUE(title, start_date)
);

CREATE TABLE IF NOT EXISTS experiment_checkins (
    experiment_id INTEGER NOT NULL,
    date TEXT NOT NULL,
    PRIMARY KEY (experiment_id, date)
);

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub onboarding_done: bool,
    pub primary_goal: String,
    pub hardware_tier: String,
    pub morning_feeling: u32,
    pub sync_mode: String,
    pub webdav_url: String,
    pub webdav_username: String,
    pub webdav_password: String,
    pub byo_server_url: String,
    pub cloud_url: String,
    pub cloud_handle: String,
    pub cloud_token: String,
    pub ai_mode: String,
    pub zdr_consented: bool,
    pub lan_ai_url: String,
    pub lan_ai_model: String,
    pub morning_push_enabled: bool,
    pub morning_push_time: string_or!(),
    pub thermal_throttled: bool,
    pub simulated_ram_gb: u32,
    pub recovery_phrase: String,
    pub device_id: String,
    pub sync_seq: u64,
    pub pull_cursor_id: i64,
    pub last_sync_summary: String,
    pub checked_action_today: bool,
}

macro_rules! string_or {
    () => {
        String
    };
}
pub(crate) use string_or;

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            onboarding_done: false,
            primary_goal: "energy_sleep".to_owned(),
            hardware_tier: "tier_a".to_owned(),
            morning_feeling: 4,
            sync_mode: "icloud_drive".to_owned(),
            webdav_url: "https://dav.jianguoyun.com/dav/qi-vault/".to_owned(),
            webdav_username: String::new(),
            webdav_password: String::new(),
            byo_server_url: "http://127.0.0.1:8788".to_owned(),
            cloud_url: "http://127.0.0.1:8788".to_owned(),
            cloud_handle: String::new(),
            cloud_token: String::new(),
            ai_mode: "official_zdr_cloud".to_owned(),
            zdr_consented: false,
            lan_ai_url: "http://127.0.0.1:11434/v1".to_owned(),
            lan_ai_model: "qwen3:latest".to_owned(),
            morning_push_enabled: true,
            morning_push_time: "08:30".to_owned(),
            thermal_throttled: false,
            simulated_ram_gb: 8,
            recovery_phrase: generate_recovery_phrase(),
            device_id: generate_device_id(),
            sync_seq: 0,
            pull_cursor_id: 0,
            last_sync_summary: "尚未执行跨端加密同步".to_owned(),
            checked_action_today: false,
        }
    }
}

pub struct VaultDatabase {
    conn: Connection,
}

impl VaultDatabase {
    pub fn open_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        Self::init_conn(conn, true)
    }

    /// 供跨设备 12 词恢复测试使用：打开严格空白数据库（不预置演示数据）
    pub fn open_empty_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        Self::init_conn(conn, false)
    }

    pub fn open_file(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let _ = conn.execute_batch("PRAGMA journal_mode = WAL;");
        Self::init_conn(conn, true)
    }

    fn init_conn(conn: Connection, auto_seed: bool) -> Result<Self, String> {
        conn.execute_batch(SCHEMA_SQL).map_err(|e| e.to_string())?;
        let db = Self { conn };
        db.ensure_defaults(auto_seed)?;
        Ok(db)
    }

    fn ensure_defaults(&self, auto_seed: bool) -> Result<(), String> {
        let settings_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM app_settings", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if settings_count == 0 {
            self.save_settings(&AppSettings::default())?;
        }
        if auto_seed {
            let count: i64 = self
                .conn
                .query_row("SELECT COUNT(*) FROM daily_metrics_by_source", [], |r| r.get(0))
                .map_err(|e| e.to_string())?;
            if count == 0 {
                self.seed_demo_35d()?;
            }
        }
        Ok(())
    }

    pub fn save_settings(&self, s: &AppSettings) -> Result<(), String> {
        let pairs = [
            ("onboarding_done", if s.onboarding_done { "1" } else { "0" }.to_owned()),
            ("primary_goal", s.primary_goal.clone()),
            ("hardware_tier", s.hardware_tier.clone()),
            ("morning_feeling", s.morning_feeling.to_string()),
            ("sync_mode", s.sync_mode.clone()),
            ("webdav_url", s.webdav_url.clone()),
            ("webdav_username", s.webdav_username.clone()),
            ("webdav_password", s.webdav_password.clone()),
            ("byo_server_url", s.byo_server_url.clone()),
            ("cloud_url", s.cloud_url.clone()),
            ("cloud_handle", s.cloud_handle.clone()),
            ("cloud_token", s.cloud_token.clone()),
            ("ai_mode", s.ai_mode.clone()),
            ("zdr_consented", if s.zdr_consented { "1" } else { "0" }.to_owned()),
            ("lan_ai_url", s.lan_ai_url.clone()),
            ("lan_ai_model", s.lan_ai_model.clone()),
            (
                "morning_push_enabled",
                if s.morning_push_enabled { "1" } else { "0" }.to_owned(),
            ),
            ("morning_push_time", s.morning_push_time.clone()),
            (
                "thermal_throttled",
                if s.thermal_throttled { "1" } else { "0" }.to_owned(),
            ),
            ("simulated_ram_gb", s.simulated_ram_gb.to_string()),
            ("recovery_phrase", s.recovery_phrase.clone()),
            ("device_id", s.device_id.clone()),
            ("sync_seq", s.sync_seq.to_string()),
            ("pull_cursor_id", s.pull_cursor_id.to_string()),
            ("last_sync_summary", s.last_sync_summary.clone()),
            (
                "checked_action_today",
                if s.checked_action_today { "1" } else { "0" }.to_owned(),
            ),
        ];
        for (k, v) in pairs {
            self.conn
                .execute(
                    "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![k, v],
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn load_settings(&self) -> Result<AppSettings, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM app_settings")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;

        let mut map = BTreeMap::new();
        for r in rows {
            let (k, v) = r.map_err(|e| e.to_string())?;
            map.insert(k, v);
        }

        let mut s = AppSettings::default();
        if let Some(v) = map.get("onboarding_done") { s.onboarding_done = v == "1"; }
        if let Some(v) = map.get("primary_goal") { s.primary_goal = v.clone(); }
        if let Some(v) = map.get("hardware_tier") { s.hardware_tier = v.clone(); }
        if let Some(v) = map.get("morning_feeling") { s.morning_feeling = v.parse().unwrap_or(4); }
        if let Some(v) = map.get("sync_mode") { s.sync_mode = v.clone(); }
        if let Some(v) = map.get("webdav_url") { s.webdav_url = v.clone(); }
        if let Some(v) = map.get("webdav_username") { s.webdav_username = v.clone(); }
        if let Some(v) = map.get("webdav_password") { s.webdav_password = v.clone(); }
        if let Some(v) = map.get("byo_server_url") { s.byo_server_url = v.clone(); }
        if let Some(v) = map.get("cloud_url") { s.cloud_url = v.clone(); }
        if let Some(v) = map.get("cloud_handle") { s.cloud_handle = v.clone(); }
        if let Some(v) = map.get("cloud_token") { s.cloud_token = v.clone(); }
        if let Some(v) = map.get("ai_mode") { s.ai_mode = v.clone(); }
        if let Some(v) = map.get("zdr_consented") { s.zdr_consented = v == "1"; }
        if let Some(v) = map.get("lan_ai_url") { s.lan_ai_url = v.clone(); }
        if let Some(v) = map.get("lan_ai_model") { s.lan_ai_model = v.clone(); }
        if let Some(v) = map.get("morning_push_enabled") { s.morning_push_enabled = v == "1"; }
        if let Some(v) = map.get("morning_push_time") { s.morning_push_time = v.clone(); }
        if let Some(v) = map.get("thermal_throttled") { s.thermal_throttled = v == "1"; }
        if let Some(v) = map.get("simulated_ram_gb") { s.simulated_ram_gb = v.parse().unwrap_or(8); }
        if let Some(v) = map.get("recovery_phrase") { s.recovery_phrase = v.clone(); }
        if let Some(v) = map.get("device_id") { s.device_id = v.clone(); }
        if let Some(v) = map.get("sync_seq") { s.sync_seq = v.parse().unwrap_or(0); }
        if let Some(v) = map.get("pull_cursor_id") { s.pull_cursor_id = v.parse().unwrap_or(0); }
        if let Some(v) = map.get("last_sync_summary") { s.last_sync_summary = v.clone(); }
        if let Some(v) = map.get("checked_action_today") { s.checked_action_today = v == "1"; }
        Ok(s)
    }

    pub fn upsert_daily_metric(&self, row: &DailyMetricRow) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO daily_metrics_by_source (date, metric, source, value, unit, revision_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1)
                 ON CONFLICT(date, metric, source) DO UPDATE SET value = excluded.value, unit = excluded.unit",
                params![row.date, row.metric, row.source, row.value, row.unit],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn seed_demo_35d(&self) -> Result<(), String> {
        let start = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
        for i in 0..35_i64 {
            let d = (start + Duration::days(i)).format("%Y-%m-%d").to_string();
            let late_coffee_effect = i == 12 || i == 23 || i == 31;
            let alcohol_effect = i == 18 || i == 27;

            let rmssd = if alcohol_effect {
                41.0
            } else if late_coffee_effect {
                46.5
            } else {
                55.0 + ((i * 7) % 6) as f64
            };
            let sdnn = 43.0 + ((i * 5) % 5) as f64;
            let rhr = if alcohol_effect {
                61.0
            } else if late_coffee_effect {
                58.0
            } else {
                53.0 + (i % 3) as f64
            };
            let sleep_min = if late_coffee_effect || alcohol_effect {
                375.0
            } else {
                442.0 + ((i * 3) % 24) as f64
            };
            let deep_min = if alcohol_effect {
                52.0
            } else if late_coffee_effect {
                68.0
            } else {
                88.0 + (i % 14) as f64
            };
            let steps = 7200.0 + ((i * 650) % 4500) as f64;
            let azm = if i % 2 == 0 { 38.0 } else { 18.0 };

            for (metric, source, val, unit) in [
                ("hrv_rmssd_ms", "garmin", rmssd, "ms"),
                ("hrv_sdnn_ms", "apple_health", sdnn, "ms"),
                ("resting_heart_rate_bpm", "garmin", rhr, "bpm"),
                ("sleep_minutes", "garmin", sleep_min, "min"),
                ("sleep_deep_minutes", "garmin", deep_min, "min"),
                ("steps", "apple_health", steps, "步"),
                ("active_zone_minutes", "garmin", azm, "min"),
            ] {
                self.upsert_daily_metric(&DailyMetricRow {
                    date: d.clone(),
                    metric: metric.to_owned(),
                    source: source.to_owned(),
                    value: val,
                    unit: unit.to_owned(),
                })?;
            }

            if i % 3 == 0 {
                self.conn
                    .execute(
                        "INSERT OR IGNORE INTO workouts (workout_id, date, activity_type, duration_minutes, calories_kcal, source)
                         VALUES (?1, ?2, ?3, ?4, ?5, 'garmin')",
                        params![format!("w-{i}"), d, "Zone 2 慢跑", 42.0, 340.0],
                    )
                    .map_err(|e| e.to_string())?;
            }

            if i == 11 || i == 22 || i == 30 {
                self.insert_intake(&d, "caffeine", "15:30 冰美式", Some(15.0), "下午工作提神", &["晚间咖啡"])?;
            }
            if i == 17 || i == 26 {
                self.insert_intake(&d, "alcohol", "晚间精酿小酌", Some(260.0), "朋友聚餐", &["晚间饮酒"])?;
            }
        }

        // 预置多期体检档案，立即可见纵向改善对比箭头
        let _ = self.save_parsed_lab(
            "2026-06-15",
            &ParsedLabMarkerDraft {
                marker_name: "25-羟维生素D".into(),
                canonical_key: "vitamin_d".into(),
                value: 16.2,
                unit: "ng/mL".into(),
                ref_low: Some(30.0),
                ref_high: Some(100.0),
                flag: "low".into(),
                plain_note: "维生素 D 偏低：建议每日上午适度户外日照 15 分钟。".into(),
            },
        );
        let _ = self.save_parsed_lab(
            "2026-10-20",
            &ParsedLabMarkerDraft {
                marker_name: "25-羟维生素D".into(),
                canonical_key: "vitamin_d".into(),
                value: 24.8,
                unit: "ng/mL".into(),
                ref_low: Some(30.0),
                ref_high: Some(100.0),
                flag: "low".into(),
                plain_note: "维生素 D 较上期回升，继续每日日照 15 分钟并随餐补充。".into(),
            },
        );
        let _ = self.save_parsed_lab(
            "2026-10-20",
            &ParsedLabMarkerDraft {
                marker_name: "空腹血糖 (GLU)".into(),
                canonical_key: "fasting_glucose".into(),
                value: 4.92,
                unit: "mmol/L".into(),
                ref_low: Some(3.9),
                ref_high: Some(6.1),
                flag: "normal".into(),
                plain_note: "位于常规参考区间内，继续保持良好作息与饮食平衡即可。".into(),
            },
        );

        // 预置 1 个 N-of-1 微习惯实验室对照条目
        let exp_id = self.create_experiment_if_missing(
            "15:00 后零咖啡因",
            "减少下午半衰期咖啡因残留，观察当晚深睡时长与次日夜间 HRV",
            7,
            "2026-11-05",
        )?;
        for offset in 0..5_i64 {
            let d = (NaiveDate::from_ymd_opt(2026, 11, 5).unwrap() + Duration::days(offset))
                .format("%Y-%m-%d")
                .to_string();
            let _ = self.conn.execute(
                "INSERT OR IGNORE INTO experiment_checkins (experiment_id, date) VALUES (?1, ?2)",
                params![exp_id, d],
            );
        }

        Ok(())
    }

    pub fn create_experiment_if_missing(
        &self,
        title: &str,
        hypothesis: &str,
        target_days: u32,
        start_date: &str,
    ) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO experiments (title, hypothesis, target_days, start_date, status)
                 VALUES (?1, ?2, ?3, ?4, 'active')",
                params![title.trim(), hypothesis.trim(), target_days, start_date],
            )
            .map_err(|e| e.to_string())?;
        let id: i64 = self
            .conn
            .query_row(
                "SELECT id FROM experiments WHERE title = ?1 AND start_date = ?2",
                params![title.trim(), start_date],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub fn toggle_experiment_checkin(&self, experiment_id: i64, date: &str) -> Result<bool, String> {
        let exists: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM experiment_checkins WHERE experiment_id = ?1 AND date = ?2",
                params![experiment_id, date],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if exists > 0 {
            self.conn
                .execute(
                    "DELETE FROM experiment_checkins WHERE experiment_id = ?1 AND date = ?2",
                    params![experiment_id, date],
                )
                .map_err(|e| e.to_string())?;
            Ok(false)
        } else {
            self.conn
                .execute(
                    "INSERT INTO experiment_checkins (experiment_id, date) VALUES (?1, ?2)",
                    params![experiment_id, date],
                )
                .map_err(|e| e.to_string())?;
            Ok(true)
        }
    }

    pub fn load_experiments(&self) -> Result<Vec<ExperimentRow>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, hypothesis, target_days, start_date, status FROM experiments ORDER BY id DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, u32>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut out = Vec::new();
        for (id, title, hypothesis, target_days, start_date, status) in rows {
            let mut c_stmt = self
                .conn
                .prepare("SELECT date FROM experiment_checkins WHERE experiment_id = ?1 ORDER BY date ASC")
                .map_err(|e| e.to_string())?;
            let dates = c_stmt
                .query_map(params![id], |r| r.get(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<String>, _>>()
                .map_err(|e| e.to_string())?;
            out.push(ExperimentRow {
                id,
                title,
                hypothesis,
                target_days,
                start_date,
                status,
                checkin_dates: dates,
            });
        }
        Ok(out)
    }

    pub fn insert_intake(
        &self,
        date: &str,
        category: &str,
        item_name: &str,
        calories_kcal: Option<f64>,
        notes: &str,
        tags: &[&str],
    ) -> Result<(), String> {
        let tags_vec: Vec<String> = tags.iter().map(|s| (*s).to_owned()).collect();
        let tags_json = serde_json::to_string(&tags_vec).map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "INSERT INTO intake_logs (date, category, item_name, calories_kcal, notes, tags_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![date, category, item_name, calories_kcal, notes, tags_json],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_intake_idempotent(
        &self,
        date: &str,
        category: &str,
        item_name: &str,
        calories_kcal: Option<f64>,
        notes: &str,
        tags: &[String],
    ) -> Result<(), String> {
        let exists: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM intake_logs WHERE date = ?1 AND item_name = ?2 AND category = ?3",
                params![date, item_name, category],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if exists == 0 {
            let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
            self.insert_intake(date, category, item_name, calories_kcal, notes, &tag_refs)?;
        }
        Ok(())
    }

    pub fn import_lab_text(
        &self,
        test_date: &str,
        raw_text: &str,
    ) -> Result<(String, Vec<String>, Vec<ParsedLabMarkerDraft>), String> {
        let scrub = scrub_lab_text_pii(raw_text);
        let parsed = parse_lab_text_and_verify(&scrub.scrubbed_text);
        for draft in &parsed {
            self.save_parsed_lab(test_date, draft)?;
        }
        Ok((scrub.scrubbed_text, scrub.redacted_fields, parsed))
    }

    pub fn save_parsed_lab(&self, test_date: &str, d: &ParsedLabMarkerDraft) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO lab_biomarkers (test_date, marker_name, canonical_key, value, unit, ref_low, ref_high, flag, plain_note)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(test_date, canonical_key) DO UPDATE SET
                    marker_name = excluded.marker_name,
                    value = excluded.value,
                    unit = excluded.unit,
                    ref_low = excluded.ref_low,
                    ref_high = excluded.ref_high,
                    flag = excluded.flag,
                    plain_note = excluded.plain_note",
                params![
                    test_date,
                    d.marker_name,
                    d.canonical_key,
                    d.value,
                    d.unit,
                    d.ref_low,
                    d.ref_high,
                    d.flag,
                    d.plain_note
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_metrics(&self) -> Result<Vec<DailyMetricRow>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT date, metric, source, value, unit FROM daily_metrics_by_source ORDER BY date ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(DailyMetricRow {
                    date: r.get(0)?,
                    metric: r.get(1)?,
                    source: r.get(2)?,
                    value: r.get(3)?,
                    unit: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn load_workouts(&self) -> Result<Vec<WorkoutRow>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT date, activity_type, duration_minutes, calories_kcal FROM workouts ORDER BY date ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(WorkoutRow {
                    date: r.get(0)?,
                    activity_type: r.get(1)?,
                    duration_minutes: r.get(2)?,
                    calories_kcal: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn load_intakes(&self) -> Result<Vec<IntakeLogRow>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, date, category, item_name, calories_kcal, notes, tags_json FROM intake_logs ORDER BY id DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let tags_raw: String = r.get(6)?;
                let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
                Ok(IntakeLogRow {
                    id: r.get(0)?,
                    date: r.get(1)?,
                    category: r.get(2)?,
                    item_name: r.get(3)?,
                    calories_kcal: r.get(4)?,
                    notes: r.get(5)?,
                    tags,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn load_labs(&self) -> Result<Vec<LabBiomarkerRow>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, test_date, marker_name, canonical_key, value, unit, ref_low, ref_high, flag, plain_note FROM lab_biomarkers ORDER BY test_date DESC, id DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(LabBiomarkerRow {
                    id: r.get(0)?,
                    test_date: r.get(1)?,
                    marker_name: r.get(2)?,
                    canonical_key: r.get(3)?,
                    value: r.get(4)?,
                    unit: r.get(5)?,
                    ref_low: r.get(6)?,
                    ref_high: r.get(7)?,
                    flag: r.get(8)?,
                    plain_note: r.get(9)?,
                    prev_value: None,
                    prev_date: None,
                    delta: None,
                    trend: None,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(enrich_lab_longitudinal_trends(rows))
    }

    pub fn export_compact_jsonl(&self) -> Result<String, String> {
        let metrics = self.load_metrics()?;
        let mut lines = Vec::new();
        for m in metrics.iter().rev().take(12) {
            lines.push(format!(
                r#"{{"date":"{}","canonical":"{}","source":"{}","value":{},"unit":"{}"}}"#,
                m.date, m.metric, m.source, m.value, m.unit
            ));
        }
        Ok(lines.join("\n"))
    }

    /// 全量增量同步编码：将指标、运动、饮食、体检与实验打包为多行幂等 JSONL
    pub fn export_full_sync_jsonl(&self) -> Result<String, String> {
        let mut lines = Vec::new();
        for m in self.load_metrics()? {
            lines.push(
                json!({
                    "kind": "metric",
                    "date": m.date,
                    "metric": m.metric,
                    "source": m.source,
                    "value": m.value,
                    "unit": m.unit
                })
                .to_string(),
            );
        }
        for log in self.load_intakes()? {
            lines.push(
                json!({
                    "kind": "intake",
                    "date": log.date,
                    "category": log.category,
                    "item_name": log.item_name,
                    "calories_kcal": log.calories_kcal,
                    "notes": log.notes,
                    "tags": log.tags
                })
                .to_string(),
            );
        }
        for lab in self.load_labs()? {
            lines.push(
                json!({
                    "kind": "lab",
                    "test_date": lab.test_date,
                    "marker_name": lab.marker_name,
                    "canonical_key": lab.canonical_key,
                    "value": lab.value,
                    "unit": lab.unit,
                    "ref_low": lab.ref_low,
                    "ref_high": lab.ref_high,
                    "flag": lab.flag,
                    "plain_note": lab.plain_note
                })
                .to_string(),
            );
        }
        for exp in self.load_experiments()? {
            lines.push(
                json!({
                    "kind": "experiment",
                    "title": exp.title,
                    "hypothesis": exp.hypothesis,
                    "target_days": exp.target_days,
                    "start_date": exp.start_date,
                    "checkin_dates": exp.checkin_dates
                })
                .to_string(),
            );
        }
        Ok(lines.join("\n"))
    }

    /// 解密远端分片后，按天然幂等键合并入本地 SQLite，返还本次合并条数
    pub fn merge_sync_jsonl(&self, jsonl: &str) -> Result<usize, String> {
        let mut merged = 0usize;
        for line in jsonl.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let val: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match val["kind"].as_str().unwrap_or("metric") {
                "metric" => {
                    if let (Some(date), Some(metric), Some(value)) = (
                        val["date"].as_str(),
                        val["metric"].as_str().or_else(|| val["canonical"].as_str()),
                        val["value"].as_f64(),
                    ) {
                        self.upsert_daily_metric(&DailyMetricRow {
                            date: date.to_owned(),
                            metric: metric.to_owned(),
                            source: val["source"].as_str().unwrap_or("synced").to_owned(),
                            value,
                            unit: val["unit"].as_str().unwrap_or("").to_owned(),
                        })?;
                        merged += 1;
                    }
                }
                "intake" => {
                    if let (Some(date), Some(item)) =
                        (val["date"].as_str(), val["item_name"].as_str())
                    {
                        let tags: Vec<String> = val["tags"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|t| t.as_str().map(|s| s.to_owned()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        self.insert_intake_idempotent(
                            date,
                            val["category"].as_str().unwrap_or("meal"),
                            item,
                            val["calories_kcal"].as_f64(),
                            val["notes"].as_str().unwrap_or(""),
                            &tags,
                        )?;
                        merged += 1;
                    }
                }
                "lab" => {
                    if let (Some(test_date), Some(name), Some(key), Some(val_num)) = (
                        val["test_date"].as_str(),
                        val["marker_name"].as_str(),
                        val["canonical_key"].as_str(),
                        val["value"].as_f64(),
                    ) {
                        self.save_parsed_lab(
                            test_date,
                            &ParsedLabMarkerDraft {
                                marker_name: name.to_owned(),
                                canonical_key: key.to_owned(),
                                value: val_num,
                                unit: val["unit"].as_str().unwrap_or("").to_owned(),
                                ref_low: val["ref_low"].as_f64(),
                                ref_high: val["ref_high"].as_f64(),
                                flag: val["flag"].as_str().unwrap_or("normal").to_owned(),
                                plain_note: val["plain_note"].as_str().unwrap_or("").to_owned(),
                            },
                        )?;
                        merged += 1;
                    }
                }
                "experiment" => {
                    if let (Some(title), Some(hyp), Some(sd)) = (
                        val["title"].as_str(),
                        val["hypothesis"].as_str(),
                        val["start_date"].as_str(),
                    ) {
                        let days = val["target_days"].as_u64().unwrap_or(7) as u32;
                        let exp_id = self.create_experiment_if_missing(title, hyp, days, sd)?;
                        if let Some(arr) = val["checkin_dates"].as_array() {
                            for d in arr.iter().filter_map(|x| x.as_str()) {
                                let _ = self.conn.execute(
                                    "INSERT OR IGNORE INTO experiment_checkins (experiment_id, date) VALUES (?1, ?2)",
                                    params![exp_id, d],
                                );
                            }
                        }
                        merged += 1;
                    }
                }
                _ => {}
            }
        }
        Ok(merged)
    }

    /// 合规物理销毁本机健康库（App Review 5.1.1(v)）
    pub fn wipe_all_data(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "DELETE FROM daily_metrics_by_source;
                 DELETE FROM workouts;
                 DELETE FROM intake_logs;
                 DELETE FROM lab_biomarkers;
                 DELETE FROM experiment_checkins;
                 DELETE FROM experiments;",
            )
            .map_err(|e| e.to_string())?;
        let mut s = self.load_settings()?;
        s.onboarding_done = false;
        s.checked_action_today = false;
        s.cloud_token.clear();
        s.cloud_handle.clear();
        s.last_sync_summary = "本机与会话凭证已物理清空".to_owned();
        self.save_settings(&s)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_35_days_and_wipes_cleanly() {
        let db = VaultDatabase::open_memory().unwrap();
        assert!(db.load_metrics().unwrap().len() >= 200);
        assert!(!db.load_intakes().unwrap().is_empty());
        assert!(!db.load_labs().unwrap().is_empty());
        assert!(!db.load_experiments().unwrap().is_empty());

        // 验证维 D 多期纵向变化趋势已计算
        let labs = db.load_labs().unwrap();
        let vd_latest = labs
            .iter()
            .find(|l| l.canonical_key == "vitamin_d" && l.test_date == "2026-10-20")
            .unwrap();
        assert_eq!(vd_latest.trend.as_deref(), Some("improved"));

        db.wipe_all_data().unwrap();
        assert!(db.load_metrics().unwrap().is_empty());
        assert!(db.load_labs().unwrap().is_empty());
        assert!(db.load_experiments().unwrap().is_empty());
    }
}

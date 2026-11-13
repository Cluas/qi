//! 本地优先 SQLite（rusqlite bundled）存储层与 35 天真实节律演示数据集。

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{Duration, NaiveDate};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::analytics::{DailyMetricRow, IntakeLogRow, LabBiomarkerRow, WorkoutRow};
use crate::crypto_sync::generate_recovery_phrase;
use crate::safety_ai::{ParsedLabMarkerDraft, parse_lab_text_and_verify, scrub_lab_text_pii};

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
    pub byo_server_url: String,
    pub ai_mode: String,
    pub zdr_consented: bool,
    pub lan_ai_url: String,
    pub lan_ai_model: String,
    pub morning_push_enabled: bool,
    pub morning_push_time: String,
    pub thermal_throttled: bool,
    pub simulated_ram_gb: u32,
    pub recovery_phrase: String,
    pub checked_action_today: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            onboarding_done: false,
            primary_goal: "energy_sleep".to_owned(),
            hardware_tier: "tier_a".to_owned(),
            morning_feeling: 4,
            sync_mode: "icloud_drive".to_owned(),
            webdav_url: "https://dav.jianguoyun.com/dav/qi-vault/".to_owned(),
            byo_server_url: "https://health.home.cluas.eu.org".to_owned(),
            ai_mode: "official_zdr_cloud".to_owned(),
            zdr_consented: false,
            lan_ai_url: "http://192.168.2.18:11434/v1".to_owned(),
            lan_ai_model: "qwen3:8b".to_owned(),
            morning_push_enabled: true,
            morning_push_time: "08:30".to_owned(),
            thermal_throttled: false,
            simulated_ram_gb: 8,
            recovery_phrase: generate_recovery_phrase(),
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
        Self::init_conn(conn)
    }

    pub fn open_file(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let _ = conn.execute_batch("PRAGMA journal_mode = WAL;");
        Self::init_conn(conn)
    }

    fn init_conn(conn: Connection) -> Result<Self, String> {
        conn.execute_batch(SCHEMA_SQL).map_err(|e| e.to_string())?;
        let db = Self { conn };
        db.ensure_defaults_and_seed()?;
        Ok(db)
    }

    fn ensure_defaults_and_seed(&self) -> Result<(), String> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM daily_metrics_by_source", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let settings_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM app_settings", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;

        if settings_count == 0 {
            self.save_settings(&AppSettings::default())?;
        }
        if count == 0 {
            self.seed_demo_35d()?;
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
            ("byo_server_url", s.byo_server_url.clone()),
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
        if let Some(v) = map.get("onboarding_done") {
            s.onboarding_done = v == "1";
        }
        if let Some(v) = map.get("primary_goal") {
            s.primary_goal = v.clone();
        }
        if let Some(v) = map.get("hardware_tier") {
            s.hardware_tier = v.clone();
        }
        if let Some(v) = map.get("morning_feeling") {
            s.morning_feeling = v.parse().unwrap_or(4);
        }
        if let Some(v) = map.get("sync_mode") {
            s.sync_mode = v.clone();
        }
        if let Some(v) = map.get("webdav_url") {
            s.webdav_url = v.clone();
        }
        if let Some(v) = map.get("byo_server_url") {
            s.byo_server_url = v.clone();
        }
        if let Some(v) = map.get("ai_mode") {
            s.ai_mode = v.clone();
        }
        if let Some(v) = map.get("zdr_consented") {
            s.zdr_consented = v == "1";
        }
        if let Some(v) = map.get("lan_ai_url") {
            s.lan_ai_url = v.clone();
        }
        if let Some(v) = map.get("lan_ai_model") {
            s.lan_ai_model = v.clone();
        }
        if let Some(v) = map.get("morning_push_enabled") {
            s.morning_push_enabled = v == "1";
        }
        if let Some(v) = map.get("morning_push_time") {
            s.morning_push_time = v.clone();
        }
        if let Some(v) = map.get("thermal_throttled") {
            s.thermal_throttled = v == "1";
        }
        if let Some(v) = map.get("simulated_ram_gb") {
            s.simulated_ram_gb = v.parse().unwrap_or(8);
        }
        if let Some(v) = map.get("recovery_phrase") {
            s.recovery_phrase = v.clone();
        }
        if let Some(v) = map.get("checked_action_today") {
            s.checked_action_today = v == "1";
        }
        Ok(s)
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
                self.conn
                    .execute(
                        "INSERT INTO daily_metrics_by_source (date, metric, source, value, unit, revision_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5, 1)
                         ON CONFLICT(date, metric, source) DO UPDATE SET value = excluded.value",
                        params![d, metric, source, val, unit],
                    )
                    .map_err(|e| e.to_string())?;
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

            // 在对应前一天插入饮食因子，支撑 D -> D+1 滞后相关性分析展示
            if i == 11 || i == 22 || i == 30 {
                self.insert_intake(&d, "caffeine", "15:30 冰美式", Some(15.0), "下午工作提神", &["晚间咖啡"])?;
            }
            if i == 17 || i == 26 {
                self.insert_intake(&d, "alcohol", "晚间精酿小酌", Some(260.0), "朋友聚餐", &["晚间饮酒"])?;
            }
        }

        // 预置 2 条体检档案
        let _ = self.save_parsed_lab(
            "2026-10-20",
            &ParsedLabMarkerDraft {
                marker_name: "25-羟维生素D".into(),
                canonical_key: "vitamin_d".into(),
                value: 22.4,
                unit: "ng/mL".into(),
                ref_low: Some(30.0),
                ref_high: Some(100.0),
                flag: "low".into(),
                plain_note: "维生素 D 偏低：建议每日上午适度户外日照 15 分钟，可在膳食中搭配富含维 D 食材。".into(),
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

        Ok(())
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
                    value = excluded.value,
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
            .prepare("SELECT id, test_date, marker_name, canonical_key, value, unit, ref_low, ref_high, flag, plain_note FROM lab_biomarkers ORDER BY id DESC")
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
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
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

    /// 合规物理销毁本机健康库（App Review 5.1.1(v)）
    pub fn wipe_all_data(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "DELETE FROM daily_metrics_by_source;
                 DELETE FROM workouts;
                 DELETE FROM intake_logs;
                 DELETE FROM lab_biomarkers;",
            )
            .map_err(|e| e.to_string())?;
        let mut s = self.load_settings()?;
        s.onboarding_done = false;
        s.checked_action_today = false;
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

        db.wipe_all_data().unwrap();
        assert!(db.load_metrics().unwrap().is_empty());
        assert!(db.load_labs().unwrap().is_empty());
    }
}

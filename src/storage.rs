//! 本地优先 SQLite（rusqlite bundled）存储层、35 天节律演示集、N-of-1 实验与跨端增量同步合并。

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::analytics::{DailyMetricRow, IntakeLogRow, LabBiomarkerRow, WorkoutRow};
use crate::crypto_sync::{generate_device_id, generate_recovery_phrase};
use crate::entitlements::{DEFAULT_LOCALE, is_en_locale, normalize_locale};
use crate::experiments::ExperimentRow;
use crate::safety_ai::{
    ParsedLabMarkerDraft, biomarker_plain_note, enrich_lab_longitudinal_trends,
    parse_lab_text_and_verify, scrub_lab_text_pii,
};
use crate::solar_atelier::{
    CustomPetCompanion, DewLedgerEntry, FoodSticker, PetWeeklyLetter, SanctuaryPreset,
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
    profile_id TEXT NOT NULL DEFAULT 'self',
    test_date TEXT NOT NULL,
    marker_name TEXT NOT NULL,
    canonical_key TEXT NOT NULL,
    value REAL NOT NULL,
    unit TEXT NOT NULL DEFAULT '',
    ref_low REAL,
    ref_high REAL,
    reference_high REAL,
    flag TEXT NOT NULL DEFAULT 'normal',
    plain_note TEXT NOT NULL DEFAULT '',
    UNIQUE(profile_id, test_date, canonical_key)
);

CREATE TABLE IF NOT EXISTS experiments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    hypothesis TEXT NOT NULL,
    target_days INTEGER NOT NULL DEFAULT 7,
    start_date TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    washout_days INTEGER NOT NULL DEFAULT 2,
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

CREATE TABLE IF NOT EXISTS dew_ledger (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL,
    delta INTEGER NOT NULL,
    source TEXT NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    reward_id TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS sticker_journal (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    title TEXT NOT NULL,
    filter TEXT NOT NULL DEFAULT 'polaroid',
    note TEXT NOT NULL DEFAULT '',
    nutrition_hint TEXT NOT NULL DEFAULT '',
    image_ref TEXT NOT NULL DEFAULT '',
    is_favorite INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS sanctuary_presets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    world_id TEXT NOT NULL,
    weather_mood TEXT NOT NULL,
    seal_ink TEXT NOT NULL DEFAULT 'cinnabar',
    keepsakes_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS custom_pets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    species TEXT NOT NULL DEFAULT 'cat',
    persona TEXT NOT NULL DEFAULT '',
    photo_data_url TEXT NOT NULL DEFAULT '',
    die_cut_frame TEXT NOT NULL DEFAULT 'celadon_oval',
    starlight_memorial INTEGER NOT NULL DEFAULT 0,
    paw_stamp_title TEXT NOT NULL DEFAULT '',
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS pet_weekly_letters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pet_id TEXT NOT NULL,
    pet_name TEXT NOT NULL,
    species TEXT NOT NULL,
    week_number INTEGER NOT NULL,
    solar_term TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    ex_libris_seal TEXT NOT NULL,
    starlight_memorial INTEGER NOT NULL DEFAULT 0,
    locale TEXT NOT NULL DEFAULT 'zh-CN',
    created_at TEXT NOT NULL
);
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_persona")]
    pub active_persona: String,
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
    #[serde(default)]
    pub entitlement_token: String,
    #[serde(default)]
    pub max_seen_epoch: i64,
    #[serde(default)]
    pub guardian_sponsor: bool,
    #[serde(default)]
    pub companion_loyalty_days: i64,
    #[serde(default = "default_sandbox_tier")]
    pub sandbox_tier: String,
    #[serde(default = "default_sandbox_state")]
    pub sandbox_state: String,
    #[serde(default = "default_weather_mood")]
    pub weather_mood: String,
    #[serde(default = "default_widget_privacy_mode")]
    pub widget_privacy_mode: String,
    #[serde(default = "default_empty_json_array")]
    pub equipped_keepsakes_json: String,
    #[serde(default = "default_pair_code")]
    pub pair_code: String,
    #[serde(default)]
    pub partner_name: String,
    #[serde(default)]
    pub partner_mood: i64,
    #[serde(default)]
    pub bonsai_dew: i64,
    #[serde(default = "default_seal_ink")]
    pub active_seal_ink: String,
    #[serde(default = "default_locale")]
    pub locale: String,
}

macro_rules! string_or {
    () => {
        String
    };
}
pub(crate) use string_or;

fn default_persona() -> String {
    "urban_overwork".to_owned()
}
fn default_sandbox_tier() -> String {
    "free".to_owned()
}
fn default_sandbox_state() -> String {
    "active".to_owned()
}
fn default_weather_mood() -> String {
    "sunny_crisp".to_owned()
}
fn default_widget_privacy_mode() -> String {
    "metaphor".to_owned()
}
fn default_empty_json_array() -> String {
    "[]".to_owned()
}
fn default_pair_code() -> String {
    "QI-2026".to_owned()
}
fn default_seal_ink() -> String {
    "cinnabar".to_owned()
}
fn default_locale() -> String {
    DEFAULT_LOCALE.to_owned()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            active_persona: default_persona(),
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
            entitlement_token: String::new(),
            max_seen_epoch: 0,
            guardian_sponsor: false,
            companion_loyalty_days: 0,
            sandbox_tier: default_sandbox_tier(),
            sandbox_state: default_sandbox_state(),
            weather_mood: default_weather_mood(),
            widget_privacy_mode: default_widget_privacy_mode(),
            equipped_keepsakes_json: default_empty_json_array(),
            pair_code: default_pair_code(),
            partner_name: String::new(),
            partner_mood: 0,
            bonsai_dew: 0,
            active_seal_ink: default_seal_ink(),
            locale: default_locale(),
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
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!("无法创建数据库目录 {}: {e}", parent.display())
            })?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let _ = conn.execute_batch("PRAGMA journal_mode = WAL;");
        Self::init_conn(conn, true)
    }

    fn init_conn(conn: Connection, auto_seed: bool) -> Result<Self, String> {
        conn.execute_batch(SCHEMA_SQL).map_err(|e| e.to_string())?;
        let db = Self { conn };
        db.run_idempotent_migrations()?;
        db.ensure_defaults(auto_seed)?;
        Ok(db)
    }

    fn has_column(&self, table: &str, column: &str) -> Result<bool, String> {
        let sql = format!("PRAGMA table_info({table})");
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let cols = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(cols.iter().any(|c| c == column))
    }

    fn run_idempotent_migrations(&self) -> Result<(), String> {
        if !self.has_column("lab_biomarkers", "profile_id")? {
            self.conn
                .execute(
                    "ALTER TABLE lab_biomarkers ADD COLUMN profile_id TEXT NOT NULL DEFAULT 'self'",
                    [],
                )
                .map_err(|e| e.to_string())?;
        }
        if !self.has_column("lab_biomarkers", "reference_high")? {
            self.conn
                .execute(
                    "ALTER TABLE lab_biomarkers ADD COLUMN reference_high REAL",
                    [],
                )
                .map_err(|e| e.to_string())?;
        }
        if !self.has_column("experiments", "washout_days")? {
            self.conn
                .execute(
                    "ALTER TABLE experiments ADD COLUMN washout_days INTEGER NOT NULL DEFAULT 2",
                    [],
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn ensure_defaults(&self, auto_seed: bool) -> Result<(), String> {
        let settings_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM app_settings", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if settings_count == 0 {
            self.save_settings(&AppSettings::default())?;
        }
        self.conn
            .execute(
                "INSERT OR IGNORE INTO app_settings (key, value) VALUES ('locale', ?1)",
                params![DEFAULT_LOCALE],
            )
            .map_err(|e| e.to_string())?;
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
            ("active_persona", s.active_persona.clone()),
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
            ("entitlement_token", s.entitlement_token.clone()),
            ("max_seen_epoch", s.max_seen_epoch.to_string()),
            (
                "guardian_sponsor",
                if s.guardian_sponsor { "1" } else { "0" }.to_owned(),
            ),
            (
                "companion_loyalty_days",
                s.companion_loyalty_days.to_string(),
            ),
            ("sandbox_tier", s.sandbox_tier.clone()),
            ("sandbox_state", s.sandbox_state.clone()),
            ("weather_mood", s.weather_mood.clone()),
            ("widget_privacy_mode", s.widget_privacy_mode.clone()),
            (
                "equipped_keepsakes_json",
                s.equipped_keepsakes_json.clone(),
            ),
            ("pair_code", s.pair_code.clone()),
            ("partner_name", s.partner_name.clone()),
            ("partner_mood", s.partner_mood.to_string()),
            ("bonsai_dew", s.bonsai_dew.to_string()),
            ("active_seal_ink", s.active_seal_ink.clone()),
            ("locale", normalize_locale(&s.locale).to_owned()),
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
        if let Some(v) = map.get("active_persona") { s.active_persona = v.clone(); }
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
        if let Some(v) = map.get("entitlement_token") { s.entitlement_token = v.clone(); }
        if let Some(v) = map.get("max_seen_epoch") { s.max_seen_epoch = v.parse().unwrap_or(0); }
        if let Some(v) = map.get("guardian_sponsor") { s.guardian_sponsor = v == "1"; }
        if let Some(v) = map.get("companion_loyalty_days") { s.companion_loyalty_days = v.parse().unwrap_or(0); }
        if let Some(v) = map.get("sandbox_tier") { s.sandbox_tier = v.clone(); }
        if let Some(v) = map.get("sandbox_state") { s.sandbox_state = v.clone(); }
        if let Some(v) = map.get("weather_mood") { s.weather_mood = v.clone(); }
        if let Some(v) = map.get("widget_privacy_mode") { s.widget_privacy_mode = v.clone(); }
        if let Some(v) = map.get("equipped_keepsakes_json") { s.equipped_keepsakes_json = v.clone(); }
        if let Some(v) = map.get("pair_code") { s.pair_code = v.clone(); }
        if let Some(v) = map.get("partner_name") { s.partner_name = v.clone(); }
        if let Some(v) = map.get("partner_mood") { s.partner_mood = v.parse().unwrap_or(0); }
        if let Some(v) = map.get("bonsai_dew") { s.bonsai_dew = v.parse().unwrap_or(0); }
        if let Some(v) = map.get("active_seal_ink") { s.active_seal_ink = v.clone(); }
        if let Some(v) = map.get("locale") {
            s.locale = normalize_locale(v).to_owned();
        }
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

    pub fn insert_workout(
        &self,
        workout_id: &str,
        date: &str,
        activity_type: &str,
        duration_minutes: f64,
        calories_kcal: Option<f64>,
        source: &str,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO workouts (workout_id, date, activity_type, duration_minutes, calories_kcal, source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![workout_id, date, activity_type, duration_minutes, calories_kcal, source],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_experiment_checkin(&self, experiment_id: i64, date: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO experiment_checkins (experiment_id, date) VALUES (?1, ?2)",
                params![experiment_id, date],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 清空全部业务数据表，但保留 `app_settings`（云会话、12词恢复码与同步配置等）。
    pub fn clear_business_data(&self) -> Result<(), String> {
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
        Ok(())
    }

    pub fn seed_demo_35d(&self) -> Result<(), String> {
        crate::personas::seed_persona_data(self, "urban_overwork").map(|_| ())
    }

    pub fn create_experiment_if_missing(
        &self,
        title: &str,
        hypothesis: &str,
        target_days: u32,
        start_date: &str,
    ) -> Result<i64, String> {
        self.create_experiment_with_washout(title, hypothesis, target_days, start_date, 2)
    }

    pub fn create_experiment_with_washout(
        &self,
        title: &str,
        hypothesis: &str,
        target_days: u32,
        start_date: &str,
        washout_days: i64,
    ) -> Result<i64, String> {
        let washout = washout_days.clamp(0, 14);
        self.conn
            .execute(
                "INSERT OR IGNORE INTO experiments (title, hypothesis, target_days, start_date, status, washout_days)
                 VALUES (?1, ?2, ?3, ?4, 'active', ?5)",
                params![title.trim(), hypothesis.trim(), target_days, start_date, washout],
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
            .prepare("SELECT id, title, hypothesis, target_days, start_date, status, washout_days FROM experiments ORDER BY id DESC")
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
                    r.get::<_, i64>(6).unwrap_or(2),
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut out = Vec::new();
        for (id, title, hypothesis, target_days, start_date, status, washout_days) in rows {
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
                washout_days,
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
        self.save_parsed_lab_for_profile("self", test_date, d)
    }

    pub fn save_parsed_lab_for_profile(
        &self,
        profile_id: &str,
        test_date: &str,
        d: &ParsedLabMarkerDraft,
    ) -> Result<(), String> {
        let pid = if profile_id.trim().is_empty() {
            "self"
        } else {
            profile_id.trim()
        };
        let existing_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM lab_biomarkers WHERE profile_id = ?1 AND test_date = ?2 AND canonical_key = ?3",
                params![pid, test_date, d.canonical_key],
                |r| r.get(0),
            )
            .ok();
        if let Some(id) = existing_id {
            self.conn
                .execute(
                    "UPDATE lab_biomarkers
                     SET marker_name = ?1, value = ?2, unit = ?3, ref_low = ?4, ref_high = ?5,
                         reference_high = ?6, flag = ?7, plain_note = ?8
                     WHERE id = ?9",
                    params![
                        d.marker_name,
                        d.value,
                        d.unit,
                        d.ref_low,
                        d.ref_high,
                        d.ref_high,
                        d.flag,
                        d.plain_note,
                        id
                    ],
                )
                .map_err(|e| e.to_string())?;
        } else {
            self.conn
                .execute(
                    "INSERT INTO lab_biomarkers
                     (profile_id, test_date, marker_name, canonical_key, value, unit, ref_low, ref_high, reference_high, flag, plain_note)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        pid,
                        test_date,
                        d.marker_name,
                        d.canonical_key,
                        d.value,
                        d.unit,
                        d.ref_low,
                        d.ref_high,
                        d.ref_high,
                        d.flag,
                        d.plain_note
                    ],
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn insert_lab_marker_for_profile(
        &self,
        profile_id: &str,
        test_date: &str,
        marker_name: &str,
        canonical_key: &str,
        value: f64,
        unit: &str,
        ref_low: Option<f64>,
        ref_high: Option<f64>,
    ) -> Result<LabBiomarkerRow, String> {
        let flag = match (ref_low, ref_high) {
            (Some(lo), Some(hi)) => {
                if value > hi {
                    "high"
                } else if value < lo {
                    "low"
                } else {
                    "normal"
                }
            }
            (None, Some(hi)) if value > hi => "high",
            (Some(lo), None) if value < lo => "low",
            _ => "normal",
        };
        let draft = ParsedLabMarkerDraft {
            marker_name: marker_name.trim().to_owned(),
            canonical_key: canonical_key.trim().to_owned(),
            value,
            unit: unit.trim().to_owned(),
            ref_low,
            ref_high,
            flag: flag.to_owned(),
            plain_note: biomarker_plain_note(canonical_key.trim(), flag),
        };
        self.save_parsed_lab_for_profile(profile_id, test_date, &draft)?;
        let pid = if profile_id.trim().is_empty() {
            "self"
        } else {
            profile_id.trim()
        };
        let id: i64 = self
            .conn
            .query_row(
                "SELECT id FROM lab_biomarkers WHERE profile_id = ?1 AND test_date = ?2 AND canonical_key = ?3",
                params![pid, test_date, draft.canonical_key],
                |r| r.get(0),
            )
            .unwrap_or(0);
        Ok(LabBiomarkerRow {
            id,
            profile_id: pid.to_owned(),
            test_date: test_date.to_owned(),
            marker_name: draft.marker_name,
            canonical_key: draft.canonical_key,
            value: draft.value,
            unit: draft.unit,
            ref_low: draft.ref_low,
            ref_high: draft.ref_high,
            reference_high: draft.ref_high,
            flag: draft.flag,
            plain_note: draft.plain_note,
            prev_value: None,
            prev_date: None,
            delta: None,
            trend: None,
        })
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
            .prepare("SELECT id, profile_id, test_date, marker_name, canonical_key, value, unit, ref_low, ref_high, reference_high, flag, plain_note FROM lab_biomarkers ORDER BY test_date DESC, id DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let ref_high: Option<f64> = r.get(8)?;
                let reference_high: Option<f64> = r.get::<_, Option<f64>>(9)?.or(ref_high);
                Ok(LabBiomarkerRow {
                    id: r.get(0)?,
                    profile_id: r.get::<_, String>(1).unwrap_or_else(|_| "self".to_owned()),
                    test_date: r.get(2)?,
                    marker_name: r.get(3)?,
                    canonical_key: r.get(4)?,
                    value: r.get(5)?,
                    unit: r.get(6)?,
                    ref_low: r.get(7)?,
                    ref_high,
                    reference_high,
                    flag: r.get(10)?,
                    plain_note: r.get(11)?,
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
                    "profile_id": lab.profile_id,
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
                    "washout_days": exp.washout_days,
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
                        let pid = val["profile_id"].as_str().unwrap_or("self");
                        self.save_parsed_lab_for_profile(
                            pid,
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
                        let washout = val["washout_days"].as_i64().unwrap_or(2);
                        let exp_id =
                            self.create_experiment_with_washout(title, hyp, days, sd, washout)?;
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

    pub fn add_dew_entry(
        &self,
        delta: i64,
        source: &str,
        note: &str,
        reward_id: &str,
    ) -> Result<i64, String> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO dew_ledger (created_at, delta, source, note, reward_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![now, delta, source, note, reward_id],
            )
            .map_err(|e| e.to_string())?;
        self.get_dew_balance()
    }

    pub fn ensure_daily_recovery_dew(&self, date: &str) -> Result<bool, String> {
        let source_key = format!("recovery_priority:{date}");
        let exists: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM dew_ledger WHERE source = ?1",
                params![source_key],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if exists > 0 {
            return Ok(false);
        }
        self.add_dew_entry(
            crate::solar_atelier::RECOVERY_PRIORITY_DEW_BONUS,
            &source_key,
            "安心静养 · 睡饱双倍露 +3 滴晨露（永久免费）",
            "",
        )?;
        Ok(true)
    }

    pub fn get_dew_balance(&self) -> Result<i64, String> {
        self.conn
            .query_row("SELECT COALESCE(SUM(delta), 0) FROM dew_ledger", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }

    pub fn list_dew_ledger(&self, limit: usize) -> Result<Vec<DewLedgerEntry>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, created_at, delta, source, note, reward_id
                 FROM dew_ledger ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(DewLedgerEntry {
                    id: r.get(0)?,
                    created_at: r.get(1)?,
                    delta: r.get(2)?,
                    source: r.get(3)?,
                    note: r.get(4)?,
                    reward_id: r.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn list_claimed_pass_rewards(&self) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT reward_id FROM dew_ledger WHERE reward_id != '' ORDER BY id ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn insert_sticker(
        &self,
        date: &str,
        title: &str,
        filter: &str,
        note: &str,
        nutrition_hint: &str,
        image_ref: &str,
    ) -> Result<FoodSticker, String> {
        self.conn
            .execute(
                "INSERT INTO sticker_journal (date, title, filter, note, nutrition_hint, image_ref, is_favorite)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                params![date, title, filter, note, nutrition_hint, image_ref],
            )
            .map_err(|e| e.to_string())?;
        let id = self.conn.last_insert_rowid();
        Ok(FoodSticker {
            id,
            date: date.to_owned(),
            title: title.to_owned(),
            filter: filter.to_owned(),
            note: note.to_owned(),
            nutrition_hint: nutrition_hint.to_owned(),
            image_ref: image_ref.to_owned(),
            is_favorite: false,
        })
    }

    pub fn list_stickers(&self) -> Result<Vec<FoodSticker>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, date, title, filter, note, nutrition_hint, image_ref, is_favorite
                 FROM sticker_journal ORDER BY date DESC, id DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let fav: i64 = r.get(7)?;
                Ok(FoodSticker {
                    id: r.get(0)?,
                    date: r.get(1)?,
                    title: r.get(2)?,
                    filter: r.get(3)?,
                    note: r.get(4)?,
                    nutrition_hint: r.get(5)?,
                    image_ref: r.get(6)?,
                    is_favorite: fav != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn count_stickers_in_month(&self, year_month: &str) -> Result<usize, String> {
        let pattern = format!("{year_month}%");
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sticker_journal WHERE date LIKE ?1",
                params![pattern],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(count.max(0) as usize)
    }

    pub fn toggle_sticker_favorite(&self, id: i64) -> Result<bool, String> {
        let cur: i64 = self
            .conn
            .query_row(
                "SELECT is_favorite FROM sticker_journal WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .map_err(|_| format!("未找到贴纸 #{id}"))?;
        let next = if cur == 0 { 1 } else { 0 };
        self.conn
            .execute(
                "UPDATE sticker_journal SET is_favorite = ?1 WHERE id = ?2",
                params![next, id],
            )
            .map_err(|e| e.to_string())?;
        Ok(next != 0)
    }

    pub fn delete_sticker(&self, id: i64) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM sticker_journal WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save_preset(
        &self,
        name: &str,
        world_id: &str,
        weather_mood: &str,
        seal_ink: &str,
        keepsakes_json: &str,
    ) -> Result<SanctuaryPreset, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let trimmed = name.trim();
        self.conn
            .execute(
                "INSERT INTO sanctuary_presets (name, world_id, weather_mood, seal_ink, keepsakes_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(name) DO UPDATE SET
                    world_id = excluded.world_id,
                    weather_mood = excluded.weather_mood,
                    seal_ink = excluded.seal_ink,
                    keepsakes_json = excluded.keepsakes_json,
                    created_at = excluded.created_at",
                params![trimmed, world_id, weather_mood, seal_ink, keepsakes_json, now],
            )
            .map_err(|e| e.to_string())?;
        let id: i64 = self
            .conn
            .query_row(
                "SELECT id FROM sanctuary_presets WHERE name = ?1",
                params![trimmed],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(SanctuaryPreset {
            id,
            name: trimmed.to_owned(),
            world_id: world_id.to_owned(),
            weather_mood: weather_mood.to_owned(),
            seal_ink: seal_ink.to_owned(),
            keepsakes_json: keepsakes_json.to_owned(),
            created_at: now,
        })
    }

    pub fn list_presets(&self) -> Result<Vec<SanctuaryPreset>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, world_id, weather_mood, seal_ink, keepsakes_json, created_at
                 FROM sanctuary_presets ORDER BY id DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(SanctuaryPreset {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    world_id: r.get(2)?,
                    weather_mood: r.get(3)?,
                    seal_ink: r.get(4)?,
                    keepsakes_json: r.get(5)?,
                    created_at: r.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn count_presets(&self) -> Result<usize, String> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM sanctuary_presets", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(count.max(0) as usize)
    }

    pub fn get_preset(&self, id: i64) -> Result<Option<SanctuaryPreset>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, world_id, weather_mood, seal_ink, keepsakes_json, created_at
                 FROM sanctuary_presets WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![id], |r| {
                Ok(SanctuaryPreset {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    world_id: r.get(2)?,
                    weather_mood: r.get(3)?,
                    seal_ink: r.get(4)?,
                    keepsakes_json: r.get(5)?,
                    created_at: r.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        if let Some(res) = rows.next() {
            Ok(Some(res.map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    pub fn list_custom_pets(&self) -> Result<Vec<CustomPetCompanion>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, species, persona, photo_data_url, die_cut_frame,
                        starlight_memorial, paw_stamp_title, is_active, created_at, updated_at
                 FROM custom_pets
                 ORDER BY is_active DESC, created_at ASC, id ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(CustomPetCompanion {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    species: r.get(2)?,
                    persona: r.get(3)?,
                    photo_data_url: r.get(4)?,
                    die_cut_frame: r.get(5)?,
                    starlight_memorial: r.get::<_, i64>(6)? != 0,
                    paw_stamp_title: r.get(7)?,
                    is_active: r.get::<_, i64>(8)? != 0,
                    created_at: r.get(9)?,
                    updated_at: r.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn get_custom_pet(&self, pet_id: &str) -> Result<Option<CustomPetCompanion>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, species, persona, photo_data_url, die_cut_frame,
                        starlight_memorial, paw_stamp_title, is_active, created_at, updated_at
                 FROM custom_pets WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![pet_id], |r| {
                Ok(CustomPetCompanion {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    species: r.get(2)?,
                    persona: r.get(3)?,
                    photo_data_url: r.get(4)?,
                    die_cut_frame: r.get(5)?,
                    starlight_memorial: r.get::<_, i64>(6)? != 0,
                    paw_stamp_title: r.get(7)?,
                    is_active: r.get::<_, i64>(8)? != 0,
                    created_at: r.get(9)?,
                    updated_at: r.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;
        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    pub fn count_custom_pets(&self) -> Result<usize, String> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM custom_pets", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(count.max(0) as usize)
    }

    pub fn next_custom_pet_id(&self) -> String {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let existing = self.count_custom_pets().unwrap_or(0) + 1;
        format!("pet_{now_ms}_{existing}")
    }

    pub fn upsert_custom_pet(&self, pet: &CustomPetCompanion) -> Result<(), String> {
        if pet.is_active {
            self.conn
                .execute("UPDATE custom_pets SET is_active = 0", [])
                .map_err(|e| e.to_string())?;
        }
        self.conn
            .execute(
                "INSERT INTO custom_pets
                 (id, name, species, persona, photo_data_url, die_cut_frame,
                  starlight_memorial, paw_stamp_title, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                   name = excluded.name,
                   species = excluded.species,
                   persona = excluded.persona,
                   photo_data_url = excluded.photo_data_url,
                   die_cut_frame = excluded.die_cut_frame,
                   starlight_memorial = excluded.starlight_memorial,
                   paw_stamp_title = excluded.paw_stamp_title,
                   is_active = excluded.is_active,
                   updated_at = excluded.updated_at",
                params![
                    pet.id,
                    pet.name,
                    pet.species,
                    pet.persona,
                    pet.photo_data_url,
                    pet.die_cut_frame,
                    if pet.starlight_memorial { 1_i64 } else { 0_i64 },
                    pet.paw_stamp_title,
                    if pet.is_active { 1_i64 } else { 0_i64 },
                    pet.created_at,
                    pet.updated_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_active_custom_pet(&self, pet_id: &str) -> Result<Option<CustomPetCompanion>, String> {
        let trimmed = pet_id.trim();
        if trimmed.is_empty() {
            self.conn
                .execute("UPDATE custom_pets SET is_active = 0", [])
                .map_err(|e| e.to_string())?;
            return Ok(None);
        }
        let target = self.get_custom_pet(trimmed)?;
        if target.is_none() {
            return Err("custom_pet_not_found".to_owned());
        }
        self.conn
            .execute("UPDATE custom_pets SET is_active = 0", [])
            .map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "UPDATE custom_pets SET is_active = 1 WHERE id = ?1",
                params![trimmed],
            )
            .map_err(|e| e.to_string())?;
        self.get_custom_pet(trimmed)
    }

    pub fn delete_custom_pet(&self, pet_id: &str) -> Result<bool, String> {
        let trimmed = pet_id.trim();
        self.conn
            .execute(
                "DELETE FROM pet_weekly_letters WHERE pet_id = ?1",
                params![trimmed],
            )
            .map_err(|e| e.to_string())?;
        let deleted = self
            .conn
            .execute("DELETE FROM custom_pets WHERE id = ?1", params![trimmed])
            .map_err(|e| e.to_string())?;
        Ok(deleted > 0)
    }

    pub fn insert_pet_weekly_letter(&self, letter: &PetWeeklyLetter) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO pet_weekly_letters
                 (pet_id, pet_name, species, week_number, solar_term, title, body,
                  ex_libris_seal, starlight_memorial, locale, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    letter.pet_id,
                    letter.pet_name,
                    letter.species,
                    letter.week_number as i64,
                    letter.solar_term,
                    letter.title,
                    letter.body,
                    letter.ex_libris_seal,
                    if letter.starlight_memorial { 1_i64 } else { 0_i64 },
                    normalize_locale(&letter.locale),
                    letter.created_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_pet_weekly_letters(&self) -> Result<Vec<PetWeeklyLetter>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, pet_id, pet_name, species, week_number, solar_term, title, body,
                        ex_libris_seal, starlight_memorial, locale, created_at
                 FROM pet_weekly_letters
                 ORDER BY week_number DESC, id DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(PetWeeklyLetter {
                    id: r.get(0)?,
                    pet_id: r.get(1)?,
                    pet_name: r.get(2)?,
                    species: r.get(3)?,
                    week_number: r.get::<_, i64>(4)?.max(1) as u32,
                    solar_term: r.get(5)?,
                    title: r.get(6)?,
                    body: r.get(7)?,
                    ex_libris_seal: r.get(8)?,
                    starlight_memorial: r.get::<_, i64>(9)? != 0,
                    locale: r.get(10)?,
                    created_at: r.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    /// 合规物理销毁本机健康库（App Review 5.1.1(v)）
    pub fn wipe_all_data(&self) -> Result<(), String> {
        self.clear_business_data()?;
        self.conn
            .execute_batch(
                "DELETE FROM dew_ledger;
                 DELETE FROM sticker_journal;
                 DELETE FROM sanctuary_presets;
                 DELETE FROM custom_pets;
                 DELETE FROM pet_weekly_letters;",
            )
            .map_err(|e| e.to_string())?;
        let mut s = self.load_settings()?;
        s.onboarding_done = false;
        s.checked_action_today = false;
        s.cloud_token.clear();
        s.cloud_handle.clear();
        s.entitlement_token.clear();
        s.equipped_keepsakes_json = "[]".to_owned();
        s.bonsai_dew = 0;
        s.weather_mood = "sunny_crisp".to_owned();
        s.active_seal_ink = "cinnabar".to_owned();
        s.last_sync_summary = if is_en_locale(&s.locale) {
            "Local vault and session credentials cleared".to_owned()
        } else {
            "本机与会话凭证已物理清空".to_owned()
        };
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
        assert!(db.list_custom_pets().unwrap().is_empty());
        assert!(db.list_pet_weekly_letters().unwrap().is_empty());
    }

    #[test]
    fn legacy_settings_without_locale_defaults_to_zh_cn() {
        let legacy = json!({
            "active_persona": "urban_overwork",
            "onboarding_done": true,
            "primary_goal": "energy_sleep",
            "hardware_tier": "tier_a",
            "morning_feeling": 4,
            "sync_mode": "icloud_drive",
            "webdav_url": "https://dav.jianguoyun.com/dav/qi-vault/",
            "webdav_username": "",
            "webdav_password": "",
            "byo_server_url": "http://127.0.0.1:8788",
            "cloud_url": "http://127.0.0.1:8788",
            "cloud_handle": "",
            "cloud_token": "",
            "ai_mode": "official_zdr_cloud",
            "zdr_consented": false,
            "lan_ai_url": "http://127.0.0.1:11434/v1",
            "lan_ai_model": "qwen3:latest",
            "morning_push_enabled": true,
            "morning_push_time": "08:30",
            "thermal_throttled": false,
            "simulated_ram_gb": 8,
            "recovery_phrase": "test phrase",
            "device_id": "dev-legacy",
            "sync_seq": 0,
            "pull_cursor_id": 0,
            "last_sync_summary": "",
            "checked_action_today": false,
            "entitlement_token": "",
            "max_seen_epoch": 0,
            "guardian_sponsor": false,
            "companion_loyalty_days": 0
        });
        let parsed: AppSettings = serde_json::from_value(legacy).unwrap();
        assert_eq!(parsed.locale, "zh-CN");

        let db = VaultDatabase::open_empty_memory().unwrap();
        assert_eq!(db.load_settings().unwrap().locale, "zh-CN");
    }
}

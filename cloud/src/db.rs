//! 极简 Control Plane SQLite 存储层：
//! 严守「零健康明文库」原则 —— 仅保存账号令牌哈希、月度 ZDR 免费配额计数与 E2EE 密文信封。

use std::path::Path;
use std::sync::Mutex;

use chrono::{Datelike, Utc};
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
    created_at TEXT NOT NULL
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
"#;

pub const FREE_LIMIT_MEAL_SCANS: i64 = 3;
pub const FREE_LIMIT_LAB_PARSES: i64 = 1;
pub const FREE_LIMIT_BRIEFINGS: i64 = 30;

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
    pub year_month: String,
    pub meal_scans_used: i64,
    pub meal_scans_remaining: i64,
    pub lab_parses_used: i64,
    pub lab_parses_remaining: i64,
    pub briefings_used: i64,
    pub briefings_remaining: i64,
    pub icloud_sync_free: bool,
    pub cross_platform_webdav_unlocked: bool,
    pub local_ollama_unlocked: bool,
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
    pub wiped_at: String,
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

impl ControlPlaneDb {
    pub fn open_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        conn.execute_batch(CONTROL_PLANE_SCHEMA)
            .map_err(|e| e.to_string())?;
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

        let is_pro = id.plan_tier == "pro";
        Ok(EntitlementsSnapshot {
            account_id: id.account_id.clone(),
            handle: id.handle.clone(),
            plan_tier: id.plan_tier.clone(),
            year_month: ym,
            meal_scans_used: meal,
            meal_scans_remaining: if is_pro {
                -1
            } else {
                (FREE_LIMIT_MEAL_SCANS - meal).max(0)
            },
            lab_parses_used: lab,
            lab_parses_remaining: if is_pro {
                -1
            } else {
                (FREE_LIMIT_LAB_PARSES - lab).max(0)
            },
            briefings_used: brief,
            briefings_remaining: if is_pro {
                -1
            } else {
                (FREE_LIMIT_BRIEFINGS - brief).max(0)
            },
            icloud_sync_free: true,
            cross_platform_webdav_unlocked: is_pro,
            local_ollama_unlocked: is_pro,
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
        let tier = if new_tier == "pro" { "pro" } else { "free" };
        let guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        guard
            .execute(
                "UPDATE accounts SET plan_tier = ?1 WHERE account_id = ?2",
                params![tier, id.account_id],
            )
            .map_err(|e| e.to_string())?;
        let mut updated = id.clone();
        updated.plan_tier = tier.to_owned();
        Self::load_entitlements_locked(&guard, &updated)
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

        if id.plan_tier != "pro" {
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

    /// Apple 5.1.1(v) 账号与全量密文分片事务性物理销毁
    pub fn delete_account_physical(&self, account_id: &str) -> Result<DeletionReceipt, String> {
        let mut guard = self.conn.lock().map_err(|_| "DB 锁异常".to_owned())?;
        let tx = guard.transaction().map_err(|e| e.to_string())?;
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

//! `qi-cloud` 全流程端到端集成测试：
//! 验证零健康明文表、Free 配额 429 拦截、Pro 解锁、E2EE 密文推拉鉴权及 5.1.1(v) 物理销毁。

use std::collections::BTreeSet;
use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Verifier};
use qi_cloud::db::{
    ControlPlaneDb, FREE_LIMIT_LAB_PARSES, OFFLINE_GRACE_SECONDS, OsetClaims,
    default_verifying_key,
};
use qi_cloud::{AppState, build_router};
use serde_json::{Value, json};

async fn spawn_test_server() -> (String, Arc<ControlPlaneDb>) {
    let db = Arc::new(ControlPlaneDb::open_memory().unwrap());
    let app = build_router(AppState {
        db: db.clone(),
        zdr_upstream: None,
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{addr}"), db)
}

#[tokio::test]
async fn cloud_control_plane_quota_e2ee_and_physical_wipe() {
    let (base_url, db) = spawn_test_server().await;

    // 1. 审计服务端表清单：严禁出现任何健康明文表
    let tables = db.table_names().unwrap();
    assert_eq!(
        tables,
        vec!["accounts", "e2ee_sync_envelopes", "monthly_quotas"]
    );

    // 使用 blocking 线程跑 ureq 客户端模拟真实 HTTP 请求
    let base = base_url.clone();
    tokio::task::spawn_blocking(move || {
        let h_resp: Value = ureq::get(&format!("{base}/healthz"))
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(h_resp["health_plaintext_retained"], false);

        // 2. 注册并获取 Bearer Token + Free 权益
        let session: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "qi-tester@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let token = session["token"].as_str().unwrap().to_owned();
        let auth = format!("Bearer {token}");
        assert_eq!(session["plan_tier"], "free");
        assert_eq!(
            session["entitlements"]["lab_parses_remaining"],
            FREE_LIMIT_LAB_PARSES
        );

        // 3. Free 用户 ZDR 体检解读：第 1 次成功并带 no-store 响应头；第 2 次触发 429
        let ok_resp = ureq::post(&format!("{base}/v1/zdr/complete"))
            .set("Authorization", &auth)
            .send_json(json!({
                "task": "lab",
                "scrubbed_prompt": "姓名：陈默 身份证：31010419920518432X 尿酸 468 umol/L"
            }))
            .unwrap();
        assert_eq!(ok_resp.header("cache-control"), Some("no-store"));
        assert_eq!(
            ok_resp.header("x-qi-zdr"),
            Some("memory-only-zero-retention")
        );
        let ok_json: Value = ok_resp.into_json().unwrap();
        assert!(!ok_json["content"].as_str().unwrap().contains("陈默"));
        assert!(!ok_json["content"].as_str().unwrap().contains("31010419920518432X"));

        let err_429 = ureq::post(&format!("{base}/v1/zdr/complete"))
            .set("Authorization", &auth)
            .send_json(json!({ "task": "lab", "scrubbed_prompt": "second attempt" }))
            .unwrap_err();
        match err_429 {
            ureq::Error::Status(code, _) => assert_eq!(code, 429),
            other => panic!("expected 429 quota error, got {other}"),
        }

        // 4. 激活 Pro 后，ZDR 配额无限放行，且跨端 WebDAV / Ollama 权益解锁
        let pro_ent: Value = ureq::post(&format!("{base}/v1/billing/activate-pro"))
            .set("Authorization", &auth)
            .send_json(json!({ "tier": "pro" }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(pro_ent["plan_tier"], "pro");
        assert_eq!(pro_ent["lab_parses_remaining"], -1);
        assert_eq!(pro_ent["local_ollama_unlocked"], true);

        let pro_ok = ureq::post(&format!("{base}/v1/zdr/complete"))
            .set("Authorization", &auth)
            .send_json(json!({ "task": "lab", "scrubbed_prompt": "pro unlimited" }))
            .unwrap();
        assert_eq!(pro_ok.status(), 200);

        // 5. E2EE 推送严格拒绝非 XChaCha20-Poly1305 明文载荷
        let bad_push = ureq::post(&format!("{base}/v1/sync/ops/push"))
            .set("Authorization", &auth)
            .send_json(json!({
                "device_id": "dev-a",
                "batch_seq": 1,
                "path": "ops/dev-a/000001.enc",
                "protocol": "open-health-sync-v2",
                "cipher": "plaintext",
                "nonce_hex": "00",
                "ciphertext_hex": "not-encrypted",
                "payload_bytes": 13
            }))
            .unwrap_err();
        match bad_push {
            ureq::Error::Status(code, _) => assert_eq!(code, 400),
            other => panic!("expected 400 invalid_e2ee_envelope, got {other}"),
        }

        // 6. 合规 XChaCha20 密文推送与跨设备排除自身拉取
        let valid_nonce = "a".repeat(48);
        let push_ok: Value = ureq::post(&format!("{base}/v1/sync/ops/push"))
            .set("Authorization", &auth)
            .send_json(json!({
                "device_id": "dev-iphone",
                "batch_seq": 1,
                "path": "ops/dev-iphone/000001.enc",
                "protocol": "open-health-sync-v2",
                "cipher": "XChaCha20-Poly1305",
                "nonce_hex": valid_nonce,
                "ciphertext_hex": "deadbeefcafe01020304",
                "payload_bytes": 64
            }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(push_ok["status"], "stored_ciphertext_only");

        let pull_b: Value = ureq::get(&format!("{base}/v1/sync/ops/pull?after_id=0&exclude_device=dev-mac"))
            .set("Authorization", &auth)
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(pull_b["count"], 1);
        assert_eq!(pull_b["envelopes"][0]["device_id"], "dev-iphone");

        // 7. Apple 5.1.1(v) 一键物理销毁账号及全部密文信封
        let wipe: Value = ureq::delete(&format!("{base}/v1/account"))
            .set("Authorization", &auth)
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(wipe["deleted_accounts"], 1);
        assert_eq!(wipe["deleted_e2ee_envelopes"], 1);

        // 销毁后旧 Token 立即 401
        let after_del = ureq::get(&format!("{base}/v1/entitlements"))
            .set("Authorization", &auth)
            .call()
            .unwrap_err();
        match after_del {
            ureq::Error::Status(code, _) => assert_eq!(code, 401),
            other => panic!("expected 401 after wipe, got {other}"),
        }
    })
    .await
    .unwrap();
}

fn verify_and_decode_oset(token: &str) -> OsetClaims {
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "OSET token must have 3 dot-separated parts");
    assert_eq!(parts[0], "qient1");

    let vk = default_verifying_key();
    let sig_bytes = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
    let sig = Signature::from_slice(&sig_bytes).unwrap();
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    vk.verify(signing_input.as_bytes(), &sig)
        .expect("Ed25519 OSET signature must verify with default verifying key");

    let claims_bytes = URL_SAFE_NO_PAD.decode(parts[1]).unwrap();
    serde_json::from_slice(&claims_bytes).unwrap()
}

#[tokio::test]
async fn oset_ed25519_minting_schema_aliases_and_refresh() {
    let (base_url, db) = spawn_test_server().await;
    assert_eq!(
        db.table_names().unwrap(),
        vec!["accounts", "e2ee_sync_envelopes", "monthly_quotas"]
    );

    let base = base_url.clone();
    tokio::task::spawn_blocking(move || {
        let session: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "atelier-oset@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let account_id = session["account_id"].as_str().unwrap().to_owned();
        let auth = format!("Bearer {}", session["token"].as_str().unwrap());

        // Free tier aliases & empty OSET token
        let free_ent = &session["entitlements"];
        assert_eq!(free_ent["year_month"], free_ent["month"]);
        assert_eq!(
            free_ent["cross_platform_webdav_unlocked"],
            free_ent["webdav_unlocked"]
        );
        assert_eq!(free_ent["webdav_unlocked"], false);
        assert_eq!(free_ent["entitlement_token"], "");
        assert_eq!(free_ent["entitlement_expires_epoch"], 0);
        assert_eq!(free_ent["seats"], 1);
        assert_eq!(free_ent["guardian_sponsor"], false);
        assert_eq!(free_ent["companion_loyalty_days"], 0);

        // Reject invalid tier on mock-upgrade
        let bad_tier = ureq::post(&format!("{base}/v1/billing/mock-upgrade"))
            .set("Authorization", &auth)
            .send_json(json!({ "tier": "nonexistent_tier" }))
            .unwrap_err();
        match bad_tier {
            ureq::Error::Status(code, _) => assert_eq!(code, 400),
            other => panic!("expected 400 invalid_tier, got {other}"),
        }

        // Verify all paid tiers mint valid Ed25519 OSET tokens
        let cases = [
            ("seasonal", 1u8, true, 35i64, true, true),
            ("pro_annual", 1u8, false, 12i64, false, true),
            ("duo", 2u8, true, 30i64, true, true),
            ("family", 4u8, false, 45i64, true, true),
            ("lifetime_local", 1u8, true, 60i64, true, false),
        ];

        for (tier, expected_seats, sponsor, loyalty_days, expect_loyalty, expect_cloud_sync) in
            cases
        {
            let ent: Value = ureq::post(&format!("{base}/v1/billing/mock-upgrade"))
                .set("Authorization", &auth)
                .send_json(json!({
                    "tier": tier,
                    "guardian_sponsor": sponsor,
                    "companion_loyalty_days": loyalty_days
                }))
                .unwrap()
                .into_json()
                .unwrap();

            assert_eq!(ent["plan_tier"], tier);
            assert_eq!(ent["year_month"], ent["month"]);
            assert_eq!(ent["cross_platform_webdav_unlocked"], true);
            assert_eq!(ent["webdav_unlocked"], true);
            assert_eq!(ent["seats"], expected_seats);
            assert_eq!(ent["guardian_sponsor"], sponsor);
            assert_eq!(ent["companion_loyalty_days"], loyalty_days);
            assert!(!ent["tier_display"].as_str().unwrap().is_empty());
            assert!(ent["entitlement_expires_epoch"].as_i64().unwrap() > 0);

            let oset_str = ent["entitlement_token"].as_str().unwrap();
            assert!(oset_str.starts_with("qient1."));
            let claims = verify_and_decode_oset(oset_str);
            assert_eq!(claims.ver, 1);
            assert_eq!(claims.iss, "qi-atelier-auth");
            assert_eq!(claims.sub, account_id);
            assert_eq!(claims.tier, tier);
            assert_eq!(claims.seats, expected_seats);
            assert_eq!(claims.guardian_sponsor, sponsor);
            assert_eq!(claims.companion_loyalty_eligible, expect_loyalty);
            assert_eq!(
                claims.grace_until,
                claims.exp_epoch + OFFLINE_GRACE_SECONDS
            );
            assert_eq!(
                claims.features.iter().any(|f| f == "cloud_e2ee_sync"),
                expect_cloud_sync
            );
            assert!(claims.features.iter().any(|f| f == "raw_biometrics"));
        }

        // Refresh endpoint issues a fresh valid OSET token with distinct nonce
        let prev_ent: Value = ureq::get(&format!("{base}/v1/entitlements"))
            .set("Authorization", &auth)
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        let prev_claims = verify_and_decode_oset(prev_ent["entitlement_token"].as_str().unwrap());

        let refreshed: Value = ureq::post(&format!("{base}/v1/entitlements/refresh"))
            .set("Authorization", &auth)
            .send_json(json!({}))
            .unwrap()
            .into_json()
            .unwrap();
        let refreshed_claims =
            verify_and_decode_oset(refreshed["entitlement_token"].as_str().unwrap());
        assert_eq!(refreshed_claims.tier, "lifetime_local");
        assert_ne!(refreshed_claims.nonce, prev_claims.nonce);

        // Paid seasonal/family/etc. tiers unlock unlimited ZDR quota
        let zdr_ok = ureq::post(&format!("{base}/v1/zdr/complete"))
            .set("Authorization", &auth)
            .send_json(json!({ "task": "lab", "scrubbed_prompt": "atelier lab check" }))
            .unwrap();
        assert_eq!(zdr_ok.status(), 200);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cosanctuary_privacy_first_pair_mood_care_and_dew() {
    let (base_url, db) = spawn_test_server().await;
    assert_eq!(
        db.table_names().unwrap(),
        vec!["accounts", "e2ee_sync_envelopes", "monthly_quotas"]
    );

    let base = base_url.clone();
    tokio::task::spawn_blocking(move || {
        let sess_a: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "partner-a@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let auth_a = format!("Bearer {}", sess_a["token"].as_str().unwrap());

        let sess_b: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "partner-b@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let auth_b = format!("Bearer {}", sess_b["token"].as_str().unwrap());

        // 1. Account A generates 6-char uppercase pair code
        let pair_a: Value = ureq::post(&format!("{base}/v1/cosanctuary/pair"))
            .set("Authorization", &auth_a)
            .send_json(json!({}))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(pair_a["paired"], false);
        let code = pair_a["pair_code"].as_str().unwrap().to_owned();
        assert_eq!(code.len(), 6);
        assert!(
            code.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        );

        // Self-pairing is rejected
        let self_pair = ureq::post(&format!("{base}/v1/cosanctuary/pair"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "pair_code": code }))
            .unwrap_err();
        match self_pair {
            ureq::Error::Status(status, _) => assert_eq!(status, 400),
            other => panic!("expected 400 self-pair error, got {other}"),
        }

        // 2. Account B pairs symmetrically using A's code
        let pair_b: Value = ureq::post(&format!("{base}/v1/cosanctuary/pair"))
            .set("Authorization", &auth_b)
            .send_json(json!({ "pair_code": code }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(pair_b["paired"], true);
        assert_eq!(pair_b["pair_code"], code);
        assert_eq!(pair_b["bonsai_dew"], 0);

        // 3. Account A sets 1-byte mood enum + outfit; invalid >2 rejected
        let bad_mood = ureq::post(&format!("{base}/v1/cosanctuary/mood"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "mood": 3 }))
            .unwrap_err();
        match bad_mood {
            ureq::Error::Status(status, _) => assert_eq!(status, 400),
            other => panic!("expected 400 invalid mood, got {other}"),
        }

        let mood_a: Value = ureq::post(&format!("{base}/v1/cosanctuary/mood"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "mood": 2, "outfit_id": "linen_haori" }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(mood_a["self"]["mood"], 2);
        assert_eq!(mood_a["self"]["mood_label"], "COZY_RECOVER");
        assert_eq!(mood_a["self"]["outfit_id"], "linen_haori");

        // 4. Mutual care delivery & bonsai_dew increments on both sides
        let bad_care = ureq::post(&format!("{base}/v1/cosanctuary/care"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "care_action": "send_biometrics" }))
            .unwrap_err();
        match bad_care {
            ureq::Error::Status(status, _) => assert_eq!(status, 400),
            other => panic!("expected 400 invalid care_action, got {other}"),
        }

        let care_a: Value = ureq::post(&format!("{base}/v1/cosanctuary/care"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "care_action": "warm_tea" }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(care_a["bonsai_dew"], 1);
        assert_eq!(care_a["partner"]["care_action"], "warm_tea");

        let care_b: Value = ureq::post(&format!("{base}/v1/cosanctuary/care"))
            .set("Authorization", &auth_b)
            .send_json(json!({ "care_action": "paw_stamp" }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(care_b["bonsai_dew"], 2);
        assert_eq!(care_b["self"]["care_action"], "warm_tea");
        assert_eq!(care_b["partner"]["care_action"], "paw_stamp");

        // 5. Status payload strictly contains zero health/biometric/lab/diet fields
        let status_a: Value = ureq::get(&format!("{base}/v1/cosanctuary/status"))
            .set("Authorization", &auth_a)
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(status_a["paired"], true);
        assert_eq!(status_a["bonsai_dew"], 2);
        assert_eq!(status_a["self"]["care_action"], "paw_stamp");
        assert_eq!(status_a["partner"]["care_action"], "warm_tea");

        let top_keys: BTreeSet<&str> = status_a
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            top_keys,
            BTreeSet::from(["bonsai_dew", "pair_code", "paired", "partner", "self"])
        );
        let party_keys: BTreeSet<&str> = status_a["partner"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            party_keys,
            BTreeSet::from(["care_action", "mood", "mood_label", "outfit_id", "updated_at"])
        );
    })
    .await
    .unwrap();
}

#[test]
fn idempotent_alter_table_migration_from_legacy_accounts_schema() {
    let mut path = std::env::temp_dir();
    path.push(format!("qi_cloud_legacy_mig_{}.sqlite3", std::process::id()));
    let _ = std::fs::remove_file(&path);

    // Create a legacy 3-table schema without the new v4.0 columns
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE accounts (
                account_id TEXT PRIMARY KEY,
                handle TEXT UNIQUE NOT NULL,
                token_hash TEXT NOT NULL,
                plan_tier TEXT NOT NULL DEFAULT 'free',
                created_at TEXT NOT NULL
            );
            CREATE TABLE monthly_quotas (
                account_id TEXT NOT NULL,
                year_month TEXT NOT NULL,
                meal_scans INTEGER NOT NULL DEFAULT 0,
                lab_parses INTEGER NOT NULL DEFAULT 0,
                briefings INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (account_id, year_month)
            );
            CREATE TABLE e2ee_sync_envelopes (
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
            "#,
        )
        .unwrap();
    }

    // Opening via ControlPlaneDb::open_file runs idempotent migrations & keeps 3-table invariant
    let db = ControlPlaneDb::open_file(&path).unwrap();
    assert_eq!(
        db.table_names().unwrap(),
        vec!["accounts", "e2ee_sync_envelopes", "monthly_quotas"]
    );
    let (id, _, ent) = db.create_or_rotate_session("legacy-user@example.com").unwrap();
    assert_eq!(ent.plan_tier, "free");
    let upgraded = db
        .upgrade_plan(&id, "duo", Some(true), Some(31))
        .unwrap();
    assert_eq!(upgraded.seats, 2);
    assert!(upgraded.entitlement_token.starts_with("qient1."));

    // Re-opening is completely idempotent
    drop(db);
    let db2 = ControlPlaneDb::open_file(&path).unwrap();
    assert_eq!(
        db2.table_names().unwrap(),
        vec!["accounts", "e2ee_sync_envelopes", "monthly_quotas"]
    );
    let _ = std::fs::remove_file(&path);
}

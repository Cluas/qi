//! `qi-cloud` 全流程端到端集成测试：
//! 验证零健康明文表、Free 配额 429 拦截、Pro 解锁、E2EE 密文推拉鉴权及 5.1.1(v) 物理销毁。

use std::collections::BTreeSet;
use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use ed25519_dalek::{Signature, Signer, Verifier};
use qi_cloud::db::{
    ControlPlaneDb, FREE_LIMIT_LAB_PARSES, HEARTH_GIFT_PASS_PRODUCT_ID,
    HEARTH_GIFT_PASS_TITLE_EN, HEARTH_GIFT_PASS_TITLE_ZH, OFFLINE_GRACE_SECONDS, OsetClaims,
    default_signing_key, default_verifying_key,
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
        vec![
            "accounts",
            "apple_iap_transactions",
            "e2ee_sync_envelopes",
            "monthly_quotas"
        ]
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
        vec![
            "accounts",
            "apple_iap_transactions",
            "e2ee_sync_envelopes",
            "monthly_quotas"
        ]
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
        vec![
            "accounts",
            "apple_iap_transactions",
            "e2ee_sync_envelopes",
            "monthly_quotas"
        ]
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

    // Opening via ControlPlaneDb::open_file runs idempotent migrations & keeps zero-health-plaintext invariant
    let db = ControlPlaneDb::open_file(&path).unwrap();
    assert_eq!(
        db.table_names().unwrap(),
        vec![
            "accounts",
            "apple_iap_transactions",
            "e2ee_sync_envelopes",
            "monthly_quotas"
        ]
    );
    let (id, _, ent) = db.create_or_rotate_session("legacy-user@example.com").unwrap();
    assert_eq!(ent.plan_tier, "free");
    assert_eq!(ent.hearth_gift_credits, 0);
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
        vec![
            "accounts",
            "apple_iap_transactions",
            "e2ee_sync_envelopes",
            "monthly_quotas"
        ]
    );
    let _ = std::fs::remove_file(&path);
}

fn make_jws_es256_mock(claims: &Value) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"ES256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap());
    let sig = URL_SAFE_NO_PAD.encode(&[0x42u8; 64]);
    format!("{header}.{payload}.{sig}")
}

fn make_jws_eddsa_signed(claims: &Value) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"EdDSA","typ":"JWT","kid":"qi-sandbox-ed25519"}"#);
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap());
    let signing_input = format!("{header}.{payload}");
    let sig = default_signing_key().sign(signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
    format!("{signing_input}.{sig_b64}")
}

#[tokio::test]
async fn apple_iap_verify_subscription_idempotency_hearth_gift_pass_and_rejections() {
    let (base_url, db) = spawn_test_server().await;
    let base = base_url.clone();

    tokio::task::spawn_blocking(move || {
        // 0. Unauthenticated request is rejected with 401
        let unauth_err = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .send_json(json!({ "product_id": "com.qi.atelier.pro_annual" }))
            .unwrap_err();
        match unauth_err {
            ureq::Error::Status(code, _) => assert_eq!(code, 401),
            other => panic!("expected 401 unauthenticated, got {other}"),
        }

        // Register accounts A, B, C
        let sess_a: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "apple-iap-a@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let account_a = sess_a["account_id"].as_str().unwrap().to_owned();
        let auth_a = format!("Bearer {}", sess_a["token"].as_str().unwrap());

        let sess_b: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "apple-iap-b@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let auth_b = format!("Bearer {}", sess_b["token"].as_str().unwrap());

        let sess_c: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "apple-iap-c@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let auth_c = format!("Bearer {}", sess_c["token"].as_str().unwrap());

        let future_ms = (Utc::now().timestamp() + 365 * 86_400) * 1000;
        let sub_jws = make_jws_es256_mock(&json!({
            "transactionId": "2000000100001",
            "originalTransactionId": "2000000100001",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.atelier.pro_annual",
            "type": "Auto-Renewable Subscription",
            "environment": "Sandbox",
            "expiresDate": future_ms,
            "quantity": 1
        }));

        // 1. Subscription JWS verification mints Ed25519 OSET (qient1.*)
        let sub_resp: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({
                "jws_representation": sub_jws,
                "companion_loyalty_days": 35
            }))
            .unwrap()
            .into_json()
            .unwrap();

        assert_eq!(sub_resp["verified"], true);
        assert_eq!(sub_resp["verification_mode"], "storekit2_jws_es256");
        assert_eq!(sub_resp["product_id"], "com.qi.atelier.pro_annual");
        assert_eq!(sub_resp["plan_tier"], "pro_annual");
        assert_eq!(sub_resp["idempotent_replay"], false);
        assert_eq!(sub_resp["seats_credited"], 0);
        let oset = sub_resp["entitlement_token"].as_str().unwrap();
        assert!(oset.starts_with("qient1."));
        let claims = verify_and_decode_oset(oset);
        assert_eq!(claims.sub, account_a);
        assert_eq!(claims.tier, "pro_annual");
        assert_eq!(claims.companion_loyalty_eligible, true);
        assert_eq!(db.count_apple_transactions_for_account(&account_a).unwrap(), 1);

        // Pro entitlement unlocks ZDR unlimited calls
        let zdr_ok = ureq::post(&format!("{base}/v1/zdr/complete"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "task": "lab", "scrubbed_prompt": "apple iap pro check" }))
            .unwrap();
        assert_eq!(zdr_ok.status(), 200);

        // 2. Restore / replay idempotency with same transaction_id
        let replay_resp: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({
                "jws_representation": sub_jws,
                "product_id": "com.qi.atelier.pro_annual",
                "transaction_id": "2000000100001",
                "original_transaction_id": "2000000100001"
            }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(replay_resp["idempotent_replay"], true);
        assert_eq!(replay_resp["plan_tier"], "pro_annual");
        assert!(replay_resp["entitlement_token"].as_str().unwrap().starts_with("qient1."));
        assert_eq!(db.count_apple_transactions_for_account(&account_a).unwrap(), 1);

        // 3. Renewal on same original_transaction_id with new transaction_id & extended expiry
        let extended_sec = Utc::now().timestamp() + 400 * 86_400;
        let renew_jws = make_jws_eddsa_signed(&json!({
            "transactionId": "2000000100002",
            "originalTransactionId": "2000000100001",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.atelier.pro_annual",
            "environment": "Sandbox",
            "expiresDate": extended_sec * 1000
        }));
        let renew_resp: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "jws_representation": renew_jws }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(renew_resp["idempotent_replay"], false);
        assert_eq!(renew_resp["verification_mode"], "storekit2_jws_eddsa");
        assert_eq!(
            renew_resp["entitlements"]["entitlement_expires_epoch"],
            extended_sec
        );
        assert_eq!(db.count_apple_transactions_for_account(&account_a).unwrap(), 2);

        // 4. Commercial Consumable Hearth Gift Pass credit + idempotency + guideline 3.2.1(vi) compliance
        let gift_jws_1 = make_jws_es256_mock(&json!({
            "transactionId": "3000000100001",
            "originalTransactionId": "3000000100001",
            "bundleId": "eu.org.cluas.qi",
            "productId": HEARTH_GIFT_PASS_PRODUCT_ID,
            "type": "Consumable",
            "environment": "Xcode",
            "quantity": 1
        }));
        let gift_resp_1: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "jws_representation": gift_jws_1 }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(gift_resp_1["product_id"], HEARTH_GIFT_PASS_PRODUCT_ID);
        assert_eq!(gift_resp_1["purchase_type"], "consumable");
        assert_eq!(gift_resp_1["idempotent_replay"], false);
        assert_eq!(gift_resp_1["seats_credited"], 1);
        assert_eq!(gift_resp_1["hearth_gift_credits_total"], 1);
        assert_eq!(gift_resp_1["plan_tier"], "pro_annual");
        assert_eq!(gift_resp_1["hearth_gift_pass_title_zh"], HEARTH_GIFT_PASS_TITLE_ZH);
        assert_eq!(gift_resp_1["hearth_gift_pass_title_en"], HEARTH_GIFT_PASS_TITLE_EN);
        let gift_code_1 = gift_resp_1["community_gift_code"].as_str().unwrap().to_owned();
        assert!(gift_code_1.starts_with("QI-SEAT-"));

        let resp_str = serde_json::to_string(&gift_resp_1).unwrap().to_lowercase();
        for forbidden in [
            "\u{516c}\u{76ca}",
            "\u{6148}\u{5584}",
            "\u{6350}",
            concat!("char", "ity"),
            concat!("don", "at"),
        ] {
            assert!(!resp_str.contains(forbidden), "forbidden wording found: {forbidden}");
        }

        // Replay same consumable transaction -> idempotent, must NOT double-credit
        let gift_replay: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "jws_representation": gift_jws_1 }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(gift_replay["idempotent_replay"], true);
        assert_eq!(gift_replay["seats_credited"], 0);
        assert_eq!(gift_replay["hearth_gift_credits_total"], 1);
        assert_eq!(gift_replay["community_gift_code"], gift_code_1);

        // Second distinct consumable transaction increments credits to 2
        let gift_jws_2 = make_jws_es256_mock(&json!({
            "transactionId": "3000000100002",
            "originalTransactionId": "3000000100002",
            "bundleId": "eu.org.cluas.qi",
            "productId": HEARTH_GIFT_PASS_PRODUCT_ID,
            "type": "Consumable",
            "environment": "Sandbox",
            "quantity": 1
        }));
        let gift_resp_2: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "jws_representation": gift_jws_2 }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(gift_resp_2["idempotent_replay"], false);
        assert_eq!(gift_resp_2["seats_credited"], 1);
        assert_eq!(gift_resp_2["hearth_gift_credits_total"], 2);

        // Free user C buying Hearth Gift Pass in Sandbox keeps plan_tier == "free"
        let gift_free: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_c)
            .send_json(json!({
                "environment": "Sandbox",
                "product_id": HEARTH_GIFT_PASS_PRODUCT_ID,
                "transaction_id": "3000000900001",
                "original_transaction_id": "3000000900001",
                "bundle_id": "eu.org.cluas.qi"
            }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(gift_free["plan_tier"], "free");
        assert_eq!(gift_free["entitlement_token"], "");
        assert_eq!(gift_free["hearth_gift_credits_total"], 1);

        // 5. Cross-account ownership conflict rejected with 409
        let hijack_err = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_b)
            .send_json(json!({ "jws_representation": sub_jws }))
            .unwrap_err();
        match hijack_err {
            ureq::Error::Status(code, _) => assert_eq!(code, 409),
            other => panic!("expected 409 conflict on cross-account replay, got {other}"),
        }

        // 6. Invalid payloads rejected with 400
        let tampered_eddsa = {
            let mut parts: Vec<String> = renew_jws.split('.').map(ToOwned::to_owned).collect();
            let tampered_claims = URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&json!({
                    "transactionId": "2000000100099",
                    "originalTransactionId": "2000000100099",
                    "bundleId": "eu.org.cluas.qi",
                    "productId": "com.qi.atelier.family",
                    "expiresDate": extended_sec * 1000
                }))
                .unwrap(),
            );
            parts[1] = tampered_claims;
            parts.join(".")
        };
        let alg_none_jws = format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#),
            URL_SAFE_NO_PAD.encode(br#"{"transactionId":"x1","productId":"com.qi.atelier.pro_annual"}"#),
            URL_SAFE_NO_PAD.encode(b"sig")
        );
        let short_sig_jws = format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(br#"{"alg":"ES256"}"#),
            URL_SAFE_NO_PAD.encode(br#"{"transactionId":"x2","productId":"com.qi.atelier.pro_annual"}"#),
            URL_SAFE_NO_PAD.encode(b"too_short")
        );
        let wrong_bundle_jws = make_jws_es256_mock(&json!({
            "transactionId": "2000000888001",
            "originalTransactionId": "2000000888001",
            "bundleId": "com.evil.clone",
            "productId": "com.qi.atelier.pro_annual",
            "expiresDate": future_ms
        }));
        let unknown_product_jws = make_jws_es256_mock(&json!({
            "transactionId": "2000000888002",
            "originalTransactionId": "2000000888002",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.unknown_sku",
            "expiresDate": future_ms
        }));
        let revoked_jws = make_jws_es256_mock(&json!({
            "transactionId": "2000000888003",
            "originalTransactionId": "2000000888003",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.atelier.pro_annual",
            "expiresDate": future_ms,
            "revocationDate": Utc::now().timestamp() * 1000
        }));
        let expired_jws = make_jws_es256_mock(&json!({
            "transactionId": "2000000888004",
            "originalTransactionId": "2000000888004",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.atelier.pro_annual",
            "expiresDate": (Utc::now().timestamp() - 3600) * 1000
        }));

        let bad_payloads = vec![
            json!({ "jws_representation": "not-three-parts" }),
            json!({ "jws_representation": alg_none_jws }),
            json!({ "jws_representation": short_sig_jws }),
            json!({ "jws_representation": tampered_eddsa }),
            json!({ "jws_representation": wrong_bundle_jws }),
            json!({ "jws_representation": unknown_product_jws }),
            json!({ "jws_representation": revoked_jws }),
            json!({ "jws_representation": expired_jws }),
            json!({ "jws_representation": sub_jws, "product_id": "com.qi.atelier.pro_monthly" }),
            json!({
                "environment": "Production",
                "product_id": "com.qi.atelier.pro_annual",
                "transaction_id": "prod_unsigned_1",
                "original_transaction_id": "prod_unsigned_1"
            }),
            json!({
                "environment": "InvalidEnv",
                "product_id": "com.qi.atelier.pro_annual",
                "transaction_id": "bad_env_1",
                "original_transaction_id": "bad_env_1"
            }),
        ];

        for bad in bad_payloads {
            let err = ureq::post(&format!("{base}/v1/billing/apple/verify"))
                .set("Authorization", &auth_b)
                .send_json(bad.clone())
                .unwrap_err();
            match err {
                ureq::Error::Status(code, _) => assert_eq!(code, 400, "expected 400 for {bad}"),
                other => panic!("expected 400 error for {bad}, got {other}"),
            }
        }

        // 7. Physical account wipe also deletes apple_iap_transactions for account A
        let wipe_a: Value = ureq::delete(&format!("{base}/v1/account"))
            .set("Authorization", &auth_a)
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(wipe_a["deleted_accounts"], 1);
        assert_eq!(wipe_a["deleted_apple_transactions"], 4);
        assert_eq!(db.count_apple_transactions_for_account(&account_a).unwrap(), 0);
    })
    .await
    .unwrap();
}

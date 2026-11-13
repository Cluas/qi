//! `qi-cloud` 全流程端到端集成测试：
//! 验证零健康明文表、Free 配额 429 拦截、Pro 解锁、E2EE 密文推拉鉴权及 5.1.1(v) 物理销毁。

use std::sync::Arc;

use qi_cloud::db::{ControlPlaneDb, FREE_LIMIT_LAB_PARSES};
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

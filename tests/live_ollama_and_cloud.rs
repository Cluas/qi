//! 真实环境联调测试（默认 #[ignore]，验证本机 Vision/Ollama 探针、真实 qi-cloud 子进程与可选上游 ZDR 网关）：
//! 运行：`cargo test --test live_ollama_and_cloud -- --ignored --nocapture`

use nsh_app_qi::QiEngine;
use serde_json::{Value, json};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

fn pick_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "requires local runtime environment (Apple Vision / Ollama / qi-cloud build)"]
fn live_ollama_qwen3_and_qi_cloud_e2ee_roundtrip() {
    // 1. 探测本机 Apple Vision OCR 与局域网 Ollama 连通性
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    engine
        .complete_onboarding("energy_sleep".into(), "tier_a".into())
        .unwrap();
    engine.probe_capabilities().unwrap();
    let st: Value = serde_json::from_str(&engine.state_json().unwrap()).unwrap();
    let probe = &st["capability_probe"];
    println!("[PROBE] {}", serde_json::to_string_pretty(probe).unwrap());
    assert_eq!(probe["vision_ocr_available"], true);

    // 2. 测试局域网 Ollama 调用路径（若本机已加载有效 GGUF 则断言完整推理，若仅残留 manifest 则断言友好降级错误）
    engine
        .update_ai_config(
            "lan_ollama".into(),
            false,
            "http://127.0.0.1:11434/v1".into(),
            "qwen3:latest".into(),
            false,
            16,
        )
        .unwrap();
    match engine.refresh_live_briefing() {
        Ok(raw) => {
            let st_ollama: Value = serde_json::from_str(&raw).unwrap();
            let ollama_text = st_ollama["briefing"]["text"].as_str().unwrap();
            println!(
                "[OLLAMA BRIEFING] ({}) => {}",
                st_ollama["briefing"]["provider_label"], ollama_text
            );
            assert_eq!(st_ollama["briefing"]["degraded"], false);
            assert!(!ollama_text.contains("<think>"));
            assert!(!ollama_text.is_empty());
        }
        Err(err) => {
            println!("[OLLAMA GRACEFUL ERROR HANDLED] {err}");
            assert!(err.contains("局域网模型连接失败"));
        }
    }

    // 3. 编译并拉起真实 qi-cloud 控制面子进程（支持外部环境变量注入 ZDR 上游）
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cloud_dir = manifest_dir.join("cloud");
    let build_status = Command::new("cargo")
        .arg("build")
        .current_dir(&cloud_dir)
        .status()
        .expect("failed to build qi-cloud");
    assert!(build_status.success());

    let port = pick_free_port();
    let bind_addr = format!("127.0.0.1:{port}");
    let base_url = format!("http://{bind_addr}");
    let cloud_db =
        std::env::temp_dir().join(format!("qi_live_cloud_{}.sqlite3", std::process::id()));
    let _ = std::fs::remove_file(&cloud_db);

    let cloud_bin = cloud_dir.join("target").join("debug").join("qi-cloud");
    let mut cmd = Command::new(&cloud_bin);
    cmd.env("QI_CLOUD_BIND", &bind_addr)
        .env("QI_CLOUD_DB", &cloud_db)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Ok(up) = std::env::var("QI_ZDR_UPSTREAM_URL") {
        cmd.env("QI_ZDR_UPSTREAM_URL", up);
    }
    if let Ok(key) = std::env::var("QI_ZDR_UPSTREAM_KEY") {
        cmd.env("QI_ZDR_UPSTREAM_KEY", key);
    }
    if let Ok(model) = std::env::var("QI_ZDR_UPSTREAM_MODEL") {
        cmd.env("QI_ZDR_UPSTREAM_MODEL", model);
    }

    let child = cmd.spawn().expect("failed to start qi-cloud");
    let _guard = ChildGuard(child);

    let mut ready = false;
    for _ in 0..30 {
        if ureq::get(&format!("{base_url}/healthz")).call().is_ok() {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(ready, "qi-cloud did not become ready on {base_url}");

    // 4. 注册会话 + Pro 激活 + ZDR 代理早报 + BYO-Server E2EE 跨设备还原
    let sess: Value = ureq::post(&format!("{base_url}/v1/auth/session"))
        .send_json(json!({ "handle": "live-tester@qi.cn" }))
        .unwrap()
        .into_json()
        .unwrap();
    let token = sess["token"].as_str().unwrap().to_owned();
    let _pro: Value = ureq::post(&format!("{base_url}/v1/billing/activate-pro"))
        .set("Authorization", &format!("Bearer {token}"))
        .send_json(json!({ "tier": "pro" }))
        .unwrap()
        .into_json()
        .unwrap();

    engine
        .update_cloud_session(base_url.clone(), "live-tester@qi.cn".into(), token.clone())
        .unwrap();
    engine
        .update_ai_config(
            "official_zdr_cloud".into(),
            true,
            "".into(),
            "".into(),
            false,
            16,
        )
        .unwrap();
    engine.refresh_live_briefing().unwrap();
    let st_zdr: Value = serde_json::from_str(&engine.state_json().unwrap()).unwrap();
    let zdr_text = st_zdr["briefing"]["text"].as_str().unwrap();
    println!(
        "[QI-CLOUD ZDR BRIEFING] ({}) => {}",
        st_zdr["briefing"]["provider_label"], zdr_text
    );
    assert!(!zdr_text.contains("<think>"));
    assert!(!zdr_text.is_empty());

    engine
        .update_sync_config(
            "byo_server".into(),
            "".into(),
            base_url.clone(),
            "".into(),
            "".into(),
        )
        .unwrap();
    engine
        .log_natural_intake("晚餐吃潮汕牛肉火锅并加了一份西兰花".into())
        .unwrap();
    engine.run_e2ee_sync_now().unwrap();
    let phrase = st_zdr["settings"]["recovery_phrase"]
        .as_str()
        .unwrap()
        .to_owned();

    let mut new_device = QiEngine::new(Some(":empty:".to_owned())).unwrap();
    new_device
        .update_cloud_session(base_url.clone(), "live-tester@qi.cn".into(), token)
        .unwrap();
    new_device
        .restore_from_recovery_phrase(phrase, "byo_server".into(), base_url)
        .unwrap();
    let restored: Value = serde_json::from_str(&new_device.state_json().unwrap()).unwrap();
    println!(
        "[RESTORED ON NEW DEVICE] summary={} | intakes={}",
        restored["settings"]["last_sync_summary"],
        restored["intakes"].as_array().unwrap().len()
    );
    assert!(restored["intakes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["item_name"].as_str().unwrap_or("").contains("牛肉火锅")));

    let _ = std::fs::remove_file(&cloud_db);
}

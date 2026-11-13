use nsh_app_qi::analytics::{
    AnalyticsSnapshot, DailyMetricRow, HardwareTier, TrendPoint, compute_snapshot,
};
use nsh_app_qi::personas::PersonaMeta;
use nsh_app_qi::storage::AppSettings;
use nsh_app_qi::{QiEngine, QiMeta};
use chrono::{Duration, NaiveDate};
use serde_json::Value;

fn parse_json(raw: &str) -> Value {
    serde_json::from_str(raw).expect("valid json payload")
}

#[test]
fn personas_metadata_lists_all_six_with_valid_contracts() {
    let metas: Vec<PersonaMeta> =
        serde_json::from_str(&QiMeta::personas_json()).expect("valid personas json");
    assert_eq!(metas.len(), 6);

    let expected_ids = [
        "urban_overwork",
        "endurance_runner",
        "metabolic_care",
        "band_commuter",
        "phone_minimalist",
        "sparse_chaotic",
    ];
    for id in expected_ids {
        let m = metas.iter().find(|p| p.id == id).expect("persona present");
        assert!(!m.name.is_empty());
        assert!(!m.badge.is_empty());
        assert!(!m.summary.is_empty());
        assert!(!m.highlight_metrics.is_empty());
        assert!(matches!(
            m.hardware_tier.as_str(),
            "tier_a" | "tier_b" | "tier_c"
        ));
        assert!(m.days_count >= 30);
    }
}

#[test]
fn all_six_personas_pass_data_integrity_audit() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let ids = [
        "urban_overwork",
        "endurance_runner",
        "metabolic_care",
        "band_commuter",
        "phone_minimalist",
        "sparse_chaotic",
    ];

    for id in ids {
        let state = parse_json(&engine.load_persona(id.to_owned()).unwrap());
        assert_eq!(state["settings"]["active_persona"], id);
        assert_eq!(state["settings"]["onboarding_done"], true);
        assert_eq!(state["settings"]["checked_action_today"], false);
        assert_eq!(state["snapshot"]["target_date"], "2026-11-13");

        let audit = parse_json(&engine.run_data_integrity_audit().unwrap());
        assert_eq!(audit["active_persona"], id);
        assert_eq!(audit["passed"], true, "audit failed for {id}: {audit}");
        assert_eq!(audit["qi_score_in_bounds"], true);
        assert_eq!(audit["rmssd_sdnn_isolated"], true);
        assert_eq!(audit["tier_hrv_contract_held"], true);
        assert_eq!(audit["lab_trends_consistent"], true);
        assert_eq!(audit["pii_guard_verified"], true);

        let checks = audit["checks"].as_array().unwrap();
        assert_eq!(checks.len(), 6);
        for c in checks {
            assert_eq!(c["passed"], true, "check failed for {id}: {c}");
        }
    }
}

#[test]
fn urban_overwork_sleep_short_recovery_and_lagged_impacts() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let state = parse_json(&engine.load_persona("urban_overwork".into()).unwrap());

    assert_eq!(state["snapshot"]["state"], "recovery_priority");
    assert!(state["snapshot"]["metrics"]["hrv_rmssd_ms"]["value"].as_f64().unwrap() < 42.0);
    assert!(state["snapshot"]["metrics"]["hrv_sdnn_ms"]["value"].as_f64().is_some());
    assert_ne!(
        state["snapshot"]["metrics"]["hrv_rmssd_ms"]["source"],
        state["snapshot"]["metrics"]["hrv_sdnn_ms"]["source"]
    );

    // 睡眠分期应有完整 4 段拆分
    let stages = &state["snapshot"]["sleep_stages_last_night"];
    assert!(!stages.is_null());
    let total_stages = stages["deep_min"].as_f64().unwrap()
        + stages["core_min"].as_f64().unwrap()
        + stages["rem_min"].as_f64().unwrap()
        + stages["awake_min"].as_f64().unwrap();
    assert!((total_stages - 320.0).abs() < 1.0);

    // 负向 D+1 归因（15:00后冰美式 / 22:30夜宵烧烤）
    let impacts = state["snapshot"]["lagged_impacts"].as_array().unwrap();
    assert!(!impacts.is_empty());
    assert!(impacts.iter().any(|imp| {
        let tag = imp["tag"].as_str().unwrap_or("");
        tag.contains("冰美式") || tag.contains("夜宵烧烤")
    }));

    // 维 D 2026-08-12 18.4 -> 2026-10-20 26.5 improved + 尿酸/ALT 偏高
    let labs = state["labs"].as_array().unwrap();
    let vd_latest = labs
        .iter()
        .find(|l| l["canonical_key"] == "vitamin_d" && l["test_date"] == "2026-10-20")
        .unwrap();
    assert_eq!(vd_latest["value"], 26.5);
    assert_eq!(vd_latest["prev_value"], 18.4);
    assert_eq!(vd_latest["trend"], "improved");
    assert!(labs.iter().any(|l| l["canonical_key"] == "uric_acid" && l["flag"] == "high"));
    assert!(labs.iter().any(|l| l["canonical_key"] == "alt" && l["flag"] == "high"));

    let exps = state["experiments"].as_array().unwrap();
    assert!(exps.iter().any(|e| e["title"] == "15:00 后停摄入咖啡因"));
}

#[test]
fn endurance_runner_dedup_priority_and_overload_acwr() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let state = parse_json(&engine.load_persona("endurance_runner".into()).unwrap());

    // Garmin 优先级高于 Apple Health：RHR 应为 46..=49，而非 Apple Health 的 +5
    let rhr = &state["snapshot"]["metrics"]["resting_heart_rate_bpm"];
    assert_eq!(rhr["source"], "garmin");
    let rhr_val = rhr["value"].as_f64().unwrap();
    assert!((46.0..=49.0).contains(&rhr_val), "unexpected rhr {rhr_val}");

    // HRV RMSSD (68..=75) 与 Apple Health SDNN 独立并存
    let rmssd = state["snapshot"]["metrics"]["hrv_rmssd_ms"]["value"].as_f64().unwrap();
    assert!((68.0..=75.0).contains(&rmssd), "unexpected rmssd {rmssd}");
    assert_eq!(
        state["snapshot"]["metrics"]["hrv_sdnn_ms"]["source"],
        "apple_health"
    );

    // ACWR 落在 1.38~1.48 过载警戒区间
    let acwr = state["snapshot"]["acwr"].as_f64().unwrap();
    assert!((1.38..=1.48).contains(&acwr), "acwr out of target range: {acwr}");

    let labs = state["labs"].as_array().unwrap();
    assert!(labs.iter().any(|l| l["canonical_key"] == "ferritin"));
    assert!(labs.iter().any(|l| l["canonical_key"] == "hemoglobin"));
    assert!(labs.iter().any(|l| l["canonical_key"] == "ck" && l["flag"] == "high"));

    let exps = state["experiments"].as_array().unwrap();
    assert!(exps.iter().any(|e| e["title"] == "跑后补充甘氨酸镁"));
}

#[test]
fn metabolic_care_prime_ready_and_three_period_lab_trends() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let state = parse_json(&engine.load_persona("metabolic_care".into()).unwrap());

    assert_eq!(state["snapshot"]["state"], "prime_ready");
    assert!(state["snapshot"]["qi_score"].as_u64().unwrap() >= 82);

    let labs = state["labs"].as_array().unwrap();
    for key in ["ldl_c", "triglycerides", "hba1c", "uric_acid", "alt", "fasting_glucose"] {
        let latest = labs
            .iter()
            .find(|l| l["canonical_key"] == key && l["test_date"] == "2026-11-10")
            .unwrap_or_else(|| panic!("missing 2026-11-10 lab for {key}"));
        assert_eq!(latest["flag"], "normal", "expected normal for {key}");
        assert_eq!(latest["trend"], "improved", "expected improved for {key}");
    }

    // 检查 3 期序列值精确匹配
    let ldl_may = labs
        .iter()
        .find(|l| l["canonical_key"] == "ldl_c" && l["test_date"] == "2026-05-15")
        .unwrap();
    assert_eq!(ldl_may["value"], 4.12);
    let ldl_aug = labs
        .iter()
        .find(|l| l["canonical_key"] == "ldl_c" && l["test_date"] == "2026-08-18")
        .unwrap();
    assert_eq!(ldl_aug["value"], 3.68);
    let ldl_nov = labs
        .iter()
        .find(|l| l["canonical_key"] == "ldl_c" && l["test_date"] == "2026-11-10")
        .unwrap();
    assert_eq!(ldl_nov["value"], 3.18);

    // 14 天「晚餐提前至 19:30 前」实验已显效
    let exp = state["experiments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["title"] == "晚餐提前至 19:30 前")
        .unwrap();
    assert_eq!(exp["completed_days"], 14);
    assert!(exp["hrv_delta_ms"].as_f64().unwrap() > 0.0);
    assert!(exp["rhr_delta_bpm"].as_f64().unwrap() < 0.0);
    assert!(exp["deep_sleep_delta_min"].as_f64().unwrap() > 0.0);
}

#[test]
fn band_commuter_tier_b_never_fabricates_hrv_or_sleep_stages() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let state = parse_json(&engine.load_persona("band_commuter".into()).unwrap());

    assert_eq!(state["snapshot"]["hardware_tier"], "tier_b");
    assert_eq!(state["snapshot"]["state"], "moderate_steady");
    assert!(state["snapshot"]["sleep_stages_last_night"].is_null());

    let metrics = state["snapshot"]["metrics"].as_object().unwrap();
    assert!(!metrics.contains_key("hrv_rmssd_ms"));
    assert!(!metrics.contains_key("hrv_sdnn_ms"));
    assert!(!metrics.contains_key("sleep_deep_minutes"));
    assert_eq!(metrics["sleep_minutes"]["source"], "xiaomi");
    assert_eq!(metrics["steps"]["source"], "xiaomi");

    for pt in state["snapshot"]["trend_14d"].as_array().unwrap() {
        assert!(pt["hrv_ms"].is_null());
        assert!(pt["sleep_h"].as_f64().is_some());
    }

    let labs = state["labs"].as_array().unwrap();
    assert!(labs.iter().any(|l| l["canonical_key"] == "ferritin" && l["flag"] == "low"));
}

#[test]
fn phone_minimalist_tier_c_steps_only_and_chronic_labs() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let state = parse_json(&engine.load_persona("phone_minimalist".into()).unwrap());

    assert_eq!(state["snapshot"]["hardware_tier"], "tier_c");
    assert_eq!(state["settings"]["morning_feeling"], 4);
    assert_eq!(state["snapshot"]["qi_score"], 85);
    assert!(state["snapshot"]["sleep_stages_last_night"].is_null());
    assert!(state["snapshot"]["sleep_last_night_hours"].is_null());
    assert!(state["snapshot"]["acwr"].is_null());

    let metrics = state["snapshot"]["metrics"].as_object().unwrap();
    assert_eq!(metrics.len(), 1);
    let steps = metrics["steps"]["value"].as_f64().unwrap();
    assert!((5500.0..=8800.0).contains(&steps));
    assert_eq!(metrics["steps"]["source"], "phone_coremotion");

    for pt in state["snapshot"]["trend_14d"].as_array().unwrap() {
        assert!(pt["hrv_ms"].is_null());
        assert!(pt["sleep_h"].is_null());
    }

    let labs = state["labs"].as_array().unwrap();
    for key in ["fasting_glucose", "hba1c", "ldl_c", "creatinine", "tsh", "uric_acid"] {
        assert!(
            labs.iter().any(|l| l["canonical_key"] == key),
            "missing chronic marker {key}"
        );
    }
}

#[test]
fn sparse_chaotic_9day_gap_extremes_and_zero_checkin_experiment() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let state = parse_json(&engine.load_persona("sparse_chaotic".into()).unwrap());
    let audit = parse_json(&engine.run_data_integrity_audit().unwrap());

    // 45 天跨度扣除连续 9 天断戴 = 36 个活跃日
    assert_eq!(audit["distinct_days"], 36);
    assert_eq!(audit["passed"], true);

    let score = state["snapshot"]["qi_score"].as_u64().unwrap();
    assert!((15..=99).contains(&score));

    // 多源冲突在 2026-11-13 必须落到 garmin
    assert_eq!(state["snapshot"]["metrics"]["steps"]["source"], "garmin");
    assert_eq!(
        state["snapshot"]["metrics"]["resting_heart_rate_bpm"]["source"],
        "garmin"
    );

    // 0 打卡实验不会除零崩溃
    let exp = &state["experiments"].as_array().unwrap()[0];
    assert_eq!(exp["completed_days"], 0);
    assert!(exp["hrv_delta_ms"].is_null());

    // 全角化验单已正确解析且未泄露天头/证件/手机
    let labs = state["labs"].as_array().unwrap();
    assert!(labs.iter().any(|l| l["canonical_key"] == "alt" && l["value"] == 68.0));
    assert!(labs.iter().any(|l| l["canonical_key"] == "uric_acid" && l["value"] == 492.0));
    let serialized = serde_json::to_string(&state).unwrap();
    assert!(!serialized.contains("31010119900101123X"));
    assert!(!serialized.contains("13800138000"));
}

#[test]
fn sleep_stages_breakdown_formula_and_tier_gates() {
    let date = "2026-11-13".to_owned();
    let full_rows = vec![
        DailyMetricRow {
            date: date.clone(),
            metric: "sleep_minutes".into(),
            source: "garmin".into(),
            value: 450.0,
            unit: "min".into(),
        },
        DailyMetricRow {
            date: date.clone(),
            metric: "sleep_deep_minutes".into(),
            source: "garmin".into(),
            value: 95.0,
            unit: "min".into(),
        },
        DailyMetricRow {
            date: date.clone(),
            metric: "sleep_rem_minutes".into(),
            source: "garmin".into(),
            value: 105.0,
            unit: "min".into(),
        },
    ];

    let snap_a = compute_snapshot(&full_rows, &[], &[], HardwareTier::TierA, 4);
    let stages = snap_a.sleep_stages_last_night.expect("tier A stages");
    assert_eq!(stages.deep_min, 95.0);
    assert_eq!(stages.rem_min, 105.0);
    assert_eq!(stages.awake_min, 27.0); // (450 * 0.06).round() = 27
    assert_eq!(stages.core_min, 223.0); // 450 - 95 - 105 - 27

    // TierB / TierC 即使库中有分期行也严格返回 None
    assert!(
        compute_snapshot(&full_rows, &[], &[], HardwareTier::TierB, 4)
            .sleep_stages_last_night
            .is_none()
    );
    assert!(
        compute_snapshot(&full_rows, &[], &[], HardwareTier::TierC, 4)
            .sleep_stages_last_night
            .is_none()
    );

    // 缺少 deep & rem 时严格返回 None
    let no_stage_rows = vec![DailyMetricRow {
        date: date.clone(),
        metric: "sleep_minutes".into(),
        source: "garmin".into(),
        value: 450.0,
        unit: "min".into(),
    }];
    assert!(
        compute_snapshot(&no_stage_rows, &[], &[], HardwareTier::TierA, 4)
            .sleep_stages_last_night
            .is_none()
    );

    // deep + rem + awake > total 时 core clamp 为 0.0
    let overflow_rows = vec![
        DailyMetricRow {
            date: date.clone(),
            metric: "sleep_minutes".into(),
            source: "garmin".into(),
            value: 200.0,
            unit: "min".into(),
        },
        DailyMetricRow {
            date: date.clone(),
            metric: "sleep_deep_minutes".into(),
            source: "garmin".into(),
            value: 120.0,
            unit: "min".into(),
        },
        DailyMetricRow {
            date,
            metric: "sleep_rem_minutes".into(),
            source: "garmin".into(),
            value: 90.0,
            unit: "min".into(),
        },
    ];
    let snap_overflow = compute_snapshot(&overflow_rows, &[], &[], HardwareTier::TierA, 4);
    assert_eq!(snap_overflow.sleep_stages_last_night.unwrap().core_min, 0.0);
}

#[test]
fn trend_point_hrv_and_sleep_visibility_and_sdnn_fallback() {
    let start = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
    let mut rows = Vec::new();
    for i in 0..15_i64 {
        let d = (start + Duration::days(i)).format("%Y-%m-%d").to_string();
        rows.push(DailyMetricRow {
            date: d.clone(),
            metric: "hrv_sdnn_ms".into(),
            source: "apple_health".into(),
            value: 48.0 + i as f64,
            unit: "ms".into(),
        });
        rows.push(DailyMetricRow {
            date: d,
            metric: "sleep_minutes".into(),
            source: "apple_health".into(),
            value: 420.0,
            unit: "min".into(),
        });
    }

    // TierA 无 RMSSD 时自动回退到 SDNN
    let snap_a = compute_snapshot(&rows, &[], &[], HardwareTier::TierA, 4);
    assert_eq!(snap_a.trend_14d.len(), 14);
    assert_eq!(snap_a.trend_14d.last().unwrap().hrv_ms, Some(62.0));
    assert_eq!(snap_a.trend_14d.last().unwrap().sleep_h, Some(7.0));

    // TierB 隐藏 HRV 但保留睡眠时长
    let snap_b = compute_snapshot(&rows, &[], &[], HardwareTier::TierB, 4);
    assert!(snap_b.trend_14d.iter().all(|p| p.hrv_ms.is_none()));
    assert_eq!(snap_b.trend_14d.last().unwrap().sleep_h, Some(7.0));

    // TierC 同时隐藏 HRV 与睡眠时长
    let snap_c = compute_snapshot(&rows, &[], &[], HardwareTier::TierC, 4);
    assert!(snap_c
        .trend_14d
        .iter()
        .all(|p| p.hrv_ms.is_none() && p.sleep_h.is_none()));
}

#[test]
fn load_persona_preserves_cloud_credentials_recovery_phrase_and_sync_config() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    engine
        .update_cloud_session(
            "https://cloud.example.com".into(),
            "tester@qi.app".into(),
            "secret-jwt-token".into(),
        )
        .unwrap();
    engine
        .update_sync_config(
            "webdav".into(),
            "https://dav.example.com/qi/".into(),
            "https://byo.example.com".into(),
            "dav_user".into(),
            "dav_pass".into(),
        )
        .unwrap();
    let before = parse_json(&engine.state_json().unwrap());
    let phrase = before["settings"]["recovery_phrase"].as_str().unwrap().to_owned();
    let dev_id = before["settings"]["device_id"].as_str().unwrap().to_owned();

    let after = parse_json(&engine.load_persona("metabolic_care".into()).unwrap());
    assert_eq!(after["settings"]["active_persona"], "metabolic_care");
    assert_eq!(after["settings"]["cloud_url"], "https://cloud.example.com");
    assert_eq!(after["settings"]["cloud_handle"], "tester@qi.app");
    assert_eq!(after["settings"]["cloud_token"], "secret-jwt-token");
    assert_eq!(after["settings"]["sync_mode"], "webdav");
    assert_eq!(after["settings"]["webdav_url"], "https://dav.example.com/qi/");
    assert_eq!(after["settings"]["webdav_username"], "dav_user");
    assert_eq!(after["settings"]["webdav_password"], "dav_pass");
    assert_eq!(after["settings"]["recovery_phrase"], phrase);
    assert_eq!(after["settings"]["device_id"], dev_id);
}

#[test]
fn load_persona_rejects_unknown_id_without_wiping_existing_data() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    engine.load_persona("metabolic_care".into()).unwrap();

    let err = engine.load_persona("non_existent_persona".into()).unwrap_err();
    assert!(err.contains("未知测试画像"));

    let state = parse_json(&engine.state_json().unwrap());
    assert_eq!(state["settings"]["active_persona"], "metabolic_care");
    assert!(!state["labs"].as_array().unwrap().is_empty());
}

#[test]
fn fullwidth_dirty_chinese_ocr_pii_scrub_and_lab_reverification() {
    let mut engine = QiEngine::new(Some(":empty:".to_owned())).unwrap();
    let dirty_ocr = "某某市第一人民医院体检中心\n\
        姓名：王小二 身份证号：44030119880808123X 手机：13987654321\n\
        尿酸（UA）：495↓ umol/L 208–428\n\
        25-羟维生素D：19.5↑ ng/mL 30.0–100.0\n\
        空腹血糖（GLU）：5.10 mmol/L 3.9–6.1";

    let state = parse_json(&engine.import_lab_report("2026-11-12".into(), dirty_ocr.into()).unwrap());
    let feedback = &state["last_lab_import"];
    let preview = feedback["scrubbed_preview"].as_str().unwrap();
    assert!(!preview.contains("王小二"));
    assert!(!preview.contains("44030119880808123X"));
    assert!(!preview.contains("13987654321"));
    assert_eq!(feedback["imported_count"], 3);

    // 即使原文箭头写反（495↓、19.5↑），Rust 仍按区间确定性重算
    let labs = state["labs"].as_array().unwrap();
    let ua = labs.iter().find(|l| l["canonical_key"] == "uric_acid").unwrap();
    assert_eq!(ua["flag"], "high");
    let vd = labs.iter().find(|l| l["canonical_key"] == "vitamin_d").unwrap();
    assert_eq!(vd["flag"], "low");
    let glu = labs.iter().find(|l| l["canonical_key"] == "fasting_glucose").unwrap();
    assert_eq!(glu["flag"], "normal");
}

#[test]
fn e2ee_encrypted_archive_export_and_restore_after_persona_switch() {
    let mut engine_a = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let st_a = parse_json(&engine_a.load_persona("metabolic_care".into()).unwrap());
    let phrase = st_a["settings"]["recovery_phrase"].as_str().unwrap().to_owned();
    let archive_json = engine_a.export_encrypted_archive().unwrap();

    let mut engine_b = QiEngine::new(Some(":empty:".to_owned())).unwrap();
    let st_b = parse_json(
        &engine_b
            .restore_from_recovery_phrase(phrase, "local_archive".into(), archive_json)
            .unwrap(),
    );
    assert_eq!(st_b["settings"]["onboarding_done"], true);
    assert_eq!(st_b["snapshot"]["qi_score"], st_a["snapshot"]["qi_score"]);
    assert_eq!(
        st_b["labs"].as_array().unwrap().len(),
        st_a["labs"].as_array().unwrap().len()
    );
    assert_eq!(
        st_b["experiments"].as_array().unwrap().len(),
        st_a["experiments"].as_array().unwrap().len()
    );

    let audit_b = parse_json(&engine_b.run_data_integrity_audit().unwrap());
    assert_eq!(audit_b["passed"], true);
}

#[test]
fn empty_database_snapshot_and_audit_zero_panic() {
    let engine = QiEngine::new(Some(":empty:".to_owned())).unwrap();
    let state = parse_json(&engine.state_json().unwrap());
    let score = state["snapshot"]["qi_score"].as_u64().unwrap();
    assert!((15..=99).contains(&score));
    assert!(state["snapshot"]["sleep_stages_last_night"].is_null());
    assert_eq!(state["snapshot"]["trend_14d"].as_array().unwrap().len(), 14);

    let audit = parse_json(&engine.run_data_integrity_audit().unwrap());
    assert_eq!(audit["metric_rows"], 0);
    assert_eq!(audit["passed"], true);
}

#[test]
fn backward_compatibility_serde_defaults_for_v1_json() {
    let v1_trend = r#"{"date":"2026-11-13","short_label":"11/13","qi_score":72,"state":"moderate_steady"}"#;
    let pt: TrendPoint = serde_json::from_str(v1_trend).unwrap();
    assert!(pt.hrv_ms.is_none());
    assert!(pt.sleep_h.is_none());

    let v1_snap = r#"{
        "target_date":"2026-11-13",
        "hardware_tier":"tier_a",
        "hardware_tier_label":"全功能穿戴",
        "qi_score":75,
        "state":"moderate_steady",
        "state_badge":"守中 · 稳行",
        "headline":"测试",
        "micro_action":"测试",
        "signals":[],
        "sleep_debt_7d_hours":null,
        "sleep_last_night_hours":7.2,
        "acwr":1.05,
        "metrics":{},
        "trend_14d":[],
        "lagged_impacts":[]
    }"#;
    let snap: AnalyticsSnapshot = serde_json::from_str(v1_snap).unwrap();
    assert!(snap.sleep_stages_last_night.is_none());

    let mut settings_val = serde_json::to_value(AppSettings::default()).unwrap();
    settings_val.as_object_mut().unwrap().remove("active_persona");
    let decoded: AppSettings = serde_json::from_value(settings_val).unwrap();
    assert_eq!(decoded.active_persona, "urban_overwork");
}

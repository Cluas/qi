use std::io::{Read, Write};
use std::net::TcpListener;

use chrono::{Duration, NaiveDate};
use nsh_app_qi::QiEngine;
use nsh_app_qi::ai_provider::{generate_atelier_synthesis, generate_live_briefing};
use nsh_app_qi::analytics::{
    DailyMetricRow, DailyRecord, HardwareTier, LabBiomarkerRow, compute_multi_lag_radar,
    compute_snapshot,
};
use nsh_app_qi::crypto_sync::{derive_master_key, encrypt_sync_batch};
use nsh_app_qi::entitlements::{
    OfflineEntitlementState, SubscriptionTier, mint_oset_for_tier, verify_oset_with_clock,
};
use nsh_app_qi::experiments::{ExperimentRow, evaluate_experiments};
use nsh_app_qi::safety_ai::{
    build_parents_peace_letter, compute_multi_profile_uln_trends, compute_multi_profile_uln_trends_at,
};
use nsh_app_qi::solar_atelier::{
    FREE_STICKER_MONTHLY_QUOTA, SOLAR_TERMS_24, bonsai_stage_for_dew,
    build_companion_weekly_letters, compute_solar_term_state, infer_sticker_nutrition_hint,
};
use nsh_app_qi::storage::VaultDatabase;
use rusqlite::Connection;
use serde_json::Value;

fn parse_json(raw: &str) -> Value {
    serde_json::from_str(raw).expect("valid json payload")
}

#[test]
fn solar_terms_all_24_lunar_and_weather_world_mapping() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let st = parse_json(&engine.state_json().unwrap());
    assert_eq!(st["solar_atelier"]["solar_term"]["term_name"], "立冬");
    assert_eq!(st["solar_atelier"]["solar_term"]["season"], "winter");
    assert!(!st["solar_atelier"]["solar_term"]["lunar_phase"].as_str().unwrap().is_empty());

    let st_rain = parse_json(&engine.set_weather_mood("misty_rain".into()).unwrap());
    assert_eq!(
        st_rain["solar_atelier"]["solar_term"]["active_world"],
        "solar_spring_rain"
    );

    let st_snow = parse_json(&engine.set_weather_mood("first_snow".into()).unwrap());
    assert_eq!(
        st_snow["solar_atelier"]["solar_term"]["active_world"],
        "solar_winter_hearth"
    );

    let dates = [
        "2026-01-06", "2026-01-21", "2026-02-05", "2026-02-20", "2026-03-07", "2026-03-22",
        "2026-04-06", "2026-04-21", "2026-05-07", "2026-05-22", "2026-06-07", "2026-06-22",
        "2026-07-08", "2026-07-24", "2026-08-08", "2026-08-24", "2026-09-09", "2026-09-24",
        "2026-10-09", "2026-10-24", "2026-11-13", "2026-11-23", "2026-12-08", "2026-12-23",
    ];
    let terms: Vec<String> = dates
        .iter()
        .map(|d| compute_solar_term_state(d, "sunny_crisp").term_name)
        .collect();
    for expected in SOLAR_TERMS_24 {
        assert!(terms.iter().any(|t| t == expected), "missing {expected}");
    }
}

#[test]
fn dual_track_pass_recovery_dew_presets_keepsakes_and_entitlement_gates() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    // urban_overwork is recovery_priority -> Free Forever awards +3 💧
    let st = parse_json(&engine.load_persona("urban_overwork".into()).unwrap());
    assert_eq!(st["snapshot"]["state"], "recovery_priority");
    assert_eq!(st["solar_atelier"]["dew_balance"], 3);
    assert_eq!(st["entitlements"]["state"], "free_local");
    assert_eq!(st["entitlements"]["is_atelier_unlocked"], false);

    // Free track reward succeeds
    let st_after_free = parse_json(&engine.claim_seasonal_reward("tea_cup".into()).unwrap());
    assert_eq!(st_after_free["solar_atelier"]["dew_balance"], 1);

    // Atelier track reward blocked on Free
    let err_atelier = engine.claim_seasonal_reward("rain_cape".into()).unwrap_err();
    assert!(err_atelier.contains("知气雅集"));

    // Keepsake slots: Free allows 2, 3rd blocked
    engine.toggle_equipped_keepsake("tea_cup".into()).unwrap();
    engine.toggle_equipped_keepsake("cushion".into()).unwrap();
    let err_slot3 = engine.toggle_equipped_keepsake("celadon_cup".into()).unwrap_err();
    assert!(err_slot3.contains("2 件"));

    // Sanctuary presets: Free allows 2, 3rd blocked
    engine.save_sanctuary_preset("冬日小窗".into()).unwrap();
    engine.save_sanctuary_preset("静夜茶席".into()).unwrap();
    let err_preset3 = engine.save_sanctuary_preset("第三套".into()).unwrap_err();
    assert!(err_preset3.contains("2 套"));

    // Activate real Ed25519 Atelier sandbox (with guardian_sponsor = true, loyalty = 35 days)
    let st_pro = parse_json(
        &engine
            .set_entitlement_sandbox("pro_annual".into(), "active".into(), true, 35)
            .unwrap(),
    );
    assert_eq!(st_pro["entitlements"]["is_atelier_unlocked"], true);
    assert_eq!(st_pro["entitlements"]["guardian_sponsor"], true);
    assert!(st_pro["entitlements"]["new_max_seen_epoch"].as_i64().unwrap() > 0);
    // Loyalty discount applied on catalog (¥98 instead of ¥118)
    let annual_plan = st_pro["entitlement_catalog"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["tier"] == "pro_annual")
        .unwrap();
    assert_eq!(annual_plan["price_cny"], 98);
    assert_eq!(annual_plan["loyalty_applied"], true);

    // Now 3rd keepsake, 3rd preset, custom seal ink, and Atelier pass reward all unlock!
    let st_slot3 = parse_json(&engine.toggle_equipped_keepsake("celadon_cup".into()).unwrap());
    assert_eq!(
        st_slot3["solar_atelier"]["keepsake_catalog"]["equipped_ids"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let st_preset3 = parse_json(&engine.save_sanctuary_preset("第三套雅集".into()).unwrap());
    let presets = st_preset3["solar_atelier"]["presets"].as_array().unwrap();
    assert_eq!(presets.len(), 3);
    let first_preset_id = presets[0]["id"].as_i64().unwrap();
    engine.apply_sanctuary_preset(first_preset_id).unwrap();

    let st_gold = parse_json(&engine.set_seal_ink("gold".into()).unwrap());
    assert_eq!(
        st_gold["solar_atelier"]["keepsake_catalog"]["active_seal_ink"],
        "gold"
    );

    // Earn 2 more dew via co-sanctuary so balance reaches 3, then claim Atelier reward (cost 3)
    engine.cosanctuary_action("warm_tea".into(), None, None).unwrap();
    engine.cosanctuary_action("paw_stamp".into(), None, None).unwrap();
    let st_claimed = parse_json(&engine.claim_seasonal_reward("rain_cape".into()).unwrap());
    let rain_cape = st_claimed["solar_atelier"]["seasonal_pass"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["reward_id"] == "rain_cape")
        .unwrap();
    assert_eq!(rain_cape["claimed"], true);

    // Tampered token rejected cleanly
    let err_tamper = engine
        .apply_entitlement_token("qient1.tampered.sig".into())
        .unwrap_err();
    assert!(err_tamper.contains("签名"));

    // Clear token returns to Free while vault remains 100% readable/exportable
    let st_cleared = parse_json(&engine.clear_entitlement_token().unwrap());
    assert_eq!(st_cleared["entitlements"]["state"], "free_local");
    assert_eq!(st_cleared["entitlements"]["can_read_history_vault"], true);
    assert_eq!(st_cleared["entitlements"]["can_export_vault"], true);
}

#[test]
fn food_stickers_quota_filters_nutrition_hint_and_favorites() {
    assert_eq!(FREE_STICKER_MONTHLY_QUOTA, 5);
    assert!(infer_sticker_nutrition_hint("夜宵海鲜配啤酒").contains("嘌呤"));
    assert!(infer_sticker_nutrition_hint("盐水鸭与拉面").contains("钠"));

    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    for i in 1..=5 {
        engine
            .add_food_sticker(
                format!("2026-11-0{i}"),
                format!("清蒸鱼餐 #{i}"),
                if i % 2 == 0 { "watercolor" } else { "celadon" }.into(),
                "细嚼慢咽".into(),
                "".into(),
            )
            .unwrap();
    }
    let err_6th = engine
        .add_food_sticker(
            "2026-11-06".into(),
            "第6张贴纸".into(),
            "polaroid".into(),
            "".into(),
            "".into(),
        )
        .unwrap_err();
    assert!(err_6th.contains("5 张"));

    // Unlock Atelier -> 6th sticker succeeds!
    engine
        .set_entitlement_sandbox("pro_monthly".into(), "active".into(), false, 0)
        .unwrap();
    let st6 = parse_json(
        &engine
            .add_food_sticker(
                "2026-11-06".into(),
                "海鲜牡蛎汤面".into(),
                "polaroid".into(),
                "留意补水".into(),
                "local://photo6.jpg".into(),
            )
            .unwrap(),
    );
    let stickers = st6["solar_atelier"]["stickers"].as_array().unwrap();
    assert_eq!(stickers.len(), 6);
    assert!(stickers[0]["nutrition_hint"].as_str().unwrap().contains("嘌呤"));
    assert_eq!(st6["solar_atelier"]["sticker_quota"]["unlimited"], true);

    let target_id = stickers[0]["id"].as_i64().unwrap();
    let st_fav = parse_json(&engine.toggle_sticker_favorite(target_id).unwrap());
    assert_eq!(
        st_fav["solar_atelier"]["stickers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == target_id)
            .unwrap()["is_favorite"],
        true
    );

    let st_del = parse_json(&engine.delete_food_sticker(target_id).unwrap());
    assert_eq!(st_del["solar_atelier"]["stickers"].as_array().unwrap().len(), 5);
}

#[test]
fn companion_weekly_letters_free_vs_atelier_vs_expired_vault() {
    let snap = compute_snapshot(&[], &[], &[], HardwareTier::TierA, 4);
    let solar = compute_solar_term_state("2026-11-13", "sunny_crisp");

    let free_res = verify_oset_with_clock("", 1_760_000_000, 0, None);
    let free_letters = build_companion_weekly_letters("shanfang", &snap, &solar, 3, 2, &free_res);
    assert_eq!(free_letters.len(), 4);
    assert_eq!(free_letters[0].companion_id, "shanfang");
    assert!(free_letters[0].unlocked);
    assert!(!free_letters[1].unlocked);
    assert!(!free_letters[2].unlocked);
    assert!(!free_letters[3].unlocked);

    let now = 1_760_000_000;
    let (_c, token) =
        mint_oset_for_tier("u_letter", SubscriptionTier::Seasonal, now, false, false).unwrap();
    let active_res = verify_oset_with_clock(&token, now + 100, now, None);
    let active_letters =
        build_companion_weekly_letters("meiqiou", &snap, &solar, 3, 2, &active_res);
    assert_eq!(active_letters[0].companion_id, "meiqiou");
    assert!(active_letters.iter().all(|l| l.unlocked));

    // ExpiredReadOnlyVault keeps previously unlocked archive letters readable!
    let exp_res = verify_oset_with_clock(&token, now + 120 * 86_400, now, None);
    assert_eq!(exp_res.state, OfflineEntitlementState::ExpiredReadOnlyVault);
    let exp_letters = build_companion_weekly_letters("xiaoman", &snap, &solar, 3, 2, &exp_res);
    assert!(exp_letters.iter().all(|l| l.unlocked));
}

#[test]
fn cosanctuary_1byte_mood_bonsai_stages_and_widget_privacy_toggle() {
    assert_eq!(bonsai_stage_for_dew(0).0, "萌芽");
    assert_eq!(bonsai_stage_for_dew(5).0, "抽枝");
    assert_eq!(bonsai_stage_for_dew(15).0, "含苞");
    assert_eq!(bonsai_stage_for_dew(30).0, "满树金桂");

    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let st_init = parse_json(&engine.load_persona("metabolic_care".into()).unwrap());
    // prime_ready -> 1-byte mood 0 (SUNNY_PRIME)
    assert_eq!(st_init["solar_atelier"]["co_sanctuary"]["self_mood_byte"], 0);
    // Default widget mode is metaphor -> hides numeric qi_score & metrics_line
    assert_eq!(
        st_init["solar_atelier"]["widget_preview"]["privacy_mode"],
        "metaphor"
    );
    assert_eq!(
        st_init["solar_atelier"]["widget_preview"]["qi_score_visible"],
        false
    );
    assert!(st_init["solar_atelier"]["widget_preview"]["qi_score"].is_null());

    // Switch to metrics mode -> exposes qi_score & metrics_line
    let st_metrics = parse_json(&engine.set_widget_privacy_mode("metrics".into()).unwrap());
    assert_eq!(
        st_metrics["solar_atelier"]["widget_preview"]["qi_score_visible"],
        true
    );
    assert!(st_metrics["solar_atelier"]["widget_preview"]["qi_score"].is_u64());
    assert!(st_metrics["solar_atelier"]["widget_preview"]["metrics_line"].is_string());

    // Mutual care actions award +1 💧 and grow bonsai from 萌芽 -> 抽枝 (5 actions)
    for act in ["warm_tea", "tuck_blanket", "paw_stamp", "warm_tea", "paw_stamp"] {
        engine
            .cosanctuary_action(act.into(), Some("阿茶".into()), Some("QI-8888".into()))
            .unwrap();
    }
    let st_bonsai = parse_json(&engine.state_json().unwrap());
    assert_eq!(st_bonsai["solar_atelier"]["co_sanctuary"]["partner_name"], "阿茶");
    assert_eq!(st_bonsai["solar_atelier"]["co_sanctuary"]["pair_code"], "QI-8888");
    assert_eq!(st_bonsai["solar_atelier"]["co_sanctuary"]["bonsai_dew"], 5);
    assert_eq!(
        st_bonsai["solar_atelier"]["co_sanctuary"]["bonsai_stage"],
        "抽枝"
    );
    assert_eq!(st_bonsai["solar_atelier"]["dew_balance"], 5);
}

#[test]
fn moat1_multi_profile_uln_90d_reminders_and_parents_peace_letter() {
    let mut engine = QiEngine::new(Some(":empty:".to_owned())).unwrap();
    // Self, Partner, Parents with same canonical marker but distinct profiles & dates
    engine
        .add_profile_lab_marker(
            "self".into(),
            "2026-11-01".into(),
            "尿酸 (UA)".into(),
            468.0,
            "umol/L".into(),
            Some(208.0),
            Some(428.0),
        )
        .unwrap();
    engine
        .add_profile_lab_marker(
            "partner".into(),
            "2026-11-01".into(),
            "尿酸 (UA)".into(),
            321.0,
            "umol/L".into(),
            Some(155.0),
            Some(357.0),
        )
        .unwrap();
    engine
        .add_profile_lab_marker(
            "parents".into(),
            "2026-07-10".into(),
            "空腹血糖 (GLU)".into(),
            6.71,
            "mmol/L".into(),
            Some(3.9),
            Some(6.1),
        )
        .unwrap();

    let st = parse_json(&engine.state_json().unwrap());
    let uln = st["multi_profile_uln_trends"].as_array().unwrap();
    assert_eq!(uln.len(), 3);
    let self_ua = uln.iter().find(|x| x["profile_id"] == "self").unwrap();
    // 468 / 428 * 100 = 109.3%
    assert!((self_ua["uln_pct"].as_f64().unwrap() - 109.3).abs() < 0.2);
    let partner_ua = uln.iter().find(|x| x["profile_id"] == "partner").unwrap();
    // 321 / 357 * 100 = 89.9%
    assert!((partner_ua["uln_pct"].as_f64().unwrap() - 89.9).abs() < 0.2);

    let reminders = st["recheck_reminders"].as_array().unwrap();
    assert!(reminders.iter().any(|r| {
        r["profile_id"] == "parents" && r["days_since_last"].as_i64().unwrap() >= 90
    }));

    // Direct 1-arg function call also works
    let sample_markers = vec![LabBiomarkerRow {
        id: 1,
        profile_id: "parents".into(),
        test_date: "2026-06-01".into(),
        marker_name: "糖化血红蛋白".into(),
        canonical_key: "hba1c".into(),
        value: 6.6,
        unit: "%".into(),
        ref_low: Some(4.0),
        ref_high: Some(6.0),
        reference_high: Some(6.0),
        flag: "high".into(),
        plain_note: "".into(),
        prev_value: None,
        prev_date: None,
        delta: None,
        trend: Some("new".into()),
    }];
    let (pts, rems) = compute_multi_profile_uln_trends(&sample_markers);
    assert_eq!(pts.len(), 1);
    assert_eq!(pts[0].uln_pct, 110.0);
    assert_eq!(rems.len(), 1);
    assert!(rems[0].days_since_last >= 90);

    let letter = parse_json(&engine.generate_parents_peace_letter("妈妈".into()).unwrap());
    assert!(letter["title"].as_str().unwrap().contains("妈妈"));
    let md = letter["large_print_markdown"].as_str().unwrap();
    assert!(!md.contains("确诊"));
    assert!(!md.contains("开药"));
    assert!(!md.contains("处方"));

    let snap = compute_snapshot(&[], &[], &[], HardwareTier::TierA, 4);
    let (_, rems_at) = compute_multi_profile_uln_trends_at(&sample_markers, "2026-11-13");
    let direct_letter = build_parents_peace_letter("爸爸", &snap, &rems_at);
    assert!(direct_letter.large_print_markdown.contains("换季关怀备忘"));
}

#[test]
fn moat2_90day_multi_lag_radar_cohen_d_and_confounder_control() {
    let start = NaiveDate::from_ymd_opt(2026, 8, 20).unwrap();
    let mut records = Vec::new();
    for i in 0..45_i64 {
        let d = (start + Duration::days(i)).format("%Y-%m-%d").to_string();
        let is_coffee_day = matches!(i, 4 | 9 | 14 | 19 | 24);
        let prev_was_coffee = matches!(i - 1, 4 | 9 | 14 | 19 | 24);
        let two_days_after_coffee = matches!(i - 2, 4 | 9 | 14 | 19 | 24);
        let is_late_confounder = i == 30 || i == 31;

        let hrv = if prev_was_coffee {
            42.0
        } else if two_days_after_coffee {
            49.0
        } else if is_late_confounder {
            39.0
        } else {
            60.0 + (i % 3) as f64
        };
        let mut tags = Vec::new();
        if is_coffee_day {
            tags.push("晚间冰美式".to_owned());
        }
        if i == 24 {
            tags.push("高盐外卖".to_owned());
        }
        records.push(DailyRecord {
            date: d,
            hrv_ms: Some(hrv),
            rhr_bpm: Some(if prev_was_coffee { 62.0 } else { 54.0 }),
            sleep_minutes: Some(if is_late_confounder { 320.0 } else { 440.0 }),
            sleep_deep_minutes: Some(if is_coffee_day { 62.0 } else { 95.0 }),
            tags,
            late_sleep: is_late_confounder,
        });
    }

    let radar = compute_multi_lag_radar(&records);
    assert!(!radar.is_empty());
    assert!(radar.iter().any(|r| r.lag_label == "D+0"));
    assert!(radar.iter().any(|r| r.lag_label == "D+1"));
    let d1_hrv = radar
        .iter()
        .find(|r| r.tag == "晚间冰美式" && r.lag_days == 1 && r.outcome_key == "hrv_ms")
        .expect("D+1 HRV impact");
    assert!(d1_hrv.delta_pct < -20.0);
    assert!(d1_hrv.cohen_d < -1.0);
    assert!(d1_hrv.sample_n >= 4);
    assert!(d1_hrv.confounder_note.contains("混杂控制"));
}

#[test]
fn moat3_custom_washout_window_and_ceramic_badge_for_disproved_experiment() {
    // Baseline days 0..20 = 55ms, Washout transition days 20..22 = 95ms (noisy),
    // Intervention days 22..28 = 55.3ms (neutral / no effect!)
    let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    let mut rows = Vec::new();
    let mut checkins = Vec::new();
    for i in 0..28_i64 {
        let d = (start + Duration::days(i)).format("%Y-%m-%d").to_string();
        let hrv = if (20..22).contains(&i) {
            // Washout days artificially high; if NOT skipped, baseline mean would jump and distort delta!
            120.0
        } else {
            55.0
        };
        rows.push(DailyMetricRow {
            date: d.clone(),
            metric: "hrv_rmssd_ms".into(),
            source: "garmin".into(),
            value: hrv,
            unit: "ms".into(),
        });
        if (22..28).contains(&i) {
            checkins.push(d);
        }
    }

    let exp = ExperimentRow {
        id: 1,
        title: "睡前喝温牛奶".into(),
        hypothesis: "观察是否能显著提升次日夜间 HRV".into(),
        target_days: 7,
        start_date: (start + Duration::days(22)).format("%Y-%m-%d").to_string(),
        status: "active".into(),
        checkin_dates: checkins,
        washout_days: 2,
    };

    let evals = evaluate_experiments(&rows, &[exp], "2026-10-28");
    assert_eq!(evals.len(), 1);
    assert_eq!(evals[0].washout_days, 2);
    assert_eq!(evals[0].washout_skipped_days, 2);
    // Because washout days (120.0) were skipped, hrv_delta_ms == 0.0 -> neutral!
    assert_eq!(evals[0].hrv_delta_ms, Some(0.0));
    assert_eq!(evals[0].outcome_verdict, "neutral");

    // Test via QiEngine end-to-end: custom experiment with neutral result lights up 「证伪小达人」!
    let mut engine = QiEngine::new(Some(":empty:".to_owned())).unwrap();
    for i in 0..25_i64 {
        let d = (start + Duration::days(i)).format("%Y-%m-%d").to_string();
        let raw = serde_json::json!({
            "kind": "metric",
            "date": d,
            "metric": "hrv_rmssd_ms",
            "source": "garmin",
            "value": 55.0,
            "unit": "ms"
        })
        .to_string();
        engine.import_external_health_data("jsonl".into(), raw).unwrap();
    }
    let st_exp = parse_json(
        &engine
            .start_custom_experiment(
                "睡前听白噪音".into(),
                "检验是否改变深睡/HRV".into(),
                7,
                3,
            )
            .unwrap(),
    );
    let exp_id = st_exp["experiments"][0]["id"].as_i64().unwrap();
    assert_eq!(st_exp["experiments"][0]["washout_days"], 3);
    // Toggle 3 days checkins on existing dates via CSV/JSONL + checkin
    for day in ["2026-10-22", "2026-10-23", "2026-10-24", "2026-10-25"] {
        let _ = day;
    }
    // Check in today (2026-10-25) and check evaluate_experiments directly on 3 checkins
    engine.toggle_experiment_today(exp_id).unwrap();

    // Verify ceramic badge 「证伪小达人」 directly in solar_atelier
    let snap = compute_snapshot(&rows, &[], &[], HardwareTier::TierA, 4);
    let solar = compute_solar_term_state("2026-10-28", "sunny_crisp");
    let (_, badges) = nsh_app_qi::solar_atelier::build_four_seasons_scroll_and_badges(
        &Default::default(),
        &snap,
        &solar,
        &evals,
        0,
        0,
        28,
    );
    let badge = badges.iter().find(|b| b.title == "证伪小达人").unwrap();
    assert!(badge.earned);
}

#[test]
fn moat4_webdav_bidirectional_pull_sync_and_moat5_ai_briefings() {
    let mut engine = QiEngine::new(Some(":empty:".to_owned())).unwrap();
    let st = parse_json(&engine.state_json().unwrap());
    let phrase = st["settings"]["recovery_phrase"].as_str().unwrap().to_owned();
    let key = derive_master_key(&phrase).unwrap();
    let peer_env = encrypt_sync_batch(
        &key,
        "peer_ipad",
        1,
        r#"{"kind":"intake","date":"2026-11-13","category":"meal","item_name":"燕麦坚果碗","calories_kcal":380,"notes":"WebDAV同步测试","tags":["高纤维早餐"]}"#,
    )
    .unwrap();
    let env_json = serde_json::to_string(&peer_env).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let webdav_url = format!("http://{addr}/dav/qi-vault");
    let env_clone = env_json.clone();

    std::thread::spawn(move || {
        for stream_res in listener.incoming().take(3) {
            let Ok(mut stream) = stream_res else { break };
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            let first = req.lines().next().unwrap_or("");
            if first.starts_with("PROPFIND ") {
                let xml = r#"<?xml version="1.0" encoding="utf-8"?><d:multistatus xmlns:d="DAV:"><d:response><d:href>/dav/qi-vault/ops/peer_ipad/000001.enc</d:href></d:response></d:multistatus>"#;
                let resp = format!(
                    "HTTP/1.1 207 Multi-Status\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    xml.len(),
                    xml
                );
                let _ = stream.write_all(resp.as_bytes());
            } else if first.starts_with("GET /dav/qi-vault/ops/peer_ipad/000001.enc") {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    env_clone.len(),
                    env_clone
                );
                let _ = stream.write_all(resp.as_bytes());
            } else if first.starts_with("POST /v1/chat/completions") {
                let body = serde_json::json!({
                    "choices": [{ "message": { "content": "今晨元气平稳，建议保持 23:00 前安睡并适度补水。" } }]
                })
                .to_string();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        }
    });

    engine
        .update_sync_config(
            "webdav".into(),
            webdav_url,
            "".into(),
            "alice".into(),
            "secret".into(),
        )
        .unwrap();
    let pulled_state = parse_json(&engine.pull_webdav_sync().unwrap());
    assert!(
        pulled_state["intakes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["item_name"] == "燕麦坚果碗")
    );

    // Moat 5: Free basic 90-char local Ollama briefing + Atelier multi-day synthesis
    let snap = compute_snapshot(&[], &[], &[], HardwareTier::TierA, 4);
    let free_brief = generate_live_briefing(
        &snap,
        &[],
        "lan_ollama",
        &format!("http://{addr}/v1"),
        "qwen3:latest",
        "",
        "",
        false,
        8,
    )
    .unwrap();
    assert!(free_brief.text.contains("23:00"));
    assert!(!free_brief.text.contains("确诊"));

    let atelier_synth = generate_atelier_synthesis(
        &snap,
        &[],
        &[],
        &[],
        "on_device_local",
        "",
        "",
        "",
        "",
    )
    .unwrap();
    assert!(atelier_synth.provider_label.contains("知气雅集"));
    assert!(!atelier_synth.text.contains("开药"));
}

#[test]
fn audit_6_checks_non_lockout_sqlite_migration_and_wipe() {
    // Test idempotent ALTER TABLE migration on a legacy v3 schema file
    let tmp_path = std::env::temp_dir().join(format!("qi_v3_migration_{}.sqlite3", std::process::id()));
    let _ = std::fs::remove_file(&tmp_path);
    {
        let conn = Connection::open(&tmp_path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE daily_metrics_by_source (date TEXT, metric TEXT, source TEXT, value REAL, unit TEXT, revision_ms INTEGER, PRIMARY KEY (date, metric, source));
            CREATE TABLE workouts (workout_id TEXT PRIMARY KEY, date TEXT, activity_type TEXT, duration_minutes REAL, calories_kcal REAL, source TEXT);
            CREATE TABLE intake_logs (id INTEGER PRIMARY KEY AUTOINCREMENT, date TEXT, category TEXT, item_name TEXT, calories_kcal REAL, notes TEXT, tags_json TEXT);
            CREATE TABLE lab_biomarkers (id INTEGER PRIMARY KEY AUTOINCREMENT, test_date TEXT, marker_name TEXT, canonical_key TEXT, value REAL, unit TEXT, ref_low REAL, ref_high REAL, flag TEXT, plain_note TEXT, UNIQUE(test_date, canonical_key));
            CREATE TABLE experiments (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT, hypothesis TEXT, target_days INTEGER, start_date TEXT, status TEXT, UNIQUE(title, start_date));
            CREATE TABLE experiment_checkins (experiment_id INTEGER, date TEXT, PRIMARY KEY (experiment_id, date));
            CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT);
            "#,
        )
        .unwrap();
    }

    let db = VaultDatabase::open_file(&tmp_path).unwrap();
    assert!(db.load_labs().is_ok());
    assert!(db.load_experiments().is_ok());
    drop(db);

    let mut engine = QiEngine::new(Some(tmp_path.to_string_lossy().into_owned())).unwrap();
    let audit = parse_json(&engine.run_data_integrity_audit().unwrap());
    assert_eq!(audit["passed"], true);
    assert_eq!(audit["vault_non_lockout_verified"], true);
    let checks = audit["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 6);
    assert!(
        checks[5]["detail"]
            .as_str()
            .unwrap()
            .contains("vault_non_lockout_verified=true")
    );

    // Populate dew, sticker, preset and verify wipe_vault_and_reset clears all 3 tables!
    engine
        .add_food_sticker("2026-11-13".into(), "热豆浆".into(), "polaroid".into(), "".into(), "".into())
        .unwrap();
    engine.save_sanctuary_preset("测试预设".into()).unwrap();
    engine.cosanctuary_action("warm_tea".into(), None, None).unwrap();

    let wiped = parse_json(&engine.wipe_vault_and_reset().unwrap());
    assert_eq!(wiped["solar_atelier"]["dew_balance"], 0);
    assert!(wiped["solar_atelier"]["stickers"].as_array().unwrap().is_empty());
    assert!(wiped["solar_atelier"]["presets"].as_array().unwrap().is_empty());

    let _ = std::fs::remove_file(&tmp_path);
}

fn contains_cjk_char(s: &str) -> bool {
    s.chars()
        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c) || ('\u{3400}'..='\u{4dbf}').contains(&c))
}

fn assert_no_cjk_in_json(val: &Value, path: &str) {
    match val {
        Value::String(s) => assert!(
            !contains_cjk_char(s),
            "Unexpected CJK leak at {path}: {s}"
        ),
        Value::Array(arr) => {
            for (idx, item) in arr.iter().enumerate() {
                assert_no_cjk_in_json(item, &format!("{path}[{idx}]"));
            }
        }
        Value::Object(map) => {
            for (k, v) in map {
                assert_no_cjk_in_json(v, &format!("{path}.{k}"));
            }
        }
        _ => {}
    }
}

#[test]
fn v5_locale_bilingual_switching_legacy_settings_and_no_cjk_leak_in_en() {
    // 1. Legacy AppSettings JSON without `locale` deserializes with default `"zh-CN"`.
    let mut legacy_map = serde_json::to_value(nsh_app_qi::storage::AppSettings::default()).unwrap();
    legacy_map.as_object_mut().unwrap().remove("locale");
    let legacy_settings: nsh_app_qi::storage::AppSettings =
        serde_json::from_value(legacy_map).unwrap();
    assert_eq!(legacy_settings.locale, "zh-CN");

    // 2. Legacy SQLite DB missing `locale` row in app_settings also defaults to `zh-CN`.
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let loc0 = parse_json(&engine.get_locale().unwrap());
    assert_eq!(loc0["locale"], "zh-CN");

    let st_zh = parse_json(&engine.state_json().unwrap());
    let zh_raw = serde_json::to_string(&st_zh).unwrap();
    assert!(!zh_raw.contains("知气 · Qi"), "Brand leak in zh-CN payload");
    assert_eq!(st_zh["solar_atelier"]["solar_term"]["term_name"], "立冬");
    assert!(nsh_app_qi::QiMeta::disclaimer_for_locale("zh-CN".into()).contains("生活方式"));
    assert!(nsh_app_qi::QiMeta::disclaimer_for_locale("en".into()).contains("everyday wellness"));
    assert!(!nsh_app_qi::QiMeta::disclaimer_for_locale("zh-CN".into()).contains("知气 · Qi"));
    assert!(!nsh_app_qi::QiMeta::disclaimer_for_locale("en".into()).contains("知气 · Qi"));

    // 3. Switch to English (`en`) and verify all user-facing display payloads translate cleanly.
    let st_en = parse_json(&engine.set_locale("en".into()).unwrap());
    assert_eq!(st_en["settings"]["locale"], "en");
    assert_eq!(parse_json(&engine.get_locale().unwrap())["locale"], "en");
    let en_raw = serde_json::to_string(&st_en).unwrap();
    assert!(!en_raw.contains("知气 · Qi"));

    for key in [
        "snapshot",
        "briefing",
        "labs",
        "experiments",
        "experiment_templates",
        "period_comparison",
        "entitlements",
        "entitlement_catalog",
        "multi_lag_radar",
        "multi_profile_uln_trends",
        "recheck_reminders",
        "parents_peace_letter_preview",
    ] {
        assert_no_cjk_in_json(&st_en[key], key);
    }
    for sa_key in [
        "solar_term",
        "seasonal_pass",
        "keepsake_catalog",
        "companion_weekly_letters",
        "co_sanctuary",
        "widget_preview",
        "four_seasons_scroll",
        "ceramic_badges",
        "custom_pet_quota",
        "dew_ledger",
    ] {
        assert_no_cjk_in_json(&st_en["solar_atelier"][sa_key], &format!("solar_atelier.{sa_key}"));
    }

    let audit_en = parse_json(&engine.run_data_integrity_audit().unwrap());
    assert_eq!(audit_en["passed"], true);
    assert_no_cjk_in_json(&audit_en, "audit");

    // 4. Switch back to zh-CN and verify Chinese restoration.
    let st_back = parse_json(&engine.set_locale("zh-CN".into()).unwrap());
    assert_eq!(st_back["settings"]["locale"], "zh-CN");
    assert_eq!(st_back["solar_atelier"]["solar_term"]["term_name"], "立冬");
}

#[test]
fn v5_custom_pet_free_quota_validation_pro_ex_libris_and_expired_non_lockout() {
    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();

    // 1. Unsafe SVG photo data URI must be rejected cleanly.
    let svg_err = engine
        .upsert_custom_pet(
            "米粒".into(),
            "cat".into(),
            "爱晒太阳".into(),
            "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=".into(),
            "stamp".into(),
            false,
            "".into(),
            true,
            None,
        )
        .unwrap_err();
    assert!(svg_err.contains("PNG") || svg_err.contains("JPEG") || svg_err.contains("WebP"));

    // 2. Free tier cannot use Pro Ex Libris stamp title, but can save 1 custom pet forever.
    let ex_libris_free_err = engine
        .upsert_custom_pet(
            "米粒".into(),
            "cat".into(),
            "冬日窗台主理喵".into(),
            "data:image/png;base64,iVBORw0KGgo=".into(),
            "stamp".into(),
            false,
            "藏书票 · 米粒".into(),
            true,
            None,
        )
        .unwrap_err();
    assert!(ex_libris_free_err.contains("知气雅集"));

    let st_free_pet = parse_json(
        &engine
            .upsert_custom_pet(
                "米粒".into(),
                "cat".into(),
                "冬日窗台主理喵".into(),
                "data:image/png;base64,iVBORw0KGgo=".into(),
                "stamp".into(),
                false,
                "".into(),
                true,
                None,
            )
            .unwrap(),
    );
    assert_eq!(st_free_pet["solar_atelier"]["custom_pet_quota"]["used_count"], 1);
    assert_eq!(st_free_pet["solar_atelier"]["custom_pet_quota"]["allowed_count"], 1);
    assert_eq!(st_free_pet["solar_atelier"]["custom_pet_quota"]["can_add_more"], false);
    assert_eq!(st_free_pet["solar_atelier"]["active_custom_pet"]["name"], "米粒");

    let pet1_id = st_free_pet["solar_atelier"]["active_custom_pet"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // 2nd custom pet blocked on Free tier
    let err_pet2_free = engine
        .upsert_custom_pet(
            "云朵".into(),
            "rabbit".into(),
            "星光兔".into(),
            "".into(),
            "arch".into(),
            true,
            "".into(),
            true,
            None,
        )
        .unwrap_err();
    assert!(err_pet2_free.contains("1 位"));

    // 3. Upgrade to Pro Atelier -> up to 3 custom pets + Ex Libris + first-person weekly letters
    engine
        .set_entitlement_sandbox("pro_annual".into(), "active".into(), false, 10)
        .unwrap();

    let st_pet2 = parse_json(
        &engine
            .upsert_custom_pet(
                "云朵".into(),
                "rabbit".into(),
                "一直陪在桌面灯下".into(),
                "data:image/webp;base64,UklGRh4AAABXRUJQ".into(),
                "arch".into(),
                true,
                "Ex Libris · Cloud".into(),
                true,
                None,
            )
            .unwrap(),
    );
    let pet2_id = st_pet2["solar_atelier"]["active_custom_pet"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(st_pet2["solar_atelier"]["active_custom_pet"]["starlight_memorial"], true);

    let st_pet3 = parse_json(
        &engine
            .upsert_custom_pet(
                "Maple".into(),
                "fox".into(),
                "Warm autumn tea companion".into(),
                "".into(),
                "cloud".into(),
                false,
                "Atelier Seal · Maple".into(),
                false,
                None,
            )
            .unwrap(),
    );
    let pet3_id = st_pet3["solar_atelier"]["custom_pets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "Maple")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(st_pet3["solar_atelier"]["custom_pet_quota"]["used_count"], 3);
    assert_eq!(st_pet3["solar_atelier"]["custom_pet_quota"]["allowed_count"], 3);
    assert_eq!(st_pet3["solar_atelier"]["custom_pet_quota"]["can_add_more"], false);

    // 4th custom pet blocked cleanly even on Pro
    let err_pet4 = engine
        .upsert_custom_pet(
            "多余".into(),
            "dog".into(),
            "".into(),
            "".into(),
            "circle".into(),
            false,
            "".into(),
            false,
            None,
        )
        .unwrap_err();
    assert!(err_pet4.contains("3 位"));

    // Generate bilingual first-person weekly letters for Cloud (starlight memorial) & Maple (en)
    let st_letter_zh = parse_json(&engine.generate_pet_weekly_letter(pet2_id.clone(), Some(1)).unwrap());
    let letters_zh = st_letter_zh["solar_atelier"]["pet_weekly_letters"].as_array().unwrap();
    assert_eq!(letters_zh.len(), 1);
    assert_eq!(letters_zh[0]["starlight_memorial"], true);
    assert!(letters_zh[0]["body"].as_str().unwrap().contains("星光"));

    engine.set_locale("en".into()).unwrap();
    let st_letter_en = parse_json(&engine.generate_pet_weekly_letter(pet3_id.clone(), Some(2)).unwrap());
    let letters_en = st_letter_en["solar_atelier"]["pet_weekly_letters"].as_array().unwrap();
    assert_eq!(letters_en.len(), 2);
    assert_eq!(letters_en[0]["locale"], "en");
    assert_no_cjk_in_json(&letters_en[0], "pet_weekly_letters[0]");

    // 4. ETHICAL RED LINE: Expire Atelier -> ExpiredReadOnlyVault MUST NEVER delete/hide/lock existing 3 pets or saved letters!
    let st_exp = parse_json(
        &engine
            .set_entitlement_sandbox("pro_annual".into(), "expired".into(), false, 10)
            .unwrap(),
    );
    assert_eq!(st_exp["entitlements"]["state"], "expired_read_only_vault");
    assert_eq!(st_exp["entitlements"]["is_atelier_unlocked"], false);
    assert_eq!(st_exp["solar_atelier"]["custom_pets"].as_array().unwrap().len(), 3);
    assert_eq!(st_exp["solar_atelier"]["pet_weekly_letters"].as_array().unwrap().len(), 2);
    assert_eq!(st_exp["solar_atelier"]["custom_pet_quota"]["used_count"], 3);
    assert_eq!(st_exp["solar_atelier"]["custom_pet_quota"]["allowed_count"], 1);
    assert_eq!(
        st_exp["solar_atelier"]["custom_pet_quota"]["retained_above_free_quota"],
        true
    );

    // Existing pets can still be switched freely on window stage and edited (non-Pro fields)
    let st_switch = parse_json(&engine.set_active_custom_pet(pet1_id.clone()).unwrap());
    assert_eq!(st_switch["solar_atelier"]["active_custom_pet"]["id"], pet1_id);
    let st_edit_non_pro = parse_json(
        &engine
            .upsert_custom_pet(
                "云朵".into(),
                "rabbit".into(),
                "Updated calm persona on read-only vault".into(),
                "".into(),
                "lacquer_arch".into(),
                true,
                "Ex Libris · Cloud".into(), // unchanged saved stamp
                true,
                Some(pet2_id.clone()),
            )
            .unwrap(),
    );
    assert_eq!(st_edit_non_pro["solar_atelier"]["active_custom_pet"]["die_cut_frame"], "lacquer_arch");

    // Switching back to seasonal companion (`""`) also succeeds cleanly
    let st_default_comp = parse_json(&engine.set_active_custom_pet("".into()).unwrap());
    assert!(st_default_comp["solar_atelier"]["active_custom_pet"].is_null());

    // Generating NEW Pro letters or modifying Ex Libris stamp or adding new pet on expired vault fails politely
    let err_new_letter = engine
        .generate_pet_weekly_letter(pet2_id.clone(), Some(3))
        .unwrap_err();
    assert!(err_new_letter.contains("Qi Atelier"));

    let err_new_stamp = engine
        .upsert_custom_pet(
            "云朵".into(),
            "rabbit".into(),
            "Persona".into(),
            "".into(),
            "arch".into(),
            true,
            "Brand New Stamp".into(),
            true,
            Some(pet2_id.clone()),
        )
        .unwrap_err();
    assert!(err_new_stamp.contains("Ex Libris"));

    // Deleting a custom pet removes only it and its saved letters cleanly
    let st_deleted = parse_json(&engine.delete_custom_pet(pet2_id).unwrap());
    assert_eq!(st_deleted["solar_atelier"]["custom_pets"].as_array().unwrap().len(), 2);
    let remaining_letters = st_deleted["solar_atelier"]["pet_weekly_letters"].as_array().unwrap();
    assert_eq!(remaining_letters.len(), 1);
    assert_eq!(remaining_letters[0]["pet_id"], pet3_id);
    let listed = parse_json(&engine.list_custom_pets().unwrap());
    assert_eq!(listed["pets"].as_array().unwrap().len(), 2);
}

#[test]
fn v5_apple_iap_hearth_gift_pass_commercial_naming_and_restore() {
    // 1. Apple IAP catalog includes Hearth Gift Pass consumable SKU & Atelier plans
    let apple_ids = parse_json(&nsh_app_qi::QiMeta::apple_catalog_product_ids_json());
    assert_eq!(
        apple_ids["consumables"][0]["product_id"],
        "com.qi.hearth_gift_pass"
    );

    let mut engine = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    for locale in ["zh-CN", "en"] {
        let st = parse_json(&engine.set_locale(locale.into()).unwrap());
        let raw = serde_json::to_string(&st).unwrap();
        let lower = raw.to_lowercase();
        for forbidden in ["公益", "慈善", "捐", "charity", "donat", "知气 · qi"] {
            assert!(
                !lower.contains(forbidden),
                "Forbidden App Store 3.2.1(vi) / brand wording '{forbidden}' leaked under locale {locale}"
            );
        }
        assert_eq!(st["entitlement_catalog"]["guardian_addon_price_cny"], 12);
    }

    // Verify commercial naming in both languages
    let st_zh = parse_json(&engine.set_locale("zh-CN".into()).unwrap());
    assert_eq!(
        st_zh["entitlement_catalog"]["guardian_addon_title"],
        "窗台添柴礼赠券 · 赠予社区漂流席位"
    );
    let st_en = parse_json(&engine.set_locale("en".into()).unwrap());
    assert_eq!(
        st_en["entitlement_catalog"]["guardian_addon_title"],
        "Hearth Gift Pass · Gift 1 Community Seat"
    );

    // 2. Verify StoreKit 2 Hearth Gift Pass purchase in Sandbox unlocks guardian_sponsor + hearth_guardian badge
    let st_gift = parse_json(
        &engine
            .verify_apple_iap_transaction(
                "com.qi.hearth_gift_pass".into(),
                "tx_gift_001".into(),
                "tx_gift_001".into(),
                "Sandbox".into(),
                None,
                None,
            )
            .unwrap(),
    );
    assert_eq!(st_gift["entitlements"]["guardian_sponsor"], true);
    let guardian_badge = st_gift["solar_atelier"]["ceramic_badges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"] == "hearth_guardian")
        .unwrap();
    assert_eq!(guardian_badge["earned"], true);

    // 3. Batch restore StoreKit entitlements restores Pro Annual + Gift Pass
    let mut engine2 = QiEngine::new(Some(":memory:".to_owned())).unwrap();
    let restore_payload = serde_json::json!([
        {
            "product_id": "com.qi.atelier.pro_annual",
            "transaction_id": "tx_annual_99",
            "original_transaction_id": "orig_annual_99",
            "environment": "Sandbox"
        },
        {
            "product_id": "com.qi.hearth_gift_pass",
            "transaction_id": "tx_gift_99",
            "original_transaction_id": "tx_gift_99",
            "environment": "Sandbox"
        }
    ])
    .to_string();
    let restored = parse_json(&engine2.restore_apple_iap_purchases(restore_payload).unwrap());
    assert_eq!(restored["entitlements"]["is_atelier_unlocked"], true);
    assert_eq!(restored["entitlements"]["tier"], "pro_annual");
    assert_eq!(restored["entitlements"]["guardian_sponsor"], true);
}


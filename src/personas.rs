//! 6 套立体纵向健康测试画像与确定性播种器（最新基准日期固定为 2026-11-13）。

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::analytics::DailyMetricRow;
use crate::safety_ai::ParsedLabMarkerDraft;
use crate::storage::VaultDatabase;

const ANCHOR_DATE: &str = "2026-11-13";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaMeta {
    pub id: String,
    pub name: String,
    pub badge: String,
    pub hardware_tier: String,
    pub primary_goal: String,
    pub days_count: usize,
    pub summary: String,
    pub highlight_metrics: Vec<String>,
}

pub fn all_personas_meta() -> Vec<PersonaMeta> {
    vec![
        PersonaMeta {
            id: "urban_overwork".to_owned(),
            name: "林砚 · 31岁互联网高压熬夜族".to_owned(),
            badge: "高压熬夜 · 急需回血".to_owned(),
            hardware_tier: "tier_a".to_owned(),
            primary_goal: "energy_sleep".to_owned(),
            days_count: 45,
            summary: "工作日午后依赖冰美式、偶发 22:30 夜宵烧烤，昨晚熬夜睡眠偏短进入调养预警；支持 Garmin RMSSD 与 Apple Health SDNN 双源隔离对比。".to_owned(),
            highlight_metrics: vec![
                "双源 HRV 隔离".to_owned(),
                "熬夜恢复预警".to_owned(),
                "D+1 负向归因".to_owned(),
                "维D/尿酸/ALT 建档".to_owned(),
            ],
        },
        PersonaMeta {
            id: "endurance_runner".to_owned(),
            name: "周野 · 28岁马拉松备赛跑者".to_owned(),
            badge: "备赛高负荷 · ACWR 警戒".to_owned(),
            hardware_tier: "tier_a".to_owned(),
            primary_goal: "smart_training".to_owned(),
            days_count: 50,
            summary: "全马备赛冲刺期，静息心率 46-49 bpm、HRV 68-75 ms；近 7 天跑量激增使 ACWR 升至 1.41 过载警戒带，含 Garmin 与 Apple Watch 同日双源去重。".to_owned(),
            highlight_metrics: vec![
                "RHR 46-49 bpm".to_owned(),
                "HRV 68-75 ms".to_owned(),
                "ACWR 1.41 警戒".to_owned(),
                "双源并发去重".to_owned(),
            ],
        },
        PersonaMeta {
            id: "metabolic_care".to_owned(),
            name: "陈默 · 42岁控糖减脂与血脂管理".to_owned(),
            badge: "作息规律 · 指标持续向好".to_owned(),
            hardware_tier: "tier_a".to_owned(),
            primary_goal: "lab_archive".to_owned(),
            days_count: 60,
            summary: "作息规律达充沛状态 (≥82 分)，完整归档 3 期纵向体检报告（LDL-C、TG、HbA1c、尿酸、ALT、GLU 全面向好），配合 14 天显效的早晚餐实验。".to_owned(),
            highlight_metrics: vec![
                "元气 ≥82 充沛".to_owned(),
                "3期体检全改善".to_owned(),
                "14天实验已显效".to_owned(),
                "控糖低脂食谱".to_owned(),
            ],
        },
        PersonaMeta {
            id: "band_commuter".to_owned(),
            name: "苏晴 · 26岁轻量化手环通勤族".to_owned(),
            badge: "基础手环 · 诚实降级".to_owned(),
            hardware_tier: "tier_b".to_owned(),
            primary_goal: "diet_balance".to_owned(),
            days_count: 40,
            summary: "仅佩戴小米手环记录睡眠时长、静息心率与步数，库内 0 条 HRV 与睡眠分期，严格验证 Tier B 诚实降级与奶茶轻食、铁蛋白偏低管理。".to_owned(),
            highlight_metrics: vec![
                "0 条 HRV 伪造".to_owned(),
                "小米手环 Tier B".to_owned(),
                "守中稳行状态".to_owned(),
                "铁蛋白偏低追踪".to_owned(),
            ],
        },
        PersonaMeta {
            id: "phone_minimalist".to_owned(),
            name: "顾伯 · 55岁零穿戴纯手机长辈".to_owned(),
            badge: "零穿戴 · 慢病档案".to_owned(),
            hardware_tier: "tier_c".to_owned(),
            primary_goal: "lab_archive".to_owned(),
            days_count: 30,
            summary: "无任何手表手环，仅依赖手机 CoreMotion 步数（5500-8800 步）与晨起体感打分运行，重点承载跨期慢病随访化验单与清淡饮食日志。".to_owned(),
            highlight_metrics: vec![
                "纯手机计步".to_owned(),
                "体感 4/5 算分".to_owned(),
                "6 项慢病随访".to_owned(),
                "零手表心率".to_owned(),
            ],
        },
        PersonaMeta {
            id: "sparse_chaotic".to_owned(),
            name: "极端断戴与边界压力测试集".to_owned(),
            badge: "压力测试 · 9天断戴".to_owned(),
            hardware_tier: "tier_a".to_owned(),
            primary_goal: "energy_sleep".to_owned(),
            days_count: 45,
            summary: "45 天跨度内含连续 9 天完全断戴空白、跨月日期、同日多源冲突、38500 步极值、195min 极短睡眠、0 打卡实验及全角字符化验单，验证零 NaN/越界/崩溃。".to_owned(),
            highlight_metrics: vec![
                "9天断戴空白".to_owned(),
                "38500 步极值".to_owned(),
                "195min 极短睡".to_owned(),
                "全角OCR脱敏".to_owned(),
            ],
        },
    ]
}

pub fn find_persona_meta(persona_id: &str) -> Option<PersonaMeta> {
    all_personas_meta().into_iter().find(|p| p.id == persona_id)
}

pub fn seed_persona_data(db: &VaultDatabase, persona_id: &str) -> Result<PersonaMeta, String> {
    let meta = find_persona_meta(persona_id)
        .ok_or_else(|| format!("未知测试画像 ID: {persona_id}"))?;
    match persona_id {
        "urban_overwork" => seed_urban_overwork(db)?,
        "endurance_runner" => seed_endurance_runner(db)?,
        "metabolic_care" => seed_metabolic_care(db)?,
        "band_commuter" => seed_band_commuter(db)?,
        "phone_minimalist" => seed_phone_minimalist(db)?,
        "sparse_chaotic" => seed_sparse_chaotic(db)?,
        _ => return Err(format!("未知测试画像 ID: {persona_id}")),
    }
    Ok(meta)
}

fn anchor_naive_date() -> NaiveDate {
    NaiveDate::parse_from_str(ANCHOR_DATE, "%Y-%m-%d").expect("valid anchor date")
}

fn date_str(start: NaiveDate, offset: i64) -> String {
    (start + Duration::days(offset)).format("%Y-%m-%d").to_string()
}

fn put_metric(
    db: &VaultDatabase,
    date: &str,
    metric: &str,
    source: &str,
    value: f64,
    unit: &str,
) -> Result<(), String> {
    db.upsert_daily_metric(&DailyMetricRow {
        date: date.to_owned(),
        metric: metric.to_owned(),
        source: source.to_owned(),
        value,
        unit: unit.to_owned(),
    })
}

/// 1) urban_overwork：林砚 · 31岁互联网高压熬夜族（45天，2026-09-30 ~ 2026-11-13）
fn seed_urban_overwork(db: &VaultDatabase) -> Result<(), String> {
    let end = anchor_naive_date();
    let start = end - Duration::days(44); // 45 days inclusive

    let coffee_days: [i64; 5] = [8, 16, 25, 34, 41];
    let bbq_days: [i64; 4] = [12, 21, 31, 43];

    for i in 0..45_i64 {
        let d = date_str(start, i);
        let prev = i - 1;
        let after_coffee = coffee_days.contains(&prev);
        let after_bbq = bbq_days.contains(&prev);
        let is_last_night = i == 44;

        let rmssd = if is_last_night {
            37.0
        } else if after_bbq {
            40.5
        } else if after_coffee {
            44.0
        } else {
            55.0 + ((i * 7) % 6) as f64
        };
        let sdnn = 43.0 + ((i * 5) % 5) as f64;
        let rhr = if is_last_night {
            64.0
        } else if after_bbq {
            61.5
        } else if after_coffee {
            59.0
        } else {
            53.0 + (i % 3) as f64
        };
        let sleep_min = if is_last_night {
            320.0
        } else if after_bbq || after_coffee {
            372.0
        } else {
            442.0 + ((i * 3) % 24) as f64
        };
        let deep_min = if is_last_night {
            46.0
        } else if after_bbq {
            54.0
        } else if after_coffee {
            64.0
        } else {
            88.0 + (i % 14) as f64
        };
        let rem_min = if is_last_night {
            55.0
        } else if after_bbq || after_coffee {
            68.0
        } else {
            90.0 + (i % 12) as f64
        };
        let steps = 7000.0 + ((i * 650) % 4500) as f64;
        let azm = if i % 2 == 0 { 36.0 } else { 18.0 };

        for (metric, source, val, unit) in [
            ("hrv_rmssd_ms", "garmin", rmssd, "ms"),
            ("hrv_sdnn_ms", "apple_health", sdnn, "ms"),
            ("resting_heart_rate_bpm", "garmin", rhr, "bpm"),
            ("sleep_minutes", "garmin", sleep_min, "min"),
            ("sleep_deep_minutes", "garmin", deep_min, "min"),
            ("sleep_rem_minutes", "garmin", rem_min, "min"),
            ("steps", "apple_health", steps, "步"),
            ("active_zone_minutes", "garmin", azm, "min"),
        ] {
            put_metric(db, &d, metric, source, val, unit)?;
        }

        if i % 3 == 0 {
            db.insert_workout(&format!("uo-w-{i}"), &d, "Zone 2 慢跑", 42.0, Some(340.0), "garmin")?;
        }
        if coffee_days.contains(&i) {
            db.insert_intake(
                &d,
                "caffeine",
                "15:00后冰美式",
                Some(15.0),
                "下午加班提神",
                &["15:00后冰美式"],
            )?;
        }
        if bbq_days.contains(&i) {
            db.insert_intake(
                &d,
                "meal",
                "22:30夜宵烧烤",
                Some(780.0),
                "深夜聚餐高油高盐",
                &["22:30夜宵烧烤"],
            )?;
        }
    }

    // 2 期维 D 纵向趋势（2026-08-12 18.4 -> 2026-10-20 26.5 improved）+ 尿酸/ALT 偏高 + 血糖
    db.save_parsed_lab(
        "2026-08-12",
        &ParsedLabMarkerDraft {
            marker_name: "25-羟维生素D".into(),
            canonical_key: "vitamin_d".into(),
            value: 18.4,
            unit: "ng/mL".into(),
            ref_low: Some(30.0),
            ref_high: Some(100.0),
            flag: "low".into(),
            plain_note: "维生素 D 偏低：建议每日上午适度户外日照 15 分钟。".into(),
        },
    )?;
    db.save_parsed_lab(
        "2026-08-12",
        &ParsedLabMarkerDraft {
            marker_name: "尿酸 (UA)".into(),
            canonical_key: "uric_acid".into(),
            value: 482.0,
            unit: "umol/L".into(),
            ref_low: Some(208.0),
            ref_high: Some(428.0),
            flag: "high".into(),
            plain_note: "偏高于参考上限：建议每日饮水 ≥2000ml，减少啤酒、浓肉汤与夜宵烧烤。".into(),
        },
    )?;
    db.save_parsed_lab(
        "2026-08-12",
        &ParsedLabMarkerDraft {
            marker_name: "谷丙转氨酶 (ALT)".into(),
            canonical_key: "alt".into(),
            value: 64.0,
            unit: "U/L".into(),
            ref_low: Some(9.0),
            ref_high: Some(50.0),
            flag: "high".into(),
            plain_note: "转氨酶高于参考上限：可能与近期熬夜与高脂夜宵相关，建议减少酒精并按期复查。".into(),
        },
    )?;

    db.save_parsed_lab(
        "2026-10-20",
        &ParsedLabMarkerDraft {
            marker_name: "25-羟维生素D".into(),
            canonical_key: "vitamin_d".into(),
            value: 26.5,
            unit: "ng/mL".into(),
            ref_low: Some(30.0),
            ref_high: Some(100.0),
            flag: "low".into(),
            plain_note: "维生素 D 较上期显著回升，继续每日日照 15 分钟并随餐补充。".into(),
        },
    )?;
    db.save_parsed_lab(
        "2026-10-20",
        &ParsedLabMarkerDraft {
            marker_name: "尿酸 (UA)".into(),
            canonical_key: "uric_acid".into(),
            value: 458.0,
            unit: "umol/L".into(),
            ref_low: Some(208.0),
            ref_high: Some(428.0),
            flag: "high".into(),
            plain_note: "较上期略降但仍高于参考上限，减少夜间烧烤与含糖饮料。".into(),
        },
    )?;
    db.save_parsed_lab(
        "2026-10-20",
        &ParsedLabMarkerDraft {
            marker_name: "谷丙转氨酶 (ALT)".into(),
            canonical_key: "alt".into(),
            value: 55.0,
            unit: "U/L".into(),
            ref_low: Some(9.0),
            ref_high: Some(50.0),
            flag: "high".into(),
            plain_note: "仍略偏高于参考上限，优先保证 23:30 前入睡并减少夜宵。".into(),
        },
    )?;
    db.save_parsed_lab(
        "2026-10-20",
        &ParsedLabMarkerDraft {
            marker_name: "空腹血糖 (GLU)".into(),
            canonical_key: "fasting_glucose".into(),
            value: 4.95,
            unit: "mmol/L".into(),
            ref_low: Some(3.9),
            ref_high: Some(6.1),
            flag: "normal".into(),
            plain_note: "位于常规参考区间内，继续保持良好作息与饮食平衡即可。".into(),
        },
    )?;

    let exp_id = db.create_experiment_if_missing(
        "15:00 后停摄入咖啡因",
        "减少下午半衰期咖啡因残留，观察当晚深睡时长与次日夜间 HRV",
        7,
        "2026-11-06",
    )?;
    for offset in 0..4_i64 {
        let d = date_str(NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(), offset);
        db.insert_experiment_checkin(exp_id, &d)?;
    }
    Ok(())
}

/// 2) endurance_runner：周野 · 28岁马拉松备赛跑者（50天，2026-09-25 ~ 2026-11-13）
fn seed_endurance_runner(db: &VaultDatabase) -> Result<(), String> {
    let end = anchor_naive_date();
    let start = end - Duration::days(49); // 50 days inclusive

    for i in 0..50_i64 {
        let d = date_str(start, i);
        let is_surge_week = i >= 43; // 近 7 天跑量激增（i=43..49）
        let rmssd = if is_surge_week {
            68.0 + (i % 3) as f64
        } else {
            70.0 + ((i * 3) % 6) as f64
        };
        let sdnn = 55.0 + (i % 6) as f64;
        let rhr = 46.0 + (i % 4) as f64;
        let sleep_min = 452.0 + ((i * 5) % 28) as f64;
        let deep_min = 96.0 + (i % 14) as f64;
        let rem_min = 100.0 + (i % 14) as f64;
        let steps = if is_surge_week {
            19500.0 + ((i * 700) % 4000) as f64
        } else {
            13500.0 + ((i * 600) % 3800) as f64
        };
        // acute (7d) = 65.0; chronic (28d: 21*40 + 7*65)/28 = 46.25 => ACWR = 65.0 / 46.25 = 1.41
        let azm = if is_surge_week { 65.0 } else { 40.0 };

        // 主源 Garmin
        for (metric, val, unit) in [
            ("hrv_rmssd_ms", rmssd, "ms"),
            ("resting_heart_rate_bpm", rhr, "bpm"),
            ("sleep_minutes", sleep_min, "min"),
            ("sleep_deep_minutes", deep_min, "min"),
            ("sleep_rem_minutes", rem_min, "min"),
            ("steps", steps, "步"),
            ("active_zone_minutes", azm, "min"),
        ] {
            put_metric(db, &d, metric, "garmin", val, unit)?;
        }

        // 同日 Apple Watch / Apple Health 并发写入（验证优先级去重与 SDNN 隔离）
        for (metric, val, unit) in [
            ("hrv_sdnn_ms", sdnn, "ms"),
            ("resting_heart_rate_bpm", rhr + 5.0, "bpm"),
            ("sleep_minutes", sleep_min - 25.0, "min"),
            ("steps", steps - 1800.0, "步"),
            ("active_zone_minutes", 22.0, "min"),
        ] {
            put_metric(db, &d, metric, "apple_health", val, unit)?;
        }

        if is_surge_week {
            db.insert_workout(
                &format!("er-w-{i}"),
                &d,
                "马拉松长距离/节奏跑",
                68.0 + ((i - 43) * 4) as f64,
                Some(780.0),
                "garmin",
            )?;
        } else if i % 2 == 0 {
            db.insert_workout(&format!("er-w-{i}"), &d, "Zone 2 有氧耐力跑", 45.0, Some(480.0), "garmin")?;
        }

        if i % 4 == 0 {
            db.insert_intake(
                &d,
                "meal",
                "高蛋白三文鱼藜麦恢复餐",
                Some(640.0),
                "跑后 30 分钟内补充碳水与优质蛋白",
                &["高蛋白饮食"],
            )?;
        }
        if i % 5 == 0 {
            db.insert_intake(
                &d,
                "supplement",
                "乳清蛋白奶昔配香蕉",
                Some(280.0),
                "训练后肌糖原与氨基酸补给",
                &["跑后蛋白补给"],
            )?;
        }
    }

    db.save_parsed_lab(
        "2026-08-25",
        &ParsedLabMarkerDraft {
            marker_name: "铁蛋白 (Ferritin)".into(),
            canonical_key: "ferritin".into(),
            value: 34.0,
            unit: "ng/mL".into(),
            ref_low: Some(30.0),
            ref_high: Some(400.0),
            flag: "normal".into(),
            plain_note: "处于耐力跑者参考区间下沿，建议持续关注瘦红肉与维 C 搭配。".into(),
        },
    )?;
    db.save_parsed_lab(
        "2026-08-25",
        &ParsedLabMarkerDraft {
            marker_name: "血红蛋白 (Hb)".into(),
            canonical_key: "hemoglobin".into(),
            value: 145.0,
            unit: "g/L".into(),
            ref_low: Some(130.0),
            ref_high: Some(175.0),
            flag: "normal".into(),
            plain_note: "位于常规参考区间内，携氧能力稳定。".into(),
        },
    )?;
    db.save_parsed_lab(
        "2026-11-05",
        &ParsedLabMarkerDraft {
            marker_name: "铁蛋白 (Ferritin)".into(),
            canonical_key: "ferritin".into(),
            value: 46.5,
            unit: "ng/mL".into(),
            ref_low: Some(30.0),
            ref_high: Some(400.0),
            flag: "normal".into(),
            plain_note: "铁储备稳步回升，耐力训练期维持现状即可。".into(),
        },
    )?;
    db.save_parsed_lab(
        "2026-11-05",
        &ParsedLabMarkerDraft {
            marker_name: "血红蛋白 (Hb)".into(),
            canonical_key: "hemoglobin".into(),
            value: 150.0,
            unit: "g/L".into(),
            ref_low: Some(130.0),
            ref_high: Some(175.0),
            flag: "normal".into(),
            plain_note: "位于常规参考区间内，氧运输指标良好。".into(),
        },
    )?;
    db.save_parsed_lab(
        "2026-11-05",
        &ParsedLabMarkerDraft {
            marker_name: "肌酸激酶 (CK)".into(),
            canonical_key: "ck".into(),
            value: 348.0,
            unit: "U/L".into(),
            ref_low: Some(50.0),
            ref_high: Some(310.0),
            flag: "high".into(),
            plain_note: "高跑量冲刺后肌酸激酶轻度高于参考上限，提示肌肉微损伤处于修复窗口，宜安排减量日。".into(),
        },
    )?;

    let exp_id = db.create_experiment_if_missing(
        "跑后补充甘氨酸镁",
        "睡前补充 300mg 甘氨酸镁，观察大跑量周期夜间深睡与 HRV 恢复效率",
        14,
        "2026-11-04",
    )?;
    for offset in 0..8_i64 {
        let d = date_str(NaiveDate::from_ymd_opt(2026, 11, 4).unwrap(), offset);
        db.insert_experiment_checkin(exp_id, &d)?;
    }
    Ok(())
}

/// 3) metabolic_care：陈默 · 42岁控糖减脂与血脂管理（60天，2026-09-15 ~ 2026-11-13）
fn seed_metabolic_care(db: &VaultDatabase) -> Result<(), String> {
    let end = anchor_naive_date();
    let start = end - Duration::days(59); // 60 days inclusive

    for i in 0..60_i64 {
        let d = date_str(start, i);
        let in_exp_phase = i >= 45; // 2026-10-30 起执行「晚餐提前至 19:30 前」

        let rmssd = if in_exp_phase {
            63.0 + (i % 4) as f64
        } else {
            54.0 + (i % 4) as f64
        };
        let sdnn = if in_exp_phase {
            52.0 + (i % 3) as f64
        } else {
            46.0 + (i % 3) as f64
        };
        let rhr = if in_exp_phase {
            51.0 + (i % 2) as f64
        } else {
            56.0 + (i % 3) as f64
        };
        let sleep_min = if in_exp_phase {
            466.0 + (i % 10) as f64
        } else {
            442.0 + (i % 12) as f64
        };
        let deep_min = if in_exp_phase {
            104.0 + (i % 8) as f64
        } else {
            86.0 + (i % 7) as f64
        };
        let rem_min = if in_exp_phase {
            106.0 + (i % 8) as f64
        } else {
            90.0 + (i % 7) as f64
        };
        let steps = 8800.0 + ((i * 370) % 2600) as f64;
        let azm = 34.0;

        for (metric, source, val, unit) in [
            ("hrv_rmssd_ms", "garmin", rmssd, "ms"),
            ("hrv_sdnn_ms", "apple_health", sdnn, "ms"),
            ("resting_heart_rate_bpm", "garmin", rhr, "bpm"),
            ("sleep_minutes", "garmin", sleep_min, "min"),
            ("sleep_deep_minutes", "garmin", deep_min, "min"),
            ("sleep_rem_minutes", "garmin", rem_min, "min"),
            ("steps", "apple_health", steps, "步"),
            ("active_zone_minutes", "garmin", azm, "min"),
        ] {
            put_metric(db, &d, metric, source, val, unit)?;
        }

        if i % 2 == 0 {
            db.insert_workout(&format!("mc-w-{i}"), &d, "餐后快走与功率车", 35.0, Some(240.0), "garmin")?;
        }
        if in_exp_phase && i % 3 == 0 {
            db.insert_intake(
                &d,
                "meal",
                "杂粮饭配清蒸鲈鱼时蔬",
                Some(510.0),
                "19:00 前完成晚餐，先菜后肉再主食",
                &["19:30前晚餐"],
            )?;
        } else if i % 6 == 0 {
            db.insert_intake(
                &d,
                "meal",
                "无糖豆浆配水煮蛋燕麦",
                Some(380.0),
                "低 GI 控糖早餐",
                &["控糖高纤饮食"],
            )?;
        }
    }

    // 3 期纵向体检报告：2026-05-15 -> 2026-08-18 -> 2026-11-10
    let lab_series: Vec<(&str, &str, &str, f64, &str, Option<f64>, Option<f64>, &str, &str)> = vec![
        // LDL-C: 4.12 -> 3.68 -> 3.18
        ("2026-05-15", "低密度脂蛋白 (LDL-C)", "ldl_c", 4.12, "mmol/L", Some(0.0), Some(3.37), "high", "偏高于参考上限，建议控油控糖并增加有氧。"),
        ("2026-08-18", "低密度脂蛋白 (LDL-C)", "ldl_c", 3.68, "mmol/L", Some(0.0), Some(3.37), "high", "较上期下降 0.44 mmol/L，继续坚持膳食纤维摄入。"),
        ("2026-11-10", "低密度脂蛋白 (LDL-C)", "ldl_c", 3.18, "mmol/L", Some(0.0), Some(3.37), "normal", "已回落至常规参考区间内，控制效果稳健。"),
        // TG: 2.65 -> 1.92 -> 1.48
        ("2026-05-15", "甘油三酯 (TG)", "triglycerides", 2.65, "mmol/L", Some(0.0), Some(1.70), "high", "甘油三酯偏高：建议控制精制碳水与晚间油脂。"),
        ("2026-08-18", "甘油三酯 (TG)", "triglycerides", 1.92, "mmol/L", Some(0.0), Some(1.70), "high", "较上期明显回落，继续减少精制主食。"),
        ("2026-11-10", "甘油三酯 (TG)", "triglycerides", 1.48, "mmol/L", Some(0.0), Some(1.70), "normal", "已恢复至参考区间内。"),
        // HbA1c: 6.3 -> 5.9 -> 5.6
        ("2026-05-15", "糖化血红蛋白 (HbA1c)", "hba1c", 6.3, "%", Some(4.0), Some(6.0), "high", "糖代谢指标提示关注，推荐先菜后肉再主食与餐后快走。"),
        ("2026-08-18", "糖化血红蛋白 (HbA1c)", "hba1c", 5.9, "%", Some(4.0), Some(6.0), "normal", "回落至正常区间上沿，继续保持控糖节奏。"),
        ("2026-11-10", "糖化血红蛋白 (HbA1c)", "hba1c", 5.6, "%", Some(4.0), Some(6.0), "normal", "连续三期稳步下降，长期血糖波动控制理想。"),
        // UA: 488 -> 442 -> 398
        ("2026-05-15", "尿酸 (UA)", "uric_acid", 488.0, "umol/L", Some(208.0), Some(428.0), "high", "偏高于参考上限，建议每日饮水 ≥2000ml。"),
        ("2026-08-18", "尿酸 (UA)", "uric_acid", 442.0, "umol/L", Some(208.0), Some(428.0), "high", "较上期下降 46 umol/L，持续保持足量饮水。"),
        ("2026-11-10", "尿酸 (UA)", "uric_acid", 398.0, "umol/L", Some(208.0), Some(428.0), "normal", "已稳定回落至正常参考区间。"),
        // ALT: 62 -> 38
        ("2026-05-15", "谷丙转氨酶 (ALT)", "alt", 62.0, "U/L", Some(9.0), Some(50.0), "high", "高于参考上限，建议清淡饮食并保证规律睡眠。"),
        ("2026-11-10", "谷丙转氨酶 (ALT)", "alt", 38.0, "U/L", Some(9.0), Some(50.0), "normal", "随体脂与作息改善已完全恢复至正常区间。"),
        // GLU: 6.45 -> 5.82 -> 5.26
        ("2026-05-15", "空腹血糖 (GLU)", "fasting_glucose", 6.45, "mmol/L", Some(3.9), Some(6.1), "high", "略高于常规区间，建议提前晚餐时间并配合餐后步行。"),
        ("2026-08-18", "空腹血糖 (GLU)", "fasting_glucose", 5.82, "mmol/L", Some(3.9), Some(6.1), "normal", "回落至参考区间内。"),
        ("2026-11-10", "空腹血糖 (GLU)", "fasting_glucose", 5.26, "mmol/L", Some(3.9), Some(6.1), "normal", "连续三期稳步下降，空腹血糖平稳健康。"),
    ];

    for (tdate, mname, ckey, val, unit, lo, hi, flag, note) in lab_series {
        db.save_parsed_lab(
            tdate,
            &ParsedLabMarkerDraft {
                marker_name: mname.into(),
                canonical_key: ckey.into(),
                value: val,
                unit: unit.into(),
                ref_low: lo,
                ref_high: hi,
                flag: flag.into(),
                plain_note: note.into(),
            },
        )?;
    }

    let exp_id = db.create_experiment_if_missing(
        "晚餐提前至 19:30 前",
        "避开睡前胃肠高负荷代谢，观察夜间静息心率与 HRV 改善",
        14,
        "2026-10-30",
    )?;
    for offset in 0..14_i64 {
        let d = date_str(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap(), offset);
        db.insert_experiment_checkin(exp_id, &d)?;
    }
    Ok(())
}

/// 4) band_commuter：苏晴 · 26岁轻量化手环通勤族（40天，2026-10-05 ~ 2026-11-13）
fn seed_band_commuter(db: &VaultDatabase) -> Result<(), String> {
    let end = anchor_naive_date();
    let start = end - Duration::days(39); // 40 days inclusive

    for i in 0..40_i64 {
        let d = date_str(start, i);
        // 严格只写入 xiaomi 手环指标：0 条 HRV，0 条睡眠分期
        let sleep_min = if i == 39 {
            400.0
        } else if i >= 33 {
            395.0
        } else {
            422.0 + (i % 16) as f64
        };
        let rhr = 62.0 + (i % 4) as f64;
        let steps = 7800.0 + ((i * 550) % 4200) as f64;
        let cal = 260.0 + ((i * 35) % 140) as f64;

        for (metric, val, unit) in [
            ("sleep_minutes", sleep_min, "min"),
            ("resting_heart_rate_bpm", rhr, "bpm"),
            ("steps", steps, "步"),
            ("calories_active_kcal", cal, "kcal"),
        ] {
            put_metric(db, &d, metric, "xiaomi", val, unit)?;
        }

        if i % 6 == 1 {
            db.insert_intake(
                &d,
                "drink",
                "三分糖燕麦奶茶",
                Some(290.0),
                "下午通勤茶饮",
                &["下午奶茶"],
            )?;
        }
        if i % 5 == 2 {
            db.insert_intake(
                &d,
                "meal",
                "牛油果鸡胸贝果轻食",
                Some(430.0),
                "工作日午餐轻食",
                &["轻食午餐"],
            )?;
        }
    }

    db.save_parsed_lab(
        "2026-07-15",
        &ParsedLabMarkerDraft {
            marker_name: "铁蛋白 (Ferritin)".into(),
            canonical_key: "ferritin".into(),
            value: 14.2,
            unit: "ng/mL".into(),
            ref_low: Some(30.0),
            ref_high: Some(150.0),
            flag: "low".into(),
            plain_note: "铁蛋白偏低，易导致通勤午后犯困，建议增加瘦红肉与维 C 饮食。".into(),
        },
    )?;
    db.save_parsed_lab(
        "2026-10-28",
        &ParsedLabMarkerDraft {
            marker_name: "铁蛋白 (Ferritin)".into(),
            canonical_key: "ferritin".into(),
            value: 18.5,
            unit: "ng/mL".into(),
            ref_low: Some(30.0),
            ref_high: Some(150.0),
            flag: "low".into(),
            plain_note: "较上期略有回升但仍低于参考下限，建议餐食少搭浓茶咖啡并继续补充富铁食物。".into(),
        },
    )?;
    db.save_parsed_lab(
        "2026-10-28",
        &ParsedLabMarkerDraft {
            marker_name: "血红蛋白 (Hb)".into(),
            canonical_key: "hemoglobin".into(),
            value: 112.0,
            unit: "g/L".into(),
            ref_low: Some(115.0),
            ref_high: Some(150.0),
            flag: "low".into(),
            plain_note: "处于参考下限边缘，建议结合铁蛋白一起调理营养节奏。".into(),
        },
    )?;

    Ok(())
}

/// 5) phone_minimalist：顾伯 · 55岁零穿戴纯手机长辈（30天，2026-10-15 ~ 2026-11-13）
fn seed_phone_minimalist(db: &VaultDatabase) -> Result<(), String> {
    let end = anchor_naive_date();
    let start = end - Duration::days(29); // 30 days inclusive

    for i in 0..30_i64 {
        let d = date_str(start, i);
        // 严格仅 phone_coremotion 步数（5500-8800），无手表心率与睡眠
        let steps = 5500.0 + ((i * 310) % 3301) as f64;
        put_metric(db, &d, "steps", "phone_coremotion", steps, "步")?;

        if i % 6 == 0 {
            db.insert_intake(
                &d,
                "meal",
                "小米南瓜粥配水煮蛋",
                Some(320.0),
                "清淡温和早餐",
                &["清淡饮食"],
            )?;
        } else if i % 6 == 3 {
            db.insert_intake(
                &d,
                "meal",
                "清蒸鳕鱼拌木耳杂粮饭",
                Some(460.0),
                "低盐少油家常晚餐",
                &["清淡饮食"],
            )?;
        }
    }

    let elder_labs: Vec<(&str, &str, &str, f64, &str, Option<f64>, Option<f64>, &str, &str)> = vec![
        ("2026-06-20", "空腹血糖 (GLU)", "fasting_glucose", 6.8, "mmol/L", Some(3.9), Some(6.1), "high", "空腹血糖偏高，推荐散步与低升糖主食搭配。"),
        ("2026-09-18", "空腹血糖 (GLU)", "fasting_glucose", 6.3, "mmol/L", Some(3.9), Some(6.1), "high", "较上期稳步回落 0.5 mmol/L。"),
        ("2026-11-08", "空腹血糖 (GLU)", "fasting_glucose", 5.9, "mmol/L", Some(3.9), Some(6.1), "normal", "已回落至参考区间内，保持餐后散步习惯。"),

        ("2026-06-20", "糖化血红蛋白 (HbA1c)", "hba1c", 6.6, "%", Some(4.0), Some(6.0), "high", "提示近三个月平均血糖偏高，留意主食分量。"),
        ("2026-09-18", "糖化血红蛋白 (HbA1c)", "hba1c", 6.2, "%", Some(4.0), Some(6.0), "high", "较上期下降 0.4%，控糖轨迹清晰。"),
        ("2026-11-08", "糖化血红蛋白 (HbA1c)", "hba1c", 5.9, "%", Some(4.0), Some(6.0), "normal", "已进入正常参考区间，继续保持清淡规律三餐。"),

        ("2026-06-20", "低密度脂蛋白 (LDL-C)", "ldl_c", 3.75, "mmol/L", Some(0.0), Some(3.37), "high", "建议减少动物内脏与油炸，适当增加深海鱼。"),
        ("2026-09-18", "低密度脂蛋白 (LDL-C)", "ldl_c", 3.42, "mmol/L", Some(0.0), Some(3.37), "high", "接近参考上限，继续少油清淡饮食。"),
        ("2026-11-08", "低密度脂蛋白 (LDL-C)", "ldl_c", 3.05, "mmol/L", Some(0.0), Some(3.37), "normal", "已顺利回落至参考区间。"),

        ("2026-06-20", "血肌酐 (Cr)", "creatinine", 86.0, "umol/L", Some(57.0), Some(111.0), "normal", "肾功能基础指标平稳，注意日常足量饮水。"),
        ("2026-11-08", "血肌酐 (Cr)", "creatinine", 82.0, "umol/L", Some(57.0), Some(111.0), "normal", "指标平稳在参考区间中段。"),

        ("2026-06-20", "促甲状腺激素 (TSH)", "tsh", 4.65, "mIU/L", Some(0.27), Some(4.2), "high", "略高于常规上限，复查随访并留意精力变化。"),
        ("2026-11-08", "促甲状腺激素 (TSH)", "tsh", 3.82, "mIU/L", Some(0.27), Some(4.2), "normal", "复查已回落至参考区间内。"),

        ("2026-06-20", "尿酸 (UA)", "uric_acid", 446.0, "umol/L", Some(208.0), Some(428.0), "high", "略高于上限，日常少喝老火浓汤并多饮温水。"),
        ("2026-11-08", "尿酸 (UA)", "uric_acid", 410.0, "umol/L", Some(208.0), Some(428.0), "normal", "已回落至正常区间。"),
    ];

    for (tdate, mname, ckey, val, unit, lo, hi, flag, note) in elder_labs {
        db.save_parsed_lab(
            tdate,
            &ParsedLabMarkerDraft {
                marker_name: mname.into(),
                canonical_key: ckey.into(),
                value: val,
                unit: unit.into(),
                ref_low: lo,
                ref_high: hi,
                flag: flag.into(),
                plain_note: note.into(),
            },
        )?;
    }

    Ok(())
}

/// 6) sparse_chaotic：极端断戴与边界压力测试集（45天跨度 2026-09-30 ~ 2026-11-13）
fn seed_sparse_chaotic(db: &VaultDatabase) -> Result<(), String> {
    let end = anchor_naive_date();
    let start = end - Duration::days(44); // 45 days span

    for i in 0..45_i64 {
        // 中间连续 9 天完全断戴空白（i = 18..27 => 2026-10-18 ~ 2026-10-26）
        if (18..27).contains(&i) {
            continue;
        }
        let d = date_str(start, i);

        // 单日步数极值 38500（i = 33 => 2026-11-02）与 睡眠极低值 195min（i = 37 => 2026-11-06）
        let steps = if i == 33 {
            38500.0
        } else {
            4200.0 + ((i * 730) % 5800) as f64
        };
        let sleep_min = if i == 37 {
            195.0
        } else if i == 44 {
            340.0
        } else {
            390.0 + ((i * 11) % 60) as f64
        };
        let deep_min = if i == 37 {
            28.0
        } else if i == 44 {
            54.0
        } else {
            72.0 + (i % 15) as f64
        };
        let rem_min = if i == 37 {
            24.0
        } else if i == 44 {
            58.0
        } else {
            76.0 + (i % 14) as f64
        };
        let rmssd = if i == 44 { 41.0 } else { 49.0 + (i % 8) as f64 };
        let rhr = if i == 44 { 62.0 } else { 56.0 + (i % 5) as f64 };
        let azm = if i == 33 { 165.0 } else { 24.0 };

        for (metric, source, val, unit) in [
            ("hrv_rmssd_ms", "garmin", rmssd, "ms"),
            ("hrv_sdnn_ms", "apple_health", 42.0 + (i % 4) as f64, "ms"),
            ("resting_heart_rate_bpm", "garmin", rhr, "bpm"),
            ("sleep_minutes", "garmin", sleep_min, "min"),
            ("sleep_deep_minutes", "garmin", deep_min, "min"),
            ("sleep_rem_minutes", "garmin", rem_min, "min"),
            ("steps", "garmin", steps, "步"),
            ("active_zone_minutes", "garmin", azm, "min"),
        ] {
            put_metric(db, &d, metric, source, val, unit)?;
        }

        // 跨月与末日同日多源冲突写入（apple_health / xiaomi / phone_coremotion）
        if i == 31 || i == 32 || i == 44 {
            put_metric(db, &d, "steps", "apple_health", steps + 1111.0, "步")?;
            put_metric(db, &d, "steps", "xiaomi", steps - 999.0, "步")?;
            put_metric(db, &d, "steps", "phone_coremotion", steps - 2100.0, "步")?;
            put_metric(db, &d, "resting_heart_rate_bpm", "apple_health", rhr + 7.0, "bpm")?;
            put_metric(db, &d, "sleep_minutes", "xiaomi", sleep_min + 45.0, "min")?;
        }
    }

    db.insert_workout("sc-w-extreme", &date_str(start, 33), "越野百公里拉练", 180.0, Some(1850.0), "garmin")?;

    // 0 打卡实验
    db.create_experiment_if_missing(
        "睡前冥想呼吸 10 分钟",
        "验证断戴恢复后的副交感唤醒回稳",
        7,
        "2026-11-10",
    )?;

    // 含全角字符与 PII 混排文本，走端侧真实脱敏与解析管线入库
    let raw_fullwidth_lab = "某某省立中心医院检验科\n\
        姓名：测试样本 身份证号：31010119900101123X 手机：13800138000\n\
        谷丙转氨酶（ALT）：68 U/L 9–50\n\
        尿酸（UA）：492 umol/L 208–428\n\
        25-羟维生素D：22.0 ng/mL 30.0–100.0";
    db.import_lab_text("2026-11-09", raw_fullwidth_lab)?;

    Ok(())
}

//! iOS 原生 Apple HealthKit（`HealthKit.framework`）静态链接桥接与生理边界校验模块。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::analytics::DailyMetricRow;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthKitMetricItem {
    pub date: String,
    pub metric: String,
    #[serde(default)]
    pub source: String,
    pub value: f64,
    #[serde(default)]
    pub unit: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthKitWorkoutItem {
    pub workout_id: String,
    pub date: String,
    pub activity_type: String,
    pub duration_minutes: f64,
    #[serde(default)]
    pub calories_kcal: Option<f64>,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthKitBridgeResponse {
    pub ok: bool,
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub authorized: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub metrics: Vec<HealthKitMetricItem>,
    #[serde(default)]
    pub workouts: Vec<HealthKitWorkoutItem>,
}

#[derive(Debug, Clone)]
pub struct ValidatedHealthKitBatch {
    pub authorized: bool,
    pub metrics: Vec<DailyMetricRow>,
    pub workouts: Vec<HealthKitWorkoutItem>,
    pub distinct_days: usize,
    pub recommended_tier: &'static str,
}

#[cfg(target_os = "ios")]
mod ffi {
    use std::ffi::{CStr, c_char};

    unsafe extern "C" {
        fn qi_hk_is_available() -> i32;
        fn qi_hk_request_and_sync_json(days: i32) -> *mut c_char;
        fn qi_hk_free_string(ptr: *mut c_char);
    }

    pub fn is_available() -> bool {
        // SAFETY: `qi_hk_is_available` takes no pointers and checks HKHealthStore availability.
        unsafe { qi_hk_is_available() != 0 }
    }

    pub fn request_and_query_json(days: u32) -> Result<String, String> {
        let clamped = days.clamp(1, 180) as i32;
        // SAFETY: `qi_hk_request_and_sync_json` allocates a UTF-8 C string freed via `qi_hk_free_string`.
        let ptr = unsafe { qi_hk_request_and_sync_json(clamped) };
        if ptr.is_null() {
            return Err("Apple HealthKit 桥接返回空指针".to_owned());
        }
        // SAFETY: `ptr` is NUL-terminated UTF-8 from strdup until `qi_hk_free_string(ptr)`.
        let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
        // SAFETY: Releases the buffer allocated by `qi_hk_request_and_sync_json`.
        unsafe { qi_hk_free_string(ptr) };
        Ok(result)
    }
}

/// 检查当前运行时是否支持直接唤起原生 Apple HealthKit 授权与同步。
pub fn is_healthkit_available() -> bool {
    #[cfg(target_os = "ios")]
    {
        ffi::is_available()
    }
    #[cfg(not(target_os = "ios"))]
    {
        std::env::var("QI_MOCK_HEALTHKIT_JSON")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }
}

/// 发起 Apple HealthKit 授权请求（若尚未授权则系统弹窗）并抓取近 `days` 天生理与运动 JSON。
pub fn request_and_query_healthkit_json(days: u32) -> Result<String, String> {
    #[cfg(target_os = "ios")]
    {
        ffi::request_and_query_json(days)
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = days;
        if let Ok(mock) = std::env::var("QI_MOCK_HEALTHKIT_JSON")
            && !mock.trim().is_empty()
        {
            return Ok(mock);
        }
        Err("当前非 iOS 真机运行环境，请在 iPhone 上打开「知气」直接授权 Apple 健康，或使用下方的 XML/CSV 离线导入。".to_owned())
    }
}

fn is_valid_iso_date(date: &str) -> bool {
    date.len() == 10 && chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok()
}

fn normalize_metric_and_check_bounds(metric: &str, unit: &str, value: f64) -> Option<(&'static str, f64)> {
    if !value.is_finite() {
        return None;
    }
    let _ = unit;
    match metric {
        "sleep_minutes" => (1.0..=1440.0).contains(&value).then_some(("min", value)),
        "sleep_deep_minutes" | "sleep_rem_minutes" => {
            (0.1..=960.0).contains(&value).then_some(("min", value))
        }
        // Apple Watch 仅提供 SDNN，严守口径隔离，绝不映射为 hrv_rmssd_ms
        "hrv_sdnn_ms" => (1.0..=500.0).contains(&value).then_some(("ms", value)),
        "resting_heart_rate_bpm" => (25.0..=220.0).contains(&value).then_some(("bpm", value)),
        "steps" => (1.0..=200_000.0).contains(&value).then_some(("步", value)),
        "calories_active_kcal" => (1.0..=20_000.0).contains(&value).then_some(("kcal", value)),
        "active_zone_minutes" => (1.0..=1440.0).contains(&value).then_some(("min", value)),
        _ => None,
    }
}

/// 校验并归一化原生 HealthKit 桥接层返回的 JSON 批次：
/// - 强制写入 `source = "apple_health"`；
/// - 按 `(date, metric)` 去重取均值/最大值并做生理合理范围过滤；
/// - 绝不把 Apple Watch SDNN 写入 `hrv_rmssd_ms`；
/// - 自动根据具备的传感器维度推荐硬件档位（`tier_a` / `tier_b` / `tier_c`）。
pub fn parse_and_validate_healthkit_payload(raw_json: &str) -> Result<ValidatedHealthKitBatch, String> {
    let resp: HealthKitBridgeResponse =
        serde_json::from_str(raw_json).map_err(|e| format!("解析 HealthKit 响应失败: {e}"))?;

    if !resp.ok {
        return Err(resp
            .error
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Apple HealthKit 读取未完成".to_owned()));
    }

    let mut dedup_metrics: BTreeMap<(String, String), DailyMetricRow> = BTreeMap::new();
    let mut days_set: BTreeSet<String> = BTreeSet::new();

    for item in resp.metrics {
        let date = item.date.trim().to_owned();
        if !is_valid_iso_date(&date) {
            continue;
        }
        let Some((def_unit, clean_val)) =
            normalize_metric_and_check_bounds(item.metric.trim(), &item.unit, item.value)
        else {
            continue;
        };
        let unit = if item.unit.trim().is_empty() {
            def_unit.to_owned()
        } else {
            item.unit.trim().to_owned()
        };
        let rounded = (clean_val * 100.0).round() / 100.0;
        let metric_key = item.metric.trim().to_owned();
        days_set.insert(date.clone());
        dedup_metrics.insert(
            (date.clone(), metric_key.clone()),
            DailyMetricRow {
                date,
                metric: metric_key,
                source: "apple_health".to_owned(),
                value: rounded,
                unit,
            },
        );
    }

    let mut dedup_workouts: BTreeMap<String, HealthKitWorkoutItem> = BTreeMap::new();
    for w in resp.workouts {
        let wid = w.workout_id.trim().to_owned();
        let date = w.date.trim().to_owned();
        if wid.is_empty()
            || !is_valid_iso_date(&date)
            || !w.duration_minutes.is_finite()
            || !(0.5..=1440.0).contains(&w.duration_minutes)
        {
            continue;
        }
        let clean_kcal = w
            .calories_kcal
            .filter(|c| c.is_finite() && *c > 0.0 && *c <= 20_000.0)
            .map(|c| (c * 100.0).round() / 100.0);
        days_set.insert(date.clone());
        dedup_workouts.insert(
            wid.clone(),
            HealthKitWorkoutItem {
                workout_id: wid,
                date,
                activity_type: if w.activity_type.trim().is_empty() {
                    "体能训练".to_owned()
                } else {
                    w.activity_type.trim().to_owned()
                },
                duration_minutes: (w.duration_minutes * 100.0).round() / 100.0,
                calories_kcal: clean_kcal,
                source: "apple_health".to_owned(),
            },
        );
    }

    let metrics: Vec<DailyMetricRow> = dedup_metrics.into_values().collect();
    let workouts: Vec<HealthKitWorkoutItem> = dedup_workouts.into_values().collect();

    let has_sdnn = metrics.iter().any(|m| m.metric == "hrv_sdnn_ms");
    let has_stages = metrics
        .iter()
        .any(|m| m.metric == "sleep_deep_minutes" || m.metric == "sleep_rem_minutes");
    let has_sleep = metrics.iter().any(|m| m.metric == "sleep_minutes");
    let has_rhr = metrics.iter().any(|m| m.metric == "resting_heart_rate_bpm");

    let recommended_tier = if has_sdnn || has_stages {
        "tier_a"
    } else if has_sleep || has_rhr {
        "tier_b"
    } else {
        "tier_c"
    };

    Ok(ValidatedHealthKitBatch {
        authorized: resp.authorized,
        metrics,
        workouts,
        distinct_days: days_set.len(),
        recommended_tier,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_apple_health_payload_and_keeps_sdnn_isolated() {
        let json = serde_json::json!({
            "ok": true,
            "available": true,
            "authorized": true,
            "metrics": [
                { "date": "2026-11-14", "metric": "sleep_minutes", "source": "watch", "value": 442.5, "unit": "min" },
                { "date": "2026-11-14", "metric": "sleep_deep_minutes", "source": "watch", "value": 88.0, "unit": "min" },
                { "date": "2026-11-14", "metric": "hrv_sdnn_ms", "source": "watch", "value": 46.2, "unit": "ms" },
                { "date": "2026-11-14", "metric": "resting_heart_rate_bpm", "source": "watch", "value": 58.0, "unit": "bpm" },
                { "date": "2026-11-14", "metric": "steps", "source": "iphone", "value": 8420.0, "unit": "步" },
                // 应被拒绝写入的越界与非白名单指标：
                { "date": "2026-11-14", "metric": "sleep_minutes", "source": "bad", "value": 2400.0, "unit": "min" },
                { "date": "2026-11-14", "metric": "hrv_rmssd_ms", "source": "bad", "value": 55.0, "unit": "ms" }
            ],
            "workouts": [
                {
                    "workout_id": "HK-UUID-001",
                    "date": "2026-11-14",
                    "activity_type": "跑步",
                    "duration_minutes": 34.5,
                    "calories_kcal": 295.0,
                    "source": "watch"
                }
            ]
        })
        .to_string();

        let batch = parse_and_validate_healthkit_payload(&json).expect("should parse valid payload");
        assert_eq!(batch.recommended_tier, "tier_a");
        assert_eq!(batch.distinct_days, 1);
        assert_eq!(batch.workouts.len(), 1);
        assert_eq!(batch.workouts[0].source, "apple_health");
        assert!(batch.metrics.iter().all(|m| m.source == "apple_health"));
        assert!(!batch.metrics.iter().any(|m| m.metric == "hrv_rmssd_ms"));
        assert!(batch
            .metrics
            .iter()
            .any(|m| m.metric == "hrv_sdnn_ms" && (m.value - 46.2).abs() < 1e-6));
    }

    #[test]
    fn infers_tier_c_when_only_steps_are_present() {
        let json = serde_json::json!({
            "ok": true,
            "available": true,
            "authorized": true,
            "metrics": [
                { "date": "2026-11-14", "metric": "steps", "value": 6300.0 }
            ],
            "workouts": []
        })
        .to_string();

        let batch = parse_and_validate_healthkit_payload(&json).unwrap();
        assert_eq!(batch.recommended_tier, "tier_c");
        assert_eq!(batch.metrics[0].unit, "步");
        assert_eq!(batch.metrics[0].source, "apple_health");
    }
}

//! 确定性生理基线与归因引擎（移植并演进自 HealthAnalyticsEngine）。
//! 严格坚持「数学归代码，表达归模型」与 RMSSD/SDNN 生理口径物理隔离。

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareTier {
    /// 全功能穿戴（如 Apple Watch / Garmin / Oura，含 HRV 与睡眠分期）
    TierA,
    /// 基础手环（仅有睡眠时长、步数与活动时长，无连续 HRV）
    TierB,
    /// 零穿戴纯手机（手机步数 + 晨起主观体感分）
    TierC,
}

impl HardwareTier {
    pub fn from_str(s: &str) -> Self {
        match s {
            "tier_b" => Self::TierB,
            "tier_c" => Self::TierC,
            _ => Self::TierA,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TierA => "tier_a",
            Self::TierB => "tier_b",
            Self::TierC => "tier_c",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::TierA => "全功能穿戴（HRV + 睡眠分期）",
            Self::TierB => "基础手环（睡眠时长 + 步数）",
            Self::TierC => "零穿戴纯手机（步数 + 晨起体感）",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricPolarity {
    HigherBetter,
    LowerBetter,
    Neutral,
}

#[derive(Debug, Clone)]
pub struct DailyMetricRow {
    pub date: String,
    pub metric: String,
    pub source: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone)]
pub struct WorkoutRow {
    pub date: String,
    pub activity_type: String,
    pub duration_minutes: f64,
    pub calories_kcal: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntakeLogRow {
    pub id: i64,
    pub date: String,
    pub category: String,
    pub item_name: String,
    pub calories_kcal: Option<f64>,
    pub notes: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabBiomarkerRow {
    pub id: i64,
    pub test_date: String,
    pub marker_name: String,
    pub canonical_key: String,
    pub value: f64,
    pub unit: String,
    pub ref_low: Option<f64>,
    pub ref_high: Option<f64>,
    pub flag: String,
    pub plain_note: String,
    #[serde(default)]
    pub prev_value: Option<f64>,
    #[serde(default)]
    pub prev_date: Option<String>,
    #[serde(default)]
    pub delta: Option<f64>,
    #[serde(default)]
    pub trend: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSummary {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub value: Option<f64>,
    pub avg_7d: Option<f64>,
    pub baseline_30d_mean: Option<f64>,
    pub baseline_30d_std: Option<f64>,
    pub baseline_days: usize,
    pub z_score: Option<f64>,
    pub status: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    pub date: String,
    pub short_label: String,
    pub qi_score: u32,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaggedImpact {
    pub tag: String,
    pub outcome_label: String,
    pub event_days: usize,
    pub delta_value: f64,
    pub unit: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSnapshot {
    pub target_date: String,
    pub hardware_tier: String,
    pub hardware_tier_label: String,
    pub qi_score: u32,
    pub state: String,
    pub state_badge: String,
    pub headline: String,
    pub micro_action: String,
    pub signals: Vec<String>,
    pub sleep_debt_7d_hours: Option<f64>,
    pub sleep_last_night_hours: Option<f64>,
    pub acwr: Option<f64>,
    pub metrics: BTreeMap<String, MetricSummary>,
    pub trend_14d: Vec<TrendPoint>,
    pub lagged_impacts: Vec<LaggedImpact>,
}

fn metric_meta(key: &str) -> (&'static str, &'static str, MetricPolarity) {
    match key {
        "sleep_minutes" => ("昨晚总睡眠", "min", MetricPolarity::HigherBetter),
        "sleep_deep_minutes" => ("深睡时长", "min", MetricPolarity::HigherBetter),
        "sleep_rem_minutes" => ("REM 睡眠", "min", MetricPolarity::HigherBetter),
        "hrv_rmssd_ms" => ("夜间 HRV (RMSSD)", "ms", MetricPolarity::HigherBetter),
        "hrv_sdnn_ms" => ("腕表 HRV (SDNN)", "ms", MetricPolarity::HigherBetter),
        "resting_heart_rate_bpm" => ("静息心率 (RHR)", "bpm", MetricPolarity::LowerBetter),
        "steps" => ("今日步数", "步", MetricPolarity::HigherBetter),
        "active_zone_minutes" => ("中高强度负荷", "min", MetricPolarity::Neutral),
        "calories_active_kcal" => ("活动消耗", "kcal", MetricPolarity::Neutral),
        _ => ("生理指标", "", MetricPolarity::Neutral),
    }
}

fn source_priority(source: &str) -> usize {
    match source {
        "garmin" | "oura" => 1,
        "apple_health" | "watch_os" => 2,
        "google_health_v4" | "fitbit" => 3,
        "huawei_health" | "xiaomi" => 4,
        "phone_coremotion" => 5,
        _ => 9,
    }
}

/// 按来源优先级与日期消解权威指标映射，严禁 RMSSD 与 SDNN 合并。
pub fn resolve_canonical_metrics(
    rows: &[DailyMetricRow],
) -> BTreeMap<String, BTreeMap<String, DailyMetricRow>> {
    let mut resolved: BTreeMap<String, BTreeMap<String, DailyMetricRow>> = BTreeMap::new();
    for row in rows {
        let day_map = resolved.entry(row.date.clone()).or_default();
        match day_map.get(&row.metric) {
            Some(existing) if source_priority(&existing.source) <= source_priority(&row.source) => {}
            _ => {
                day_map.insert(row.metric.clone(), row.clone());
            }
        }
    }
    resolved
}

fn mean(vals: &[f64]) -> Option<f64> {
    if vals.is_empty() {
        None
    } else {
        Some(vals.iter().sum::<f64>() / vals.len() as f64)
    }
}

fn sample_std(vals: &[f64], m: f64) -> Option<f64> {
    if vals.len() < 4 {
        return None;
    }
    let var = vals.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (vals.len() - 1) as f64;
    Some(var.sqrt())
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn classify_z_status(z: Option<f64>, polarity: MetricPolarity, threshold: f64) -> &'static str {
    let Some(z_val) = z else {
        return "no_baseline";
    };
    match polarity {
        MetricPolarity::HigherBetter => {
            if z_val >= 0.8 {
                "optimal"
            } else if z_val <= -threshold {
                "alert_low"
            } else if z_val <= -0.8 {
                "watch_low"
            } else {
                "normal"
            }
        }
        MetricPolarity::LowerBetter => {
            if z_val <= -0.8 {
                "optimal"
            } else if z_val >= threshold {
                "alert_high"
            } else if z_val >= 0.8 {
                "watch_high"
            } else {
                "normal"
            }
        }
        MetricPolarity::Neutral => {
            if z_val.abs() >= threshold {
                "shifted"
            } else {
                "normal"
            }
        }
    }
}

fn compute_daily_load(
    canonical: &BTreeMap<String, BTreeMap<String, DailyMetricRow>>,
    workouts: &[WorkoutRow],
    start: NaiveDate,
    end: NaiveDate,
) -> f64 {
    let mut w_by_day: BTreeMap<String, f64> = BTreeMap::new();
    for w in workouts {
        if let Ok(d) = NaiveDate::parse_from_str(&w.date, "%Y-%m-%d")
            && d >= start
            && d <= end
        {
            *w_by_day.entry(w.date.clone()).or_default() += w.duration_minutes;
        }
    }
    let days = ((end - start).num_days() + 1).max(1) as f64;
    let mut total = 0.0;
    let mut cur = start;
    while cur <= end {
        let ds = cur.format("%Y-%m-%d").to_string();
        if let Some(day_map) = canonical.get(&ds) {
            if let Some(azm) = day_map.get("active_zone_minutes") {
                total += azm.value;
            } else if let Some(wd) = w_by_day.get(&ds) {
                total += *wd;
            } else if let Some(cal) = day_map.get("calories_active_kcal") {
                total += cal.value / 8.0;
            }
        } else if let Some(wd) = w_by_day.get(&ds) {
            total += *wd;
        }
        cur += Duration::days(1);
    }
    total / days
}

fn evaluate_day_qi(
    tier: HardwareTier,
    metrics: &BTreeMap<String, MetricSummary>,
    sleep_debt_min: Option<f64>,
    acwr: Option<f64>,
    morning_feeling: u32,
) -> (u32, &'static str, &'static str, String, String, Vec<String>) {
    let mut score: f64 = 78.0;
    let mut signals = Vec::new();

    match tier {
        HardwareTier::TierA => {
            // 严守物理隔离：优先使用夜间 RMSSD，若缺失则独立回退至 Apple Watch SDNN，绝不跨口径混合基线
            let hrv_primary = metrics
                .get("hrv_rmssd_ms")
                .filter(|m| m.value.is_some())
                .or_else(|| metrics.get("hrv_sdnn_ms").filter(|m| m.value.is_some()));

            if let Some(hrv) = hrv_primary
                && let Some(z) = hrv.z_score
            {
                score += (z * 11.0).clamp(-22.0, 14.0);
                if z <= -1.1 {
                    signals.push(format!(
                        "{} 低于 30 天个人基线（Z={z:+.2}），自主神经压力偏高",
                        hrv.label
                    ));
                } else if z >= 0.7 {
                    signals.push(format!(
                        "{} 高于个人基线（Z={z:+.2}），副交感恢复充沛",
                        hrv.label
                    ));
                }
            }

            if let Some(rhr) = metrics.get("resting_heart_rate_bpm")
                && let Some(z) = rhr.z_score
                && let Some(val) = rhr.value
            {
                score -= (z * 9.0).clamp(-10.0, 20.0);
                if z >= 1.1 {
                    signals.push(format!(
                        "静息心率升至 {val:.0} bpm（Z={z:+.2}），警惕晚间饮食/咖啡或疲劳累积"
                    ));
                } else if z <= -0.6 {
                    signals.push(format!("静息心率平稳回落至 {val:.0} bpm，心肺负荷轻松"));
                }
            }
        }
        HardwareTier::TierB => {
            // 基础手环：诚实评估，不伪造 HRV
            signals.push("当前按「睡眠时长 + 步数负荷」评估，未伪造 HRV 数据".to_owned());
        }
        HardwareTier::TierC => {
            // 纯手机模式：手机步数 + 晨起主观体感 (1..=5)
            let feel_clamped = morning_feeling.clamp(1, 5) as f64;
            score = 45.0 + feel_clamped * 10.0;
            signals.push(format!(
                "零穿戴纯手机模式：结合今日步数与晨起体感 ({morning_feeling}/5) 生成参考"
            ));
        }
    }

    if tier != HardwareTier::TierC
        && let Some(sleep) = metrics.get("sleep_minutes")
        && let Some(min_val) = sleep.value
    {
        let hrs = min_val / 60.0;
        if hrs < 6.0 {
            score -= 14.0;
            signals.push(format!("昨晚睡眠仅 {hrs:.1} 小时，深层修复窗口不足"));
        } else if hrs >= 7.5 {
            score += 5.0;
            signals.push(format!("昨晚睡足 {hrs:.1} 小时，睡眠质量在线"));
        }
    }

    if let Some(debt) = sleep_debt_min
        && debt > 180.0
    {
        let debt_h = debt / 60.0;
        score -= (debt_h * 2.5).clamp(0.0, 12.0);
        signals.push(format!("近 7 天睡眠蓄水池累计缺口 {debt_h:.1} 小时"));
    }

    if let Some(acwr_val) = acwr {
        if acwr_val > 1.5 {
            score -= 10.0;
            signals.push(format!(
                "近 7 天负荷陡增（ACWR={acwr_val:.2} > 1.5），注意防过度疲劳"
            ));
        } else if (0.8..=1.3).contains(&acwr_val) {
            score += 2.0;
        }
    }

    if signals.is_empty() {
        signals.push("核心生理指标与近 30 天基线吻合，节律平稳".to_owned());
    }

    let final_score = score.round().clamp(15.0, 99.0) as u32;
    let (state, badge, headline, micro_action) = if final_score >= 76 {
        (
            "prime_ready",
            "充沛 · 宜进",
            "气血与自主神经恢复充足，今天适合推进高强度训练或重要深度工作。",
            "安排 30–45 分钟高专注训练或核心攻坚任务，15:00 后控制咖啡因以锁住好状态。",
        )
    } else if final_score >= 56 {
        (
            "moderate_steady",
            "守中 · 稳行",
            "存在轻度恢复瓶颈，宜顺应节律稳健推进，避免盲目冲刺加量。",
            "将今日运动调为 Zone 2 慢跑或快走 30 分钟，晚间提前 20 分钟放下电子屏幕。",
        )
    } else {
        (
            "recovery_priority",
            "调养 · 蓄能",
            "多项生理指标明显偏离个人基线，身体发出休整信号，今日重在回血而非打卡。",
            "果断跳过高强度间歇训练，改做 15 分钟拉伸与腹式呼吸，今晚力争提前 45 分钟入睡。",
        )
    };

    (
        final_score,
        state,
        badge,
        headline.to_owned(),
        micro_action.to_owned(),
        signals,
    )
}

/// 归因：计算 D 日饮食/行为标签对 D+1 晨间恢复指标的滞后均值差异。
pub fn compute_lagged_impacts(
    canonical: &BTreeMap<String, BTreeMap<String, DailyMetricRow>>,
    intakes: &[IntakeLogRow],
) -> Vec<LaggedImpact> {
    let mut tag_dates: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for log in intakes {
        for tag in &log.tags {
            if !tag.trim().is_empty() {
                tag_dates
                    .entry(tag.trim().to_owned())
                    .or_default()
                    .insert(log.date.clone());
            }
        }
    }

    let outcome_specs = [
        ("hrv_rmssd_ms", "次日夜间 HRV", "ms", true),
        ("resting_heart_rate_bpm", "次日静息心率", "bpm", false),
        ("sleep_deep_minutes", "当晚深睡时长", "min", true),
    ];

    let all_days: Vec<&String> = canonical.keys().collect();
    let mut impacts = Vec::new();

    for (tag, dates) in tag_dates {
        if dates.is_empty() {
            continue;
        }
        let next_days: BTreeSet<String> = dates
            .iter()
            .filter_map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .map(|nd| (nd + Duration::days(1)).format("%Y-%m-%d").to_string())
            .collect();

        for (metric_key, outcome_label, unit, higher_good) in outcome_specs {
            let mut with_vals = Vec::new();
            let mut without_vals = Vec::new();
            for day in &all_days {
                if let Some(row) = canonical.get(*day).and_then(|m| m.get(metric_key)) {
                    if next_days.contains(*day) {
                        with_vals.push(row.value);
                    } else {
                        without_vals.push(row.value);
                    }
                }
            }
            if !with_vals.is_empty()
                && without_vals.len() >= 3
                && let (Some(mw), Some(mwo)) = (mean(&with_vals), mean(&without_vals))
            {
                let delta = round2(mw - mwo);
                if delta.abs() < 1.0 {
                    continue;
                }
                let direction = if delta > 0.0 {
                    format!("平均提升 {delta:+.1} {unit}")
                } else {
                    format!("平均波动 {delta:+.1} {unit}")
                };
                let hint = if (higher_good && delta < 0.0) || (!higher_good && delta > 0.0) {
                    "（提示恢复承压）"
                } else {
                    "（有助于次日恢复）"
                };
                impacts.push(LaggedImpact {
                    tag: tag.clone(),
                    outcome_label: outcome_label.to_owned(),
                    event_days: with_vals.len(),
                    delta_value: delta,
                    unit: unit.to_owned(),
                    summary: format!(
                        "记录「{tag}」{} 天：{}{direction}{hint}",
                        with_vals.len(),
                        outcome_label
                    ),
                });
            }
        }
    }

    impacts.sort_by(|a, b| b.event_days.cmp(&a.event_days));
    impacts.truncate(4);
    impacts
}

/// 基于已入库指标、运动与摄入日志，构建端侧确定性统计快照。
pub fn compute_snapshot(
    rows: &[DailyMetricRow],
    workouts: &[WorkoutRow],
    intakes: &[IntakeLogRow],
    tier: HardwareTier,
    morning_feeling: u32,
) -> AnalyticsSnapshot {
    let canonical = resolve_canonical_metrics(rows);
    let target_str = canonical
        .keys()
        .next_back()
        .cloned()
        .unwrap_or_else(|| "2026-11-13".to_owned());
    let target_date = NaiveDate::parse_from_str(&target_str, "%Y-%m-%d")
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2026, 11, 13).expect("valid date"));

    let summary_for_day = |d: NaiveDate| -> (
        BTreeMap<String, MetricSummary>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
    ) {
        let d_str = d.format("%Y-%m-%d").to_string();
        let short_start = (d - Duration::days(6)).format("%Y-%m-%d").to_string();
        let base_start = (d - Duration::days(30)).format("%Y-%m-%d").to_string();
        let base_end = (d - Duration::days(1)).format("%Y-%m-%d").to_string();

        let mut seen = BTreeSet::new();
        for day_map in canonical.values() {
            for k in day_map.keys() {
                seen.insert(k.clone());
            }
        }

        let mut summaries = BTreeMap::new();
        for metric in seen {
            if tier == HardwareTier::TierB
                && (metric == "hrv_rmssd_ms" || metric == "hrv_sdnn_ms")
            {
                continue;
            }
            if tier == HardwareTier::TierC && metric != "steps" {
                continue;
            }

            let (label, def_unit, polarity) = metric_meta(&metric);
            let today_entry = canonical.get(&d_str).and_then(|m| m.get(&metric));

            let short_vals: Vec<f64> = canonical
                .iter()
                .filter(|(day, _)| *day >= &short_start && *day <= &d_str)
                .filter_map(|(_, m)| m.get(&metric).map(|r| r.value))
                .collect();

            let base_vals: Vec<f64> = canonical
                .iter()
                .filter(|(day, _)| *day >= &base_start && *day <= &base_end)
                .filter_map(|(_, m)| m.get(&metric).map(|r| r.value))
                .collect();

            if today_entry.is_none() && short_vals.is_empty() && base_vals.is_empty() {
                continue;
            }

            let cur_val = today_entry.map(|r| round2(r.value));
            let avg_7d = mean(&short_vals).map(round2);
            let base_mean = mean(&base_vals).map(round2);
            let base_std = base_mean.and_then(|bm| sample_std(&base_vals, bm)).map(round2);
            let z_score = match (cur_val, base_mean, base_std) {
                (Some(cv), Some(bm), Some(bs)) if bs > 1e-6 => Some(round2((cv - bm) / bs)),
                _ => None,
            };
            let status = classify_z_status(z_score, polarity, 1.5).to_owned();

            summaries.insert(
                metric.clone(),
                MetricSummary {
                    key: metric,
                    label: label.to_owned(),
                    unit: today_entry
                        .map(|r| r.unit.clone())
                        .filter(|u| !u.is_empty())
                        .unwrap_or_else(|| def_unit.to_owned()),
                    value: cur_val,
                    avg_7d,
                    baseline_30d_mean: base_mean,
                    baseline_30d_std: base_std,
                    baseline_days: base_vals.len(),
                    z_score,
                    status,
                    source: today_entry.map(|r| r.source.clone()),
                },
            );
        }

        let sleep_7d: Vec<f64> = canonical
            .iter()
            .filter(|(day, _)| *day >= &short_start && *day <= &d_str)
            .filter_map(|(_, m)| m.get("sleep_minutes").map(|r| r.value))
            .collect();
        let sleep_debt = if tier == HardwareTier::TierC || sleep_7d.is_empty() {
            None
        } else {
            Some(sleep_7d.iter().map(|s| 450.0 - s).sum::<f64>())
        };

        let acute = compute_daily_load(&canonical, workouts, d - Duration::days(6), d);
        let chronic = compute_daily_load(&canonical, workouts, d - Duration::days(27), d);
        let acwr = if tier == HardwareTier::TierC || chronic <= 1.0 {
            None
        } else {
            Some(round2(acute / chronic))
        };

        let sleep_min = summaries.get("sleep_minutes").and_then(|s| s.value);
        (summaries, sleep_debt, acwr, sleep_min)
    };

    let (metrics, sleep_debt_min, acwr, sleep_min) = summary_for_day(target_date);
    let (qi_score, state, badge, headline, micro_action, signals) =
        evaluate_day_qi(tier, &metrics, sleep_debt_min, acwr, morning_feeling);

    let mut trend_14d = Vec::new();
    for offset in (0..14).rev() {
        let d = target_date - Duration::days(offset);
        let (m, sd, a, _) = summary_for_day(d);
        let (q, st, _, _, _, _) = evaluate_day_qi(tier, &m, sd, a, morning_feeling);
        trend_14d.push(TrendPoint {
            date: d.format("%Y-%m-%d").to_string(),
            short_label: d.format("%m/%d").to_string(),
            qi_score: q,
            state: st.to_owned(),
        });
    }

    let lagged_impacts = compute_lagged_impacts(&canonical, intakes);

    AnalyticsSnapshot {
        target_date: target_str,
        hardware_tier: tier.as_str().to_owned(),
        hardware_tier_label: tier.label().to_owned(),
        qi_score,
        state: state.to_owned(),
        state_badge: badge.to_owned(),
        headline,
        micro_action,
        signals,
        sleep_debt_7d_hours: sleep_debt_min.map(|m| round2(m / 60.0)),
        sleep_last_night_hours: sleep_min.map(|m| round2(m / 60.0)),
        acwr,
        metrics,
        trend_14d,
        lagged_impacts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_series() -> Vec<DailyMetricRow> {
        let start = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
        let mut rows = Vec::new();
        for i in 0..35 {
            let d = (start + Duration::days(i)).format("%Y-%m-%d").to_string();
            rows.push(DailyMetricRow {
                date: d.clone(),
                metric: "hrv_rmssd_ms".into(),
                source: "garmin".into(),
                value: if i == 34 { 38.0 } else { 54.0 + (i % 4) as f64 },
                unit: "ms".into(),
            });
            rows.push(DailyMetricRow {
                date: d.clone(),
                metric: "hrv_sdnn_ms".into(),
                source: "apple_health".into(),
                value: 44.0 + (i % 3) as f64,
                unit: "ms".into(),
            });
            rows.push(DailyMetricRow {
                date: d.clone(),
                metric: "resting_heart_rate_bpm".into(),
                source: "garmin".into(),
                value: if i == 34 { 63.0 } else { 54.0 + (i % 2) as f64 },
                unit: "bpm".into(),
            });
            rows.push(DailyMetricRow {
                date: d,
                metric: "sleep_minutes".into(),
                source: "garmin".into(),
                value: if i == 34 { 330.0 } else { 445.0 },
                unit: "min".into(),
            });
        }
        rows
    }

    #[test]
    fn isolates_rmssd_and_sdnn_baselines() {
        let snap = compute_snapshot(&sample_series(), &[], &[], HardwareTier::TierA, 4);
        let rmssd = snap.metrics.get("hrv_rmssd_ms").unwrap();
        let sdnn = snap.metrics.get("hrv_sdnn_ms").unwrap();
        assert!((rmssd.baseline_30d_mean.unwrap() - 55.5).abs() < 1.0);
        assert!((sdnn.baseline_30d_mean.unwrap() - 45.0).abs() < 1.0);
        assert!(rmssd.z_score.unwrap() < -2.0);
        assert_eq!(snap.state, "recovery_priority");
    }

    #[test]
    fn tier_b_and_c_never_fabricate_hrv() {
        let rows = sample_series();
        let snap_b = compute_snapshot(&rows, &[], &[], HardwareTier::TierB, 4);
        assert!(!snap_b.metrics.contains_key("hrv_rmssd_ms"));
        assert!(!snap_b.metrics.contains_key("hrv_sdnn_ms"));

        let snap_c = compute_snapshot(&rows, &[], &[], HardwareTier::TierC, 5);
        assert_eq!(snap_c.qi_score, 95);
    }
}

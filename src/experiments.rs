//! N-of-1 微习惯实验室与 7 天周期前后对照引擎（Phase 2 核心体验提前端侧就绪）。

use std::collections::BTreeSet;

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::analytics::{DailyMetricRow, resolve_canonical_metrics};

fn default_washout_days() -> i64 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentRow {
    pub id: i64,
    pub title: String,
    pub hypothesis: String,
    pub target_days: u32,
    pub start_date: String,
    pub status: String,
    pub checkin_dates: Vec<String>,
    #[serde(default = "default_washout_days")]
    pub washout_days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentEvaluation {
    pub id: i64,
    pub title: String,
    pub hypothesis: String,
    pub target_days: u32,
    pub start_date: String,
    pub status: String,
    pub washout_days: i64,
    pub washout_skipped_days: usize,
    pub completed_days: usize,
    pub checked_today: bool,
    pub hrv_delta_ms: Option<f64>,
    pub rhr_delta_bpm: Option<f64>,
    pub deep_sleep_delta_min: Option<f64>,
    pub outcome_verdict: String,
    pub verdict_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodComparison {
    pub current_window: String,
    pub previous_window: String,
    pub hrv_current: Option<f64>,
    pub hrv_prev: Option<f64>,
    pub rhr_current: Option<f64>,
    pub rhr_prev: Option<f64>,
    pub sleep_current_h: Option<f64>,
    pub sleep_prev_h: Option<f64>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentTemplate {
    pub id: &'static str,
    pub title: &'static str,
    pub hypothesis: &'static str,
    pub target_days: u32,
}

pub const EXPERIMENT_TEMPLATES: &[ExperimentTemplate] = &[
    ExperimentTemplate {
        id: "no_caffeine_after_15",
        title: "15:00 后零咖啡因",
        hypothesis: "减少下午半衰期咖啡因残留，观察当晚深睡时长与次日夜间 HRV",
        target_days: 7,
    },
    ExperimentTemplate {
        id: "screen_off_60m",
        title: "睡前 60 分钟放下屏幕",
        hypothesis: "降低睡前蓝光与交感唤醒，观察 RHR 回落与入睡节律",
        target_days: 7,
    },
    ExperimentTemplate {
        id: "early_dinner_1930",
        title: "晚餐提前至 19:30 前",
        hypothesis: "避开睡前胃肠高负荷代谢，观察夜间静息心率与 HRV",
        target_days: 14,
    },
    ExperimentTemplate {
        id: "morning_sunlight_15m",
        title: "晨间户外日照 15 分钟",
        hypothesis: "锚定视交叉上核昼夜节律，助力维 D 合成与夜晚褪黑素节律",
        target_days: 14,
    },
];

fn mean(vals: &[f64]) -> Option<f64> {
    if vals.is_empty() {
        None
    } else {
        Some(vals.iter().sum::<f64>() / vals.len() as f64)
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub fn evaluate_experiments(
    rows: &[DailyMetricRow],
    experiments: &[ExperimentRow],
    target_date: &str,
) -> Vec<ExperimentEvaluation> {
    let canonical = resolve_canonical_metrics(rows);
    let mut out = Vec::new();

    for exp in experiments {
        let washout = exp.washout_days.max(0);
        let checkin_set: BTreeSet<&str> = exp.checkin_dates.iter().map(|s| s.as_str()).collect();
        let next_days: BTreeSet<String> = exp
            .checkin_dates
            .iter()
            .filter_map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .map(|nd| (nd + Duration::days(1)).format("%Y-%m-%d").to_string())
            .collect();

        // 计算洗脱期日期集合：
        // 1) 实验开始前 [start_date - washout, start_date) 过渡洗脱窗
        // 2) 每次干预打卡后 (d + 1 ..= d + washout] 的残留洗脱窗（若当天未继续打卡）
        let mut washout_dates: BTreeSet<String> = BTreeSet::new();
        if washout > 0 {
            if let Ok(sd) = NaiveDate::parse_from_str(&exp.start_date, "%Y-%m-%d") {
                for offset in 1..=washout {
                    let wd = (sd - Duration::days(offset)).format("%Y-%m-%d").to_string();
                    if !checkin_set.contains(wd.as_str()) {
                        washout_dates.insert(wd);
                    }
                }
            }
            for c_str in &exp.checkin_dates {
                if let Ok(cd) = NaiveDate::parse_from_str(c_str, "%Y-%m-%d") {
                    for offset in 1..=washout {
                        let wd = (cd + Duration::days(offset)).format("%Y-%m-%d").to_string();
                        if !checkin_set.contains(wd.as_str()) {
                            washout_dates.insert(wd);
                        }
                    }
                }
            }
        }
        let washout_skipped_days = canonical
            .keys()
            .filter(|d| washout_dates.contains(*d))
            .count();

        let metric_delta = |metric_key: &str, use_same_night: bool| -> Option<f64> {
            let mut on_vals = Vec::new();
            let mut off_vals = Vec::new();
            for (day, day_map) in &canonical {
                if let Some(m) = day_map.get(metric_key) {
                    let hit = if use_same_night {
                        checkin_set.contains(day.as_str())
                    } else {
                        next_days.contains(day)
                    };
                    if hit {
                        on_vals.push(m.value);
                    } else if !washout_dates.contains(day) {
                        off_vals.push(m.value);
                    }
                }
            }
            if on_vals.len() >= 2 && off_vals.len() >= 3 {
                Some(round2(mean(&on_vals)? - mean(&off_vals)?))
            } else {
                None
            }
        };

        let hrv_delta = metric_delta("hrv_rmssd_ms", false)
            .or_else(|| metric_delta("hrv_sdnn_ms", false));
        let rhr_delta = metric_delta("resting_heart_rate_bpm", false);
        let deep_delta = metric_delta("sleep_deep_minutes", true);

        let has_any_delta = hrv_delta.is_some() || rhr_delta.is_some() || deep_delta.is_some();
        let positive_signal = hrv_delta.map(|h| h >= 1.5).unwrap_or(false)
            || deep_delta.map(|d| d >= 5.0).unwrap_or(false)
            || rhr_delta.map(|r| r <= -1.0).unwrap_or(false);
        let outcome_verdict = if !has_any_delta {
            "insufficient_data".to_owned()
        } else if positive_signal {
            "effective".to_owned()
        } else {
            "neutral".to_owned()
        };

        let washout_tag = if washout > 0 {
            format!("（已跳过 {washout} 天洗脱期）")
        } else {
            String::new()
        };

        let verdict_summary = if outcome_verdict == "neutral" && exp.checkin_dates.len() >= 3 {
            format!(
                "已完成 {} 天对照{}：指标波动未见显著增益（假说暂未成立，可省下无效坚持）。",
                exp.checkin_dates.len(),
                washout_tag
            )
        } else {
            match (hrv_delta, deep_delta, rhr_delta) {
                (Some(h), Some(d), _) => format!(
                    "依从 {} 天对比基线{}：次日夜间 HRV {:+.1} ms，当晚深睡 {:+.1} min。",
                    exp.checkin_dates.len(),
                    washout_tag,
                    h,
                    d
                ),
                (Some(h), None, _) => format!(
                    "依从 {} 天对比基线{}：次日夜间 HRV {:+.1} ms。",
                    exp.checkin_dates.len(),
                    washout_tag,
                    h
                ),
                (_, Some(d), _) => format!(
                    "依从 {} 天对比基线{}：当晚深睡时长 {:+.1} min。",
                    exp.checkin_dates.len(),
                    washout_tag,
                    d
                ),
                _ => format!(
                    "已打卡 {} / {} 天：继续坚持 2 天以上即可解锁实验期 vs 基线生理差值。",
                    exp.checkin_dates.len(),
                    exp.target_days
                ),
            }
        };

        out.push(ExperimentEvaluation {
            id: exp.id,
            title: exp.title.clone(),
            hypothesis: exp.hypothesis.clone(),
            target_days: exp.target_days,
            start_date: exp.start_date.clone(),
            status: exp.status.clone(),
            washout_days: washout,
            washout_skipped_days,
            completed_days: exp.checkin_dates.len(),
            checked_today: checkin_set.contains(target_date),
            hrv_delta_ms: hrv_delta,
            rhr_delta_bpm: rhr_delta,
            deep_sleep_delta_min: deep_delta,
            outcome_verdict,
            verdict_summary,
        });
    }

    out
}

pub fn compute_period_comparison(rows: &[DailyMetricRow], target_date: &str) -> PeriodComparison {
    let canonical = resolve_canonical_metrics(rows);
    let end = NaiveDate::parse_from_str(target_date, "%Y-%m-%d")
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2026, 11, 13).expect("valid date"));
    let cur_start = end - Duration::days(6);
    let prev_end = cur_start - Duration::days(1);
    let prev_start = prev_end - Duration::days(6);

    let avg_between = |metric: &str, s: NaiveDate, e: NaiveDate| -> Option<f64> {
        let s_str = s.format("%Y-%m-%d").to_string();
        let e_str = e.format("%Y-%m-%d").to_string();
        let vals: Vec<f64> = canonical
            .iter()
            .filter(|(d, _)| *d >= &s_str && *d <= &e_str)
            .filter_map(|(_, m)| m.get(metric).map(|r| r.value))
            .collect();
        mean(&vals).map(round2)
    };

    let hrv_cur = avg_between("hrv_rmssd_ms", cur_start, end)
        .or_else(|| avg_between("hrv_sdnn_ms", cur_start, end));
    let hrv_prev = avg_between("hrv_rmssd_ms", prev_start, prev_end)
        .or_else(|| avg_between("hrv_sdnn_ms", prev_start, prev_end));
    let rhr_cur = avg_between("resting_heart_rate_bpm", cur_start, end);
    let rhr_prev = avg_between("resting_heart_rate_bpm", prev_start, prev_end);
    let sleep_cur = avg_between("sleep_minutes", cur_start, end).map(|m| round2(m / 60.0));
    let sleep_prev = avg_between("sleep_minutes", prev_start, prev_end).map(|m| round2(m / 60.0));

    let summary = match (hrv_cur, hrv_prev, sleep_cur, sleep_prev) {
        (Some(hc), Some(hp), Some(sc), Some(sp)) => format!(
            "近 7 天较前 7 天：HRV 均值 {:.1} → {:.1} ms（{:+.1}），日均睡眠 {:.1}h → {:.1}h。",
            hp,
            hc,
            hc - hp,
            sp,
            sc
        ),
        _ => "持续积累满 14 天数据后自动展开近 7 天 vs 前 7 天趋势对比。".to_owned(),
    };

    PeriodComparison {
        current_window: format!("{} ~ {}", cur_start.format("%m/%d"), end.format("%m/%d")),
        previous_window: format!(
            "{} ~ {}",
            prev_start.format("%m/%d"),
            prev_end.format("%m/%d")
        ),
        hrv_current: hrv_cur,
        hrv_prev,
        rhr_current: rhr_cur,
        rhr_prev,
        sleep_current_h: sleep_cur,
        sleep_prev_h: sleep_prev,
        summary,
    }
}

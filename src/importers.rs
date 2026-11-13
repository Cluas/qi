//! 外部健康数据导入器：支持 Apple Health `export.xml` 片段、CSV 日报及 OpenHealthSync JSONL。
//! 严格维持 `hrv_rmssd_ms` 与 Apple Watch `hrv_sdnn_ms` 的物理口径隔离。

use std::collections::BTreeMap;

use crate::analytics::DailyMetricRow;

/// 解析 Apple Health `export.xml` 片段（纯字符串流式解析，避免引入大体积 XML 依赖），
/// 按日聚合后转化为标准 `DailyMetricRow`。
pub fn parse_apple_health_xml_snippet(xml_text: &str) -> Vec<DailyMetricRow> {
    let mut sum_steps: BTreeMap<String, f64> = BTreeMap::new();
    let mut list_sdnn: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut list_rhr: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut sum_sleep_min: BTreeMap<String, f64> = BTreeMap::new();

    for line in xml_text.lines() {
        let trimmed = line.trim();
        if !trimmed.contains("<Record") {
            continue;
        }
        let rec_type = extract_xml_attr(trimmed, "type").unwrap_or_default();
        let date = extract_xml_attr(trimmed, "startDate")
            .or_else(|| extract_xml_attr(trimmed, "endDate"))
            .and_then(|s| s.get(..10).map(|d| d.to_owned()))
            .unwrap_or_default();
        if date.len() != 10 {
            continue;
        }

        match rec_type.as_str() {
            "HKQuantityTypeIdentifierHeartRateVariabilitySDNN" => {
                if let Some(v) = extract_xml_attr(trimmed, "value")
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    list_sdnn.entry(date).or_default().push(v);
                }
            }
            "HKQuantityTypeIdentifierRestingHeartRate" => {
                if let Some(v) = extract_xml_attr(trimmed, "value")
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    list_rhr.entry(date).or_default().push(v);
                }
            }
            "HKQuantityTypeIdentifierStepCount" => {
                if let Some(v) = extract_xml_attr(trimmed, "value")
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    *sum_steps.entry(date).or_default() += v;
                }
            }
            "HKCategoryTypeIdentifierSleepAnalysis" => {
                if let Some(v) = extract_xml_attr(trimmed, "durationMinutes")
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    *sum_sleep_min.entry(date).or_default() += v;
                }
            }
            _ => {}
        }
    }

    let mut out = Vec::new();
    for (d, vals) in list_sdnn {
        let avg = vals.iter().sum::<f64>() / vals.len() as f64;
        out.push(DailyMetricRow {
            date: d,
            metric: "hrv_sdnn_ms".to_owned(),
            source: "apple_health".to_owned(),
            value: (avg * 100.0).round() / 100.0,
            unit: "ms".to_owned(),
        });
    }
    for (d, vals) in list_rhr {
        let avg = vals.iter().sum::<f64>() / vals.len() as f64;
        out.push(DailyMetricRow {
            date: d,
            metric: "resting_heart_rate_bpm".to_owned(),
            source: "apple_health".to_owned(),
            value: (avg * 100.0).round() / 100.0,
            unit: "bpm".to_owned(),
        });
    }
    for (d, total) in sum_steps {
        out.push(DailyMetricRow {
            date: d,
            metric: "steps".to_owned(),
            source: "apple_health".to_owned(),
            value: total.round(),
            unit: "步".to_owned(),
        });
    }
    for (d, total) in sum_sleep_min {
        out.push(DailyMetricRow {
            date: d,
            metric: "sleep_minutes".to_owned(),
            source: "apple_health".to_owned(),
            value: total.round(),
            unit: "min".to_owned(),
        });
    }
    out
}

fn extract_xml_attr(line: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let start = line.find(&needle)? + needle.len();
    let end = line[start..].find('"')?;
    Some(line[start..start + end].to_owned())
}

/// 解析 CSV 日报：`date,metric,source,value,unit`（可带表头）
pub fn parse_csv_metrics(csv_text: &str) -> Vec<DailyMetricRow> {
    let mut out = Vec::new();
    for (idx, line) in csv_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || (idx == 0 && trimmed.to_ascii_lowercase().starts_with("date,")) {
            continue;
        }
        let cols: Vec<&str> = trimmed.split(',').map(|s| s.trim()).collect();
        if cols.len() < 4 {
            continue;
        }
        if let Ok(val) = cols[3].parse::<f64>() {
            out.push(DailyMetricRow {
                date: cols[0].to_owned(),
                metric: cols[1].to_owned(),
                source: if cols[2].is_empty() {
                    "csv_import".to_owned()
                } else {
                    cols[2].to_owned()
                },
                value: val,
                unit: cols.get(4).copied().unwrap_or("").to_owned(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_apple_health_xml_and_csv_without_mixing_sdnn_rmssd() {
        let xml = r#"
<HealthData>
  <Record type="HKQuantityTypeIdentifierHeartRateVariabilitySDNN" startDate="2026-11-12 06:10:00 +0800" value="48.5"/>
  <Record type="HKQuantityTypeIdentifierStepCount" startDate="2026-11-12 12:00:00 +0800" value="4200"/>
  <Record type="HKQuantityTypeIdentifierStepCount" startDate="2026-11-12 19:00:00 +0800" value="5100"/>
</HealthData>"#;
        let rows = parse_apple_health_xml_snippet(xml);
        assert!(rows.iter().any(|r| r.metric == "hrv_sdnn_ms" && (r.value - 48.5).abs() < 0.01));
        assert!(!rows.iter().any(|r| r.metric == "hrv_rmssd_ms"));
        assert!(rows.iter().any(|r| r.metric == "steps" && (r.value - 9300.0).abs() < 0.1));

        let csv = "date,metric,source,value,unit\n2026-11-12,hrv_rmssd_ms,garmin,58.2,ms\n";
        let csv_rows = parse_csv_metrics(csv);
        assert_eq!(csv_rows.len(), 1);
        assert_eq!(csv_rows[0].metric, "hrv_rmssd_ms");
    }
}

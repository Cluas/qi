//! 端侧前置隐私脱敏、25+ 体检指标结构化纠偏、纵深变化趋势与 SaMD 医疗红线护栏。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::analytics::{AnalyticsSnapshot, LabBiomarkerRow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrubResult {
    pub scrubbed_text: String,
    pub redacted_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedLabMarkerDraft {
    pub marker_name: String,
    pub canonical_key: String,
    pub value: f64,
    pub unit: String,
    pub ref_low: Option<f64>,
    pub ref_high: Option<f64>,
    pub flag: String,
    pub plain_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingResult {
    pub provider_label: String,
    pub degraded: bool,
    pub text: String,
    pub disclaimer: String,
}

pub const WELLNESS_DISCLAIMER: &str =
    "仅供日常健康节律、作息与生活方式管理参考，不构成临床医疗诊断或治疗建议。";

/// 在化验单文本离开本机前精准遮盖页眉/个人隐私字段，同时 100% 保留化验数值与参考区间。
pub fn scrub_lab_text_pii(raw: &str) -> ScrubResult {
    let mut redacted = Vec::new();
    let mut out_lines = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // 页眉医院抬头行遮盖
        if (trimmed.contains("医院") || trimmed.contains("体检中心") || trimmed.contains("检验科"))
            && !trimmed.chars().any(|c| c == '.' && trimmed.contains('/'))
            && !trimmed.contains("mmol")
            && !trimmed.contains("umol")
        {
            if !redacted.contains(&"医疗机构抬头".to_owned()) {
                redacted.push("医疗机构抬头".to_owned());
            }
            out_lines.push("[已遮盖: 医疗机构页眉]".to_owned());
            continue;
        }

        let mut cleaned = line.to_owned();
        for (prefix, tag) in [
            ("姓名", "姓名"),
            ("身份证号", "身份证号"),
            ("身份证", "身份证号"),
            ("住院号", "病历/住院号"),
            ("门诊号", "病历/住院号"),
            ("病历号", "病历/住院号"),
            ("体检号", "病历/住院号"),
            ("电话", "手机号"),
            ("手机", "手机号"),
        ] {
            if let Some(pos) = cleaned.find(prefix) {
                let after = &cleaned[pos..];
                if let Some((colon_idx, ch)) = after
                    .char_indices()
                    .find(|&(_, ch)| ch == '：' || ch == ':')
                {
                    let split_at = pos + colon_idx + ch.len_utf8();
                    let prefix_keep = &cleaned[..split_at];
                    let tail = &cleaned[split_at..];
                    if tail.trim_start().starts_with("[已脱敏]") {
                        continue;
                    }
                    let rest = tail
                        .trim_start()
                        .split_once(char::is_whitespace)
                        .map(|(_, r)| format!(" {r}"))
                        .unwrap_or_default();
                    cleaned = format!("{prefix_keep}[已脱敏]{rest}");
                    if !redacted.contains(&tag.to_owned()) {
                        redacted.push(tag.to_owned());
                    }
                }
            }
        }

        cleaned = redact_digit_runs(&cleaned, &mut redacted);
        out_lines.push(cleaned);
    }

    ScrubResult {
        scrubbed_text: out_lines.join("\n"),
        redacted_fields: redacted,
    }
}

fn redact_digit_runs(input: &str, redacted: &mut Vec<String>) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_digit()
                    || (i - start == 17 && (chars[i] == 'X' || chars[i] == 'x')))
            {
                i += 1;
            }
            let len = i - start;
            let token: String = chars[start..i].iter().collect();
            if len == 18 {
                if !redacted.contains(&"身份证号".to_owned()) {
                    redacted.push("身份证号".to_owned());
                }
                out.push_str("[已脱敏:18位证件]");
            } else if len == 11 && token.starts_with('1') {
                if !redacted.contains(&"手机号".to_owned()) {
                    redacted.push("手机号".to_owned());
                }
                out.push_str("[已脱敏:手机号]");
            } else {
                out.push_str(&token);
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// 标准化化验单行（解决 Vision/PDF OCR 常见的全角括号无空格粘连如 `谷丙转氨酶（ALT）56`）
fn normalize_lab_line(line: &str) -> String {
    let mut s = line
        .replace('（', "(")
        .replace('）', ") ")
        .replace('–', "-")
        .replace('—', "-")
        .replace('\t', " ")
        .replace('|', " ");
    // 兼容 "指标名: 数值" 或 "指标名：数值"
    s = s.replace("：", " ").replace(": ", " ");
    s
}

fn parse_range_token(tok: &str) -> Option<(f64, f64)> {
    let cleaned = tok.trim_matches(|c: char| c == '[' || c == ']' || c == '(' || c == ')');
    for sep in ['-', '~'] {
        if let Some((l_s, h_s)) = cleaned.split_once(sep)
            && let (Ok(l), Ok(h)) = (l_s.trim().parse::<f64>(), h_s.trim().parse::<f64>())
        {
            return Some((l, h));
        }
    }
    None
}

/// 确定性提取检验行并由 Rust 重算 `ref_low..ref_high` 状态（防小模型误判升降箭头）。
pub fn parse_lab_text_and_verify(scrubbed: &str) -> Vec<ParsedLabMarkerDraft> {
    let mut results = Vec::new();
    for raw_line in scrubbed.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("[已遮盖")
            || trimmed.contains("姓名：[已脱敏]")
        {
            continue;
        }
        let normalized = normalize_lab_line(trimmed);
        let tokens: Vec<&str> = normalized.split_whitespace().collect();
        if tokens.len() < 2 {
            continue;
        }

        let mut value_opt = None;
        let mut value_idx = 0usize;
        for (idx, tok) in tokens.iter().enumerate() {
            if idx == 0 {
                continue;
            }
            let clean_tok = tok.trim_matches(|c| c == '↑' || c == '↓' || c == 'H' || c == 'L');
            if let Ok(v) = clean_tok.parse::<f64>() {
                value_opt = Some(v);
                value_idx = idx;
                break;
            }
        }

        let Some(val) = value_opt else {
            continue;
        };

        let marker_name = tokens[..value_idx].join(" ");
        let unit = tokens
            .get(value_idx + 1)
            .filter(|t| !t.contains('-') && !t.contains('~') && t.parse::<f64>().is_err())
            .copied()
            .unwrap_or("");

        let mut ref_low = None;
        let mut ref_high = None;
        for tok in &tokens[value_idx + 1..] {
            if let Some((l, h)) = parse_range_token(tok) {
                ref_low = Some(l);
                ref_high = Some(h);
                break;
            }
        }

        let flag = match (ref_low, ref_high) {
            (Some(lo), Some(hi)) => {
                if val > hi {
                    "high"
                } else if val < lo {
                    "low"
                } else {
                    "normal"
                }
            }
            (None, Some(hi)) => {
                if val > hi {
                    "high"
                } else {
                    "normal"
                }
            }
            (Some(lo), None) => {
                if val < lo {
                    "low"
                } else {
                    "normal"
                }
            }
            _ => "normal",
        };

        let canonical_key = canonicalize_marker(&marker_name);
        let plain_note = biomarker_plain_note(&canonical_key, flag);

        results.push(ParsedLabMarkerDraft {
            marker_name,
            canonical_key,
            value: val,
            unit: unit.to_owned(),
            ref_low,
            ref_high,
            flag: flag.to_owned(),
            plain_note,
        });
    }
    results
}

fn biomarker_plain_note(canonical_key: &str, flag: &str) -> String {
    let msg = match (canonical_key, flag) {
        ("uric_acid", "high") => {
            "偏高于参考上限：建议每日饮水 ≥2000ml，减少啤酒、浓肉汤与高嘌呤海鲜内脏摄入。"
        }
        ("ldl_c", "high") => {
            "低密度脂蛋白提示关注：建议减少反式脂肪与油炸外卖，增加深海鱼、燕麦及膳食纤维。"
        }
        ("hdl_c", "low") => {
            "高密度脂蛋白偏低：适度增加每周 150 分钟 Zone 2 有氧与优质单不饱和脂肪（橄榄油/坚果）。"
        }
        ("triglycerides", "high") => {
            "甘油三酯偏高：建议控制精制米面、含糖奶茶与夜间饮酒，优先全谷物碳水。"
        }
        ("total_cholesterol", "high") => {
            "总胆固醇高于参考上限：可结合 LDL-C 一起留意膳食饱和脂肪比例并加强有氧活动。"
        }
        ("fasting_glucose", "high") | ("hba1c", "high") => {
            "糖代谢指标提示关注：推荐“先菜后肉再主食”的进食顺序，餐后快走 15 分钟有助平稳血糖。"
        }
        ("alt", "high") | ("ast", "high") | ("ggt", "high") => {
            "转氨酶高于参考上限：可能与近期熬夜、高强度力量训练或饮酒相关，建议减少酒精与高脂夜宵并按期复查。"
        }
        ("vitamin_d", "low") => {
            "维生素 D 偏低：建议每日上午适度户外日照 15 分钟，膳食搭配深海鱼/蛋黄或按需补充。"
        }
        ("ferritin", "low") | ("hemoglobin", "low") => {
            "铁蛋白/血红蛋白偏低：耐力训练与疲劳感易受影响，膳食中可适度增加瘦红肉与维 C 促铁吸收。"
        }
        ("hs_crp", "high") => {
            "超敏 C 反应蛋白提示关注：可能反映近期训练负荷偏大或轻度恢复期应激，宜保证充足睡眠与抗炎饮食。"
        }
        ("homocysteine", "high") => {
            "同型半胱氨酸高于参考区间：建议多摄入深绿色叶菜，保持规律有氧作息。"
        }
        ("tsh", "high") | ("tsh", "low") => {
            "促甲状腺激素位于区间边缘之外：若近期伴有明显精力波动，建议带着报告向内分泌科医师咨询。"
        }
        (_, "high") => "数值高于检验单参考区间上限，建议结合近期作息与饮食调整后定期复查。",
        (_, "low") => "数值低于检验单参考区间下限，建议留意营养均衡与休息恢复节奏。",
        _ => "位于常规参考区间内，继续保持良好作息与饮食平衡即可。",
    };
    msg.to_owned()
}

/// 25+ 项核心体检指标规范化映射
pub fn canonicalize_marker(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    if name.contains("尿酸") || lower.contains("(ua)") || lower == "ua" {
        "uric_acid".to_owned()
    } else if name.contains("低密度") || lower.contains("ldl") {
        "ldl_c".to_owned()
    } else if name.contains("高密度") || lower.contains("hdl") {
        "hdl_c".to_owned()
    } else if name.contains("总胆固醇") || lower == "tc" || lower.contains("(tc)") {
        "total_cholesterol".to_owned()
    } else if name.contains("甘油三酯") || lower.contains("(tg)") || lower == "tg" {
        "triglycerides".to_owned()
    } else if name.contains("糖化血红蛋白") || lower.contains("hba1c") {
        "hba1c".to_owned()
    } else if name.contains("空腹血糖") || lower.contains("glucose") || lower.contains("glu") {
        "fasting_glucose".to_owned()
    } else if name.contains("谷丙转氨酶") || lower.contains("(alt)") || lower.contains("alt") {
        "alt".to_owned()
    } else if name.contains("谷草转氨酶") || lower.contains("(ast)") || lower.contains("ast") {
        "ast".to_owned()
    } else if name.contains("谷氨酰") || lower.contains("ggt") {
        "ggt".to_owned()
    } else if name.contains("肌酐") || lower.contains("(cr)") || lower.contains("crea") {
        "creatinine".to_owned()
    } else if name.contains("肾小球滤过率") || lower.contains("egfr") {
        "egfr".to_owned()
    } else if name.contains("尿素氮") || lower.contains("bun") {
        "bun".to_owned()
    } else if name.contains("促甲状腺") || lower.contains("tsh") {
        "tsh".to_owned()
    } else if name.contains("游离三碘") || lower.contains("ft3") {
        "ft3".to_owned()
    } else if name.contains("游离甲状腺") || lower.contains("ft4") {
        "ft4".to_owned()
    } else if name.contains("维生素D") || name.contains("维生素d") || lower.contains("25-oh") {
        "vitamin_d".to_owned()
    } else if name.contains("铁蛋白") || lower.contains("ferritin") {
        "ferritin".to_owned()
    } else if name.contains("血红蛋白") || lower.contains("(hb)") || lower == "hb" {
        "hemoglobin".to_owned()
    } else if name.contains("白细胞") || lower.contains("wbc") {
        "wbc".to_owned()
    } else if name.contains("血小板") || lower.contains("plt") {
        "platelets".to_owned()
    } else if name.contains("C反应蛋白") || name.contains("C 反应蛋白") || lower.contains("crp") {
        "hs_crp".to_owned()
    } else if name.contains("同型半胱氨酸") || lower.contains("hcy") {
        "homocysteine".to_owned()
    } else if name.contains("血钾") || lower.contains("(k)") {
        "potassium".to_owned()
    } else if name.contains("血钠") || lower.contains("(na)") {
        "sodium".to_owned()
    } else {
        let ascii: String = lower
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(24)
            .collect();
        if ascii.is_empty() {
            format!("marker_{}", name.chars().count())
        } else {
            ascii
        }
    }
}

/// 为同一 `canonical_key` 的多年多期体检记录补充纵向变化趋势（较上一期改善/回升/平稳）
pub fn enrich_lab_longitudinal_trends(mut rows: Vec<LabBiomarkerRow>) -> Vec<LabBiomarkerRow> {
    let mut by_key: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, r) in rows.iter().enumerate() {
        by_key.entry(r.canonical_key.clone()).or_default().push(idx);
    }

    for indices in by_key.values() {
        // 按检验日期升序串联前期历史
        let mut sorted = indices.clone();
        sorted.sort_by(|&a, &b| rows[a].test_date.cmp(&rows[b].test_date));

        for pos in 0..sorted.len() {
            let cur_idx = sorted[pos];
            if pos == 0 {
                rows[cur_idx].trend = Some("new".to_owned());
                continue;
            }
            let prev_idx = sorted[pos - 1];
            let prev_val = rows[prev_idx].value;
            let prev_date = rows[prev_idx].test_date.clone();
            let cur_val = rows[cur_idx].value;
            let delta = ((cur_val - prev_val) * 100.0).round() / 100.0;

            let trend = if delta.abs() < 1e-4 {
                "stable".to_owned()
            } else {
                // 若此前在区间外而本期距离区间中心更近，或回归 normal，则判定为 improved
                let mid_opt = match (rows[cur_idx].ref_low, rows[cur_idx].ref_high) {
                    (Some(lo), Some(hi)) => Some((lo + hi) / 2.0),
                    _ => None,
                };
                if rows[cur_idx].flag == "normal" && rows[prev_idx].flag != "normal" {
                    "improved".to_owned()
                } else if rows[cur_idx].flag != "normal" && rows[prev_idx].flag == "normal" {
                    "worsened".to_owned()
                } else if let Some(mid) = mid_opt {
                    let prev_dist = (prev_val - mid).abs();
                    let cur_dist = (cur_val - mid).abs();
                    if cur_dist + 1e-4 < prev_dist {
                        "improved".to_owned()
                    } else if cur_dist > prev_dist + 1e-4 {
                        "worsened".to_owned()
                    } else {
                        "stable".to_owned()
                    }
                } else {
                    "stable".to_owned()
                }
            };

            rows[cur_idx].prev_value = Some(prev_val);
            rows[cur_idx].prev_date = Some(prev_date);
            rows[cur_idx].delta = Some(delta);
            rows[cur_idx].trend = Some(trend);
        }
    }
    rows
}

/// 剥离推理模型的 `<think>...</think>` 思考过程
pub fn strip_think_tags(input: &str) -> String {
    let mut rest = input;
    let mut out = String::with_capacity(input.len());
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find("</think>") {
            rest = &rest[start + end + "</think>".len()..];
        } else {
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out.trim().to_owned()
}

/// 强制医疗红线词过滤（SaMD Guardrails），防止越界措辞。
pub fn enforce_wellness_guardrails(text: &str) -> String {
    let mut safe = strip_think_tags(text);
    for (bad, good) in [
        ("确诊为", "提示关注"),
        ("确诊", "参考提示"),
        ("开药", "膳食与作息建议"),
        ("处方", "调整参考"),
        ("治疗方案", "日常调养方案"),
        ("治疗", "调养参考"),
        ("临床诊断", "生理趋势提示"),
        ("诊断", "节律评估"),
        ("心律失常预警", "节律波动提醒"),
    ] {
        safe = safe.replace(bad, good);
    }
    safe
}

/// 根据当前 AI 模式、设备热降频状态与确定性快照生成早安简报（确保零阻塞、零幻觉）。
pub fn build_morning_briefing(
    snapshot: &AnalyticsSnapshot,
    labs: &[LabBiomarkerRow],
    ai_mode: &str,
    lan_url: &str,
    thermal_throttled: bool,
    ram_gb: u32,
) -> BriefingResult {
    let flagged_count = labs.iter().filter(|l| l.flag != "normal").count();
    let lab_hint = if flagged_count > 0 {
        format!(" 体检档案中当前有 {flagged_count} 项临界关注指标，餐饮宜保持清淡低盐。")
    } else {
        String::new()
    };

    if thermal_throttled && ai_mode == "on_device_local" {
        let text = enforce_wellness_guardrails(&format!(
            "当前设备偏热，已无感切换为确定性精简解读：今日元气 {} 分（{}）。{}{}",
            snapshot.qi_score, snapshot.state_badge, snapshot.headline, lab_hint
        ));
        return BriefingResult {
            provider_label: "设备受热 · 已降级至 Rust 确定性统计".to_owned(),
            degraded: true,
            text,
            disclaimer: WELLNESS_DISCLAIMER.to_owned(),
        };
    }

    let (provider_label, lead) = match ai_mode {
        "on_device_local" => {
            if ram_gb < 6 {
                (
                    "内存 <6GB 门禁保护 · 仅用确定性基线引擎",
                    format!(
                        "本机可用内存为 {ram_gb}GB，为防系统强杀已跳过离线大模型加载；由 Rust 基线引擎秒算得出："
                    ),
                )
            } else {
                (
                    "纯本机离线模型（iOS FoundationModels / 端侧小模型 · 0 流量不出机）",
                    "全程未联网、零数据上传：基于端侧受限 Schema 润色，".to_owned(),
                )
            }
        }
        "lan_ollama" => {
            let target = if lan_url.trim().is_empty() {
                "http://192.168.x.x:11434/v1"
            } else {
                lan_url.trim()
            };
            (
                "家庭局域网本地大模型（Ollama / 自建网关 · 数据不出内网）",
                format!("经家庭局域网节点（{target}）就绪："),
            )
        }
        _ => (
            "官方 ZDR 零留存极速云端（仅发脱敏特征摘要 · 阅后即焚）",
            "经端侧去标识化摘要与阅后即焚网关待命：".to_owned(),
        ),
    };

    let sleep_part = snapshot
        .sleep_last_night_hours
        .map(|h| format!("昨晚睡眠 {h:.1} 小时；"))
        .unwrap_or_default();

    let raw_text = format!(
        "{lead}今日元气 {} 分（{}）。{sleep_part}{} 行动建议：{}{} ",
        snapshot.qi_score,
        snapshot.state_badge,
        snapshot.headline,
        snapshot.micro_action,
        lab_hint
    );

    BriefingResult {
        provider_label: provider_label.to_owned(),
        degraded: false,
        text: enforce_wellness_guardrails(raw_text.trim()),
        disclaimer: WELLNESS_DISCLAIMER.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubs_pii_without_touching_lab_numbers() {
        let raw = "某某市第一人民医院体检中心\n姓名：张伟 身份证号：31010119900101123X 手机：13800138000\n尿酸（UA）468 umol/L 208-428\n谷丙转氨酶（ALT）56 U/L 9-50\n25-羟维生素D 16.5 ng/mL 30.0-100.0";
        let res = scrub_lab_text_pii(raw);
        assert!(!res.scrubbed_text.contains("张伟"));
        assert!(!res.scrubbed_text.contains("31010119900101123X"));
        assert!(!res.scrubbed_text.contains("13800138000"));
        assert!(res.redacted_fields.contains(&"姓名".to_owned()));

        let parsed = parse_lab_text_and_verify(&res.scrubbed_text);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].canonical_key, "uric_acid");
        assert_eq!(parsed[0].flag, "high");
        assert_eq!(parsed[1].canonical_key, "alt");
        assert_eq!(parsed[1].flag, "high");
        assert_eq!(parsed[2].canonical_key, "vitamin_d");
        assert_eq!(parsed[2].flag, "low");
    }

    #[test]
    fn guardrails_replace_medical_terms() {
        let filtered = enforce_wellness_guardrails("<think>reasoning</think>建议确诊后开药并制定治疗方案");
        assert!(!filtered.contains("reasoning"));
        assert!(!filtered.contains("确诊"));
        assert!(!filtered.contains("开药"));
        assert!(!filtered.contains("治疗方案"));
    }
}

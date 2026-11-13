//! 端侧前置隐私脱敏、25+ 体检指标结构化纠偏、纵深变化趋势与 SaMD 医疗红线护栏。

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::analytics::{AnalyticsSnapshot, LabBiomarkerRow};
use crate::entitlements::{is_en_locale, DEFAULT_LOCALE};

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

pub const WELLNESS_DISCLAIMER_EN: &str =
    "For everyday wellness rhythms, sleep, and lifestyle reference only. Not clinical medical diagnosis or treatment advice.";

pub fn wellness_disclaimer_for_locale(locale: &str) -> &'static str {
    if is_en_locale(locale) {
        WELLNESS_DISCLAIMER_EN
    } else {
        WELLNESS_DISCLAIMER
    }
}

pub fn contains_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        ('\u{4E00}'..='\u{9FFF}').contains(&c)
            || ('\u{3400}'..='\u{4DBF}').contains(&c)
            || ('\u{3000}'..='\u{303F}').contains(&c)
            || ('\u{FF00}'..='\u{FFEF}').contains(&c)
    })
}

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
    parse_lab_text_and_verify_localized(scrubbed, DEFAULT_LOCALE)
}

pub fn parse_lab_text_and_verify_localized(
    scrubbed: &str,
    locale: &str,
) -> Vec<ParsedLabMarkerDraft> {
    let mut results = Vec::new();
    for raw_line in scrubbed.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("[已遮盖")
            || trimmed.starts_with("[Redacted")
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
        let display_name = localized_marker_name(&canonical_key, &marker_name, locale);
        let plain_note = biomarker_plain_note_localized(&canonical_key, flag, locale);

        results.push(ParsedLabMarkerDraft {
            marker_name: display_name,
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

pub fn localized_marker_name(canonical_key: &str, raw_marker_name: &str, locale: &str) -> String {
    if !is_en_locale(locale) {
        return raw_marker_name.to_owned();
    }
    if !contains_cjk(raw_marker_name) && !raw_marker_name.trim().is_empty() {
        return raw_marker_name.to_owned();
    }
    match canonical_key {
        "uric_acid" => "Uric acid (UA)".to_owned(),
        "ldl_c" => "LDL cholesterol (LDL-C)".to_owned(),
        "hdl_c" => "HDL cholesterol (HDL-C)".to_owned(),
        "total_cholesterol" => "Total cholesterol (TC)".to_owned(),
        "triglycerides" => "Triglycerides (TG)".to_owned(),
        "hba1c" => "HbA1c".to_owned(),
        "fasting_glucose" => "Fasting glucose (GLU)".to_owned(),
        "alt" => "Alanine aminotransferase (ALT)".to_owned(),
        "ast" => "Aspartate aminotransferase (AST)".to_owned(),
        "ggt" => "Gamma-GT (GGT)".to_owned(),
        "creatinine" => "Creatinine (Cr)".to_owned(),
        "egfr" => "eGFR".to_owned(),
        "bun" => "Blood urea nitrogen (BUN)".to_owned(),
        "tsh" => "Thyroid-stimulating hormone (TSH)".to_owned(),
        "ft3" => "Free T3 (FT3)".to_owned(),
        "ft4" => "Free T4 (FT4)".to_owned(),
        "vitamin_d" => "25-OH Vitamin D".to_owned(),
        "ferritin" => "Ferritin".to_owned(),
        "hemoglobin" => "Hemoglobin (Hb)".to_owned(),
        "wbc" => "White blood cell count (WBC)".to_owned(),
        "platelets" => "Platelets (PLT)".to_owned(),
        "hs_crp" => "High-sensitivity CRP".to_owned(),
        "homocysteine" => "Homocysteine (Hcy)".to_owned(),
        "potassium" => "Serum potassium (K)".to_owned(),
        "sodium" => "Serum sodium (Na)".to_owned(),
        other => {
            let trimmed = raw_marker_name.trim();
            if !trimmed.is_empty() && !contains_cjk(trimmed) {
                trimmed.to_owned()
            } else {
                other.replace('_', " ")
            }
        }
    }
}

pub fn biomarker_plain_note(canonical_key: &str, flag: &str) -> String {
    biomarker_plain_note_localized(canonical_key, flag, DEFAULT_LOCALE)
}

pub fn biomarker_plain_note_localized(canonical_key: &str, flag: &str, locale: &str) -> String {
    if is_en_locale(locale) {
        let msg = match (canonical_key, flag) {
            ("uric_acid", "high") => {
                "Above reference upper bound: aim for >=2000 ml water daily and cut back on beer, rich broths, and high-purine seafood or organ meats."
            }
            ("ldl_c", "high") => {
                "LDL cholesterol needs attention: limit fried takeout and trans fats, and add oily fish, oats, and dietary fiber."
            }
            ("hdl_c", "low") => {
                "HDL cholesterol is low: work toward 150 weekly minutes of Zone 2 cardio and healthy monounsaturated fats like olive oil and nuts."
            }
            ("triglycerides", "high") => {
                "Triglycerides are above range: ease up on refined grains, sugary drinks, and late-night alcohol, favoring whole-grain carbs."
            }
            ("total_cholesterol", "high") => {
                "Total cholesterol is above the upper bound: check dietary saturated fat alongside LDL-C and keep steady aerobic activity."
            }
            ("fasting_glucose", "high") | ("hba1c", "high") => {
                "Glucose marker needs attention: try eating vegetables and protein before starches, with a 15-minute walk after meals."
            }
            ("alt", "high") | ("ast", "high") | ("ggt", "high") => {
                "Liver enzyme is above the upper limit: recent late nights, intense strength blocks, or alcohol can nudge it up; lighten late meals and recheck on schedule."
            }
            ("vitamin_d", "low") => {
                "Vitamin D is low: get ~15 minutes of morning sunlight daily and pair fatty fish or egg yolks with supplements as needed."
            }
            ("ferritin", "low") | ("hemoglobin", "low") => {
                "Ferritin or hemoglobin is low: endurance stamina and fatigue can feel heavier; pair lean red meat with vitamin C for absorption."
            }
            ("hs_crp", "high") => {
                "hs-CRP needs attention: may reflect recent training load or mild recovery strain; prioritize deep sleep and anti-inflammatory meals."
            }
            ("homocysteine", "high") => {
                "Homocysteine is above range: add dark leafy greens and keep regular aerobic movement."
            }
            ("tsh", "high") | ("tsh", "low") => {
                "TSH sits outside the reference range: if you notice persistent energy shifts, bring this report to an endocrinology clinician."
            }
            (_, "high") => {
                "Above the lab reference upper bound; review recent sleep and meals and recheck at a routine follow-up."
            }
            (_, "low") => {
                "Below the lab reference lower bound; keep nutrition balanced and allow steady rest recovery."
            }
            _ => "Within the standard reference range. Keep up your steady sleep and balanced meals.",
        };
        return msg.to_owned();
    }

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

/// 为同一 `(profile_id, canonical_key)` 的多年多期体检记录补充纵向变化趋势（较上一期改善/回升/平稳）
pub fn enrich_lab_longitudinal_trends(mut rows: Vec<LabBiomarkerRow>) -> Vec<LabBiomarkerRow> {
    let mut by_key: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (idx, r) in rows.iter().enumerate() {
        by_key
            .entry((r.profile_id.clone(), r.canonical_key.clone()))
            .or_default()
            .push(idx);
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
        ("clinical diagnosis", "physiological trend cue"),
        ("Clinical diagnosis", "Physiological trend cue"),
        ("medical diagnosis", "wellness rhythm note"),
        ("Medical diagnosis", "Wellness rhythm note"),
        ("treatment plan", "daily wellness routine"),
        ("Treatment plan", "Daily wellness routine"),
        ("prescribe medication", "suggest nutrition and rest adjustments"),
        ("prescription", "lifestyle suggestion"),
        ("Prescription", "Lifestyle suggestion"),
        ("diagnose", "assess rhythm trends for"),
        ("diagnosis", "rhythm summary"),
        ("arrhythmia alert", "rhythm fluctuation cue"),
    ] {
        safe = safe.replace(bad, good);
    }
    safe
}

fn english_snapshot_summary(snapshot: &AnalyticsSnapshot) -> (&'static str, &'static str, &'static str) {
    let badge = if !contains_cjk(&snapshot.state_badge) && !snapshot.state_badge.trim().is_empty() {
        // Prefer deterministic static str or map by state for &'static str consistency
        match snapshot.state.as_str() {
            "prime_ready" => "Well rested",
            "recovery_priority" => "Rest gently",
            _ => "Balanced",
        }
    } else {
        match snapshot.state.as_str() {
            "prime_ready" => "Well rested",
            "recovery_priority" => "Rest gently",
            _ => "Balanced",
        }
    };
    let headline = match snapshot.state.as_str() {
        "prime_ready" => "Steady sleep and autonomic tone leave good headroom for focused work or light cardio.",
        "recovery_priority" => "Recent rest and heart rate variability suggest easing intensity and prioritizing early wind-down.",
        _ => "Daily rhythms remain steady; keep hydration, balanced meals, and consistent sleep timing.",
    };
    let micro_action = match snapshot.state.as_str() {
        "prime_ready" => "Take a 20-minute outdoor walk in daylight and keep hydration steady.",
        "recovery_priority" => "Pause vigorous sets today; stretch gently for 10 minutes and wind down 30 minutes earlier.",
        _ => "Enjoy a warm beverage after meals and aim for a calm evening screen-free window.",
    };
    (badge, headline, micro_action)
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
    build_morning_briefing_localized(
        snapshot,
        labs,
        ai_mode,
        lan_url,
        thermal_throttled,
        ram_gb,
        DEFAULT_LOCALE,
    )
}

pub fn build_morning_briefing_localized(
    snapshot: &AnalyticsSnapshot,
    labs: &[LabBiomarkerRow],
    ai_mode: &str,
    lan_url: &str,
    thermal_throttled: bool,
    ram_gb: u32,
    locale: &str,
) -> BriefingResult {
    let en = is_en_locale(locale);
    let flagged_count = labs.iter().filter(|l| l.flag != "normal").count();

    if en {
        let (fallback_badge, fallback_headline, fallback_action) = english_snapshot_summary(snapshot);
        let badge = if contains_cjk(&snapshot.state_badge) || snapshot.state_badge.trim().is_empty()
        {
            fallback_badge
        } else {
            snapshot.state_badge.as_str()
        };
        let headline = if contains_cjk(&snapshot.headline) || snapshot.headline.trim().is_empty() {
            fallback_headline
        } else {
            snapshot.headline.as_str()
        };
        let micro_action =
            if contains_cjk(&snapshot.micro_action) || snapshot.micro_action.trim().is_empty() {
                fallback_action
            } else {
                snapshot.micro_action.as_str()
            };
        let lab_hint = if flagged_count > 0 {
            format!(
                " You have {flagged_count} lab markers sitting slightly outside range—favor lighter, lower-sodium meals today."
            )
        } else {
            String::new()
        };

        if thermal_throttled && ai_mode == "on_device_local" {
            let text = enforce_wellness_guardrails(&format!(
                "Device is warm, so Qi switched smoothly to deterministic stats: today's vitality score is {} ({badge}). {headline}{lab_hint}",
                snapshot.qi_score
            ));
            return BriefingResult {
                provider_label: "Warm device · Fell back to deterministic Rust summary".to_owned(),
                degraded: true,
                text,
                disclaimer: WELLNESS_DISCLAIMER_EN.to_owned(),
            };
        }

        let (provider_label, lead) = match ai_mode {
            "on_device_local" => {
                if ram_gb < 6 {
                    (
                        "Memory <6GB guard · Deterministic baseline only".to_owned(),
                        format!(
                            "Device memory is {ram_gb}GB, skipping heavy on-device model weights; computed locally by Rust: "
                        ),
                    )
                } else {
                    (
                        "On-device local model (0 network traffic · data stays local)".to_owned(),
                        "Fully offline with zero cloud upload: ".to_owned(),
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
                    "Home LAN local model (Ollama / self-hosted · private network)".to_owned(),
                    format!("Ready via local network endpoint ({target}): "),
                )
            }
            _ => (
                "Official ZDR zero-retention cloud (de-identified feature summary only)".to_owned(),
                "Prepared via scrubbed zero-retention summary gateway: ".to_owned(),
            ),
        };

        let sleep_part = snapshot
            .sleep_last_night_hours
            .map(|h| format!("Last night's sleep was {h:.1} hours; "))
            .unwrap_or_default();

        let raw_text = format!(
            "{lead}today's vitality is {} ({badge}). {sleep_part}{headline} Gentle step: {micro_action}{lab_hint}",
            snapshot.qi_score
        );

        return BriefingResult {
            provider_label,
            degraded: false,
            text: enforce_wellness_guardrails(raw_text.trim()),
            disclaimer: WELLNESS_DISCLAIMER_EN.to_owned(),
        };
    }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiProfileUlnPoint {
    pub profile_id: String,
    pub profile_label: String,
    pub canonical_key: String,
    pub marker_name: String,
    pub test_date: String,
    pub value: f64,
    pub unit: String,
    pub reference_high: f64,
    pub uln_pct: f64,
    pub flag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiProfileRecheckReminder {
    pub profile_id: String,
    pub profile_label: String,
    pub canonical_key: String,
    pub marker_name: String,
    pub last_test_date: String,
    pub days_since_last: i64,
    pub flag: String,
    pub reminder_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentsPeaceLetter {
    pub recipient_label: String,
    pub title: String,
    pub greeting: String,
    pub body: String,
    pub highlights: Vec<String>,
    pub recheck_notes: Vec<String>,
    pub closing: String,
    pub large_print_markdown: String,
    pub e2ee_note: String,
}

pub fn profile_display_label(profile_id: &str) -> &'static str {
    profile_display_label_localized(profile_id, DEFAULT_LOCALE)
}

pub fn profile_display_label_localized(profile_id: &str, locale: &str) -> &'static str {
    if is_en_locale(locale) {
        match profile_id.trim() {
            "partner" => "Partner",
            "parents" => "Parents & elders",
            _ => "Self",
        }
    } else {
        match profile_id.trim() {
            "partner" => "伴侣",
            "parents" => "父母长辈",
            _ => "本人",
        }
    }
}

/// 跨档案 %ULN 归一化趋势与 90 天复查提醒（支持 `self` / `partner` / `parents`）。
pub fn compute_multi_profile_uln_trends(
    markers: &[LabBiomarkerRow],
) -> (Vec<MultiProfileUlnPoint>, Vec<MultiProfileRecheckReminder>) {
    compute_multi_profile_uln_trends_localized(markers, DEFAULT_LOCALE)
}

pub fn compute_multi_profile_uln_trends_localized(
    markers: &[LabBiomarkerRow],
    locale: &str,
) -> (Vec<MultiProfileUlnPoint>, Vec<MultiProfileRecheckReminder>) {
    let today = chrono::Local::now().date_naive();
    let max_marker_date = markers
        .iter()
        .filter_map(|m| NaiveDate::parse_from_str(&m.test_date, "%Y-%m-%d").ok())
        .max()
        .unwrap_or(today);
    let ref_str = max_marker_date.max(today).format("%Y-%m-%d").to_string();
    compute_multi_profile_uln_trends_at_localized(markers, &ref_str, locale)
}

/// 指定参考日期的跨档案 %ULN 归一化趋势与 90 天复查提醒。
pub fn compute_multi_profile_uln_trends_at(
    markers: &[LabBiomarkerRow],
    reference_date: &str,
) -> (Vec<MultiProfileUlnPoint>, Vec<MultiProfileRecheckReminder>) {
    compute_multi_profile_uln_trends_at_localized(markers, reference_date, DEFAULT_LOCALE)
}

pub fn compute_multi_profile_uln_trends_at_localized(
    markers: &[LabBiomarkerRow],
    reference_date: &str,
    locale: &str,
) -> (Vec<MultiProfileUlnPoint>, Vec<MultiProfileRecheckReminder>) {
    let en = is_en_locale(locale);
    let ref_date = NaiveDate::parse_from_str(reference_date.trim(), "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Local::now().date_naive());

    let mut uln_points = Vec::new();
    let mut latest_by_profile_key: BTreeMap<(String, String), &LabBiomarkerRow> = BTreeMap::new();

    for m in markers {
        let profile_id = if m.profile_id.trim().is_empty() {
            "self".to_owned()
        } else {
            m.profile_id.clone()
        };
        let marker_display = localized_marker_name(&m.canonical_key, &m.marker_name, locale);
        if let Some(hi) = m.reference_high.or(m.ref_high).filter(|v| v.is_finite() && *v > 0.0) {
            let uln_pct = (((m.value / hi) * 100.0) * 10.0).round() / 10.0;
            uln_points.push(MultiProfileUlnPoint {
                profile_id: profile_id.clone(),
                profile_label: profile_display_label_localized(&profile_id, locale).to_owned(),
                canonical_key: m.canonical_key.clone(),
                marker_name: marker_display,
                test_date: m.test_date.clone(),
                value: m.value,
                unit: m.unit.clone(),
                reference_high: hi,
                uln_pct,
                flag: m.flag.clone(),
            });
        }

        let key = (profile_id, m.canonical_key.clone());
        match latest_by_profile_key.get(&key) {
            Some(prev) if prev.test_date >= m.test_date => {}
            _ => {
                latest_by_profile_key.insert(key, m);
            }
        }
    }

    uln_points.sort_by(|a, b| {
        a.profile_id
            .cmp(&b.profile_id)
            .then_with(|| a.canonical_key.cmp(&b.canonical_key))
            .then_with(|| a.test_date.cmp(&b.test_date))
    });

    let mut reminders = Vec::new();
    for ((profile_id, canonical_key), latest) in latest_by_profile_key {
        if let Ok(last_d) = NaiveDate::parse_from_str(&latest.test_date, "%Y-%m-%d") {
            let days_since = (ref_date - last_d).num_days();
            if days_since >= 90 {
                let profile_label = profile_display_label_localized(&profile_id, locale).to_owned();
                let marker_display =
                    localized_marker_name(&latest.canonical_key, &latest.marker_name, locale);
                let reminder_text = if en {
                    format!(
                        "{profile_label}'s {marker_display} has been {days_since} days since the last test ({}) — ready for a routine 90-day follow-up.",
                        latest.test_date
                    )
                } else {
                    format!(
                        "{profile_label}的「{}」距离上次检验（{}）已过 {days_since} 天，已满 90 天复查窗口，可择期随访对照。",
                        marker_display, latest.test_date
                    )
                };
                reminders.push(MultiProfileRecheckReminder {
                    profile_id,
                    profile_label,
                    canonical_key,
                    marker_name: marker_display,
                    last_test_date: latest.test_date.clone(),
                    days_since_last: days_since,
                    flag: latest.flag.clone(),
                    reminder_text,
                });
            }
        }
    }
    reminders.sort_by(|a, b| b.days_since_last.cmp(&a.days_since_last));

    (uln_points, reminders)
}

/// 生成大字暖心、无焦虑措辞的《气韵平安家书》（E2EE 分享草稿）。
pub fn build_parents_peace_letter(
    recipient_label: &str,
    snapshot: &AnalyticsSnapshot,
    reminders: &[MultiProfileRecheckReminder],
) -> ParentsPeaceLetter {
    build_parents_peace_letter_localized(recipient_label, snapshot, reminders, DEFAULT_LOCALE)
}

pub fn build_parents_peace_letter_localized(
    recipient_label: &str,
    snapshot: &AnalyticsSnapshot,
    reminders: &[MultiProfileRecheckReminder],
    locale: &str,
) -> ParentsPeaceLetter {
    if is_en_locale(locale) {
        let raw_recipient = recipient_label.trim();
        let recipient = if raw_recipient.is_empty() || raw_recipient == "爸妈" {
            "Mom & Dad".to_owned()
        } else {
            raw_recipient.to_owned()
        };

        let (_, _, fallback_action) = english_snapshot_summary(snapshot);
        let action_text =
            if contains_cjk(&snapshot.micro_action) || snapshot.micro_action.trim().is_empty() {
                fallback_action
            } else {
                snapshot.micro_action.as_str()
            };

        let sleep_phrase = snapshot
            .sleep_last_night_hours
            .map(|h| format!("I slept {h:.1} hours last night, and "))
            .unwrap_or_default();
        let mood_phrase = match snapshot.state.as_str() {
            "prime_ready" => "energy has been bright and my daily schedule feels well-paced",
            "recovery_priority" => "I have been intentionally slowing down for extra rest and gentle walks",
            _ => "my daily routine stays calm and balanced with warm regular meals",
        };

        let mut highlights = vec![
            format!("Recent vitality: {mood_phrase} (vitality index {}).", snapshot.qi_score),
            format!("{sleep_phrase}today's small habit: {action_text}"),
        ];
        if let Some(steps) = snapshot.metrics.get("steps").and_then(|m| m.value) {
            highlights.push(format!("Daily movement: around {:.0} gentle steps.", steps));
        }

        let recheck_notes: Vec<String> = reminders
            .iter()
            .take(3)
            .map(|r| {
                format!(
                    "Gentle reminder: {}'s {} was logged {} days ago — good timing for a calm routine checkup whenever the weather is nice.",
                    r.profile_label, r.marker_name, r.days_since_last
                )
            })
            .collect();

        let title = format!("Family Peace Letter for {recipient}");
        let greeting = format!("Dear {recipient},");
        let body = enforce_wellness_guardrails(&format!(
            "Hope this letter brings warmth as the season shifts. Things are going smoothly on my side: {sleep_phrase}{mood_phrase}. I am eating on time, winding down early, and keeping track of my daily rhythm here, so please do not worry. Remember to dress warmly, sip warm water morning and evening, and enjoy a relaxed walk outdoors."
        ));
        let closing = "Wishing you warmth and peace,\n— Thinking of you always".to_owned();

        let mut md_lines = vec![
            format!("# {title}"),
            String::new(),
            greeting.clone(),
            String::new(),
            body.clone(),
            String::new(),
            "### This week's gentle notes".to_owned(),
        ];
        for h in &highlights {
            md_lines.push(format!("- {h}"));
        }
        if !recheck_notes.is_empty() {
            md_lines.push(String::new());
            md_lines.push("### Seasonal care reminders".to_owned());
            for rn in &recheck_notes {
                md_lines.push(format!("- {rn}"));
            }
        }
        md_lines.push(String::new());
        md_lines.push(closing.clone());
        md_lines.push(String::new());
        md_lines.push(format!("> {WELLNESS_DISCLAIMER_EN}"));

        return ParentsPeaceLetter {
            recipient_label: recipient,
            title,
            greeting,
            body,
            highlights,
            recheck_notes,
            closing,
            large_print_markdown: enforce_wellness_guardrails(&md_lines.join("\n")),
            e2ee_note: "Generated locally · Only shares this warm large-print summary, never raw lab values or personal IDs.".to_owned(),
        };
    }

    let recipient = if recipient_label.trim().is_empty() {
        "爸妈".to_owned()
    } else {
        recipient_label.trim().to_owned()
    };

    let sleep_phrase = snapshot
        .sleep_last_night_hours
        .map(|h| format!("昨晚睡了 {h:.1} 个小时，"))
        .unwrap_or_default();
    let mood_phrase = match snapshot.state.as_str() {
        "prime_ready" => "最近精力充沛、作息步调很稳当",
        "recovery_priority" => "这几天特意放慢了脚步、多留了些休息和散步的时间",
        _ => "最近作息温和守中、三餐都有好好照顾自己",
    };

    let mut highlights = vec![
        format!("近况气色：{mood_phrase}（今日元气参考 {} 分）。", snapshot.qi_score),
        format!("{sleep_phrase}日常小习惯：{}", snapshot.micro_action),
    ];
    if let Some(steps) = snapshot.metrics.get("steps").and_then(|m| m.value) {
        highlights.push(format!("每日活动：走动约 {:.0} 步，活动舒展筋骨。", steps));
    }

    let recheck_notes: Vec<String> = reminders
        .iter()
        .take(3)
        .map(|r| {
            format!(
                "温馨小记：{}的「{}」距上次留档已满 {} 天，这阵子天气好时可顺便预约一次常规体检复核。",
                r.profile_label, r.marker_name, r.days_since_last
            )
        })
        .collect();

    let title = format!("给{recipient}的《气韵平安家书》");
    let greeting = format!("亲爱的{recipient}：");
    let body = enforce_wellness_guardrails(&format!(
        "见信安好！最近换季天凉，我在这边一切顺遂。{sleep_phrase}{mood_phrase}。每天按时吃饭、早点歇息，手机里也把生活节律照看得妥妥当当，不用挂念我。你们在家也要记得天冷添衣、早晚多喝温水，闲时下楼散散步。"
    ));
    let closing = "顺颂冬安，常联系。\n—— 惦念你们的家人敬上".to_owned();

    let mut md_lines = vec![
        format!("# {title}"),
        String::new(),
        greeting.clone(),
        String::new(),
        body.clone(),
        String::new(),
        "### 本期平安小记".to_owned(),
    ];
    for h in &highlights {
        md_lines.push(format!("- {h}"));
    }
    if !recheck_notes.is_empty() {
        md_lines.push(String::new());
        md_lines.push("### 换季关怀备忘".to_owned());
        for rn in &recheck_notes {
            md_lines.push(format!("- {rn}"));
        }
    }
    md_lines.push(String::new());
    md_lines.push(closing.clone());
    md_lines.push(String::new());
    md_lines.push(format!("> {WELLNESS_DISCLAIMER}"));

    ParentsPeaceLetter {
        recipient_label: recipient,
        title,
        greeting,
        body,
        highlights,
        recheck_notes,
        closing,
        large_print_markdown: enforce_wellness_guardrails(&md_lines.join("\n")),
        e2ee_note: "端侧加密生成 · 仅导出您主动挑选的大字温情文本，绝不附带原始化验数值与个人证件号。".to_owned(),
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

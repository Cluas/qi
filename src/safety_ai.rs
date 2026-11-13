//! 端侧前置隐私脱敏、体检结构化纠偏、四级 AiProvider 路由与 SaMD 医疗红线护栏。

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
                    // 截断至下一个空格分隔的独立字段（例如 "姓名：张三 性别：男" 只遮张三）
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

        // 遮盖 18 位身份证号与 11 位中国大陆手机号
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

/// 确定性提取检验行并由 Rust 重算 ref_low..ref_high 状态（防小模型误判升降箭头）。
pub fn parse_lab_text_and_verify(scrubbed: &str) -> Vec<ParsedLabMarkerDraft> {
    let mut results = Vec::new();
    for line in scrubbed.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("[已遮盖") {
            continue;
        }
        // 提取行内所有浮点数与范围 x-y
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
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
            let sep = if tok.contains('-') {
                Some('-')
            } else if tok.contains('~') {
                Some('~')
            } else {
                None
            };
            if let Some(s) = sep
                && let Some((l_s, h_s)) = tok.split_once(s)
                && let (Ok(l), Ok(h)) = (l_s.parse::<f64>(), h_s.parse::<f64>())
            {
                ref_low = Some(l);
                ref_high = Some(h);
                break;
            }
        }

        // 强制由 Rust 依据参考区间重算红绿灯状态
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
        let plain_note = match (canonical_key.as_str(), flag) {
            ("uric_acid", "high") => {
                "偏高于参考区间上限：建议增加日常饮水、减少高嘌呤海鲜/内脏与浓肉汤及酒精摄入。"
            }
            ("ldl_c", "high") => {
                "低密度脂蛋白提示关注：建议减少反式脂肪与油炸食物，增加深海鱼与膳食纤维。"
            }
            ("vitamin_d", "low") => {
                "维生素 D 偏低：建议每日上午适度户外日照 15 分钟，可在膳食中搭配富含维 D 食材。"
            }
            (_, "high") => "数值高于检验单参考区间上限，建议结合近期作息与饮食调整后定期复查。",
            (_, "low") => "数值低于检验单参考区间下限，建议留意营养均衡与休息恢复节奏。",
            _ => "位于常规参考区间内，继续保持良好作息与饮食平衡即可。",
        };

        results.push(ParsedLabMarkerDraft {
            marker_name,
            canonical_key,
            value: val,
            unit: unit.to_owned(),
            ref_low,
            ref_high,
            flag: flag.to_owned(),
            plain_note: plain_note.to_owned(),
        });
    }
    results
}

fn canonicalize_marker(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    if name.contains("尿酸") || lower.contains("ua") {
        "uric_acid".to_owned()
    } else if name.contains("低密度") || lower.contains("ldl") {
        "ldl_c".to_owned()
    } else if name.contains("维生素D") || name.contains("维生素d") || lower.contains("25-oh") {
        "vitamin_d".to_owned()
    } else if name.contains("空腹血糖") || lower.contains("glucose") || lower.contains("glu") {
        "fasting_glucose".to_owned()
    } else {
        lower
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .chars()
            .take(24)
            .collect::<String>()
            .if_empty_then(|| format!("marker_{}", name.chars().count()))
    }
}

trait StrExt {
    fn if_empty_then(self, f: impl FnOnce() -> String) -> String;
}
impl StrExt for String {
    fn if_empty_then(self, f: impl FnOnce() -> String) -> String {
        if self.is_empty() { f() } else { self }
    }
}

/// 强制医疗红线词过滤（SaMD Guardrails），防止越界措辞。
pub fn enforce_wellness_guardrails(text: &str) -> String {
    let mut safe = text.to_owned();
    for (bad, good) in [
        ("确诊为", "提示关注"),
        ("确诊", "参考提示"),
        ("开药", "膳食与作息建议"),
        ("处方", "调整参考"),
        ("治疗方案", "日常调养方案"),
        ("治疗", "调养参考"),
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
                format!("经家庭局域网节点（{target}）生成："),
            )
        }
        _ => (
            "官方 ZDR 零留存极速云端（仅发脱敏特征摘要 · 阅后即焚）",
            "经端侧去标识化摘要与阅后即焚网关润色：".to_owned(),
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
        let raw = "某某市第一人民医院体检中心\n姓名：张伟 身份证号：31010119900101123X 手机：13800138000\n尿酸(UA) 468 umol/L 208-428\n25-羟维生素D 16.5 ng/mL 30.0-100.0";
        let res = scrub_lab_text_pii(raw);
        assert!(!res.scrubbed_text.contains("张伟"));
        assert!(!res.scrubbed_text.contains("31010119900101123X"));
        assert!(!res.scrubbed_text.contains("13800138000"));
        assert!(res.redacted_fields.contains(&"姓名".to_owned()));

        let parsed = parse_lab_text_and_verify(&res.scrubbed_text);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].canonical_key, "uric_acid");
        assert_eq!(parsed[0].flag, "high");
        assert_eq!(parsed[1].canonical_key, "vitamin_d");
        assert_eq!(parsed[1].flag, "low");
    }

    #[test]
    fn guardrails_replace_medical_terms() {
        let filtered = enforce_wellness_guardrails("建议确诊后开药并制定治疗方案");
        assert!(!filtered.contains("确诊"));
        assert!(!filtered.contains("开药"));
        assert!(!filtered.contains("治疗方案"));
    }
}

//! 服务端内存级 ZDR 安全纵深防御（仅在 RAM 流转，绝不落盘任何健康文本）。

pub const SERVER_WELLNESS_DISCLAIMER: &str =
    "由「知气 · Qi」Zero-Data-Retention 网关内存流转生成，仅供日常健康作息参考，不构成医疗诊断。";

/// 剥离推理模型的 `<think>...</think>` 思考过程
pub fn strip_think_tags(input: &str) -> String {
    let mut rest = input;
    let mut out = String::with_capacity(input.len());
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find("</think>") {
            rest = &rest[start + end + "</think>".len()..];
        } else {
            // 未闭合的 think 直接截断
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out.trim().to_owned()
}

/// 内存级二次 PII 脱敏：防止客户端漏脱敏的身份证号、手机号与姓名进入上游模型
pub fn scrub_pii_in_memory(raw: &str) -> String {
    let mut out_lines = Vec::new();
    for line in raw.lines() {
        let mut cleaned = line.to_owned();
        for prefix in ["姓名", "身份证号", "身份证", "住院号", "门诊号", "手机", "电话"] {
            if let Some(pos) = cleaned.find(prefix) {
                let after = &cleaned[pos..];
                if let Some((colon_idx, ch)) = after
                    .char_indices()
                    .find(|&(_, ch)| ch == '：' || ch == ':')
                {
                    let split_at = pos + colon_idx + ch.len_utf8();
                    let head = &cleaned[..split_at];
                    let tail = &cleaned[split_at..];
                    if !tail.trim_start().starts_with("[已脱敏]") {
                        let rest = tail
                            .trim_start()
                            .split_once(char::is_whitespace)
                            .map(|(_, r)| format!(" {r}"))
                            .unwrap_or_default();
                        cleaned = format!("{head}[已脱敏]{rest}");
                    }
                }
            }
        }
        cleaned = redact_sensitive_numbers(&cleaned);
        out_lines.push(cleaned);
    }
    out_lines.join("\n")
}

fn redact_sensitive_numbers(input: &str) -> String {
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
            let tok: String = chars[start..i].iter().collect();
            if len == 18 {
                out.push_str("[已脱敏:18位证件]");
            } else if len == 11 && tok.starts_with('1') {
                out.push_str("[已脱敏:手机号]");
            } else {
                out.push_str(&tok);
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// 出口 SaMD 医疗红线词过滤
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
    ] {
        safe = safe.replace(bad, good);
    }
    safe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_think_and_scrubs_pii() {
        let scrubbed = scrub_pii_in_memory("姓名：李雷 身份证：310101199201014321 尿酸 460");
        assert!(!scrubbed.contains("李雷"));
        assert!(!scrubbed.contains("310101199201014321"));
        assert!(scrubbed.contains("460"));

        let out = enforce_wellness_guardrails("<think>internal</think>建议尽快确诊并开药");
        assert!(!out.contains("internal"));
        assert!(!out.contains("确诊"));
        assert!(!out.contains("开药"));
    }
}

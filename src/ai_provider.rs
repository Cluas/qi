//! 四级可插拔 AI 推理路由（官方 ZDR 云 / 家庭局域网 Ollama / 端侧确定性兜底）。

use std::time::Duration;

use serde_json::json;

use crate::analytics::{AnalyticsSnapshot, LabBiomarkerRow};
use crate::safety_ai::{BriefingResult, WELLNESS_DISCLAIMER, enforce_wellness_guardrails};

/// 构造脱敏后的 AI 润色 Prompt（仅包含统计特征与数值白名单，严禁传入姓名/原文 PII）
pub fn build_scrubbed_briefing_prompt(
    snapshot: &AnalyticsSnapshot,
    labs: &[LabBiomarkerRow],
) -> String {
    let sleep = snapshot
        .sleep_last_night_hours
        .map(|h| format!("{h:.1}h"))
        .unwrap_or_else(|| "未采集".to_owned());
    let acwr = snapshot
        .acwr
        .map(|a| format!("{a:.2}"))
        .unwrap_or_else(|| "未采集".to_owned());
    let flagged_labs: Vec<String> = labs
        .iter()
        .filter(|l| l.flag != "normal")
        .take(3)
        .map(|l| format!("{}({}{})", l.marker_name, l.value, l.unit))
        .collect();
    let signals_joined = snapshot.signals.join("；");

    format!(
        "请用温和简洁的宋韵东方语言（90字以内）润色今日早安健康简报，严禁改写任何数字，严禁使用诊断/开药等医疗用词：\n\
         今日元气：{}/100（{}）；硬件口径：{}；昨晚睡眠：{}；ACWR：{}；关键信号：{}；建议行动：{}；近期关注体检项：{}",
        snapshot.qi_score,
        snapshot.state_badge,
        snapshot.hardware_tier_label,
        sleep,
        acwr,
        signals_joined,
        snapshot.micro_action,
        if flagged_labs.is_empty() {
            "无".to_owned()
        } else {
            flagged_labs.join("、")
        }
    )
}

/// 在端侧实时调用配置好的 AI 推理节点（Ollama / Magpie / 官方 qi-cloud ZDR）生成润色简报，
/// 并强制经过 `enforce_wellness_guardrails` 合规过滤。
pub fn generate_live_briefing(
    snapshot: &AnalyticsSnapshot,
    labs: &[LabBiomarkerRow],
    ai_mode: &str,
    lan_url: &str,
    lan_model: &str,
    cloud_url: &str,
    cloud_token: &str,
    thermal_throttled: bool,
    ram_gb: u32,
) -> Result<BriefingResult, String> {
    if ai_mode == "on_device_local" {
        if thermal_throttled {
            return Err("设备当前处于发热降频状态，已自动保持确定性精简解读".to_owned());
        }
        if ram_gb < 6 {
            return Err(format!("当前设备内存 {ram_gb}GB 低于 6GB 安全门禁阈值，跳过离线权重加载"));
        }
        return Ok(BriefingResult {
            provider_label: "纯本机端侧模型（受限 Schema 润色 · 0 流量）".to_owned(),
            degraded: false,
            text: enforce_wellness_guardrails(&format!(
                "晨安。今日元气 {} 分（{}）。{} 今晨只需专注于这一件高杠杆微行动：{}",
                snapshot.qi_score, snapshot.state_badge, snapshot.headline, snapshot.micro_action
            )),
            disclaimer: WELLNESS_DISCLAIMER.to_owned(),
        });
    }

    let prompt = build_scrubbed_briefing_prompt(snapshot, labs);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout_read(Duration::from_secs(18))
        .build();

    if ai_mode == "lan_ollama" {
        let base = lan_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return Err("请先填写家庭局域网 Ollama / 自建网关地址".to_owned());
        }
        let endpoint = if base.ends_with("/chat/completions") {
            base.to_owned()
        } else {
            format!("{base}/chat/completions")
        };
        let model = if lan_model.trim().is_empty() {
            "qwen3:latest"
        } else {
            lan_model.trim()
        };
        let payload = json!({
            "model": model,
            "temperature": 0.3,
            "max_tokens": 280,
            "messages": [
                {
                    "role": "system",
                    "content": "你是「知气 · Qi」温和克制的东方节律健康管家。严禁使用临床诊断、确诊、开药、处方字样，只做生活方式温和表达。"
                },
                {
                    "role": "user",
                    "content": format!("/no_think\n{prompt}")
                }
            ]
        });
        let resp = agent
            .post(&endpoint)
            .set("Content-Type", "application/json")
            .send_json(payload)
            .map_err(|e| format!("局域网模型连接失败 ({endpoint}): {e}"))?;
        let val: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("解析局域网模型响应失败: {e}"))?;
        let raw = val["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim();
        let cleaned = enforce_wellness_guardrails(raw);
        if cleaned.is_empty() {
            return Err("局域网模型返回为空".to_owned());
        }
        return Ok(BriefingResult {
            provider_label: format!("家庭局域网实测生成 ({model} · 数据不出内网)"),
            degraded: false,
            text: cleaned,
            disclaimer: WELLNESS_DISCLAIMER.to_owned(),
        });
    }

    // 官方 ZDR 零留存网关模式（走 qi-cloud /v1/zdr/complete）
    let c_base = cloud_url.trim().trim_end_matches('/');
    if c_base.is_empty() || cloud_token.trim().is_empty() {
        return Err("请先在「我的 → 云端控制面」连接 qi-cloud 会话或切换为局域网 Ollama".to_owned());
    }
    let endpoint = format!("{c_base}/v1/zdr/complete");
    let resp = agent
        .post(&endpoint)
        .set("Authorization", &format!("Bearer {}", cloud_token.trim()))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "task": "briefing",
            "scrubbed_prompt": prompt
        }))
        .map_err(|e| match e {
            ureq::Error::Status(429, r) => {
                let err_val: serde_json::Value = r.into_json().unwrap_or_default();
                err_val["message"]
                    .as_str()
                    .unwrap_or("本月免费云端额度已用完")
                    .to_owned()
            }
            other => format!("官方 ZDR 网关请求失败: {other}"),
        })?;
    let val: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("解析 ZDR 响应失败: {e}"))?;
    let label = val["provider_label"]
        .as_str()
        .unwrap_or("官方 ZDR 零留存网关")
        .to_owned();
    let content = enforce_wellness_guardrails(val["content"].as_str().unwrap_or(""));
    let disclaimer = val["disclaimer"]
        .as_str()
        .unwrap_or(WELLNESS_DISCLAIMER)
        .to_owned();

    Ok(BriefingResult {
        provider_label: label,
        degraded: false,
        text: content,
        disclaimer,
    })
}

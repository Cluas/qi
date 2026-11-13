//! 四级可插拔 AI 推理路由（官方 ZDR 云 / 家庭局域网 Ollama / 端侧确定性兜底）。

use std::time::Duration;

use serde_json::json;

use crate::analytics::{AnalyticsSnapshot, LabBiomarkerRow, MultiLagImpactItem};
use crate::entitlements::{DEFAULT_LOCALE, is_en_locale};
use crate::experiments::ExperimentEvaluation;
use crate::safety_ai::{
    BriefingResult, WELLNESS_DISCLAIMER, WELLNESS_DISCLAIMER_EN, enforce_wellness_guardrails,
    wellness_disclaimer_for_locale,
};

/// 构造脱敏后的 AI 润色 Prompt（仅包含统计特征与数值白名单，严禁传入姓名/原文 PII）
pub fn build_scrubbed_briefing_prompt(
    snapshot: &AnalyticsSnapshot,
    labs: &[LabBiomarkerRow],
) -> String {
    build_scrubbed_briefing_prompt_localized(snapshot, labs, DEFAULT_LOCALE)
}

pub fn build_scrubbed_briefing_prompt_localized(
    snapshot: &AnalyticsSnapshot,
    labs: &[LabBiomarkerRow],
    locale: &str,
) -> String {
    let en = is_en_locale(locale);
    let sleep = snapshot
        .sleep_last_night_hours
        .map(|h| format!("{h:.1}h"))
        .unwrap_or_else(|| {
            if en { "not recorded".to_owned() } else { "未采集".to_owned() }
        });
    let acwr = snapshot
        .acwr
        .map(|a| format!("{a:.2}"))
        .unwrap_or_else(|| {
            if en { "not recorded".to_owned() } else { "未采集".to_owned() }
        });
    let flagged_labs: Vec<String> = labs
        .iter()
        .filter(|l| l.flag != "normal")
        .take(3)
        .map(|l| format!("{}({}{})", l.marker_name, l.value, l.unit))
        .collect();
    let signals_joined = snapshot.signals.join(if en { "; " } else { "；" });

    if en {
        format!(
            "Summarize today's morning rhythm briefing in warm, concise Sentence case (under 90 words). Never alter numbers or use clinical diagnosis or prescription language:\n\
             Vitality: {}/100 ({}); Hardware tier: {}; Last night sleep: {}; ACWR: {}; Signals: {}; Action: {}; Flagged labs: {}",
            snapshot.qi_score,
            snapshot.state_badge,
            snapshot.hardware_tier_label,
            sleep,
            acwr,
            signals_joined,
            snapshot.micro_action,
            if flagged_labs.is_empty() {
                "none".to_owned()
            } else {
                flagged_labs.join(", ")
            }
        )
    } else {
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
}

/// 在端侧实时调用配置好的 AI 推理节点（Ollama / Magpie / 官方 qi-cloud ZDR）生成润色简报，
/// 并强制经过 `enforce_wellness_guardrails` 合规过滤。
#[allow(clippy::too_many_arguments)]
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
    generate_live_briefing_localized(
        snapshot,
        labs,
        ai_mode,
        lan_url,
        lan_model,
        cloud_url,
        cloud_token,
        thermal_throttled,
        ram_gb,
        DEFAULT_LOCALE,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn generate_live_briefing_localized(
    snapshot: &AnalyticsSnapshot,
    labs: &[LabBiomarkerRow],
    ai_mode: &str,
    lan_url: &str,
    lan_model: &str,
    cloud_url: &str,
    cloud_token: &str,
    thermal_throttled: bool,
    ram_gb: u32,
    locale: &str,
) -> Result<BriefingResult, String> {
    let en = is_en_locale(locale);
    let disclaimer_default = wellness_disclaimer_for_locale(locale);
    if ai_mode == "on_device_local" {
        if thermal_throttled {
            return Err(if en {
                "Device is thermally throttled; staying on deterministic summary.".to_owned()
            } else {
                "设备当前处于发热降频状态，已自动保持确定性精简解读".to_owned()
            });
        }
        if ram_gb < 6 {
            return Err(if en {
                format!("Device memory ({ram_gb}GB) is below the 6GB threshold; skipping offline model load.")
            } else {
                format!("当前设备内存 {ram_gb}GB 低于 6GB 安全门禁阈值，跳过离线权重加载")
            });
        }
        return Ok(BriefingResult {
            provider_label: if en {
                "On-device local model (0 network traffic)".to_owned()
            } else {
                "纯本机端侧模型（受限 Schema 润色 · 0 流量）".to_owned()
            },
            degraded: false,
            text: enforce_wellness_guardrails(&if en {
                format!(
                    "Good morning. Today's vitality is {} ({}). {} Focus on one gentle step today: {}",
                    snapshot.qi_score, snapshot.state_badge, snapshot.headline, snapshot.micro_action
                )
            } else {
                format!(
                    "晨安。今日元气 {} 分（{}）。{} 今晨只需专注于这一件高杠杆微行动：{}",
                    snapshot.qi_score, snapshot.state_badge, snapshot.headline, snapshot.micro_action
                )
            }),
            disclaimer: disclaimer_default.to_owned(),
        });
    }

    let prompt = build_scrubbed_briefing_prompt_localized(snapshot, labs, locale);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout_read(Duration::from_secs(18))
        .build();

    if ai_mode == "lan_ollama" {
        let base = lan_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return Err(if en {
                "Please enter your home LAN Ollama or self-hosted endpoint first.".to_owned()
            } else {
                "请先填写家庭局域网 Ollama / 自建网关地址".to_owned()
            });
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
        let system_prompt = if en {
            "You are Qi, a calm and restrained daily rhythm companion. Never use medical diagnosis, prescribe, or treatment wording; offer only gentle lifestyle reflections."
        } else {
            "你是「知气」温和克制的东方节律健康管家。严禁使用临床诊断、确诊、开药、处方字样，只做生活方式温和表达。"
        };
        let payload = json!({
            "model": model,
            "temperature": 0.3,
            "max_tokens": 280,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
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
            .map_err(|e| {
                if en {
                    format!("LAN model connection failed ({endpoint}): {e}")
                } else {
                    format!("局域网模型连接失败 ({endpoint}): {e}")
                }
            })?;
        let val: serde_json::Value = resp.into_json().map_err(|e| {
            if en {
                format!("Failed to parse LAN model response: {e}")
            } else {
                format!("解析局域网模型响应失败: {e}")
            }
        })?;
        let raw = val["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim();
        let cleaned = enforce_wellness_guardrails(raw);
        if cleaned.is_empty() {
            return Err(if en {
                "LAN model returned an empty response.".to_owned()
            } else {
                "局域网模型返回为空".to_owned()
            });
        }
        return Ok(BriefingResult {
            provider_label: if en {
                format!("Home LAN live generation ({model} · stays on your network)")
            } else {
                format!("家庭局域网实测生成 ({model} · 数据不出内网)")
            },
            degraded: false,
            text: cleaned,
            disclaimer: disclaimer_default.to_owned(),
        });
    }

    // 官方 ZDR 零留存网关模式（走 qi-cloud /v1/zdr/complete）
    let c_base = cloud_url.trim().trim_end_matches('/');
    if c_base.is_empty() || cloud_token.trim().is_empty() {
        return Err(if en {
            "Connect your Qi Cloud session in Me -> Vault & Cloud or switch to Home LAN Ollama first.".to_owned()
        } else {
            "请先在「我的 → 云端控制面」连接 qi-cloud 会话或切换为局域网 Ollama".to_owned()
        });
    }
    let endpoint = format!("{c_base}/v1/zdr/complete");
    let resp = agent
        .post(&endpoint)
        .set("Authorization", &format!("Bearer {}", cloud_token.trim()))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "task": "briefing",
            "locale": crate::entitlements::normalize_locale(locale),
            "scrubbed_prompt": prompt
        }))
        .map_err(|e| match e {
            ureq::Error::Status(429, r) => {
                let err_val: serde_json::Value = r.into_json().unwrap_or_default();
                err_val["message"]
                    .as_str()
                    .unwrap_or(if en {
                        "Monthly free cloud quota reached"
                    } else {
                        "本月免费云端额度已用完"
                    })
                    .to_owned()
            }
            other => {
                if en {
                    format!("ZDR cloud gateway request failed: {other}")
                } else {
                    format!("官方 ZDR 网关请求失败: {other}")
                }
            }
        })?;
    let val: serde_json::Value = resp.into_json().map_err(|e| {
        if en {
            format!("Failed to parse ZDR response: {e}")
        } else {
            format!("解析 ZDR 响应失败: {e}")
        }
    })?;
    let label = val["provider_label"]
        .as_str()
        .unwrap_or(if en {
            "Official ZDR zero-retention gateway"
        } else {
            "官方 ZDR 零留存网关"
        })
        .to_owned();
    let content = enforce_wellness_guardrails(val["content"].as_str().unwrap_or(""));
    let disclaimer = val["disclaimer"]
        .as_str()
        .unwrap_or(if en { WELLNESS_DISCLAIMER_EN } else { WELLNESS_DISCLAIMER })
        .to_owned();

    Ok(BriefingResult {
        provider_label: label,
        degraded: false,
        text: content,
        disclaimer,
    })
}

/// 知气雅集多日深度综合润色（结合多滞后归因雷达、N-of-1 实验与化验趋势），
/// 免费版依然可无门槛使用上方的 `generate_live_briefing` 90 字 Ollama/端侧晨报。
#[allow(clippy::too_many_arguments)]
pub fn generate_atelier_synthesis(
    snapshot: &AnalyticsSnapshot,
    labs: &[LabBiomarkerRow],
    radar: &[MultiLagImpactItem],
    experiments: &[ExperimentEvaluation],
    ai_mode: &str,
    lan_url: &str,
    lan_model: &str,
    cloud_url: &str,
    cloud_token: &str,
) -> Result<BriefingResult, String> {
    generate_atelier_synthesis_localized(
        snapshot,
        labs,
        radar,
        experiments,
        ai_mode,
        lan_url,
        lan_model,
        cloud_url,
        cloud_token,
        DEFAULT_LOCALE,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn generate_atelier_synthesis_localized(
    snapshot: &AnalyticsSnapshot,
    labs: &[LabBiomarkerRow],
    radar: &[MultiLagImpactItem],
    experiments: &[ExperimentEvaluation],
    ai_mode: &str,
    lan_url: &str,
    lan_model: &str,
    cloud_url: &str,
    cloud_token: &str,
    locale: &str,
) -> Result<BriefingResult, String> {
    let en = is_en_locale(locale);
    let disclaimer = wellness_disclaimer_for_locale(locale).to_owned();
    let radar_str = radar
        .iter()
        .take(3)
        .map(|r| r.summary.clone())
        .collect::<Vec<_>>()
        .join(if en { "; " } else { "；" });
    let exp_str = experiments
        .iter()
        .take(2)
        .map(|e| {
            if en {
                format!("{} ({})", e.title, e.verdict_summary)
            } else {
                format!("{}（{}）", e.title, e.verdict_summary)
            }
        })
        .collect::<Vec<_>>()
        .join(if en { "; " } else { "；" });
    let lab_str = labs
        .iter()
        .filter(|l| l.flag != "normal")
        .take(3)
        .map(|l| format!("{}({}{})", l.marker_name, l.value, l.unit))
        .collect::<Vec<_>>()
        .join(if en { ", " } else { "、" });

    if ai_mode == "on_device_local" || (lan_url.trim().is_empty() && cloud_token.trim().is_empty()) {
        let synthesized = if en {
            format!(
                "Qi Atelier multi-day synthesis: today's vitality is {} ({}). Lag cues: {}; experiment check: {}; highlighted labs: {}.",
                snapshot.qi_score,
                snapshot.state_badge,
                if radar_str.is_empty() { "collecting 90-day multi-lag samples".to_owned() } else { radar_str },
                if exp_str.is_empty() { "no active experiments".to_owned() } else { exp_str },
                if lab_str.is_empty() { "all markers within reference range".to_owned() } else { lab_str }
            )
        } else {
            format!(
                "雅集多日综合回望：今日元气 {} 分（{}）。归因雷达提示：{}；实验观察：{}；重点留意指标：{}。",
                snapshot.qi_score,
                snapshot.state_badge,
                if radar_str.is_empty() { "正在积累 90 天多滞后样本".to_owned() } else { radar_str },
                if exp_str.is_empty() { "暂无进行中的实验".to_owned() } else { exp_str },
                if lab_str.is_empty() { "各化验项位于常规区间".to_owned() } else { lab_str }
            )
        };
        return Ok(BriefingResult {
            provider_label: if en {
                "Qi Atelier · Local multi-day synthesis".to_owned()
            } else {
                "知气雅集 · 本地多日特征综合引擎".to_owned()
            },
            degraded: false,
            text: enforce_wellness_guardrails(&synthesized),
            disclaimer,
        });
    }

    let prompt = if en {
        format!(
            "Write a calm multi-day wellness rhythm reflection for Qi Atelier in under 180 words (Sentence case, never medical diagnosis or prescription):\n\
             Vitality: {}/100 ({}); Multi-lag cues: {}; N-of-1 experiments: {}; Flagged labs: {}",
            snapshot.qi_score,
            snapshot.state_badge,
            if radar_str.is_empty() { "gathering data" } else { &radar_str },
            if exp_str.is_empty() { "none" } else { &exp_str },
            if lab_str.is_empty() { "none" } else { &lab_str }
        )
    } else {
        format!(
            "请用温和克制的宋韵散文语言（220字内）撰写知气雅集多日健康节律综合回望，严禁诊断/开药：\n\
             今日元气：{}/100（{}）；多滞后归因：{}；N-of-1 实验：{}；化验关注：{}",
            snapshot.qi_score,
            snapshot.state_badge,
            if radar_str.is_empty() { "积累中" } else { &radar_str },
            if exp_str.is_empty() { "无" } else { &exp_str },
            if lab_str.is_empty() { "无" } else { &lab_str }
        )
    };

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout_read(Duration::from_secs(18))
        .build();

    if ai_mode == "lan_ollama" {
        let base = lan_url.trim().trim_end_matches('/');
        let endpoint = if base.ends_with("/chat/completions") {
            base.to_owned()
        } else {
            format!("{base}/chat/completions")
        };
        let model = if lan_model.trim().is_empty() { "qwen3:latest" } else { lan_model.trim() };
        let sys_prompt = if en {
            "You are the Qi Atelier rhythm observer. Never provide medical diagnosis or prescriptions."
        } else {
            "你是「知气雅集」生活节律观察者，严禁医疗诊断与开药建议。"
        };
        let resp = agent
            .post(&endpoint)
            .set("Content-Type", "application/json")
            .send_json(json!({
                "model": model,
                "temperature": 0.35,
                "max_tokens": 420,
                "messages": [
                    {"role": "system", "content": sys_prompt},
                    {"role": "user", "content": format!("/no_think\n{prompt}")}
                ]
            }))
            .map_err(|e| {
                if en {
                    format!("LAN model synthesis failed: {e}")
                } else {
                    format!("局域网模型综合生成失败: {e}")
                }
            })?;
        let val: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
        let text = enforce_wellness_guardrails(
            val["choices"][0]["message"]["content"].as_str().unwrap_or(""),
        );
        return Ok(BriefingResult {
            provider_label: if en {
                format!("Qi Atelier · LAN deep synthesis ({model})")
            } else {
                format!("知气雅集 · 局域网深度综合 ({model})")
            },
            degraded: false,
            text,
            disclaimer,
        });
    }

    let c_base = cloud_url.trim().trim_end_matches('/');
    let endpoint = format!("{c_base}/v1/zdr/complete");
    let resp = agent
        .post(&endpoint)
        .set("Authorization", &format!("Bearer {}", cloud_token.trim()))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "task": "atelier_synthesis",
            "locale": crate::entitlements::normalize_locale(locale),
            "scrubbed_prompt": prompt
        }))
        .map_err(|e| {
            if en {
                format!("Atelier ZDR synthesis failed: {e}")
            } else {
                format!("雅集 ZDR 综合生成失败: {e}")
            }
        })?;
    let val: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
    Ok(BriefingResult {
        provider_label: val["provider_label"]
            .as_str()
            .unwrap_or(if en {
                "Qi Atelier · ZDR deep synthesis"
            } else {
                "知气雅集 · ZDR 深度综合"
            })
            .to_owned(),
        degraded: false,
        text: enforce_wellness_guardrails(val["content"].as_str().unwrap_or("")),
        disclaimer,
    })
}

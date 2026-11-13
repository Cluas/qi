//! Apple 原生离线 Vision OCR 与 FoundationModels 系统端侧大模型探测器桥接。

use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAiCapabilityProbe {
    pub platform: String,
    pub vision_ocr_available: bool,
    pub vision_revision: i32,
    pub foundation_models_supported: bool,
    pub foundation_models_status: String,
    pub ollama_reachable: bool,
    pub ollama_models: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SwiftProbeJson {
    platform: String,
    #[serde(rename = "visionOcrAvailable")]
    vision_ocr_available: bool,
    #[serde(rename = "visionRevision")]
    vision_revision: i32,
    #[serde(rename = "foundationModelsSupported")]
    foundation_models_supported: bool,
    #[serde(rename = "foundationModelsStatus")]
    foundation_models_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SwiftOcrJson {
    ok: bool,
    text: String,
    #[serde(rename = "lineCount")]
    line_count: usize,
    error: Option<String>,
}

fn find_helper_binary() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(explicit) = std::env::var("QI_VISION_HELPER") {
        candidates.push(PathBuf::from(explicit));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("qi-vision-helper"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("bin").join("qi-vision-helper"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".qi").join("bin").join("qi-vision-helper"));
    }
    candidates.into_iter().find(|p| p.is_file())
}

pub fn probe_ollama_models(lan_url: &str) -> (bool, Vec<String>) {
    let trimmed = lan_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return (false, Vec::new());
    }
    let root = trimmed.strip_suffix("/v1").unwrap_or(trimmed);
    let tags_url = format!("{root}/api/tags");
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_millis(600))
        .timeout_read(std::time::Duration::from_millis(900))
        .build();
    if let Ok(resp) = agent.get(&tags_url).call()
        && let Ok(val) = resp.into_json::<serde_json::Value>()
        && let Some(arr) = val["models"].as_array()
    {
        let names = arr
            .iter()
            .filter_map(|m| m["name"].as_str().map(|s| s.to_owned()))
            .collect::<Vec<_>>();
        return (true, names);
    }
    (false, Vec::new())
}

pub fn probe_local_ai_capabilities(lan_url: &str) -> LocalAiCapabilityProbe {
    let (ollama_reachable, ollama_models) = probe_ollama_models(lan_url);

    if let Some(helper) = find_helper_binary()
        && let Ok(out) = Command::new(helper).arg("probe").output()
        && out.status.success()
        && let Ok(parsed) = serde_json::from_slice::<SwiftProbeJson>(&out.stdout)
    {
        let fm_note = if parsed.foundation_models_supported {
            "Apple FoundationModels 可用（0MB 端侧推理就绪）".to_owned()
        } else {
            format!("FoundationModels 未激活 ({})", parsed.foundation_models_status)
        };
        let ollama_note = if ollama_reachable {
            format!("Ollama 在线 [{}]", ollama_models.join(", "))
        } else {
            "Ollama 未连接".to_owned()
        };
        return LocalAiCapabilityProbe {
            platform: parsed.platform,
            vision_ocr_available: parsed.vision_ocr_available,
            vision_revision: parsed.vision_revision,
            foundation_models_supported: parsed.foundation_models_supported,
            foundation_models_status: parsed.foundation_models_status,
            ollama_reachable,
            ollama_models,
            summary: format!(
                "Vision 离线 OCR Rev.{} 就绪 | {} | {}",
                parsed.vision_revision, fm_note, ollama_note
            ),
        };
    }

    let ollama_note = if ollama_reachable {
        format!("局域网 Ollama 在线 [{}]", ollama_models.join(", "))
    } else {
        "局域网 Ollama 未连接".to_owned()
    };
    LocalAiCapabilityProbe {
        platform: std::env::consts::OS.to_owned(),
        vision_ocr_available: false,
        vision_revision: 0,
        foundation_models_supported: false,
        foundation_models_status: "helper_not_installed".to_owned(),
        ollama_reachable,
        ollama_models,
        summary: format!(
            "端侧基础探测（{}）| {} | Vision 需在 Mac/iOS 宿主运行",
            std::env::consts::OS,
            ollama_note
        ),
    }
}

/// 调用 Apple Vision (`VNRecognizeTextRequest`) 对本地体检单图片执行离线 OCR（0 数据出机）
pub fn run_offline_vision_ocr(image_path: &str) -> Result<String, String> {
    let helper = find_helper_binary().ok_or_else(|| {
        "未找到 qi-vision-helper，请先运行 hosts/macos/build_helper.sh 或直接粘贴体检文本".to_owned()
    })?;
    let output = Command::new(helper)
        .arg("ocr")
        .arg(image_path)
        .output()
        .map_err(|e| format!("启动 Vision OCR 失败: {e}"))?;

    if let Ok(parsed) = serde_json::from_slice::<SwiftOcrJson>(&output.stdout) {
        if parsed.ok {
            return Ok(parsed.text);
        }
        return Err(parsed.error.unwrap_or_else(|| "Vision OCR 未识别到文字".to_owned()));
    }
    Err(format!(
        "Vision OCR 返回异常 (exit {:?}): {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_capabilities_without_panic() {
        let p = probe_local_ai_capabilities("http://127.0.0.1:19999/v1");
        assert!(!p.platform.is_empty());
        assert!(!p.summary.is_empty());
    }
}

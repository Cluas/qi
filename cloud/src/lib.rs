//! 「知气 · Qi」官方零健康库云端控制面与无状态 ZDR AI 网关路由实现。

pub mod db;
pub mod guardrails;

use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::cors::CorsLayer;

use crate::db::{AccountIdentity, ControlPlaneDb, SyncEnvelopeInput};
use crate::guardrails::{
    SERVER_WELLNESS_DISCLAIMER, enforce_wellness_guardrails, scrub_pii_in_memory,
};

#[derive(Debug, Clone)]
pub struct ZdrUpstreamConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<ControlPlaneDb>,
    pub zdr_upstream: Option<ZdrUpstreamConfig>,
}

#[derive(Debug, Deserialize)]
pub struct AuthSessionRequest {
    pub handle: String,
}

#[derive(Debug, Deserialize)]
pub struct ActivateProRequest {
    pub tier: Option<String>,
    pub promo_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ZdrCompleteRequest {
    pub task: String,
    pub scrubbed_prompt: String,
    pub system_hint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZdrCompleteResponse {
    pub provider_label: String,
    pub upstream_used: bool,
    pub content: String,
    pub disclaimer: String,
}

#[derive(Debug, Deserialize)]
pub struct PullQuery {
    pub after_id: Option<i64>,
    pub exclude_device: Option<String>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/auth/session", post(auth_session))
        .route("/v1/entitlements", get(get_entitlements))
        .route("/v1/billing/activate-pro", post(activate_pro))
        .route("/v1/account", delete(delete_account))
        .route("/v1/zdr/complete", post(zdr_complete))
        .route("/v1/sync/ops/push", post(sync_push))
        .route("/v1/sync/ops/pull", get(sync_pull))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

fn json_error(status: StatusCode, code: &str, msg: &str) -> Response {
    (
        status,
        Json(json!({
            "error": code,
            "message": msg
        })),
    )
        .into_response()
}

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get("authorization")?.to_str().ok()?;
    raw.strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
        .map(|s| s.trim().to_owned())
}

fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<AccountIdentity, Response> {
    let Some(token) = extract_bearer(headers) else {
        return Err(json_error(
            StatusCode::UNAUTHORIZED,
            "missing_bearer",
            "缺少 Authorization Bearer 登录凭证",
        ));
    };
    state
        .db
        .authenticate(&token)
        .map_err(|msg| json_error(StatusCode::UNAUTHORIZED, "invalid_token", &msg))
}

async fn healthz() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "qi-cloud",
        "version": "1.0.0",
        "health_plaintext_retained": false
    }))
}

async fn auth_session(
    State(state): State<AppState>,
    Json(req): Json<AuthSessionRequest>,
) -> Response {
    match state.db.create_or_rotate_session(&req.handle) {
        Ok((identity, token, entitlements)) => (
            StatusCode::OK,
            Json(json!({
                "account_id": identity.account_id,
                "handle": identity.handle,
                "plan_tier": identity.plan_tier,
                "token": token,
                "entitlements": entitlements
            })),
        )
            .into_response(),
        Err(msg) => json_error(StatusCode::BAD_REQUEST, "invalid_handle", &msg),
    }
}

async fn get_entitlements(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let identity = match require_auth(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match state.db.get_entitlements(&identity) {
        Ok(ent) => (StatusCode::OK, Json(ent)).into_response(),
        Err(msg) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error", &msg),
    }
}

async fn activate_pro(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ActivateProRequest>,
) -> Response {
    let identity = match require_auth(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let target_tier = match req.tier.as_deref() {
        Some("free") => "free",
        Some("pro") => "pro",
        _ => match req.promo_code.as_deref() {
            Some("QI-FREE") => "free",
            _ => "pro",
        },
    };
    match state.db.set_plan_tier(&identity, target_tier) {
        Ok(ent) => (StatusCode::OK, Json(ent)).into_response(),
        Err(msg) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error", &msg),
    }
}

async fn delete_account(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let identity = match require_auth(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match state.db.delete_account_physical(&identity.account_id) {
        Ok(receipt) => (StatusCode::OK, Json(receipt)).into_response(),
        Err(msg) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "wipe_failed", &msg),
    }
}

fn call_upstream_zdr_blocking(
    cfg: &ZdrUpstreamConfig,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, String> {
    let endpoint = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout_read(Duration::from_secs(12))
        .build();

    let payload = json!({
        "model": cfg.model,
        "temperature": 0.3,
        "max_tokens": 360,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": format!("/no_think\n{user_prompt}") }
        ]
    });

    let mut req = agent
        .post(&endpoint)
        .set("Content-Type", "application/json");
    if !cfg.api_key.trim().is_empty() {
        req = req.set("Authorization", &format!("Bearer {}", cfg.api_key.trim()));
    }

    let resp = req
        .send_json(payload)
        .map_err(|e| format!("上游 ZDR 请求失败: {e}"))?;
    let body: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("上游 JSON 解析失败: {e}"))?;
    let text = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_owned();
    if text.is_empty() {
        return Err("上游返回空文本".to_owned());
    }
    Ok(text)
}

async fn zdr_complete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ZdrCompleteRequest>,
) -> Response {
    let identity = match require_auth(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let task = match req.task.as_str() {
        "intake" => "intake",
        "lab" => "lab",
        _ => "briefing",
    };

    match state.db.try_consume_zdr_quota(&identity, task) {
        Ok(true) => {}
        Ok(false) => {
            return json_error(
                StatusCode::TOO_MANY_REQUESTS,
                "quota_exceeded",
                "本月免费云端额度已用完，可升级 Pro 或免费切换纯本机/家庭局域网模型",
            );
        }
        Err(msg) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, "quota_error", &msg),
    }

    // 内存级二次脱敏（绝不落盘）
    let scrubbed_input = scrub_pii_in_memory(&req.scrubbed_prompt);
    let system_prompt = req.system_hint.unwrap_or_else(|| {
        "你是「知气 · Qi」温和克制的东方节律健康助手。严禁使用诊断、确诊、开药、处方等医疗用词，只基于给定已核对的统计特征给出日常作息与饮食微调建议，控制在 120 字以内。".to_owned()
    });

    let mut upstream_used = false;
    let mut provider_label = "官方 ZDR 零留存网关（内存级脱敏·确定性回退）".to_owned();

    let raw_reply = if let Some(cfg) = state.zdr_upstream.clone() {
        let sys_c = system_prompt.clone();
        let usr_c = scrubbed_input.clone();
        match tokio::task::spawn_blocking(move || call_upstream_zdr_blocking(&cfg, &sys_c, &usr_c))
            .await
        {
            Ok(Ok(txt)) => {
                upstream_used = true;
                provider_label = format!("官方 ZDR 零留存网关（已接入 {} · 阅后即焚）", state.zdr_upstream.as_ref().map(|c| c.model.as_str()).unwrap_or("LLM"));
                txt
            }
            _ => format!(
                "已通过内存级 ZDR 去标识化校验（未触发外发）。基于你提供的脱敏摘要：{scrubbed_input}。建议顺应今日身体节律稳健安排训练强度，并在睡前 2 小时避免重餐与咖啡因。"
            ),
        }
    } else {
        format!(
            "已通过 ZDR 内存流转核验（零落盘）。根据脱敏特征：{scrubbed_input}，建议保持规律作息与温和补水，优先完成今日唯一高杠杆微行动。"
        )
    };

    let safe_reply = enforce_wellness_guardrails(&raw_reply);
    let mut resp = (
        StatusCode::OK,
        Json(ZdrCompleteResponse {
            provider_label,
            upstream_used,
            content: safe_reply,
            disclaimer: SERVER_WELLNESS_DISCLAIMER.to_owned(),
        }),
    )
        .into_response();

    resp.headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    resp.headers_mut().insert(
        "x-qi-zdr",
        HeaderValue::from_static("memory-only-zero-retention"),
    );
    resp
}

async fn sync_push(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(env): Json<SyncEnvelopeInput>,
) -> Response {
    let identity = match require_auth(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match state.db.push_sync_envelope(&identity.account_id, &env) {
        Ok(id) => (
            StatusCode::OK,
            Json(json!({
                "status": "stored_ciphertext_only",
                "envelope_id": id,
                "path": env.path
            })),
        )
            .into_response(),
        Err(msg) => json_error(StatusCode::BAD_REQUEST, "invalid_e2ee_envelope", &msg),
    }
}

async fn sync_pull(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<PullQuery>,
) -> Response {
    let identity = match require_auth(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let after_id = q.after_id.unwrap_or(0);
    match state
        .db
        .pull_sync_envelopes(&identity.account_id, after_id, q.exclude_device.as_deref())
    {
        Ok((envelopes, cursor_id)) => (
            StatusCode::OK,
            Json(json!({
                "cursor_id": cursor_id,
                "count": envelopes.len(),
                "envelopes": envelopes
            })),
        )
            .into_response(),
        Err(msg) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "sync_pull_failed", &msg),
    }
}

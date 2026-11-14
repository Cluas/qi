//! 「知气 (Qi)」官方零健康库云端控制面与无状态 ZDR AI 网关路由实现。

pub mod db;
pub mod guardrails;

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::cors::CorsLayer;

use crate::db::{
    AccountIdentity, AppleVerifyError, AppleVerifyRequest, ControlPlaneDb, ControlPlaneError,
    SubscriptionTier, SyncEnvelopeInput,
};
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

struct SlidingWindowRateLimiter {
    max_per_window: usize,
    window: Duration,
    buckets: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl SlidingWindowRateLimiter {
    fn new(max_per_window: usize, window: Duration) -> Self {
        Self {
            max_per_window: max_per_window.max(1),
            window,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    fn check_and_record(&self, key: &str) -> bool {
        let now = Instant::now();
        let Ok(mut guard) = self.buckets.lock() else {
            return true;
        };
        if guard.len() > 4096 {
            guard.retain(|_, q| {
                while let Some(&front) = q.front() {
                    if now.duration_since(front) >= self.window {
                        q.pop_front();
                    } else {
                        break;
                    }
                }
                !q.is_empty()
            });
        }
        let deque = guard.entry(key.to_owned()).or_default();
        while let Some(&front) = deque.front() {
            if now.duration_since(front) >= self.window {
                deque.pop_front();
            } else {
                break;
            }
        }
        if deque.len() >= self.max_per_window {
            return false;
        }
        deque.push_back(now);
        true
    }
}

#[derive(Clone)]
struct RouterState {
    app: AppState,
    rate_limiter: Arc<SlidingWindowRateLimiter>,
}

#[derive(Debug, Deserialize)]
pub struct AuthSessionRequest {
    pub handle: String,
    pub auth_proof_hex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ActivateProRequest {
    pub tier: Option<String>,
    pub promo_code: Option<String>,
    pub guardian_sponsor: Option<bool>,
    pub companion_loyalty_days: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MockUpgradeRequest {
    pub tier: Option<String>,
    pub guardian_sponsor: Option<bool>,
    pub companion_loyalty_days: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CoPairRequest {
    pub pair_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CoMoodRequest {
    pub mood: i64,
    pub outfit_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CoCareRequest {
    pub care_action: String,
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

fn env_rate_limit_per_min() -> usize {
    std::env::var("QI_RATE_LIMIT_PER_MIN")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(60)
}

fn build_cors_layer(production_mode: bool) -> CorsLayer {
    if !production_mode {
        return CorsLayer::permissive();
    }
    let allowed_origins: Vec<HeaderValue> = std::env::var("QI_CORS_ALLOWED_ORIGINS")
        .ok()
        .map(|raw| {
            raw.split(',')
                .filter_map(|part| {
                    let trimmed = part.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        trimmed.parse::<HeaderValue>().ok()
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let base = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);
    if allowed_origins.is_empty() {
        base
    } else {
        base.allow_origin(allowed_origins)
    }
}

pub fn build_router(state: AppState) -> Router {
    build_router_with_rate_limit(state, env_rate_limit_per_min())
}

pub fn build_router_with_rate_limit(state: AppState, max_per_min: usize) -> Router {
    let cors = build_cors_layer(state.db.is_production_mode());
    let router_state = RouterState {
        app: state,
        rate_limiter: Arc::new(SlidingWindowRateLimiter::new(
            max_per_min,
            Duration::from_secs(60),
        )),
    };
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/auth/session", post(auth_session))
        .route("/v1/entitlements", get(get_entitlements))
        .route("/v1/entitlements/refresh", post(refresh_entitlements))
        .route("/v1/billing/activate-pro", post(activate_pro))
        .route("/v1/billing/mock-upgrade", post(mock_upgrade))
        .route("/v1/billing/apple/verify", post(apple_verify))
        .route("/v1/cosanctuary/pair", post(co_pair))
        .route("/v1/cosanctuary/mood", post(co_mood))
        .route("/v1/cosanctuary/care", post(co_care))
        .route("/v1/cosanctuary/status", get(co_status))
        .route("/v1/account", delete(delete_account))
        .route("/v1/zdr/complete", post(zdr_complete))
        .route("/v1/sync/ops/push", post(sync_push))
        .route("/v1/sync/ops/pull", get(sync_pull))
        .layer(cors)
        .with_state(router_state)
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

fn control_plane_error_response(err: ControlPlaneError) -> Response {
    match err {
        ControlPlaneError::Unauthorized(code, msg) => json_error(StatusCode::UNAUTHORIZED, code, &msg),
        ControlPlaneError::Forbidden(code, msg) => json_error(StatusCode::FORBIDDEN, code, &msg),
        ControlPlaneError::BadRequest(code, msg) => json_error(StatusCode::BAD_REQUEST, code, &msg),
        ControlPlaneError::PayloadTooLarge(code, msg) => {
            json_error(StatusCode::PAYLOAD_TOO_LARGE, code, &msg)
        }
        ControlPlaneError::Internal(msg) => {
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error", &msg)
        }
    }
}

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get("authorization")?.to_str().ok()?;
    raw.strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
        .map(|s| s.trim().to_owned())
}

fn extract_client_hint(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("direct")
        .to_owned()
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
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(req): Json<AuthSessionRequest>,
) -> Response {
    let client_hint = extract_client_hint(&headers);
    if !state
        .rate_limiter
        .check_and_record(&format!("auth:{client_hint}"))
    {
        return json_error(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limit_exceeded",
            "会话请求过于频繁，请稍后再试",
        );
    }
    let bearer = extract_bearer(&headers);
    match state.app.db.create_or_rotate_session_with_auth(
        &req.handle,
        bearer.as_deref(),
        req.auth_proof_hex.as_deref(),
    ) {
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
        Err(err) => control_plane_error_response(err),
    }
}

async fn get_entitlements(State(state): State<RouterState>, headers: HeaderMap) -> Response {
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match state.app.db.get_entitlements(&identity) {
        Ok(ent) => (StatusCode::OK, Json(ent)).into_response(),
        Err(msg) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error", &msg),
    }
}

async fn refresh_entitlements(State(state): State<RouterState>, headers: HeaderMap) -> Response {
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match state.app.db.refresh_entitlements(&identity) {
        Ok(ent) => (StatusCode::OK, Json(ent)).into_response(),
        Err(msg) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error", &msg),
    }
}

async fn activate_pro(
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(req): Json<ActivateProRequest>,
) -> Response {
    if state.app.db.is_production_mode() {
        return json_error(
            StatusCode::FORBIDDEN,
            "mock_billing_disabled_in_production",
            "生产模式已永久关闭非苹果内购测试升级接口",
        );
    }
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let target_tier = match req.tier.as_deref().map(str::trim) {
        Some("free") => "free",
        Some("pro") => "pro",
        Some(other) => SubscriptionTier::from_str_checked(other)
            .map(|t| t.as_str())
            .unwrap_or("pro"),
        None => match req.promo_code.as_deref() {
            Some("QI-FREE") => "free",
            _ => "pro",
        },
    };
    match state.app.db.upgrade_plan(
        &identity,
        target_tier,
        req.guardian_sponsor,
        req.companion_loyalty_days,
    ) {
        Ok(ent) => (StatusCode::OK, Json(ent)).into_response(),
        Err(msg) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error", &msg),
    }
}

async fn mock_upgrade(
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(req): Json<MockUpgradeRequest>,
) -> Response {
    if state.app.db.is_production_mode() {
        return json_error(
            StatusCode::FORBIDDEN,
            "mock_billing_disabled_in_production",
            "生产模式已永久关闭模拟订阅接口",
        );
    }
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let stored_tier = match req.tier.as_deref().map(str::trim) {
        None | Some("") => "pro_annual",
        Some(raw) => match SubscriptionTier::from_str_checked(raw) {
            Some(t) => t.as_str(),
            None => {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_tier",
                    "不支持的订阅档位（支持 free/pro_monthly/seasonal/pro_annual/duo/family/lifetime_local）",
                );
            }
        },
    };
    match state.app.db.upgrade_plan(
        &identity,
        stored_tier,
        req.guardian_sponsor,
        req.companion_loyalty_days,
    ) {
        Ok(ent) => (StatusCode::OK, Json(ent)).into_response(),
        Err(msg) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error", &msg),
    }
}

async fn apple_verify(
    State(state): State<RouterState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let req: AppleVerifyRequest = match serde_json::from_slice(&body) {
        Ok(parsed) => parsed,
        Err(_) => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "invalid_json",
                "Apple 交易校验请求体必须为合法 JSON",
            );
        }
    };
    match state.app.db.verify_apple_purchase(&identity, &req) {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(AppleVerifyError::BadRequest(code, msg)) => {
            json_error(StatusCode::BAD_REQUEST, code, &msg)
        }
        Err(AppleVerifyError::Conflict(code, msg)) => {
            json_error(StatusCode::CONFLICT, code, &msg)
        }
        Err(AppleVerifyError::Internal(msg)) => {
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error", &msg)
        }
    }
}

async fn co_pair(State(state): State<RouterState>, headers: HeaderMap, body: Bytes) -> Response {
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let req: CoPairRequest = if body.is_empty() {
        CoPairRequest::default()
    } else {
        match serde_json::from_slice(&body) {
            Ok(parsed) => parsed,
            Err(_) => {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_json",
                    "请求体必须为合法 JSON",
                );
            }
        }
    };
    match state
        .app
        .db
        .cosanctuary_pair(&identity.account_id, req.pair_code.as_deref())
    {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(err) => control_plane_error_response(err),
    }
}

async fn co_mood(
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(req): Json<CoMoodRequest>,
) -> Response {
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if !(0..=2).contains(&req.mood) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "invalid_mood",
            "mood 必须为 0 (SUNNY_PRIME)、1 (BREEZY_STEADY) 或 2 (COZY_RECOVER)",
        );
    }
    match state
        .app
        .db
        .cosanctuary_set_mood(&identity.account_id, req.mood, req.outfit_id.as_deref())
    {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(err) => control_plane_error_response(err),
    }
}

async fn co_care(
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(req): Json<CoCareRequest>,
) -> Response {
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match state
        .app
        .db
        .cosanctuary_send_care(&identity.account_id, &req.care_action)
    {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(err) => control_plane_error_response(err),
    }
}

async fn co_status(State(state): State<RouterState>, headers: HeaderMap) -> Response {
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match state.app.db.cosanctuary_status(&identity.account_id) {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(msg) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error", &msg),
    }
}

async fn delete_account(State(state): State<RouterState>, headers: HeaderMap) -> Response {
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match state.app.db.delete_account_physical(&identity.account_id) {
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
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(req): Json<ZdrCompleteRequest>,
) -> Response {
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    if !state
        .rate_limiter
        .check_and_record(&format!("zdr:{}", identity.account_id))
    {
        return json_error(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limit_exceeded",
            "ZDR 推理请求过于频繁，请稍后再试",
        );
    }

    let task = match req.task.as_str() {
        "intake" => "intake",
        "lab" => "lab",
        "atelier_synthesis" => "atelier_synthesis",
        _ => "briefing",
    };

    match state.app.db.try_consume_zdr_quota(&identity, task) {
        Ok(true) => {}
        Ok(false) => {
            let msg = if task == "atelier_synthesis" {
                "Atelier 综合推理需要有效付费会员权益，当前账号不可用或已过期"
            } else {
                "本月免费云端额度已用完，可升级 Pro 或免费切换纯本机/家庭局域网模型"
            };
            return json_error(StatusCode::TOO_MANY_REQUESTS, "quota_exceeded", msg);
        }
        Err(msg) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, "quota_error", &msg),
    }

    // 内存级二次脱敏（绝不落盘）
    let scrubbed_input = scrub_pii_in_memory(&req.scrubbed_prompt);
    let system_prompt = req.system_hint.unwrap_or_else(|| {
        "你是「知气」温和克制的东方节律健康助手。严禁使用诊断、确诊、开药、处方等医疗用词，只基于给定已核对的统计特征给出日常作息与饮食微调建议，控制在 120 字以内。".to_owned()
    });

    let mut upstream_used = false;
    let mut provider_label = "官方 ZDR 零留存网关（内存级脱敏·确定性回退）".to_owned();

    let raw_reply = if let Some(cfg) = state.app.zdr_upstream.clone() {
        let sys_c = system_prompt.clone();
        let usr_c = scrubbed_input.clone();
        match tokio::task::spawn_blocking(move || call_upstream_zdr_blocking(&cfg, &sys_c, &usr_c))
            .await
        {
            Ok(Ok(txt)) => {
                upstream_used = true;
                provider_label = format!(
                    "官方 ZDR 零留存网关（已接入 {} · 阅后即焚）",
                    state
                        .app
                        .zdr_upstream
                        .as_ref()
                        .map(|c| c.model.as_str())
                        .unwrap_or("LLM")
                );
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
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(env): Json<SyncEnvelopeInput>,
) -> Response {
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match state.app.db.push_sync_envelope(&identity.account_id, &env) {
        Ok(id) => (
            StatusCode::OK,
            Json(json!({
                "status": "stored_ciphertext_only",
                "envelope_id": id,
                "path": env.path
            })),
        )
            .into_response(),
        Err(err) => control_plane_error_response(err),
    }
}

async fn sync_pull(
    State(state): State<RouterState>,
    headers: HeaderMap,
    Query(q): Query<PullQuery>,
) -> Response {
    let identity = match require_auth(&state.app, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let after_id = q.after_id.unwrap_or(0);
    match state
        .app
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

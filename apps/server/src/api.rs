//! HTTP routes for health, public catalog/recommendation, admin, and internal jobs.

use axum::body::Body;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use mpgs_domain::{
    FeedSection, FeedbackType, RankingSignals, RecommendationConfig, UserPreferences,
};
use mpgs_recommender::{ALGORITHM_VERSION, RankingInput, friend_fit, rank_feed_configured};
use mpgs_storage::{CreateOverrideRequest, EnqueueJob, Repository, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{OpenApi, ToSchema};

use crate::rate_limit::{RateLimitConfig, RateLimiter};

tokio::task_local! {
    static CURRENT_REQUEST_ID: String;
}
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone)]
pub struct AppState {
    pub repo: Option<Repository>,
    pub admin_token: Option<String>,
    pub rate_limits: RateLimitConfig,
}

#[derive(Debug, Serialize, ToSchema)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
struct ReadyResponse {
    status: &'static str,
    database: &'static str,
    schema_version: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
struct MetaResponse {
    api_version: &'static str,
    service_version: &'static str,
    algorithm_version: String,
    supported_sections: Vec<&'static str>,
    ai_available: bool,
    storage_enabled: bool,
}

#[derive(Debug, Serialize, ToSchema)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize, ToSchema)]
struct ErrorDetail {
    code: String,
    message: String,
    request_id: Option<String>,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct SessionResponseSchema {
    access_token: String,
    refresh_token: String,
    user_id: String,
    expires_at_ms: i64,
    refresh_expires_at_ms: i64,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct PartySchema {
    recommended_min: Option<u8>,
    recommended_max: Option<u8>,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct MultiplayerSummarySchema {
    dominant_mode: Option<String>,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct ScoreComponentsSchema {
    friend_fit: f64,
    section_score: f64,
    personalized_score: f64,
    final_score: f64,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct FeedItemSchema {
    app_id: u32,
    name: String,
    section: FeedSection,
    score: f64,
    confidence: f64,
    party: PartySchema,
    multiplayer: MultiplayerSummarySchema,
    reasons: Vec<String>,
    cautions: Vec<String>,
    evidence_ids: Vec<String>,
    components: ScoreComponentsSchema,
    algorithm_version: String,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct FeedResponseSchema {
    items: Vec<FeedItemSchema>,
    next_cursor: Option<String>,
    snapshot_at_ms: i64,
    algorithm_version: String,
    data_updated_at_ms: i64,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct CalendarItemSchema {
    app_id: u32,
    app_type: String,
    canonical_name: String,
    release_state: String,
    release_date: Option<String>,
    release_date_raw: Option<String>,
    release_date_precision: Option<String>,
    is_early_access: Option<bool>,
    current_data_confidence: Option<f64>,
    source_modified_at_ms: Option<i64>,
    created_at_ms: i64,
    updated_at_ms: i64,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct CalendarResponseSchema {
    dated_items: Vec<CalendarItemSchema>,
    undated_items: Vec<CalendarItemSchema>,
    data_updated_at_ms: i64,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct SearchItemSchema {
    app_id: u32,
    name: String,
    release_state: String,
    release_date: Option<String>,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct SearchResponseSchema {
    items: Vec<SearchItemSchema>,
    algorithm_version: String,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct MultiplayerDetailSchema {
    dominant_mode: Option<String>,
    private_session: Option<bool>,
    online_coop: Option<bool>,
    self_hosted_server: Option<bool>,
    recommended_min: Option<u8>,
    recommended_max: Option<u8>,
    profile_confidence: Option<f64>,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct ReviewSummarySchema {
    total: Option<u32>,
    positive: Option<u32>,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct GameAvailabilitySchema {
    platforms: Vec<String>,
    languages: Vec<String>,
    typical_session_minutes_min: Option<u32>,
    typical_session_minutes_max: Option<u32>,
    is_free: Option<bool>,
    final_price_minor: Option<i64>,
    price_currency: Option<String>,
    has_demo: bool,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct GameResponseSchema {
    app_id: u32,
    name: String,
    app_type: String,
    release_state: String,
    release_date: Option<String>,
    steam_url: String,
    multiplayer: MultiplayerDetailSchema,
    reviews: ReviewSummarySchema,
    latest_ccu: Option<u32>,
    availability: GameAvailabilitySchema,
    algorithm_version: String,
    data_updated_at_ms: i64,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct EvidenceItemSchema {
    evidence_id: String,
    feature: String,
    #[schema(value_type = Object)]
    value: serde_json::Value,
    source_type: String,
    source_label: String,
    confidence: f64,
    observed_at_ms: i64,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct EvidenceResponseSchema {
    items: Vec<EvidenceItemSchema>,
}

#[allow(dead_code)]
#[derive(ToSchema)]
struct FeedbackResponseSchema {
    feedback_id: i64,
    app_id: u32,
    #[schema(rename = "type")]
    feedback_type: String,
    recommendation_run_id: Option<String>,
    created_at_ms: i64,
}

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};

        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "MPGS API",
        version = "0.1.0",
        description = "Deterministic friend-group multiplayer game recommendation API"
    ),
    paths(
        health_live,
        health_ready,
        meta,
        create_session,
        refresh_session,
        get_preferences,
        put_preferences,
        get_feed,
        get_calendar,
        search_games,
        get_game,
        get_evidence,
        post_feedback,
        undo_feedback
    ),
    components(schemas(
        HealthResponse,
        ReadyResponse,
        MetaResponse,
        ErrorBody,
        ErrorDetail,
        SessionResponseSchema,
        RefreshSessionBody,
        UserPreferences,
        FeedSection,
        FeedItemSchema,
        FeedResponseSchema,
        PartySchema,
        MultiplayerSummarySchema,
        ScoreComponentsSchema,
        CalendarItemSchema,
        CalendarResponseSchema,
        SearchItemSchema,
        SearchResponseSchema,
        GameResponseSchema,
        MultiplayerDetailSchema,
        ReviewSummarySchema,
        GameAvailabilitySchema,
        EvidenceItemSchema,
        EvidenceResponseSchema,
        FeedbackBody,
        FeedbackResponseSchema
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Liveness and readiness"),
        (name = "session", description = "Anonymous sessions"),
        (name = "preferences", description = "User recommendation preferences"),
        (name = "recommendations", description = "Deterministic recommendation feeds"),
        (name = "catalog", description = "Catalog, calendar, and evidence"),
        (name = "feedback", description = "Recommendation feedback"),
        (name = "public", description = "Public service metadata")
    )
)]
struct ApiDoc;

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

pub fn build_router(state: AppState) -> Router {
    let rate_limiter = Arc::new(RateLimiter::new(state.rate_limits.clone()));
    let state = Arc::new(state);
    Router::new()
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/openapi.json", get(openapi_json))
        .route("/v1/meta", get(meta))
        .route("/v1/session/anonymous", post(create_session))
        .route("/v1/session/refresh", post(refresh_session))
        .route("/v1/preferences", get(get_preferences).put(put_preferences))
        .route("/v1/feeds/{section}", get(get_feed))
        .route("/v1/calendar", get(get_calendar))
        .route("/v1/search", get(search_games))
        .route("/v1/games/{app_id}", get(get_game))
        .route("/v1/games/{app_id}/evidence", get(get_evidence))
        .route("/v1/feedback", post(post_feedback))
        .route("/v1/feedback/{feedback_id}/undo", post(undo_feedback))
        .route("/admin/v1/games/{app_id}/overrides", post(create_override))
        .route("/admin/v1/overrides/{id}/revoke", post(revoke_override))
        .route("/admin/v1/games/{app_id}/debug", get(game_debug))
        .route("/internal/v1/jobs/enqueue", post(enqueue_job))
        .route("/internal/v1/jobs/lease", post(lease_jobs))
        .route("/internal/v1/jobs/{job_id}/complete", post(complete_job))
        .route("/internal/v1/jobs/{job_id}/fail", post(fail_job))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn_with_state(
            rate_limiter,
            crate::rate_limit::middleware,
        ))
        .layer(middleware::from_fn(request_id_middleware))
        .with_state(state)
}

async fn request_id_middleware(mut req: axum::http::Request<Body>, next: Next) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|value| valid_request_id(value))
        .map(str::to_owned)
        .unwrap_or_else(new_request_id);
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        req.headers_mut().insert("x-request-id", value);
    }
    let mut response = CURRENT_REQUEST_ID
        .scope(request_id.clone(), next.run(req))
        .await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}

fn new_request_id() -> String {
    static FALLBACK_COUNTER: AtomicU64 = AtomicU64::new(1);
    let mut bytes = [0_u8; 16];
    if getrandom::fill(&mut bytes).is_err() {
        let counter = FALLBACK_COUNTER.fetch_add(1, Ordering::Relaxed);
        return format!("req-fallback-{}-{counter}", std::process::id());
    }
    let suffix: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("req-{suffix}")
}

fn valid_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

#[utoipa::path(
    get,
    path = "/health/live",
    responses((status = 200, description = "Process is alive", body = HealthResponse)),
    tag = "health"
)]
async fn health_live() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "mpgs-server",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[utoipa::path(
    get,
    path = "/health/ready",
    responses(
        (status = 200, description = "Service is ready", body = ReadyResponse),
        (status = 503, description = "Storage or catalog is not ready", body = ReadyResponse)
    ),
    tag = "health"
)]
async fn health_ready(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match &state.repo {
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyResponse {
                status: "not_ready",
                database: "disabled",
                schema_version: None,
            }),
        )
            .into_response(),
        Some(repo) => match storage_result(repo, |repo| {
            repo.readiness_check()?;
            repo.database().schema_version()
        })
        .await
        {
            Ok(schema_version) => (
                StatusCode::OK,
                Json(ReadyResponse {
                    status: "ready",
                    database: "ok",
                    schema_version: Some(schema_version),
                }),
            )
                .into_response(),
            Err(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ReadyResponse {
                    status: "not_ready",
                    database: "unavailable",
                    schema_version: None,
                }),
            )
                .into_response(),
        },
    }
}

#[utoipa::path(
    get,
    path = "/v1/meta",
    responses(
        (status = 200, body = MetaResponse),
        (status = 304, description = "Not modified")
    ),
    tag = "public"
)]
async fn meta(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    let algorithm_version = match state.repo.as_ref() {
        Some(repo) => match storage_result(repo, |repo| repo.active_algorithm_config()).await {
            Ok(active) => active.version,
            Err(error) => return map_storage_error(error, None),
        },
        None => ALGORITHM_VERSION.to_owned(),
    };
    let body = MetaResponse {
        api_version: "v1",
        service_version: env!("CARGO_PKG_VERSION"),
        algorithm_version,
        supported_sections: FeedSection::ALL
            .into_iter()
            .map(FeedSection::as_str)
            .collect(),
        ai_available: false,
        storage_enabled: state.repo.is_some(),
    };
    let etag = weak_etag(&format!(
        "{}:{}:{}",
        body.service_version, body.algorithm_version, body.storage_enabled
    ));
    if_none_match_ok(&headers, &etag)
        .unwrap_or_else(|| ([(header::ETAG, etag)], Json(body)).into_response())
}

#[utoipa::path(
    post,
    path = "/v1/session/anonymous",
    responses(
        (status = 201, body = SessionResponseSchema),
        (status = 503, body = ErrorBody)
    ),
    tag = "session"
)]
async fn create_session(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    match storage_result(repo, |repo| repo.create_anonymous_session()).await {
        Ok(session) => (StatusCode::CREATED, Json(session_json(&session))).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

#[derive(Debug, Deserialize, ToSchema)]
struct RefreshSessionBody {
    refresh_token: String,
}

#[utoipa::path(
    post,
    path = "/v1/session/refresh",
    request_body = RefreshSessionBody,
    responses(
        (status = 200, body = SessionResponseSchema),
        (status = 401, body = ErrorBody)
    ),
    tag = "session"
)]
async fn refresh_session(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RefreshSessionBody>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let refresh_token = body.refresh_token;
    match storage_result(repo, move |repo| {
        repo.refresh_anonymous_session(&refresh_token)
    })
    .await
    {
        Ok(session) => (StatusCode::OK, Json(session_json(&session))).into_response(),
        Err(StorageError::NotFound { .. }) => error_response(
            StatusCode::UNAUTHORIZED,
            "unauthenticated",
            "invalid or expired refresh token",
            None,
        ),
        Err(error) => map_storage_error(error, None),
    }
}

fn session_json(session: &mpgs_storage::users::SessionTokens) -> serde_json::Value {
    json!({
        "access_token": session.access_token,
        "refresh_token": session.refresh_token,
        "user_id": session.user_id,
        "expires_at_ms": session.expires_at_ms,
        "refresh_expires_at_ms": session.refresh_expires_at_ms,
    })
}

#[utoipa::path(
    get,
    path = "/v1/preferences",
    responses(
        (status = 200, body = UserPreferences),
        (status = 401, body = ErrorBody)
    ),
    security(("bearer_auth" = [])),
    tag = "preferences"
)]
async fn get_preferences(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let user_id = match require_user(repo, &headers).await {
        Ok(id) => id,
        Err(resp) => return *resp,
    };
    match storage_result(repo, move |repo| repo.get_preferences(&user_id)).await {
        Ok(prefs) => (StatusCode::OK, Json(prefs)).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

#[utoipa::path(
    put,
    path = "/v1/preferences",
    request_body = UserPreferences,
    responses(
        (status = 200, body = UserPreferences),
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
        (status = 409, body = ErrorBody)
    ),
    security(("bearer_auth" = [])),
    tag = "preferences"
)]
async fn put_preferences(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<UserPreferences>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let user_id = match require_user(repo, &headers).await {
        Ok(id) => id,
        Err(resp) => return *resp,
    };
    match storage_result(repo, move |repo| repo.put_preferences(&user_id, &body)).await {
        Ok(prefs) => (StatusCode::OK, Json(prefs)).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct FeedQuery {
    limit: Option<i64>,
    cursor: Option<String>,
    party_size: Option<u8>,
    platforms: Option<String>,
    languages: Option<String>,
    session_minutes_min: Option<u32>,
    session_minutes_max: Option<u32>,
    max_price_minor: Option<i64>,
    currency: Option<String>,
    demo_only: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/v1/feeds/{section}",
    params(
        ("section" = FeedSection, Path, description = "Recommendation section"),
        FeedQuery
    ),
    responses(
        (status = 200, body = FeedResponseSchema),
        (status = 304, description = "Not modified"),
        (status = 400, body = ErrorBody),
        (status = 409, body = ErrorBody)
    ),
    tag = "recommendations"
)]
async fn get_feed(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(section): Path<String>,
    Query(query): Query<FeedQuery>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let active_config = match storage_result(repo, |repo| repo.active_algorithm_config()).await {
        Ok(config) => config,
        Err(error) => return map_storage_error(error, None),
    };
    let section = match FeedSection::parse(&section) {
        Some(s) => s,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_argument",
                "unknown feed section",
                None,
            );
        }
    };
    let (mut prefs, user_id) = match user_context(repo, &headers).await {
        Ok(context) => context,
        Err(response) => return *response,
    };
    if let Some(party_size) = query.party_size {
        if !(1..=64).contains(&party_size) {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_argument",
                "party_size must be between 1 and 64",
                None,
            );
        }
        prefs.party_size = party_size;
    }
    if let Err(response) = apply_feed_overrides(&mut prefs, &query) {
        return *response;
    }
    let requested_limit = query.limit.unwrap_or(20);
    if !(1..=100).contains(&requested_limit) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_argument",
            "limit must be between 1 and 100",
            None,
        );
    }
    let limit = requested_limit as usize;
    let active_feedback = match user_id {
        Some(ref user_id) => {
            let user_id = user_id.clone();
            match storage_result(repo, move |repo| repo.list_active_feedback(&user_id)).await {
                Ok(feedback) => feedback,
                Err(error) => return map_storage_error(error, None),
            }
        }
        None => Vec::new(),
    };
    let feedback_by_app: HashMap<_, _> = active_feedback
        .iter()
        .map(|feedback| (feedback.app_id, feedback.feedback_type.as_str()))
        .collect();
    let preference_context = recommendation_context(
        &prefs,
        &active_feedback,
        &active_config.version,
        &active_config.config,
    );
    let snapshot_ms = match storage_result(repo, |repo| repo.data_updated_at_ms()).await {
        Ok(value) => value,
        Err(error) => return map_storage_error(error, None),
    };
    let offset = match decode_cursor(
        query.cursor.as_deref(),
        section,
        snapshot_ms,
        &preference_context,
    ) {
        Ok(value) => value,
        Err(CursorError::Invalid) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_argument",
                "invalid feed cursor",
                None,
            );
        }
        Err(CursorError::Stale) => {
            return error_response(
                StatusCode::CONFLICT,
                "cursor_stale",
                "feed cursor snapshot is stale; restart pagination",
                None,
            );
        }
    };
    let etag = weak_etag(&format!(
        "feed:v1:{}:{snapshot_ms}:{preference_context}:{offset}:{limit}:{}",
        section.as_str(),
        active_config.version
    ));
    if let Some(response) = if_none_match_ok(&headers, &etag) {
        return response;
    }

    let now_ms = repo.database().now_ms();
    let today = mpgs_storage::util::day_utc_from_ms(now_ms);
    let cutoff = mpgs_storage::util::day_utc_from_ms(
        now_ms.saturating_sub(i64::from(active_config.config.recent_days) * 24 * 60 * 60 * 1000),
    );

    let cutoff_query = cutoff.clone();
    let today_query = today.clone();
    let currency = prefs.budget_currency.clone();
    let recommendation_config = active_config.config.clone();
    let candidate_limit = i64::from(active_config.config.candidate_limit);
    let candidates = match storage_result(repo, move |repo| {
        repo.list_candidates(
            section,
            &cutoff_query,
            &today_query,
            &currency,
            &recommendation_config,
            candidate_limit,
        )
    })
    .await
    {
        Ok(rows) => rows,
        Err(error) => return map_storage_error(error, None),
    };
    let inputs: Vec<RankingInput> = candidates
        .into_iter()
        .filter_map(|row| {
            if query.demo_only == Some(true) && !row.has_demo {
                return None;
            }
            let signals = row.to_ranking_signals();
            let feedback = feedback_by_app.get(&row.app_id).copied();
            if matches!(feedback, Some("not_interested" | "party_size_mismatch")) {
                return None;
            }
            let personal_adjustment = match feedback {
                Some("like") => 0.20,
                Some("played") => 0.05,
                Some("too_competitive") if signals.multiplayer.matchmaking_core >= 0.5 => -0.30,
                Some("too_competitive" | "hosting_friction") => -0.15,
                _ => 0.0,
            };
            let availability = row.availability();
            section_matches(
                section,
                &row,
                &signals,
                &cutoff,
                &today,
                &active_config.config,
            )
            .then_some(RankingInput {
                app_id: row.app_id,
                name: row.name,
                dominant_mode: row.dominant_mode,
                recommended_min: row.recommended_min,
                recommended_max: row.recommended_max,
                availability,
                signals,
                personal_adjustment,
            })
        })
        .collect();

    let ranked = rank_feed_configured(
        section,
        &inputs,
        &prefs,
        &active_config.config,
        &active_config.version,
    );
    let total = ranked.items.len();
    let page: Vec<_> = ranked
        .items
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|item| {
            json!({
                "app_id": item.app_id,
                "name": item.name,
                "section": section.as_str(),
                "score": item.score.final_score,
                "confidence": item.score.friend_fit,
                "party": {
                    "recommended_min": item.recommended_min,
                    "recommended_max": item.recommended_max,
                },
                "multiplayer": {
                    "dominant_mode": item.dominant_mode,
                },
                "reasons": item.explanation.reasons,
                "cautions": item.explanation.cautions,
                "evidence_ids": item.explanation.evidence_ids,
                "components": {
                    "friend_fit": item.score.friend_fit,
                    "section_score": item.score.section_score,
                    "personalized_score": item.score.personalized_score,
                    "final_score": item.score.final_score,
                },
                "algorithm_version": item.algorithm_version,
            })
        })
        .collect();

    let next_offset = offset.saturating_add(limit);
    let next_cursor = if next_offset < total {
        Some(encode_cursor(
            section,
            snapshot_ms,
            &preference_context,
            next_offset,
        ))
    } else {
        None
    };
    let body = json!({
        "items": page,
        "next_cursor": next_cursor,
        "snapshot_at_ms": snapshot_ms,
        "algorithm_version": active_config.version,
        "data_updated_at_ms": snapshot_ms,
    });
    (StatusCode::OK, [(header::ETAG, etag)], Json(body)).into_response()
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct CalendarQuery {
    from: Option<String>,
    to: Option<String>,
}

#[utoipa::path(
    get,
    path = "/v1/calendar",
    params(CalendarQuery),
    responses(
        (status = 200, body = CalendarResponseSchema),
        (status = 304, description = "Not modified"),
        (status = 400, body = ErrorBody)
    ),
    tag = "catalog"
)]
async fn get_calendar(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<CalendarQuery>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let now_ms = repo.database().now_ms();
    let from = query
        .from
        .unwrap_or_else(|| mpgs_storage::util::day_utc_from_ms(now_ms));
    let to = query.to.unwrap_or_else(|| {
        mpgs_storage::util::day_utc_from_ms(now_ms.saturating_add(365_i64 * 24 * 60 * 60 * 1000))
    });
    let Some(from_day) = mpgs_storage::util::iso_day_to_unix_days(&from) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_argument",
            "calendar dates must use valid YYYY-MM-DD values",
            None,
        );
    };
    let Some(to_day) = mpgs_storage::util::iso_day_to_unix_days(&to) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_argument",
            "calendar dates must use valid YYYY-MM-DD values",
            None,
        );
    };
    if to_day < from_day || to_day - from_day > 366 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_argument",
            "calendar range must be ordered and not exceed one year",
            None,
        );
    }
    let data_updated_at_ms = match storage_result(repo, |repo| repo.data_updated_at_ms()).await {
        Ok(value) => value,
        Err(error) => return map_storage_error(error, None),
    };
    let etag = weak_etag(&format!("calendar:v1:{from}:{to}:{data_updated_at_ms}"));
    if let Some(response) = if_none_match_ok(&headers, &etag) {
        return response;
    }
    match storage_result(repo, move |repo| repo.list_calendar(&from, &to)).await {
        Ok((dated, undated)) => {
            let body = json!({
                "dated_items": dated,
                "undated_items": undated,
                "data_updated_at_ms": data_updated_at_ms,
            });
            (StatusCode::OK, [(header::ETAG, etag)], Json(body)).into_response()
        }
        Err(error) => map_storage_error(error, None),
    }
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct SearchQuery {
    q: Option<String>,
    limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/v1/search",
    params(SearchQuery),
    responses(
        (status = 200, body = SearchResponseSchema),
        (status = 400, body = ErrorBody)
    ),
    tag = "catalog"
)]
async fn search_games(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let q = query.q.unwrap_or_default();
    if q.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_argument",
            "q is required",
            None,
        );
    }
    let limit = query.limit.unwrap_or(20);
    if !(1..=100).contains(&limit) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_argument",
            "limit must be between 1 and 100",
            None,
        );
    }
    match storage_result(repo, move |repo| repo.search_games(&q, limit)).await {
        Ok(items) => {
            let body = json!({
                "items": items.iter().map(|g| json!({
                    "app_id": g.app_id,
                    "name": g.name,
                    "release_state": g.release_state,
                    "release_date": g.release_date,
                })).collect::<Vec<_>>(),
                "algorithm_version": ALGORITHM_VERSION,
            });
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(error) => map_storage_error(error, None),
    }
}

#[utoipa::path(
    get,
    path = "/v1/games/{app_id}",
    params(("app_id" = u32, Path, description = "Steam AppID")),
    responses(
        (status = 200, body = GameResponseSchema),
        (status = 304, description = "Not modified"),
        (status = 404, body = ErrorBody)
    ),
    tag = "catalog"
)]
async fn get_game(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_id): Path<u32>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let data_updated_at_ms = match storage_result(repo, |repo| repo.data_updated_at_ms()).await {
        Ok(value) => value,
        Err(error) => return map_storage_error(error, None),
    };
    let etag = weak_etag(&format!(
        "game:v1:{app_id}:{data_updated_at_ms}:{ALGORITHM_VERSION}"
    ));
    if let Some(response) = if_none_match_ok(&headers, &etag) {
        return response;
    }
    match storage_result(repo, move |repo| repo.game_detail(app_id)).await {
        Ok(Some(game)) => {
            let body = json!({
                "app_id": game.app_id,
                "name": game.name,
                "app_type": game.app_type,
                "release_state": game.release_state,
                "release_date": game.release_date,
                "steam_url": format!("https://store.steampowered.com/app/{app_id}/"),
                "multiplayer": {
                    "dominant_mode": game.dominant_mode,
                    "private_session": game.private_session,
                    "online_coop": game.online_coop,
                    "self_hosted_server": game.self_hosted_server,
                    "recommended_min": game.recommended_min,
                    "recommended_max": game.recommended_max,
                    "profile_confidence": game.profile_confidence,
                },
                "reviews": {
                    "total": game.total_reviews,
                    "positive": game.total_positive,
                },
                "latest_ccu": game.latest_ccu,
                "availability": {
                    "platforms": game.platforms,
                    "languages": game.languages,
                    "typical_session_minutes_min": game.typical_session_minutes_min,
                    "typical_session_minutes_max": game.typical_session_minutes_max,
                    "is_free": game.is_free,
                    "final_price_minor": game.final_price_minor,
                    "price_currency": game.price_currency,
                    "has_demo": game.has_demo,
                },
                "algorithm_version": ALGORITHM_VERSION,
                "data_updated_at_ms": data_updated_at_ms,
            });
            (StatusCode::OK, [(header::ETAG, etag)], Json(body)).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "not_found", "game not found", None),
        Err(error) => map_storage_error(error, None),
    }
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct EvidenceQuery {
    feature: Option<String>,
}

#[utoipa::path(
    get,
    path = "/v1/games/{app_id}/evidence",
    params(
        ("app_id" = u32, Path, description = "Steam AppID"),
        EvidenceQuery
    ),
    responses(
        (status = 200, body = EvidenceResponseSchema),
        (status = 400, body = ErrorBody),
        (status = 404, body = ErrorBody)
    ),
    tag = "catalog"
)]
async fn get_evidence(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<u32>,
    Query(query): Query<EvidenceQuery>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    if query
        .feature
        .as_ref()
        .is_some_and(|value| value.trim().is_empty() || value.len() > 64)
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_argument",
            "feature must contain between 1 and 64 bytes",
            None,
        );
    }
    let game = match storage_result(repo, move |repo| repo.game_detail(app_id)).await {
        Ok(Some(game)) => game,
        Ok(None) => {
            return error_response(StatusCode::NOT_FOUND, "not_found", "game not found", None);
        }
        Err(error) => return map_storage_error(error, None),
    };
    let feature = query.feature.clone();
    match storage_result(repo, move |repo| {
        repo.list_evidence(app_id, feature.as_deref())
    })
    .await
    {
        Ok(items) => {
            let mut seen_features = HashSet::new();
            let mut public_items: Vec<_> = items
                .iter()
                .map(|e| {
                    let canonical = seen_features.insert(e.feature_name.clone());
                    let evidence_id = if canonical {
                        format!("feature:{}:{app_id}", e.feature_name)
                    } else {
                        format!("feature:{}:{app_id}:{}", e.feature_name, e.evidence_id)
                    };
                    json!({
                        "evidence_id": evidence_id,
                        "feature": e.feature_name,
                        "value": serde_json::from_str::<serde_json::Value>(&e.value_json).unwrap_or(json!(null)),
                        "source_type": e.source_type,
                        "source_label": e.source_ref,
                        "confidence": e.confidence,
                        "observed_at_ms": e.observed_at_ms,
                    })
                })
                .collect();
            let data_updated_at_ms =
                match storage_result(repo, |repo| repo.data_updated_at_ms()).await {
                    Ok(value) => value,
                    Err(error) => return map_storage_error(error, None),
                };
            for (feature, value) in [
                ("private_session", game.private_session),
                ("self_hosted_server", game.self_hosted_server),
                ("online_coop", game.online_coop),
            ] {
                let requested = query.feature.as_deref().is_none_or(|name| name == feature);
                if requested
                    && !seen_features.contains(feature)
                    && let Some(value) = value
                {
                    public_items.push(json!({
                        "evidence_id": format!("feature:{feature}:{app_id}"),
                        "feature": feature,
                        "value": value,
                        "source_type": "computed_profile",
                        "source_label": "normalized multiplayer profile",
                        "confidence": game.profile_confidence.unwrap_or(0.4),
                        "observed_at_ms": data_updated_at_ms,
                    }));
                }
            }
            if query
                .feature
                .as_deref()
                .is_none_or(|name| name == "review_summary")
                && let Some(total) = game.total_reviews
            {
                public_items.push(json!({
                    "evidence_id": format!("review:{app_id}:summary"),
                    "feature": "review_summary",
                    "value": {
                        "total": total,
                        "positive": game.total_positive,
                        "wilson_lower": game.wilson_lower,
                    },
                    "source_type": "steam_reviews",
                    "source_label": "latest normalized review summary",
                    "confidence": 0.9,
                    "observed_at_ms": data_updated_at_ms,
                }));
            }
            let body = json!({
                "items": public_items,
            });
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(error) => map_storage_error(error, None),
    }
}

#[derive(Debug, Deserialize, ToSchema)]
struct FeedbackBody {
    app_id: u32,
    #[serde(rename = "type")]
    feedback_type: String,
    recommendation_run_id: Option<String>,
    client_created_at_ms: Option<i64>,
}

#[utoipa::path(
    post,
    path = "/v1/feedback",
    request_body = FeedbackBody,
    params(("Idempotency-Key" = String, Header, description = "Client-generated idempotency key")),
    responses(
        (status = 201, body = FeedbackResponseSchema),
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
        (status = 409, body = ErrorBody)
    ),
    security(("bearer_auth" = [])),
    tag = "feedback"
)]
async fn post_feedback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<FeedbackBody>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let user_id = match require_user(repo, &headers).await {
        Ok(id) => id,
        Err(resp) => return *resp,
    };
    let Some(feedback_type) = FeedbackType::parse(&body.feedback_type) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_argument",
            "unknown feedback type",
            None,
        );
    };
    let Some(idem) = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
    else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_argument",
            "Idempotency-Key header is required",
            None,
        );
    };
    match storage_result(repo, move |repo| {
        repo.create_feedback(
            &user_id,
            body.app_id,
            feedback_type,
            body.recommendation_run_id.as_deref(),
            &idem,
            body.client_created_at_ms,
        )
    })
    .await
    {
        Ok(record) => (StatusCode::CREATED, Json(record_json(&record))).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

#[utoipa::path(
    post,
    path = "/v1/feedback/{feedback_id}/undo",
    params(("feedback_id" = i64, Path, description = "Feedback event ID")),
    responses(
        (status = 200, body = FeedbackResponseSchema),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody)
    ),
    security(("bearer_auth" = [])),
    tag = "feedback"
)]
async fn undo_feedback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(feedback_id): Path<i64>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let user_id = match require_user(repo, &headers).await {
        Ok(id) => id,
        Err(resp) => return *resp,
    };
    match storage_result(repo, move |repo| repo.undo_feedback(&user_id, feedback_id)).await {
        Ok(record) => (StatusCode::OK, Json(record_json(&record))).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

fn record_json(record: &mpgs_storage::feedback::FeedbackRecord) -> serde_json::Value {
    json!({
        "feedback_id": record.feedback_id,
        "app_id": record.app_id,
        "type": record.feedback_type,
        "recommendation_run_id": record.recommendation_run_id,
        "created_at_ms": record.created_at_ms,
    })
}

// --- admin / internal (M2) ---

#[derive(Debug, Deserialize)]
struct CreateOverrideBody {
    feature_name: String,
    value: serde_json::Value,
    reason: String,
    external_evidence: Option<String>,
    operator: String,
}

#[derive(Debug, Deserialize)]
struct RevokeOverrideBody {
    operator: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct LeaseJobsBody {
    owner: String,
    limit: Option<i64>,
    lease_ms: Option<i64>,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompleteJobBody {
    owner: String,
    idempotency_key: String,
}

#[derive(Debug, Deserialize)]
struct FailJobBody {
    owner: String,
    error_category: String,
    retry_delay_ms: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct EnqueueJobBody {
    source: String,
    task_type: String,
    entity_key: String,
    priority: Option<i64>,
    due_at_ms: Option<i64>,
    idempotency_key: String,
    payload_json: Option<serde_json::Value>,
    max_attempts: Option<i64>,
}

async fn create_override(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_id): Path<u32>,
    Json(body): Json<CreateOverrideBody>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin(&state, &headers) {
        return *resp;
    }
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let request = CreateOverrideRequest {
        feature_name: body.feature_name,
        value_json: body.value,
        reason: body.reason,
        external_evidence: body.external_evidence,
        operator: body.operator,
        request_id: header_str(&headers, "x-request-id"),
    };
    match storage_result(repo, move |repo| repo.create_override(app_id, &request)).await {
        Ok(over) => (StatusCode::CREATED, Json(over)).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

async fn revoke_override(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(body): Json<RevokeOverrideBody>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin(&state, &headers) {
        return *resp;
    }
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let request_id = header_str(&headers, "x-request-id");
    match storage_result(repo, move |repo| {
        repo.revoke_override(id, &body.operator, &body.reason, request_id.as_deref())
    })
    .await
    {
        Ok(over) => (StatusCode::OK, Json(over)).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

async fn game_debug(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_id): Path<u32>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin(&state, &headers) {
        return *resp;
    }
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let (app, profile) = match storage_result(repo, move |repo| {
        Ok((repo.get_app(app_id)?, repo.get_profile(app_id)?))
    })
    .await
    {
        Ok(v) => v,
        Err(error) => return map_storage_error(error, None),
    };
    (
        StatusCode::OK,
        Json(json!({"app": app, "multiplayer_profile": profile})),
    )
        .into_response()
}

async fn enqueue_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<EnqueueJobBody>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin(&state, &headers) {
        return *resp;
    }
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let now = repo.database().now_ms();
    let job = EnqueueJob {
        source: body.source,
        task_type: body.task_type,
        entity_key: body.entity_key,
        priority: body.priority.unwrap_or(100),
        due_at_ms: body.due_at_ms.unwrap_or(now),
        idempotency_key: body.idempotency_key,
        payload_json: body.payload_json.map(|v| v.to_string()),
        max_attempts: body.max_attempts.unwrap_or(5),
    };
    match storage_result(repo, move |repo| repo.enqueue_job(&job)).await {
        Ok(job_id) => (StatusCode::CREATED, Json(json!({"job_id": job_id}))).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

async fn lease_jobs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<LeaseJobsBody>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin(&state, &headers) {
        return *resp;
    }
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    match storage_result(repo, move |repo| {
        repo.lease_jobs(
            &body.owner,
            body.limit.unwrap_or(10),
            body.lease_ms.unwrap_or(60_000),
            body.source.as_deref(),
        )
    })
    .await
    {
        Ok(jobs) => (StatusCode::OK, Json(jobs)).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

async fn complete_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_id): Path<i64>,
    Json(body): Json<CompleteJobBody>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin(&state, &headers) {
        return *resp;
    }
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    match storage_result(repo, move |repo| {
        repo.complete_job(job_id, &body.owner, &body.idempotency_key)
    })
    .await
    {
        Ok(job) => (StatusCode::OK, Json(job)).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

async fn fail_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_id): Path<i64>,
    Json(body): Json<FailJobBody>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin(&state, &headers) {
        return *resp;
    }
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    match storage_result(repo, move |repo| {
        repo.fail_job(
            job_id,
            &body.owner,
            &body.error_category,
            body.retry_delay_ms.unwrap_or(60_000),
        )
    })
    .await
    {
        Ok(job) => (StatusCode::OK, Json(job)).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

// --- helpers ---

fn require_repo(state: &AppState) -> Option<&Repository> {
    state.repo.as_ref()
}

async fn require_user(repo: &Repository, headers: &HeaderMap) -> Result<String, Box<Response>> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_owned)
        .ok_or_else(|| {
            Box::new(error_response(
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "missing bearer token",
                None,
            ))
        })?;
    storage_result(repo, move |repo| repo.resolve_access_token(&token))
        .await
        .map_err(|error| {
            if matches!(error, StorageError::NotFound { .. }) {
                Box::new(error_response(
                    StatusCode::UNAUTHORIZED,
                    "unauthenticated",
                    "invalid or expired bearer token",
                    None,
                ))
            } else {
                Box::new(map_storage_error(error, None))
            }
        })
}

async fn user_context(
    repo: &Repository,
    headers: &HeaderMap,
) -> Result<(UserPreferences, Option<String>), Box<Response>> {
    if !headers.contains_key(header::AUTHORIZATION) {
        return Ok((UserPreferences::default(), None));
    }
    let user_id = require_user(repo, headers).await?;
    let lookup_user_id = user_id.clone();
    let preferences = storage_result(repo, move |repo| repo.get_preferences(&lookup_user_id))
        .await
        .map_err(|error| Box::new(map_storage_error(error, None)))?;
    Ok((preferences, Some(user_id)))
}

async fn storage_result<T, F>(repo: &Repository, operation: F) -> Result<T, StorageError>
where
    T: Send + 'static,
    F: FnOnce(Repository) -> Result<T, StorageError> + Send + 'static,
{
    let repo = repo.clone();
    tokio::task::spawn_blocking(move || operation(repo))
        .await
        .map_err(|error| StorageError::Io(std::io::Error::other(error.to_string())))?
}

fn section_matches(
    section: FeedSection,
    row: &mpgs_storage::query::GameCandidateRow,
    signals: &RankingSignals,
    cutoff_date: &str,
    today: &str,
    config: &RecommendationConfig,
) -> bool {
    let friend_score = friend_fit(&signals.multiplayer);
    let activity = row.typical_ccu_7d.or(row.latest_ccu).unwrap_or(0);
    let date = row.release_date.as_deref();
    match section {
        FeedSection::Upcoming => {
            let has_multiplayer_evidence = row.dominant_mode.is_some()
                || row.private_session == Some(true)
                || row.online_coop == Some(true)
                || row.self_hosted_server == Some(true);
            (row.release_state == "upcoming"
                || row.release_state == "coming_soon"
                || row.app_type == "demo"
                || row.app_type == "playtest")
                && has_multiplayer_evidence
        }
        FeedSection::RecentRelease => {
            row.release_state == "released"
                && date.is_some_and(|value| value >= cutoff_date && value <= today)
                && friend_score >= config.recent_min_friend_fit
        }
        FeedSection::PopularLegacy => {
            let quality_floor = if activity >= config.popular_high_ccu {
                config.popular_high_ccu_min_wilson
            } else {
                config.popular_min_wilson
            };
            row.release_state == "released"
                && date.is_some_and(|value| value < cutoff_date)
                && activity >= config.popular_min_ccu
                && row.wilson_lower.is_some_and(|value| value >= quality_floor)
                && friend_score >= config.popular_min_friend_fit
        }
        FeedSection::ClassicLegacy => {
            let friend_group_independent =
                row.private_session == Some(true) || row.self_hosted_server == Some(true);
            row.release_state == "released"
                && date.is_some_and(|value| value < cutoff_date)
                && row
                    .total_reviews
                    .is_some_and(|value| value >= config.classic_min_reviews)
                && row
                    .wilson_lower
                    .is_some_and(|value| value >= config.classic_min_wilson)
                && friend_score >= config.classic_min_friend_fit
                && (friend_group_independent || activity >= config.classic_public_min_ccu)
        }
    }
}

fn encode_cursor(
    section: FeedSection,
    snapshot_ms: i64,
    preference_context: &str,
    offset: usize,
) -> String {
    format!(
        "v1:{}:{snapshot_ms}:{preference_context}:{offset}",
        section.as_str()
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CursorError {
    Invalid,
    Stale,
}

fn decode_cursor(
    cursor: Option<&str>,
    expected_section: FeedSection,
    expected_snapshot_ms: i64,
    expected_preference_context: &str,
) -> Result<usize, CursorError> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    let mut parts = cursor.split(':');
    let valid_version = parts.next() == Some("v1");
    let section = parts.next();
    let snapshot = parts.next().and_then(|value| value.parse::<i64>().ok());
    let preference_context = parts.next();
    let offset = parts.next().and_then(|value| value.parse::<usize>().ok());
    if !valid_version
        || section != Some(expected_section.as_str())
        || snapshot.is_none()
        || preference_context.is_none()
        || offset.is_none()
        || parts.next().is_some()
    {
        return Err(CursorError::Invalid);
    }
    if snapshot != Some(expected_snapshot_ms)
        || preference_context != Some(expected_preference_context)
    {
        return Err(CursorError::Stale);
    }
    Ok(offset.expect("validated above"))
}

fn recommendation_context(
    prefs: &UserPreferences,
    feedback: &[mpgs_storage::feedback::ActiveFeedback],
    algorithm_version: &str,
    config: &RecommendationConfig,
) -> String {
    let payload = serde_json::to_string(prefs).unwrap_or_else(|_| format!("v{}", prefs.version));
    let mut context = payload;
    context.push('|');
    context.push_str(algorithm_version);
    context.push('|');
    context.push_str(&serde_json::to_string(config).unwrap_or_default());
    for item in feedback {
        context.push('|');
        context.push_str(&item.app_id.to_string());
        context.push(':');
        context.push_str(&item.feedback_type);
    }
    format!("{:x}", hash64(&context))
}

fn apply_feed_overrides(
    prefs: &mut UserPreferences,
    query: &FeedQuery,
) -> Result<(), Box<Response>> {
    if let Some(platforms) = query.platforms.as_deref() {
        prefs.platforms = parse_csv_filter("platforms", platforms)?;
    }
    if let Some(languages) = query.languages.as_deref() {
        prefs.languages = parse_csv_filter("languages", languages)?;
    }
    if let Some(value) = query.session_minutes_min {
        prefs.session_minutes_min = value;
    }
    if let Some(value) = query.session_minutes_max {
        prefs.session_minutes_max = value;
    }
    if let Some(value) = query.max_price_minor {
        prefs.budget_max_each_minor = Some(value);
    }
    if let Some(value) = query.currency.as_deref() {
        prefs.budget_currency = value.to_ascii_uppercase();
    }
    prefs.validate().map_err(|message| {
        Box::new(error_response(
            StatusCode::BAD_REQUEST,
            "invalid_argument",
            &message,
            None,
        ))
    })
}

fn parse_csv_filter(name: &str, value: &str) -> Result<Vec<String>, Box<Response>> {
    let values: Vec<_> = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    if values.is_empty() || values.len() > 32 || values.iter().any(|item| item.len() > 64) {
        return Err(Box::new(error_response(
            StatusCode::BAD_REQUEST,
            "invalid_argument",
            &format!("{name} must be a comma-separated list of 1 to 32 values"),
            None,
        )));
    }
    Ok(values)
}

fn weak_etag(payload: &str) -> HeaderValue {
    let hash = hash64(payload);
    HeaderValue::from_str(&format!("W/\"{hash:x}\"")).unwrap_or(HeaderValue::from_static("W/\"0\""))
}

fn hash64(payload: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in payload.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn if_none_match_ok(headers: &HeaderMap, etag: &HeaderValue) -> Option<Response> {
    let inm = headers.get(header::IF_NONE_MATCH)?.to_str().ok()?;
    if inm == etag.to_str().ok()? {
        return Some(StatusCode::NOT_MODIFIED.into_response());
    }
    None
}

fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), Box<Response>> {
    let Some(expected) = state.admin_token.as_deref() else {
        return Err(Box::new(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "temporarily_unavailable",
            "MPGS_ADMIN_TOKEN is not configured",
            None,
        )));
    };
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if provided != Some(expected) {
        return Err(Box::new(error_response(
            StatusCode::UNAUTHORIZED,
            "unauthenticated",
            "invalid admin token",
            None,
        )));
    }
    Ok(())
}

fn storage_disabled() -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "temporarily_unavailable",
        "storage is disabled",
        None,
    )
}

fn map_storage_error(error: StorageError, request_id: Option<String>) -> Response {
    let (status, code) = match &error {
        StorageError::NotFound { .. } => (StatusCode::NOT_FOUND, "not_found"),
        StorageError::Validation { .. } => (StatusCode::BAD_REQUEST, "invalid_argument"),
        StorageError::Conflict { .. } => (StatusCode::CONFLICT, "version_conflict"),
        StorageError::Lease { .. } => (StatusCode::CONFLICT, "version_conflict"),
        StorageError::Migration { .. } => {
            (StatusCode::SERVICE_UNAVAILABLE, "temporarily_unavailable")
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
    };
    let message = match status {
        StatusCode::INTERNAL_SERVER_ERROR => "internal server error".to_owned(),
        StatusCode::SERVICE_UNAVAILABLE => "storage is temporarily unavailable".to_owned(),
        _ => error.to_string(),
    };
    if status.is_server_error() {
        tracing::error!(%error, http_status = status.as_u16(), "storage request failed");
    }
    error_response(status, code, &message, request_id)
}

fn error_response(
    status: StatusCode,
    code: &str,
    message: &str,
    request_id: Option<String>,
) -> Response {
    let request_id = request_id.or_else(|| CURRENT_REQUEST_ID.try_with(Clone::clone).ok());
    (
        status,
        Json(ErrorBody {
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                request_id,
            },
        }),
    )
        .into_response()
}

fn header_str(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

//! HTTP routes for health, public catalog/recommendation, admin, and internal jobs.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use mpgs_domain::{FeedSection, FeedbackType, UserPreferences};
use mpgs_recommender::{ALGORITHM_VERSION, RankingInput, rank_feed};
use mpgs_storage::{CreateOverrideRequest, EnqueueJob, Repository, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone)]
pub struct AppState {
    pub repo: Option<Repository>,
    pub admin_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct ReadyResponse {
    status: &'static str,
    database: &'static str,
    schema_version: Option<i64>,
}

#[derive(Debug, Serialize)]
struct MetaResponse {
    api_version: &'static str,
    service_version: &'static str,
    algorithm_version: &'static str,
    supported_sections: Vec<&'static str>,
    ai_available: bool,
    storage_enabled: bool,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
    request_id: Option<String>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/v1/meta", get(meta))
        .route("/v1/session/anonymous", post(create_session))
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
        .layer(middleware::from_fn(request_id_middleware))
        .with_state(Arc::new(state))
}

async fn request_id_middleware(req: axum::http::Request<Body>, next: Next) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(new_request_id);
    let mut response = next.run(req).await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}

fn new_request_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("req-{ms}-{}", ms % 9973)
}

async fn health_live() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "mpgs-server",
        version: env!("CARGO_PKG_VERSION"),
    })
}

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
        Some(repo) => match repo.assert_ready() {
            Ok(()) => (
                StatusCode::OK,
                Json(ReadyResponse {
                    status: "ready",
                    database: "ok",
                    schema_version: repo.database().schema_version().ok(),
                }),
            )
                .into_response(),
            Err(error) => map_storage_error(error, None),
        },
    }
}

async fn meta(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let body = MetaResponse {
        api_version: "v1",
        service_version: env!("CARGO_PKG_VERSION"),
        algorithm_version: ALGORITHM_VERSION,
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
    ([(header::ETAG, etag)], Json(body)).into_response()
}

async fn create_session(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    match repo.create_anonymous_session() {
        Ok(session) => (
            StatusCode::CREATED,
            Json(json!({
                "access_token": session.access_token,
                "refresh_token": session.refresh_token,
                "user_id": session.user_id,
                "expires_at_ms": session.expires_at_ms,
            })),
        )
            .into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

async fn get_preferences(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let user_id = match require_user(repo, &headers) {
        Ok(id) => id,
        Err(resp) => return *resp,
    };
    match repo.get_preferences(&user_id) {
        Ok(prefs) => (StatusCode::OK, Json(prefs)).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

async fn put_preferences(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<UserPreferences>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let user_id = match require_user(repo, &headers) {
        Ok(id) => id,
        Err(resp) => return *resp,
    };
    match repo.put_preferences(&user_id, &body) {
        Ok(prefs) => (StatusCode::OK, Json(prefs)).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

#[derive(Debug, Deserialize)]
struct FeedQuery {
    limit: Option<i64>,
    cursor: Option<String>,
    party_size: Option<u8>,
}

async fn get_feed(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(section): Path<String>,
    Query(query): Query<FeedQuery>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
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
    let mut prefs = prefs_for_request(repo, &headers);
    if let Some(party_size) = query.party_size {
        prefs.party_size = party_size;
    }
    let limit = query.limit.unwrap_or(20).clamp(1, 100) as usize;
    let offset = decode_cursor(query.cursor.as_deref()).unwrap_or(0);

    let candidates = match repo.list_candidates(500) {
        Ok(rows) => rows,
        Err(error) => return map_storage_error(error, None),
    };
    let inputs: Vec<RankingInput> = candidates
        .into_iter()
        .filter(|row| {
            section_matches(
                section,
                row.release_state.as_str(),
                row.release_date.as_deref(),
            )
        })
        .map(|row| RankingInput {
            app_id: row.app_id,
            name: row.name.clone(),
            dominant_mode: row.dominant_mode.clone(),
            recommended_min: row.recommended_min,
            recommended_max: row.recommended_max,
            signals: row.to_ranking_signals(),
        })
        .collect();

    let ranked = rank_feed(section, &inputs, &prefs, Some(0.75));
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

    let next_cursor = if offset + limit < total {
        Some(encode_cursor(offset + limit))
    } else {
        None
    };
    let snapshot_ms = repo.database().now_ms();
    let body = json!({
        "items": page,
        "next_cursor": next_cursor,
        "snapshot_at_ms": snapshot_ms,
        "algorithm_version": ALGORITHM_VERSION,
        "data_updated_at_ms": snapshot_ms,
    });
    let etag = weak_etag(&body.to_string());
    if_none_match_ok(&headers, &etag)
        .unwrap_or_else(|| (StatusCode::OK, [(header::ETAG, etag)], Json(body)).into_response())
}

#[derive(Debug, Deserialize)]
struct CalendarQuery {
    from: Option<String>,
    to: Option<String>,
}

async fn get_calendar(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CalendarQuery>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let from = query.from.unwrap_or_else(|| "2026-01-01".into());
    let to = query.to.unwrap_or_else(|| "2026-12-31".into());
    match repo.list_calendar(&from, &to) {
        Ok((dated, undated)) => {
            let body = json!({
                "dated_items": dated,
                "undated_items": undated,
                "data_updated_at_ms": repo.database().now_ms(),
            });
            let etag = weak_etag(&body.to_string());
            (StatusCode::OK, [(header::ETAG, etag)], Json(body)).into_response()
        }
        Err(error) => map_storage_error(error, None),
    }
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: Option<String>,
    limit: Option<i64>,
}

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
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    match repo.search_games(&q, limit) {
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

async fn get_game(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<u32>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    match repo.game_detail(app_id) {
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
                "algorithm_version": ALGORITHM_VERSION,
                "data_updated_at_ms": repo.database().now_ms(),
            });
            let etag = weak_etag(&body.to_string());
            (StatusCode::OK, [(header::ETAG, etag)], Json(body)).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "not_found", "game not found", None),
        Err(error) => map_storage_error(error, None),
    }
}

#[derive(Debug, Deserialize)]
struct EvidenceQuery {
    feature: Option<String>,
}

async fn get_evidence(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<u32>,
    Query(query): Query<EvidenceQuery>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    match repo.list_evidence(app_id, query.feature.as_deref()) {
        Ok(items) => {
            let body = json!({
                "items": items.iter().map(|e| json!({
                    "evidence_id": format!("feature:{}:{}", e.feature_name, app_id),
                    "feature": e.feature_name,
                    "value": serde_json::from_str::<serde_json::Value>(&e.value_json).unwrap_or(json!(null)),
                    "source_type": e.source_type,
                    "source_label": e.source_ref,
                    "confidence": e.confidence,
                    "observed_at_ms": e.observed_at_ms,
                })).collect::<Vec<_>>(),
            });
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(error) => map_storage_error(error, None),
    }
}

#[derive(Debug, Deserialize)]
struct FeedbackBody {
    app_id: u32,
    #[serde(rename = "type")]
    feedback_type: String,
    recommendation_run_id: Option<String>,
    client_created_at_ms: Option<i64>,
}

async fn post_feedback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<FeedbackBody>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let user_id = match require_user(repo, &headers) {
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
    match repo.create_feedback(
        &user_id,
        body.app_id,
        feedback_type,
        body.recommendation_run_id.as_deref(),
        &idem,
        body.client_created_at_ms,
    ) {
        Ok(record) => (StatusCode::CREATED, Json(record_json(&record))).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

async fn undo_feedback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(feedback_id): Path<i64>,
) -> impl IntoResponse {
    let Some(repo) = require_repo(&state) else {
        return storage_disabled();
    };
    let user_id = match require_user(repo, &headers) {
        Ok(id) => id,
        Err(resp) => return *resp,
    };
    match repo.undo_feedback(&user_id, feedback_id) {
        Ok(record) => (StatusCode::OK, Json(record_json(&record))).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

fn record_json(record: &mpgs_storage::feedback::FeedbackRecord) -> serde_json::Value {
    json!({
        "feedback_id": record.feedback_id,
        "user_id": record.user_id,
        "app_id": record.app_id,
        "type": record.feedback_type,
        "recommendation_run_id": record.recommendation_run_id,
        "idempotency_key": record.idempotency_key,
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
    match repo.create_override(app_id, &request) {
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
    match repo.revoke_override(
        id,
        &body.operator,
        &body.reason,
        header_str(&headers, "x-request-id").as_deref(),
    ) {
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
    let app = match repo.get_app(app_id) {
        Ok(v) => v,
        Err(error) => return map_storage_error(error, None),
    };
    let profile = match repo.get_profile(app_id) {
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
    match repo.enqueue_job(&job) {
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
    match repo.lease_jobs(
        &body.owner,
        body.limit.unwrap_or(10),
        body.lease_ms.unwrap_or(60_000),
        body.source.as_deref(),
    ) {
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
    match repo.complete_job(job_id, &body.owner, &body.idempotency_key) {
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
    match repo.fail_job(
        job_id,
        &body.owner,
        &body.error_category,
        body.retry_delay_ms.unwrap_or(60_000),
    ) {
        Ok(job) => (StatusCode::OK, Json(job)).into_response(),
        Err(error) => map_storage_error(error, None),
    }
}

// --- helpers ---

fn require_repo(state: &AppState) -> Option<&Repository> {
    state.repo.as_ref()
}

fn require_user(repo: &Repository, headers: &HeaderMap) -> Result<String, Box<Response>> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| {
            Box::new(error_response(
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "missing bearer token",
                None,
            ))
        })?;
    repo.resolve_access_token(token)
        .map_err(|e| Box::new(map_storage_error(e, None)))
}

fn prefs_for_request(repo: &Repository, headers: &HeaderMap) -> UserPreferences {
    if let Ok(user_id) = require_user(repo, headers)
        && let Ok(prefs) = repo.get_preferences(&user_id)
    {
        return prefs;
    }
    UserPreferences::default()
}

fn section_matches(section: FeedSection, release_state: &str, release_date: Option<&str>) -> bool {
    match section {
        FeedSection::Upcoming => release_state == "upcoming" || release_state == "coming_soon",
        FeedSection::RecentRelease => {
            release_state == "released" && release_date.is_some_and(|d| d >= "2023-01-01")
        }
        FeedSection::PopularLegacy => {
            // Older released titles; popularity still drives score inside the ranker.
            release_state == "released" && release_date.is_some_and(|d| d < "2023-01-01")
        }
        FeedSection::ClassicLegacy => {
            release_state == "released" && release_date.is_some_and(|d| d < "2023-01-01")
        }
    }
}

fn encode_cursor(offset: usize) -> String {
    format!("o:{offset}")
}

fn decode_cursor(cursor: Option<&str>) -> Option<usize> {
    let cursor = cursor?;
    cursor.strip_prefix("o:")?.parse().ok()
}

fn weak_etag(payload: &str) -> HeaderValue {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in payload.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    HeaderValue::from_str(&format!("W/\"{hash:x}\"")).unwrap_or(HeaderValue::from_static("W/\"0\""))
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
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
    };
    error_response(status, code, &error.to_string(), request_id)
}

fn error_response(
    status: StatusCode,
    code: &str,
    message: &str,
    request_id: Option<String>,
) -> Response {
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

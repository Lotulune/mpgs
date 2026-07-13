#![forbid(unsafe_code)]

use std::{env, error::Error, net::SocketAddr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use mpgs_domain::FeedSection;
use mpgs_storage::{CreateOverrideRequest, Database, EnqueueJob, Repository, StorageError};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";
const ALGORITHM_VERSION: &str = "rules-0.1.0";

#[derive(Clone)]
struct AppState {
    repo: Option<Repository>,
    admin_token: Option<String>,
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
    error: String,
    category: String,
}

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let state = build_state()?;
    let address: SocketAddr = env::var("MPGS_BIND_ADDR")
        .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned())
        .parse()?;
    let listener = TcpListener::bind(address).await?;

    info!(%address, storage = state.repo.is_some(), "mpgs server listening");
    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn build_state() -> Result<AppState, Box<dyn Error>> {
    let admin_token = env::var("MPGS_ADMIN_TOKEN").ok().filter(|s| !s.is_empty());
    let repo = match env::var("MPGS_DATABASE_PATH") {
        Ok(path) if !path.is_empty() => {
            let db = Database::open(&path)?;
            let repo = Repository::new(db);
            let version = repo.migrate()?;
            repo.assert_ready()?;
            info!(%path, version, "database ready");
            Some(repo)
        }
        _ => {
            info!("MPGS_DATABASE_PATH not set; storage routes disabled");
            None
        }
    };
    Ok(AppState { repo, admin_token })
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/v1/meta", get(meta))
        .route("/admin/v1/games/{app_id}/overrides", post(create_override))
        .route("/admin/v1/overrides/{id}/revoke", post(revoke_override))
        .route("/admin/v1/games/{app_id}/debug", get(game_debug))
        .route("/internal/v1/jobs/enqueue", post(enqueue_job))
        .route("/internal/v1/jobs/lease", post(lease_jobs))
        .route("/internal/v1/jobs/{job_id}/complete", post(complete_job))
        .route("/internal/v1/jobs/{job_id}/fail", post(fail_job))
        .with_state(Arc::new(state))
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
            Ok(()) => {
                let version = repo.database().schema_version().ok();
                (
                    StatusCode::OK,
                    Json(ReadyResponse {
                        status: "ready",
                        database: "ok",
                        schema_version: version,
                    }),
                )
                    .into_response()
            }
            Err(error) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorBody {
                    error: error.to_string(),
                    category: "storage".into(),
                }),
            )
                .into_response(),
        },
    }
}

async fn meta(State(state): State<Arc<AppState>>) -> Json<MetaResponse> {
    Json(MetaResponse {
        api_version: "v1",
        service_version: env!("CARGO_PKG_VERSION"),
        algorithm_version: ALGORITHM_VERSION,
        supported_sections: FeedSection::ALL
            .into_iter()
            .map(FeedSection::as_str)
            .collect(),
        ai_available: false,
        storage_enabled: state.repo.is_some(),
    })
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
    let Some(repo) = state.repo.as_ref() else {
        return storage_disabled();
    };
    let request = CreateOverrideRequest {
        feature_name: body.feature_name,
        value_json: body.value,
        reason: body.reason,
        external_evidence: body.external_evidence,
        operator: body.operator,
        request_id: header_request_id(&headers),
    };
    match repo.create_override(app_id, &request) {
        Ok(over) => (StatusCode::CREATED, Json(over)).into_response(),
        Err(error) => map_storage_error(error),
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
    let Some(repo) = state.repo.as_ref() else {
        return storage_disabled();
    };
    match repo.revoke_override(
        id,
        &body.operator,
        &body.reason,
        header_request_id(&headers).as_deref(),
    ) {
        Ok(over) => (StatusCode::OK, Json(over)).into_response(),
        Err(error) => map_storage_error(error),
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
    let Some(repo) = state.repo.as_ref() else {
        return storage_disabled();
    };
    let app = match repo.get_app(app_id) {
        Ok(v) => v,
        Err(error) => return map_storage_error(error),
    };
    let profile = match repo.get_profile(app_id) {
        Ok(v) => v,
        Err(error) => return map_storage_error(error),
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "app": app,
            "multiplayer_profile": profile,
        })),
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
    let Some(repo) = state.repo.as_ref() else {
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
        Ok(job_id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "job_id": job_id })),
        )
            .into_response(),
        Err(error) => map_storage_error(error),
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
    let Some(repo) = state.repo.as_ref() else {
        return storage_disabled();
    };
    match repo.lease_jobs(
        &body.owner,
        body.limit.unwrap_or(10),
        body.lease_ms.unwrap_or(60_000),
        body.source.as_deref(),
    ) {
        Ok(jobs) => (StatusCode::OK, Json(jobs)).into_response(),
        Err(error) => map_storage_error(error),
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
    let Some(repo) = state.repo.as_ref() else {
        return storage_disabled();
    };
    match repo.complete_job(job_id, &body.owner, &body.idempotency_key) {
        Ok(job) => (StatusCode::OK, Json(job)).into_response(),
        Err(error) => map_storage_error(error),
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
    let Some(repo) = state.repo.as_ref() else {
        return storage_disabled();
    };
    match repo.fail_job(
        job_id,
        &body.owner,
        &body.error_category,
        body.retry_delay_ms.unwrap_or(60_000),
    ) {
        Ok(job) => (StatusCode::OK, Json(job)).into_response(),
        Err(error) => map_storage_error(error),
    }
}

fn require_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), Box<axum::response::Response>> {
    let Some(expected) = state.admin_token.as_deref() else {
        return Err(Box::new(
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorBody {
                    error: "MPGS_ADMIN_TOKEN is not configured".into(),
                    category: "config".into(),
                }),
            )
                .into_response(),
        ));
    };
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if provided != Some(expected) {
        return Err(Box::new(
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorBody {
                    error: "invalid admin token".into(),
                    category: "auth".into(),
                }),
            )
                .into_response(),
        ));
    }
    Ok(())
}

fn storage_disabled() -> axum::response::Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody {
            error: "storage is disabled; set MPGS_DATABASE_PATH".into(),
            category: "config".into(),
        }),
    )
        .into_response()
}

fn map_storage_error(error: StorageError) -> axum::response::Response {
    let (status, category) = match &error {
        StorageError::NotFound { .. } => (StatusCode::NOT_FOUND, "not_found"),
        StorageError::Validation { .. } => (StatusCode::BAD_REQUEST, "validation"),
        StorageError::Conflict { .. } => (StatusCode::CONFLICT, "conflict"),
        StorageError::Lease { .. } => (StatusCode::CONFLICT, "lease"),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "storage"),
    };
    (
        status,
        Json(ErrorBody {
            error: error.to_string(),
            category: category.into(),
        }),
    )
        .into_response()
}

fn header_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "failed to install Ctrl+C handler");
    }
}

#[cfg(test)]
mod tests {
    use super::{ALGORITHM_VERSION, AppState, app, health_live, meta};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use mpgs_storage::{Database, Repository};
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn live_health_reports_ok() {
        let response = health_live().await;
        assert_eq!(response.status, "ok");
        assert_eq!(response.service, "mpgs-server");
    }

    #[tokio::test]
    async fn metadata_exposes_all_feed_sections() {
        let state = Arc::new(AppState {
            repo: None,
            admin_token: None,
        });
        let response = meta(axum::extract::State(state)).await;
        assert_eq!(response.api_version, "v1");
        assert_eq!(response.algorithm_version, ALGORITHM_VERSION);
        assert_eq!(response.supported_sections.len(), 4);
        assert!(!response.ai_available);
        assert!(!response.storage_enabled);
    }

    #[tokio::test]
    async fn ready_with_database() {
        let db = Database::open_in_memory().unwrap();
        let repo = Repository::new(db);
        repo.migrate().unwrap();
        let state = AppState {
            repo: Some(repo),
            admin_token: Some("test-token".into()),
        };
        let app = app(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn admin_override_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let repo = Repository::new(db);
        repo.migrate().unwrap();
        let state = AppState {
            repo: Some(repo),
            admin_token: Some("secret".into()),
        };
        let app = app(state);

        let create = Request::builder()
            .method("POST")
            .uri("/admin/v1/games/892970/overrides")
            .header("authorization", "Bearer secret")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"feature_name":"self_hosted_server","value":true,"reason":"manual","operator":"t"}"#,
            ))
            .unwrap();
        let response = app.clone().oneshot(create).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let override_id = json["override_id"].as_i64().unwrap();

        let revoke = Request::builder()
            .method("POST")
            .uri(format!("/admin/v1/overrides/{override_id}/revoke"))
            .header("authorization", "Bearer secret")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"operator":"t","reason":"done"}"#))
            .unwrap();
        let response = app.oneshot(revoke).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

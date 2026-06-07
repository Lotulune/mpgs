use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use mpgs_server::{
    build_router_with_state, AdminAuthConfig, AppState, AuditSink, DatabaseHealth, RateLimitConfig,
    RateLimiters, RestartCoordinator, ServiceInfoConfig,
};
use serde_json::json;
use std::fs;
use std::path::Path;
use tower::ServiceExt;

fn test_config() -> ServiceInfoConfig {
    ServiceInfoConfig {
        service_instance_id: "018fb770-8998-7699-a6e4-b7b59f2f9c01".to_string(),
        service_name: "MPGS Restart Test Service".to_string(),
        service_version: "0.1.0".to_string(),
    }
}

fn write_active_config(config_dir: &Path) {
    let active_dir = config_dir.join("active");
    fs::create_dir(&active_dir).unwrap();
    fs::write(
        active_dir.join("service.toml"),
        r#"
bind_addr = "0.0.0.0:4310"

[service_identity]
instance_id = "018fb770-8998-7699-a6e4-b7b59f2f9c01"
name = "MPGS Active Service"
version = "0.1.0"
"#,
    )
    .unwrap();
    fs::write(
        active_dir.join("secrets.toml"),
        r#"
[database]
url = "postgres://mpgs:secret@postgres:5432/mpgs"

[admin]
token_hash = "sha256:admin-hash"
session_secret = "admin-session-secret"
"#,
    )
    .unwrap();
}

fn write_pending_service_config(config_dir: &Path) {
    let pending_dir = config_dir.join("pending");
    fs::create_dir(&pending_dir).unwrap();
    fs::write(
        pending_dir.join("service.toml"),
        r#"
bind_addr = "0.0.0.0:4310"

[service_identity]
instance_id = "018fb770-8998-7699-a6e4-b7b59f2f9c01"
name = "MPGS Pending Service"
version = "0.1.0"
"#,
    )
    .unwrap();
}

fn restart_app(config_dir: &Path, restart: RestartCoordinator) -> axum::Router {
    restart_app_with_audit(config_dir, restart, AuditSink::Noop)
}

fn restart_app_with_audit(
    config_dir: &Path,
    restart: RestartCoordinator,
    audit: AuditSink,
) -> axum::Router {
    build_router_with_state(
        AppState::new_with_admin_auth(
            test_config().service_info(),
            DatabaseHealth::HealthyForTest,
            AdminAuthConfig::for_test_token("correct-admin-token"),
        )
        .with_config_file_manager(config_dir)
        .with_restart_coordinator(restart)
        .with_audit_sink(audit),
    )
}

fn rate_limited_restart_app(config_dir: &Path, restart: RestartCoordinator) -> axum::Router {
    let rate_limits = RateLimitConfig {
        restart_limit: 1,
        admin_limit: 10,
        ..RateLimitConfig::for_tests(10)
    };

    build_router_with_state(
        AppState::new_with_admin_auth(
            test_config().service_info(),
            DatabaseHealth::HealthyForTest,
            AdminAuthConfig::for_test_token("correct-admin-token"),
        )
        .with_config_file_manager(config_dir)
        .with_restart_coordinator(restart)
        .with_rate_limits(RateLimiters::new(rate_limits)),
    )
}

async fn request_json(
    app: axum::Router,
    method: Method,
    uri: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> (StatusCode, serde_json::Value, axum::http::HeaderMap) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }

    let response = app
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = serde_json::from_slice(&body).unwrap();

    (status, value, headers)
}

async fn admin_cookie(app: axum::Router) -> String {
    let (_status, _value, headers) = request_json(
        app,
        Method::POST,
        "/api/v1/admin/session",
        json!({ "token": "correct-admin-token" }),
        None,
    )
    .await;

    headers
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn restart_requires_admin_session() {
    let temp_dir = tempfile::tempdir().unwrap();
    write_active_config(temp_dir.path());
    write_pending_service_config(temp_dir.path());
    let restart = RestartCoordinator::for_test();

    let (status, value, _headers) = request_json(
        restart_app(temp_dir.path(), restart.clone()),
        Method::POST,
        "/api/v1/admin/restart",
        json!({ "confirm": true }),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(value["error"]["code"], "admin_session_required");
    assert!(!restart.was_requested());
}

#[tokio::test]
async fn restart_requires_explicit_confirmation() {
    let temp_dir = tempfile::tempdir().unwrap();
    write_active_config(temp_dir.path());
    write_pending_service_config(temp_dir.path());
    let restart = RestartCoordinator::for_test();
    let app = restart_app(temp_dir.path(), restart.clone());
    let cookie = admin_cookie(app.clone()).await;

    let (status, value, _headers) = request_json(
        app,
        Method::POST,
        "/api/v1/admin/restart",
        json!({ "confirm": false }),
        Some(&cookie),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(value["error"]["code"], "restart_confirmation_required");
    assert!(!restart.was_requested());
}

#[tokio::test]
async fn restart_requires_valid_pending_config() {
    let temp_dir = tempfile::tempdir().unwrap();
    write_active_config(temp_dir.path());
    let restart = RestartCoordinator::for_test();
    let app = restart_app(temp_dir.path(), restart.clone());
    let cookie = admin_cookie(app.clone()).await;

    let (status, value, _headers) = request_json(
        app,
        Method::POST,
        "/api/v1/admin/restart",
        json!({ "confirm": true }),
        Some(&cookie),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(value["error"]["code"], "pending_config_required");
    assert!(!restart.was_requested());
}

#[tokio::test]
async fn restart_accepts_valid_pending_config_and_requests_self_exit() {
    let temp_dir = tempfile::tempdir().unwrap();
    write_active_config(temp_dir.path());
    write_pending_service_config(temp_dir.path());
    let restart = RestartCoordinator::for_test();
    let app = restart_app(temp_dir.path(), restart.clone());
    let cookie = admin_cookie(app.clone()).await;

    let (status, value, _headers) = request_json(
        app,
        Method::POST,
        "/api/v1/admin/restart",
        json!({ "confirm": true }),
        Some(&cookie),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(value["restartScheduled"], true);
    assert_eq!(value["mode"], "self_exit");
    assert!(restart.was_requested());
}

#[tokio::test]
async fn restart_records_audit_event_when_scheduled() {
    let temp_dir = tempfile::tempdir().unwrap();
    write_active_config(temp_dir.path());
    write_pending_service_config(temp_dir.path());
    let restart = RestartCoordinator::for_test();
    let audit = AuditSink::memory();
    let app = restart_app_with_audit(temp_dir.path(), restart.clone(), audit.clone());
    let cookie = admin_cookie(app.clone()).await;

    let (status, _value, _headers) = request_json(
        app,
        Method::POST,
        "/api/v1/admin/restart",
        json!({ "confirm": true }),
        Some(&cookie),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(restart.was_requested());
    let records = audit.records_for_test();
    assert!(records.iter().any(|record| {
        record.event_type == "admin.restart.requested" && record.outcome == "success"
    }));
}

#[tokio::test]
async fn restart_route_is_rate_limited() {
    let temp_dir = tempfile::tempdir().unwrap();
    write_active_config(temp_dir.path());
    write_pending_service_config(temp_dir.path());
    let restart = RestartCoordinator::for_test();
    let app = rate_limited_restart_app(temp_dir.path(), restart.clone());
    let cookie = admin_cookie(app.clone()).await;

    let (first_status, first_value, _headers) = request_json(
        app.clone(),
        Method::POST,
        "/api/v1/admin/restart",
        json!({ "confirm": true }),
        Some(&cookie),
    )
    .await;
    let (second_status, second_value, _headers) = request_json(
        app,
        Method::POST,
        "/api/v1/admin/restart",
        json!({ "confirm": true }),
        Some(&cookie),
    )
    .await;

    assert_eq!(first_status, StatusCode::ACCEPTED);
    assert_eq!(first_value["restartScheduled"], true);
    assert_eq!(second_status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(second_value["error"]["code"], "rate_limited");
    assert!(restart.was_requested());
}

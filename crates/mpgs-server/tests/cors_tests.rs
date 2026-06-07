use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use mpgs_server::{
    build_router_with_state, AdminAuthConfig, AppState, DatabaseHealth, PublicCorsConfig,
    ServiceInfoConfig,
};
use serde_json::json;
use tower::ServiceExt;

fn test_config() -> ServiceInfoConfig {
    ServiceInfoConfig {
        service_instance_id: "018fb770-8998-7699-a6e4-b7b59f2f9c01".to_string(),
        service_name: "MPGS CORS Test Service".to_string(),
        service_version: "0.1.0".to_string(),
    }
}

fn cors_app() -> axum::Router {
    build_router_with_state(
        AppState::new_with_admin_auth(
            test_config().service_info(),
            DatabaseHealth::HealthyForTest,
            AdminAuthConfig::for_test_token("correct-admin-token"),
        )
        .with_public_cors(PublicCorsConfig::allow_any_origin()),
    )
}

async fn request(
    method: Method,
    uri: &str,
    origin: Option<&str>,
    body: Option<serde_json::Value>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(origin) = origin {
        builder = builder.header(header::ORIGIN, origin);
    }
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }

    cors_app()
        .oneshot(
            builder
                .body(Body::from(body.unwrap_or_else(|| json!({})).to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn public_routes_include_cors_headers_for_allowed_origin() {
    let response = request(
        Method::GET,
        "/api/v1/service-info",
        Some("https://client.example.test"),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&"*".parse().unwrap())
    );
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS),
        None
    );
}

#[tokio::test]
async fn public_read_preflight_allows_conditional_cache_headers() {
    let response = request(
        Method::OPTIONS,
        "/api/v1/discovery-home",
        Some("https://client.example.test"),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&"*".parse().unwrap())
    );
    assert!(response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_METHODS)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("GET"));
    assert!(response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("If-None-Match"));
}

#[tokio::test]
async fn admin_routes_do_not_emit_public_cors_headers() {
    let response = request(
        Method::POST,
        "/api/v1/admin/session",
        Some("https://client.example.test"),
        Some(json!({ "token": "correct-admin-token" })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        None
    );
}

use axum::body::Body;
use axum::http::{Request, StatusCode};
use mpgs_server::{build_router_with_state, AppState, DatabaseHealth, ServiceInfoConfig};
use serde_json::json;
use tower::ServiceExt;

fn test_config() -> ServiceInfoConfig {
    ServiceInfoConfig {
        service_instance_id: "018fb770-8998-7699-a6e4-b7b59f2f9c01".to_string(),
        service_name: "MPGS Health Test Service".to_string(),
        service_version: "0.1.0".to_string(),
    }
}

#[tokio::test]
async fn healthz_returns_ok_without_diagnostics_when_dependencies_are_healthy() {
    let app = build_router_with_state(AppState::new(
        test_config().service_info(),
        DatabaseHealth::HealthyForTest,
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value, json!({ "status": "ok" }));
}

#[tokio::test]
async fn healthz_returns_unavailable_without_leaking_dependency_details() {
    let app = build_router_with_state(AppState::new(
        test_config().service_info(),
        DatabaseHealth::Unavailable,
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value, json!({ "status": "unavailable" }));
    assert!(value.get("databaseUrl").is_none());
    assert!(value.get("details").is_none());
}

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use mpgs_server::{
    build_router_with_state, AdminAuthConfig, AppState, DatabaseHealth, ServiceInfoConfig,
};
use serde_json::json;
use tower::ServiceExt;

fn test_config() -> ServiceInfoConfig {
    ServiceInfoConfig {
        service_instance_id: "018fb770-8998-7699-a6e4-b7b59f2f9c01".to_string(),
        service_name: "MPGS Admin Test Service".to_string(),
        service_version: "0.1.0".to_string(),
    }
}

fn admin_app() -> axum::Router {
    build_router_with_state(AppState::new_with_admin_auth(
        test_config().service_info(),
        DatabaseHealth::HealthyForTest,
        AdminAuthConfig::for_test_token("correct-admin-token"),
    ))
}

async fn request_json(
    method: Method,
    uri: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }

    admin_app()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

async fn get_json(uri: &str, cookie: Option<&str>) -> (StatusCode, serde_json::Value) {
    let response = request_json(Method::GET, uri, json!({}), cookie).await;
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = serde_json::from_slice(&body).unwrap();

    (status, value)
}

#[tokio::test]
async fn admin_overview_requires_session_cookie() {
    let (status, value) = get_json("/api/v1/admin/overview", None).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(value["error"]["code"], "admin_session_required");
}

#[tokio::test]
async fn admin_session_rejects_wrong_token() {
    let response = request_json(
        Method::POST,
        "/api/v1/admin/session",
        json!({ "token": "wrong-token" }),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
}

#[tokio::test]
async fn admin_session_sets_http_only_cookie_for_valid_token() {
    let response = request_json(
        Method::POST,
        "/api/v1/admin/session",
        json!({ "token": "correct-admin-token" }),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("admin login should set a session cookie")
        .to_str()
        .unwrap();

    assert!(cookie.contains("mpgs_admin_session="));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));
}

#[tokio::test]
async fn admin_session_cookie_allows_overview_and_diagnostics() {
    let login_response = request_json(
        Method::POST,
        "/api/v1/admin/session",
        json!({ "token": "correct-admin-token" }),
        None,
    )
    .await;
    let cookie = login_response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    let (overview_status, overview) = get_json("/api/v1/admin/overview", Some(&cookie)).await;
    let (diagnostics_status, diagnostics) =
        get_json("/api/v1/admin/diagnostics", Some(&cookie)).await;

    assert_eq!(overview_status, StatusCode::OK);
    assert_eq!(overview["serviceName"], "MPGS Admin Test Service");
    assert_eq!(overview["publicCatalogStatus"], "empty");
    assert_eq!(diagnostics_status, StatusCode::OK);
    assert_eq!(diagnostics["postgres"], "ok");
    assert!(diagnostics.get("adminToken").is_none());
}

use axum::body::Body;
use axum::http::{Request, StatusCode};
use mpgs_server::{build_router_with_state, AppState, DatabaseHealth, ServiceInfoConfig};
use std::fs;
use tower::ServiceExt;

fn test_config() -> ServiceInfoConfig {
    ServiceInfoConfig {
        service_instance_id: "018fb770-8998-7699-a6e4-b7b59f2f9c01".to_string(),
        service_name: "MPGS Admin Static Test Service".to_string(),
        service_version: "0.1.0".to_string(),
    }
}

#[tokio::test]
async fn admin_static_route_serves_admin_html_entry() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
        temp_dir.path().join("admin.html"),
        r#"<!doctype html><html><body><div id="admin-root"></div><script type="module" src="/assets/admin.js"></script></body></html>"#,
    )
    .unwrap();

    let app = build_router_with_state(
        AppState::new(test_config().service_info(), DatabaseHealth::HealthyForTest)
            .with_admin_static_dir(temp_dir.path()),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    assert!(html.contains(r#"id="admin-root""#));
    assert!(html.contains("/assets/admin.js"));
}

#[tokio::test]
async fn admin_static_deep_links_fall_back_to_admin_html() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
        temp_dir.path().join("admin.html"),
        r#"<!doctype html><html><body><div id="admin-root"></div></body></html>"#,
    )
    .unwrap();

    let app = build_router_with_state(
        AppState::new(test_config().service_info(), DatabaseHealth::HealthyForTest)
            .with_admin_static_dir(temp_dir.path()),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    assert!(html.contains(r#"id="admin-root""#));
}

#[tokio::test]
async fn api_routes_are_not_shadowed_by_admin_static_hosting() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
        temp_dir.path().join("admin.html"),
        r#"<!doctype html><html><body><div id="admin-root"></div></body></html>"#,
    )
    .unwrap();

    let app = build_router_with_state(
        AppState::new(test_config().service_info(), DatabaseHealth::HealthyForTest)
            .with_admin_static_dir(temp_dir.path()),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/service-info")
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

    assert_eq!(value["serviceName"], "MPGS Admin Static Test Service");
}

#[tokio::test]
async fn admin_static_assets_are_served_with_content_types() {
    let temp_dir = tempfile::tempdir().unwrap();
    let assets_dir = temp_dir.path().join("assets");
    fs::create_dir(&assets_dir).unwrap();
    fs::write(assets_dir.join("admin.js"), "console.log('admin');").unwrap();
    fs::write(
        assets_dir.join("admin.css"),
        ".admin-shell { display: grid; }",
    )
    .unwrap();

    let app = build_router_with_state(
        AppState::new(test_config().service_info(), DatabaseHealth::HealthyForTest)
            .with_admin_static_dir(temp_dir.path()),
    );

    let js_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/assets/admin.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(js_response.status(), StatusCode::OK);
    assert_eq!(
        js_response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/javascript; charset=utf-8")
    );

    let css_response = app
        .oneshot(
            Request::builder()
                .uri("/assets/admin.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(css_response.status(), StatusCode::OK);
    assert_eq!(
        css_response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/css; charset=utf-8")
    );
}

#[tokio::test]
async fn admin_static_assets_reject_path_traversal() {
    let temp_dir = tempfile::tempdir().unwrap();
    let assets_dir = temp_dir.path().join("assets");
    fs::create_dir(&assets_dir).unwrap();
    fs::write(temp_dir.path().join("secret.txt"), "secret").unwrap();

    let app = build_router_with_state(
        AppState::new(test_config().service_info(), DatabaseHealth::HealthyForTest)
            .with_admin_static_dir(temp_dir.path()),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/assets/%2e%2e/secret.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_static_routes_return_not_found_when_static_dir_is_not_configured() {
    let app = build_router_with_state(AppState::new(
        test_config().service_info(),
        DatabaseHealth::HealthyForTest,
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

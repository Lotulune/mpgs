use axum::body::Body;
use axum::http::{Request, StatusCode};
use mpgs_server::{build_router_with_state, AppState, DatabaseHealth, ServiceInfoConfig};
use tower::ServiceExt;

fn test_config() -> ServiceInfoConfig {
    ServiceInfoConfig {
        service_instance_id: "018fb770-8998-7699-a6e4-b7b59f2f9c01".to_string(),
        service_name: "MPGS Public Catalog Test Service".to_string(),
        service_version: "0.1.0".to_string(),
    }
}

fn public_empty_app() -> axum::Router {
    build_router_with_state(AppState::new(
        test_config().service_info(),
        DatabaseHealth::HealthyForTest,
    ))
}

async fn get_json(uri: &str) -> (StatusCode, serde_json::Value) {
    let response = public_empty_app()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = serde_json::from_slice(&body).unwrap();

    (status, value)
}

#[tokio::test]
async fn discovery_home_returns_empty_public_catalog_summary() {
    let (status, value) = get_json("/api/v1/discovery-home").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["status"], "empty");
    assert_eq!(value["totalGames"], 0);
    assert_eq!(
        value["sections"]["newlyPublished"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        value["sections"]["highConfidence"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        value["sections"]["recentlyAdded"].as_array().unwrap().len(),
        0
    );
}

#[tokio::test]
async fn games_returns_empty_page_for_empty_public_catalog() {
    let (status, value) = get_json("/api/v1/games?limit=10&offset=0").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["items"].as_array().unwrap().len(), 0);
    assert_eq!(value["page"]["limit"], 10);
    assert_eq!(value["page"]["offset"], 0);
    assert_eq!(value["page"]["total"], 0);
}

#[tokio::test]
async fn game_detail_and_analysis_return_not_found_for_missing_public_game() {
    let (detail_status, detail_value) = get_json("/api/v1/games/730").await;
    let (analysis_status, analysis_value) = get_json("/api/v1/games/730/analysis").await;

    assert_eq!(detail_status, StatusCode::NOT_FOUND);
    assert_eq!(detail_value["error"]["code"], "public_game_not_found");
    assert_eq!(analysis_status, StatusCode::NOT_FOUND);
    assert_eq!(
        analysis_value["error"]["code"],
        "public_game_analysis_not_found"
    );
}

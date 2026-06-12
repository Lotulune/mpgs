use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use mpgs_server::public_catalog::{
    PublicGameAnalysis, PublicGameDetail, PublicGameListItem, PublicReviewSnippet,
};
use mpgs_server::{
    build_router_with_state, AppState, DatabaseHealth, RateLimitConfig, RateLimiters,
    ServiceInfoConfig,
};
use serde_json::json;
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

fn public_fixture_app() -> axum::Router {
    let game = fixture_game_item(730, "Counter-Strike 2");
    let detail = PublicGameDetail { game };
    let analysis = PublicGameAnalysis {
        appid: 730,
        report: json!({ "overview": "Public analysis fixture." }),
        generated_at: "2026-06-08 03:10:00+00".to_string(),
    };

    build_router_with_state(AppState::new(
        test_config().service_info(),
        DatabaseHealth::PublicCatalogFixture {
            revision: 7,
            detail: Some(detail),
            analysis: Some(analysis),
        },
    ))
}

fn fixture_game_item(appid: u32, name: &str) -> PublicGameListItem {
    PublicGameListItem {
        appid,
        name: name.to_string(),
        short_description: Some("Fixture public game detail.".to_string()),
        section: "classic".to_string(),
        release_date: Some("2026-06-08".to_string()),
        release_date_text: "Jun 8, 2026".to_string(),
        release_state: "released".to_string(),
        demo_status: "released_with_demo".to_string(),
        supported_languages: vec!["English".to_string()],
        is_adult_content: false,
        is_free: false,
        price_text: Some("$19.99".to_string()),
        discount_percent: Some(10),
        positive_review_pct: Some(93.0),
        total_reviews: Some(12_000),
        current_players: Some(4_200),
        recommendation_score: Some(91.5),
        capsule_url: format!("https://cdn.example.test/{appid}.jpg"),
        store_screenshot_urls: vec![format!("https://cdn.example.test/{appid}-1.jpg")],
        tags: vec!["Co-op".to_string()],
        multiplayer_modes: vec!["Online Co-op".to_string()],
        review_snippets: vec![PublicReviewSnippet {
            voted_up: true,
            review: "Public fixture review.".to_string(),
            playtime_hours: Some(12.0),
        }],
        updated_at: "2026-06-08 03:00:00+00".to_string(),
    }
}

fn public_unavailable_app() -> axum::Router {
    build_router_with_state(AppState::new(
        test_config().service_info(),
        DatabaseHealth::Unavailable,
    ))
}

fn rate_limited_public_app() -> axum::Router {
    build_router_with_state(
        AppState::new(test_config().service_info(), DatabaseHealth::HealthyForTest)
            .with_rate_limits(RateLimiters::new(RateLimitConfig::for_tests(1))),
    )
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

async fn get_response(uri: &str) -> axum::response::Response {
    public_empty_app()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn get_response_from(app: axum::Router, uri: &str) -> axum::response::Response {
    app.oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
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
async fn discovery_home_supports_etag_and_conditional_reads() {
    let first_response = get_response("/api/v1/discovery-home").await;

    assert_eq!(first_response.status(), StatusCode::OK);
    let etag = first_response
        .headers()
        .get(header::ETAG)
        .expect("discovery-home should expose an ETag")
        .to_str()
        .unwrap()
        .to_string();

    let second_response = public_empty_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/discovery-home")
                .header(header::IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(second_response.status(), StatusCode::NOT_MODIFIED);
    let body = axum::body::to_bytes(second_response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(body.is_empty());
}

#[tokio::test]
async fn games_etag_includes_normalized_pagination() {
    let first_response = get_response("/api/v1/games?limit=10&offset=0").await;
    let second_response = get_response("/api/v1/games?limit=11&offset=0").await;

    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(second_response.status(), StatusCode::OK);

    let first_etag = first_response.headers().get(header::ETAG).unwrap();
    let second_etag = second_response.headers().get(header::ETAG).unwrap();

    assert_ne!(first_etag, second_etag);
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

#[tokio::test]
async fn unavailable_database_returns_public_catalog_unavailable_not_safe_mode() {
    let response = get_response_from(public_unavailable_app(), "/api/v1/discovery-home").await;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value["error"]["code"], "public_catalog_unavailable");
}

#[tokio::test]
async fn game_detail_supports_etag_and_conditional_reads_for_public_game() {
    let first_response = get_response_from(public_fixture_app(), "/api/v1/games/730").await;

    assert_eq!(first_response.status(), StatusCode::OK);
    let etag = first_response
        .headers()
        .get(header::ETAG)
        .expect("game detail should expose an ETag")
        .to_str()
        .unwrap()
        .to_string();

    let second_response = public_fixture_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/games/730")
                .header(header::IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(second_response.status(), StatusCode::NOT_MODIFIED);
    let body = axum::body::to_bytes(second_response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(body.is_empty());
}

#[tokio::test]
async fn game_analysis_supports_etag_and_conditional_reads_for_public_analysis() {
    let first_response =
        get_response_from(public_fixture_app(), "/api/v1/games/730/analysis").await;

    assert_eq!(first_response.status(), StatusCode::OK);
    let etag = first_response
        .headers()
        .get(header::ETAG)
        .expect("game analysis should expose an ETag")
        .to_str()
        .unwrap()
        .to_string();

    let second_response = public_fixture_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/games/730/analysis")
                .header(header::IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(second_response.status(), StatusCode::NOT_MODIFIED);
    let body = axum::body::to_bytes(second_response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(body.is_empty());
}

#[tokio::test]
async fn public_read_routes_are_rate_limited() {
    let app = rate_limited_public_app();
    let first_response = get_response_from(app.clone(), "/api/v1/discovery-home").await;
    let second_response = get_response_from(app, "/api/v1/discovery-home").await;

    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(second_response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = axum::body::to_bytes(second_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["error"]["code"], "rate_limited");
}

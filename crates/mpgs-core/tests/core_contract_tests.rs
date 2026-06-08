use mpgs_core::analysis::build_rule_report;
use mpgs_core::models::{
    AnalysisSource, GameCard, PublicCatalogStatus, ReviewSnippet, ServiceCapability, ServiceInfo,
    StoreReleaseState, UserGameState,
};
use mpgs_core::recommendation::{compute_recommendation_score, DemoStatus, GameFacts};
use mpgs_core::steam_mapping::{build_discovered_game_card, SteamAppListItem, SteamGameSnapshot};

fn fixture_game() -> GameCard {
    GameCard {
        appid: 7301,
        name: "Harbor Crew".to_string(),
        short_description: Some("A cooperative harbor sim with short-session runs.".to_string()),
        section: "new".to_string(),
        release_date: Some("2026-03-18".to_string()),
        release_date_text: "2026-03-18".to_string(),
        release_state: StoreReleaseState::Released,
        demo_status: DemoStatus::ReleasedWithDemo,
        supported_languages: vec!["English".to_string(), "Simplified Chinese".to_string()],
        is_adult_content: false,
        is_free: false,
        price_text: Some("$19.99".to_string()),
        discount_percent: Some(15),
        positive_review_pct: Some(91.0),
        total_reviews: Some(1248),
        current_players: Some(1860),
        recommendation_score: 86.0,
        ai_score: Some(88.0),
        ai_summary: "placeholder".to_string(),
        capsule_url: "https://example.test/capsule.jpg".to_string(),
        store_screenshot_urls: vec![],
        tags: vec!["Co-op".to_string(), "Simulation".to_string()],
        multiplayer_modes: vec!["Online Co-op".to_string(), "LAN Co-op".to_string()],
        review_snippets: vec![ReviewSnippet {
            voted_up: true,
            review: "Great with friends and easy to teach.".to_string(),
            playtime_hours: Some(18.4),
        }],
        user_state: UserGameState::default(),
    }
}

#[test]
fn core_exports_pure_scoring_and_rule_analysis() {
    let facts = GameFacts {
        appid: 1,
        name: "Moonlit Co-op".to_string(),
        release_date: Some("2026-04-20".to_string()),
        positive_review_pct: Some(92.0),
        total_reviews: Some(820),
        current_players: Some(1200),
        multiplayer_modes: vec!["Online Co-op".to_string(), "Co-op".to_string()],
        demo_status: DemoStatus::ReleasedWithDemo,
        ai_score: None,
    };

    let score = compute_recommendation_score(&facts, "2026-04-26");
    let report = build_rule_report(&fixture_game(), "2026-04-30T12:00:00Z".to_string()).unwrap();

    assert!((60.0..80.0).contains(&score), "score was {score}");
    assert_eq!(report.source, AnalysisSource::Rule);
    assert_eq!(report.dimension_scores.len(), 6);
}

#[test]
fn service_info_model_serializes_public_identity_contract() {
    let info = ServiceInfo {
        service_instance_id: "018fb770-8998-7699-a6e4-b7b59f2f9c01".to_string(),
        service_name: "MPGS Test Service".to_string(),
        service_version: "0.1.0".to_string(),
        api_version: "v1".to_string(),
        public_catalog_status: PublicCatalogStatus::Empty,
        capabilities: vec![ServiceCapability::PublicCatalogRead],
    };

    let value = serde_json::to_value(&info).unwrap();

    assert_eq!(value["apiVersion"], "v1");
    assert_eq!(value["publicCatalogStatus"], "empty");
    assert_eq!(value["capabilities"][0], "public_catalog_read");
}

#[test]
fn core_exports_pure_steam_discovery_mapping() {
    let app = SteamAppListItem {
        appid: 3_900_001,
        name: "Fallback Name".to_string(),
    };
    let snapshot = SteamGameSnapshot {
        name: Some("Moonbase Kitchen Panic".to_string()),
        short_description: Some("A readable co-op kitchen scramble.".to_string()),
        release_date: Some("2026-04-20".to_string()),
        release_date_text: Some("Apr 20, 2026".to_string()),
        release_state: Some(StoreReleaseState::Released),
        demo_status: DemoStatus::ReleasedWithDemo,
        supported_languages: Some(vec!["English".to_string()]),
        is_adult_content: Some(false),
        is_free: Some(false),
        price_text: Some("$19.99".to_string()),
        discount_percent: Some(20),
        positive_review_pct: Some(93.0),
        total_reviews: Some(240),
        current_players: Some(88),
        capsule_url: None,
        store_screenshot_urls: Vec::new(),
        tags: vec!["Co-op".to_string()],
        multiplayer_modes: vec!["Online Co-op".to_string()],
        review_snippets: Vec::new(),
    };

    let card = build_discovered_game_card(&app, snapshot, "2026-04-26")
        .expect("multiplayer Steam snapshot should map to a game card");

    assert_eq!(card.appid, 3_900_001);
    assert_eq!(card.name, "Moonbase Kitchen Panic");
    assert_eq!(card.section, "new");
    assert_eq!(
        card.capsule_url,
        "https://cdn.cloudflare.steamstatic.com/steam/apps/3900001/header.jpg"
    );
    assert!(card.recommendation_score > 70.0);
}

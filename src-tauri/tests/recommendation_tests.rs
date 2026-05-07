use tauri_app_lib::recommendation::{
    bucket_game, compute_recommendation_score, DemoStatus, GameFacts, ReleaseBucket,
};
use tauri_app_lib::{
    ai_recommendation::recommend_games_locally,
    models::{
        AiRecommendationMessage, AiRecommendationRequest, GameCard, StoreReleaseState,
        UserGameState,
    },
};

fn base_facts() -> GameFacts {
    GameFacts {
        appid: 1,
        name: "Moonlit Co-op".to_string(),
        release_date: Some("2026-04-20".to_string()),
        positive_review_pct: Some(92.0),
        total_reviews: Some(820),
        current_players: Some(1200),
        multiplayer_modes: vec!["Online Co-op".to_string(), "Co-op".to_string()],
        demo_status: DemoStatus::ReleasedWithDemo,
        ai_score: Some(88.0),
    }
}

fn recommendation_request(prompt: &str) -> AiRecommendationRequest {
    AiRecommendationRequest {
        prompt: prompt.to_string(),
        context_messages: Vec::<AiRecommendationMessage>::new(),
        limit: Some(5),
    }
}

fn card(
    appid: u32,
    name: &str,
    section: &str,
    release_state: StoreReleaseState,
    tags: &[&str],
    multiplayer_modes: &[&str],
    recommendation_score: f64,
) -> GameCard {
    GameCard {
        appid,
        name: name.to_string(),
        short_description: Some(format!("{name} test fixture")),
        section: section.to_string(),
        release_date: Some("2025-01-01".to_string()),
        release_date_text: "2025-01-01".to_string(),
        release_state,
        demo_status: DemoStatus::Released,
        supported_languages: vec!["Simplified Chinese".to_string()],
        is_adult_content: false,
        is_free: false,
        price_text: Some("$9.99".to_string()),
        discount_percent: None,
        positive_review_pct: Some(88.0),
        total_reviews: Some(900),
        current_players: Some(320),
        recommendation_score,
        ai_score: None,
        ai_summary: format!("{name} summary"),
        capsule_url: format!("https://example.test/{appid}.jpg"),
        store_screenshot_urls: vec![],
        tags: tags.iter().map(|tag| tag.to_string()).collect(),
        multiplayer_modes: multiplayer_modes
            .iter()
            .map(|mode| mode.to_string())
            .collect(),
        review_snippets: vec![],
        user_state: UserGameState::default(),
    }
}

#[test]
fn assistant_recommendation_searches_hidden_released_games_but_excludes_upcoming() {
    let hidden = card(
        10,
        "Couch Cat Quest",
        "classic_hidden",
        StoreReleaseState::Released,
        &["Cute", "Casual", "Puzzle"],
        &["Local Co-op", "Shared/Split Screen Co-op"],
        72.0,
    );
    let upcoming = card(
        11,
        "Future Couch Cat Quest",
        "new",
        StoreReleaseState::Upcoming,
        &["Cute", "Casual", "Puzzle"],
        &["Local Co-op"],
        99.0,
    );
    let shooter = card(
        12,
        "Hardcore Raid",
        "classic",
        StoreReleaseState::Released,
        &["Shooter", "Difficult"],
        &["Online Co-op"],
        91.0,
    );

    let response = recommend_games_locally(
        &[upcoming, shooter, hidden],
        &recommendation_request("想找本地合作，画风可爱，轻松一点"),
    );

    assert_eq!(response.items[0].game.appid, 10);
    assert!(response.items.iter().all(|item| item.game.appid != 11));
    assert!(response.items[0]
        .matched_traits
        .iter()
        .any(|trait_label| trait_label == "本地合作"));
}

#[test]
fn assistant_recommendation_marks_near_matches_when_no_game_hits_every_trait() {
    let online_survival = card(
        20,
        "Orbit Survival",
        "classic",
        StoreReleaseState::Released,
        &["Survival", "Co-op"],
        &["Online Co-op"],
        84.0,
    );
    let local_party = card(
        21,
        "Local Party",
        "new",
        StoreReleaseState::Released,
        &["Party", "Casual"],
        &["Local Co-op"],
        76.0,
    );

    let response = recommend_games_locally(
        &[online_survival, local_party],
        &recommendation_request("想找 6 人以上 本地合作 生存 不要像素风"),
    );

    assert!(response.exact_match_count < response.items.len());
    assert!(response.reply.contains("没有完全匹配"));
    assert!(response.follow_up_question.is_some());
    assert!(response
        .items
        .iter()
        .any(|item| !item.missing_traits.is_empty()));
}

#[test]
fn assistant_recommendation_uses_review_quality_when_intent_match_is_equal() {
    let mut polished = card(
        30,
        "Polished Couch Adventure",
        "classic",
        StoreReleaseState::Released,
        &["Casual", "Cute", "Puzzle"],
        &["Local Co-op", "Shared/Split Screen Co-op"],
        88.0,
    );
    polished.positive_review_pct = Some(96.0);
    polished.total_reviews = Some(18_000);
    polished.current_players = Some(2_400);
    polished.ai_score = Some(90.0);

    let mut shaky = card(
        31,
        "Shaky Couch Adventure",
        "new",
        StoreReleaseState::Released,
        &["Casual", "Cute", "Puzzle"],
        &["Local Co-op", "Shared/Split Screen Co-op"],
        88.0,
    );
    shaky.positive_review_pct = Some(61.0);
    shaky.total_reviews = Some(34);
    shaky.current_players = Some(3);
    shaky.ai_score = Some(90.0);

    let response = recommend_games_locally(
        &[shaky, polished],
        &recommendation_request("想找本地合作，可爱，轻松解谜"),
    );

    assert_eq!(response.items[0].game.appid, 30);
    assert!(response.items[0].match_score > response.items[1].match_score);
    assert!(response.items[0].match_score <= 100.0);
    assert!(response.items[0]
        .reason
        .contains("好评"));
}

#[test]
fn scoring_rewards_balanced_multiplayer_recommendations() {
    let score = compute_recommendation_score(&base_facts(), "2026-04-26");

    assert!((88.0..=94.0).contains(&score));
}

#[test]
fn scoring_keeps_low_player_hidden_gems_above_burial_threshold() {
    let mut facts = base_facts();
    facts.positive_review_pct = Some(97.0);
    facts.total_reviews = Some(180);
    facts.current_players = Some(18);
    facts.ai_score = Some(91.0);

    let score = compute_recommendation_score(&facts, "2026-04-26");

    assert!(score >= 80.0, "score was {score}");
}

#[test]
fn scoring_without_ai_score_does_not_get_a_hidden_default_boost() {
    let mut facts = base_facts();
    facts.ai_score = None;

    let score = compute_recommendation_score(&facts, "2026-04-26");

    assert!(
        score < 80.0,
        "score should stay conservative without a real AI score, got {score}"
    );
}

#[test]
fn bucket_game_splits_recent_and_classic_games() {
    assert_eq!(bucket_game(&base_facts(), "2026-04-26"), ReleaseBucket::New);

    let mut hidden_classic = base_facts();
    hidden_classic.release_date = Some("2020-05-13".to_string());

    assert_eq!(
        bucket_game(&hidden_classic, "2026-04-26"),
        ReleaseBucket::ClassicHidden
    );

    let mut classic = hidden_classic;
    classic.total_reviews = Some(1_200);

    assert_eq!(bucket_game(&classic, "2026-04-26"), ReleaseBucket::Classic);
}

use crate::scoring::signals::CanonicalGameSignals;

pub fn freshness_score(signals: &CanonicalGameSignals) -> f64 {
    match signals.release.release_age_days {
        Some(days) if days <= 30 => 100.0,
        Some(days) if days <= 90 => 75.0,
        Some(days) if days <= 365 => 45.0,
        Some(_) => 25.0,
        None => 35.0,
    }
}

pub fn score_discovery_value(signals: &CanonicalGameSignals, review_quality: f64) -> f64 {
    let freshness = freshness_score(signals);
    let total_reviews = signals.review_stats.total_reviews;
    let positive_pct = signals.review_stats.positive_review_pct.unwrap_or(0.0);
    let current_players = signals.activity.current_players.unwrap_or(0);
    let sleeper_score = if review_quality >= 78.0 && total_reviews < 500 {
        25.0
    } else {
        0.0
    } + if current_players < 300 && positive_pct >= 85.0 {
        20.0
    } else {
        0.0
    } + if signals.demo.has_demo { 15.0 } else { 0.0 }
        + if signals.release.recent_release {
            10.0
        } else {
            0.0
        };
    let demo_potential_score = if signals.demo.is_demo_only {
        60.0
    } else if signals.demo.has_demo {
        45.0
    } else if signals.release.recent_release {
        25.0
    } else {
        10.0
    };

    (0.45 * freshness + 0.40 * sleeper_score + 0.15 * demo_potential_score).clamp(0.0, 100.0)
}

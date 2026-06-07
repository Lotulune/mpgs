use crate::scoring::signals::ReviewStats;

pub fn review_confidence(stats: &ReviewStats) -> f64 {
    1.0 - (-(stats.total_reviews as f64) / 120.0).exp()
}

pub fn score_review_quality(stats: &ReviewStats) -> f64 {
    let raw_positive_rate = stats.positive_review_pct.unwrap_or(0.0).clamp(0.0, 100.0) / 100.0;
    let total_reviews = stats.total_reviews as f64;
    let positive = total_reviews * raw_positive_rate;
    let bayes_positive_rate = (positive + 35.0) / (total_reviews + 35.0 + 15.0);
    let confidence = review_confidence(stats);

    (100.0 * (bayes_positive_rate * 0.85 + raw_positive_rate * 0.15) * confidence
        + 55.0 * (1.0 - confidence))
        .clamp(0.0, 100.0)
}

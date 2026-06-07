use crate::scoring::review_quality::review_confidence;
use crate::scoring::signals::CanonicalGameSignals;

pub fn score_confidence(signals: &CanonicalGameSignals) -> f64 {
    if !signals.multiplayer_modes.has_any {
        return 0.30;
    }

    let review_confidence = review_confidence(&signals.review_stats);
    let mode_confidence = (signals.multiplayer_modes.signal_count as f64 / 3.0).min(1.0);
    let activity_confidence = if signals.activity.current_players.is_some() {
        1.0
    } else {
        0.35
    };
    let text_confidence = (signals.review_stats.analyzed_review_count as f64 / 30.0).min(1.0);

    (0.40 * review_confidence
        + 0.25 * mode_confidence
        + 0.20 * activity_confidence
        + 0.15 * text_confidence)
        .clamp(0.0, 1.0)
}

use crate::scoring::signals::CanonicalGameSignals;

pub fn score_content_depth(signals: &CanonicalGameSignals) -> f64 {
    let tag_variety_score = (signals.tags.len() as f64 * 2.0).min(12.0);
    let replayability_score = if has_any_tag(signals, &["ROGUELIKE", "PVP"]) {
        10.0
    } else {
        0.0
    } + (signals.review_topics.replayability.positive as f64 * 6.0)
        .min(8.0);
    let progression_score = if has_any_tag(signals, &["SIMULATION"]) {
        5.0
    } else {
        0.0
    } + (signals.review_topics.progression.positive as f64 * 4.0).min(7.0);
    let mode_variety_score = (signals.review_topics.mode_variety.positive as f64 * 4.0).min(6.0)
        + (signals.multiplayer_modes.raw_mode_count as f64 * 1.5).min(4.0);
    let repetition_penalty = ((signals.review_topics.repetition.negative
        + signals.review_topics.content_depth.negative) as f64
        * 6.0)
        .min(18.0);
    let thin_content_penalty =
        (signals.review_topics.content_depth.negative as f64 * 7.5).min(15.0);

    (45.0 + tag_variety_score + replayability_score + progression_score + mode_variety_score
        - repetition_penalty
        - thin_content_penalty)
        .clamp(0.0, 100.0)
}

fn has_any_tag(signals: &CanonicalGameSignals, tags: &[&str]) -> bool {
    tags.iter()
        .any(|expected| signals.tags.iter().any(|tag| tag == expected))
}

use crate::scoring::signals::{CanonicalGameSignals, LanguageCode};

pub fn score_accessibility(signals: &CanonicalGameSignals) -> f64 {
    let demo_bonus = if signals.demo.has_demo { 10.0 } else { 0.0 };
    let language_bonus = if signals
        .language_codes
        .iter()
        .any(|code| matches!(code, LanguageCode::ZhCn | LanguageCode::ZhTw))
    {
        12.0
    } else {
        0.0
    };
    let casual_bonus = if has_any_tag(signals, &["CASUAL", "PARTY"]) {
        8.0
    } else {
        0.0
    } + if signals.review_topics.casual.positive > 0 {
        4.0
    } else {
        0.0
    };
    let controller_bonus = if has_any_tag(signals, &["CONTROLLER"]) {
        5.0
    } else {
        0.0
    } + if signals.review_topics.controller.positive > 0 {
        3.0
    } else {
        0.0
    };
    let tutorial_bonus = if signals.review_topics.tutorial.positive > 0 {
        5.0
    } else {
        0.0
    };
    let complexity_penalty = ((signals.review_topics.complexity.negative as f64) * 4.0
        + if has_any_tag(signals, &["SIMULATION"]) && !has_any_tag(signals, &["CASUAL"]) {
            3.0
        } else {
            0.0
        })
    .min(10.0);
    let localization_complaint_penalty =
        (signals.review_topics.localization.negative as f64 * 5.0).min(10.0);

    (38.0 + demo_bonus + language_bonus + casual_bonus + controller_bonus + tutorial_bonus
        - complexity_penalty
        - localization_complaint_penalty)
        .clamp(0.0, 100.0)
}

fn has_any_tag(signals: &CanonicalGameSignals, tags: &[&str]) -> bool {
    tags.iter()
        .any(|expected| signals.tags.iter().any(|tag| tag == expected))
}

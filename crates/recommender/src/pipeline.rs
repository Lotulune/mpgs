use mpgs_domain::{
    CandidateAvailability, FeedSection, RankingSignals, RecommendationConfig, SteamAppId,
    UserPreferences,
};
use serde::{Deserialize, Serialize};

use crate::ALGORITHM_VERSION;
use crate::ScoreBreakdown;
use crate::explain::{Explanation, explain};
use crate::mmr::mmr_rerank;
use crate::personalize::{apply_personalization, hard_filter};
use crate::score;

#[derive(Debug, Clone, PartialEq)]
pub struct RankingInput {
    pub app_id: SteamAppId,
    pub name: String,
    pub dominant_mode: Option<String>,
    pub recommended_min: Option<u8>,
    pub recommended_max: Option<u8>,
    pub availability: CandidateAvailability,
    pub signals: RankingSignals,
    pub personal_adjustment: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankedCandidate {
    pub app_id: SteamAppId,
    pub name: String,
    pub dominant_mode: Option<String>,
    pub recommended_min: Option<u8>,
    pub recommended_max: Option<u8>,
    pub score: ScoreBreakdown,
    pub explanation: Explanation,
    pub algorithm_version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RankedFeed {
    pub section: FeedSection,
    pub algorithm_version: String,
    pub items: Vec<RankedCandidate>,
}

pub fn rank_feed(
    section: FeedSection,
    candidates: &[RankingInput],
    prefs: &UserPreferences,
    mmr_lambda: Option<f64>,
) -> RankedFeed {
    rank_feed_inner(
        section,
        candidates,
        prefs,
        mmr_lambda.unwrap_or(RecommendationConfig::default().mmr_lambda),
        ALGORITHM_VERSION,
    )
}

pub fn rank_feed_configured(
    section: FeedSection,
    candidates: &[RankingInput],
    prefs: &UserPreferences,
    config: &RecommendationConfig,
    algorithm_version: &str,
) -> RankedFeed {
    rank_feed_inner(
        section,
        candidates,
        prefs,
        config.mmr_lambda,
        algorithm_version,
    )
}

fn rank_feed_inner(
    section: FeedSection,
    candidates: &[RankingInput],
    prefs: &UserPreferences,
    mmr_lambda: f64,
    algorithm_version: &str,
) -> RankedFeed {
    let mut scored = Vec::new();
    for candidate in candidates {
        if !hard_filter(
            prefs,
            candidate.recommended_min,
            candidate.recommended_max,
            candidate.dominant_mode.as_deref(),
            &candidate.signals,
            &candidate.availability,
        ) {
            continue;
        }
        let mut signals = candidate.signals;
        apply_personalization(
            prefs,
            &mut signals,
            candidate.recommended_min,
            candidate.recommended_max,
            &candidate.availability,
        );
        signals.personal_fit =
            crate::unit(signals.personal_fit + candidate.personal_adjustment.clamp(-0.5, 0.5));
        let breakdown = score(section, &signals, None);
        let explanation = explain(
            candidate.app_id,
            &signals,
            &breakdown,
            candidate.dominant_mode.as_deref(),
        );
        scored.push(RankedCandidate {
            app_id: candidate.app_id,
            name: candidate.name.clone(),
            dominant_mode: candidate.dominant_mode.clone(),
            recommended_min: candidate.recommended_min,
            recommended_max: candidate.recommended_max,
            score: breakdown,
            explanation,
            algorithm_version: algorithm_version.to_owned(),
        });
    }

    let items = mmr_rerank(scored, mmr_lambda, 2);
    RankedFeed {
        section,
        algorithm_version: algorithm_version.to_owned(),
        items,
    }
}

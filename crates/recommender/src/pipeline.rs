use mpgs_domain::{FeedSection, RankingSignals, SteamAppId, UserPreferences};
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
    pub signals: RankingSignals,
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
    pub algorithm_version: &'static str,
    pub items: Vec<RankedCandidate>,
}

pub fn rank_feed(
    section: FeedSection,
    candidates: &[RankingInput],
    prefs: &UserPreferences,
    mmr_lambda: Option<f64>,
) -> RankedFeed {
    let mut scored = Vec::new();
    for candidate in candidates {
        if !hard_filter(
            prefs,
            candidate.recommended_min,
            candidate.recommended_max,
            candidate.dominant_mode.as_deref(),
            &candidate.signals,
        ) {
            continue;
        }
        let mut signals = candidate.signals;
        apply_personalization(
            prefs,
            &mut signals,
            candidate.recommended_min,
            candidate.recommended_max,
        );
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
            algorithm_version: ALGORITHM_VERSION.to_owned(),
        });
    }

    let items = mmr_rerank(scored, mmr_lambda.unwrap_or(0.75), 2);
    RankedFeed {
        section,
        algorithm_version: ALGORITHM_VERSION,
        items,
    }
}

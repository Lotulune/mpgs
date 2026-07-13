#![forbid(unsafe_code)]

use mpgs_domain::{FeedSection, MultiplayerSignals, RankingSignals};
use serde::{Deserialize, Serialize};

const PERSONAL_WEIGHT: f64 = 0.25;
const AI_WEIGHT: f64 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AiAdjustment {
    pub fit: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub friend_fit: f64,
    pub section_score: f64,
    pub personalized_score: f64,
    pub final_score: f64,
}

pub fn score(
    section: FeedSection,
    signals: &RankingSignals,
    ai: Option<AiAdjustment>,
) -> ScoreBreakdown {
    let friend_fit = friend_fit(&signals.multiplayer);
    let section_score = section_score(section, signals, friend_fit);
    let personalized_score = blend_personal_fit(section_score, signals.personal_fit);
    let final_score = blend_ai(personalized_score, ai);

    ScoreBreakdown {
        friend_fit,
        section_score,
        personalized_score,
        final_score,
    }
}

pub fn friend_fit(signals: &MultiplayerSignals) -> f64 {
    let base = 0.22 * unit(signals.private_session)
        + 0.20 * unit(signals.self_host_or_dedicated)
        + 0.18 * unit(signals.online_coop)
        + 0.15 * unit(signals.group_size_fit)
        + 0.10 * unit(signals.low_public_population_dependency)
        + 0.08 * unit(signals.drop_in_out)
        + 0.07 * unit(signals.cross_platform_fit);

    let penalty = 0.18 * unit(signals.matchmaking_core)
        + 0.15 * unit(signals.public_world_dependency)
        + 0.10 * unit(signals.group_size_mismatch)
        + 0.08 * unit(signals.service_shutdown_risk)
        + 0.06 * unit(signals.external_account_friction)
        + 0.05 * unit(signals.platform_or_anticheat_restriction);

    unit(base - penalty)
}

pub fn section_score(section: FeedSection, signals: &RankingSignals, friend_fit: f64) -> f64 {
    let raw = match section {
        FeedSection::RecentRelease => {
            0.35 * friend_fit
                + 0.22 * unit(signals.quality)
                + 0.15 * unit(signals.momentum)
                + 0.10 * unit(signals.evidence)
                + 0.10 * unit(signals.freshness)
                + 0.08 * unit(signals.data_confidence)
        }
        FeedSection::Upcoming => {
            0.40 * friend_fit
                + 0.25 * unit(signals.demo_playability)
                + 0.12 * unit(signals.release_date_confidence)
                + 0.10 * unit(signals.release_proximity)
                + 0.08 * unit(signals.studio_prior)
                + 0.05 * unit(signals.data_confidence)
        }
        FeedSection::PopularLegacy => {
            0.35 * friend_fit
                + 0.32 * unit(signals.popularity)
                + 0.12 * unit(signals.quality)
                + 0.10 * unit(signals.momentum)
                + 0.11 * unit(signals.data_confidence)
        }
        FeedSection::ClassicLegacy => {
            0.40 * friend_fit
                + 0.30 * unit(signals.quality)
                + 0.18 * unit(signals.evidence)
                + 0.08 * unit(signals.longevity)
                + 0.04 * unit(signals.maintenance_health)
        }
    };

    unit(raw - unit(signals.risk))
}

pub fn blend_personal_fit(base: f64, personal_fit: f64) -> f64 {
    unit((1.0 - PERSONAL_WEIGHT) * unit(base) + PERSONAL_WEIGHT * unit(personal_fit))
}

pub fn blend_ai(base: f64, ai: Option<AiAdjustment>) -> f64 {
    let base = unit(base);
    let Some(ai) = ai else {
        return base;
    };

    let confidence = unit(ai.confidence);
    let effective = confidence * unit(ai.fit) + (1.0 - confidence) * base;
    unit((1.0 - AI_WEIGHT) * base + AI_WEIGHT * effective)
}

fn unit(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{AiAdjustment, blend_ai, friend_fit, score};
    use mpgs_domain::{FeedSection, MultiplayerSignals, RankingSignals};

    fn cooperative_signals() -> MultiplayerSignals {
        MultiplayerSignals {
            private_session: 1.0,
            self_host_or_dedicated: 0.8,
            online_coop: 1.0,
            group_size_fit: 1.0,
            low_public_population_dependency: 1.0,
            drop_in_out: 0.8,
            cross_platform_fit: 0.5,
            ..Default::default()
        }
    }

    fn matchmaking_signals() -> MultiplayerSignals {
        MultiplayerSignals {
            private_session: 0.2,
            online_coop: 0.0,
            group_size_fit: 0.8,
            low_public_population_dependency: 0.0,
            matchmaking_core: 1.0,
            public_world_dependency: 0.8,
            ..Default::default()
        }
    }

    #[test]
    fn private_coop_outranks_matchmaking_for_friend_fit() {
        assert!(friend_fit(&cooperative_signals()) > friend_fit(&matchmaking_signals()));
    }

    #[test]
    fn ai_adjustment_is_bounded() {
        let base = 0.2;
        let adjusted = blend_ai(
            base,
            Some(AiAdjustment {
                fit: 1.0,
                confidence: 1.0,
            }),
        );

        assert!((adjusted - base).abs() <= 0.15);
    }

    #[test]
    fn invalid_signal_values_are_clamped() {
        let signals = RankingSignals {
            multiplayer: cooperative_signals(),
            quality: 10.0,
            popularity: f64::NAN,
            momentum: -5.0,
            evidence: 2.0,
            freshness: 2.0,
            data_confidence: 2.0,
            personal_fit: 5.0,
            ..Default::default()
        };

        let result = score(FeedSection::RecentRelease, &signals, None);

        assert!((0.0..=1.0).contains(&result.final_score));
    }

    #[test]
    fn default_profile_favors_cooperative_archetype() {
        let common = RankingSignals {
            quality: 0.85,
            popularity: 0.85,
            momentum: 0.5,
            evidence: 0.8,
            data_confidence: 0.9,
            longevity: 0.8,
            maintenance_health: 0.8,
            personal_fit: 0.8,
            ..Default::default()
        };
        let cooperative = RankingSignals {
            multiplayer: cooperative_signals(),
            ..common
        };
        let matchmaking = RankingSignals {
            multiplayer: matchmaking_signals(),
            ..common
        };

        let cooperative_score = score(FeedSection::ClassicLegacy, &cooperative, None);
        let matchmaking_score = score(FeedSection::ClassicLegacy, &matchmaking, None);

        assert!(cooperative_score.final_score > matchmaking_score.final_score);
    }
}

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub type SteamAppId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum FeedSection {
    RecentRelease,
    Upcoming,
    PopularLegacy,
    ClassicLegacy,
}

impl FeedSection {
    pub const ALL: [Self; 4] = [
        Self::RecentRelease,
        Self::Upcoming,
        Self::PopularLegacy,
        Self::ClassicLegacy,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RecentRelease => "recent_release",
            Self::Upcoming => "upcoming",
            Self::PopularLegacy => "popular_legacy",
            Self::ClassicLegacy => "classic_legacy",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "recent_release" => Some(Self::RecentRelease),
            "upcoming" => Some(Self::Upcoming),
            "popular_legacy" => Some(Self::PopularLegacy),
            "classic_legacy" => Some(Self::ClassicLegacy),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct MultiplayerSignals {
    pub private_session: f64,
    pub self_host_or_dedicated: f64,
    pub online_coop: f64,
    pub group_size_fit: f64,
    pub low_public_population_dependency: f64,
    pub drop_in_out: f64,
    pub cross_platform_fit: f64,
    pub matchmaking_core: f64,
    pub public_world_dependency: f64,
    pub group_size_mismatch: f64,
    pub service_shutdown_risk: f64,
    pub external_account_friction: f64,
    pub platform_or_anticheat_restriction: f64,
}

/// Deterministic familiar-group multiplayer fit used by feed eligibility and
/// recommendation scoring. Inputs are normalized at the application boundary.
pub fn friend_fit(signals: &MultiplayerSignals) -> f64 {
    let base = 0.22 * unit_interval(signals.private_session)
        + 0.20 * unit_interval(signals.self_host_or_dedicated)
        + 0.18 * unit_interval(signals.online_coop)
        + 0.15 * unit_interval(signals.group_size_fit)
        + 0.10 * unit_interval(signals.low_public_population_dependency)
        + 0.08 * unit_interval(signals.drop_in_out)
        + 0.07 * unit_interval(signals.cross_platform_fit);

    let penalty = 0.18 * unit_interval(signals.matchmaking_core)
        + 0.15 * unit_interval(signals.public_world_dependency)
        + 0.10 * unit_interval(signals.group_size_mismatch)
        + 0.08 * unit_interval(signals.service_shutdown_risk)
        + 0.06 * unit_interval(signals.external_account_friction)
        + 0.05 * unit_interval(signals.platform_or_anticheat_restriction);

    unit_interval(base - penalty)
}

fn unit_interval(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct RankingSignals {
    pub multiplayer: MultiplayerSignals,
    pub quality: f64,
    pub popularity: f64,
    pub momentum: f64,
    pub evidence: f64,
    pub freshness: f64,
    pub data_confidence: f64,
    pub demo_playability: f64,
    pub release_date_confidence: f64,
    pub release_proximity: f64,
    pub studio_prior: f64,
    pub longevity: f64,
    pub maintenance_health: f64,
    pub risk: f64,
    pub personal_fit: f64,
}

/// User preference snapshot used by API and recommender personalization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UserPreferences {
    pub version: i64,
    pub party_size: u8,
    /// 0 = pure coop preference, 1 = strong competitive preference.
    pub coop_competitive: f64,
    pub session_minutes_min: u32,
    pub session_minutes_max: u32,
    pub budget_currency: String,
    pub budget_max_each_minor: Option<i64>,
    pub platforms: Vec<String>,
    pub self_hosting_willingness: f64,
    pub languages: Vec<String>,
    pub excluded_modes: Vec<String>,
}

/// Versioned rule parameters loaded from the active algorithm configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(default, deny_unknown_fields)]
pub struct RecommendationConfig {
    pub recent_days: u32,
    pub recent_min_friend_fit: f64,
    pub popular_min_ccu: u32,
    pub popular_high_ccu: u32,
    pub popular_min_wilson: f64,
    pub popular_high_ccu_min_wilson: f64,
    pub popular_min_friend_fit: f64,
    pub classic_min_reviews: u32,
    pub classic_min_wilson: f64,
    pub classic_min_friend_fit: f64,
    pub classic_public_min_ccu: u32,
    pub mmr_lambda: f64,
    pub candidate_limit: u32,
    /// Max score boost from community play-intent votes (0 disables the signal).
    pub play_intent_weight: f64,
    /// Vote count at which the play-intent signal reaches half of its weight.
    pub play_intent_saturation: u32,
}

impl Default for RecommendationConfig {
    fn default() -> Self {
        Self {
            recent_days: 180,
            recent_min_friend_fit: 0.45,
            popular_min_ccu: 1_000,
            popular_high_ccu: 100_000,
            popular_min_wilson: 0.58,
            popular_high_ccu_min_wilson: 0.55,
            popular_min_friend_fit: 0.45,
            classic_min_reviews: 3_000,
            classic_min_wilson: 0.82,
            classic_min_friend_fit: 0.55,
            classic_public_min_ccu: 1_000,
            mmr_lambda: 0.75,
            candidate_limit: 10_000,
            play_intent_weight: 0.15,
            play_intent_saturation: 8,
        }
    }
}

impl RecommendationConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=3_650).contains(&self.recent_days) {
            return Err("recent_days must be between 1 and 3650".into());
        }
        for (name, value) in [
            ("recent_min_friend_fit", self.recent_min_friend_fit),
            ("popular_min_wilson", self.popular_min_wilson),
            (
                "popular_high_ccu_min_wilson",
                self.popular_high_ccu_min_wilson,
            ),
            ("popular_min_friend_fit", self.popular_min_friend_fit),
            ("classic_min_wilson", self.classic_min_wilson),
            ("classic_min_friend_fit", self.classic_min_friend_fit),
            ("mmr_lambda", self.mmr_lambda),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(format!("{name} must be between 0 and 1"));
            }
        }
        if self.popular_high_ccu < self.popular_min_ccu {
            return Err("popular_high_ccu must be >= popular_min_ccu".into());
        }
        if self.popular_high_ccu_min_wilson > self.popular_min_wilson {
            return Err("popular_high_ccu_min_wilson must be <= popular_min_wilson".into());
        }
        if !(1..=50_000).contains(&self.candidate_limit) {
            return Err("candidate_limit must be between 1 and 50000".into());
        }
        // A weight or saturation of 0 disables the play-intent signal; this keeps
        // pre-0.2.0 configs (whose JSON lacks these fields) valid and inert.
        if !self.play_intent_weight.is_finite() || !(0.0..=1.0).contains(&self.play_intent_weight) {
            return Err("play_intent_weight must be between 0 and 1".into());
        }
        Ok(())
    }
}

/// Candidate facts used for deterministic preference filtering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CandidateAvailability {
    pub platforms: Vec<String>,
    pub languages: Vec<String>,
    pub typical_session_minutes_min: Option<u32>,
    pub typical_session_minutes_max: Option<u32>,
    pub price_currency: Option<String>,
    pub final_price_minor: Option<i64>,
    pub is_free: Option<bool>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            party_size: 4,
            coop_competitive: 0.15,
            session_minutes_min: 30,
            session_minutes_max: 180,
            budget_currency: "CNY".into(),
            budget_max_each_minor: Some(15_000),
            platforms: vec!["windows".into()],
            self_hosting_willingness: 0.7,
            languages: vec!["schinese".into(), "english".into()],
            excluded_modes: vec!["mmo".into()],
        }
    }
}

impl UserPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.version < 1 {
            return Err("version must be positive".into());
        }
        if !(1..=64).contains(&self.party_size) {
            return Err("party_size must be between 1 and 64".into());
        }
        if !(0.0..=1.0).contains(&self.coop_competitive) {
            return Err("coop_competitive must be between 0 and 1".into());
        }
        if !(0.0..=1.0).contains(&self.self_hosting_willingness) {
            return Err("self_hosting_willingness must be between 0 and 1".into());
        }
        if self.session_minutes_min > self.session_minutes_max {
            return Err("session_minutes_min must be <= session_minutes_max".into());
        }
        if self.session_minutes_max > 24 * 60 {
            return Err("session_minutes_max is too large".into());
        }
        if self.budget_currency.len() != 3
            || !self
                .budget_currency
                .bytes()
                .all(|byte| byte.is_ascii_uppercase())
        {
            return Err("budget_currency must be a 3-letter uppercase code".into());
        }
        if self.budget_max_each_minor.is_some_and(|value| value < 0) {
            return Err("budget_max_each_minor must not be negative".into());
        }
        for (name, values) in [
            ("platforms", &self.platforms),
            ("languages", &self.languages),
            ("excluded_modes", &self.excluded_modes),
        ] {
            if values.len() > 32 {
                return Err(format!("{name} must contain at most 32 values"));
            }
            if values
                .iter()
                .any(|value| value.trim().is_empty() || value.len() > 64)
            {
                return Err(format!("{name} values must contain between 1 and 64 bytes"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    Like,
    NotInterested,
    Played,
    TooCompetitive,
    PartySizeMismatch,
    HostingFriction,
}

impl FeedbackType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Like => "like",
            Self::NotInterested => "not_interested",
            Self::Played => "played",
            Self::TooCompetitive => "too_competitive",
            Self::PartySizeMismatch => "party_size_mismatch",
            Self::HostingFriction => "hosting_friction",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "like" => Some(Self::Like),
            "not_interested" => Some(Self::NotInterested),
            "played" => Some(Self::Played),
            "too_competitive" => Some(Self::TooCompetitive),
            "party_size_mismatch" => Some(Self::PartySizeMismatch),
            "hosting_friction" => Some(Self::HostingFriction),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FeedSection, FeedbackType, RecommendationConfig, UserPreferences};

    #[test]
    fn feed_section_names_are_stable() {
        let names = FeedSection::ALL.map(FeedSection::as_str);
        assert_eq!(
            names,
            [
                "recent_release",
                "upcoming",
                "popular_legacy",
                "classic_legacy"
            ]
        );
        assert_eq!(
            FeedSection::parse("classic_legacy"),
            Some(FeedSection::ClassicLegacy)
        );
    }

    #[test]
    fn default_preferences_validate() {
        assert!(UserPreferences::default().validate().is_ok());
    }

    #[test]
    fn preferences_reject_negative_budget_and_invalid_currency() {
        let mut prefs = UserPreferences {
            budget_max_each_minor: Some(-1),
            ..UserPreferences::default()
        };
        assert!(prefs.validate().is_err());
        prefs.budget_max_each_minor = Some(100);
        prefs.budget_currency = "cny".into();
        assert!(prefs.validate().is_err());
    }

    #[test]
    fn feedback_type_roundtrip() {
        assert_eq!(
            FeedbackType::parse(FeedbackType::Like.as_str()),
            Some(FeedbackType::Like)
        );
    }

    #[test]
    fn recommendation_config_defaults_and_partial_json_validate() {
        let defaults = RecommendationConfig::default();
        assert!(defaults.validate().is_ok());
        let partial: RecommendationConfig = serde_json::from_str(r#"{"recent_days":90}"#).unwrap();
        assert_eq!(partial.recent_days, 90);
        assert_eq!(partial.classic_min_reviews, defaults.classic_min_reviews);
        assert!(partial.validate().is_ok());
    }

    #[test]
    fn recommendation_config_rejects_unsafe_ranges() {
        let config = RecommendationConfig {
            popular_high_ccu: 100,
            popular_min_ccu: 1_000,
            ..RecommendationConfig::default()
        };
        assert!(config.validate().is_err());
    }
}

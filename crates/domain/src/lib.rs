#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub type SteamAppId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    use super::{FeedSection, FeedbackType, UserPreferences};

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
    fn feedback_type_roundtrip() {
        assert_eq!(
            FeedbackType::parse(FeedbackType::Like.as_str()),
            Some(FeedbackType::Like)
        );
    }
}

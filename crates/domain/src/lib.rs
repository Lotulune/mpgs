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

#[cfg(test)]
mod tests {
    use super::FeedSection;

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
    }
}

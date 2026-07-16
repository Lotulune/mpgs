//! M1 golden multiplayer label set (human-curated, fixture-backed).

use serde::{Deserialize, Serialize};

use mpgs_domain::SteamAppId;

use crate::error::SourceError;

pub const GOLDEN_SET_VERSION: &str = "golden-0.1.0";
pub const GOLDEN_SET_JSON: &str = include_str!("../fixtures/golden_set_v0.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStateLabel {
    Released,
    Upcoming,
    ComingSoon,
    Retired,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DominantModeLabel {
    Coop,
    Competitive,
    Mixed,
    Mmo,
    SinglePrimary,
    Unknown,
}

/// Human multiplayer feature labels for ranking / data quality tests.
///
/// Values are ternary where needed: `true` / `false` / omitted as unknown via Option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoldenMultiplayerLabels {
    pub private_session: Option<bool>,
    pub self_host_or_dedicated: Option<bool>,
    pub online_coop: Option<bool>,
    pub matchmaking_core: Option<bool>,
    pub public_world_dependency: Option<bool>,
    pub drop_in_out: Option<bool>,
    pub crossplay: Option<bool>,
    pub service_shutdown_risk: Option<bool>,
    pub dominant_mode: DominantModeLabel,
    pub recommended_min_players: Option<u8>,
    pub recommended_max_players: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoldenGame {
    pub app_id: SteamAppId,
    pub name: String,
    pub release_state: ReleaseStateLabel,
    pub review_bucket: String,
    pub multiplayer: GoldenMultiplayerLabels,
    pub evidence_notes: String,
    pub case_tags: Vec<String>,
    pub dual_reviewed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoldenSet {
    pub version: String,
    pub description: String,
    pub games: Vec<GoldenGame>,
}

impl GoldenSet {
    pub fn load_embedded() -> Result<Self, SourceError> {
        let set: Self = serde_json::from_str(GOLDEN_SET_JSON).map_err(SourceError::json_parse)?;
        set.validate()?;
        Ok(set)
    }

    pub fn validate(&self) -> Result<(), SourceError> {
        if self.version != GOLDEN_SET_VERSION {
            return Err(SourceError::invalid_structure(format!(
                "golden set version {} does not match parser version {GOLDEN_SET_VERSION}",
                self.version
            )));
        }
        if self.games.len() < 50 {
            return Err(SourceError::invalid_structure(format!(
                "golden set requires at least 50 games, found {}",
                self.games.len()
            )));
        }

        let mut seen = std::collections::BTreeSet::new();
        for game in &self.games {
            if game.app_id == 0 {
                return Err(SourceError::invalid_structure(
                    "golden game app_id must be non-zero",
                ));
            }
            if !seen.insert(game.app_id) {
                return Err(SourceError::invalid_structure(format!(
                    "duplicate golden app_id {}",
                    game.app_id
                )));
            }
            if game.name.trim().is_empty() {
                return Err(SourceError::invalid_structure(format!(
                    "golden app {} missing name",
                    game.app_id
                )));
            }
            if game.evidence_notes.trim().is_empty() {
                return Err(SourceError::invalid_structure(format!(
                    "golden app {} missing evidence_notes",
                    game.app_id
                )));
            }
            if let (Some(min), Some(max)) = (
                game.multiplayer.recommended_min_players,
                game.multiplayer.recommended_max_players,
            ) && min > max
            {
                return Err(SourceError::invalid_structure(format!(
                    "golden app {} has min_players > max_players",
                    game.app_id
                )));
            }
        }

        let dual = self.games.iter().filter(|g| g.dual_reviewed).count();
        if dual < 10 {
            return Err(SourceError::invalid_structure(format!(
                "expected at least 10 dual-reviewed high-impact games, found {dual}"
            )));
        }

        Ok(())
    }

    pub fn by_tag<'a>(&'a self, tag: &str) -> Vec<&'a GoldenGame> {
        self.games
            .iter()
            .filter(|g| g.case_tags.iter().any(|t| t == tag))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_golden_set_meets_m1_exit_bar() {
        let set = GoldenSet::load_embedded().expect("golden set must parse");
        assert!(set.games.len() >= 50);
        assert!(!set.by_tag("self_host").is_empty());
        assert!(!set.by_tag("matchmaking_core").is_empty());
        assert!(!set.by_tag("mmo").is_empty());
        assert!(!set.by_tag("coop").is_empty());
        assert!(!set.by_tag("shutdown_risk").is_empty());
        assert!(!set.by_tag("party_size").is_empty());

        for game in &set.games {
            // Basic multiplayer feature surface required by M1 exit criteria.
            assert!(
                game.multiplayer.private_session.is_some()
                    || game.multiplayer.self_host_or_dedicated.is_some()
                    || game.multiplayer.online_coop.is_some()
                    || game.multiplayer.matchmaking_core.is_some(),
                "app {} lacks basic multiplayer labels",
                game.app_id
            );
            assert!(
                !matches!(game.release_state, ReleaseStateLabel::Unknown),
                "app {} release_state unknown",
                game.app_id
            );
            assert!(
                !game.review_bucket.is_empty(),
                "app {} missing review bucket",
                game.app_id
            );
        }
    }

    #[test]
    fn embedded_data_version_must_match_parser_version() {
        let mut set = GoldenSet::load_embedded().unwrap();
        set.version = "golden-stale".into();
        assert!(set.validate().is_err());
    }
}

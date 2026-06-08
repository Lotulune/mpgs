use serde::{Deserialize, Serialize};

pub const DISCOVERY_CURSOR_CONFIG_KEY: &str = "steam_discovery_last_appid";

pub use mpgs_core::steam_mapping::{
    build_discovered_game_card, clamp_discovery_page_size, clamp_discovery_pages,
    clamp_discovery_target_added_games, next_discovery_cursor, parse_saved_cursor,
    store_search_reached_page_budget, store_search_start_for_page,
    DISCOVERY_TASK_TARGET_ADDED_GAMES_DEFAULT, DISCOVERY_TASK_TARGET_ADDED_GAMES_MAX,
    STORE_SEARCH_DISCOVERY_MAX_PAGES_PER_RUN,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamDiscoveryReport {
    pub scanned_apps: usize,
    pub skipped_existing: usize,
    pub skipped_non_multiplayer: usize,
    pub added_games: usize,
    pub added_new_games: usize,
    pub added_classic_games: usize,
    pub failed_games: usize,
    pub last_appid: Option<u32>,
    pub have_more_results: bool,
    pub message: String,
}

impl SteamDiscoveryReport {
    pub fn new() -> Self {
        Self {
            scanned_apps: 0,
            skipped_existing: 0,
            skipped_non_multiplayer: 0,
            added_games: 0,
            added_new_games: 0,
            added_classic_games: 0,
            failed_games: 0,
            last_appid: None,
            have_more_results: false,
            message: String::new(),
        }
    }

    pub fn finish_message(&mut self) {
        let tail = if self.have_more_results {
            "本轮仍有更多最近发售候选，可增加扫描页数扩大范围。"
        } else {
            "Steam 最近发售候选已扫描到末尾。"
        };
        self.message = format!(
            "已从 Steam 最近发售多人候选扫描 {} 个应用，新增 {} 个多人游戏（新游区 {}、老游区 {}）；跳过已存在 {} 个、非多人 {} 个、失败 {} 个。{}",
            self.scanned_apps,
            self.added_games,
            self.added_new_games,
            self.added_classic_games,
            self.skipped_existing,
            self.skipped_non_multiplayer,
            self.failed_games,
            tail
        );
    }
}

#[allow(dead_code)]
fn _demo_status_exhaustiveness(
    status: crate::recommendation::DemoStatus,
) -> crate::recommendation::DemoStatus {
    status
}

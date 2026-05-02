#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LanguageCode {
    ZhCn,
    ZhTw,
    En,
    Ja,
    Other(String),
}

pub type CanonicalTag = String;

#[derive(Debug, Clone, Default)]
pub struct CanonicalGameSignals {
    pub language_codes: Vec<LanguageCode>,
    pub tags: Vec<CanonicalTag>,
    pub multiplayer_modes: MultiplayerModes,
    pub review_stats: ReviewStats,
    pub review_topics: ReviewTopics,
    pub activity: ActivityStats,
    pub release: ReleaseInfo,
    pub demo: DemoInfo,
}

#[derive(Debug, Clone, Default)]
pub struct MultiplayerModes {
    pub has_any: bool,
    pub online_coop: bool,
    pub local_coop: bool,
    pub online_pvp: bool,
    pub lan: bool,
    pub cross_platform: bool,
    pub remote_play_together: bool,
    pub supports_2_players: bool,
    pub supports_4_players: bool,
    pub flexible_player_count: bool,
    pub signal_count: u32,
    pub raw_mode_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ReviewStats {
    pub positive_review_pct: Option<f64>,
    pub total_reviews: u32,
    pub analyzed_review_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct TopicCounter {
    pub positive: u32,
    pub negative: u32,
}

impl TopicCounter {
    pub fn delta(&self) -> f64 {
        self.positive as f64 - self.negative as f64
    }

    pub fn total(&self) -> u32 {
        self.positive + self.negative
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReviewTopics {
    pub multiplayer: TopicCounter,
    pub server: TopicCounter,
    pub disconnect: TopicCounter,
    pub invite: TopicCounter,
    pub content_depth: TopicCounter,
    pub repetition: TopicCounter,
    pub bug: TopicCounter,
    pub balance: TopicCounter,
    pub monetization: TopicCounter,
    pub abandonment: TopicCounter,
    pub localization: TopicCounter,
    pub tutorial: TopicCounter,
    pub controller: TopicCounter,
    pub replayability: TopicCounter,
    pub progression: TopicCounter,
    pub mode_variety: TopicCounter,
    pub casual: TopicCounter,
    pub complexity: TopicCounter,
}

#[derive(Debug, Clone, Default)]
pub struct ActivityStats {
    pub current_players: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct ReleaseInfo {
    pub release_date: Option<String>,
    pub release_age_days: Option<i64>,
    pub recent_release: bool,
    pub early_access_hint: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DemoInfo {
    pub has_demo: bool,
    pub is_demo_only: bool,
}

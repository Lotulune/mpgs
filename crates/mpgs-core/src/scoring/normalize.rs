use crate::models::{GameCard, ReviewSnippet};
use crate::recommendation::DemoStatus;
use crate::scoring::signals::{
    ActivityStats, CanonicalGameSignals, DemoInfo, LanguageCode, MultiplayerModes, ReleaseInfo,
    ReviewStats, ReviewTopics, TopicCounter,
};
use time::{format_description::FormatItem, macros::format_description, Date};

const ISO_DATE: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");

pub fn normalize_game_signals(game: &GameCard) -> CanonicalGameSignals {
    CanonicalGameSignals {
        language_codes: normalize_languages(&game.supported_languages),
        tags: normalize_tags(&game.tags),
        multiplayer_modes: normalize_multiplayer_modes(&game.multiplayer_modes),
        review_stats: ReviewStats {
            positive_review_pct: game.positive_review_pct,
            total_reviews: game.total_reviews.unwrap_or(0),
            analyzed_review_count: game.review_snippets.len(),
        },
        review_topics: extract_review_topics(&game.review_snippets),
        activity: ActivityStats {
            current_players: game.current_players,
        },
        release: normalize_release(game),
        demo: DemoInfo {
            has_demo: matches!(
                game.demo_status,
                DemoStatus::DemoOnly | DemoStatus::ReleasedWithDemo
            ),
            is_demo_only: matches!(game.demo_status, DemoStatus::DemoOnly),
        },
    }
}

fn normalize_languages(languages: &[String]) -> Vec<LanguageCode> {
    let mut codes = Vec::new();
    for language in languages {
        let normalized = language.trim().to_ascii_lowercase();
        let code = if normalized.contains("schinese")
            || normalized.contains("simplified chinese")
            || normalized.contains("简体中文")
        {
            LanguageCode::ZhCn
        } else if normalized.contains("tchinese")
            || normalized.contains("traditional chinese")
            || normalized.contains("繁體中文")
            || normalized.contains("繁体中文")
        {
            LanguageCode::ZhTw
        } else if normalized == "en" || normalized.contains("english") {
            LanguageCode::En
        } else if normalized.contains("japanese") || normalized.contains("日本語") {
            LanguageCode::Ja
        } else {
            LanguageCode::Other(normalized)
        };

        if !codes.contains(&code) {
            codes.push(code);
        }
    }
    codes
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        for mapped in map_tag(tag) {
            if !normalized.contains(&mapped) {
                normalized.push(mapped);
            }
        }
    }
    normalized
}

fn map_tag(tag: &str) -> Vec<String> {
    let text = tag.trim().to_ascii_lowercase();
    let mut result = Vec::new();

    let mappings: &[(&[&str], &str)] = &[
        (&["single-player", "single player", "单人"], "SINGLE_PLAYER"),
        (&["co-op", "cooperative", "合作"], "COOP"),
        (&["online co-op", "online coop", "在线合作"], "ONLINE_COOP"),
        (&["local co-op", "local coop", "本地合作"], "LOCAL_COOP"),
        (&["pvp", "玩家对战", "对战"], "PVP"),
        (&["party", "派对"], "PARTY"),
        (&["casual", "休闲"], "CASUAL"),
        (&["simulation", "模拟"], "SIMULATION"),
        (&["roguelike", "rogue", "类 rogue"], "ROGUELIKE"),
        (&["puzzle", "解谜"], "PUZZLE"),
        (&["controller", "手柄"], "CONTROLLER"),
        (&["cross-platform", "跨平台"], "CROSS_PLATFORM"),
        (&["remote play together", "远程同乐"], "REMOTE_PLAY"),
        (&["early access", "抢先体验"], "EARLY_ACCESS"),
    ];

    for (needles, mapped) in mappings {
        if needles.iter().any(|needle| text.contains(needle)) {
            result.push(mapped.to_string());
        }
    }

    if result.is_empty() {
        result.push(text.to_uppercase());
    }

    result
}

fn normalize_multiplayer_modes(modes: &[String]) -> MultiplayerModes {
    let mut normalized = MultiplayerModes {
        has_any: !modes.is_empty(),
        raw_mode_count: modes.len(),
        ..Default::default()
    };

    for mode in modes {
        let text = mode.trim().to_ascii_lowercase();
        if text.contains("online co-op")
            || text.contains("online coop")
            || text.contains("在线合作")
        {
            normalized.online_coop = true;
        }
        if text.contains("local co-op")
            || text.contains("local coop")
            || text.contains("shared/split screen")
            || text.contains("split screen")
            || text.contains("本地合作")
        {
            normalized.local_coop = true;
        }
        if text.contains("pvp") || text.contains("玩家对战") {
            normalized.online_pvp = true;
        }
        if text.contains("lan") {
            normalized.lan = true;
        }
        if text.contains("cross-platform") || text.contains("跨平台") {
            normalized.cross_platform = true;
        }
        if text.contains("remote play together") || text.contains("远程同乐") {
            normalized.remote_play_together = true;
        }
    }

    normalized.supports_2_players = normalized.has_any;
    normalized.supports_4_players = normalized.online_coop
        || normalized.local_coop
        || normalized.online_pvp
        || normalized.raw_mode_count >= 2;
    normalized.flexible_player_count = normalized.raw_mode_count >= 2;
    normalized.signal_count = u32::from(normalized.online_coop)
        + u32::from(normalized.local_coop)
        + u32::from(normalized.online_pvp)
        + u32::from(normalized.lan)
        + u32::from(normalized.cross_platform)
        + u32::from(normalized.remote_play_together);

    normalized
}

fn normalize_release(game: &GameCard) -> ReleaseInfo {
    let release_age_days = days_since_release(game.release_date.as_deref());
    let text = game
        .short_description
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    ReleaseInfo {
        release_date: game.release_date.clone(),
        release_age_days,
        recent_release: release_age_days.is_some_and(|days| days <= 90),
        early_access_hint: text.contains("early access") || text.contains("抢先体验"),
    }
}

fn extract_review_topics(snippets: &[ReviewSnippet]) -> ReviewTopics {
    let mut topics = ReviewTopics::default();
    for snippet in snippets {
        let text = snippet.review.trim();
        if text.is_empty() {
            continue;
        }
        let normalized = text.to_ascii_lowercase();

        if contains_any(
            &normalized,
            &[
                "multiplayer",
                "co-op",
                "cooperative",
                "friends",
                "开黑",
                "联机",
                "合作",
            ],
        ) {
            bump(&mut topics.multiplayer, snippet.voted_up);
        }
        if contains_any(
            &normalized,
            &["server", "matchmaking", "服务器", "匹配", "排队"],
        ) {
            bump(&mut topics.server, snippet.voted_up);
        }
        if contains_any(&normalized, &["disconnect", "drop", "掉线", "断线"]) {
            bump(&mut topics.disconnect, snippet.voted_up);
        }
        if contains_any(&normalized, &["invite", "邀请", "组队"]) {
            bump(&mut topics.invite, snippet.voted_up);
        }
        if contains_any(
            &normalized,
            &[
                "content", "mode", "map", "level", "内容", "模式", "地图", "关卡",
            ],
        ) {
            bump(&mut topics.content_depth, snippet.voted_up);
        }
        if contains_any(
            &normalized,
            &["repeat", "repetitive", "boring", "重复", "无聊", "枯燥"],
        ) {
            bump(&mut topics.repetition, snippet.voted_up);
        }
        if contains_any(
            &normalized,
            &["bug", "crash", "stutter", "卡顿", "闪退", "崩溃", "buggy"],
        ) {
            bump(&mut topics.bug, snippet.voted_up);
        }
        if contains_any(&normalized, &["balance", "unbalanced", "失衡", "不平衡"]) {
            bump(&mut topics.balance, snippet.voted_up);
        }
        if contains_any(
            &normalized,
            &["dlc", "microtransaction", "pay to win", "氪金", "付费"],
        ) {
            bump(&mut topics.monetization, snippet.voted_up);
        }
        if contains_any(&normalized, &["abandoned", "no updates", "停更", "跑路"]) {
            bump(&mut topics.abandonment, snippet.voted_up);
        }
        if contains_any(
            &normalized,
            &["translation", "localization", "翻译", "中文", "机翻"],
        ) {
            bump(&mut topics.localization, snippet.voted_up);
        }
        if contains_any(&normalized, &["tutorial", "teach", "教程", "引导", "上手"]) {
            bump(&mut topics.tutorial, snippet.voted_up);
        }
        if contains_any(&normalized, &["controller", "steam deck", "手柄", "deck"]) {
            bump(&mut topics.controller, snippet.voted_up);
        }
        if contains_any(
            &normalized,
            &["replay", "build", "random", "重玩", "构筑", "随机"],
        ) {
            bump(&mut topics.replayability, snippet.voted_up);
        }
        if contains_any(
            &normalized,
            &[
                "progression",
                "unlock",
                "skill tree",
                "成长",
                "解锁",
                "升级",
                "技能树",
            ],
        ) {
            bump(&mut topics.progression, snippet.voted_up);
        }
        if contains_any(
            &normalized,
            &["modes", "maps", "职业", "build variety", "地图多", "模式多"],
        ) {
            bump(&mut topics.mode_variety, snippet.voted_up);
        }
        if contains_any(&normalized, &["casual", "relax", "轻松", "休闲", "容易"]) {
            bump(&mut topics.casual, snippet.voted_up);
        }
        if contains_any(
            &normalized,
            &["hard", "confusing", "complex", "难", "复杂", "机制不清"],
        ) {
            bump(&mut topics.complexity, snippet.voted_up);
        }
    }
    topics
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn bump(counter: &mut TopicCounter, voted_up: bool) {
    if voted_up {
        counter.positive += 1;
    } else {
        counter.negative += 1;
    }
}

fn days_since_release(release_date: Option<&str>) -> Option<i64> {
    let release = Date::parse(release_date?, ISO_DATE).ok()?;
    let today = Date::parse(&crate::recommendation::today_iso_utc(), ISO_DATE).ok()?;
    Some((today - release).whole_days())
}

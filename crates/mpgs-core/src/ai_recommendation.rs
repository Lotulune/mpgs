use crate::models::{
    AiRecommendationRequest, AiRecommendationResponse, AiRecommendedGame, AnalysisSource, GameCard,
    StoreReleaseState,
};
use std::cmp::Ordering;

const DEFAULT_LIMIT: usize = 5;
const MAX_LIMIT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
enum IntentKind {
    LocalCoop,
    OnlineCoop,
    Casual,
    Cute,
    Survival,
    Puzzle,
    Shooter,
    Party,
    Roguelike,
    Pixel,
    Free,
    Cheap,
    HighPlayerCount,
    MinPlayers(u8),
    ExcludePixel,
    ExcludeHorror,
    ExcludeRoguelike,
}

#[derive(Debug, Clone)]
struct TraitMatch {
    label: &'static str,
    matched: bool,
}

pub fn recommend_games_locally(
    games: &[GameCard],
    request: &AiRecommendationRequest,
) -> AiRecommendationResponse {
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let query = merged_query(request);
    let intents = parse_intents(&query);
    let positive_intents = intents
        .iter()
        .filter(|intent| !is_exclusion_intent(intent))
        .count();
    let mut ranked = games
        .iter()
        .filter(|game| game.release_state == StoreReleaseState::Released)
        .filter_map(|game| score_game(game, &intents, positive_intents))
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .match_score
            .partial_cmp(&left.match_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                right
                    .game
                    .recommendation_score
                    .partial_cmp(&left.game.recommendation_score)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left.game.name.cmp(&right.game.name))
    });

    ranked.truncate(limit);
    let exact_match_count = ranked.iter().filter(|item| item.exact_match).count();
    let reply = build_reply(ranked.len(), exact_match_count);
    let follow_up_question = if ranked.is_empty() {
        Some("你愿意放宽玩法、人数或联机方式中的哪一项？".to_string())
    } else if exact_match_count < ranked.len() {
        Some("你更愿意优先保留玩法、人数，还是联机方式？".to_string())
    } else {
        None
    };

    AiRecommendationResponse {
        reply,
        follow_up_question,
        exact_match_count,
        source: AnalysisSource::Rule,
        llm_used: false,
        diagnostic: Some(
            "未配置 LLM Key 或未完成增强时，使用本地规则匹配和库内质量指标排序。".to_string(),
        ),
        items: ranked,
    }
}

fn merged_query(request: &AiRecommendationRequest) -> String {
    let mut parts = request
        .context_messages
        .iter()
        .rev()
        .take(4)
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>();
    parts.reverse();
    parts.push(request.prompt.as_str());
    parts.join(" ")
}

fn parse_intents(query: &str) -> Vec<IntentKind> {
    let normalized = normalize(query);
    let mut intents = Vec::new();

    if contains_any(
        &normalized,
        &[
            "本地合作",
            "本地联机",
            "本地多人",
            "同屏",
            "分屏",
            "沙发合作",
            "local coop",
            "local co-op",
            "couch coop",
            "split screen",
        ],
    ) {
        intents.push(IntentKind::LocalCoop);
    }
    if contains_any(
        &normalized,
        &[
            "在线合作",
            "线上合作",
            "在线联机",
            "online coop",
            "online co-op",
        ],
    ) {
        intents.push(IntentKind::OnlineCoop);
    }
    if contains_any(
        &normalized,
        &["轻松", "休闲", "放松", "简单", "casual", "relaxing", "cozy"],
    ) {
        intents.push(IntentKind::Casual);
    }
    if contains_any(&normalized, &["可爱", "萌", "治愈", "cute", "wholesome"]) {
        intents.push(IntentKind::Cute);
    }
    if contains_any(&normalized, &["生存", "survival"]) {
        intents.push(IntentKind::Survival);
    }
    if contains_any(&normalized, &["解谜", "谜题", "puzzle"]) {
        intents.push(IntentKind::Puzzle);
    }
    if contains_any(&normalized, &["射击", "枪", "shooter", "fps"]) {
        intents.push(IntentKind::Shooter);
    }
    if contains_any(&normalized, &["派对", "party"]) {
        intents.push(IntentKind::Party);
    }
    if contains_any(
        &normalized,
        &["肉鸽", "roguelike", "roguelite", "rougelike", "rougelite"],
    ) {
        intents.push(IntentKind::Roguelike);
    }
    if contains_any(&normalized, &["像素", "pixel"]) {
        intents.push(IntentKind::Pixel);
    }
    if contains_any(&normalized, &["免费", "free to play", "free"]) {
        intents.push(IntentKind::Free);
    }
    if contains_any(&normalized, &["便宜", "低价", "打折", "cheap", "discount"]) {
        intents.push(IntentKind::Cheap);
    }
    if contains_any(&normalized, &["人多", "活跃", "在线人数", "high player"]) {
        intents.push(IntentKind::HighPlayerCount);
    }
    if contains_any(
        &normalized,
        &["不要像素", "排除像素", "非像素", "不想要像素", "no pixel"],
    ) {
        intents.retain(|intent| !matches!(intent, IntentKind::Pixel));
        intents.push(IntentKind::ExcludePixel);
    }
    if contains_any(
        &normalized,
        &["不要恐怖", "不恐怖", "排除恐怖", "no horror"],
    ) {
        intents.push(IntentKind::ExcludeHorror);
    }
    if contains_any(
        &normalized,
        &[
            "不要肉鸽",
            "排除肉鸽",
            "非肉鸽",
            "no roguelike",
            "no roguelite",
        ],
    ) {
        intents.retain(|intent| !matches!(intent, IntentKind::Roguelike));
        intents.push(IntentKind::ExcludeRoguelike);
    }
    if contains_any(
        &normalized,
        &["6人", "6 人", "六人", "六 人", "6+", "6 人以上", "6人以上"],
    ) {
        intents.push(IntentKind::MinPlayers(6));
    } else if contains_any(
        &normalized,
        &["4人", "4 人", "四人", "四 人", "4+", "4 人以上", "4人以上"],
    ) {
        intents.push(IntentKind::MinPlayers(4));
    }

    dedupe_intents(intents)
}

fn score_game(
    game: &GameCard,
    intents: &[IntentKind],
    positive_intents: usize,
) -> Option<AiRecommendedGame> {
    let corpus = normalize(&format!(
        "{} {} {} {} {}",
        game.name,
        game.short_description.as_deref().unwrap_or_default(),
        game.tags.join(" "),
        game.multiplayer_modes.join(" "),
        game.ai_summary
    ));

    if intents
        .iter()
        .any(|intent| exclusion_blocks_game(intent, &corpus))
    {
        return None;
    }

    let mut trait_matches = Vec::new();
    for intent in intents.iter().filter(|intent| !is_exclusion_intent(intent)) {
        trait_matches.push(evaluate_intent(intent, game, &corpus));
    }

    let matched_count = trait_matches.iter().filter(|item| item.matched).count();
    let missing_count = trait_matches.len().saturating_sub(matched_count);
    let semantic_score = if positive_intents == 0 {
        55.0
    } else {
        (matched_count as f64 / positive_intents as f64) * 55.0
    };
    let quality_score = quality_score(game) * 0.30;
    let confidence_score = confidence_score(game) * 0.15;
    let mut score = semantic_score + quality_score + confidence_score;

    if missing_count > 0 {
        score -= (missing_count as f64 * 4.0).min(16.0);
    }
    if game.user_state.favorite || game.user_state.wishlist {
        score += 3.0;
    }

    let matched_traits = trait_matches
        .iter()
        .filter(|item| item.matched)
        .map(|item| item.label.to_string())
        .collect::<Vec<_>>();
    let missing_traits = trait_matches
        .iter()
        .filter(|item| !item.matched)
        .map(|item| item.label.to_string())
        .collect::<Vec<_>>();
    let exact_match = !matched_traits.is_empty() && missing_traits.is_empty();

    if positive_intents > 0 && matched_traits.is_empty() {
        return None;
    }

    Some(AiRecommendedGame {
        game: game.clone(),
        match_score: score.clamp(0.0, 100.0),
        reason: build_reason(game, &matched_traits, &missing_traits),
        matched_traits,
        missing_traits,
        caveats: build_caveats(game),
        exact_match,
    })
}

fn evaluate_intent(intent: &IntentKind, game: &GameCard, corpus: &str) -> TraitMatch {
    match intent {
        IntentKind::LocalCoop => TraitMatch {
            label: "本地合作",
            matched: contains_any(
                corpus,
                &["local co-op", "local coop", "split screen", "shared/split"],
            ),
        },
        IntentKind::OnlineCoop => TraitMatch {
            label: "在线合作",
            matched: contains_any(corpus, &["online co-op", "online coop", "online"]),
        },
        IntentKind::Casual => TraitMatch {
            label: "轻松休闲",
            matched: contains_any(
                corpus,
                &[
                    "casual", "cozy", "relaxing", "cute", "party", "轻松", "休闲",
                ],
            ),
        },
        IntentKind::Cute => TraitMatch {
            label: "可爱画风",
            matched: contains_any(corpus, &["cute", "wholesome", "cozy", "可爱", "治愈"]),
        },
        IntentKind::Survival => TraitMatch {
            label: "生存玩法",
            matched: contains_any(corpus, &["survival", "生存"]),
        },
        IntentKind::Puzzle => TraitMatch {
            label: "解谜玩法",
            matched: contains_any(corpus, &["puzzle", "解谜", "谜题"]),
        },
        IntentKind::Shooter => TraitMatch {
            label: "射击玩法",
            matched: contains_any(corpus, &["shooter", "fps", "射击"]),
        },
        IntentKind::Party => TraitMatch {
            label: "派对氛围",
            matched: contains_any(corpus, &["party", "派对"]),
        },
        IntentKind::Roguelike => TraitMatch {
            label: "肉鸽玩法",
            matched: contains_any(corpus, &["roguelike", "roguelite", "肉鸽"]),
        },
        IntentKind::Pixel => TraitMatch {
            label: "像素风格",
            matched: contains_any(corpus, &["pixel", "像素"]),
        },
        IntentKind::Free => TraitMatch {
            label: "免费",
            matched: game.is_free || contains_any(corpus, &["free to play", "免费"]),
        },
        IntentKind::Cheap => TraitMatch {
            label: "价格友好",
            matched: game.is_free
                || game.discount_percent.unwrap_or(0) >= 20
                || game
                    .price_text
                    .as_deref()
                    .map(|text| contains_any(&normalize(text), &["free", "$0", "¥0"]))
                    .unwrap_or(false),
        },
        IntentKind::HighPlayerCount => TraitMatch {
            label: "活跃人数",
            matched: game.current_players.unwrap_or(0) >= 500,
        },
        IntentKind::MinPlayers(min_players) => TraitMatch {
            label: "多人规模",
            matched: supports_min_players(corpus, *min_players),
        },
        IntentKind::ExcludePixel | IntentKind::ExcludeHorror | IntentKind::ExcludeRoguelike => {
            TraitMatch {
                label: "排除项",
                matched: true,
            }
        }
    }
}

fn exclusion_blocks_game(intent: &IntentKind, corpus: &str) -> bool {
    match intent {
        IntentKind::ExcludePixel => contains_any(corpus, &["pixel", "像素"]),
        IntentKind::ExcludeHorror => contains_any(corpus, &["horror", "恐怖"]),
        IntentKind::ExcludeRoguelike => contains_any(corpus, &["roguelike", "roguelite", "肉鸽"]),
        _ => false,
    }
}

fn is_exclusion_intent(intent: &IntentKind) -> bool {
    matches!(
        intent,
        IntentKind::ExcludePixel | IntentKind::ExcludeHorror | IntentKind::ExcludeRoguelike
    )
}

fn supports_min_players(corpus: &str, min_players: u8) -> bool {
    let threshold = min_players.to_string();
    contains_any(
        corpus,
        &[
            &format!("{threshold} players"),
            &format!("up to {threshold}"),
            &format!("{threshold}+"),
            &format!("{threshold} 人"),
            &format!("{threshold}人"),
        ],
    )
}

fn quality_score(game: &GameCard) -> f64 {
    let base_score = game
        .ai_score
        .unwrap_or(game.recommendation_score)
        .clamp(0.0, 100.0);
    let review_score = game
        .positive_review_pct
        .unwrap_or(base_score)
        .clamp(0.0, 100.0);
    let review_confidence = review_confidence(game.total_reviews.unwrap_or(0));

    base_score * 0.45 + review_score * 0.40 + review_confidence * 0.15
}

fn confidence_score(game: &GameCard) -> f64 {
    let review_confidence = review_confidence(game.total_reviews.unwrap_or(0));
    let activity_score = activity_score(game.current_players.unwrap_or(0));

    review_confidence * 0.55 + activity_score * 0.45
}

fn review_confidence(total_reviews: u32) -> f64 {
    if total_reviews == 0 {
        return 35.0;
    }

    (total_reviews as f64)
        .log10()
        .mul_add(20.0, 20.0)
        .clamp(35.0, 100.0)
}

fn activity_score(current_players: u32) -> f64 {
    if current_players == 0 {
        return 35.0;
    }

    (current_players as f64)
        .log10()
        .mul_add(18.0, 35.0)
        .clamp(35.0, 100.0)
}

fn build_reason(game: &GameCard, matched_traits: &[String], missing_traits: &[String]) -> String {
    let quality_summary = build_quality_summary(game);
    if !matched_traits.is_empty() && missing_traits.is_empty() {
        return format!("匹配{}，{}。", matched_traits.join("、"), quality_summary);
    }
    if !matched_traits.is_empty() {
        return format!(
            "先按{}召回，{}，但{}仍不完全满足。",
            matched_traits.join("、"),
            quality_summary,
            missing_traits.join("、")
        );
    }

    format!(
        "《{}》{}，可作为放宽条件后的备选。",
        game.name, quality_summary
    )
}

fn build_quality_summary(game: &GameCard) -> String {
    let review_text = match (game.positive_review_pct, game.total_reviews) {
        (Some(pct), Some(total)) if total > 0 => {
            format!("好评率 {}%，{} 条评测提供质量参考", pct.round(), total)
        }
        (Some(pct), _) => format!("好评率 {}% 可作质量参考", pct.round()),
        (_, Some(total)) if total > 0 => format!("{} 条评测提供质量参考", total),
        _ => "库内推荐分提供基础质量参考".to_string(),
    };
    let player_text = match game.current_players {
        Some(players) if players >= 500 => format!("当前在线 {players}，活跃度较稳"),
        Some(players) if players > 0 => format!("当前在线 {players}，适合确认组队热度"),
        _ => "在线人数样本不足，适合打开详情再确认".to_string(),
    };

    format!("{review_text}；{player_text}")
}

fn build_caveats(game: &GameCard) -> Vec<String> {
    let mut caveats = Vec::new();
    if game.current_players.unwrap_or(0) < 100 {
        caveats.push("在线人数偏低".to_string());
    }
    if game.positive_review_pct.unwrap_or(100.0) < 75.0 {
        caveats.push("口碑存在分歧".to_string());
    }
    if game.section == "classic_hidden" {
        caveats.push("来自隐藏候选库，需要更仔细确认口味".to_string());
    }
    if caveats.is_empty() {
        caveats.push("仍建议打开详情核对近期评测".to_string());
    }
    caveats
}

fn build_reply(result_count: usize, exact_match_count: usize) -> String {
    if result_count == 0 {
        return "当前已入库且已发售的游戏里没有找到可用候选。".to_string();
    }
    if exact_match_count == result_count {
        return format!("我在已入库且已发售的游戏里找到了 {result_count} 个匹配候选。");
    }
    if exact_match_count == 0 {
        return "当前库里没有完全匹配的游戏，我先按最关键条件给出近似推荐。".to_string();
    }
    format!("我找到了 {exact_match_count} 个完全匹配候选，也补充了几个近似选择供比较。")
}

fn normalize(value: &str) -> String {
    value.to_ascii_lowercase()
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles
        .iter()
        .any(|needle| value.contains(&needle.to_ascii_lowercase()))
}

fn dedupe_intents(intents: Vec<IntentKind>) -> Vec<IntentKind> {
    let mut deduped = Vec::new();
    for intent in intents {
        if !deduped.contains(&intent) {
            deduped.push(intent);
        }
    }
    deduped
}

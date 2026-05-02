use crate::scoring::signals::CanonicalGameSignals;

pub fn score_multiplayer_fit(signals: &CanonicalGameSignals) -> f64 {
    let modes = &signals.multiplayer_modes;
    if !modes.has_any {
        return 20.0;
    }

    let mut mode_score: f64 = 0.0;
    if modes.online_coop {
        mode_score += 25.0;
    }
    if modes.local_coop {
        mode_score += 14.0;
    }
    if modes.online_pvp {
        mode_score += 12.0;
    }
    if modes.lan {
        mode_score += 8.0;
    }
    if modes.cross_platform {
        mode_score += 6.0;
    }
    if modes.remote_play_together {
        mode_score += 5.0;
    }
    mode_score = mode_score.min(55.0);

    let mut group_score = 0.0;
    if modes.supports_2_players {
        group_score += 15.0;
    }
    if modes.supports_4_players {
        group_score += 8.0;
    }
    if modes.flexible_player_count {
        group_score += 5.0;
    }

    let review_sentiment =
        (12.0 + signals.review_topics.multiplayer.delta() * 4.0).clamp(0.0, 20.0);
    let mut friction_penalty = 0.0;
    if signals.review_topics.server.negative > 0 {
        friction_penalty += 12.0;
    }
    if signals.review_topics.disconnect.negative > 0 {
        friction_penalty += 8.0;
    }
    if signals.review_topics.invite.negative > 0 {
        friction_penalty += 6.0;
    }
    let optional_social_mode = modes.signal_count <= 1
        && !modes.online_pvp
        && !modes.local_coop
        && signals.review_topics.multiplayer.total() == 0;
    if optional_social_mode {
        match signals.activity.current_players.unwrap_or(0) {
            0..=499 => friction_penalty += 10.0,
            500..=999 => friction_penalty += 6.0,
            _ => {}
        }
    }
    if optional_social_mode
        && has_any_tag(signals, &["SINGLE_PLAYER"])
        && !has_any_tag(signals, &["PVP", "PARTY"])
    {
        friction_penalty += 8.0;
    }

    (mode_score + group_score + review_sentiment - friction_penalty).clamp(0.0, 100.0)
}

fn has_any_tag(signals: &CanonicalGameSignals, tags: &[&str]) -> bool {
    tags.iter()
        .any(|expected| signals.tags.iter().any(|tag| tag == expected))
}

use mpgs_domain::{RankingSignals, UserPreferences};

use crate::unit;

/// Apply hard filters before scoring. Returns false when the candidate must be dropped.
pub fn hard_filter(
    prefs: &UserPreferences,
    recommended_min: Option<u8>,
    recommended_max: Option<u8>,
    dominant_mode: Option<&str>,
    signals: &RankingSignals,
) -> bool {
    if let Some(mode) = dominant_mode {
        let mode_l = mode.to_ascii_lowercase();
        if prefs
            .excluded_modes
            .iter()
            .any(|ex| mode_l.contains(&ex.to_ascii_lowercase()))
        {
            return false;
        }
    }

    // Party size completely disjoint => drop.
    if let (Some(min), Some(max)) = (recommended_min, recommended_max) {
        let party = prefs.party_size;
        if party < min || party > max {
            // Allow soft mismatch only if ranges overlap poorly? Spec says hard filter on
            // complete non-intersection. Party outside [min,max] is non-intersecting.
            return false;
        }
    }

    // Require some multiplayer evidence confidence.
    let mp = &signals.multiplayer;
    let any_mp = unit(mp.private_session)
        + unit(mp.online_coop)
        + unit(mp.self_host_or_dedicated)
        + unit(mp.matchmaking_core)
        > 0.05;
    if !any_mp && unit(signals.data_confidence) < 0.2 {
        return false;
    }

    true
}

/// Mutate ranking signals with preference-derived personal_fit and group_size adjustments.
pub fn apply_personalization(
    prefs: &UserPreferences,
    signals: &mut RankingSignals,
    recommended_min: Option<u8>,
    recommended_max: Option<u8>,
) {
    let party = prefs.party_size;
    let (size_fit, size_mismatch) = match (recommended_min, recommended_max) {
        (Some(min), Some(max)) if party >= min && party <= max => {
            let mid = f64::from(min + max) / 2.0;
            let span = f64::from(max.saturating_sub(min).max(1));
            let dist = (f64::from(party) - mid).abs() / span;
            (unit(1.0 - dist * 0.5), 0.0)
        }
        (Some(min), Some(max)) => {
            let outside = if party < min {
                f64::from(min - party)
            } else {
                f64::from(party.saturating_sub(max))
            };
            (0.2, unit(outside / 8.0))
        }
        _ => (0.5, 0.0),
    };
    signals.multiplayer.group_size_fit = size_fit;
    signals.multiplayer.group_size_mismatch = size_mismatch;

    let coop_pref = 1.0 - unit(prefs.coop_competitive);
    let competitive_pref = unit(prefs.coop_competitive);
    let host_pref = unit(prefs.self_hosting_willingness);

    let coop_alignment = unit(signals.multiplayer.online_coop) * coop_pref
        + unit(signals.multiplayer.private_session) * coop_pref
        + unit(signals.multiplayer.self_host_or_dedicated) * host_pref;

    let competitive_alignment = unit(signals.multiplayer.matchmaking_core) * competitive_pref;

    let penalty = unit(signals.multiplayer.public_world_dependency) * coop_pref * 0.5
        + unit(signals.multiplayer.service_shutdown_risk) * 0.3;

    signals.personal_fit = unit(
        0.55 * coop_alignment + 0.35 * competitive_alignment + 0.25 - penalty + 0.15 * size_fit,
    );
}

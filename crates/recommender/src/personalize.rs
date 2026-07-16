use mpgs_domain::{CandidateAvailability, RankingSignals, UserPreferences};

use crate::unit;

/// Apply hard filters before scoring. Returns false when the candidate must be dropped.
pub fn hard_filter(
    prefs: &UserPreferences,
    recommended_min: Option<u8>,
    recommended_max: Option<u8>,
    dominant_mode: Option<&str>,
    signals: &RankingSignals,
    availability: &CandidateAvailability,
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

    if platform_list_mismatch(&prefs.platforms, &availability.platforms)
        || known_list_mismatch(&prefs.languages, &availability.languages)
    {
        return false;
    }

    if let (Some(candidate_min), Some(candidate_max)) = (
        availability.typical_session_minutes_min,
        availability.typical_session_minutes_max,
    ) && (candidate_max < prefs.session_minutes_min || candidate_min > prefs.session_minutes_max)
    {
        return false;
    }

    if availability.is_free != Some(true)
        && let (Some(max_price), Some(price), Some(currency)) = (
            prefs.budget_max_each_minor,
            availability.final_price_minor,
            availability.price_currency.as_deref(),
        )
        && currency.eq_ignore_ascii_case(&prefs.budget_currency)
        && price > max_price
    {
        return false;
    }

    true
}

fn known_list_mismatch(required: &[String], available: &[String]) -> bool {
    !required.is_empty()
        && !available.is_empty()
        && !required.iter().any(|required| {
            available
                .iter()
                .any(|available| required.eq_ignore_ascii_case(available))
        })
}

fn platform_list_mismatch(required: &[String], available: &[String]) -> bool {
    !required.is_empty()
        && !available.is_empty()
        && !required.iter().any(|required| {
            available
                .iter()
                .any(|available| platform_value_matches(required, available))
        })
}

fn platform_value_matches(required: &str, available: &str) -> bool {
    required.eq_ignore_ascii_case(available)
        || (matches!(required.to_ascii_lowercase().as_str(), "mac" | "macos")
            && matches!(available.to_ascii_lowercase().as_str(), "mac" | "macos"))
}

/// Mutate ranking signals with preference-derived personal_fit and group_size adjustments.
pub fn apply_personalization(
    prefs: &UserPreferences,
    signals: &mut RankingSignals,
    recommended_min: Option<u8>,
    recommended_max: Option<u8>,
    availability: &CandidateAvailability,
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

    let gameplay_fit = unit(
        0.55 * coop_alignment + 0.35 * competitive_alignment + 0.25 - penalty + 0.15 * size_fit,
    );
    let availability_fit = availability_fit(prefs, availability);
    signals.personal_fit = unit(0.8 * gameplay_fit + 0.2 * availability_fit);

    if !prefs.platforms.is_empty() && !availability.platforms.is_empty() {
        let supported = prefs
            .platforms
            .iter()
            .filter(|required| {
                availability
                    .platforms
                    .iter()
                    .any(|available| platform_value_matches(required, available))
            })
            .count();
        signals.multiplayer.cross_platform_fit =
            unit(supported as f64 / prefs.platforms.len() as f64);
    }
}

fn availability_fit(prefs: &UserPreferences, availability: &CandidateAvailability) -> f64 {
    let mut total = 0.0;
    let mut known = 0_u8;
    if !prefs.platforms.is_empty() && !availability.platforms.is_empty() {
        known += 1;
        if !platform_list_mismatch(&prefs.platforms, &availability.platforms) {
            total += 1.0;
        }
    }
    if !prefs.languages.is_empty() && !availability.languages.is_empty() {
        known += 1;
        if !known_list_mismatch(&prefs.languages, &availability.languages) {
            total += 1.0;
        }
    }
    if let (Some(candidate_min), Some(candidate_max)) = (
        availability.typical_session_minutes_min,
        availability.typical_session_minutes_max,
    ) {
        known += 1;
        if candidate_max >= prefs.session_minutes_min && candidate_min <= prefs.session_minutes_max
        {
            total += 1.0;
        }
    }
    if availability.is_free == Some(true) {
        known += 1;
        total += 1.0;
    } else if let (Some(max_price), Some(price), Some(currency)) = (
        prefs.budget_max_each_minor,
        availability.final_price_minor,
        availability.price_currency.as_deref(),
    ) && currency.eq_ignore_ascii_case(&prefs.budget_currency)
    {
        known += 1;
        if price <= max_price {
            total += 1.0;
        }
    }
    if known == 0 {
        0.5
    } else {
        total / f64::from(known)
    }
}

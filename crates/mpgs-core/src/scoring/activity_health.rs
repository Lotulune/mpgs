use crate::scoring::signals::ActivityStats;

pub fn score_activity_health(activity: &ActivityStats) -> f64 {
    let Some(players) = activity.current_players else {
        return 50.0;
    };

    let players = players as f64;
    let absolute_activity = (20.0 + (players + 1.0).log10() * 18.0).clamp(0.0, 85.0);
    let cohort_percentile_score = (((players + 1.0).log10() / 4.0) * 100.0).clamp(0.0, 100.0);

    (0.55 * absolute_activity + 0.45 * cohort_percentile_score).clamp(0.0, 100.0)
}

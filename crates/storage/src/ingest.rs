use rusqlite::Connection;
use rusqlite::params;

use mpgs_steam_source::{
    AppCatalogProposal, AppRelationProposal, AppTypeProposal, CcuProposal, RelationTypeProposal,
    ReleaseStateProposal, ReviewSummaryProposal, StoreDetailsProposal,
};

use crate::catalog::{self, upsert_app, upsert_relation};
use crate::curation::{has_active_override, insert_feature_evidence};
use crate::error::StorageResult;
use crate::util::{day_utc_from_ms, wilson_lower_bound};

pub fn ingest_app_catalog(
    conn: &Connection,
    proposal: &AppCatalogProposal,
    now_ms: i64,
) -> StorageResult<()> {
    let app_type = app_type_str(proposal.app_type);
    let source_modified = proposal.last_modified.map(|s| i64::from(s) * 1000);
    upsert_app(
        conn,
        proposal.app_id,
        app_type,
        &proposal.name,
        "unknown",
        None,
        None,
        source_modified,
        now_ms,
    )?;
    Ok(())
}

pub fn ingest_review_summary(
    conn: &Connection,
    proposal: &ReviewSummaryProposal,
    now_ms: i64,
) -> StorageResult<()> {
    catalog::ensure_app_stub(
        conn,
        proposal.app_id,
        &format!("app-{}", proposal.app_id),
        now_ms,
    )?;
    let wilson = wilson_lower_bound(proposal.total_positive, proposal.total_reviews);
    conn.execute(
        "INSERT OR REPLACE INTO review_snapshots (
            app_id, region_scope, language_scope, captured_at_ms,
            total_positive, total_negative, total_reviews, review_score, review_score_desc,
            wilson_lower, filter_offtopic_activity, parameter_hash, content_hash, source
         ) VALUES (?1, 'all', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            proposal.app_id,
            proposal.language_scope,
            now_ms,
            proposal.total_positive,
            proposal.total_negative,
            proposal.total_reviews,
            proposal.review_score,
            proposal.review_score_desc,
            wilson,
            if proposal.filter_offtopic_activity {
                1
            } else {
                0
            },
            proposal.parameter_hash,
            proposal.content_hash,
            proposal.source,
        ],
    )?;
    Ok(())
}

pub fn ingest_ccu(conn: &Connection, proposal: &CcuProposal, now_ms: i64) -> StorageResult<()> {
    catalog::ensure_app_stub(
        conn,
        proposal.app_id,
        &format!("app-{}", proposal.app_id),
        now_ms,
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO player_snapshots (
            app_id, captured_at_ms, player_count, result_code, missing_reason,
            content_hash, source, offline_players_excluded
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            proposal.app_id,
            now_ms,
            proposal.player_count,
            proposal.result_code,
            proposal.missing_reason,
            proposal.content_hash,
            proposal.source,
            if proposal.offline_players_excluded {
                1
            } else {
                0
            },
        ],
    )?;
    upsert_player_daily(conn, proposal.app_id, proposal.player_count, now_ms)?;
    Ok(())
}

pub fn ingest_store_details(
    conn: &Connection,
    details: &StoreDetailsProposal,
    relations: &[AppRelationProposal],
    now_ms: i64,
) -> StorageResult<()> {
    let app_type = app_type_str(details.app_type);
    let release_state = release_state_str(details.release_state);
    let name = details
        .name
        .clone()
        .unwrap_or_else(|| format!("app-{}", details.app_id));

    // Capture prior release state/date for event log.
    let prior = catalog::get_app(conn, details.app_id)?;
    upsert_app(
        conn,
        details.app_id,
        app_type,
        &name,
        release_state,
        details.release_date_raw.as_deref(),
        None,
        None,
        now_ms,
    )?;

    if let Some(prev) = prior
        && (prev.release_state != release_state
            || prev.release_date.as_deref() != details.release_date_raw.as_deref())
    {
        conn.execute(
            "INSERT INTO release_events (
                app_id, old_release_date, new_release_date, old_precision, new_precision,
                old_release_state, new_release_state, source, observed_at_ms
             ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8)",
            params![
                details.app_id,
                prev.release_date,
                details.release_date_raw,
                prev.release_date_precision,
                prev.release_state,
                release_state,
                details.source,
                now_ms
            ],
        )?;
    }

    for relation in relations {
        let rel = relation_type_str(&relation.relation_type);
        upsert_relation(
            conn,
            relation.source_app_id,
            relation.target_app_id,
            rel,
            relation.confidence,
            false,
            now_ms,
        )?;
    }

    // Category multiplayer hints become low-confidence evidence only.
    for hint in &details.multiplayer_category_hints {
        insert_feature_evidence(
            conn,
            details.app_id,
            "category_hint",
            &serde_json::json!(hint),
            "store_category",
            details.source,
            0.3,
            now_ms,
        )?;
    }
    Ok(())
}

/// Apply a source-derived multiplayer boolean without clobbering human overrides.
#[allow(clippy::too_many_arguments)]
pub fn ingest_multiplayer_bool(
    conn: &Connection,
    app_id: u32,
    feature_name: &str,
    value: bool,
    source_type: &str,
    source_ref: &str,
    confidence: f64,
    now_ms: i64,
) -> StorageResult<bool> {
    insert_feature_evidence(
        conn,
        app_id,
        feature_name,
        &serde_json::json!(value),
        source_type,
        source_ref,
        confidence,
        now_ms,
    )?;

    if has_active_override(conn, app_id, feature_name)? {
        return Ok(false);
    }

    catalog::set_profile_bool_field(conn, app_id, feature_name, Some(value), now_ms)?;
    Ok(true)
}

struct DailyAgg {
    min_ccu: Option<i64>,
    max_ccu: Option<i64>,
    mean_ccu: Option<f64>,
    sample_count: i64,
    missing_rate: f64,
}

fn upsert_player_daily(
    conn: &Connection,
    app_id: u32,
    player_count: Option<u32>,
    now_ms: i64,
) -> StorageResult<()> {
    let day = day_utc_from_ms(now_ms);
    let existing: Option<DailyAgg> = conn
        .query_row(
            "SELECT min_ccu, max_ccu, mean_ccu, sample_count, missing_rate
             FROM player_daily WHERE app_id = ?1 AND day_utc = ?2",
            params![app_id, day],
            |row| {
                Ok(DailyAgg {
                    min_ccu: row.get(0)?,
                    max_ccu: row.get(1)?,
                    mean_ccu: row.get(2)?,
                    sample_count: row.get(3)?,
                    missing_rate: row.get(4)?,
                })
            },
        )
        .optional_compat()?;

    match (existing, player_count) {
        (None, Some(count)) => {
            let c = i64::from(count);
            conn.execute(
                "INSERT INTO player_daily (
                    app_id, day_utc, min_ccu, max_ccu, mean_ccu, median_approx_ccu,
                    sample_count, missing_rate, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?3, ?3, ?3, 1, 0, ?4)",
                params![app_id, day, c, now_ms],
            )?;
        }
        (None, None) => {
            conn.execute(
                "INSERT INTO player_daily (
                    app_id, day_utc, min_ccu, max_ccu, mean_ccu, median_approx_ccu,
                    sample_count, missing_rate, updated_at_ms
                 ) VALUES (?1, ?2, NULL, NULL, NULL, NULL, 0, 1, ?3)",
                params![app_id, day, now_ms],
            )?;
        }
        (Some(agg), Some(count)) => {
            let c = i64::from(count);
            let sample = agg.sample_count + 1;
            let min_v = Some(agg.min_ccu.map_or(c, |m| m.min(c)));
            let max_v = Some(agg.max_ccu.map_or(c, |m| m.max(c)));
            let mean_v = agg.mean_ccu.map_or(c as f64, |m| {
                (m * agg.sample_count as f64 + c as f64) / sample as f64
            });
            conn.execute(
                "UPDATE player_daily SET
                    min_ccu = ?1, max_ccu = ?2, mean_ccu = ?3, median_approx_ccu = ?3,
                    sample_count = ?4, missing_rate = 0, updated_at_ms = ?5
                 WHERE app_id = ?6 AND day_utc = ?7",
                params![min_v, max_v, mean_v, sample, now_ms, app_id, day],
            )?;
        }
        (Some(agg), None) => {
            let total_slots = agg.sample_count + 1;
            let missing = ((agg.missing_rate * agg.sample_count as f64) + 1.0) / total_slots as f64;
            conn.execute(
                "UPDATE player_daily SET
                    sample_count = ?1, missing_rate = ?2, updated_at_ms = ?3
                 WHERE app_id = ?4 AND day_utc = ?5",
                params![total_slots, missing, now_ms, app_id, day],
            )?;
        }
    }
    Ok(())
}

trait OptionalCompat<T> {
    fn optional_compat(self) -> StorageResult<Option<T>>;
}

impl<T> OptionalCompat<T> for Result<T, rusqlite::Error> {
    fn optional_compat(self) -> StorageResult<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

fn app_type_str(value: AppTypeProposal) -> &'static str {
    match value {
        AppTypeProposal::Game => "game",
        AppTypeProposal::Demo => "demo",
        AppTypeProposal::Playtest => "playtest",
        AppTypeProposal::Dlc => "dlc",
        AppTypeProposal::Tool => "tool",
        AppTypeProposal::Application => "application",
        AppTypeProposal::Music => "music",
        AppTypeProposal::Video => "video",
        AppTypeProposal::Series => "series",
        AppTypeProposal::Comic => "comic",
        AppTypeProposal::Advertising => "advertising",
        AppTypeProposal::Mod => "mod",
        AppTypeProposal::Hardware => "hardware",
        AppTypeProposal::Unknown => "unknown",
    }
}

fn release_state_str(value: ReleaseStateProposal) -> &'static str {
    match value {
        ReleaseStateProposal::Released => "released",
        ReleaseStateProposal::Upcoming => "upcoming",
        ReleaseStateProposal::ComingSoon => "coming_soon",
        ReleaseStateProposal::Retired => "retired",
        ReleaseStateProposal::Unknown => "unknown",
    }
}

fn relation_type_str(value: &RelationTypeProposal) -> &'static str {
    match value {
        RelationTypeProposal::DemoOf => "demo_of",
        RelationTypeProposal::PlaytestOf => "playtest_of",
        RelationTypeProposal::DedicatedServerFor => "dedicated_server_for",
        RelationTypeProposal::EditionOf => "edition_of",
        RelationTypeProposal::Replaces => "replaces",
    }
}

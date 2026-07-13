//! Read models for feeds, search, calendar, and game detail.

use mpgs_domain::{MultiplayerSignals, RankingSignals, SteamAppId};
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{StorageError, StorageResult};
use crate::models::AppRecord;
use crate::util::sql_to_opt_bool;

#[derive(Debug, Clone, PartialEq)]
pub struct GameCandidateRow {
    pub app_id: SteamAppId,
    pub name: String,
    pub app_type: String,
    pub release_state: String,
    pub release_date: Option<String>,
    pub dominant_mode: Option<String>,
    pub private_session: Option<bool>,
    pub online_coop: Option<bool>,
    pub self_hosted_server: Option<bool>,
    pub recommended_min: Option<u8>,
    pub recommended_max: Option<u8>,
    pub profile_confidence: Option<f64>,
    pub total_reviews: Option<u32>,
    pub total_positive: Option<u32>,
    pub latest_ccu: Option<u32>,
}

impl GameCandidateRow {
    pub fn to_ranking_signals(&self) -> RankingSignals {
        let quality = match (self.total_positive, self.total_reviews) {
            (Some(pos), Some(total)) if total > 0 => f64::from(pos) / f64::from(total),
            _ => 0.5,
        };
        let popularity = self
            .latest_ccu
            .map(|c| (1.0 + (c as f64).ln()).min(12.0) / 12.0)
            .unwrap_or(0.3);
        let confidence = self.profile_confidence.unwrap_or(0.4);
        let mode = self.dominant_mode.as_deref().unwrap_or("");
        let matchmaking_core = mode.contains("match") || mode.contains("competitive");
        let public_world_mode = mode.contains("mmo") || mode.contains("public");
        // Dedicated servers for matchmaking-core titles do not count as friend-group self-host.
        let private = if matchmaking_core {
            bool01(self.private_session) * 0.25
        } else {
            bool01(self.private_session)
        };
        let coop = if matchmaking_core {
            bool01(self.online_coop) * 0.2
        } else {
            bool01(self.online_coop)
        };
        let self_host = if matchmaking_core {
            bool01(self.self_hosted_server) * 0.15
        } else {
            bool01(self.self_hosted_server)
        };
        let matchmaking = if matchmaking_core { 0.95 } else { 0.12 };
        let public_world = if public_world_mode {
            0.85
        } else if matchmaking_core {
            0.75
        } else {
            0.12
        };
        let shutdown = if mode.contains("live") || matchmaking_core {
            0.35
        } else {
            0.1
        };

        RankingSignals {
            multiplayer: MultiplayerSignals {
                private_session: private,
                self_host_or_dedicated: self_host,
                online_coop: coop,
                group_size_fit: 0.5,
                low_public_population_dependency: 1.0 - public_world,
                drop_in_out: 0.4,
                cross_platform_fit: 0.4,
                matchmaking_core: matchmaking,
                public_world_dependency: public_world,
                group_size_mismatch: 0.0,
                service_shutdown_risk: shutdown,
                external_account_friction: 0.1,
                platform_or_anticheat_restriction: 0.1,
            },
            quality,
            popularity,
            momentum: popularity * 0.7,
            evidence: confidence,
            freshness: if self.release_state == "released" {
                0.5
            } else {
                0.8
            },
            data_confidence: confidence,
            demo_playability: if self.app_type == "demo" { 0.9 } else { 0.2 },
            release_date_confidence: if self.release_date.is_some() {
                0.8
            } else {
                0.3
            },
            release_proximity: if self.release_state == "coming_soon"
                || self.release_state == "upcoming"
            {
                0.7
            } else {
                0.2
            },
            studio_prior: 0.5,
            longevity: if self.release_state == "released" {
                0.7
            } else {
                0.2
            },
            maintenance_health: 0.6,
            risk: shutdown * 0.5 + public_world * 0.2,
            personal_fit: 0.5,
        }
    }
}

fn bool01(value: Option<bool>) -> f64 {
    match value {
        Some(true) => 1.0,
        Some(false) => 0.05,
        None => 0.0,
    }
}

pub fn list_candidates(conn: &Connection, limit: i64) -> StorageResult<Vec<GameCandidateRow>> {
    let mut stmt = conn.prepare(
        "SELECT a.app_id, a.canonical_name, a.app_type, a.release_state, a.release_date,
                p.dominant_mode, p.private_session, p.online_coop, p.self_hosted_server,
                p.recommended_min_players, p.recommended_max_players, p.profile_confidence,
                (
                    SELECT r.total_reviews FROM review_snapshots r
                    WHERE r.app_id = a.app_id
                    ORDER BY r.captured_at_ms DESC LIMIT 1
                ),
                (
                    SELECT r.total_positive FROM review_snapshots r
                    WHERE r.app_id = a.app_id
                    ORDER BY r.captured_at_ms DESC LIMIT 1
                ),
                (
                    SELECT s.player_count FROM player_snapshots s
                    WHERE s.app_id = a.app_id AND s.player_count IS NOT NULL
                    ORDER BY s.captured_at_ms DESC LIMIT 1
                )
         FROM apps a
         LEFT JOIN multiplayer_profiles p ON p.app_id = a.app_id
         WHERE a.app_type IN ('game', 'demo', 'playtest', 'unknown')
         ORDER BY a.updated_at_ms DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], map_candidate)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn search_by_name(
    conn: &Connection,
    query: &str,
    limit: i64,
) -> StorageResult<Vec<GameCandidateRow>> {
    let pattern = format!("%{}%", query.trim());
    let mut stmt = conn.prepare(
        "SELECT a.app_id, a.canonical_name, a.app_type, a.release_state, a.release_date,
                p.dominant_mode, p.private_session, p.online_coop, p.self_hosted_server,
                p.recommended_min_players, p.recommended_max_players, p.profile_confidence,
                NULL, NULL, NULL
         FROM apps a
         LEFT JOIN multiplayer_profiles p ON p.app_id = a.app_id
         WHERE a.canonical_name LIKE ?1 COLLATE NOCASE
         ORDER BY a.canonical_name
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![pattern, limit], map_candidate)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn get_game_detail(conn: &Connection, app_id: u32) -> StorageResult<Option<GameCandidateRow>> {
    conn.query_row(
        "SELECT a.app_id, a.canonical_name, a.app_type, a.release_state, a.release_date,
                p.dominant_mode, p.private_session, p.online_coop, p.self_hosted_server,
                p.recommended_min_players, p.recommended_max_players, p.profile_confidence,
                (
                    SELECT r.total_reviews FROM review_snapshots r
                    WHERE r.app_id = a.app_id
                    ORDER BY r.captured_at_ms DESC LIMIT 1
                ),
                (
                    SELECT r.total_positive FROM review_snapshots r
                    WHERE r.app_id = a.app_id
                    ORDER BY r.captured_at_ms DESC LIMIT 1
                ),
                (
                    SELECT s.player_count FROM player_snapshots s
                    WHERE s.app_id = a.app_id AND s.player_count IS NOT NULL
                    ORDER BY s.captured_at_ms DESC LIMIT 1
                )
         FROM apps a
         LEFT JOIN multiplayer_profiles p ON p.app_id = a.app_id
         WHERE a.app_id = ?1",
        params![app_id],
        map_candidate,
    )
    .optional()
    .map_err(StorageError::from)
}

pub fn list_evidence(
    conn: &Connection,
    app_id: u32,
    feature: Option<&str>,
) -> StorageResult<Vec<EvidenceRow>> {
    if let Some(feature) = feature {
        let mut stmt = conn.prepare(
            "SELECT evidence_id, feature_name, value_json, source_type, source_ref, confidence, observed_at_ms
             FROM feature_evidence
             WHERE app_id = ?1 AND feature_name = ?2 AND is_active = 1
             ORDER BY observed_at_ms DESC LIMIT 50",
        )?;
        let rows = stmt.query_map(params![app_id, feature], map_evidence)?;
        return rows.collect::<Result<Vec<_>, _>>().map_err(Into::into);
    }
    let mut stmt = conn.prepare(
        "SELECT evidence_id, feature_name, value_json, source_type, source_ref, confidence, observed_at_ms
         FROM feature_evidence
         WHERE app_id = ?1 AND is_active = 1
         ORDER BY observed_at_ms DESC LIMIT 50",
    )?;
    let rows = stmt.query_map(params![app_id], map_evidence)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceRow {
    pub evidence_id: i64,
    pub feature_name: String,
    pub value_json: String,
    pub source_type: String,
    pub source_ref: String,
    pub confidence: f64,
    pub observed_at_ms: i64,
}

pub fn list_calendar(
    conn: &Connection,
    from_date: &str,
    to_date: &str,
) -> StorageResult<(Vec<AppRecord>, Vec<AppRecord>)> {
    let mut dated = Vec::new();
    let mut undated = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT app_id, app_type, canonical_name, release_state, release_date,
                release_date_precision, is_early_access, current_data_confidence,
                source_modified_at_ms, created_at_ms, updated_at_ms
         FROM apps
         WHERE release_state IN ('upcoming', 'coming_soon')
            OR (release_date IS NOT NULL AND release_date >= ?1 AND release_date <= ?2)",
    )?;
    let rows = stmt.query_map(params![from_date, to_date], |row| {
        Ok(AppRecord {
            app_id: row.get::<_, i64>(0)? as u32,
            app_type: row.get(1)?,
            canonical_name: row.get(2)?,
            release_state: row.get(3)?,
            release_date: row.get(4)?,
            release_date_precision: row.get(5)?,
            is_early_access: sql_to_opt_bool(row.get(6)?),
            current_data_confidence: row.get(7)?,
            source_modified_at_ms: row.get(8)?,
            created_at_ms: row.get(9)?,
            updated_at_ms: row.get(10)?,
        })
    })?;
    for row in rows {
        let app = row?;
        match &app.release_date {
            Some(d) if d.len() >= 10 && d.as_str() >= from_date && d.as_str() <= to_date => {
                dated.push(app);
            }
            Some(_) | None => undated.push(app),
        }
    }
    Ok((dated, undated))
}

fn map_candidate(row: &rusqlite::Row<'_>) -> rusqlite::Result<GameCandidateRow> {
    Ok(GameCandidateRow {
        app_id: row.get::<_, i64>(0)? as u32,
        name: row.get(1)?,
        app_type: row.get(2)?,
        release_state: row.get(3)?,
        release_date: row.get(4)?,
        dominant_mode: row.get(5)?,
        private_session: sql_to_opt_bool(row.get(6)?),
        online_coop: sql_to_opt_bool(row.get(7)?),
        self_hosted_server: sql_to_opt_bool(row.get(8)?),
        recommended_min: row.get::<_, Option<i64>>(9)?.map(|v| v.clamp(0, 255) as u8),
        recommended_max: row
            .get::<_, Option<i64>>(10)?
            .map(|v| v.clamp(0, 255) as u8),
        profile_confidence: row.get(11)?,
        total_reviews: row.get::<_, Option<i64>>(12)?.map(|v| v as u32),
        total_positive: row.get::<_, Option<i64>>(13)?.map(|v| v as u32),
        latest_ccu: row.get::<_, Option<i64>>(14)?.map(|v| v as u32),
    })
}

fn map_evidence(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceRow> {
    Ok(EvidenceRow {
        evidence_id: row.get(0)?,
        feature_name: row.get(1)?,
        value_json: row.get(2)?,
        source_type: row.get(3)?,
        source_ref: row.get(4)?,
        confidence: row.get(5)?,
        observed_at_ms: row.get(6)?,
    })
}

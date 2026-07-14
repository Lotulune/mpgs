//! High-level repository façade over the SQLite database.

use crate::backup::{self, restore_from_backup};
use crate::catalog;
use crate::curation;
use crate::db::Database;
use crate::error::StorageResult;
use crate::ingest;
use crate::jobs;
use crate::models::{
    AppRecord, CreateOverrideRequest, CurationOverride, EffectiveFeatureValue, EnqueueJob,
    JobRecord, M3CatalogCoverage, MultiplayerProfile,
};
use crate::quality::{self, QualityFinding};
use mpgs_steam_source::{
    AppCatalogProposal, AppRelationProposal, CcuProposal, ReviewSummaryProposal,
    StoreDetailsProposal, StoreSearchPage,
};
use std::path::Path;

#[derive(Clone)]
pub struct Repository {
    db: Database,
}

impl Repository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn database(&self) -> &Database {
        &self.db
    }

    pub fn migrate(&self) -> StorageResult<i64> {
        self.db.migrate()
    }

    pub fn assert_ready(&self) -> StorageResult<()> {
        self.db.assert_ready()
    }

    pub fn readiness_check(&self) -> StorageResult<()> {
        self.db.readiness_check()?;
        self.db.with_conn(|conn| {
            let active_algorithms: i64 = conn.query_row(
                "SELECT COUNT(*) FROM algorithm_configs WHERE status = 'active'",
                [],
                |row| row.get(0),
            )?;
            let apps: i64 = conn.query_row("SELECT COUNT(*) FROM apps", [], |row| row.get(0))?;
            if active_algorithms != 1 {
                return Err(crate::StorageError::migration(
                    "exactly one active algorithm config is required",
                ));
            }
            if apps == 0 {
                return Err(crate::StorageError::migration(
                    "catalog has no app snapshot yet",
                ));
            }
            crate::users::active_algorithm_config(conn)?;
            Ok(())
        })
    }

    pub fn upsert_catalog(&self, proposal: &AppCatalogProposal) -> StorageResult<()> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            ingest::ingest_app_catalog(&tx, proposal, now)?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn ingest_review(&self, proposal: &ReviewSummaryProposal) -> StorageResult<()> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            ingest::ingest_review_summary(&tx, proposal, now)?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn ingest_ccu(&self, proposal: &CcuProposal) -> StorageResult<()> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            ingest::ingest_ccu(&tx, proposal, now)?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn ingest_store_details(
        &self,
        details: &StoreDetailsProposal,
        relations: &[AppRelationProposal],
    ) -> StorageResult<()> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            ingest::ingest_store_details(&tx, details, relations, now)?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn ingest_store_search_page(&self, page: &StoreSearchPage) -> StorageResult<usize> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            let ingested = ingest::ingest_store_search_page(&tx, page, now)?;
            tx.commit()?;
            Ok(ingested)
        })
    }

    pub fn ingest_multiplayer_bool(
        &self,
        app_id: u32,
        feature_name: &str,
        value: bool,
        source_type: &str,
        source_ref: &str,
        confidence: f64,
    ) -> StorageResult<bool> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            let applied = ingest::ingest_multiplayer_bool(
                &tx,
                app_id,
                feature_name,
                value,
                source_type,
                source_ref,
                confidence,
                now,
            )?;
            tx.commit()?;
            Ok(applied)
        })
    }

    pub fn get_app(&self, app_id: u32) -> StorageResult<Option<AppRecord>> {
        self.db.with_conn(|conn| catalog::get_app(conn, app_id))
    }

    pub fn get_profile(&self, app_id: u32) -> StorageResult<Option<MultiplayerProfile>> {
        self.db
            .with_conn(|conn| catalog::get_multiplayer_profile(conn, app_id))
    }

    pub fn count_apps(&self) -> StorageResult<i64> {
        self.db.with_conn(catalog::count_apps)
    }

    pub fn m3_catalog_coverage(&self) -> StorageResult<M3CatalogCoverage> {
        self.db.with_conn(|conn| {
            conn.query_row(
                "WITH candidates AS (
                     SELECT a.app_id
                     FROM apps a
                     WHERE a.app_type IN ('game', 'demo', 'playtest')
                       AND (
                           EXISTS (
                               SELECT 1 FROM feature_evidence e
                               WHERE e.app_id = a.app_id
                                 AND e.feature_name = 'category_hint'
                                 AND e.is_active = 1
                                 AND e.confidence >= 0.3
                           )
                           OR EXISTS (
                               SELECT 1 FROM multiplayer_profiles profile
                               WHERE profile.app_id = a.app_id
                                 AND (
                                     profile.dominant_mode IS NOT NULL
                                     OR profile.private_session IS NOT NULL
                                     OR profile.online_coop IS NOT NULL
                                     OR profile.self_hosted_server IS NOT NULL
                                     OR profile.drop_in_out IS NOT NULL
                                     OR profile.crossplay IS NOT NULL
                                     OR profile.recommended_max_players IS NOT NULL
                                 )
                           )
                       )
                 )
                 SELECT
                     COUNT(*),
                     COALESCE(SUM(CASE WHEN EXISTS (
                         SELECT 1 FROM feature_evidence evidence
                         WHERE evidence.app_id = candidates.app_id
                           AND evidence.feature_name = 'category_hint'
                           AND evidence.is_active = 1
                           AND evidence.confidence >= 0.3
                     ) THEN 1 ELSE 0 END), 0),
                     COALESCE(SUM(CASE WHEN
                         profile.dominant_mode IS NOT NULL
                         OR profile.private_session IS NOT NULL
                         OR profile.online_coop IS NOT NULL
                         OR profile.self_hosted_server IS NOT NULL
                         OR profile.drop_in_out IS NOT NULL
                         OR profile.crossplay IS NOT NULL
                         OR profile.recommended_max_players IS NOT NULL
                     THEN 1 ELSE 0 END), 0),
                     COALESCE(SUM(CASE WHEN profile.profile_confidence >= 0.7 AND (
                         profile.dominant_mode IS NOT NULL
                         OR profile.private_session = 1
                         OR profile.online_coop = 1
                         OR profile.self_hosted_server = 1
                     ) THEN 1 ELSE 0 END), 0),
                     COALESCE(SUM(CASE WHEN COALESCE(v.platforms_json, '[]') <> '[]' THEN 1 ELSE 0 END), 0),
                     COALESCE(SUM(CASE WHEN COALESCE(v.languages_json, '[]') <> '[]' THEN 1 ELSE 0 END), 0),
                     COALESCE(SUM(CASE WHEN v.typical_session_minutes_min IS NOT NULL
                                       AND v.typical_session_minutes_max IS NOT NULL
                                  THEN 1 ELSE 0 END), 0),
                     COALESCE(SUM(CASE WHEN EXISTS (
                         SELECT 1 FROM price_snapshots price
                         WHERE price.app_id = candidates.app_id
                     ) THEN 1 ELSE 0 END), 0),
                     COALESCE(SUM(CASE WHEN EXISTS (
                         SELECT 1 FROM review_snapshots review
                         WHERE review.app_id = candidates.app_id
                     ) THEN 1 ELSE 0 END), 0),
                     COALESCE(SUM(CASE WHEN EXISTS (
                         SELECT 1 FROM player_snapshots player
                         WHERE player.app_id = candidates.app_id
                     ) THEN 1 ELSE 0 END), 0)
                 FROM candidates
                 LEFT JOIN multiplayer_profiles profile ON profile.app_id = candidates.app_id
                 LEFT JOIN app_availability v ON v.app_id = candidates.app_id",
                [],
                |row| {
                    Ok(M3CatalogCoverage {
                        normalized_multiplayer_candidates: row.get(0)?,
                        category_evidence_candidates: row.get(1)?,
                        recommendation_ready_profiles: row.get(2)?,
                        trusted_familiar_profiles: row.get(3)?,
                        with_platforms: row.get(4)?,
                        with_languages: row.get(5)?,
                        with_typical_session: row.get(6)?,
                        with_price: row.get(7)?,
                        with_reviews: row.get(8)?,
                        with_ccu: row.get(9)?,
                    })
                },
            )
            .map_err(Into::into)
        })
    }

    pub fn source_cursor(&self, cursor_key: &str) -> StorageResult<Option<String>> {
        self.db
            .with_conn(|conn| crate::source_state::load_cursor(conn, cursor_key))
    }

    pub fn save_source_cursor(
        &self,
        cursor_key: &str,
        source: &str,
        cursor: &serde_json::Value,
    ) -> StorageResult<()> {
        let now = self.db.now_ms();
        let cursor_json = serde_json::to_string(cursor)?;
        self.db.with_conn_mut(|conn| {
            crate::source_state::save_cursor(conn, cursor_key, source, &cursor_json, now)
        })
    }

    pub fn start_source_run(
        &self,
        source: &str,
        task_type: &str,
        parser_version: &str,
        notes: Option<&str>,
    ) -> StorageResult<i64> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            crate::source_state::start_run(conn, source, task_type, parser_version, notes, now)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_source_run(
        &self,
        run_id: i64,
        status: &str,
        request_count: i64,
        success_count: i64,
        error_category: Option<&str>,
        notes: Option<&str>,
    ) -> StorageResult<()> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            crate::source_state::finish_run(
                conn,
                run_id,
                status,
                request_count,
                success_count,
                error_category,
                notes,
                now,
            )
        })
    }

    pub fn create_override(
        &self,
        app_id: u32,
        request: &CreateOverrideRequest,
    ) -> StorageResult<CurationOverride> {
        let now = self.db.now_ms();
        self.db
            .with_conn_mut(|conn| curation::create_override(conn, app_id, request, now))
    }

    pub fn revoke_override(
        &self,
        override_id: i64,
        operator: &str,
        reason: &str,
        request_id: Option<&str>,
    ) -> StorageResult<CurationOverride> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            curation::revoke_override(conn, override_id, operator, reason, request_id, now)
        })
    }

    pub fn resolve_feature(
        &self,
        app_id: u32,
        feature_name: &str,
    ) -> StorageResult<EffectiveFeatureValue> {
        self.db
            .with_conn(|conn| curation::resolve_effective_feature(conn, app_id, feature_name))
    }

    pub fn enqueue_job(&self, job: &EnqueueJob) -> StorageResult<i64> {
        let now = self.db.now_ms();
        self.db
            .with_conn_mut(|conn| jobs::enqueue_job(conn, job, now))
    }

    pub fn lease_jobs(
        &self,
        owner: &str,
        limit: i64,
        lease_ms: i64,
        source_filter: Option<&str>,
    ) -> StorageResult<Vec<JobRecord>> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            jobs::lease_jobs(conn, owner, limit, lease_ms, now, source_filter)
        })
    }

    pub fn complete_job(
        &self,
        job_id: i64,
        owner: &str,
        idempotency_key: &str,
    ) -> StorageResult<JobRecord> {
        let now = self.db.now_ms();
        self.db
            .with_conn_mut(|conn| jobs::complete_job(conn, job_id, owner, idempotency_key, now))
    }

    pub fn fail_job(
        &self,
        job_id: i64,
        owner: &str,
        error_category: &str,
        retry_delay_ms: i64,
    ) -> StorageResult<JobRecord> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            jobs::fail_job(conn, job_id, owner, error_category, retry_delay_ms, now)
        })
    }

    pub fn run_quality_checks(&self) -> StorageResult<Vec<QualityFinding>> {
        let now = self.db.now_ms();
        self.db
            .with_conn_mut(|conn| quality::run_quality_checks(conn, now))
    }

    pub fn backup_to(&self, dest: impl AsRef<Path>) -> StorageResult<()> {
        backup::backup_to_path(&self.db, dest)
    }

    pub fn restore_backup(
        backup_path: impl AsRef<Path>,
        dest_path: impl AsRef<Path>,
        now_ms: i64,
    ) -> StorageResult<Self> {
        let db = restore_from_backup(backup_path, dest_path, now_ms)?;
        Ok(Self::new(db))
    }

    pub fn seed_demo_if_empty(&self) -> StorageResult<usize> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            let seeded = crate::seed::seed_demo_catalog_if_empty(&tx, now)?;
            tx.commit()?;
            Ok(seeded)
        })
    }

    pub fn ensure_runtime_defaults(&self) -> StorageResult<()> {
        let now = self.db.now_ms();
        self.db.with_conn_mut(|conn| {
            crate::users::ensure_algorithm_config(conn, now)?;
            Ok(())
        })
    }

    pub fn create_anonymous_session(&self) -> StorageResult<crate::users::SessionTokens> {
        let now = self.db.now_ms();
        self.db
            .with_conn_mut(|conn| crate::users::create_anonymous_session(conn, now))
    }

    pub fn refresh_anonymous_session(
        &self,
        refresh_token: &str,
    ) -> StorageResult<crate::users::SessionTokens> {
        let now = self.db.now_ms();
        self.db
            .with_conn_mut(|conn| crate::users::refresh_anonymous_session(conn, refresh_token, now))
    }

    pub fn resolve_access_token(&self, token: &str) -> StorageResult<String> {
        let now = self.db.now_ms();
        self.db
            .with_conn_mut(|conn| crate::users::resolve_user_id(conn, token, now))
    }

    pub fn get_preferences(&self, user_id: &str) -> StorageResult<mpgs_domain::UserPreferences> {
        self.db
            .with_conn(|conn| crate::users::get_preferences(conn, user_id))
    }

    pub fn put_preferences(
        &self,
        user_id: &str,
        prefs: &mpgs_domain::UserPreferences,
    ) -> StorageResult<mpgs_domain::UserPreferences> {
        let now = self.db.now_ms();
        self.db
            .with_conn_mut(|conn| crate::users::put_preferences(conn, user_id, prefs, now))
    }

    pub fn active_algorithm_config(&self) -> StorageResult<crate::users::ActiveAlgorithmConfig> {
        self.db.with_conn(crate::users::active_algorithm_config)
    }

    pub fn list_candidates(
        &self,
        section: mpgs_domain::FeedSection,
        cutoff_date: &str,
        today: &str,
        budget_currency: &str,
        config: &mpgs_domain::RecommendationConfig,
        limit: i64,
    ) -> StorageResult<Vec<crate::query::GameCandidateRow>> {
        self.db.with_conn(|conn| {
            crate::query::list_candidates(
                conn,
                section,
                cutoff_date,
                today,
                budget_currency,
                config,
                limit,
            )
        })
    }

    pub fn search_games(
        &self,
        q: &str,
        limit: i64,
    ) -> StorageResult<Vec<crate::query::GameCandidateRow>> {
        self.db
            .with_conn(|conn| crate::query::search_by_name(conn, q, limit))
    }

    pub fn game_detail(
        &self,
        app_id: u32,
    ) -> StorageResult<Option<crate::query::GameCandidateRow>> {
        self.db
            .with_conn(|conn| crate::query::get_game_detail(conn, app_id))
    }

    pub fn list_evidence(
        &self,
        app_id: u32,
        feature: Option<&str>,
    ) -> StorageResult<Vec<crate::query::EvidenceRow>> {
        self.db
            .with_conn(|conn| crate::query::list_evidence(conn, app_id, feature))
    }

    pub fn list_calendar(
        &self,
        from: &str,
        to: &str,
    ) -> StorageResult<(Vec<AppRecord>, Vec<AppRecord>)> {
        self.db
            .with_conn(|conn| crate::query::list_calendar(conn, from, to))
    }

    pub fn data_updated_at_ms(&self) -> StorageResult<i64> {
        self.db.with_conn(crate::query::data_updated_at_ms)
    }

    pub fn create_feedback(
        &self,
        user_id: &str,
        app_id: u32,
        feedback_type: mpgs_domain::FeedbackType,
        recommendation_run_id: Option<&str>,
        idempotency_key: &str,
        client_created_at_ms: Option<i64>,
    ) -> StorageResult<crate::feedback::FeedbackRecord> {
        let now = self.db.now_ms();
        let fingerprint = crate::feedback::request_fingerprint(
            app_id,
            feedback_type,
            recommendation_run_id,
            client_created_at_ms,
        )?;
        self.db.with_conn_mut(|conn| {
            crate::feedback::create_feedback(
                conn,
                user_id,
                app_id,
                feedback_type,
                recommendation_run_id,
                idempotency_key,
                client_created_at_ms,
                &fingerprint,
                now,
            )
        })
    }

    pub fn undo_feedback(
        &self,
        user_id: &str,
        feedback_id: i64,
    ) -> StorageResult<crate::feedback::FeedbackRecord> {
        let now = self.db.now_ms();
        self.db
            .with_conn_mut(|conn| crate::feedback::undo_feedback(conn, user_id, feedback_id, now))
    }

    pub fn list_active_feedback(
        &self,
        user_id: &str,
    ) -> StorageResult<Vec<crate::feedback::ActiveFeedback>> {
        self.db
            .with_conn(|conn| crate::feedback::list_active_feedback(conn, user_id))
    }
}

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
    JobRecord, MultiplayerProfile,
};
use crate::quality::{self, QualityFinding};
use mpgs_steam_source::{
    AppCatalogProposal, AppRelationProposal, CcuProposal, ReviewSummaryProposal,
    StoreDetailsProposal,
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

    pub fn upsert_catalog(&self, proposal: &AppCatalogProposal) -> StorageResult<()> {
        let now = self.db.now_ms();
        self.db
            .with_conn(|conn| ingest::ingest_app_catalog(conn, proposal, now))
    }

    pub fn ingest_review(&self, proposal: &ReviewSummaryProposal) -> StorageResult<()> {
        let now = self.db.now_ms();
        self.db
            .with_conn(|conn| ingest::ingest_review_summary(conn, proposal, now))
    }

    pub fn ingest_ccu(&self, proposal: &CcuProposal) -> StorageResult<()> {
        let now = self.db.now_ms();
        self.db
            .with_conn(|conn| ingest::ingest_ccu(conn, proposal, now))
    }

    pub fn ingest_store_details(
        &self,
        details: &StoreDetailsProposal,
        relations: &[AppRelationProposal],
    ) -> StorageResult<()> {
        let now = self.db.now_ms();
        self.db
            .with_conn(|conn| ingest::ingest_store_details(conn, details, relations, now))
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
        self.db.with_conn(|conn| {
            ingest::ingest_multiplayer_bool(
                conn,
                app_id,
                feature_name,
                value,
                source_type,
                source_ref,
                confidence,
                now,
            )
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

    pub fn create_override(
        &self,
        app_id: u32,
        request: &CreateOverrideRequest,
    ) -> StorageResult<CurationOverride> {
        let now = self.db.now_ms();
        self.db
            .with_conn(|conn| curation::create_override(conn, app_id, request, now))
    }

    pub fn revoke_override(
        &self,
        override_id: i64,
        operator: &str,
        reason: &str,
        request_id: Option<&str>,
    ) -> StorageResult<CurationOverride> {
        let now = self.db.now_ms();
        self.db.with_conn(|conn| {
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
        self.db.with_conn(|conn| jobs::enqueue_job(conn, job, now))
    }

    pub fn lease_jobs(
        &self,
        owner: &str,
        limit: i64,
        lease_ms: i64,
        source_filter: Option<&str>,
    ) -> StorageResult<Vec<JobRecord>> {
        let now = self.db.now_ms();
        self.db
            .with_conn(|conn| jobs::lease_jobs(conn, owner, limit, lease_ms, now, source_filter))
    }

    pub fn complete_job(
        &self,
        job_id: i64,
        owner: &str,
        idempotency_key: &str,
    ) -> StorageResult<JobRecord> {
        let now = self.db.now_ms();
        self.db
            .with_conn(|conn| jobs::complete_job(conn, job_id, owner, idempotency_key, now))
    }

    pub fn fail_job(
        &self,
        job_id: i64,
        owner: &str,
        error_category: &str,
        retry_delay_ms: i64,
    ) -> StorageResult<JobRecord> {
        let now = self.db.now_ms();
        self.db.with_conn(|conn| {
            jobs::fail_job(conn, job_id, owner, error_category, retry_delay_ms, now)
        })
    }

    pub fn run_quality_checks(&self) -> StorageResult<Vec<QualityFinding>> {
        let now = self.db.now_ms();
        self.db
            .with_conn(|conn| quality::run_quality_checks(conn, now))
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
}

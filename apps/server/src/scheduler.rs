//! Controlled M7 background maintenance. Network collection remains a leased
//! job so no server process opens a shared SQLite file from a worker. Locally
//! executable derived work (quality and retrieval sync) runs in-process and
//! updates `data_refresh_state` only after it actually succeeds.

use std::{env, time::Duration};

use mpgs_storage::{DataRefreshStatus, EnqueueJob, Repository};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};

const DEFAULT_INTERVAL_SECS: u64 = 300;
const DEFAULT_CATALOG_SYNC_INTERVAL_SECS: u64 = 15 * 60;
const DEFAULT_CANDIDATE_COLLECTION_INTERVAL_SECS: u64 = 6 * 60 * 60;
const DEFAULT_ENRICHMENT_INTERVAL_SECS: u64 = 5 * 60;
const TASK_INTERVAL_MIN_SECS: u64 = 60;
const TASK_INTERVAL_MAX_SECS: u64 = 86_400;

#[derive(Clone, Copy)]
struct TaskIntervals {
    catalog_sync_secs: u64,
    candidate_collection_secs: u64,
    enrichment_secs: u64,
}

impl TaskIntervals {
    fn from_env() -> Self {
        Self {
            catalog_sync_secs: configured_interval(
                "MPGS_CATALOG_SYNC_INTERVAL_SECS",
                DEFAULT_CATALOG_SYNC_INTERVAL_SECS,
                TASK_INTERVAL_MIN_SECS,
            ),
            candidate_collection_secs: configured_interval(
                "MPGS_CANDIDATE_COLLECTION_INTERVAL_SECS",
                DEFAULT_CANDIDATE_COLLECTION_INTERVAL_SECS,
                TASK_INTERVAL_MIN_SECS,
            ),
            enrichment_secs: configured_interval(
                "MPGS_ENRICHMENT_INTERVAL_SECS",
                DEFAULT_ENRICHMENT_INTERVAL_SECS,
                TASK_INTERVAL_MIN_SECS,
            ),
        }
    }
}

impl Default for TaskIntervals {
    fn default() -> Self {
        Self {
            catalog_sync_secs: DEFAULT_CATALOG_SYNC_INTERVAL_SECS,
            candidate_collection_secs: DEFAULT_CANDIDATE_COLLECTION_INTERVAL_SECS,
            enrichment_secs: DEFAULT_ENRICHMENT_INTERVAL_SECS,
        }
    }
}

pub fn spawn(repo: Option<Repository>) {
    let Some(repo) = repo else {
        return;
    };
    let interval_secs =
        configured_interval("MPGS_SCHEDULER_INTERVAL_SECS", DEFAULT_INTERVAL_SECS, 30);
    let task_intervals = TaskIntervals::from_env();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let run_repo = repo.clone();
            match tokio::task::spawn_blocking(move || {
                run_once(&run_repo, interval_secs, task_intervals)
            })
            .await
            {
                Ok(Ok(())) => info!("background data maintenance completed"),
                Ok(Err(error)) => warn!(error = %error, "background data maintenance failed"),
                Err(error) => warn!(error = %error, "background data maintenance task panicked"),
            }
        }
    });
}

fn configured_interval(name: &str, default: u64, minimum: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| (minimum..=TASK_INTERVAL_MAX_SECS).contains(value))
        .unwrap_or(default)
}

fn run_once(
    repo: &Repository,
    interval_secs: u64,
    task_intervals: TaskIntervals,
) -> mpgs_storage::StorageResult<()> {
    run_once_with_schedule(
        repo,
        interval_secs,
        steam_web_api_key_configured(),
        task_intervals,
    )
}

#[cfg(test)]
fn run_once_with_catalog_sync(
    repo: &Repository,
    interval_secs: u64,
    catalog_sync_enabled: bool,
) -> mpgs_storage::StorageResult<()> {
    run_once_with_schedule(
        repo,
        interval_secs,
        catalog_sync_enabled,
        TaskIntervals::default(),
    )
}

fn run_once_with_schedule(
    repo: &Repository,
    interval_secs: u64,
    catalog_sync_enabled: bool,
    task_intervals: TaskIntervals,
) -> mpgs_storage::StorageResult<()> {
    let now_ms = repo.database().now_ms();
    let interval_ms = (interval_secs as i64).saturating_mul(1_000);
    let next_run_at_ms = now_ms.saturating_add(interval_ms);
    let coverage = repo.m3_catalog_coverage()?;
    let coverage_ratio = if coverage.normalized_multiplayer_candidates > 0 {
        Some(
            (coverage.recommendation_ready_profiles as f64
                / coverage.normalized_multiplayer_candidates as f64)
                .clamp(0.0, 1.0),
        )
    } else {
        Some(0.0)
    };
    let previous_status = repo.data_refresh_status()?;

    // Lease-backed collection work is deliberately queued rather than run in a
    // database transaction. A co-located worker leases these tasks and writes
    // source snapshots independently, so Steam failure cannot clear a good
    // catalog snapshot held by the API process. Each task has an independent
    // cadence and only one active scheduled job, which prevents a slow catalog
    // sync from accumulating ahead of candidate discovery or enrichment.
    for (task_name, task_type, task_interval_secs, enabled) in [
        (
            "catalog_sync",
            "sync_catalog",
            task_intervals.catalog_sync_secs,
            catalog_sync_enabled,
        ),
        (
            "candidate_collection",
            "collect_candidates",
            task_intervals.candidate_collection_secs,
            true,
        ),
        (
            "enrichment",
            "enrich_catalog",
            task_intervals.enrichment_secs,
            true,
        ),
    ] {
        let previous = previous_status
            .iter()
            .find(|status| status.task_name == task_name);
        let task_interval_ms = (task_interval_secs as i64).saturating_mul(1_000);
        let task_next_run_at_ms = now_ms.saturating_add(task_interval_ms);
        if !enabled {
            update_scheduled_status(
                repo,
                task_name,
                previous,
                task_next_run_at_ms,
                Some("auth"),
                coverage_ratio,
            )?;
            continue;
        }
        let due = previous
            .and_then(|status| status.next_run_at_ms)
            .is_none_or(|next_run_at_ms| next_run_at_ms <= now_ms);
        if !due || repo.has_active_job("steam", task_type, "scheduled")? {
            continue;
        }
        let slot = now_ms / task_interval_ms.max(1);
        let _ = repo.enqueue_job(&EnqueueJob {
            source: "steam".to_owned(),
            task_type: task_type.to_owned(),
            entity_key: "scheduled".to_owned(),
            priority: 50,
            due_at_ms: now_ms,
            idempotency_key: format!("m7-scheduler:{task_name}:{slot}"),
            payload_json: None,
            max_attempts: 3,
        })?;
        update_scheduled_status(
            repo,
            task_name,
            previous,
            task_next_run_at_ms,
            None,
            coverage_ratio,
        )?;
    }

    let quality_previous = previous_status
        .iter()
        .find(|status| status.task_name == "quality_check");
    match repo.run_quality_checks() {
        Ok(_) => repo.update_data_refresh_status(
            "quality_check",
            Some(now_ms),
            Some(next_run_at_ms),
            None,
            None,
            coverage_ratio,
        )?,
        Err(error) => {
            repo.update_data_refresh_status(
                "quality_check",
                quality_previous.and_then(|status| status.last_success_at_ms),
                Some(next_run_at_ms),
                Some("quality_check_failed"),
                quality_previous.and_then(|status| status.cursor_value.as_deref()),
                coverage_ratio,
            )?;
            return Err(error);
        }
    }

    let retrieval_previous = previous_status
        .iter()
        .find(|status| status.task_name == "retrieval_sync");
    match repo.sync_retrieval_from_catalog(2_000, 0, true) {
        Ok(stats) => repo.update_data_refresh_status(
            "retrieval_sync",
            Some(now_ms),
            Some(next_run_at_ms),
            None,
            Some(&stats.apps_scanned.to_string()),
            coverage_ratio,
        )?,
        Err(error) => {
            repo.update_data_refresh_status(
                "retrieval_sync",
                retrieval_previous.and_then(|status| status.last_success_at_ms),
                Some(next_run_at_ms),
                Some("retrieval_sync_failed"),
                retrieval_previous.and_then(|status| status.cursor_value.as_deref()),
                coverage_ratio,
            )?;
            return Err(error);
        }
    }
    Ok(())
}

fn steam_web_api_key_configured() -> bool {
    env::var("MPGS_STEAM_WEB_API_KEY")
        .ok()
        .map(|value| value.trim().to_owned())
        .is_some_and(|value| {
            value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

fn update_scheduled_status(
    repo: &Repository,
    task_name: &str,
    previous: Option<&DataRefreshStatus>,
    next_run_at_ms: i64,
    error_category: Option<&str>,
    coverage_ratio: Option<f64>,
) -> mpgs_storage::StorageResult<()> {
    repo.update_data_refresh_status(
        task_name,
        previous.and_then(|status| status.last_success_at_ms),
        Some(next_run_at_ms),
        error_category,
        previous.and_then(|status| status.cursor_value.as_deref()),
        coverage_ratio,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use mpgs_storage::{Database, FakeClock, Repository};

    #[test]
    fn records_only_completed_derived_work_as_success() {
        let db = Database::open_in_memory().unwrap();
        let repo = Repository::new(db);
        repo.migrate().unwrap();
        repo.ensure_runtime_defaults().unwrap();
        repo.seed_demo_if_empty().unwrap();
        run_once_with_catalog_sync(&repo, DEFAULT_INTERVAL_SECS, false).unwrap();
        let status = repo.data_refresh_status().unwrap();
        let quality = status
            .iter()
            .find(|item| item.task_name == "quality_check")
            .unwrap();
        assert!(quality.last_success_at_ms.is_some());
        let collection = status
            .iter()
            .find(|item| item.task_name == "candidate_collection")
            .unwrap();
        assert!(collection.last_success_at_ms.is_none());
        assert!(collection.last_error_category.is_none());
        let catalog = status
            .iter()
            .find(|item| item.task_name == "catalog_sync")
            .unwrap();
        assert_eq!(catalog.last_error_category.as_deref(), Some("auth"));
    }

    #[test]
    fn keeps_collection_success_when_scheduling_the_next_job() {
        let db = Database::open_in_memory().unwrap();
        let repo = Repository::new(db);
        repo.migrate().unwrap();
        repo.ensure_runtime_defaults().unwrap();
        repo.seed_demo_if_empty().unwrap();
        repo.update_data_refresh_status(
            "candidate_collection",
            Some(123),
            None,
            None,
            Some("cursor"),
            Some(0.5),
        )
        .unwrap();

        run_once_with_catalog_sync(&repo, DEFAULT_INTERVAL_SECS, false).unwrap();

        let status = repo.data_refresh_status().unwrap();
        let collection = status
            .iter()
            .find(|item| item.task_name == "candidate_collection")
            .unwrap();
        assert_eq!(collection.last_success_at_ms, Some(123));
        assert_eq!(collection.cursor_value.as_deref(), Some("cursor"));
        assert!(collection.next_run_at_ms.is_some());
        assert!(collection.last_error_category.is_none());
    }

    #[test]
    fn keeps_one_active_job_per_collection_task_and_respects_task_cadence() {
        let clock = Arc::new(FakeClock::new(0));
        let db = Database::open_in_memory_with_clock(clock.clone()).unwrap();
        let repo = Repository::new(db);
        repo.migrate().unwrap();
        repo.ensure_runtime_defaults().unwrap();
        repo.seed_demo_if_empty().unwrap();
        let task_intervals = TaskIntervals {
            catalog_sync_secs: 60,
            candidate_collection_secs: 600,
            enrichment_secs: 60,
        };

        run_once_with_schedule(&repo, 30, false, task_intervals).unwrap();
        assert!(
            repo.has_active_job("steam", "collect_candidates", "scheduled")
                .unwrap()
        );
        assert!(
            repo.has_active_job("steam", "enrich_catalog", "scheduled")
                .unwrap()
        );

        clock.advance_ms(60_000);
        run_once_with_schedule(&repo, 30, false, task_intervals).unwrap();
        let scheduled_jobs: i64 = repo
            .database()
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM jobs
                     WHERE source = 'steam' AND entity_key = 'scheduled'",
                    [],
                    |row| row.get(0),
                )
                .map_err(Into::into)
            })
            .unwrap();
        assert_eq!(
            scheduled_jobs, 2,
            "due tasks must not accumulate while active"
        );

        let jobs = repo
            .lease_jobs("test-worker", 10, 60_000, Some("steam"))
            .unwrap();
        assert_eq!(jobs.len(), 2);
        for job in jobs {
            repo.complete_job(job.job_id, "test-worker", &format!("done-{}", job.job_id))
                .unwrap();
        }

        run_once_with_schedule(&repo, 30, false, task_intervals).unwrap();
        assert!(
            repo.has_active_job("steam", "enrich_catalog", "scheduled")
                .unwrap()
        );
        assert!(
            !repo
                .has_active_job("steam", "collect_candidates", "scheduled")
                .unwrap(),
            "candidate collection waits for its longer interval"
        );
    }
}

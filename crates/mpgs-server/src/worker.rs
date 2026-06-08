use crate::admin::AdminTaskKind;
use crate::db::{self, ClaimedTask, TaskFailureInput};
use anyhow::{anyhow, Context, Result};
use mpgs_core::analysis::build_rule_report;
use mpgs_core::steam_mapping::{build_discovered_game_card, SteamAppListItem, SteamGameSnapshot};
use sqlx_postgres::PgPool;
use std::future::Future;
use std::pin::Pin;

pub trait SteamSnapshotSource {
    fn fetch_snapshot<'a>(
        &'a self,
        appid: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Option<SteamGameSnapshot>>> + Send + 'a>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerTickOutcome {
    Idle,
    Completed { task_id: i64, appid: u32 },
    Failed { task_id: i64, reason: String },
}

pub async fn run_one_worker_tick(
    pool: &PgPool,
    worker_id: &str,
    snapshot_source: &impl SteamSnapshotSource,
) -> Result<WorkerTickOutcome> {
    let Some(claimed) = db::claim_next_task(pool, worker_id).await? else {
        return Ok(WorkerTickOutcome::Idle);
    };

    process_claimed_task(pool, claimed, snapshot_source).await
}

async fn process_claimed_task(
    pool: &PgPool,
    claimed: ClaimedTask,
    snapshot_source: &impl SteamSnapshotSource,
) -> Result<WorkerTickOutcome> {
    if claimed.task.task_type != AdminTaskKind::ManualAppidDiscovery.as_str() {
        let reason = format!("Unsupported task type {}", claimed.task.task_type);
        fail_claimed_task(pool, &claimed, "task_dispatch", None, false, &reason).await?;
        return Ok(WorkerTickOutcome::Failed {
            task_id: claimed.task.id,
            reason,
        });
    }

    let Some(appid) = claimed.task.target_appid else {
        let reason = "Manual AppID discovery task is missing target_appid.".to_string();
        fail_claimed_task(pool, &claimed, "task_validation", None, false, &reason).await?;
        return Ok(WorkerTickOutcome::Failed {
            task_id: claimed.task.id,
            reason,
        });
    };

    match process_manual_appid(pool, &claimed, appid, snapshot_source).await {
        Ok(()) => Ok(WorkerTickOutcome::Completed {
            task_id: claimed.task.id,
            appid,
        }),
        Err(error) => {
            let reason = error.to_string();
            fail_claimed_task(
                pool,
                &claimed,
                "manual_appid_discovery",
                None,
                true,
                &reason,
            )
            .await?;
            Ok(WorkerTickOutcome::Failed {
                task_id: claimed.task.id,
                reason,
            })
        }
    }
}

async fn process_manual_appid(
    pool: &PgPool,
    claimed: &ClaimedTask,
    appid: u32,
    snapshot_source: &impl SteamSnapshotSource,
) -> Result<()> {
    let snapshot = snapshot_source
        .fetch_snapshot(appid)
        .await
        .with_context(|| format!("fetch Steam snapshot for appid {appid}"))?
        .ok_or_else(|| {
            anyhow!("Steam appdetails did not return importable data for appid {appid}")
        })?;
    let app = SteamAppListItem {
        appid,
        name: format!("Steam App {appid}"),
    };
    let today = mpgs_core::recommendation::today_iso_utc();
    let game = build_discovered_game_card(&app, snapshot, &today)
        .ok_or_else(|| anyhow!("Steam snapshot for appid {appid} did not pass discovery rules"))?;
    let generated_at = format!("{}T00:00:00Z", mpgs_core::recommendation::today_iso_utc());
    let report = build_rule_report(&game, generated_at)?;
    let (review_status, visibility) = review_state_for_score(game.recommendation_score);

    db::upsert_public_catalog_game(pool, &game, &report, review_status, visibility)
        .await
        .with_context(|| format!("upsert public catalog candidate for appid {appid}"))?;
    db::complete_task_run(
        pool,
        claimed.run_id,
        Some("manual AppID discovery imported a public catalog candidate"),
    )
    .await?
    .ok_or_else(|| anyhow!("claimed task run was no longer running"))?;

    Ok(())
}

fn review_state_for_score(recommendation_score: f64) -> (&'static str, &'static str) {
    if recommendation_score >= 82.0 {
        ("accepted", "public")
    } else if recommendation_score >= 55.0 {
        ("needs_review", "hidden")
    } else {
        ("rejected", "hidden")
    }
}

async fn fail_claimed_task(
    pool: &PgPool,
    claimed: &ClaimedTask,
    stage: &'static str,
    provider: Option<&'static str>,
    retryable: bool,
    reason: &str,
) -> Result<()> {
    db::fail_task_run(
        pool,
        claimed.run_id,
        TaskFailureInput {
            stage,
            target: claimed.task.target.as_deref(),
            provider,
            retryable,
            reason,
        },
    )
    .await?
    .ok_or_else(|| anyhow!("claimed task run was no longer running"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_score_candidates_become_public_and_medium_score_candidates_need_review() {
        assert_eq!(review_state_for_score(82.0), ("accepted", "public"));
        assert_eq!(review_state_for_score(70.0), ("needs_review", "hidden"));
        assert_eq!(review_state_for_score(54.9), ("rejected", "hidden"));
    }
}

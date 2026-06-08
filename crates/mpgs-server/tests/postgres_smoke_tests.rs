use mpgs_core::models::PublicCatalogStatus;
use mpgs_core::models::{ReviewSnippet, StoreReleaseState};
use mpgs_core::recommendation::DemoStatus;
use mpgs_core::steam_mapping::SteamGameSnapshot;
use mpgs_server::admin::AdminReviewAction;
use mpgs_server::admin::AdminTaskKind;
use mpgs_server::{
    build_router_with_state, db, worker, AppState, ConfigHealth, DatabaseHealth, ServiceInfoConfig,
};

use anyhow::Result;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::fs;
use std::future::Future;
use std::pin::Pin;
use tower::ServiceExt;

fn test_config() -> ServiceInfoConfig {
    ServiceInfoConfig {
        service_instance_id: "018fb770-8998-7699-a6e4-b7b59f2f9c01".to_string(),
        service_name: "MPGS Postgres Smoke Test Service".to_string(),
        service_version: "0.1.0".to_string(),
    }
}

struct StaticSnapshotSource {
    snapshot: Option<SteamGameSnapshot>,
}

impl worker::SteamSnapshotSource for StaticSnapshotSource {
    fn fetch_snapshot<'a>(
        &'a self,
        _appid: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Option<SteamGameSnapshot>>> + Send + 'a>> {
        Box::pin(async { Ok(self.snapshot.clone()) })
    }
}

fn importable_snapshot(name: &str) -> SteamGameSnapshot {
    SteamGameSnapshot {
        name: Some(name.to_string()),
        short_description: Some("A polished co-op action game for small groups.".to_string()),
        release_date: Some("2026-04-20".to_string()),
        release_date_text: Some("Apr 20, 2026".to_string()),
        release_state: Some(StoreReleaseState::Released),
        demo_status: DemoStatus::ReleasedWithDemo,
        supported_languages: Some(vec!["English".to_string()]),
        is_adult_content: Some(false),
        is_free: Some(false),
        price_text: Some("$19.99".to_string()),
        discount_percent: Some(10),
        positive_review_pct: Some(96.0),
        total_reviews: Some(12_000),
        current_players: Some(8_000),
        capsule_url: Some("https://cdn.example.test/header.jpg".to_string()),
        store_screenshot_urls: Vec::new(),
        tags: vec!["Co-op".to_string(), "Action".to_string()],
        multiplayer_modes: vec!["Online Co-op".to_string(), "Multi-player".to_string()],
        review_snippets: vec![ReviewSnippet {
            voted_up: true,
            review: "Great with friends and easy to teach.".to_string(),
            playtime_hours: Some(12.0),
        }],
    }
}

#[tokio::test]
async fn migrates_empty_postgres_database_when_test_database_is_configured() {
    let Ok(database_url) = std::env::var("MPGS_TEST_DATABASE_URL") else {
        return;
    };

    let pool = db::connect_and_migrate(&database_url)
        .await
        .expect("connect to Postgres and run migrations");
    let status = db::public_catalog_status(&pool)
        .await
        .expect("read public catalog status");

    assert_eq!(status, PublicCatalogStatus::Empty);
}

#[tokio::test]
async fn healthz_is_ok_after_postgres_migration_and_active_config_are_ready() {
    let Ok(database_url) = std::env::var("MPGS_TEST_DATABASE_URL") else {
        return;
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let active_dir = temp_dir.path().join("active");
    fs::create_dir(&active_dir).unwrap();
    let service_path = active_dir.join("service.toml");
    let secrets_path = active_dir.join("secrets.toml");
    fs::write(
        &service_path,
        r#"
bind_addr = "127.0.0.1:4310"

[service_identity]
instance_id = "018fb770-8998-7699-a6e4-b7b59f2f9c01"
name = "MPGS Postgres Smoke Test Service"
version = "0.1.0"
"#,
    )
    .unwrap();
    fs::write(
        &secrets_path,
        format!(
            r#"
[database]
url = "{}"
"#,
            database_url
        ),
    )
    .unwrap();

    let pool = db::connect_and_migrate(&database_url)
        .await
        .expect("connect to Postgres and run migrations");
    let app = build_router_with_state(AppState::new_with_config_health(
        test_config().service_info(),
        DatabaseHealth::Pool(pool),
        ConfigHealth::active_files(service_path, secrets_path),
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn records_pending_config_state_in_ops_schema() {
    let Ok(database_url) = std::env::var("MPGS_TEST_DATABASE_URL") else {
        return;
    };

    let pool = db::connect_and_migrate(&database_url)
        .await
        .expect("connect to Postgres and run migrations");

    db::record_active_config_startup(&pool, "sha256:active-test")
        .await
        .expect("record active config startup state");
    db::mark_pending_config(&pool, "sha256:pending-test")
        .await
        .expect("record pending config state");

    let state = db::service_config_state(&pool)
        .await
        .expect("read service config state");

    assert_eq!(
        state.active_config_version.as_deref(),
        Some("sha256:active-test")
    );
    assert_eq!(
        state.pending_config_version.as_deref(),
        Some("sha256:pending-test")
    );
    assert!(state.restart_required);
    assert_eq!(state.last_startup_status, "ok");
}

#[tokio::test]
async fn records_audit_events_in_ops_schema() {
    let Ok(database_url) = std::env::var("MPGS_TEST_DATABASE_URL") else {
        return;
    };

    let pool = db::connect_and_migrate(&database_url)
        .await
        .expect("connect to Postgres and run migrations");

    db::record_audit_event(&pool, "admin.restart.requested", "admin", "success")
        .await
        .expect("record audit event");

    let event = db::latest_audit_event(&pool)
        .await
        .expect("read latest audit event")
        .expect("audit event should exist");

    assert_eq!(event.event_type, "admin.restart.requested");
    assert_eq!(event.actor, "admin");
    assert_eq!(event.outcome, "success");
}

#[tokio::test]
async fn public_review_action_advances_public_catalog_revision() {
    let Ok(database_url) = std::env::var("MPGS_TEST_DATABASE_URL") else {
        return;
    };

    let pool = db::connect_and_migrate(&database_url)
        .await
        .expect("connect to Postgres and run migrations");

    sqlx_core::query::query::<sqlx_postgres::Postgres>(
        r#"
        INSERT INTO public_catalog.games (
            appid,
            name,
            review_status,
            visibility,
            recommendation_score
        )
        VALUES ($1, 'Review Smoke Candidate', 'needs_review', 'hidden', 87.0)
        ON CONFLICT (appid) DO UPDATE
        SET review_status = 'needs_review',
            visibility = 'hidden',
            recommendation_score = 87.0,
            review_note = NULL,
            updated_at = now()
        "#,
    )
    .bind(901_337_i32)
    .execute(&pool)
    .await
    .expect("insert review candidate");

    let before = db::public_catalog_revision(&pool)
        .await
        .expect("read revision before review action");
    let reviewed = db::apply_admin_review_action(
        &pool,
        901_337,
        AdminReviewAction::AcceptPublic,
        Some("smoke approved"),
    )
    .await
    .expect("apply public review action")
    .expect("review candidate should be updated");
    let after = db::public_catalog_revision(&pool)
        .await
        .expect("read revision after review action");

    assert_eq!(reviewed.review_status, "accepted");
    assert_eq!(reviewed.visibility, "public");
    assert_eq!(reviewed.review_note.as_deref(), Some("smoke approved"));
    assert_eq!(after, before + 1);
}

#[tokio::test]
async fn admin_task_controls_use_ops_schema_in_postgres() {
    let Ok(database_url) = std::env::var("MPGS_TEST_DATABASE_URL") else {
        return;
    };

    let pool = db::connect_and_migrate(&database_url)
        .await
        .expect("connect to Postgres and run migrations");

    let task = db::create_admin_task(&pool, AdminTaskKind::ManualAppidDiscovery, Some(730))
        .await
        .expect("create manual appid task");

    sqlx_core::query::query::<sqlx_postgres::Postgres>(
        r#"
        INSERT INTO ops.task_failures (
            task_id,
            stage,
            target,
            provider,
            retryable,
            attempt,
            reason
        )
        VALUES ($1, 'steam_lookup', 'appid:730', 'steam', TRUE, 1, 'Steam lookup timed out.')
        "#,
    )
    .bind(task.id)
    .execute(&pool)
    .await
    .expect("insert sanitized task failure");

    let state = db::admin_task_control_state(&pool)
        .await
        .expect("read admin task control state");

    assert!(state
        .recent_tasks
        .iter()
        .any(|item| item.id == task.id && item.target_appid == Some(730)));
    assert!(state.failure_summary.open_failure_count >= 1);
    assert!(state.failure_summary.retryable_failure_count >= 1);
    assert!(state.failures.iter().any(|failure| {
        failure.task_id == Some(task.id)
            && failure.stage == "steam_lookup"
            && failure.reason == "Steam lookup timed out."
    }));
}

#[tokio::test]
async fn postgres_workers_claim_only_one_queued_task_at_a_time() {
    let Ok(database_url) = std::env::var("MPGS_TEST_DATABASE_URL") else {
        return;
    };

    let pool = db::connect_and_migrate(&database_url)
        .await
        .expect("connect to Postgres and run migrations");

    let first_task =
        db::create_admin_task(&pool, AdminTaskKind::ManualAppidDiscovery, Some(910_001))
            .await
            .expect("create first manual appid task");
    let second_task =
        db::create_admin_task(&pool, AdminTaskKind::ManualAppidDiscovery, Some(910_002))
            .await
            .expect("create second manual appid task");

    sqlx_core::query::query::<sqlx_postgres::Postgres>(
        r#"
        UPDATE ops.tasks
        SET priority = 1
        WHERE id IN ($1, $2)
        "#,
    )
    .bind(first_task.id)
    .bind(second_task.id)
    .execute(&pool)
    .await
    .expect("prioritize smoke tasks");

    let first_claim = db::claim_next_task(&pool, "smoke-worker-a")
        .await
        .expect("claim first queued task")
        .expect("first queued task should be claimed");
    let second_claim = db::claim_next_task(&pool, "smoke-worker-b")
        .await
        .expect("claim second queued task")
        .expect("second queued task should be claimed");

    assert_ne!(first_claim.task.id, second_claim.task.id);
    assert_eq!(first_claim.task.status, "running");
    assert_eq!(second_claim.task.status, "running");
    assert!(first_claim.run_id > 0);
    assert!(second_claim.run_id > 0);

    let claimed_task_count = sqlx_core::query_scalar::query_scalar::<sqlx_postgres::Postgres, i64>(
        r#"
        SELECT COUNT(*)
        FROM ops.tasks
        WHERE id IN ($1, $2)
          AND status = 'running'
          AND claimed_at IS NOT NULL
        "#,
    )
    .bind(first_task.id)
    .bind(second_task.id)
    .fetch_one(&pool)
    .await
    .expect("count claimed smoke tasks");
    let running_run_count = sqlx_core::query_scalar::query_scalar::<sqlx_postgres::Postgres, i64>(
        r#"
        SELECT COUNT(*)
        FROM ops.task_runs
        WHERE task_id IN ($1, $2)
          AND status = 'running'
        "#,
    )
    .bind(first_task.id)
    .bind(second_task.id)
    .fetch_one(&pool)
    .await
    .expect("count running smoke task runs");

    assert_eq!(claimed_task_count, 2);
    assert_eq!(running_run_count, 2);
}

#[tokio::test]
async fn postgres_task_runs_complete_and_fail_with_sanitized_failure_records() {
    let Ok(database_url) = std::env::var("MPGS_TEST_DATABASE_URL") else {
        return;
    };

    let pool = db::connect_and_migrate(&database_url)
        .await
        .expect("connect to Postgres and run migrations");

    let completing_task =
        db::create_admin_task(&pool, AdminTaskKind::ManualAppidDiscovery, Some(910_101))
            .await
            .expect("create task to complete");
    let failing_task =
        db::create_admin_task(&pool, AdminTaskKind::ManualAppidDiscovery, Some(910_102))
            .await
            .expect("create task to fail");

    sqlx_core::query::query::<sqlx_postgres::Postgres>(
        r#"
        UPDATE ops.tasks
        SET priority = 0
        WHERE id IN ($1, $2)
        "#,
    )
    .bind(completing_task.id)
    .bind(failing_task.id)
    .execute(&pool)
    .await
    .expect("prioritize lifecycle smoke tasks");

    let completing_claim = db::claim_next_task(&pool, "smoke-worker-complete")
        .await
        .expect("claim completing task")
        .expect("completing task should be claimed");
    let completed = db::complete_task_run(
        &pool,
        completing_claim.run_id,
        Some("manual AppID discovery completed"),
    )
    .await
    .expect("complete task run")
    .expect("running task run should complete");

    assert_eq!(completed.task.id, completing_claim.task.id);
    assert_eq!(completed.task.status, "completed");
    assert_eq!(completed.run_status, "completed");

    let failing_claim = db::claim_next_task(&pool, "smoke-worker-fail")
        .await
        .expect("claim failing task")
        .expect("failing task should be claimed");
    let failed = db::fail_task_run(
        &pool,
        failing_claim.run_id,
        db::TaskFailureInput {
            stage: "steam_lookup",
            target: None,
            provider: Some("steam"),
            retryable: true,
            reason: "Steam lookup timed out.",
        },
    )
    .await
    .expect("fail task run")
    .expect("running task run should fail");

    assert_eq!(failed.task.id, failing_claim.task.id);
    assert_eq!(failed.task.status, "failed");
    assert_eq!(failed.run_status, "failed");

    let failure = db::admin_task_control_state(&pool)
        .await
        .expect("read task failure summary")
        .failures
        .into_iter()
        .find(|failure| failure.task_id == Some(failing_claim.task.id))
        .expect("failed task should have a sanitized failure record");

    assert_eq!(failure.stage, "steam_lookup");
    assert_eq!(failure.target.as_deref(), Some("appid:910102"));
    assert_eq!(failure.provider.as_deref(), Some("steam"));
    assert!(failure.retryable);
    assert_eq!(failure.reason, "Steam lookup timed out.");
}

#[tokio::test]
async fn manual_appid_worker_imports_public_catalog_candidate_from_claimed_task() {
    let Ok(database_url) = std::env::var("MPGS_TEST_DATABASE_URL") else {
        return;
    };

    let pool = db::connect_and_migrate(&database_url)
        .await
        .expect("connect to Postgres and run migrations");

    let task = db::create_admin_task(&pool, AdminTaskKind::ManualAppidDiscovery, Some(910_201))
        .await
        .expect("create manual appid task");
    let outcome = worker::run_one_worker_tick(
        &pool,
        "smoke-worker-manual-appid",
        &StaticSnapshotSource {
            snapshot: Some(importable_snapshot("Smoke Co-op Harbor")),
        },
    )
    .await
    .expect("run one manual appid worker tick");

    assert_eq!(
        outcome,
        worker::WorkerTickOutcome::Completed {
            task_id: task.id,
            appid: 910_201,
        }
    );

    let detail = db::public_game_detail(&pool, 910_201)
        .await
        .expect("read imported public game")
        .expect("high-confidence manual AppID task should publish public game");
    let analysis = db::public_game_analysis(&pool, 910_201)
        .await
        .expect("read imported public game analysis")
        .expect("manual AppID task should store rule analysis");
    let task_state = db::admin_task_control_state(&pool)
        .await
        .expect("read task state after worker tick");

    assert_eq!(detail.game.name, "Smoke Co-op Harbor");
    assert!(analysis.report["overview"]
        .as_str()
        .unwrap_or("")
        .contains("综合推荐"));
    assert!(task_state
        .recent_tasks
        .iter()
        .any(|item| item.id == task.id && item.status == "completed"));
}

use mpgs_core::models::PublicCatalogStatus;
use mpgs_server::{
    build_router_with_state, db, AppState, ConfigHealth, DatabaseHealth, ServiceInfoConfig,
};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::fs;
use tower::ServiceExt;

fn test_config() -> ServiceInfoConfig {
    ServiceInfoConfig {
        service_instance_id: "018fb770-8998-7699-a6e4-b7b59f2f9c01".to_string(),
        service_name: "MPGS Postgres Smoke Test Service".to_string(),
        service_version: "0.1.0".to_string(),
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

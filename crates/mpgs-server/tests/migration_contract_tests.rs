use std::fs;
use std::path::Path;

use mpgs_server::db;

#[test]
fn initial_migration_creates_public_catalog_and_ops_boundaries() {
    let migration_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("migrations")
        .join("0001_public_catalog_ops.sql");
    let sql = fs::read_to_string(&migration_path).expect("initial migration should exist");

    assert!(sql.contains("CREATE SCHEMA IF NOT EXISTS public_catalog"));
    assert!(sql.contains("CREATE SCHEMA IF NOT EXISTS ops"));
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS public_catalog.games"));
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS public_catalog.game_analysis"));
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS public_catalog.public_catalog_state"));
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS ops.service_config_state"));
    assert!(!sql.to_lowercase().contains("steam_key"));
    assert!(!sql.to_lowercase().contains("llm_key"));
    assert!(!sql.to_lowercase().contains("admin_token"));
}

#[test]
fn initial_migration_is_embedded_for_binary_startup() {
    let migrator = db::migrator();
    let migrations: Vec<_> = migrator.iter().collect();

    assert_eq!(migrations.len(), 1);
    assert_eq!(migrations[0].version, 1);
    assert_eq!(migrations[0].description, "public_catalog_ops");
    assert!(migrations[0]
        .sql
        .contains("CREATE SCHEMA IF NOT EXISTS public_catalog"));
}

#[test]
fn database_health_checks_the_sqlx_migration_record() {
    let source_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("db.rs");
    let source = fs::read_to_string(&source_path).expect("db source should exist");

    assert!(source.contains("FROM _sqlx_migrations"));
    assert!(source.contains("description = 'public_catalog_ops'"));
    assert!(source.contains("success = TRUE"));
}

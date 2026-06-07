use mpgs_core::models::PublicCatalogStatus;
use mpgs_server::db;

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

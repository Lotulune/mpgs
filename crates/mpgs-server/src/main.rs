use anyhow::Result;
use mpgs_server::{build_router_with_state, db, AppState, DatabaseHealth, ServerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|arg| arg == "--export-openapi") {
        println!(
            "{}",
            serde_json::to_string_pretty(&mpgs_server::build_openapi())?
        );
        return Ok(());
    }

    let config = ServerConfig::from_env()?;
    let pool = db::connect_and_migrate(&config.database_url).await?;
    let public_catalog_status = db::public_catalog_status(&pool).await?;
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let app = build_router_with_state(AppState::new_with_config_health(
        config
            .service_info
            .service_info_with_catalog_status(public_catalog_status),
        DatabaseHealth::Pool(pool),
        config.config_health,
    ));

    axum::serve(listener, app).await?;
    Ok(())
}

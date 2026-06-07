use anyhow::Result;
use mpgs_server::{
    build_router_with_state, db, AppState, AuditSink, DatabaseHealth, StartupConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|arg| arg == "--export-openapi") {
        println!(
            "{}",
            serde_json::to_string_pretty(&mpgs_server::build_openapi())?
        );
        return Ok(());
    }

    let startup_config = StartupConfig::from_env()?;
    let (bind_addr, app) = match startup_config {
        StartupConfig::Ready(config) => {
            let pool = db::connect_and_migrate(&config.database_url).await?;
            let public_catalog_status = db::public_catalog_status(&pool).await?;
            let mut app_state = AppState::new_with_config_health(
                config
                    .service_info
                    .service_info_with_catalog_status(public_catalog_status),
                DatabaseHealth::Pool(pool.clone()),
                config.config_health,
            )
            .with_audit_sink(AuditSink::Pool(pool.clone()));
            if let Some(admin_auth) = config.admin_auth {
                app_state = app_state.with_admin_auth(admin_auth);
            }
            if let Some(setup_access) = config.setup_access {
                app_state = app_state.with_setup_config(setup_access);
            }
            app_state = app_state.with_public_cors(config.public_cors);
            if let Some(config_file_manager) = config.config_file_manager {
                if let Ok(active_config_version) = config_file_manager.active_config_version() {
                    db::record_active_config_startup(&pool, &active_config_version).await?;
                }
                app_state = app_state.with_config_manager(config_file_manager);
            }
            (config.bind_addr, build_router_with_state(app_state))
        }
        StartupConfig::SafeMode {
            bind_addr,
            service_info,
            setup_access,
        } => {
            let mut app_state = AppState::safe_mode(service_info);
            if let Some(setup_access) = setup_access {
                app_state = app_state.with_setup_config(setup_access);
            }
            (bind_addr, build_router_with_state(app_state))
        }
    };
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    axum::serve(listener, app).await?;
    Ok(())
}

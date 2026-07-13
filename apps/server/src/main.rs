#![forbid(unsafe_code)]

mod api;

use std::{env, error::Error, net::SocketAddr};

use mpgs_storage::{Database, Repository};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::api::{AppState, build_router};

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();
    let state = build_state()?;
    let address: SocketAddr = env::var("MPGS_BIND_ADDR")
        .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned())
        .parse()?;
    let listener = TcpListener::bind(address).await?;
    info!(%address, storage = state.repo.is_some(), "mpgs server listening");
    axum::serve(listener, build_router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn build_state() -> Result<AppState, Box<dyn Error>> {
    let admin_token = env::var("MPGS_ADMIN_TOKEN").ok().filter(|s| !s.is_empty());
    let repo = match env::var("MPGS_DATABASE_PATH") {
        Ok(path) if !path.is_empty() => {
            let db = Database::open(&path)?;
            let repo = Repository::new(db);
            let version = repo.migrate()?;
            repo.ensure_runtime_defaults()?;
            repo.assert_ready()?;
            info!(%path, version, "database ready");
            Some(repo)
        }
        _ => {
            // In-memory DB for local public API demos without configuring a path.
            let db = Database::open_in_memory()?;
            let repo = Repository::new(db);
            repo.migrate()?;
            repo.ensure_runtime_defaults()?;
            info!("using in-memory database (set MPGS_DATABASE_PATH for persistence)");
            Some(repo)
        }
    };
    Ok(AppState { repo, admin_token })
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "failed to install Ctrl+C handler");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;

    fn test_app() -> axum::Router {
        let db = Database::open_in_memory().unwrap();
        let repo = Repository::new(db);
        repo.migrate().unwrap();
        repo.ensure_runtime_defaults().unwrap();
        build_router(AppState {
            repo: Some(repo),
            admin_token: Some("secret".into()),
        })
    }

    #[tokio::test]
    async fn public_feed_and_default_ranking() {
        let app = test_app();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/feeds/classic_legacy?limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get("etag").is_some());
        assert!(response.headers().get("x-request-id").is_some());
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let items = json["items"].as_array().unwrap();
        assert!(!items.is_empty());
        let ids: Vec<u64> = items.iter().filter_map(|i| i["app_id"].as_u64()).collect();
        // Coop classics should appear before CS2 under default prefs.
        let drg = ids.iter().position(|id| *id == 548430);
        let cs2 = ids.iter().position(|id| *id == 730);
        if let (Some(d), Some(c)) = (drg, cs2) {
            assert!(d < c, "DRG should rank above CS2: {ids:?}");
        }
    }

    #[tokio::test]
    async fn session_preferences_feedback_flow() {
        let app = test_app();
        let session = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/session/anonymous")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(session.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(session.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let token = json["access_token"].as_str().unwrap();

        let prefs = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/preferences")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(prefs.status(), StatusCode::OK);

        let feedback = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/feedback")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header("idempotency-key", "fb-1")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"app_id":548430,"type":"like"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(feedback.status(), StatusCode::CREATED);

        let again = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/feedback")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header("idempotency-key", "fb-1")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"app_id":548430,"type":"like"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(again.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn search_and_detail() {
        let app = test_app();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/search?q=Deep&limit=5")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(!json["items"].as_array().unwrap().is_empty());

        let detail = app
            .oneshot(
                Request::builder()
                    .uri("/v1/games/548430")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(detail.status(), StatusCode::OK);
    }
}

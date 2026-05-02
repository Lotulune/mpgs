use crate::commands;
use crate::db;
use crate::state::AppState;
use anyhow::Result;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::VecDeque;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiBatchRefreshRuntimeSnapshot {
    pub running: bool,
    pub concurrency: u8,
    pub pending_count: usize,
    pub active_count: usize,
    pub total_count: usize,
    pub processed_count: usize,
    pub updated_count: usize,
    pub failed_count: usize,
    pub last_error: Option<String>,
    pub last_error_appid: Option<u32>,
}

#[derive(Debug, Default)]
pub struct AiBatchRefreshRuntimeState {
    pub active: bool,
    concurrency: u8,
    pending_count: usize,
    active_count: usize,
    total_count: usize,
    processed_count: usize,
    updated_count: usize,
    failed_count: usize,
    last_error: Option<String>,
    last_error_appid: Option<u32>,
}

impl AiBatchRefreshRuntimeState {
    pub fn start(&mut self, total_count: usize, concurrency: u8) -> bool {
        if self.active {
            return false;
        }

        self.active = true;
        self.concurrency = concurrency;
        self.pending_count = total_count;
        self.active_count = 0;
        self.total_count = total_count;
        self.processed_count = 0;
        self.updated_count = 0;
        self.failed_count = 0;
        self.last_error = None;
        self.last_error_appid = None;
        true
    }

    pub fn mark_job_started(&mut self) {
        if self.pending_count > 0 {
            self.pending_count -= 1;
        }
        self.active_count += 1;
    }

    pub fn finish_job(&mut self, appid: u32, updated: bool, error: Option<String>) {
        if self.active_count > 0 {
            self.active_count -= 1;
        }
        self.processed_count += 1;
        if updated {
            self.updated_count += 1;
        }
        if let Some(error) = error {
            self.failed_count += 1;
            self.last_error = Some(error);
            self.last_error_appid = Some(appid);
        }
    }

    pub fn finish_batch(&mut self) {
        self.active = false;
        self.pending_count = 0;
        self.active_count = 0;
    }

    pub fn fail_batch(&mut self, error: String) {
        self.active = false;
        self.pending_count = 0;
        self.active_count = 0;
        self.last_error = Some(error);
    }

    pub fn snapshot(&self) -> AiBatchRefreshRuntimeSnapshot {
        AiBatchRefreshRuntimeSnapshot {
            running: self.active,
            concurrency: self.concurrency,
            pending_count: self.pending_count,
            active_count: self.active_count,
            total_count: self.total_count,
            processed_count: self.processed_count,
            updated_count: self.updated_count,
            failed_count: self.failed_count,
            last_error: self.last_error.clone(),
            last_error_appid: self.last_error_appid,
        }
    }
}

pub fn spawn_ai_batch_refresh_worker(app: AppHandle, appids: Vec<u32>, concurrency: u8) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run_ai_batch_refresh_worker(app.clone(), appids, concurrency).await {
            eprintln!("ai batch refresh worker failed: {error:#}");
            let _ = fail_worker(&app, error.to_string());
        }
    });
}

async fn run_ai_batch_refresh_worker(
    app: AppHandle,
    appids: Vec<u32>,
    concurrency: u8,
) -> Result<()> {
    let concurrency = usize::from(db::clamp_ai_batch_refresh_concurrency(concurrency));
    let mut pending = VecDeque::from(appids);
    let mut in_flight = FuturesUnordered::new();

    loop {
        while in_flight.len() < concurrency {
            let Some(appid) = pending.pop_front() else {
                break;
            };

            {
                let state = app.state::<AppState>();
                let mut runtime = state
                    .ai_batch_refresh
                    .lock()
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                runtime.mark_job_started();
            }

            let app_handle = app.clone();
            in_flight.push(async move {
                let state = app_handle.state::<AppState>();
                let result =
                    commands::generate_or_load_game_analysis(state.inner(), appid, true).await;
                (appid, result)
            });
        }

        let Some((appid, outcome)) = in_flight.next().await else {
            break;
        };

        let error_text = outcome.err().map(|error| error.to_string());
        let state = app.state::<AppState>();
        let mut runtime = state
            .ai_batch_refresh
            .lock()
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        runtime.finish_job(appid, error_text.is_none(), error_text);
    }

    let state = app.state::<AppState>();
    let mut runtime = state
        .ai_batch_refresh
        .lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    runtime.finish_batch();
    Ok(())
}

fn fail_worker(app: &AppHandle, error: String) -> Result<()> {
    let state = app.state::<AppState>();
    let mut runtime = state
        .ai_batch_refresh
        .lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    runtime.fail_batch(error);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::AiBatchRefreshRuntimeState;

    #[test]
    fn snapshot_tracks_batch_refresh_progress() {
        let mut runtime = AiBatchRefreshRuntimeState::default();
        assert!(runtime.start(4, 5));

        runtime.mark_job_started();
        runtime.mark_job_started();
        runtime.finish_job(101, true, None);
        runtime.finish_job(202, false, Some("timeout".to_string()));
        runtime.finish_batch();

        let snapshot = runtime.snapshot();
        assert!(!snapshot.running);
        assert_eq!(snapshot.concurrency, 5);
        assert_eq!(snapshot.total_count, 4);
        assert_eq!(snapshot.processed_count, 2);
        assert_eq!(snapshot.updated_count, 1);
        assert_eq!(snapshot.failed_count, 1);
        assert_eq!(snapshot.last_error.as_deref(), Some("timeout"));
        assert_eq!(snapshot.last_error_appid, Some(202));
    }
}

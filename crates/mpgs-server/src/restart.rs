use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use utoipa::ToSchema;

#[derive(Debug, Clone)]
pub struct RestartCoordinator {
    requested: Arc<AtomicBool>,
    exit_process: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RestartRequest {
    pub confirm: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RestartResponse {
    pub restart_scheduled: bool,
    pub mode: String,
}

impl RestartCoordinator {
    pub fn process_exit() -> Self {
        Self {
            requested: Arc::new(AtomicBool::new(false)),
            exit_process: true,
        }
    }

    #[doc(hidden)]
    pub fn for_test() -> Self {
        Self {
            requested: Arc::new(AtomicBool::new(false)),
            exit_process: false,
        }
    }

    pub fn request_restart(&self) {
        self.requested.store(true, Ordering::SeqCst);
        if self.exit_process {
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(250));
                std::process::exit(0);
            });
        }
    }

    #[doc(hidden)]
    pub fn was_requested(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

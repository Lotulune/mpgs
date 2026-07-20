//! Per-account AI admission control.
//!
//! Daily usage is persisted by storage; this module owns only short-lived
//! in-process concurrency permits so a single account cannot fan out model
//! calls while a previous request is still running.

use std::{
    collections::HashMap,
    env, io,
    sync::{Arc, Mutex},
};

const DEFAULT_DAILY_BUDGET: u32 = 50;
const DEFAULT_MAX_CONCURRENT: usize = 2;

#[derive(Clone)]
pub struct AccountAiLimiter {
    daily_budget: u32,
    max_concurrent: usize,
    active: Arc<Mutex<HashMap<String, usize>>>,
}

impl Default for AccountAiLimiter {
    fn default() -> Self {
        Self {
            daily_budget: DEFAULT_DAILY_BUDGET,
            max_concurrent: DEFAULT_MAX_CONCURRENT,
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl AccountAiLimiter {
    pub fn from_env() -> Result<Self, io::Error> {
        Ok(Self {
            daily_budget: positive_u32("MPGS_AI_ACCOUNT_DAILY_BUDGET", DEFAULT_DAILY_BUDGET)?,
            max_concurrent: positive_usize(
                "MPGS_AI_ACCOUNT_MAX_CONCURRENT",
                DEFAULT_MAX_CONCURRENT,
            )?,
            active: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub const fn daily_budget(&self) -> u32 {
        self.daily_budget
    }

    pub fn try_acquire(&self, user_id: &str) -> Option<AccountAiPermit> {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let current = active.entry(user_id.to_owned()).or_insert(0);
        if *current >= self.max_concurrent {
            return None;
        }
        *current += 1;
        Some(AccountAiPermit {
            user_id: user_id.to_owned(),
            active: Arc::clone(&self.active),
        })
    }
}

pub struct AccountAiPermit {
    user_id: String,
    active: Arc<Mutex<HashMap<String, usize>>>,
}

impl Drop for AccountAiPermit {
    fn drop(&mut self) {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let Some(current) = active.get_mut(&self.user_id) else {
            return;
        };
        *current = current.saturating_sub(1);
        if *current == 0 {
            active.remove(&self.user_id);
        }
    }
}

fn positive_u32(name: &str, default: u32) -> Result<u32, io::Error> {
    let Ok(value) = env::var(name) else {
        return Ok(default);
    };
    value
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must be a positive integer"),
            )
        })
}

fn positive_usize(name: &str, default: usize) -> Result<usize, io::Error> {
    let Ok(value) = env::var(name) else {
        return Ok(default);
    };
    value
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must be a positive integer"),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_concurrent_requests_per_account_and_releases_on_drop() {
        let limiter = AccountAiLimiter {
            daily_budget: 1,
            max_concurrent: 1,
            active: Arc::new(Mutex::new(HashMap::new())),
        };
        let permit = limiter.try_acquire("u_1").expect("first request allowed");
        assert!(limiter.try_acquire("u_1").is_none());
        assert!(limiter.try_acquire("u_2").is_some());
        drop(permit);
        assert!(limiter.try_acquire("u_1").is_some());
    }
}

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitBucket {
    PublicRead,
    Admin,
    Setup,
    Restart,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub public_read_limit: u32,
    pub admin_limit: u32,
    pub setup_limit: u32,
    pub restart_limit: u32,
    pub window: Duration,
}

#[derive(Debug, Clone)]
pub struct RateLimiters {
    config: RateLimitConfig,
    windows: Arc<Mutex<HashMap<RateLimitBucket, RateWindow>>>,
}

#[derive(Debug, Clone)]
struct RateWindow {
    started_at: Instant,
    used: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            public_read_limit: 300,
            admin_limit: 30,
            setup_limit: 10,
            restart_limit: 3,
            window: Duration::from_secs(60),
        }
    }
}

impl RateLimitConfig {
    pub fn for_tests(limit: u32) -> Self {
        Self {
            public_read_limit: limit,
            admin_limit: limit,
            setup_limit: limit,
            restart_limit: limit,
            window: Duration::from_secs(60),
        }
    }

    fn limit_for(&self, bucket: RateLimitBucket) -> u32 {
        match bucket {
            RateLimitBucket::PublicRead => self.public_read_limit,
            RateLimitBucket::Admin => self.admin_limit,
            RateLimitBucket::Setup => self.setup_limit,
            RateLimitBucket::Restart => self.restart_limit,
        }
    }
}

impl Default for RateLimiters {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

impl RateLimiters {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            windows: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn allow(&self, bucket: RateLimitBucket) -> bool {
        let limit = self.config.limit_for(bucket);
        if limit == 0 {
            return false;
        }

        let Ok(mut windows) = self.windows.lock() else {
            return false;
        };

        let now = Instant::now();
        let window = windows.entry(bucket).or_insert(RateWindow {
            started_at: now,
            used: 0,
        });

        if now.duration_since(window.started_at) >= self.config.window {
            window.started_at = now;
            window.used = 0;
        }

        if window.used >= limit {
            return false;
        }

        window.used += 1;
        true
    }
}

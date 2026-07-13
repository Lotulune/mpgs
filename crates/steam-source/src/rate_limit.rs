/// Simple token-bucket limiter for source request budgeting.
///
/// Time is injected as monotonic milliseconds so tests stay deterministic.
#[derive(Debug, Clone)]
pub struct TokenBucket {
    capacity: f64,
    tokens: f64,
    refill_per_ms: f64,
    last_ms: u64,
}

impl TokenBucket {
    pub fn new(capacity: u32, refill_per_second: f64, now_ms: u64) -> Self {
        assert!(capacity > 0, "capacity must be positive");
        assert!(
            refill_per_second > 0.0,
            "refill_per_second must be positive"
        );
        let capacity = f64::from(capacity);
        Self {
            capacity,
            tokens: capacity,
            refill_per_ms: refill_per_second / 1000.0,
            last_ms: now_ms,
        }
    }

    pub fn try_acquire(&mut self, now_ms: u64, cost: f64) -> Result<(), u64> {
        self.refill(now_ms);
        if self.tokens + f64::EPSILON >= cost {
            self.tokens -= cost;
            Ok(())
        } else {
            let missing = cost - self.tokens;
            let wait_ms = (missing / self.refill_per_ms).ceil() as u64;
            Err(wait_ms.max(1))
        }
    }

    pub fn tokens(&self) -> f64 {
        self.tokens
    }

    fn refill(&mut self, now_ms: u64) {
        if now_ms <= self.last_ms {
            return;
        }
        let elapsed = (now_ms - self.last_ms) as f64;
        self.tokens = (self.tokens + elapsed * self.refill_per_ms).min(self.capacity);
        self.last_ms = now_ms;
    }
}

/// Shared Steam Web API daily budget tracker (soft local accounting).
///
/// Official terms currently advertise 100_000 calls/day; local accounting keeps
/// a safety margin and is not a substitute for server-side 429 handling.
#[derive(Debug, Clone)]
pub struct DailyBudget {
    limit: u64,
    used: u64,
    day_key: u32,
}

impl DailyBudget {
    pub fn new(limit: u64, day_key: u32) -> Self {
        Self {
            limit,
            used: 0,
            day_key,
        }
    }

    pub fn try_consume(&mut self, day_key: u32, cost: u64) -> Result<(), u64> {
        if day_key != self.day_key {
            self.day_key = day_key;
            self.used = 0;
        }
        if self.used.saturating_add(cost) > self.limit {
            return Err(self.limit.saturating_sub(self.used));
        }
        self.used += cost;
        Ok(())
    }

    pub fn used(&self) -> u64 {
        self.used
    }

    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.used)
    }
}

#[cfg(test)]
mod tests {
    use super::{DailyBudget, TokenBucket};

    #[test]
    fn token_bucket_blocks_until_refill() {
        let mut bucket = TokenBucket::new(2, 1.0, 0);
        assert!(bucket.try_acquire(0, 1.0).is_ok());
        assert!(bucket.try_acquire(0, 1.0).is_ok());
        let wait = bucket.try_acquire(0, 1.0).unwrap_err();
        assert!(wait >= 1000);
        assert!(bucket.try_acquire(1000, 1.0).is_ok());
    }

    #[test]
    fn daily_budget_resets_on_day_change() {
        let mut budget = DailyBudget::new(10, 1);
        assert!(budget.try_consume(1, 10).is_ok());
        assert!(budget.try_consume(1, 1).is_err());
        assert!(budget.try_consume(2, 1).is_ok());
        assert_eq!(budget.used(), 1);
    }
}

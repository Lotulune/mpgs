pub fn opt_bool_to_sql(value: Option<bool>) -> Option<i64> {
    value.map(|v| if v { 1 } else { 0 })
}

pub fn sql_to_opt_bool(value: Option<i64>) -> Option<bool> {
    value.map(|v| v != 0)
}

/// Wilson score lower bound for positive ratings (z=1.96).
pub fn wilson_lower_bound(positive: u32, total: u32) -> Option<f64> {
    if total == 0 {
        return None;
    }
    let n = f64::from(total);
    let phat = f64::from(positive) / n;
    let z = 1.96_f64;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let centre = phat + z2 / (2.0 * n);
    let margin = z * ((phat * (1.0 - phat) + z2 / (4.0 * n)) / n).sqrt();
    Some(((centre - margin) / denominator).clamp(0.0, 1.0))
}

pub fn day_utc_from_ms(ms: i64) -> String {
    // Approximate UTC day from unix ms without chrono dependency.
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    // Unix epoch day 0 = 1970-01-01
    let (y, m, d) = civil_from_days(days + 719_468);
    format!("{y:04}-{m:02}-{d:02}")
}

// Howard Hinnant civil_from_days algorithm (public domain style).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::{day_utc_from_ms, wilson_lower_bound};

    #[test]
    fn wilson_handles_empty_and_positive() {
        assert_eq!(wilson_lower_bound(0, 0), None);
        let w = wilson_lower_bound(90, 100).unwrap();
        assert!(w > 0.8 && w < 0.95);
    }

    #[test]
    fn day_utc_epoch() {
        assert_eq!(day_utc_from_ms(0), "1970-01-01");
        assert_eq!(day_utc_from_ms(86_400_000 - 1), "1970-01-01");
        assert_eq!(day_utc_from_ms(86_400_000), "1970-01-02");
    }
}

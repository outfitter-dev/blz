use chrono::{DateTime, Duration, Utc};

pub const DEFAULT_STALE_AFTER_DAYS: i64 = 30;

#[must_use]
pub fn is_stale(fetched_at: DateTime<Utc>, threshold_days: i64) -> bool {
    let threshold = threshold_days.max(0);
    (Utc::now() - fetched_at) > Duration::days(threshold)
}

#[must_use]
pub fn days_since(fetched_at: DateTime<Utc>) -> i64 {
    (Utc::now() - fetched_at).num_days()
}

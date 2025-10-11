//! Timestamp utilities

use chrono::{DateTime, Utc};

/// Get current UTC timestamp
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

/// Convert milliseconds to duration
pub fn millis_to_duration(millis: u64) -> std::time::Duration {
    std::time::Duration::from_millis(millis)
}

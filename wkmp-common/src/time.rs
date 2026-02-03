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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_now_returns_valid_timestamp() {
        let timestamp = now();
        // Should be a reasonable timestamp (after year 2000)
        assert!(timestamp.timestamp() > 946_684_800); // 2000-01-01 00:00:00 UTC
    }

    #[test]
    fn test_now_returns_recent_timestamp() {
        let timestamp = now();
        // Should be reasonably recent (before year 2100)
        assert!(timestamp.timestamp() < 4_102_444_800); // 2100-01-01 00:00:00 UTC
    }

    #[tokio::test]
    async fn test_now_successive_calls_advance() {
        let time1 = now();
        tokio::time::sleep(Duration::from_millis(10)).await;
        let time2 = now();
        // Second call should be after first call
        assert!(time2 > time1);
    }

    #[test]
    fn test_millis_to_duration_zero() {
        let duration = millis_to_duration(0);
        assert_eq!(duration, Duration::from_millis(0));
        assert_eq!(duration.as_millis(), 0);
    }

    #[test]
    fn test_millis_to_duration_small_value() {
        let duration = millis_to_duration(100);
        assert_eq!(duration, Duration::from_millis(100));
        assert_eq!(duration.as_millis(), 100);
    }

    #[test]
    fn test_millis_to_duration_one_second() {
        let duration = millis_to_duration(1000);
        assert_eq!(duration, Duration::from_secs(1));
        assert_eq!(duration.as_millis(), 1000);
    }

    #[test]
    fn test_millis_to_duration_large_value() {
        let duration = millis_to_duration(3_600_000); // 1 hour
        assert_eq!(duration, Duration::from_secs(3600));
        assert_eq!(duration.as_millis(), 3_600_000);
    }

    #[test]
    fn test_millis_to_duration_max_u64() {
        // Should handle maximum u64 value without panic
        let duration = millis_to_duration(u64::MAX);
        assert_eq!(duration.as_millis(), u64::MAX as u128);
    }

    #[test]
    fn test_millis_to_duration_conversion_accuracy() {
        // Test that conversion is exact (no loss of precision)
        let millis = 12345u64;
        let duration = millis_to_duration(millis);
        assert_eq!(duration.as_millis() as u64, millis);
    }
}

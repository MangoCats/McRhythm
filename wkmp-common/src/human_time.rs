//! Human-readable time formatting per SPEC024
//!
//! Provides consistent time display formatting across all WKMP modules.
//!
//! **[SPEC024]** Human-Readable Time Display Standards

/// Time display format selection thresholds (seconds)
const SHORT_FORMAT_MAX: i64 = 100;        // < 100s → X.XXs
const MEDIUM_FORMAT_MAX: i64 = 6000;      // < 100m → M:SS.Xs
const LONG_FORMAT_MAX: i64 = 90000;       // < 25h → H:MM:SS
                                           // >= 25h → Dd-H:MM:SS

/// Format seconds as human-readable time per SPEC024.
///
/// **[SPEC024-FMT-010]** Format selection by typical maximum value:
/// - Short format (`X.XXs`): typical max < 100 seconds
/// - Medium format (`M:SS.Xs`): typical max 100s to 100m
/// - Long format (`H:MM:SS`): typical max 100m to 25h
/// - Extended format (`Dd-H:MM:SS`): typical max >= 25h
///
/// # Arguments
///
/// * `seconds` - Duration in seconds (can be negative for error conditions)
/// * `typical_max` - Typical maximum value for this field type (determines format)
///
/// # Returns
///
/// Formatted time string appropriate for the magnitude
///
/// # Examples
///
/// ```
/// use wkmp_common::human_time::format_human_time;
///
/// // Short format (< 100s)
/// assert_eq!(format_human_time(45, 100), "45.00s");
/// assert_eq!(format_human_time(5, 100), "5.00s");
///
/// // Medium format (100s to 100m)
/// assert_eq!(format_human_time(120, 6000), "2:00.0s");
/// assert_eq!(format_human_time(330, 6000), "5:30.0s");
///
/// // Long format (100m to 25h)
/// assert_eq!(format_human_time(7200, 14400), "2:00:00");
/// assert_eq!(format_human_time(3661, 14400), "1:01:01");
///
/// // Extended format (>= 25h)
/// assert_eq!(format_human_time(604800, 1209600), "7d-0:00:00");
/// assert_eq!(format_human_time(90000, 1209600), "1d-1:00:00");
/// ```
pub fn format_human_time(seconds: i64, typical_max: i64) -> String {
    // [SPEC024-EDGE-030] Handle negative values (error condition)
    let is_negative = seconds < 0;
    let abs_seconds = seconds.abs();

    // [SPEC024-FMT-010] Select format by typical maximum
    let formatted = if typical_max <= SHORT_FORMAT_MAX {
        // Short format: X.XXs
        format!("{:.2}s", abs_seconds as f64)
    } else if typical_max <= MEDIUM_FORMAT_MAX {
        // Medium format: M:SS.Xs
        let minutes = abs_seconds / 60;
        let secs = abs_seconds % 60;
        format!("{}:{:04.1}s", minutes, secs as f64)
    } else if typical_max <= LONG_FORMAT_MAX {
        // Long format: H:MM:SS
        let hours = abs_seconds / 3600;
        let mins = (abs_seconds % 3600) / 60;
        let secs = abs_seconds % 60;
        format!("{}:{:02}:{:02}", hours, mins, secs)
    } else {
        // Extended format: Dual sub-format based on actual value
        // [SPEC024-FMT-060] < 25h actual value: H:MM:SS, >= 25h: X.XXd
        if abs_seconds < LONG_FORMAT_MAX {
            // Sub-format A: H:MM:SS (< 25 hours actual value)
            let hours = abs_seconds / 3600;
            let mins = (abs_seconds % 3600) / 60;
            let secs = abs_seconds % 60;
            format!("{}:{:02}:{:02}", hours, mins, secs)
        } else {
            // Sub-format B: X.XXd (>= 25 hours actual value)
            let days = abs_seconds as f64 / 86400.0;
            // Format with up to 2 decimal places, removing trailing zeros
            // Round to 2 decimals first, then check if we can simplify
            let rounded_2dp = (days * 100.0).round() / 100.0;
            let rounded_1dp = (days * 10.0).round() / 10.0;

            let formatted_days = if (rounded_2dp - rounded_2dp.floor()).abs() < 0.001 {
                // It's effectively a whole number
                format!("{:.0}d", rounded_2dp)
            } else if (rounded_2dp * 10.0 - (rounded_2dp * 10.0).floor()).abs() < 0.001 {
                // Second decimal is 0, use 1 decimal place
                format!("{:.1}d", rounded_1dp)
            } else {
                // Need 2 decimal places
                format!("{:.2}d", rounded_2dp)
            };
            formatted_days
        }
    };

    if is_negative {
        format!("-{}", formatted)
    } else {
        formatted
    }
}

/// Format seconds as human-readable time with automatic typical max inference.
///
/// **[SPEC024-APP-010]** Common field type classifications:
/// - Fade durations: < 100s
/// - File/passage durations: 100s to 100m
/// - Artist cooldowns: 100m to 25h
/// - Song/work cooldowns: >= 25h
///
/// This is a convenience wrapper that infers typical_max from the current value.
/// For consistent column formatting, prefer `format_human_time()` with explicit typical_max.
///
/// # Arguments
///
/// * `seconds` - Duration in seconds
///
/// # Returns
///
/// Formatted time string with format inferred from value magnitude
///
/// # Examples
///
/// ```
/// use wkmp_common::human_time::format_human_time_auto;
///
/// assert_eq!(format_human_time_auto(45), "45.00s");
/// assert_eq!(format_human_time_auto(330), "5:30.0s");
/// assert_eq!(format_human_time_auto(7200), "2:00:00");
/// assert_eq!(format_human_time_auto(604800), "7d-0:00:00");
/// ```
pub fn format_human_time_auto(seconds: i64) -> String {
    let abs_seconds = seconds.abs();

    // Infer typical_max from current value (add 20% headroom)
    let typical_max = (abs_seconds as f64 * 1.2) as i64;

    format_human_time(seconds, typical_max)
}

/// Format Option<i64> seconds as human-readable time per SPEC024.
///
/// **[SPEC024-EDGE-020]** NULL handling: Returns "null" for None values.
///
/// # Arguments
///
/// * `seconds_opt` - Optional duration in seconds
/// * `typical_max` - Typical maximum value for this field type
///
/// # Returns
///
/// Formatted time string, or "null" if seconds_opt is None
///
/// # Examples
///
/// ```
/// use wkmp_common::human_time::format_human_time_opt;
///
/// assert_eq!(format_human_time_opt(Some(45), 100), "45.00s");
/// assert_eq!(format_human_time_opt(None, 100), "null");
/// ```
pub fn format_human_time_opt(seconds_opt: Option<i64>, typical_max: i64) -> String {
    match seconds_opt {
        Some(seconds) => format_human_time(seconds, typical_max),
        None => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_format() {
        // [SPEC024-FMT-030] Short format: X.XXs (< 100s)
        assert_eq!(format_human_time(0, 100), "0.00s");
        assert_eq!(format_human_time(5, 100), "5.00s");
        assert_eq!(format_human_time(45, 100), "45.00s");
        assert_eq!(format_human_time(99, 100), "99.00s");
    }

    #[test]
    fn test_medium_format() {
        // [SPEC024-FMT-040] Medium format: M:SS.Xs (100s to 100m)
        assert_eq!(format_human_time(100, 6000), "1:40.0s");
        assert_eq!(format_human_time(120, 6000), "2:00.0s");
        assert_eq!(format_human_time(330, 6000), "5:30.0s");
        assert_eq!(format_human_time(5999, 6000), "99:59.0s");
    }

    #[test]
    fn test_long_format() {
        // [SPEC024-FMT-050] Long format: H:MM:SS (100m to 25h)
        assert_eq!(format_human_time(6000, 90000), "1:40:00");
        assert_eq!(format_human_time(7200, 14400), "2:00:00");
        assert_eq!(format_human_time(3661, 14400), "1:01:01");
        assert_eq!(format_human_time(89999, 90000), "24:59:59");
    }

    #[test]
    fn test_extended_format() {
        // [SPEC024-FMT-060] Extended format: Dual sub-format based on actual value
        // Sub-format A: H:MM:SS (< 25h actual value, even with >=25h typical max)
        assert_eq!(format_human_time(3600, 1209600), "1:00:00");      // 1 hour
        assert_eq!(format_human_time(7200, 1209600), "2:00:00");      // 2 hours
        assert_eq!(format_human_time(86399, 1209600), "23:59:59");    // Just under 24h

        // Sub-format B: X.XXd (>= 25h actual value)
        assert_eq!(format_human_time(90000, 1209600), "1.04d");       // 25 hours
        assert_eq!(format_human_time(604800, 1209600), "7d");         // Exactly 7 days
        assert_eq!(format_human_time(613786, 1209600), "7.1d");       // 7.104 days
        assert_eq!(format_human_time(654307, 1209600), "7.57d");      // 7.573 days
        assert_eq!(format_human_time(1209600, 1209600), "14d");       // Exactly 14 days
        assert_eq!(format_human_time(31536000, 31536000), "365d");    // Exactly 365 days
    }

    #[test]
    fn test_boundary_values() {
        // [SPEC024-EDGE-010] Boundary values use larger format
        assert_eq!(format_human_time(100, 100), "100.00s");  // At boundary, still short
        assert_eq!(format_human_time(100, 6000), "1:40.0s"); // At boundary, use medium
        assert_eq!(format_human_time(6000, 90000), "1:40:00"); // At boundary, use long
        assert_eq!(format_human_time(90000, 1209600), "1.04d"); // At boundary (25h), use days format
    }

    #[test]
    fn test_negative_values() {
        // [SPEC024-EDGE-030] Negative values prepend minus sign
        assert_eq!(format_human_time(-5, 100), "-5.00s");
        assert_eq!(format_human_time(-120, 6000), "-2:00.0s");
        assert_eq!(format_human_time(-7200, 14400), "-2:00:00");
        assert_eq!(format_human_time(-604800, 1209600), "-7d");       // Negative days format
    }

    #[test]
    fn test_option_handling() {
        // [SPEC024-EDGE-020] NULL handling
        assert_eq!(format_human_time_opt(Some(45), 100), "45.00s");
        assert_eq!(format_human_time_opt(None, 100), "null");
    }

    #[test]
    fn test_auto_format() {
        // Auto format with inferred typical_max
        assert_eq!(format_human_time_auto(45), "45.00s");
        assert_eq!(format_human_time_auto(120), "2:00.0s");
        assert_eq!(format_human_time_auto(7200), "2:00:00");
        assert_eq!(format_human_time_auto(604800), "7d");  // Updated: days format
    }

    #[test]
    fn test_real_world_cooldowns() {
        // Song cooldowns: 7-14 days (always >= 25h, use days format)
        assert_eq!(format_human_time(604800, 1209600), "7d");
        assert_eq!(format_human_time(1209600, 1209600), "14d");

        // Artist cooldowns: 2-4 hours (< 25h, use H:MM:SS even with large typical_max)
        assert_eq!(format_human_time(7200, 14400), "2:00:00");
        assert_eq!(format_human_time(14400, 14400), "4:00:00");

        // Work cooldowns: 3-7 days (always >= 25h, use days format)
        assert_eq!(format_human_time(259200, 1209600), "3d");
        assert_eq!(format_human_time(604800, 1209600), "7d");
    }
}

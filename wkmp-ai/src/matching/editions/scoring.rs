//! Edition Match Scoring
//!
//! **[PLAN030]** Provides functions to score how well detected track durations
//! match expected edition durations. Used by Stage 2 to find the best
//! parameter combination for each edition.

use crate::matching::types::CandidateTestResult;

/// Score how well an edition matches detected track durations
///
/// Simple percentage-based scoring: counts how many tracks are within tolerance.
///
/// # Arguments
/// * `detected_durations` - Track durations detected from audio (seconds)
/// * `expected_durations` - Expected track durations from MusicBrainz (milliseconds)
/// * `tolerance_secs` - Allowed error per track (seconds)
///
/// # Returns
/// Match percentage (0.0 to 100.0)
pub fn score_edition_match(
    detected_durations: &[f64],
    expected_durations: &[u32],
    tolerance_secs: f64,
) -> f64 {
    if detected_durations.len() != expected_durations.len() {
        return 0.0;
    }

    if expected_durations.is_empty() {
        return 0.0;
    }

    let mut matched = 0;
    for (detected, expected) in detected_durations.iter().zip(expected_durations.iter()) {
        let expected_secs = *expected as f64 / 1000.0;
        let error = (detected - expected_secs).abs();
        if error <= tolerance_secs {
            matched += 1;
        }
    }

    (matched as f64 / expected_durations.len() as f64) * 100.0
}

/// Analyze track matching in detail
///
/// Provides detailed analysis of how detected tracks match expected tracks,
/// including per-track errors and overall statistics.
///
/// # Arguments
/// * `detected_durations` - Track durations detected from audio (seconds)
/// * `expected_durations` - Expected track durations from MusicBrainz (milliseconds)
/// * `tolerance_secs` - Allowed error per track (seconds)
///
/// # Returns
/// CandidateTestResult with match percentage, counts, and per-track errors
pub fn analyze_track_matching(
    detected_durations: &[f64],
    expected_durations: &[u32],
    tolerance_secs: f64,
) -> CandidateTestResult {
    let expected_count = expected_durations.len();
    let detected_count = detected_durations.len();

    // Handle count mismatch
    if detected_count != expected_count {
        return CandidateTestResult {
            percentage: 0.0,
            detected_durations: detected_durations.to_vec(),
            matched_count: 0,
            expected_count,
            errors: Vec::new(),
        };
    }

    // Handle empty case
    if expected_count == 0 {
        return CandidateTestResult {
            percentage: 0.0,
            detected_durations: Vec::new(),
            matched_count: 0,
            expected_count: 0,
            errors: Vec::new(),
        };
    }

    let mut matched_count = 0;
    let mut errors = Vec::new();

    for (detected, expected) in detected_durations.iter().zip(expected_durations.iter()) {
        let expected_secs = *expected as f64 / 1000.0;
        let error = (detected - expected_secs).abs();
        errors.push(error);

        if error <= tolerance_secs {
            matched_count += 1;
        }
    }

    let percentage = (matched_count as f64 / expected_count as f64) * 100.0;

    CandidateTestResult {
        percentage,
        detected_durations: detected_durations.to_vec(),
        matched_count,
        expected_count,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_perfect_match() {
        let detected = vec![180.0, 240.0, 200.0];
        let expected = vec![180000, 240000, 200000]; // Same in ms
        let tolerance = 10.0;

        let score = score_edition_match(&detected, &expected, tolerance);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_score_within_tolerance() {
        let detected = vec![185.0, 245.0, 205.0]; // 5 seconds off each
        let expected = vec![180000, 240000, 200000];
        let tolerance = 10.0;

        let score = score_edition_match(&detected, &expected, tolerance);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_score_partial_match() {
        let detected = vec![180.0, 260.0, 200.0]; // Second track 20s off
        let expected = vec![180000, 240000, 200000];
        let tolerance = 10.0;

        let score = score_edition_match(&detected, &expected, tolerance);
        // 2 out of 3 match
        assert!((score - 66.67).abs() < 1.0);
    }

    #[test]
    fn test_score_count_mismatch() {
        let detected = vec![180.0, 240.0]; // Only 2 tracks
        let expected = vec![180000, 240000, 200000];
        let tolerance = 10.0;

        let score = score_edition_match(&detected, &expected, tolerance);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_score_empty() {
        let detected: Vec<f64> = vec![];
        let expected: Vec<u32> = vec![];
        let tolerance = 10.0;

        let score = score_edition_match(&detected, &expected, tolerance);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_analyze_detailed() {
        let detected = vec![180.0, 245.0, 211.0];
        let expected = vec![180000, 240000, 200000];
        let tolerance = 10.0;

        let result = analyze_track_matching(&detected, &expected, tolerance);

        assert_eq!(result.expected_count, 3);
        assert_eq!(result.matched_count, 2); // First and second match
        assert_eq!(result.errors.len(), 3);

        // First track: 0 error
        assert!((result.errors[0] - 0.0).abs() < 0.01);
        // Second track: 5s error
        assert!((result.errors[1] - 5.0).abs() < 0.01);
        // Third track: 11s error (outside 10s tolerance)
        assert!((result.errors[2] - 11.0).abs() < 0.01);
    }

    #[test]
    fn test_analyze_count_mismatch() {
        let detected = vec![180.0, 240.0];
        let expected = vec![180000, 240000, 200000];
        let tolerance = 10.0;

        let result = analyze_track_matching(&detected, &expected, tolerance);

        assert_eq!(result.percentage, 0.0);
        assert_eq!(result.matched_count, 0);
        assert_eq!(result.expected_count, 3);
        assert!(result.errors.is_empty());
    }
}

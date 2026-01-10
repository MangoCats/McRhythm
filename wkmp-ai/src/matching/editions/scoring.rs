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

//
// ============================================================================
// EDITION SELECTION SCORING (PLAN027)
// ============================================================================
//
// Multi-factor weighted scoring for selecting the best MusicBrainz edition
// from multiple candidates. Addresses box set/deluxe edition selection issues.
//
// Requirements:
// - REQ-AM-092: Multi-factor weighted scoring
// - REQ-AM-093: Total duration alignment with graduated penalties
// - REQ-AM-094: Track quality graduated scoring
// - REQ-AM-095: Graduated track count tolerance
//

use std::cmp::Ordering;

/// Candidate edition with calculated scores
///
/// **[PLAN027]** Used for edition selection with multi-factor scoring
#[derive(Debug, Clone)]
pub struct EditionCandidate {
    pub mbid: String,
    pub title: String,
    pub track_count: usize,
    pub total_duration_ms: u64,
    pub track_durations_secs: Vec<f64>,
    pub name_similarity: f64,
    pub score: f64,
}

/// Calculate multi-factor weighted score for an edition
///
/// **[PLAN027] REQ-AM-092:** Multi-Factor Weighted Scoring
///
/// Formula:
/// ```text
/// base_score = (duration_score × 0.35) + (quality_score × 0.40) + (name_score × 0.25)
/// final_score = base_score × track_count_penalty
/// ```
///
/// # Arguments
/// * `duration_score` - Total duration alignment score (0.0-1.0)
/// * `quality_score` - Track quality score (0.0-1.0)
/// * `name_score` - Name similarity score (0.0-1.0)
/// * `track_count_penalty` - Multiplicative penalty for track count difference (0.20-1.00)
///
/// # Returns
/// Final edition score (0.0-1.0)
pub fn calculate_edition_score(
    duration_score: f64,
    quality_score: f64,
    name_score: f64,
    track_count_penalty: f64,
) -> f64 {
    let base = (duration_score * 0.35) + (quality_score * 0.40) + (name_score * 0.25);
    base * track_count_penalty
}

/// Calculate total duration alignment score with graduated penalties
///
/// **[DEPRECATED]** Use `calculate_total_duration_score_validated()` instead.
/// This version does not validate against actual file duration and can produce
/// incorrect scores for impossible edition/file combinations (e.g., 59-minute
/// file matched to 7-hour box set).
///
/// **[PLAN027] REQ-AM-093:** Total Duration Alignment
///
/// Penalty bands:
/// - <5% difference: 0.95 (excellent)
/// - 5-10%: 0.80 (good)
/// - 10-15%: 0.60 (acceptable)
/// - 15-25%: 0.30 (poor)
/// - >25%: 0.05 (very poor)
///
/// # Arguments
/// * `detected_total_ms` - Total detected duration in milliseconds
/// * `edition_total_ms` - Edition total duration in milliseconds
///
/// # Returns
/// Duration alignment score (0.05, 0.30, 0.60, 0.80, or 0.95)
#[deprecated(since = "0.1.0", note = "Use calculate_total_duration_score_validated")]
pub fn calculate_total_duration_score(
    detected_total_ms: u64,
    edition_total_ms: u64,
) -> f64 {
    // Edge case: zero duration
    if detected_total_ms == 0 || edition_total_ms == 0 {
        return 0.05;
    }

    let diff_ms = detected_total_ms.abs_diff(edition_total_ms);
    let diff_pct = (diff_ms as f64 / detected_total_ms as f64) * 100.0;

    if diff_pct < 5.0 {
        0.95
    } else if diff_pct < 10.0 {
        0.80
    } else if diff_pct < 15.0 {
        0.60
    } else if diff_pct < 25.0 {
        0.30
    } else {
        0.05
    }
}

/// Calculate total duration score with file duration validation
///
/// **[BUG FIX]** Enhanced version of `calculate_total_duration_score()` that validates
/// detected and edition totals against actual file duration. Prevents accepting editions
/// whose expected duration exceeds the physical audio file (e.g., 59-minute file matched
/// to 7-hour box set).
///
/// **[PLAN027] REQ-AM-093:** Total Duration Alignment with Validation
///
/// # Validation Logic
/// 1. If detected_total > file_total * 1.05: REJECT (0.05) - algorithm error
/// 2. If edition_total > file_total * 1.25: REJECT (0.05) - wrong edition (box set)
/// 3. Otherwise: Apply graduated penalties based on detected vs edition difference
///
/// # Penalty Bands
/// - <5% difference: 0.95 (excellent)
/// - 5-10%: 0.80 (good)
/// - 10-15%: 0.60 (acceptable)
/// - 15-25%: 0.30 (poor)
/// - >25%: 0.05 (very poor)
///
/// # Arguments
/// * `detected_total_ms` - Total detected duration in milliseconds
/// * `edition_total_ms` - Edition total duration in milliseconds
/// * `file_total_ms` - Actual audio file duration in milliseconds (from decoder)
///
/// # Returns
/// Duration alignment score (0.05, 0.30, 0.60, 0.80, or 0.95)
pub fn calculate_total_duration_score_validated(
    detected_total_ms: u64,
    edition_total_ms: u64,
    file_total_ms: u64,
) -> f64 {
    // Edge case: zero duration
    if detected_total_ms == 0 || edition_total_ms == 0 || file_total_ms == 0 {
        return 0.05;
    }

    // **[BUG FIX]** Validate detected total against actual file duration
    // If detected total exceeds file by >5%, something is very wrong (algorithm error)
    const MAX_DETECTED_OVERAGE: f64 = 1.05;
    if detected_total_ms as f64 > file_total_ms as f64 * MAX_DETECTED_OVERAGE {
        return 0.05; // Impossible - detected more audio than exists
    }

    // **[BUG FIX]** Validate edition total against actual file duration
    // If edition total exceeds file by >25%, this is wrong edition (box set, deluxe edition, etc.)
    const MAX_EDITION_OVERAGE: f64 = 1.25;
    if edition_total_ms as f64 > file_total_ms as f64 * MAX_EDITION_OVERAGE {
        return 0.05; // Impossible - edition far too long for this file
    }

    // Compare detected vs edition (original graduated penalty logic)
    let diff_ms = detected_total_ms.abs_diff(edition_total_ms);
    let diff_pct = (diff_ms as f64 / detected_total_ms as f64) * 100.0;

    if diff_pct < 5.0 {
        0.95
    } else if diff_pct < 10.0 {
        0.80
    } else if diff_pct < 15.0 {
        0.60
    } else if diff_pct < 25.0 {
        0.30
    } else {
        0.05
    }
}

/// Calculate track quality score with graduated linear decay
///
/// **[PLAN027] REQ-AM-094:** Track Quality Graduated Scoring
///
/// Quality formula for each track:
/// ```text
/// error_capped = min(error, tolerance × 2.0)
/// quality = max(-1.0, 1.0 - (error_capped / tolerance))
/// ```
///
/// Examples with 10s tolerance:
/// - 0s error → quality = 1.0 (perfect)
/// - 5s error → quality = 0.5 (good)
/// - 10s error → quality = 0.0 (at tolerance threshold)
/// - 15s error → quality = -0.5 (poor)
/// - 20s+ error → quality = -1.0 (catastrophic, floored)
///
/// Creates a symmetric range: +1.0 (perfect) to -1.0 (catastrophic).
/// Massive failures (>2× tolerance) actively penalize the average score
/// instead of just contributing zero. This distinguishes catastrophic mismatches
/// from borderline failures.
///
/// Final score is average quality across all matched tracks.
///
/// # Arguments
/// * `detected_durations` - Detected track durations in seconds
/// * `edition_durations` - Edition track durations in seconds
/// * `tolerance_secs` - Tolerance threshold in seconds (typically 10.0s)
///
/// # Returns
/// Average quality score (-1.0 to 1.0)
///
/// # Track Count Mismatch Handling
/// Quality calculated only on first `min(detected_count, edition_count)` tracks.
/// Extra tracks beyond minimum are ignored for quality calculation.
/// Track count difference penalty applied separately via REQ-AM-095.
pub fn calculate_track_quality_score(
    detected_durations: &[f64],
    edition_durations: &[f64],
    tolerance_secs: f64,
) -> f64 {
    // Edge case: empty arrays
    if detected_durations.is_empty() || edition_durations.is_empty() {
        return 0.0;
    }

    // Edge case: zero tolerance
    if tolerance_secs <= 0.0 {
        return 0.0;
    }

    const MAX_ERROR_MULTIPLIER: f64 = 2.0;  // Cap errors at 2.0× tolerance
    const MIN_QUALITY_FLOOR: f64 = -0.5;    // Middle ground negative floor

    let track_count = detected_durations.len().min(edition_durations.len());
    let mut total_quality = 0.0;

    for i in 0..track_count {
        let error = (detected_durations[i] - edition_durations[i]).abs();

        // Cap error at maximum threshold (e.g., 15s when tolerance is 10s)
        let error_capped = error.min(tolerance_secs * MAX_ERROR_MULTIPLIER);

        // Apply linear penalty, floored at negative minimum
        let track_quality = (1.0 - (error_capped / tolerance_secs)).max(MIN_QUALITY_FLOOR);

        total_quality += track_quality;
    }

    total_quality / track_count as f64
}

/// Calculate graduated track count tolerance penalty
///
/// **[PLAN027] REQ-AM-095:** Graduated Track Count Tolerance
///
/// Penalty levels:
/// - Exact match (diff = 0): 1.00 (no penalty)
/// - ±1 track: 0.95 (minimal penalty)
/// - ±2 tracks: 0.85
/// - ±3 tracks: 0.70
/// - ±4-5 tracks: 0.50
/// - ±6+ tracks: 0.20 (severe penalty, floor)
///
/// # Arguments
/// * `detected_count` - Number of detected tracks
/// * `edition_count` - Number of tracks in edition
///
/// # Returns
/// Multiplicative penalty (0.20-1.00)
pub fn calculate_track_count_penalty(
    detected_count: usize,
    edition_count: usize,
) -> f64 {
    let diff = detected_count.abs_diff(edition_count);

    match diff {
        0 => 1.00,       // Exact match
        1 => 0.95,       // ±1 track
        2 => 0.85,       // ±2 tracks
        3 => 0.70,       // ±3 tracks
        4..=5 => 0.50,   // ±4-5 tracks
        _ => 0.20,       // ±6+ tracks
    }
}

/// Select best edition from multiple candidates using multi-factor scoring
///
/// **[PLAN027] REQ-AM-092:** Multi-Factor Weighted Scoring
///
/// # Arguments
/// * `candidates` - List of edition candidates with pre-calculated scores
///
/// # Returns
/// * `Some(EditionCandidate)` - Best edition if valid candidates exist
/// * `None` - If no valid candidates (empty list, all scores ≤ 0.0)
///
/// # Tie-Breaking
/// If multiple editions have identical scores:
/// 1. Prefer higher name_similarity
/// 2. Prefer lexicographic MBID (deterministic)
pub fn select_best_edition(
    candidates: &[EditionCandidate],
) -> Option<EditionCandidate> {
    if candidates.is_empty() {
        return None;
    }

    // Filter to valid candidates (score > 0.0)
    let valid_candidates: Vec<_> = candidates.iter()
        .filter(|c| c.score > 0.0)
        .collect();

    if valid_candidates.is_empty() {
        return None;
    }

    // Find best candidate with tie-breaking
    let best = valid_candidates.iter()
        .max_by(|a, b| {
            // Primary: highest score wins (descending)
            match a.score.partial_cmp(&b.score) {
                Some(Ordering::Equal) => {
                    // Tie-break 1: highest name_similarity wins (descending)
                    match a.name_similarity.partial_cmp(&b.name_similarity) {
                        Some(Ordering::Equal) => {
                            // Tie-break 2: smallest MBID wins (ascending, deterministic)
                            b.mbid.cmp(&a.mbid)
                        }
                        other => other.unwrap_or(Ordering::Equal),
                    }
                }
                other => other.unwrap_or(Ordering::Equal),
            }
        })?;

    Some((*best).clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // PLAN030 Tests (Existing)
    // ========================================================================

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

    // ========================================================================
    // PLAN027 Tests (Edition Selection)
    // ========================================================================

    // REQ-AM-092: Multi-Factor Weighted Scoring

    #[test]
    fn test_multi_factor_scoring_weights() {
        // TC-U-092-01: Verify correct weight application
        let duration_score = 0.80;
        let quality_score = 0.90;
        let name_score = 0.70;
        let track_count_penalty = 1.00;

        let expected = (0.80 * 0.35) + (0.90 * 0.40) + (0.70 * 0.25);
        let actual = calculate_edition_score(
            duration_score,
            quality_score,
            name_score,
            track_count_penalty,
        );

        assert!((actual - expected).abs() < 0.001, "Expected {}, got {}", expected, actual);
    }

    #[test]
    fn test_multi_factor_scoring_penalty() {
        // TC-U-092-02: Verify multiplicative track count penalty
        let duration_score = 0.95;
        let quality_score = 0.85;
        let name_score = 0.70;
        let track_count_penalty = 0.85;

        let base = (0.95 * 0.35) + (0.85 * 0.40) + (0.70 * 0.25);
        let expected = base * 0.85;
        let actual = calculate_edition_score(
            duration_score,
            quality_score,
            name_score,
            track_count_penalty,
        );

        assert!((actual - expected).abs() < 0.001, "Expected {}, got {}", expected, actual);
        assert!(actual < base, "Penalty should reduce score");
    }

    #[test]
    fn test_select_best_edition_empty() {
        // TC-U-092-03: Empty editions list returns None
        let result = select_best_edition(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_select_best_edition_all_zero_scores() {
        // TC-U-092-04: All scores ≤ 0.0 returns None
        let candidates = vec![
            EditionCandidate {
                mbid: "a".to_string(),
                title: "Edition A".to_string(),
                track_count: 11,
                total_duration_ms: 2_400_000,
                track_durations_secs: vec![],
                name_similarity: 0.85,
                score: 0.0,
            },
            EditionCandidate {
                mbid: "b".to_string(),
                title: "Edition B".to_string(),
                track_count: 11,
                total_duration_ms: 2_400_000,
                track_durations_secs: vec![],
                name_similarity: 0.80,
                score: -0.05,
            },
        ];

        let result = select_best_edition(&candidates);
        assert!(result.is_none());
    }

    #[test]
    fn test_select_best_edition_tie_breaking() {
        // TC-U-092-05: Identical scores use tie-breaking
        let candidates = vec![
            EditionCandidate {
                mbid: "zzz-123".to_string(),
                title: "Edition A".to_string(),
                track_count: 11,
                total_duration_ms: 2_400_000,
                track_durations_secs: vec![],
                name_similarity: 0.70,
                score: 0.850,
            },
            EditionCandidate {
                mbid: "aaa-456".to_string(),
                title: "Edition B".to_string(),
                track_count: 11,
                total_duration_ms: 2_400_000,
                track_durations_secs: vec![],
                name_similarity: 0.80,
                score: 0.850,
            },
            EditionCandidate {
                mbid: "mmm-789".to_string(),
                title: "Edition C".to_string(),
                track_count: 11,
                total_duration_ms: 2_400_000,
                track_durations_secs: vec![],
                name_similarity: 0.80,
                score: 0.850,
            },
        ];

        let result = select_best_edition(&candidates).unwrap();
        // B and C have higher name_similarity (0.80 > 0.70), so A loses
        // B has lexicographically smaller MBID ("aaa" < "mmm"), so B wins
        assert_eq!(result.mbid, "aaa-456");
    }

    // REQ-AM-093: Total Duration Alignment

    #[test]
    fn test_duration_score_excellent() {
        // TC-U-093-01: <5% difference (0.95)
        let score = calculate_total_duration_score(2_400_000, 2_450_000);
        assert_eq!(score, 0.95);
    }

    #[test]
    fn test_duration_score_good() {
        // TC-U-093-02: 5-10% difference (0.80)
        let score = calculate_total_duration_score(2_400_000, 2_580_000);
        assert_eq!(score, 0.80);
    }

    #[test]
    fn test_duration_score_acceptable() {
        // TC-U-093-03: 10-15% difference (0.60)
        let score = calculate_total_duration_score(2_400_000, 2_700_000);
        assert_eq!(score, 0.60);
    }

    #[test]
    fn test_duration_score_poor() {
        // TC-U-093-04: 15-25% difference (0.30)
        let score = calculate_total_duration_score(2_400_000, 2_880_000);
        assert_eq!(score, 0.30);
    }

    #[test]
    fn test_duration_score_very_poor() {
        // TC-U-093-05: >25% difference (0.05)
        let score = calculate_total_duration_score(2_400_000, 8_500_000);
        assert_eq!(score, 0.05);
    }

    #[test]
    fn test_duration_score_zero_detected() {
        // TC-U-093-06: Zero detected duration (0.05)
        let score = calculate_total_duration_score(0, 2_400_000);
        assert_eq!(score, 0.05);
    }

    #[test]
    fn test_duration_score_zero_edition() {
        // TC-U-093-07: Zero edition duration (0.05)
        let score = calculate_total_duration_score(2_400_000, 0);
        assert_eq!(score, 0.05);
    }

    // REQ-AM-094: Track Quality Graduated Scoring

    #[test]
    fn test_track_quality_perfect() {
        // TC-U-094-01: Perfect match (quality = 1.0)
        let detected = vec![180.0, 210.0, 195.0];
        let edition = vec![180.0, 210.0, 195.0];
        let score = calculate_track_quality_score(&detected, &edition, 1.5);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_track_quality_linear_decay() {
        // TC-U-094-02: Linear decay within tolerance
        let detected = vec![180.0, 210.0, 195.0];
        let edition = vec![180.5, 211.2, 194.3];
        let score = calculate_track_quality_score(&detected, &edition, 1.5);
        // Expected: (0.6667 + 0.2000 + 0.5333) / 3 = 0.4667
        assert!((score - 0.467).abs() < 0.01, "Expected ~0.467, got {}", score);
    }

    #[test]
    fn test_track_quality_zero_beyond_tolerance() {
        // TC-U-094-03: Negative quality beyond tolerance (MIN_QUALITY_FLOOR = -0.5)
        // Errors: [2.0s, 5.0s, 5.0s] with tolerance 1.5s
        // Track 1: max(1.0 - 2.0/1.5, -0.5) = -0.333
        // Track 2: max(1.0 - 3.0/1.5, -0.5) = -0.5 (error capped at 2.0×tolerance)
        // Track 3: max(1.0 - 3.0/1.5, -0.5) = -0.5
        // Average: (-0.333 + -0.5 + -0.5) / 3 = -0.444
        let detected = vec![180.0, 210.0, 195.0];
        let edition = vec![182.0, 215.0, 190.0];
        let score = calculate_track_quality_score(&detected, &edition, 1.5);
        assert!((score + 0.444).abs() < 0.01, "Expected -0.444, got {}", score);
    }

    #[test]
    fn test_track_quality_mismatch_uses_min() {
        // TC-U-094-04: Track count mismatch (uses min length)
        let detected = vec![180.0, 210.0, 195.0];
        let edition = vec![180.0, 210.5, 195.2, 220.0, 240.0];
        let score = calculate_track_quality_score(&detected, &edition, 1.5);
        // Expected: (1.0 + 0.6667 + 0.8667) / 3 = 0.8444
        assert!((score - 0.844).abs() < 0.01, "Expected ~0.844, got {}", score);
    }

    #[test]
    fn test_track_quality_empty_arrays() {
        // TC-U-094-05: Both arrays empty (0.0)
        let score = calculate_track_quality_score(&[], &[], 1.5);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_track_quality_zero_tolerance() {
        // TC-U-094-06: Zero tolerance (0.0)
        let detected = vec![180.0, 210.0];
        let edition = vec![180.0, 210.0];
        let score = calculate_track_quality_score(&detected, &edition, 0.0);
        assert_eq!(score, 0.0);
    }

    // REQ-AM-095: Graduated Track Count Tolerance

    #[test]
    fn test_track_count_exact_match() {
        // TC-U-095-01: Exact match (penalty = 1.00)
        let penalty = calculate_track_count_penalty(11, 11);
        assert_eq!(penalty, 1.00);
    }

    #[test]
    fn test_track_count_plus_minus_one() {
        // TC-U-095-02: ±1 track (penalty = 0.95)
        let penalty_a = calculate_track_count_penalty(11, 12);
        let penalty_b = calculate_track_count_penalty(12, 11);
        assert_eq!(penalty_a, 0.95);
        assert_eq!(penalty_b, 0.95);
    }

    #[test]
    fn test_track_count_plus_minus_two() {
        // TC-U-095-03: ±2 tracks (penalty = 0.85)
        let penalty = calculate_track_count_penalty(11, 13);
        assert_eq!(penalty, 0.85);
    }

    #[test]
    fn test_track_count_plus_minus_three() {
        // TC-U-095-04: ±3 tracks (penalty = 0.70)
        let penalty = calculate_track_count_penalty(11, 14);
        assert_eq!(penalty, 0.70);
    }

    #[test]
    fn test_track_count_plus_minus_four_five() {
        // TC-U-095-05: ±4-5 tracks (penalty = 0.50)
        let penalty_4 = calculate_track_count_penalty(11, 15);
        let penalty_5 = calculate_track_count_penalty(11, 16);
        assert_eq!(penalty_4, 0.50);
        assert_eq!(penalty_5, 0.50);
    }

    #[test]
    fn test_track_count_six_plus() {
        // TC-U-095-06: ±6+ tracks (penalty = 0.20)
        let penalty_6 = calculate_track_count_penalty(11, 17);
        let penalty_136 = calculate_track_count_penalty(11, 147);
        assert_eq!(penalty_6, 0.20);
        assert_eq!(penalty_136, 0.20);
    }
}

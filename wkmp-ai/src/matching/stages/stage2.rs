//! Stage 2: Parameter Grid Search
//!
//! **[PLAN030]** Tests all 180 parameter combinations against each edition.
//! Uses pre-computed silence cache for efficiency.
//!
//! # Algorithm Overview
//!
//! 1. For each edition (in name-distance order):
//!    - Test all threshold/duration combinations from silence cache
//!    - Track best match percentage and parameters
//!    - Early exit on 100% match (within grace period)
//! 2. Return results sorted by match percentage

use crate::matching::{
    constants::{EARLY_EXIT_GRACE_PERIOD_SECS, MIN_DURATION_VALUES, THRESHOLD_VALUES},
    editions::analyze_track_matching,
    types::{CandidateTestResult, Edition, SilenceCache},
};

/// Stage 2 result for a single edition
#[derive(Debug, Clone)]
pub struct Stage2Result {
    /// Edition that was tested
    pub edition: Edition,
    /// Best match percentage achieved (0-100)
    pub best_percentage: f64,
    /// Threshold index that achieved best match
    pub best_threshold_idx: usize,
    /// Min duration index that achieved best match
    pub best_duration_idx: usize,
    /// Detected track durations at best parameters (seconds)
    pub detected_durations: Vec<f64>,
    /// Per-track errors at best parameters (seconds)
    pub track_errors: Vec<f64>,
    /// Number of tracks matched within tolerance
    pub matched_count: usize,
    /// Whether all tracks matched within tolerance
    pub all_tracks_matched: bool,
}

impl Stage2Result {
    /// Get the best threshold value in dB
    pub fn best_threshold_db(&self) -> f64 {
        THRESHOLD_VALUES
            .get(self.best_threshold_idx)
            .copied()
            .unwrap_or(THRESHOLD_VALUES[0])
    }

    /// Get the best min duration value in seconds
    pub fn best_min_duration_secs(&self) -> f64 {
        MIN_DURATION_VALUES
            .get(self.best_duration_idx)
            .copied()
            .unwrap_or(MIN_DURATION_VALUES[0])
    }
}

/// Early exit configuration for Stage 2
#[derive(Debug, Clone)]
pub struct EarlyExitConfig {
    /// Enable early exit on perfect match
    pub enabled: bool,
    /// Grace period after 100% match - continue testing N more editions
    pub grace_editions: usize,
    /// Minimum match percentage to consider acceptable
    pub min_acceptable_percentage: f64,
}

impl Default for EarlyExitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            grace_editions: EARLY_EXIT_GRACE_PERIOD_SECS as usize,
            min_acceptable_percentage: 80.0,
        }
    }
}

/// Run Stage 2 parameter grid search
///
/// Tests all parameter combinations for each edition using the pre-computed
/// silence cache. Returns results sorted by match percentage (best first).
///
/// # Arguments
/// * `silence_cache` - Pre-computed silence detections (180 combinations)
/// * `audio_samples` - Raw audio sample data (for boundary refinement)
/// * `sample_rate` - Sample rate in Hz
/// * `editions` - Candidate editions to test (should be sorted by name distance)
/// * `tolerance_secs` - Track match tolerance (seconds)
/// * `early_exit` - Early exit configuration
///
/// # Returns
/// Vector of results for each edition tested, sorted by best_percentage descending
pub fn run_stage2(
    silence_cache: &SilenceCache,
    audio_samples: &[f32],
    sample_rate: u32,
    editions: &[Edition],
    tolerance_secs: f64,
    early_exit: &EarlyExitConfig,
) -> Vec<Stage2Result> {
    let num_thresholds = THRESHOLD_VALUES.len();
    let num_min_durations = MIN_DURATION_VALUES.len();

    let mut results = Vec::new();
    let mut perfect_match_found = false;
    let mut editions_since_perfect = 0;

    for (edition_idx, edition) in editions.iter().enumerate() {
        // Early exit check
        if perfect_match_found && early_exit.enabled {
            editions_since_perfect += 1;
            if editions_since_perfect > early_exit.grace_editions {
                break;
            }
        }

        let result = test_edition_stage2(
            silence_cache,
            audio_samples,
            sample_rate,
            edition,
            tolerance_secs,
            num_thresholds,
            num_min_durations,
        );

        if result.best_percentage >= 100.0 {
            perfect_match_found = true;
            editions_since_perfect = 0;

            // **[PERF-OPT-002]** Strict early exit for first-edition perfect match
            // Rationale: Editions are pre-sorted by name similarity. If the top-ranked
            // edition (index 0) achieves 100% match, it's almost certainly correct.
            // Testing additional editions wastes time (10-20 min/edition for long albums).
            // Grace period still applies for perfect matches found in later editions,
            // allowing discovery of better alternatives.
            // Expected impact: 10-20 minutes saved per obvious match.
            if edition_idx == 0 && early_exit.enabled {
                results.push(result);
                break;  // Skip remaining editions
            }
        }

        results.push(result);
    }

    // Sort by best percentage descending
    results.sort_by(|a, b| {
        b.best_percentage
            .partial_cmp(&a.best_percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

/// Test single edition across all parameter combinations
fn test_edition_stage2(
    silence_cache: &SilenceCache,
    audio_samples: &[f32],
    sample_rate: u32,
    edition: &Edition,
    tolerance_secs: f64,
    num_thresholds: usize,
    num_min_durations: usize,
) -> Stage2Result {
    let mut best_percentage = 0.0;
    let mut best_threshold_idx = 0;
    let mut best_duration_idx = 0;
    let mut best_result: Option<CandidateTestResult> = None;

    for threshold_idx in 0..num_thresholds {
        for duration_idx in 0..num_min_durations {
            // Calculate flat index: threshold_idx * num_min_durations + duration_idx
            let cache_idx = threshold_idx * num_min_durations + duration_idx;

            // Get detected durations for this parameter combination
            let detected = match silence_cache.get(cache_idx) {
                Some(d) => d,
                None => continue,
            };

            let result = analyze_track_matching(detected, &edition.durations, tolerance_secs);

            if result.percentage > best_percentage {
                best_percentage = result.percentage;
                best_threshold_idx = threshold_idx;
                best_duration_idx = duration_idx;
                best_result = Some(result);
            }

            // Early exit on 100% for this edition
            if best_percentage >= 100.0 {
                break;
            }
        }

        if best_percentage >= 100.0 {
            break;
        }
    }

    let (mut detected_durations, track_errors, matched_count) = match best_result {
        Some(r) => (r.detected_durations, r.errors, r.matched_count),
        None => (Vec::new(), Vec::new(), 0),
    };

    // **[BOUNDARY REFINEMENT]** Apply to Stage 2 results
    // **[PERF-OPT-001]** Skip refinement for perfect matches (100.0%)
    // Rationale: Refinement is computationally expensive (~10-60 minutes for long albums)
    // and provides no benefit when all tracks already match perfectly.
    // Expected impact: 30-50% reduction in processing time for albums with obvious matches.
    if !detected_durations.is_empty() && best_percentage < 100.0 {
        let edition_durations_secs: Vec<f64> = edition
            .durations
            .iter()
            .map(|&ms| ms as f64 / 1000.0)
            .collect();

        use super::apply_refinement_to_durations;
        detected_durations = apply_refinement_to_durations(
            &detected_durations,
            &edition_durations_secs,
            audio_samples,
            sample_rate as f64,
            tolerance_secs,
        );
    }

    Stage2Result {
        edition: edition.clone(),
        best_percentage,
        best_threshold_idx,
        best_duration_idx,
        detected_durations,
        track_errors,
        matched_count,
        all_tracks_matched: best_percentage >= 100.0,
    }
}

/// Check if Stage 2 found acceptable match
///
/// Returns true if the best result meets minimum percentage threshold.
pub fn stage2_success(results: &[Stage2Result], min_percentage: f64) -> bool {
    results
        .first()
        .map(|r| r.best_percentage >= min_percentage)
        .unwrap_or(false)
}

/// Get the best Stage 2 result
///
/// Returns the result with highest match percentage, or None if empty.
pub fn get_best_stage2_result(results: &[Stage2Result]) -> Option<&Stage2Result> {
    results.first()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_edition(track_count: usize, durations_ms: &[u32]) -> Edition {
        Edition {
            release_mbid: "test-mbid".to_string(),
            title: "Test Album".to_string(),
            artist: "Test Artist".to_string(),
            artist_credit: None,
            country: None,
            status: None,
            track_count,
            track_durations: durations_ms.iter().map(|&d| d as f64 / 1000.0).collect(),
            recording_mbids: Vec::new(),
            name_distance_rank: None,
            name_distance_score: None,
            durations: durations_ms.to_vec(),
            track_titles: (1..=track_count).map(|i| format!("Track {}", i)).collect(),
        }
    }

    fn create_test_silence_cache(detected_durations: Vec<f64>) -> SilenceCache {
        // Create cache with same durations for all 180 combinations
        let num_combinations = THRESHOLD_VALUES.len() * MIN_DURATION_VALUES.len();
        vec![detected_durations; num_combinations]
    }

    fn create_test_audio_samples(duration_secs: f64, sample_rate: u32) -> Vec<f32> {
        // Create dummy audio samples for testing
        let total_samples = (duration_secs * sample_rate as f64) as usize;
        vec![0.0f32; total_samples]
    }

    #[test]
    fn test_perfect_match() {
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        let cache = create_test_silence_cache(vec![180.0, 240.0, 200.0]);
        let sample_rate = 44100u32;
        let audio_samples = create_test_audio_samples(620.0, sample_rate); // 180+240+200 = 620s

        let results = run_stage2(&cache, &audio_samples, sample_rate, &[edition], 10.0, &EarlyExitConfig::default());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].best_percentage, 100.0);
        assert!(results[0].all_tracks_matched);
    }

    #[test]
    fn test_partial_match() {
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        // Third track is 25s off (outside 10s tolerance)
        let cache = create_test_silence_cache(vec![180.0, 240.0, 225.0]);
        let sample_rate = 44100u32;
        let audio_samples = create_test_audio_samples(625.0, sample_rate); // 180+240+225 = 645s

        let results = run_stage2(&cache, &audio_samples, sample_rate, &[edition], 10.0, &EarlyExitConfig::default());

        assert_eq!(results.len(), 1);
        // 2 out of 3 tracks match
        assert!((results[0].best_percentage - 66.67).abs() < 1.0);
        assert!(!results[0].all_tracks_matched);
    }

    #[test]
    fn test_count_mismatch_n_minus_1() {
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        // Only 2 tracks detected (N-1 case: compares first 2 → 2/3 = 66.67%)
        let cache = create_test_silence_cache(vec![180.0, 240.0]);
        let sample_rate = 44100u32;
        let audio_samples = create_test_audio_samples(420.0, sample_rate); // 180+240 = 420s

        let results = run_stage2(&cache, &audio_samples, sample_rate, &[edition], 10.0, &EarlyExitConfig::default());

        assert_eq!(results.len(), 1);
        assert!((results[0].best_percentage - 66.67).abs() < 0.1,
            "N-1: expected ~66.67%, got {:.2}%", results[0].best_percentage);
    }

    #[test]
    fn test_count_mismatch_n_minus_3() {
        let edition = create_test_edition(4, &[180000, 240000, 200000, 220000]);
        // Only 1 track detected (N-3: should return 0%)
        let cache = create_test_silence_cache(vec![180.0]);
        let sample_rate = 44100u32;
        let audio_samples = create_test_audio_samples(180.0, sample_rate);

        let results = run_stage2(&cache, &audio_samples, sample_rate, &[edition], 10.0, &EarlyExitConfig::default());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].best_percentage, 0.0);
    }

    #[test]
    fn test_best_parameter_selection() {
        let edition = create_test_edition(3, &[180000, 240000, 200000]);

        // Create cache where only one specific combination has good durations
        let num_combinations = THRESHOLD_VALUES.len() * MIN_DURATION_VALUES.len();
        let mut cache: SilenceCache = vec![vec![0.0, 0.0, 0.0]; num_combinations];

        // Set combination at threshold_idx=5, duration_idx=7 to have good match
        let good_idx = 5 * MIN_DURATION_VALUES.len() + 7;
        cache[good_idx] = vec![180.0, 240.0, 200.0];
        let sample_rate = 44100u32;
        let audio_samples = create_test_audio_samples(620.0, sample_rate);

        let results = run_stage2(&cache, &audio_samples, sample_rate, &[edition], 10.0, &EarlyExitConfig::default());

        assert_eq!(results[0].best_percentage, 100.0);
        assert_eq!(results[0].best_threshold_idx, 5);
        assert_eq!(results[0].best_duration_idx, 7);
    }

    #[test]
    fn test_edition_ranking() {
        let edition1 = create_test_edition(3, &[180000, 240000, 200000]);
        let edition2 = create_test_edition(3, &[100000, 200000, 300000]); // Different durations

        // Cache matches edition1 perfectly, edition2 partially
        let cache = create_test_silence_cache(vec![180.0, 240.0, 200.0]);
        let sample_rate = 44100u32;
        let audio_samples = create_test_audio_samples(620.0, sample_rate);

        let results = run_stage2(
            &cache,
            &audio_samples,
            sample_rate,
            &[edition2.clone(), edition1.clone()],
            10.0,
            &EarlyExitConfig::default(),
        );

        // Best match should be first
        assert_eq!(results[0].best_percentage, 100.0);
        assert_eq!(results[0].edition.durations, edition1.durations);
    }

    #[test]
    fn test_early_exit() {
        let edition1 = create_test_edition(3, &[180000, 240000, 200000]); // Perfect match
        let edition2 = create_test_edition(3, &[100000, 200000, 300000]); // No match
        let edition3 = create_test_edition(3, &[100000, 200000, 300000]); // No match

        let cache = create_test_silence_cache(vec![180.0, 240.0, 200.0]);
        let sample_rate = 44100u32;
        let audio_samples = create_test_audio_samples(620.0, sample_rate);

        let early_exit = EarlyExitConfig {
            enabled: true,
            grace_editions: 0, // No grace period
            min_acceptable_percentage: 80.0,
        };

        let results = run_stage2(&cache, &audio_samples, sample_rate, &[edition1, edition2, edition3], 10.0, &early_exit);

        // Should only have tested 1 edition (early exit after perfect match)
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].best_percentage, 100.0);
    }

    #[test]
    fn test_early_exit_with_grace() {
        let edition1 = create_test_edition(3, &[180000, 240000, 200000]); // Perfect match
        let edition2 = create_test_edition(3, &[100000, 200000, 300000]); // No match
        let edition3 = create_test_edition(3, &[100000, 200000, 300000]); // No match

        let cache = create_test_silence_cache(vec![180.0, 240.0, 200.0]);
        let sample_rate = 44100u32;
        let audio_samples = create_test_audio_samples(620.0, sample_rate);

        let early_exit = EarlyExitConfig {
            enabled: true,
            grace_editions: 1, // Allow 1 more edition after perfect match
            min_acceptable_percentage: 80.0,
        };

        let results = run_stage2(&cache, &audio_samples, sample_rate, &[edition1, edition2, edition3], 10.0, &early_exit);

        // Should have tested 2 editions (1 perfect + 1 grace)
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_stage2_success() {
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        let cache = create_test_silence_cache(vec![180.0, 240.0, 200.0]);
        let sample_rate = 44100u32;
        let audio_samples = create_test_audio_samples(620.0, sample_rate);

        let results = run_stage2(&cache, &audio_samples, sample_rate, &[edition], 10.0, &EarlyExitConfig::default());

        assert!(stage2_success(&results, 80.0));
        assert!(stage2_success(&results, 100.0));
        assert!(!stage2_success(&[], 80.0));
    }

    #[test]
    fn test_best_threshold_values() {
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        let cache = create_test_silence_cache(vec![180.0, 240.0, 200.0]);
        let sample_rate = 44100u32;
        let audio_samples = create_test_audio_samples(620.0, sample_rate);

        let results = run_stage2(&cache, &audio_samples, sample_rate, &[edition], 10.0, &EarlyExitConfig::default());

        // Best parameters should return valid dB and seconds values
        // THRESHOLD_VALUES range: -42 to -66 dB
        // MIN_DURATION_VALUES range: 0.05 to 5.0 seconds
        let best = &results[0];
        assert!(best.best_threshold_db() <= -40.0 && best.best_threshold_db() >= -70.0);
        assert!(best.best_min_duration_secs() >= 0.05 && best.best_min_duration_secs() <= 5.0);
    }
}

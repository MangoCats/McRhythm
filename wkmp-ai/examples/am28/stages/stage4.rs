//! # Stage 4: Edition-Guided Quiet Spot Detection (RMS)
//!
//! Uses RMS profiling to find quiet spots guided by expected track durations
//! from MusicBrainz editions.
//!
//! ## Features
//! - **RMS profiling**: Pre-compute RMS values across entire audio file
//! - **Edition-guided search**: Use expected track boundaries as search centers
//! - **Dynamic search radius**: 15% of track duration, clamped to 5-20s
//! - **Score-based selection**: Balance RMS level vs distance from expected boundary
//!
//! ## Algorithm
//! For each expected boundary position:
//! 1. Calculate search window (±15% of preceding track duration, clamped)
//! 2. Find all RMS samples within search window
//! 3. Score each candidate: `score = rms_db + distance_penalty`
//! 4. Select candidate with lowest score (quietest spot nearest to expected)
//!
//! ## Use Case
//! When silence detection fails (no clear silence regions), use RMS profiling
//! to find *relatively* quiet spots near where tracks *should* transition based
//! on MusicBrainz metadata.
//!
//! ## Performance Characteristics
//! - RMS profile computed once per album (O(n) where n = audio samples)
//! - Each edition test is O(k × w) where k = tracks, w = window candidates
//! - Typically faster than silence detection parameter sweep
//!
//! ## Requirements Coverage
//! - TEST-FUNC-006: Edition-guided quiet spot detection
//!
//! ## Related Modules
//! - `constants`: QUIET_SPOT_* constants, DB_MULTIPLIER, SILENCE_* constants
//! - `types`: CandidateTestResult
//! - `silence_detection`: calculate_rms() (RMS calculation)
//! - `matching::candidate`: test_segmentation_against_single_edition()

use crate::constants::*;
use crate::silence_detection::calculate_rms;
use crate::types::CandidateTestResult;
use crate::matching::candidate::test_segmentation_against_single_edition;
use tracing::info;

// =============================================================================
// RMS Profile Computation
// =============================================================================

/// Calculate RMS values across entire audio file for quiet spot detection
///
/// Computes RMS amplitude at regular intervals across the audio file,
/// creating a profile of loudness over time. This profile is then used
/// to find relatively quiet spots near expected track boundaries.
///
/// # Arguments
/// * `samples` - Audio samples (mono, f32, -1.0 to 1.0)
/// * `sample_rate` - Sample rate in Hz (e.g., 44100)
///
/// # Returns
/// Vec of (position_secs, rms_value) tuples representing the RMS profile
///
/// # Window Configuration
/// - Window size: QUIET_SPOT_WINDOW_SECS (0.5s = 500ms)
/// - Window step: QUIET_SPOT_WINDOW_STEP_SECS (0.25s = 250ms, 50% overlap)
///
/// # Example
/// ```ignore
/// let (samples, sample_rate) = decode_mp3("album.mp3")?;
/// let rms_profile = calculate_rms_profile(&samples, 44100);
/// // Profile contains (time, rms) pairs every 0.25s
/// ```
pub(crate) fn calculate_rms_profile(
    samples: &[f32],
    sample_rate: u32,
) -> Vec<(f64, f32)> {
    let window_samples = (sample_rate as f64 * QUIET_SPOT_WINDOW_SECS) as usize;
    let step_samples = (sample_rate as f64 * QUIET_SPOT_WINDOW_STEP_SECS) as usize;

    let mut rms_profile = Vec::new();
    let mut pos = 0;

    while pos + window_samples <= samples.len() {
        let rms = calculate_rms(&samples[pos..pos + window_samples]);
        let pos_secs = pos as f64 / sample_rate as f64;
        rms_profile.push((pos_secs, rms));
        pos += step_samples;
    }

    rms_profile
}

// =============================================================================
// Edition-Guided Boundary Detection
// =============================================================================

/// Edition-guided quiet spot detection for Stage 4
///
/// Searches for quiet spots near expected track boundaries based on edition durations.
/// Uses dynamic search radius and score-based selection to balance RMS level against
/// distance from expected boundary.
///
/// # Arguments
/// * `rms_profile` - Pre-computed RMS profile (from calculate_rms_profile)
/// * `expected_durations` - Expected track durations from MusicBrainz (seconds)
/// * `total_duration_secs` - Total audio file duration (seconds)
///
/// # Returns
/// Vec of detected boundary positions in seconds (length = tracks - 1)
///
/// # Algorithm
/// For each expected boundary:
/// 1. **Calculate expected position**: Cumulative sum of preceding track durations
/// 2. **Dynamic search radius**: 15% of preceding track duration, clamped to [5s, 20s]
/// 3. **Find candidates**: All RMS samples within [expected - radius, expected + radius]
/// 4. **Score candidates**: `score = rms_db + distance_penalty`
///    - `rms_db = 20 * log10(rms)` (quieter = lower score)
///    - `distance_penalty = (distance / radius) × 0.5 × 20` (further = higher score)
/// 5. **Select best**: Candidate with lowest score (quietest spot nearest expected)
///
/// # Fallback Behavior
/// If no RMS samples found in search window, uses expected position as boundary.
///
/// # Example
/// ```ignore
/// let expected = vec![182, 243, 191];  // 3 tracks
/// let boundaries = find_edition_guided_boundaries(&rms_profile, &expected, 616.0);
/// // Returns 2 boundaries: [~182s, ~425s]
/// ```
pub(crate) fn find_edition_guided_boundaries(
    rms_profile: &[(f64, f32)],
    expected_durations: &[u32],
    total_duration_secs: f64,
) -> Vec<f64> {
    if expected_durations.is_empty() || rms_profile.is_empty() {
        return Vec::new();
    }

    // Calculate expected boundary positions (cumulative durations)
    let mut expected_boundaries: Vec<f64> = Vec::new();
    let mut cumulative = 0.0;
    for (i, &dur) in expected_durations.iter().enumerate() {
        cumulative += dur as f64;
        // Don't add boundary after last track
        if i < expected_durations.len() - 1 {
            expected_boundaries.push(cumulative);
        }
    }

    // For each expected boundary, find quietest spot within search window
    let mut detected_boundaries = Vec::new();

    for (boundary_idx, &expected_pos) in expected_boundaries.iter().enumerate() {
        // Calculate dynamic search radius (15% of preceding track duration, clamped)
        let prev_track_dur = if boundary_idx == 0 {
            expected_durations[0] as f64
        } else {
            expected_durations[boundary_idx] as f64
        };
        let dynamic_radius = (prev_track_dur * QUIET_SPOT_SEARCH_RADIUS_RATIO)
            .max(QUIET_SPOT_SEARCH_RADIUS_MIN)
            .min(QUIET_SPOT_SEARCH_RADIUS_MAX);

        let search_start = (expected_pos - dynamic_radius).max(0.0);
        let search_end = (expected_pos + dynamic_radius).min(total_duration_secs);

        // Find RMS values within search window
        let candidates: Vec<_> = rms_profile.iter()
            .filter(|(pos, _)| *pos >= search_start && *pos <= search_end)
            .collect();

        if candidates.is_empty() {
            // No candidates in window - use expected position
            detected_boundaries.push(expected_pos);
            continue;
        }

        // Score each candidate: lower RMS is better, but penalize distance from expected
        let mut best_pos = expected_pos;
        let mut best_score = f64::INFINITY;

        for &(pos, rms) in &candidates {
            let rms_db = if *rms > SILENCE_RMS_EPSILON {
                DB_MULTIPLIER * (*rms as f64).log10()
            } else {
                SILENCE_DB_FLOOR as f64
            };

            // Score = RMS in dB + penalty for distance from expected
            let distance = (pos - expected_pos).abs();
            let distance_penalty = (distance / dynamic_radius) * QUIET_SPOT_PROXIMITY_PENALTY * QUIET_SPOT_DISTANCE_PENALTY_MULTIPLIER;
            let score = rms_db + distance_penalty;

            if score < best_score {
                best_score = score;
                best_pos = *pos;
            }
        }

        // Check if best spot meets minimum quietness threshold
        // If not, still use it but it might not be a real boundary
        detected_boundaries.push(best_pos);
    }

    detected_boundaries
}

// =============================================================================
// Helper: Boundaries to Durations Conversion
// =============================================================================

/// Convert detected boundary positions to track durations
///
/// Given boundary positions (times where tracks split), calculate the
/// duration of each track.
///
/// # Arguments
/// * `boundaries` - Boundary positions in seconds (length = tracks - 1)
/// * `total_duration_secs` - Total audio file duration (seconds)
///
/// # Returns
/// Vec of track durations in seconds (length = tracks)
///
/// # Algorithm
/// ```ignore
/// for each boundary:
///     duration[i] = boundary[i] - boundary[i-1]
/// duration[last] = total_duration - last_boundary
/// ```
///
/// # Example
/// ```ignore
/// let boundaries = vec![182.0, 425.0];  // 2 boundaries
/// let durations = boundaries_to_durations(&boundaries, 616.0);
/// // Returns [182.0, 243.0, 191.0] (3 tracks)
/// ```
pub(crate) fn boundaries_to_durations(
    boundaries: &[f64],
    total_duration_secs: f64,
) -> Vec<f64> {
    let mut durations = Vec::new();
    let mut prev_pos = 0.0;

    for &boundary in boundaries {
        durations.push(boundary - prev_pos);
        prev_pos = boundary;
    }

    // Final track (from last boundary to end of file)
    if prev_pos < total_duration_secs {
        durations.push(total_duration_secs - prev_pos);
    }

    durations
}

// =============================================================================
// Stage 4: Main Entry Point
// =============================================================================

/// Stage 4: Quiet spot detection for a SINGLE edition (Run 15+)
///
/// Uses this edition's track durations as guide for boundary detection.
/// Searches for relatively quiet spots near expected track boundaries using
/// pre-computed RMS profile.
///
/// # Arguments
/// * `expected_durations` - Expected track durations from MusicBrainz edition (seconds)
/// * `edition_id` - MusicBrainz Release ID (MBID) for logging
/// * `tolerance` - Tolerance for track matching (seconds, typically 3.0)
/// * `current_best_percentage` - Best percentage achieved so far
/// * `rms_profile` - Pre-calculated RMS profile (time, rms) pairs
/// * `total_duration_secs` - Total audio file duration (seconds)
/// * `edition_idx` - Index of this edition (0-based, for logging)
/// * `total_editions` - Total number of editions being tested (for logging)
/// * `album_idx` - Album index (0-based, for logging)
///
/// # Returns
/// * `Some(result)` - CandidateTestResult if guided detection improved best percentage
/// * `None` - If early-exit triggered, no improvement, or empty inputs
///
/// # Early-Exit Behavior
/// Returns None immediately if `current_best_percentage >= 100.0` (perfect match
/// already found by previous stage/edition).
///
/// # Algorithm
/// 1. Calculate expected boundary positions from track durations
/// 2. For each expected boundary, find quietest spot within dynamic search radius
/// 3. Convert detected boundaries to track durations
/// 4. Test durations against expected durations
/// 5. Return result if improved over current best
///
/// # Logging
/// Logs when guided detection achieves new best percentage.
///
/// # Example
/// ```ignore
/// let rms_profile = calculate_rms_profile(&samples, 44100);
/// let result = run_stage4_single_edition(
///     &vec![182, 243, 191],  // Expected durations
///     "edition-mbid",
///     3.0,  // tolerance
///     75.0,  // current_best
///     &rms_profile,
///     616.0,  // total_duration
///     0, 1, 0  // edition_idx, total_editions, album_idx
/// );
/// ```
pub(crate) fn run_stage4_single_edition(
    expected_durations: &[u32],
    edition_id: &str,
    tolerance: f64,
    current_best_percentage: f64,
    rms_profile: &[(f64, f32)],  // Pre-calculated RMS profile (time, rms)
    total_duration_secs: f64,
    edition_idx: usize,
    total_editions: usize,
    album_idx: usize,
) -> Option<CandidateTestResult> {
    // Early-exit if already have 100% match
    if current_best_percentage >= 100.0 {
        return None;
    }

    if expected_durations.is_empty() || rms_profile.is_empty() {
        return None;
    }

    // Find quiet spots near expected boundaries for this edition
    let detected_boundaries = find_edition_guided_boundaries(
        rms_profile,
        expected_durations,
        total_duration_secs,
    );

    // Convert boundaries to track durations
    let guided_durations = boundaries_to_durations(&detected_boundaries, total_duration_secs);

    // Test guided result
    let result = test_segmentation_against_single_edition(
        &guided_durations,
        expected_durations,
        edition_id,
        tolerance,
    );

    if result.percentage > current_best_percentage {
        info!("[A{}]       [Edition {}/{}] New best: {:.1}% via guided quiet spots",
            album_idx + 1, edition_idx + 1, total_editions, result.percentage);
        Some(result)
    } else {
        None
    }
}

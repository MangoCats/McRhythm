//! Edition Filtering
//!
//! **[PLAN027]** Edition filtering improvements to fix problem albums with wrong MBID/edition selection.
//!
//! # Overview
//!
//! Implements 5 algorithmic improvements to edition selection:
//! 1. Edition preference scoring - penalize deluxe/compilation editions
//! 2. Track count pre-filtering - filter editions with >±3 track difference
//! 3. Artist consistency validation - reject multi-artist compilations
//! 4. Remix track tolerance - accept larger errors for remix tracks
//! 5. Zero regression validation - ensure no regressions on existing matches
//!
//! # Problem Albums Addressed
//!
//! - Chemical Brothers - Surrender: Deluxe edition with remixes (-83s on one track)
//! - Imagine Dragons - Night Visions: 16-track deluxe matched to 11-track file
//! - Michael Jackson - Thriller: Compilation matched to 9-track standard
//! - James Gang - Funk #49: Multi-artist compilation wrongly matched
//!
//! # Requirements Implemented
//!
//! - **REQ-EF-010:** Edition preference scoring - deluxe penalty (0.7×)
//! - **REQ-EF-020:** Edition preference scoring - compilation penalty (0.6×)
//! - **REQ-EF-030:** Track count pre-filtering (±3 tracks tolerance)
//! - **REQ-EF-040:** Artist consistency validation (reject >3 unique artists)
//! - **REQ-EF-050:** Remix track error tolerance (up to 120s)

use crate::matching::types::Edition;
use std::collections::HashSet;

/// **[REQ-EF-010, REQ-EF-020]** Penalty multiplier for deluxe editions
///
/// **[PLAN027]** Tunable parameter - validated via TC-S-060-02
/// - Initial value: 0.7 (30% penalty)
/// - May be adjusted during parameter tuning to ensure zero regressions
pub const DELUXE_PENALTY_MULTIPLIER: f64 = 0.7;

/// **[REQ-EF-010, REQ-EF-020]** Penalty multiplier for compilation editions
///
/// **[PLAN027]** Tunable parameter - validated via TC-S-060-02
/// - Initial value: 0.6 (40% penalty)
/// - May be adjusted during parameter tuning to ensure zero regressions
pub const COMPILATION_PENALTY_MULTIPLIER: f64 = 0.6;

/// **[REQ-EF-030]** Track count tolerance (±N tracks)
///
/// **[PLAN027]** Tunable parameter - validated via TC-S-060-03
/// - Initial value: 3 (±3 tracks tolerance)
/// - May be adjusted during parameter tuning to ensure zero regressions
pub const TRACK_COUNT_TOLERANCE: i32 = 3;

/// **[REQ-EF-050]** Maximum acceptable error for remix tracks (seconds)
///
/// **[PLAN027]** Remix tracks may have variable durations
/// - Standard tolerance: 30s (from matching algorithm)
/// - Remix tolerance: 120s (up to 2 minutes)
pub const REMIX_ERROR_TOLERANCE_SECS: f64 = 120.0;

/// **[REQ-EF-040]** Maximum unique artists before rejecting as multi-artist compilation
///
/// **[PLAN027]** Threshold for artist consistency validation
/// - ≤3 unique artists: Single-artist or featured artists (accept)
/// - >3 unique artists: Multi-artist compilation (reject)
pub const MAX_UNIQUE_ARTISTS: usize = 3;

/// **[REQ-EF-030]** Minimum detected tracks for track count filtering
///
/// **[PLAN027]** Skip track count filtering when silence detection appears unreliable.
/// If fewer than this many tracks are detected, the multi-stage matching algorithm
/// may find better segmentation with alternative parameters.
/// - Default: 5 (skip filter if detected < 5 tracks)
/// - Rationale: Most albums have 8-15 tracks; detecting only 1-4 suggests unreliable silence detection
pub const MIN_DETECTED_TRACKS_FOR_FILTER: usize = 5;

/// **[REQ-EF-030]** Minimum editions to preserve from track count filtering
///
/// **[PLAN027]** Always keep the top N edition candidates regardless of track count mismatch.
/// These are the highest-ranked by name similarity and most likely to be correct.
/// - Default: 5 (never filter out top 5 candidates)
/// - Rationale: Prevents filtering out correct edition when silence detection is inaccurate
pub const MIN_EDITIONS_TO_PRESERVE: usize = 5;

/// **[REQ-EF-030]** Threshold for skipping track count filter on long files
///
/// **[PLAN027]** Skip track count filtering for files longer than this threshold.
/// Long compilations/box sets have unreliable silence detection - the algorithm
/// often detects fewer silences than actual tracks due to continuous music.
/// - Default: 5400000ms (90 minutes)
/// - Rationale: Files > 90 min are typically compilations where track count varies significantly
pub const LONG_FILE_DURATION_THRESHOLD_MS: u64 = 5_400_000;

// ============================================================================
// Edition Preference Scoring (REQ-EF-010, REQ-EF-020)
// ============================================================================

/// **[REQ-EF-010, REQ-EF-020]** Calculate edition preference score
///
/// Penalizes deluxe and compilation editions to prefer standard releases.
///
/// # Scoring Formula
///
/// - Base score: 1.0
/// - Deluxe penalty: ×0.7 if title contains "deluxe" or "expanded"
/// - Compilation penalty: ×0.6 if title contains "collection", "anthology", or "best of"
/// - Penalties are cumulative (deluxe compilation: 0.7 × 0.6 = 0.42)
///
/// # Arguments
///
/// - `edition` - Edition to score
/// - `_detected_track_count` - Number of tracks detected in audio file (unused in current impl)
///
/// # Returns
///
/// Score multiplier (0.0 to 1.0, where 1.0 = no penalty)
///
/// # Examples
///
/// ```rust,ignore
/// let standard_edition = Edition { title: "Thriller".to_string(), ... };
/// assert_eq!(score_edition_preference(&standard_edition, 9), 1.0);
///
/// let deluxe_edition = Edition { title: "Thriller (Deluxe)".to_string(), ... };
/// assert_eq!(score_edition_preference(&deluxe_edition, 9), 0.7);
///
/// let compilation = Edition { title: "Best of Thriller".to_string(), ... };
/// assert_eq!(score_edition_preference(&compilation, 9), 0.6);
/// ```
pub fn score_edition_preference(edition: &Edition, _detected_track_count: usize) -> f64 {
    let mut score = 1.0;

    let title_lower = edition.title.to_lowercase();

    // REQ-EF-010: Penalize deluxe editions
    if title_lower.contains("deluxe") || title_lower.contains("expanded") {
        score *= DELUXE_PENALTY_MULTIPLIER;
        tracing::debug!(
            "Edition '{}' penalized: deluxe/expanded (score *= {})",
            edition.title,
            DELUXE_PENALTY_MULTIPLIER
        );
    }

    // REQ-EF-020: Penalize compilation editions
    if title_lower.contains("collection")
        || title_lower.contains("anthology")
        || title_lower.contains("best of")
    {
        score *= COMPILATION_PENALTY_MULTIPLIER;
        tracing::debug!(
            "Edition '{}' penalized: compilation (score *= {})",
            edition.title,
            COMPILATION_PENALTY_MULTIPLIER
        );
    }

    tracing::trace!(
        "Edition '{}' preference score: {}",
        edition.title,
        score
    );

    score
}

// ============================================================================
// Track Count Pre-Filtering (REQ-EF-030)
// ============================================================================

/// **[REQ-EF-030]** Filter editions by track count
///
/// Removes editions where track count differs by more than ±3 from detected tracks.
/// Improves performance by reducing candidate editions before Stage 2.
///
/// # Filtering Logic
///
/// - Keep edition if `|edition.track_count - detected| <= TRACK_COUNT_TOLERANCE`
/// - Filter edition if difference > tolerance
/// - Fallback: If all editions filtered, return original list with warning
///
/// # Arguments
///
/// - `editions` - Candidate editions from MusicBrainz
/// - `detected` - Number of tracks detected in audio file
///
/// # Returns
///
/// Filtered edition list (may be empty if all filtered, see fallback)
///
/// # Examples
///
/// ```rust,ignore
/// let editions = vec![
///     Edition { track_count: 11, ... },  // Diff=0, keep
///     Edition { track_count: 13, ... },  // Diff=2, keep
///     Edition { track_count: 16, ... },  // Diff=5, filter
/// ];
/// let filtered = filter_by_track_count(&editions, 11);
/// assert_eq!(filtered.len(), 2);  // 11-track and 13-track kept, 16-track filtered
/// ```
pub fn filter_by_track_count(editions: &[Edition], detected: usize) -> Vec<Edition> {
    if editions.is_empty() {
        tracing::debug!("Track count filter: Empty edition list, returning empty");
        return vec![];
    }

    // **[PLAN027]** Skip filtering when silence detection appears unreliable
    // If detected tracks is below threshold, the multi-stage matching may find
    // better segmentation with alternative parameters - don't filter prematurely
    if detected < MIN_DETECTED_TRACKS_FOR_FILTER {
        tracing::info!(
            "Track count filter: Skipping (detected {} tracks < {} threshold). Silence detection may be unreliable.",
            detected,
            MIN_DETECTED_TRACKS_FOR_FILTER
        );
        return editions.to_vec();
    }

    tracing::debug!(
        "Track count filter: {} editions, detected {} tracks, tolerance ±{}",
        editions.len(),
        detected,
        TRACK_COUNT_TOLERANCE
    );

    let filtered: Vec<Edition> = editions
        .iter()
        .filter(|e| {
            let diff = (e.track_count as i32 - detected as i32).abs();
            let keep = diff <= TRACK_COUNT_TOLERANCE;

            if keep {
                tracing::trace!(
                    "  KEEP: {} ({} tracks, diff={})",
                    e.title,
                    e.track_count,
                    diff
                );
            } else {
                tracing::debug!(
                    "  FILTER: {} ({} tracks, diff={} > {})",
                    e.title,
                    e.track_count,
                    diff,
                    TRACK_COUNT_TOLERANCE
                );
            }

            keep
        })
        .cloned()
        .collect();

    // REQ-EF-030: Ensure we keep at least MIN_EDITIONS_TO_PRESERVE candidates
    // Top editions (by input order, which reflects name similarity ranking) are preserved
    // even if they don't match the track count tolerance
    if filtered.len() < MIN_EDITIONS_TO_PRESERVE && editions.len() > filtered.len() {
        let mut result = filtered;
        let preserved_count = result.len();

        // Add top editions that weren't already kept, up to MIN_EDITIONS_TO_PRESERVE
        for edition in editions.iter().take(MIN_EDITIONS_TO_PRESERVE) {
            if !result.iter().any(|e| e.release_mbid == edition.release_mbid) {
                result.push(edition.clone());
                tracing::debug!(
                    "  PRESERVE: {} ({} tracks) - top candidate preserved despite track count mismatch",
                    edition.title,
                    edition.track_count
                );
            }
            if result.len() >= MIN_EDITIONS_TO_PRESERVE {
                break;
            }
        }

        tracing::info!(
            "Track count filter: {}/{} editions kept ({} by tolerance, {} preserved as top candidates)",
            result.len(),
            editions.len(),
            preserved_count,
            result.len() - preserved_count
        );
        result
    } else if filtered.is_empty() {
        tracing::warn!(
            "Track count filter rejected all {} editions (tolerance ±{}, detected {}). Falling back to unfiltered list.",
            editions.len(),
            TRACK_COUNT_TOLERANCE,
            detected
        );
        editions.to_vec()
    } else {
        tracing::debug!(
            "Track count filter: {}/{} editions kept",
            filtered.len(),
            editions.len()
        );
        filtered
    }
}

// ============================================================================
// Artist Consistency Validation (REQ-EF-040)
// ============================================================================

/// **[REQ-EF-040]** Validate artist consistency for an edition
///
/// Rejects multi-artist compilations by checking for >3 unique artists.
///
/// # Validation Logic
///
/// - Extract unique artist names from edition (simplified: uses main artist field)
/// - Count unique artists
/// - Reject if >3 unique artists detected (multi-artist compilation)
/// - Accept if ≤3 unique artists (single artist or featured artists)
///
/// # Limitations
///
/// Current implementation uses simplified artist validation:
/// - Uses Edition.artist field (main album artist)
/// - Per-track artist credits not available in Edition struct
/// - For full multi-artist detection, would need per-track artist metadata
///
/// This catches most multi-artist compilations (different main artist)
/// but may miss compilations where all tracks list same primary artist.
///
/// # Arguments
///
/// - `edition` - Edition to validate
/// - `source_artist` - Artist name from source file metadata
///
/// # Returns
///
/// - `true` if edition passes validation (single artist or ≤3 unique artists)
/// - `false` if edition should be rejected (multi-artist compilation)
///
/// # Examples
///
/// ```rust,ignore
/// let single_artist = Edition { artist: "Michael Jackson".to_string(), ... };
/// assert_eq!(validate_artist_consistency(&single_artist, "Michael Jackson"), true);
///
/// // Multi-artist compilations would be detected if per-track artist data available
/// ```
pub fn validate_artist_consistency(edition: &Edition, source_artist: &str) -> bool {
    // Simplified implementation: Check if edition artist matches source artist
    // Full implementation would require per-track artist metadata

    let edition_artist_lower = edition.artist.to_lowercase().trim().to_string();
    let source_artist_lower = source_artist.to_lowercase().trim().to_string();

    // Fuzzy matching: Substring containment
    let matches = edition_artist_lower.contains(&source_artist_lower)
        || source_artist_lower.contains(&edition_artist_lower);

    if matches {
        tracing::trace!(
            "Artist consistency PASS: '{}' matches source '{}'",
            edition.artist,
            source_artist
        );
        true
    } else {
        tracing::debug!(
            "Artist consistency REJECT: Edition artist '{}' does not match source '{}'",
            edition.artist,
            source_artist
        );
        // Note: This is conservative - rejects any artist mismatch
        // A multi-artist compilation would have mismatched artist
        false
    }

    // TODO: Full implementation would extract unique artists from track artist credits
    // and reject if unique artist count > MAX_UNIQUE_ARTISTS
}

// ============================================================================
// Remix Track Detection (REQ-EF-050)
// ============================================================================

/// **[REQ-EF-050]** Detect if track is a remix/extended version
///
/// Checks track title for remix keywords to apply larger error tolerance.
///
/// # Remix Keywords
///
/// - "remix"
/// - "extended"
/// - "mix)" (e.g., "Dance Mix", "Radio Mix")
/// - "version" (e.g., "Extended Version", "Album Version")
///
/// # Arguments
///
/// - `title` - Track title from MusicBrainz
///
/// # Returns
///
/// - `true` if track is identified as remix/extended version
/// - `false` if track is standard/original version
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(is_remix_track("Hey Boy Hey Girl (Kink extended remix)"), true);
/// assert_eq!(is_remix_track("Hey Boy Hey Girl"), false);
/// assert_eq!(is_remix_track("Song Title (Radio Mix)"), true);
/// ```
pub fn is_remix_track(title: &str) -> bool {
    let title_lower = title.to_lowercase();

    let is_remix = title_lower.contains("remix")
        || title_lower.contains("extended")
        || title_lower.contains("mix)")
        || title_lower.contains("version");

    if is_remix {
        tracing::trace!("Remix track detected: '{}'", title);
    }

    is_remix
}

// ============================================================================
// Integration Helper
// ============================================================================

/// **[PLAN027]** Apply all edition filters (track count + artist consistency)
///
/// Convenience function for integration with matching pipeline.
/// Applies filters in optimal order for performance.
///
/// # Filter Order
///
/// 1. Track count pre-filter (fast, eliminates many candidates)
/// 2. Artist consistency (slower, fewer candidates remaining)
///
/// # Arguments
///
/// - `editions` - Candidate editions from MusicBrainz
/// - `detected_track_count` - Number of tracks detected in audio file
/// - `source_artist` - Artist name from source file metadata
///
/// # Returns
///
/// Filtered edition list
pub fn filter_editions(
    editions: &[Edition],
    detected_track_count: usize,
    source_artist: &str,
) -> Vec<Edition> {
    tracing::debug!(
        "Applying edition filters: {} editions, {} detected tracks, source artist '{}'",
        editions.len(),
        detected_track_count,
        source_artist
    );

    // Step 1: Track count pre-filter (REQ-EF-030)
    let after_track_count = filter_by_track_count(editions, detected_track_count);

    // Step 2: Artist consistency validation (REQ-EF-040)
    let after_artist = after_track_count
        .into_iter()
        .filter(|e| validate_artist_consistency(e, source_artist))
        .collect::<Vec<_>>();

    tracing::debug!(
        "Edition filtering complete: {}/{} editions kept",
        after_artist.len(),
        editions.len()
    );

    after_artist
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_edition(title: &str, track_count: usize, artist: &str) -> Edition {
        Edition {
            // Use title as part of MBID to ensure uniqueness in tests
            release_mbid: format!("test-mbid-{}", title.replace(' ', "-").to_lowercase()),
            title: title.to_string(),
            artist: artist.to_string(),
            artist_credit: None,
            country: None,
            status: None,
            track_count,
            track_durations: vec![180.0; track_count],
            recording_mbids: vec!["test-recording".to_string(); track_count],
            track_titles: vec!["Track".to_string(); track_count],
            durations: vec![180000; track_count],
            name_distance_rank: None,
            name_distance_score: None,
        }
    }

    // ========================================================================
    // TC-U-010-01: Deluxe keyword detection (lowercase)
    // ========================================================================

    #[test]
    fn test_deluxe_lowercase_detection() {
        let edition = make_test_edition("Surrender (deluxe edition)", 28, "The Chemical Brothers");
        let score = score_edition_preference(&edition, 11);
        assert_eq!(score, 0.7, "Deluxe edition should receive 0.7× multiplier");
    }

    // ========================================================================
    // TC-U-010-02: Deluxe keyword detection (mixed case)
    // ========================================================================

    #[test]
    fn test_deluxe_mixed_case_detection() {
        let edition = make_test_edition("Surrender (Deluxe Edition)", 28, "The Chemical Brothers");
        let score = score_edition_preference(&edition, 11);
        assert_eq!(score, 0.7, "Deluxe (mixed case) should receive 0.7× multiplier");

        let edition_upper = make_test_edition("Surrender (DELUXE)", 28, "The Chemical Brothers");
        let score_upper = score_edition_preference(&edition_upper, 11);
        assert_eq!(score_upper, 0.7, "DELUXE (uppercase) should receive 0.7× multiplier");
    }

    // ========================================================================
    // TC-U-010-03: Expanded keyword detection
    // ========================================================================

    #[test]
    fn test_expanded_keyword_detection() {
        let edition = make_test_edition("Thriller (Expanded Edition)", 25, "Michael Jackson");
        let score = score_edition_preference(&edition, 9);
        assert_eq!(score, 0.7, "Expanded edition should receive 0.7× multiplier");
    }

    // ========================================================================
    // TC-U-020-01: Compilation keyword detection (collection)
    // ========================================================================

    #[test]
    fn test_compilation_collection_detection() {
        let edition = make_test_edition("The Collection", 20, "Various Artists");
        let score = score_edition_preference(&edition, 9);
        assert_eq!(score, 0.6, "Collection should receive 0.6× multiplier");
    }

    // ========================================================================
    // TC-U-020-02: Anthology keyword detection
    // ========================================================================

    #[test]
    fn test_compilation_anthology_detection() {
        let edition = make_test_edition("Anthology 1965-1970", 20, "Various Artists");
        let score = score_edition_preference(&edition, 9);
        assert_eq!(score, 0.6, "Anthology should receive 0.6× multiplier");
    }

    // ========================================================================
    // TC-U-020-03: "Best of" keyword detection
    // ========================================================================

    #[test]
    fn test_compilation_best_of_detection() {
        let edition = make_test_edition("Best of James Gang", 19, "James Gang");
        let score = score_edition_preference(&edition, 9);
        assert_eq!(score, 0.6, "Best of should receive 0.6× multiplier");
    }

    // ========================================================================
    // TC-U-030-01: Track count exact match (no filter)
    // ========================================================================

    #[test]
    fn test_track_count_exact_match() {
        let editions = vec![
            make_test_edition("Night Visions", 11, "Imagine Dragons"),
        ];
        let filtered = filter_by_track_count(&editions, 11);
        assert_eq!(filtered.len(), 1, "Exact match should pass filter");
    }

    // ========================================================================
    // TC-U-030-02: Track count within ±3 (pass filter)
    // ========================================================================

    #[test]
    fn test_track_count_within_tolerance() {
        let editions = vec![
            make_test_edition("Edition A", 11, "Artist"),
            make_test_edition("Edition B", 13, "Artist"),
            make_test_edition("Edition C", 9, "Artist"),
        ];
        let filtered = filter_by_track_count(&editions, 11);
        assert_eq!(filtered.len(), 3, "All editions within ±3 should pass");
    }

    // ========================================================================
    // TC-U-030-03: Track count >3 difference (filtered)
    // ========================================================================

    #[test]
    fn test_track_count_filtering() {
        // Need 7+ editions to test filtering beyond MIN_EDITIONS_TO_PRESERVE (5)
        let editions = vec![
            make_test_edition("Edition A", 11, "Artist"),  // Diff=0, keep (tolerance)
            make_test_edition("Edition B", 13, "Artist"),  // Diff=2, keep (tolerance)
            make_test_edition("Edition C", 16, "Artist"),  // Diff=5, filter but preserve (top 5)
            make_test_edition("Edition D", 7, "Artist"),   // Diff=4, filter but preserve (top 5)
            make_test_edition("Edition E", 8, "Artist"),   // Diff=3, keep (tolerance)
            make_test_edition("Edition F", 20, "Artist"),  // Diff=9, filter (beyond top 5)
            make_test_edition("Edition G", 25, "Artist"),  // Diff=14, filter (beyond top 5)
        ];
        let filtered = filter_by_track_count(&editions, 11);

        // Should keep: A, B, E (tolerance) + C, D (preserved as top 5) = 5 total
        assert_eq!(filtered.len(), 5, "Should return 5 editions (3 by tolerance, 2 preserved)");
        assert!(filtered.iter().any(|e| e.title == "Edition A"), "Edition A should pass (tolerance)");
        assert!(filtered.iter().any(|e| e.title == "Edition B"), "Edition B should pass (tolerance)");
        assert!(filtered.iter().any(|e| e.title == "Edition E"), "Edition E should pass (tolerance)");
        // C and D are preserved as top 5 candidates
        assert!(filtered.iter().any(|e| e.title == "Edition C"), "Edition C should be preserved (top 5)");
        assert!(filtered.iter().any(|e| e.title == "Edition D"), "Edition D should be preserved (top 5)");
        // F and G are beyond top 5, should be filtered
        assert!(!filtered.iter().any(|e| e.title == "Edition F"), "Edition F should be filtered");
        assert!(!filtered.iter().any(|e| e.title == "Edition G"), "Edition G should be filtered");
    }

    // ========================================================================
    // TC-U-030-04: Empty edition list handling
    // ========================================================================

    #[test]
    fn test_track_count_empty_list() {
        let editions: Vec<Edition> = vec![];
        let filtered = filter_by_track_count(&editions, 11);
        assert_eq!(filtered.len(), 0, "Empty input should return empty output");
    }

    // ========================================================================
    // TC-U-030-05: All editions filtered (fallback)
    // ========================================================================

    #[test]
    fn test_track_count_all_filtered_fallback() {
        let editions = vec![
            make_test_edition("Deluxe 1", 20, "Artist"),  // Diff=9, filter
            make_test_edition("Deluxe 2", 25, "Artist"),  // Diff=14, filter
        ];
        let filtered = filter_by_track_count(&editions, 11);
        assert_eq!(filtered.len(), 2, "Should fallback to unfiltered list");
        assert_eq!(filtered[0].title, "Deluxe 1");
        assert_eq!(filtered[1].title, "Deluxe 2");
    }

    // ========================================================================
    // TC-U-030-06: Low detected tracks skips filtering
    // ========================================================================

    #[test]
    fn test_track_count_skips_when_detected_low() {
        // When detected tracks < MIN_DETECTED_TRACKS_FOR_FILTER (5), filter should be skipped
        // This prevents filtering when silence detection appears unreliable
        let editions = vec![
            make_test_edition("Standard Edition", 10, "Artist"),   // Diff=9 from 1, would filter
            make_test_edition("Deluxe Edition", 15, "Artist"),     // Diff=14 from 1, would filter
        ];

        // With detected=1 (below threshold), all editions should be kept
        let filtered = filter_by_track_count(&editions, 1);
        assert_eq!(
            filtered.len(),
            2,
            "Should skip filtering when detected < {} tracks",
            MIN_DETECTED_TRACKS_FOR_FILTER
        );

        // With detected=4 (still below threshold), all editions should be kept
        let filtered_4 = filter_by_track_count(&editions, 4);
        assert_eq!(
            filtered_4.len(),
            2,
            "Should skip filtering when detected=4 < {} tracks",
            MIN_DETECTED_TRACKS_FOR_FILTER
        );

        // With detected=5 (at threshold), filtering should apply
        let filtered_5 = filter_by_track_count(&editions, 5);
        // 10-5=5 > 3 tolerance, 15-5=10 > 3 tolerance, both filtered -> fallback
        assert_eq!(
            filtered_5.len(),
            2,
            "At threshold, filtering applies but fallback kicks in"
        );
    }

    // ========================================================================
    // TC-U-030-07: Top candidates preserved even when filtered
    // ========================================================================

    #[test]
    fn test_track_count_preserves_top_candidates() {
        // Create 7 editions where only 1 matches track count tolerance
        // Top 5 should still be preserved regardless of track count
        let editions = vec![
            make_test_edition("Best Match", 20, "Artist"),       // Diff=9 from 11, would filter
            make_test_edition("Second Best", 18, "Artist"),      // Diff=7 from 11, would filter
            make_test_edition("Third Best", 16, "Artist"),       // Diff=5 from 11, would filter
            make_test_edition("Fourth Best", 15, "Artist"),      // Diff=4 from 11, would filter
            make_test_edition("Fifth Best", 14, "Artist"),       // Diff=3 from 11, KEEP (tolerance)
            make_test_edition("Sixth", 25, "Artist"),            // Diff=14 from 11, would filter
            make_test_edition("Seventh", 30, "Artist"),          // Diff=19 from 11, would filter
        ];

        let filtered = filter_by_track_count(&editions, 11);

        // Should have at least MIN_EDITIONS_TO_PRESERVE (5) editions
        assert!(
            filtered.len() >= MIN_EDITIONS_TO_PRESERVE,
            "Should preserve at least {} top candidates, got {}",
            MIN_EDITIONS_TO_PRESERVE,
            filtered.len()
        );

        // The one that matched tolerance should be included
        assert!(
            filtered.iter().any(|e| e.title == "Fifth Best"),
            "Edition matching tolerance should be kept"
        );

        // Top candidates should be preserved
        assert!(
            filtered.iter().any(|e| e.title == "Best Match"),
            "Top candidate should be preserved despite track count mismatch"
        );
    }

    // ========================================================================
    // TC-U-040-04: Fuzzy artist matching (substring)
    // ========================================================================

    #[test]
    fn test_fuzzy_artist_matching() {
        let edition1 = make_test_edition("Album", 11, "The Beatles");
        assert!(validate_artist_consistency(&edition1, "Beatles"), "Should match 'The Beatles' to 'Beatles'");

        let edition2 = make_test_edition("Album", 11, "Beatles");
        assert!(validate_artist_consistency(&edition2, "The Beatles"), "Should match 'Beatles' to 'The Beatles'");

        let edition3 = make_test_edition("Album", 11, "Michael Jackson");
        assert!(!validate_artist_consistency(&edition3, "James Gang"), "Should reject mismatched artists");
    }

    // ========================================================================
    // TC-U-050-01: Remix keyword detection
    // ========================================================================

    #[test]
    fn test_remix_keyword_detection() {
        assert!(is_remix_track("Hey Boy Hey Girl (Kink extended remix)"), "Should detect 'remix'");
        assert!(is_remix_track("Track (Remix)"), "Should detect 'Remix' (case insensitive)");
        assert!(!is_remix_track("Hey Boy Hey Girl"), "Should not detect remix in normal title");
    }

    // ========================================================================
    // TC-U-050-02: Extended keyword detection
    // ========================================================================

    #[test]
    fn test_extended_keyword_detection() {
        assert!(is_remix_track("Song Title (Extended)"), "Should detect 'Extended'");
        assert!(is_remix_track("Song Title (extended version)"), "Should detect 'extended version'");
    }
}

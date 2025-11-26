//! # Edition Processing
//!
//! Edition filtering, scoring, sorting, and winner selection with artist/album verification.
//!
//! ## Features
//! - **Runtime filtering**: Keep only editions within ±25% of file duration
//! - **Edition scoring**: Runtime delta + track count penalty (60s per track diff)
//! - **Name similarity re-sorting**: Bubble sort to promote editions with better name matches
//! - **Winner selection**: Time-fit primary, with artist/album fallback for name mismatches
//! - **Track count penalty**: Extra tracks penalized 4% per track (Run 27)
//!
//! ## Edition Selection Algorithm (Run 22/24/27)
//! 1. Sort editions by time-fit (adjusted match% with track count penalty, then mean error)
//! 2. Check if winner has acceptable artist AND album similarity (>= thresholds)
//! 3. If not, evaluate top 25% (min 3) runner-ups sorted by time-fit
//! 4. For each runner-up:
//!    - Name fit "significantly better": combined similarity improvement
//!    - Time-fit "not significantly worse": time_fit_delta >= -0.5
//! 5. First qualifying runner-up becomes new winner
//! 6. If no runner-up qualifies, keep original winner with warning
//!
//! ## Track Count Penalty (Run 27)
//! Editions with extra tracks beyond expected count receive match% penalty:
//! - Penalty = 4.0% per extra track (TRACK_COUNT_PENALTY_PER_TRACK)
//! - Example: 17-track edition (5 extra) needs 20% better match to beat 12-track edition
//! - Applied when estimated_track_count available from single-track discriminator
//!
//! ## Related Modules
//! - `types`: Edition, EditionTestResult
//! - `validation`: verify_artist_match(), verify_album_match(), normalize_artist_name(), levenshtein_ratio()
//! - `constants`: SCORE_TRACK_COUNT_PENALTY, NAME_DISTANCE_SWAP_RATIO, ARTIST_FALLBACK_*, TRACK_COUNT_PENALTY_PER_TRACK

use crate::constants::*;
use crate::types::{Edition, EditionTestResult};
use crate::matching::validation::{verify_artist_match, verify_album_match, normalize_artist_name, levenshtein_ratio};
use tracing::{info, warn};
use std::cmp::Ordering;

// =============================================================================
// Edition Scoring and Sorting
// =============================================================================

/// F64 comparison helper for sorting
///
/// Eliminates repeated `.partial_cmp(&x).unwrap_or(Ordering::Equal)` pattern.
/// Treats NaN as equal to any value (returns Ordering::Equal).
///
/// # Arguments
/// * `a` - First f64 value
/// * `b` - Second f64 value
///
/// # Returns
/// Ordering::Less if a < b, Ordering::Greater if a > b, Ordering::Equal otherwise
///
/// # Example
/// ```ignore
/// editions.sort_by(|a, b| cmp_f64(a.score, b.score));
/// ```
#[inline]
pub(crate) fn cmp_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

/// Score an edition's match quality against audio file characteristics
///
/// Lower score = better match (closer to audio file in runtime and track count).
/// Each track count difference = 60 seconds of runtime error (SCORE_TRACK_COUNT_PENALTY).
///
/// # Arguments
/// * `edition` - MusicBrainz edition to score
/// * `file_duration_secs` - Actual audio file duration (seconds)
/// * `estimated_track_count` - Estimated track count from single-track discriminator (None if unknown)
///
/// # Returns
/// Score value (lower = better). Sum of:
/// - Runtime difference: |edition_duration - file_duration|
/// - Track count penalty: |edition_tracks - file_tracks| × 60 seconds
///
/// # Example
/// ```ignore
/// let score = score_edition_match(&edition, 616.0, Some(12));
/// // Edition: 615s, 12 tracks → score = 1.0 (runtime) + 0.0 (tracks) = 1.0
/// // Edition: 610s, 15 tracks → score = 6.0 (runtime) + 180.0 (3×60 tracks) = 186.0
/// ```
pub(crate) fn score_edition_match(
    edition: &Edition,
    file_duration_secs: f64,
    estimated_track_count: Option<usize>,
) -> f64 {
    let edition_duration_secs: u32 = edition.durations.iter().sum();
    let mut score = (edition_duration_secs as f64 - file_duration_secs).abs();

    // Add penalty for track count difference (60 seconds per track)
    if let Some(file_tracks) = estimated_track_count {
        let track_diff = (edition.track_count as i32 - file_tracks as i32).abs();
        score += track_diff as f64 * SCORE_TRACK_COUNT_PENALTY;
    }

    score
}

/// Re-sort editions using conservative bubble sort based on name similarity
///
/// After editions are sorted by runtime likelihood, this performs a secondary
/// sort that allows editions with significantly better name matches to bubble up,
/// while preserving the runtime-based ordering for editions with similar name scores.
///
/// # Algorithm
/// Repeatedly scan the list. If item[i] has a name_distance_score more
/// than NAME_DISTANCE_SWAP_RATIO (1.732) times the score of item[i+1], swap them.
/// Continue until a full pass has zero swaps. This ensures only significantly
/// better name matches override the runtime ordering.
///
/// # Arguments
/// * `editions` - Mutable slice of editions to re-sort in place
///
/// # Example
/// ```ignore
/// let mut editions = vec![...];  // Sorted by runtime
/// resort_by_name_similarity(&mut editions);
/// // Editions with much better name matches now appear earlier
/// ```
pub(crate) fn resort_by_name_similarity(editions: &mut [Edition]) {
    if editions.len() < 2 {
        return;
    }

    loop {
        let mut swaps = 0;
        for i in 0..editions.len() - 1 {
            // Lower name_distance_score is better
            // Swap if current item's score is more than NAME_DISTANCE_SWAP_RATIO times worse than next item's score
            if editions[i].name_distance_score > NAME_DISTANCE_SWAP_RATIO * editions[i + 1].name_distance_score {
                editions.swap(i, i + 1);
                swaps += 1;
            }
        }
        if swaps == 0 {
            break;
        }
    }
}

/// Group MusicBrainz releases into unique editions by track durations
///
/// Multiple releases with identical track durations (same duration signature)
/// are grouped into a single Edition with multiple MBIDs. This reduces redundant
/// processing of equivalent releases.
///
/// # Arguments
/// * `releases` - List of (durations, recording_mbids, mbid_info, artist, album, rank, score) tuples
/// * `album_idx` - Album index for logging (0-based)
///
/// # Returns
/// Vec of unique editions, sorted by track count
///
/// # Duration Signature
/// Format: "track_count:duration1,duration2,..."
/// Example: "12:180,200,195,..." for 12-track album
///
/// Multiple releases with same signature share one Edition but have different MBIDs.
///
/// # Name Distance Rank/Score
/// When multiple releases merge into one edition, the edition keeps the best
/// (lowest) name_distance_rank and its corresponding score. This ensures the
/// edition inherits the best name match from its constituent releases.
///
/// # Example
/// ```ignore
/// // Two releases with identical durations:
/// // Release A: MBID-A, rank=1, score=5.0
/// // Release B: MBID-B, rank=3, score=8.0
/// // Result: One edition with [MBID-A, MBID-B], rank=1, score=5.0
/// let editions = group_into_editions(releases, album_idx);
/// ```
pub(crate) fn group_into_editions(
    releases: Vec<(Vec<u32>, Vec<String>, crate::types::EditionMBID, String, String, usize, f64)>,
    album_idx: usize
) -> Vec<Edition> {
    let mut editions: Vec<Edition> = Vec::new();

    for (durations, recording_mbids, mbid_info, artist, album, rank, score) in releases {
        // Create signature: "track_count:duration1,duration2,..."
        let signature = format!("{}:{}",
            durations.len(),
            durations.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(",")
        );

        // Find existing edition with this signature
        if let Some(edition) = editions.iter_mut().find(|e| e.duration_signature == signature) {
            // Add this MBID to existing edition
            edition.mbids.push(mbid_info);
            // Update to best (lowest) rank and corresponding score
            if rank < edition.name_distance_rank {
                edition.name_distance_rank = rank;
                edition.name_distance_score = score;
            }
        } else {
            // Create new edition
            editions.push(Edition {
                track_count: durations.len(),
                durations: durations.clone(),
                recording_mbids,
                mbids: vec![mbid_info],
                duration_signature: signature,
                artist,
                album,
                name_distance_rank: rank,
                name_distance_score: score,
            });
        }
    }

    info!("[A{}]   Grouped into {} unique editions", album_idx + 1, editions.len());

    // Sort editions by track count (helps with display)
    editions.sort_by_key(|e| e.track_count);

    editions
}

// =============================================================================
// Edition Filtering and Sorting
// =============================================================================

/// Filter editions by runtime and re-sort by name similarity
///
/// Applies runtime filtering (±25% of file duration) and then re-sorts by name similarity
/// using conservative bubble sort. Returns filtered and sorted editions.
///
/// # Arguments
/// * `editions` - List of MusicBrainz editions (already sorted by score)
/// * `file_duration_secs` - Audio file duration (seconds)
/// * `estimated_track_count` - Estimated track count from discriminator (None if unknown)
/// * `album_idx` - Album index for logging (0-based)
///
/// # Returns
/// * `Ok(filtered_editions)` - Filtered and sorted editions
/// * `Err(message)` - Error message if all editions filtered out
///
/// # Runtime Filter
/// - Minimum duration: file_duration × 0.75 (RUNTIME_FILTER_MIN_RATIO)
/// - Maximum duration: file_duration × 1.25 (RUNTIME_FILTER_MAX_RATIO)
/// - Editions outside this range are removed
///
/// # Algorithm
/// 1. Sort editions by score (runtime delta + track count penalty)
/// 2. Filter editions by runtime length (must be within ±25%)
/// 3. Re-sort by name similarity using bubble sort
/// 4. Return filtered/sorted editions or error if none remain
///
/// # Example
/// ```ignore
/// let editions = filter_and_sort_editions(
///     editions,
///     616.0,  // file duration
///     Some(12),  // estimated track count
///     0  // album index
/// )?;
/// // Returns editions within 462-770 seconds, sorted by name similarity
/// ```
pub(crate) fn filter_and_sort_editions(
    mut editions: Vec<Edition>,
    file_duration_secs: f64,
    estimated_track_count: Option<usize>,
    album_idx: usize,
) -> Result<Vec<Edition>, String> {
    // Sort editions by how well they match the audio file's characteristics
    editions.sort_by(|a, b| {
        let score_a = score_edition_match(a, file_duration_secs, estimated_track_count);
        let score_b = score_edition_match(b, file_duration_secs, estimated_track_count);
        cmp_f64(score_a, score_b)
    });

    info!("[A{}]   Sorted editions by likelihood (file: {:.0}s)",
        album_idx + 1, file_duration_secs);

    // Filter editions by runtime length (must be within 25% of file duration)
    let total_editions = editions.len();
    let min_duration = file_duration_secs * RUNTIME_FILTER_MIN_RATIO;
    let max_duration = file_duration_secs * RUNTIME_FILTER_MAX_RATIO;

    editions.retain(|edition| {
        let edition_duration: u32 = edition.durations.iter().sum();
        let edition_duration_secs = edition_duration as f64;
        edition_duration_secs >= min_duration && edition_duration_secs <= max_duration
    });

    let runtime_filtered_count = total_editions - editions.len();
    if runtime_filtered_count > 0 {
        info!("[A{}]   Filtered out {} editions (runtime >25% different from file)", album_idx + 1, runtime_filtered_count);
        info!("[A{}]   Acceptable range: {:.0}s - {:.0}s", album_idx + 1, min_duration, max_duration);
    }

    if editions.is_empty() {
        return Err(if runtime_filtered_count > 0 {
            format!("FAILED: All editions filtered out ({} by runtime)", runtime_filtered_count)
        } else {
            "FAILED: No valid editions (NDR filtering applied at release level)".to_string()
        });
    }

    // Re-sort by name similarity using conservative bubble sort
    resort_by_name_similarity(&mut editions);

    Ok(editions)
}

// =============================================================================
// Winner Selection with Artist/Album Verification
// =============================================================================

/// Find best edition with artist+album verification fallback (Run 22/24/27)
///
/// Selects the best edition from test results, with fallback logic to choose
/// runner-ups that have better name matches when the time-fit winner has
/// artist/album name mismatches.
///
/// # Arguments
/// * `edition_results` - Edition test results (from Stage 2-5 matching)
/// * `editions` - MusicBrainz editions metadata
/// * `source_artist` - Artist name from ID3 tags/path
/// * `source_album` - Album name from ID3 tags/path
/// * `estimated_track_count` - Estimated track count from discriminator (None if unknown)
/// * `album_idx` - Album index for logging (0-based)
///
/// # Returns
/// Tuple of (best_result, was_fallback_used, rejected_original_winner):
/// - best_result: Selected edition's test result (None if no valid results)
/// - was_fallback_used: true if runner-up promoted over original winner
/// - rejected_original_winner: Some((winner_idx, winner_pct, winner_artist, winner_artist_sim)) if fallback used
///
/// # Algorithm
/// 1. Sort all editions by time-fit (match% adjusted for track count, then mean error)
/// 2. Get winner (best by time-fit)
/// 3. Check if winner has acceptable artist AND album similarity
/// 4. If yes → return winner
/// 5. If no → evaluate top 25% (min 3) runner-ups:
///    - For each runner-up, check:
///      - Name fit "significantly better": combined similarity improvement
///      - Time-fit "not significantly worse": time_fit_delta >= -0.5
///    - First qualifying runner-up becomes new winner
/// 6. If no runner-up qualifies → return original winner with warning
///
/// # Track Count Penalty (Run 27)
/// Editions with extra tracks beyond expected count receive match% penalty:
/// - Penalty = TRACK_COUNT_PENALTY_PER_TRACK (4.0%) per extra track
/// - Example: 17-track edition (5 extra) → 20% penalty
/// - Applied when estimated_track_count available
///
/// # Time-Fit Delta Formula
/// ```ignore
/// time_fit_delta = 10 × (runner_pct/winner_pct - 1) + 1 × (1 - runner_error/winner_error)
/// ```
/// - Acceptable if >= TIME_FIT_DELTA_THRESHOLD (-0.5)
/// - Heavily weights match percentage (10×), lightly weights error (1×)
///
/// # Example
/// ```ignore
/// let (best, fallback_used, rejected) = find_best_edition_result_with_artist_check(
///     &edition_results,
///     &editions,
///     "The Beatles",
///     "Abbey Road",
///     Some(12),
///     0
/// );
/// if fallback_used {
///     info!("Promoted runner-up due to better name match");
/// }
/// ```
pub(crate) fn find_best_edition_result_with_artist_check<'a>(
    edition_results: &'a [EditionTestResult],
    editions: &[Edition],
    source_artist: &str,
    source_album: &str,
    estimated_track_count: Option<usize>,
    album_idx: usize,
) -> (Option<&'a EditionTestResult>, bool, Option<(usize, f64, String, f64)>) {
    // Step 1: Sort all editions by time-fit (match% adjusted for track count, then mean error)
    let mut sorted_results: Vec<&EditionTestResult> = edition_results
        .iter()
        .filter(|r| r.best_result.is_some())
        .collect();

    sorted_results.sort_by(|a, b| {
        // Run 27: Calculate adjusted match% penalizing extra tracks
        let a_adjusted_pct = if let Some(expected_tracks) = estimated_track_count {
            let a_edition = &editions[a.edition_idx];
            let a_extra_tracks = if a_edition.track_count > expected_tracks {
                a_edition.track_count - expected_tracks
            } else {
                0
            };
            a.best_percentage - (a_extra_tracks as f64 * TRACK_COUNT_PENALTY_PER_TRACK)
        } else {
            a.best_percentage
        };

        let b_adjusted_pct = if let Some(expected_tracks) = estimated_track_count {
            let b_edition = &editions[b.edition_idx];
            let b_extra_tracks = if b_edition.track_count > expected_tracks {
                b_edition.track_count - expected_tracks
            } else {
                0
            };
            b.best_percentage - (b_extra_tracks as f64 * TRACK_COUNT_PENALTY_PER_TRACK)
        } else {
            b.best_percentage
        };

        // Primary: higher adjusted match% is better
        let pct_cmp = b_adjusted_pct.partial_cmp(&a_adjusted_pct)
            .unwrap_or(Ordering::Equal);
        if pct_cmp != Ordering::Equal {
            return pct_cmp;
        }
        // Secondary: lower mean error is better
        let a_error = a.best_result.as_ref().map(|r| r.mean_error).unwrap_or(f64::MAX);
        let b_error = b.best_result.as_ref().map(|r| r.mean_error).unwrap_or(f64::MAX);
        cmp_f64(a_error, b_error)
    });

    if sorted_results.is_empty() {
        return (None, false, None);
    }

    // Run 27: Log track count penalties if applied
    if let Some(expected_tracks) = estimated_track_count {
        let penalties: Vec<_> = sorted_results.iter().take(3).filter_map(|r| {
            let edition = &editions[r.edition_idx];
            if edition.track_count > expected_tracks {
                let extra = edition.track_count - expected_tracks;
                let penalty = extra as f64 * TRACK_COUNT_PENALTY_PER_TRACK;
                Some((edition.track_count, extra, penalty, r.best_percentage))
            } else {
                None
            }
        }).collect();

        if !penalties.is_empty() {
            info!("[A{}]   Run 27: Track count penalties applied (expected: {} tracks)", album_idx + 1, expected_tracks);
            for (tracks, extra, penalty, orig_pct) in penalties {
                info!("[A{}]       {} tracks ({} extra) → {:.1}% penalty, adjusted {:.1}% → {:.1}%",
                    album_idx + 1, tracks, extra, penalty, orig_pct, orig_pct - penalty);
            }
        }
    }

    // Step 2: Get the winner (best by time-fit, adjusted for track count)
    let winner = sorted_results[0];
    let winner_idx = winner.edition_idx;

    if winner_idx >= editions.len() {
        return (Some(winner), false, None);
    }

    let winner_artist = &editions[winner_idx].artist;
    let winner_album = &editions[winner_idx].album;
    let (winner_artist_sim, winner_artist_ok) = verify_artist_match(source_artist, winner_artist);
    let (winner_album_sim, winner_album_ok) = verify_album_match(source_album, winner_album);
    let winner_pct = winner.best_percentage;
    let winner_error = winner.best_result.as_ref().map(|r| r.mean_error).unwrap_or(f64::MAX);

    // Run 24: Check both artist AND album similarity
    let winner_name_ok = winner_artist_ok && winner_album_ok;

    // Step 3: If winner has acceptable artist AND album match, use it
    if winner_name_ok {
        // Log successful match with both JW and Levenshtein ratio scores
        let norm_source_artist = normalize_artist_name(source_artist);
        let norm_winner_artist = normalize_artist_name(winner_artist);
        let artist_lev_ratio = levenshtein_ratio(&norm_source_artist, &norm_winner_artist);

        // Normalize album names (same logic as verify_album_match)
        let normalize_album = |s: &str| -> String {
            let lower = s.to_lowercase();
            lower.split('(').next().unwrap_or(&lower).trim().to_string()
        };
        let norm_source_album = normalize_album(source_album);
        let norm_winner_album = normalize_album(winner_album);
        let album_lev_ratio = levenshtein_ratio(&norm_source_album, &norm_winner_album);

        info!("[A{}]   ✅ Name match OK:", album_idx + 1);
        info!("[A{}]       Artist: '{}'→'{}' (JW={:.1}%, Lev={:.1}%)",
            album_idx + 1, source_artist, winner_artist, winner_artist_sim * 100.0, artist_lev_ratio * 100.0);
        info!("[A{}]       Album: '{}'→'{}' (JW={:.1}%, Lev={:.1}%)",
            album_idx + 1, source_album, winner_album, winner_album_sim * 100.0, album_lev_ratio * 100.0);

        return (Some(winner), false, None);
    }

    // Step 4: Winner has name mismatch - evaluate runner-ups
    // Calculate Levenshtein ratios for mismatch logging
    let norm_source_artist = normalize_artist_name(source_artist);
    let norm_winner_artist = normalize_artist_name(winner_artist);
    let artist_lev_ratio = levenshtein_ratio(&norm_source_artist, &norm_winner_artist);

    let normalize_album = |s: &str| -> String {
        let lower = s.to_lowercase();
        lower.split('(').next().unwrap_or(&lower).trim().to_string()
    };
    let norm_source_album = normalize_album(source_album);
    let norm_winner_album = normalize_album(winner_album);
    let album_lev_ratio = levenshtein_ratio(&norm_source_album, &norm_winner_album);

    let mismatch_type = match (winner_artist_ok, winner_album_ok) {
        (false, false) => "Artist+Album",
        (false, true) => "Artist",
        (true, false) => "Album",
        (true, true) => unreachable!(),
    };
    info!("[A{}]   🔍 {} mismatch detected:", album_idx + 1, mismatch_type);
    info!("[A{}]       Artist: '{}'→'{}' (JW={:.1}%, Lev={:.1}%, ok={})",
        album_idx + 1, source_artist, winner_artist, winner_artist_sim * 100.0, artist_lev_ratio * 100.0, winner_artist_ok);
    info!("[A{}]       Album: '{}'→'{}' (JW={:.1}%, Lev={:.1}%, ok={})",
        album_idx + 1, source_album, winner_album, winner_album_sim * 100.0, album_lev_ratio * 100.0, winner_album_ok);
    info!("[A{}]       Evaluating runner-ups...", album_idx + 1);

    // Calculate how many runner-ups to evaluate: top 25%, minimum 3
    let num_candidates = ((sorted_results.len() as f64 * ARTIST_FALLBACK_TOP_PCT).ceil() as usize)
        .max(ARTIST_FALLBACK_MIN_CANDIDATES)
        .min(sorted_results.len());

    info!("[A{}]       Evaluating top {} of {} editions", album_idx + 1, num_candidates, sorted_results.len());

    // Combined similarity for winner (average of artist and album)
    let winner_combined_sim = (winner_artist_sim + winner_album_sim) / 2.0;

    // Step 5: Check each runner-up
    for (rank, runner) in sorted_results.iter().enumerate().skip(1).take(num_candidates - 1) {
        let runner_idx = runner.edition_idx;
        if runner_idx >= editions.len() {
            continue;
        }

        let runner_artist = &editions[runner_idx].artist;
        let runner_album = &editions[runner_idx].album;
        let (runner_artist_sim, runner_artist_ok) = verify_artist_match(source_artist, runner_artist);
        let (runner_album_sim, runner_album_ok) = verify_album_match(source_album, runner_album);
        let runner_pct = runner.best_percentage;
        let runner_error = runner.best_result.as_ref().map(|r| r.mean_error).unwrap_or(f64::MAX);

        // Run 24: Combined similarity (average of artist and album)
        let runner_combined_sim = (runner_artist_sim + runner_album_sim) / 2.0;

        // Check name fit: "significantly better"
        // Condition: runner must have both artist AND album ok, OR significantly better combined similarity
        let runner_name_ok = runner_artist_ok && runner_album_ok;
        let meets_min_similarity = runner_combined_sim >= ARTIST_FALLBACK_MIN_SIMILARITY;
        let meets_delta = runner_combined_sim > winner_combined_sim + ARTIST_FALLBACK_DELTA;
        let meets_ratio = runner_combined_sim > winner_combined_sim * ARTIST_FALLBACK_RATIO;
        let name_significantly_better = runner_name_ok || (meets_min_similarity && (meets_delta || meets_ratio));

        // Check time fit: "not significantly worse"
        // Formula: 10 × (runner_pct/winner_pct - 1) + 1 × (1 - runner_error/winner_error)
        let pct_ratio_term = 10.0 * (runner_pct / winner_pct - 1.0);
        let error_ratio_term = if winner_error > 0.0 && runner_error > 0.0 {
            1.0 * (1.0 - runner_error / winner_error)
        } else {
            0.0 // Can't compare errors if one is invalid
        };
        let time_fit_delta = pct_ratio_term + error_ratio_term;
        let time_fit_acceptable = time_fit_delta >= TIME_FIT_DELTA_THRESHOLD;

        // Calculate Levenshtein ratios for runner-up logging
        let runner_norm_artist = normalize_artist_name(runner_artist);
        let runner_artist_lev = levenshtein_ratio(&norm_source_artist, &runner_norm_artist);
        let runner_norm_album = normalize_album(runner_album);
        let runner_album_lev = levenshtein_ratio(&norm_source_album, &runner_norm_album);

        info!("[A{}]       Runner #{}: '{}' - '{}'",
            album_idx + 1, rank + 1, runner_artist, runner_album);
        info!("[A{}]         Artist: JW={:.1}%, Lev={:.1}%, ok={}",
            album_idx + 1, runner_artist_sim * 100.0, runner_artist_lev * 100.0, runner_artist_ok);
        info!("[A{}]         Album: JW={:.1}%, Lev={:.1}%, ok={}",
            album_idx + 1, runner_album_sim * 100.0, runner_album_lev * 100.0, runner_album_ok);
        info!("[A{}]         Combined: {:.1}% (winner: {:.1}%), name_ok={}, better={}",
            album_idx + 1, runner_combined_sim * 100.0, winner_combined_sim * 100.0, runner_name_ok, name_significantly_better);
        info!("[A{}]         Time: pct={:.1}%, err={:.2}s, delta={:.3}, acceptable={}",
            album_idx + 1, runner_pct, runner_error, time_fit_delta, time_fit_acceptable);

        // If both conditions met, promote this runner-up
        if name_significantly_better && time_fit_acceptable {
            info!("[A{}]   🎯 Name fallback: Promoting runner-up #{} over winner", album_idx + 1, rank + 1);
            info!("[A{}]       Winner: '{}' - '{}' (artist={:.1}%, album={:.1}%, pct={:.1}%)",
                album_idx + 1, winner_artist, winner_album, winner_artist_sim * 100.0, winner_album_sim * 100.0, winner_pct);
            info!("[A{}]       Runner: '{}' - '{}' (artist={:.1}%, album={:.1}%, pct={:.1}%)",
                album_idx + 1, runner_artist, runner_album, runner_artist_sim * 100.0, runner_album_sim * 100.0, runner_pct);
            info!("[A{}]       Time-fit delta: {:.3} (threshold: {:.1})",
                album_idx + 1, time_fit_delta, TIME_FIT_DELTA_THRESHOLD);

            return (Some(runner), true, Some((winner_idx, winner_pct, winner_artist.clone(), winner_artist_sim)));
        }
    }

    // No suitable runner-up found - use original winner with warning
    warn!("[A{}]   ⚠️ No suitable name-matched runner-up found", album_idx + 1);
    warn!("[A{}]       Using original winner: '{}' - '{}' (artist={:.1}%, album={:.1}%, pct={:.1}%)",
        album_idx + 1, winner_artist, winner_album, winner_artist_sim * 100.0, winner_album_sim * 100.0, winner_pct);

    (Some(winner), false, None)
}

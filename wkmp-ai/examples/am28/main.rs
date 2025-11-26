//! # Album Matcher 28 - Main Entry Point
//!
//! CLI parsing, album orchestration, and parallel processing.

// Module declarations (main.rs is crate root for examples)
mod constants;
mod helpers;
mod matching;
mod metadata;
mod musicbrainz;
mod orchestration;
mod silence_detection;
mod single_track_discriminator;
mod stages;
mod types;
mod utils;

use std::collections::VecDeque;
use std::io::Write;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::stream::{self, StreamExt};
use rayon::prelude::*;
use time::UtcOffset;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use tracing_subscriber::fmt::time::OffsetTime;

use constants::*;
use helpers::{classify_confidence, make_error_result};
use utils::query_stats::{spawn_heartbeat_task, QueryStats};
use matching::candidate::analyze_track_matching;
use matching::edition::{
    cmp_f64, filter_and_sort_editions, find_best_edition_result_with_artist_check,
    group_into_editions, score_edition_match,
};
use matching::mbid::select_best_mbid;
use matching::validation::verify_artist_match;
use metadata::{extract_and_reconcile_metadata, log_reconciliation_decision};
use musicbrainz::api::{comprehensive_musicbrainz_search, MBClient, RateLimiter};
use musicbrainz::cache::print_cache_statistics;
use orchestration::test_single_edition;
use silence_detection::{get_track_durations, precompute_silence_cache};
use single_track_discriminator::SingleTrackDiscriminator;
use stages::stage4::calculate_rms_profile;
use types::*;
use utils::audio::decode_mp3;

// =============================================================================
// Process Single Album
// =============================================================================

/// Process a single album through all matching stages
///
/// This function orchestrates the complete album processing pipeline:
/// 1. Staggered start delay (for rate limit compliance)
/// 2. ID3 tag extraction and metadata reconciliation
/// 3. Single-track discriminator (pre-decode)
/// 4. Parallel: audio decode + MusicBrainz lookup
/// 5. Silence cache computation
/// 6. Single-track discriminator (post-decode update)
/// 7. Edition grouping and filtering
/// 8. Edition-by-edition testing through Stages 2-5
/// 9. Best result selection with artist/album verification
/// 10. Final result construction
pub(crate) async fn process_single_album(
    album_idx: usize,
    total_albums: usize,
    file_path: PathBuf,
    threshold_db: f64,
    min_duration_secs: f64,
    match_tolerance_secs: f64,
    threshold_values: &'static [f64],
    min_duration_values: &'static [f64],
    mb_client: Arc<MBClient>,
    _acoustid_api_key: Option<String>, // Reserved for AcoustID verification (Run 24)
) -> ValidationResult {
    // === STAGGERED START DELAY ===
    let stagger_position = album_idx % MAX_CONCURRENT_ALBUMS;
    if STAGGER_MULTIPLIER > 0 && stagger_position > 0 && album_idx < MAX_CONCURRENT_ALBUMS {
        let stagger_delay_ms = stagger_position as u64 * STAGGER_MULTIPLIER * MB_RATE_LIMIT_MS;
        let stagger_delay_secs = stagger_delay_ms / 1000;
        info!(
            "[A{}] Staggered start: waiting {}s ({} position × {} multiplier × {}ms rate limit)...",
            album_idx + 1,
            stagger_delay_secs,
            stagger_position,
            STAGGER_MULTIPLIER,
            MB_RATE_LIMIT_MS
        );
        sleep(Duration::from_millis(stagger_delay_ms)).await;
    }

    let album_id = format!("A{}", album_idx + 1);
    info!("[{}] === Album {}/{} ===", album_id, album_idx + 1, total_albums);
    info!("[{}] File: {}", album_id, file_path.display());

    if !file_path.exists() {
        error!("[{}]   ERROR: File not found\n", album_id);
        return ValidationResult::error(
            &file_path,
            UNKNOWN_VALUE,
            UNKNOWN_VALUE,
            0,
            "File not found".to_string(),
        );
    }

    // === PHASE 0: ID3 Tag Extraction & Reconciliation ===
    let reconciled = extract_and_reconcile_metadata(&file_path);
    log_reconciliation_decision(&reconciled, album_idx + 1);

    // === Single-Track Discriminator (Pre-Decode) ===
    let mut single_track_analysis = SingleTrackDiscriminator::analyze_pre_decode(&file_path, None);
    SingleTrackDiscriminator::log_pre_decode(&album_id, &single_track_analysis, &file_path);

    let artist = reconciled.artist.clone();
    let album = reconciled.album.clone();

    // Build artist/album variant lists for comprehensive search
    let mut artist_variants = vec![artist.clone()];
    if let Some(ref alt_artist) = reconciled.alternate_artist {
        if !artist_variants.contains(alt_artist) {
            artist_variants.push(alt_artist.clone());
        }
    }

    let mut album_variants = vec![album.clone()];
    if let Some(ref alt_album) = reconciled.alternate_album {
        if !album_variants.contains(alt_album) {
            album_variants.push(alt_album.clone());
        }
    }

    // === PARALLEL: DECODE + MUSICBRAINZ ===
    info!(
        "[{}]   Starting parallel: decode + MusicBrainz lookup...",
        album_id
    );

    let query_stats = Arc::new(QueryStats::new());
    let heartbeat_handle = spawn_heartbeat_task(Arc::clone(&query_stats), album_idx);

    // Spawn decode task (CPU-bound)
    let file_path_for_decode = file_path.clone();
    let album_idx_for_decode = album_idx;
    let decode_handle = tokio::task::spawn_blocking(move || {
        info!("[A{}]   Decoding...", album_idx_for_decode + 1);
        decode_mp3(&file_path_for_decode)
    });

    // Spawn MusicBrainz lookup concurrently
    let mb_task = comprehensive_musicbrainz_search(
        &*mb_client,
        &artist_variants,
        &album_variants,
        album_idx,
        Some(&query_stats),
    );

    // Wait for both to complete
    let (decode_result, mb_result) = tokio::join!(decode_handle, mb_task);

    // Process decode result
    let (samples, sample_rate) = match decode_result {
        Ok(Ok((s, sr))) => {
            let duration_mins = s.len() as f64 / sr as f64 / 60.0;
            info!(
                "[{}]   Decoded: {} samples at {} Hz ({:.2} mins)",
                album_id,
                s.len(),
                sr,
                duration_mins
            );
            (s, sr)
        }
        Ok(Err(e)) => {
            return make_error_result(
                &query_stats,
                &file_path,
                &artist,
                &album,
                0,
                &album_id,
                format!("Decode failed: {}", e),
            );
        }
        Err(e) => {
            return make_error_result(
                &query_stats,
                &file_path,
                &artist,
                &album,
                0,
                &album_id,
                format!("Decode task panicked: {}", e),
            );
        }
    };

    // Get initial track durations
    let initial_durations = get_track_durations(&samples, sample_rate, threshold_db, min_duration_secs);
    let file_duration_secs = samples.len() as f64 / sample_rate as f64;

    // === SILENCE DETECTION (after decode) ===
    let samples_for_silence = samples.clone();
    let album_idx_for_silence = album_idx;
    let threshold_values_clone = threshold_values.to_vec();
    let min_duration_values_clone = min_duration_values.to_vec();

    let silence_task = tokio::task::spawn_blocking(move || {
        info!(
            "[A{}]   Single-pass silence detection (1 scan → {} param combinations, {} threads)...",
            album_idx_for_silence + 1,
            threshold_values_clone.len() * min_duration_values_clone.len(),
            rayon::current_num_threads()
        );
        let cache = precompute_silence_cache(
            &samples_for_silence,
            sample_rate,
            &threshold_values_clone,
            &min_duration_values_clone,
        );
        info!(
            "[A{}]   Silence cache ready ({} entries)",
            album_idx_for_silence + 1,
            cache.len()
        );
        cache
    });

    let silence_cache = match silence_task.await {
        Ok(cache) => cache,
        Err(e) => {
            return make_error_result(
                &query_stats,
                &file_path,
                &artist,
                &album,
                initial_durations.len(),
                &album_id,
                format!("Silence detection failed: {}", e),
            );
        }
    };

    // === Single-Track Discriminator (Post-Decode Update) ===
    let duration_mins = file_duration_secs / 60.0;
    let gap_count = initial_durations.len().saturating_sub(1);
    SingleTrackDiscriminator::update_post_decode(&mut single_track_analysis, duration_mins, gap_count);
    SingleTrackDiscriminator::log_post_decode(&album_id, &single_track_analysis);

    // Process MusicBrainz result
    let editions = match mb_result {
        Ok(releases) => {
            if releases.is_empty() {
                return make_error_result(
                    &query_stats,
                    &file_path,
                    &artist,
                    &album,
                    initial_durations.len(),
                    &album_id,
                    "MusicBrainz lookup failed: No releases found".to_string(),
                );
            }

            let editions = group_into_editions(releases, album_idx);
            match filter_and_sort_editions(
                editions,
                file_duration_secs,
                reconciled.estimated_track_count,
                album_idx,
            ) {
                Ok(filtered) => {
                    info!("[{}] Found {} unique editions to test", album_id, filtered.len());
                    filtered
                }
                Err(failure_msg) => {
                    return make_error_result(
                        &query_stats,
                        &file_path,
                        &artist,
                        &album,
                        initial_durations.len(),
                        &album_id,
                        failure_msg,
                    );
                }
            }
        }
        Err(e) => {
            return make_error_result(
                &query_stats,
                &file_path,
                &artist,
                &album,
                initial_durations.len(),
                &album_id,
                format!("MusicBrainz lookup failed: {}", e),
            );
        }
    };

    // Display edition information
    info!("[{}]   Edition Details (sorted by match likelihood):", album_id);
    for (idx, edition) in editions.iter().enumerate() {
        let edition_duration: u32 = edition.durations.iter().sum();
        let match_score = score_edition_match(edition, file_duration_secs, reconciled.estimated_track_count);
        info!(
            "[A{}]     [{}] {} - {} ({} tracks, {}s, {} MBIDs, score: {:.0}s, NDR:{},{:.1})",
            album_idx + 1,
            idx,
            edition.artist,
            edition.album,
            edition.track_count,
            edition_duration,
            edition.mbids.len(),
            match_score,
            edition.name_distance_rank,
            edition.name_distance_score
        );
    }

    // === Edition-by-Edition Processing ===
    let rms_profile = calculate_rms_profile(&samples, sample_rate);
    let total_duration_secs = samples.len() as f64 / sample_rate as f64;

    let mut best_durations: Vec<f64> = initial_durations.clone();
    let mut best_matches: Vec<TrackMatch> = Vec::new();
    let mut best_matched_count: usize = 0;
    let mut best_percentage: f64 = 0.0;
    let mut best_stage = "none";
    let mut best_threshold: Option<f64> = None;
    let mut best_min_duration: Option<f64> = None;
    let mut best_mbid: String = String::new();
    let mut best_expected_durations: Vec<u32> = Vec::new();
    let mut best_mean_error: f64 = 0.0;
    let mut best_edition_idx: Option<usize> = None;

    let num_thresholds = threshold_values.len();
    let num_min_durations = min_duration_values.len();

    let perfect_match_found = AtomicBool::new(false);
    let perfect_match_time_ms = AtomicU64::new(0);
    let parallel_start_time = Instant::now();

    info!(
        "[{}]   === EDITION-BY-EDITION PROCESSING (Run 19 - PARALLEL MB+SILENCE) ===",
        album_id
    );
    info!(
        "[{}]   Testing {} editions through Stages 2-5 ({}s between feeds, {}s grace period)...\n",
        album_id,
        editions.len(),
        EDITION_FEED_DELAY_SECS,
        EARLY_EXIT_GRACE_PERIOD_SECS
    );

    query_stats.set_activity(&format!("testing {} editions", editions.len()));

    // Wrap shared data in Arc for rayon::spawn (no lifetime constraints like rayon::scope)
    let silence_cache = Arc::new(silence_cache);
    let rms_profile = Arc::new(rms_profile);
    let initial_durations = Arc::new(initial_durations);
    let perfect_match_found = Arc::new(perfect_match_found);
    let perfect_match_time_ms = Arc::new(perfect_match_time_ms);

    // Channel for collecting results (tokio channel for async-friendly receiving)
    let (result_tx, mut result_rx) = tokio::sync::mpsc::channel::<EditionTestResult>(editions.len().max(1));

    let mut editions_started = 0;
    let mut editions_skipped = 0;
    let total_editions = editions.len();

    // Spawn editions onto rayon WITHOUT blocking scope - delay happens in async context
    for (edition_idx, edition) in editions.iter().enumerate() {
        if perfect_match_found.load(Ordering::Relaxed) {
            editions_skipped = editions.len() - edition_idx;
            info!(
                "[{}]   Stopping feed: 100% match found, {} editions not started",
                album_id, editions_skipped
            );
            break;
        }

        editions_started += 1;
        query_stats.set_activity(&format!(
            "edition {}/{}: {}",
            edition_idx + 1,
            editions.len(),
            edition.album
        ));

        info!(
            "[A{}]   Starting Edition {}/{}: {} - {}",
            album_idx + 1,
            edition_idx + 1,
            editions.len(),
            edition.artist,
            edition.album
        );

        // Clone Arc handles for this spawn
        let silence_cache = Arc::clone(&silence_cache);
        let rms_profile = Arc::clone(&rms_profile);
        let initial_durations = Arc::clone(&initial_durations);
        let perfect_match_found = Arc::clone(&perfect_match_found);
        let perfect_match_time_ms = Arc::clone(&perfect_match_time_ms);
        let result_tx = result_tx.clone();
        let edition = edition.clone();

        rayon::spawn(move || {
            let edition_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                test_single_edition(
                    edition_idx,
                    &edition,
                    &silence_cache,
                    num_thresholds,
                    num_min_durations,
                    match_tolerance_secs,
                    &rms_profile,
                    total_duration_secs,
                    &initial_durations,
                    total_editions,
                    &perfect_match_found,
                    &perfect_match_time_ms,
                    parallel_start_time,
                    album_idx,
                )
            }));

            match edition_result {
                Ok(result) => {
                    let _ = result_tx.blocking_send(result);
                }
                Err(panic_payload) => {
                    let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    error!(
                        "[A{}] Edition {}/{} PANICKED: {}",
                        album_idx + 1,
                        edition_idx + 1,
                        total_editions,
                        panic_msg
                    );
                }
            }
        });

        // Delay OUTSIDE rayon - async sleep doesn't block rayon threads
        if edition_idx < editions.len() - 1 {
            sleep(Duration::from_secs(EDITION_FEED_DELAY_SECS)).await;
        }
    }

    // Drop our sender so the channel closes when all spawned tasks complete
    drop(result_tx);

    // Collect results asynchronously (doesn't block rayon threads)
    let mut edition_results: Vec<EditionTestResult> = Vec::with_capacity(editions_started);
    while let Some(result) = result_rx.recv().await {
        edition_results.push(result);
    }

    query_stats.stop();
    let _ = heartbeat_handle.await;

    edition_results.sort_by_key(|r| r.edition_idx);

    info!(
        "[{}]   Completed: {} editions started, {} skipped",
        album_id, editions_started, editions_skipped
    );

    for result in &edition_results {
        for msg in &result.log_messages {
            debug!("[{}] {}", album_id, msg);
        }
        debug!("[{}] ", album_id);
    }

    // Find best result with artist+album verification
    let (best_result_opt, used_artist_fallback, rejected_match) =
        find_best_edition_result_with_artist_check(
            &edition_results,
            &editions,
            &artist,
            &album,
            reconciled.estimated_track_count,
            album_idx,
        );

    if let Some(best_edition_result) = best_result_opt {
        if let Some(ref result) = best_edition_result.best_result {
            best_durations = result.detected_durations.clone();
            best_matches = result.matches.clone();
            best_matched_count = result.matched_count;
            best_percentage = result.percentage;
            best_stage = best_edition_result.best_stage;
            best_threshold = best_edition_result.best_threshold;
            best_min_duration = best_edition_result.best_min_duration;
            best_mbid = format!("edition_{}", best_edition_result.edition_idx);
            best_expected_durations = best_edition_result.expected_durations.clone();
            best_mean_error = result.mean_error;
            best_edition_idx = Some(best_edition_result.edition_idx);
        }
    }

    let perfect_match_count = edition_results
        .iter()
        .filter(|r| r.best_percentage >= 100.0)
        .count();

    if perfect_match_count > 1 {
        info!(
            "[A{}]   Found {} editions with 100% match - selected edition {} (lowest mean error: {:.2}s)",
            album_idx + 1,
            perfect_match_count,
            best_edition_idx.map(|i| i + 1).unwrap_or(0),
            best_mean_error
        );
    } else {
        info!(
            "[A{}]   Parallel processing complete. Best: {:.1}% from edition {} (mean error: {:.2}s)",
            album_idx + 1,
            best_percentage,
            best_edition_idx.map(|i| i + 1).unwrap_or(0),
            best_mean_error
        );
    }

    // Convert placeholder MBID to real MBID
    if best_mbid.starts_with("edition_") {
        if let Some(idx) = best_edition_idx {
            if idx < editions.len() {
                best_mbid = select_best_mbid(&editions[idx]);
            }
        }
    }

    let perfect_count = best_durations.len() == best_expected_durations.len();

    let (winning_artist, winning_album) = if let Some(idx) = best_edition_idx {
        if idx < editions.len() {
            (editions[idx].artist.clone(), editions[idx].album.clone())
        } else {
            (UNKNOWN_VALUE.to_string(), UNKNOWN_VALUE.to_string())
        }
    } else {
        (UNKNOWN_VALUE.to_string(), UNKNOWN_VALUE.to_string())
    };

    let (artist_similarity, artist_match_ok) = verify_artist_match(&artist, &winning_artist);
    let artist_mismatch = !artist_match_ok;

    if artist_mismatch {
        warn!("[{}]   ⚠️  ARTIST MISMATCH in final result:", album_id);
        warn!("[{}]       Source artist: '{}'", album_id, artist);
        warn!("[{}]       Matched artist: '{}'", album_id, winning_artist);
        warn!(
            "[A{}]       Similarity: {:.1}% (threshold: {:.1}%)",
            album_idx + 1,
            artist_similarity * 100.0,
            ARTIST_MISMATCH_THRESHOLD * 100.0
        );

        if best_percentage < ARTIST_MISMATCH_MIN_MATCH_PCT {
            warn!(
                "[A{}]       ❌ LOW CONFIDENCE: {:.1}% track match < {:.1}% threshold",
                album_idx + 1,
                best_percentage,
                ARTIST_MISMATCH_MIN_MATCH_PCT
            );
        }
    } else if used_artist_fallback {
        info!(
            "[A{}]   ✅ Artist verification: Matched (fallback was used)",
            album_idx + 1
        );
    }

    if let Some((rejected_idx, rejected_pct, rejected_artist, rejected_sim)) = &rejected_match {
        info!(
            "[A{}]   📊 Rejected original winner (artist mismatch):",
            album_idx + 1
        );
        info!(
            "[A{}]       Edition {}: '{}' (sim={:.1}%, pct={:.1}%)",
            album_idx + 1,
            rejected_idx + 1,
            rejected_artist,
            rejected_sim * 100.0,
            rejected_pct
        );
    }

    info!("[A{}]   FINAL RESULT:", album_idx + 1);
    info!("[A{}]     Artist: {}", album_idx + 1, winning_artist);
    info!("[A{}]     Album: {}", album_idx + 1, winning_album);
    info!("[A{}]     Matching stage: {}", album_idx + 1, best_stage);
    info!(
        "[A{}]     MusicBrainz: https://musicbrainz.org/release/{}",
        album_idx + 1,
        best_mbid
    );
    info!(
        "[A{}]     Track count: {}/{} {}",
        album_idx + 1,
        best_durations.len(),
        best_expected_durations.len(),
        if perfect_count { "✓" } else { "✗" }
    );
    info!(
        "[A{}]     Matched tracks: {}/{} ({:.1}%)",
        album_idx + 1,
        best_matched_count,
        best_expected_durations.len(),
        best_percentage
    );
    info!("[A{}]     Mean error: {:.2}s", album_idx + 1, best_mean_error);
    info!(
        "[A{}]     Confidence: {}",
        album_idx + 1,
        classify_confidence(best_percentage)
    );

    if let (Some(thresh), Some(min_dur)) = (best_threshold, best_min_duration) {
        info!(
            "[A{}]     Best parameters: {}dB, {}s",
            album_idx + 1, thresh, min_dur
        );
    }

    if !best_matches.is_empty() {
        info!("[A{}]   All tracks:", album_idx + 1);
        for (i, tm) in best_matches.iter().enumerate() {
            let status = if tm.matches { "✓" } else { "✗" };
            info!(
                "[A{}]     {}. {:6.1}s vs {:6}s  error={:5.1}s  {}",
                album_idx + 1,
                i + 1,
                tm.detected_duration,
                tm.expected_duration,
                tm.error,
                status
            );
        }
    }

    // Calculate extra tracks
    let mut extra_tracks = Vec::new();
    let min_count = best_durations.len().min(best_expected_durations.len());

    if best_durations.len() > best_expected_durations.len() {
        for i in min_count..best_durations.len() {
            extra_tracks.push(ExtraTrack {
                track_index: i + 1,
                duration: best_durations[i],
                description: "Extra track with no corresponding MusicBrainz entry".to_string(),
            });
        }

        if !extra_tracks.is_empty() {
            info!(
                "[A{}]   Extra tracks detected ({} beyond expected count):",
                album_idx + 1,
                extra_tracks.len()
            );
            for et in &extra_tracks {
                info!(
                    "[A{}]     Track {}: {:.1}s (no MusicBrainz match)",
                    album_idx + 1, et.track_index, et.duration
                );
            }
        }
    } else if best_durations.len() < best_expected_durations.len() {
        let missing_count = best_expected_durations.len() - best_durations.len();
        info!(
            "[A{}]   MusicBrainz tracks not found in file ({} missing):",
            album_idx + 1, missing_count
        );
        for i in min_count..best_expected_durations.len() {
            info!(
                "[A{}]     Track {}: {}s (expected but not detected in file)",
                album_idx + 1,
                i + 1,
                best_expected_durations[i]
            );
        }
    }

    info!("[A{}] ", album_idx + 1);

    // Determine final status
    let status = if artist_mismatch && best_percentage < ARTIST_MISMATCH_MIN_MATCH_PCT {
        "Artist Mismatch - Likely Incorrect".to_string()
    } else if artist_mismatch {
        "Artist Mismatch - Review Required".to_string()
    } else if used_artist_fallback {
        "Success (Artist Fallback Used)".to_string()
    } else {
        "Success".to_string()
    };

    // AcoustID verification disabled for now (Run 24 feature)
    let acoustid_verification = None;

    ValidationResult {
        album_path: file_path.to_string_lossy().to_string(),
        artist,
        album,
        mbid: best_mbid.clone(),
        musicbrainz_url: format!("https://musicbrainz.org/release/{}", best_mbid),
        expected_track_count: best_expected_durations.len(),
        detected_track_count: best_durations.len(),
        perfect_count_match: perfect_count,
        track_matches: best_matches,
        extra_tracks,
        matched_tracks_count: best_matched_count,
        match_percentage: best_percentage,
        mean_error: best_mean_error,
        status,
        matching_stage: best_stage.to_string(),
        best_threshold_db: best_threshold,
        best_min_duration_secs: best_min_duration,
        confidence: classify_confidence(best_percentage),
        matched_artist: winning_artist,
        matched_album: winning_album,
        artist_mismatch,
        artist_similarity,
        acoustid_verification,
    }
}

// =============================================================================
// Main Entry Point
// =============================================================================

/// Main entry point for album matcher
///
/// Parses CLI arguments, loads training set, and processes albums in parallel.
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get local time offset BEFORE any threads spawn
    let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);

    let timer = OffsetTime::new(
        local_offset,
        time::macros::format_description!(
            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6][offset_hour sign:mandatory]:[offset_minute]"
        ),
    );

    // Initialize tracing with local timestamps
    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_target(false)
        .with_level(true)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .init();

    // Install panic hook
    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic payload".to_string()
        };

        error!("PANIC at {}: {}", location, message);
        let _ = writeln!(std::io::stderr(), "\n!!! PANIC at {}: {}", location, message);
        let _ = std::io::stderr().flush();

        let backtrace = std::backtrace::Backtrace::capture();
        if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            let _ = writeln!(std::io::stderr(), "Backtrace:\n{}", backtrace);
            let _ = std::io::stderr().flush();
        }
    }));

    // Parse CLI arguments for cache mode and output file
    let args: Vec<String> = std::env::args().collect();
    let cache_mode = if args.iter().any(|arg| arg == "--no-cache") {
        CacheMode::Disabled
    } else if args.iter().any(|arg| arg == "--use-cache") {
        CacheMode::ReadOnly
    } else {
        CacheMode::ReadWrite
    };

    // Parse output filename parameter (--output or -o)
    let output_json = args
        .iter()
        .enumerate()
        .find_map(|(i, arg)| {
            if (arg == "--output" || arg == "-o") && i + 1 < args.len() {
                Some(args[i + 1].clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "album_matcher_results.json".to_string());

    let cache_config = CacheConfig {
        mode: cache_mode,
        cache_dir: PathBuf::from("./cache/musicbrainz"),
    };

    let cache_mode_str = match cache_mode {
        CacheMode::Disabled => "Disabled",
        CacheMode::ReadWrite => "ReadWrite (default)",
        CacheMode::ReadOnly => "ReadOnly",
    };

    info!("=== Comprehensive Album Matcher (Run 28 - Modular) ===");
    info!("Cache Mode: {}", cache_mode_str);
    info!("Output JSON: {}", output_json);

    // Read training set
    let training_set_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\training_set.txt");
    let long_files_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\long_files_list.txt");

    let training_content = std::fs::read_to_string(training_set_path)?;
    let mut training_files = Vec::new();

    for line in training_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(path_start) = line.find("] ") {
            let path_str = &line[path_start + 2..];
            training_files.push(PathBuf::from(path_str));
        } else {
            training_files.push(PathBuf::from(trimmed));
        }
    }

    let initial_count = training_files.len();

    // Read long files list
    let long_files_content = std::fs::read_to_string(long_files_path)?;
    let mut additional_files = Vec::new();

    let training_paths: std::collections::HashSet<PathBuf> =
        training_files.iter().cloned().collect();

    for line in long_files_content.lines() {
        if let Some(path_start) = line.find("] ") {
            let path_str = &line[path_start + 2..];
            let path = PathBuf::from(path_str);

            if !training_paths.contains(&path) {
                additional_files.push(path);
            }
        }
    }

    training_files.extend(additional_files.clone());

    // Pre-scan file sizes and interleave
    info!("=== Pre-scanning file sizes for load balancing ===");
    let prescan_start = std::time::Instant::now();

    let mut files_with_sizes: Vec<(PathBuf, u64)> = training_files
        .into_iter()
        .filter_map(|path| match std::fs::metadata(&path) {
            Ok(meta) => Some((path, meta.len())),
            Err(e) => {
                warn!("Could not get size for {}: {}", path.display(), e);
                Some((path, 0))
            }
        })
        .collect();

    files_with_sizes.sort_by_key(|(_, size)| *size);

    // Interleave: shortest, middle, longest, middle, repeat
    let mut remaining: VecDeque<(PathBuf, u64)> = files_with_sizes.iter().cloned().collect();
    let mut interleaved: Vec<PathBuf> = Vec::with_capacity(remaining.len());

    while !remaining.is_empty() {
        if let Some((path, _)) = remaining.pop_front() {
            interleaved.push(path);
        }
        if remaining.is_empty() {
            break;
        }

        let mid_idx = remaining.len() / 2;
        if let Some((path, _)) = remaining.remove(mid_idx) {
            interleaved.push(path);
        }
        if remaining.is_empty() {
            break;
        }

        if let Some((path, _)) = remaining.pop_back() {
            interleaved.push(path);
        }
        if remaining.is_empty() {
            break;
        }

        let mid_idx = remaining.len() / 2;
        if let Some((path, _)) = remaining.remove(mid_idx) {
            interleaved.push(path);
        }
    }

    let training_files = interleaved;

    let total_size: u64 = files_with_sizes.iter().map(|(_, s)| s).sum();
    let min_size = files_with_sizes.first().map(|(_, s)| *s).unwrap_or(0);
    let max_size = files_with_sizes.last().map(|(_, s)| *s).unwrap_or(0);
    let avg_size = total_size / files_with_sizes.len().max(1) as u64;

    info!("  Pre-scan completed in {:?}", prescan_start.elapsed());
    info!(
        "  File sizes: min={:.1}MB, max={:.1}MB, avg={:.1}MB, total={:.1}GB",
        min_size as f64 / 1_000_000.0,
        max_size as f64 / 1_000_000.0,
        avg_size as f64 / 1_000_000.0,
        total_size as f64 / 1_000_000_000.0
    );
    info!("  Processing order: shortest/middle/longest/middle interleave for load balancing");

    info!("  First 8 files (interleaved):");
    for (i, path) in training_files.iter().take(8).enumerate() {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        info!(
            "    [{}] {:.1}MB - {}",
            i + 1,
            size as f64 / 1_000_000.0,
            path.file_name().unwrap_or_default().to_string_lossy()
        );
    }

    info!("=== Parameter Validation ===");
    info!(
        "Testing optimal parameters on {} albums from combined set",
        training_files.len()
    );
    info!(
        "  ({} training set + {} from long files list)\n",
        initial_count,
        additional_files.len()
    );

    let threshold_db: f64 = DEFAULT_THRESHOLD_DB;
    let min_duration_secs: f64 = DEFAULT_MIN_DURATION_SECS;
    let match_tolerance_secs = MATCH_TOLERANCE_SECS;

    let rate_limiter = RateLimiter::new();

    let mb_client = Arc::new(
        MBClient::new(cache_config.clone(), Arc::new(rate_limiter.clone()))
            .expect("Failed to create MBClient"),
    );

    let threshold_values: &'static [f64] = &STAGE2_THRESHOLD_VALUES;
    let min_duration_values: &'static [f64] = &STAGE2_MIN_DURATION_VALUES;

    // AcoustID disabled for now
    let acoustid_api_key: Option<String> = None;

    info!("=== Run 28: Multi-Album Parallel + Load Balanced File Order ===");
    info!("  Max concurrent albums: {}", MAX_CONCURRENT_ALBUMS);
    info!("  Album-prefixed logging: [A{{num}}] for parallel debugging");
    info!("  Shared rate limiter ensures MB API compliance\n");

    let total_albums = training_files.len();

    let results: Vec<ValidationResult> = stream::iter(training_files.into_iter().enumerate())
        .map(|(idx, file_path)| {
            let mb_client = Arc::clone(&mb_client);
            let acoustid_api_key = acoustid_api_key.clone();
            async move {
                process_single_album(
                    idx,
                    total_albums,
                    file_path,
                    threshold_db,
                    min_duration_secs,
                    match_tolerance_secs,
                    threshold_values,
                    min_duration_values,
                    mb_client,
                    acoustid_api_key,
                )
                .await
            }
        })
        .buffer_unordered(MAX_CONCURRENT_ALBUMS)
        .collect()
        .await;

    // Sort results for consistent output
    let mut results = results;
    results.sort_by(|a, b| a.album_path.cmp(&b.album_path));

    // Write results
    let output_path = PathBuf::from(&output_json);
    info!("\n=== Writing Results ===");
    info!("Output: {}", output_path.display());

    let json = serde_json::to_string_pretty(&results)?;
    std::fs::write(&output_path, json)?;

    // Analysis
    info!("\n=== COMPREHENSIVE MATCHING ANALYSIS ===");

    let successful = results.iter().filter(|r| r.status == "Success").count();
    info!("Total albums processed: {}", results.len());
    info!("Successfully analyzed: {}", successful);

    if successful > 0 {
        let stage2 = results
            .iter()
            .filter(|r| r.matching_stage == "album_extractor_2_optimization")
            .count();
        let stage3 = results
            .iter()
            .filter(|r| r.matching_stage == "album_extractor_3_assembly")
            .count();
        let stage4 = results
            .iter()
            .filter(|r| r.matching_stage == "album_extractor_4_guided")
            .count();
        let stage5 = results
            .iter()
            .filter(|r| r.matching_stage == "album_extractor_6_merging")
            .count();

        info!("\nMatching Stage Results:");
        info!("  album_extractor_2_optimization:   {} albums", stage2);
        info!("  album_extractor_3_assembly:       {} albums", stage3);
        info!("  album_extractor_4_guided:         {} albums", stage4);
        info!("  album_extractor_6_merging:        {} albums", stage5);

        let excellent = results.iter().filter(|r| r.confidence == "Excellent").count();
        let good = results.iter().filter(|r| r.confidence == "Good").count();
        let fair = results.iter().filter(|r| r.confidence == "Fair").count();
        let poor = results
            .iter()
            .filter(|r| r.confidence == "Poor" && r.status == "Success")
            .count();

        info!("\nConfidence Distribution:");
        info!("  Excellent (≥80%): {} albums", excellent);
        info!("  Good (60-79%):    {} albums", good);
        info!("  Fair (40-59%):    {} albums", fair);
        info!("  Poor (<40%):      {} albums", poor);

        let perfect_counts = results.iter().filter(|r| r.perfect_count_match).count();
        info!("\nTrack Count Matches:");
        info!(
            "  Perfect: {}/{} ({:.1}%)",
            perfect_counts,
            successful,
            (perfect_counts as f64 / successful as f64) * 100.0
        );

        let avg_match_pct = results
            .iter()
            .filter(|r| r.status == "Success")
            .map(|r| r.match_percentage)
            .sum::<f64>()
            / successful as f64;

        let avg_error = results
            .iter()
            .filter(|r| r.status == "Success" && r.mean_error > 0.0)
            .map(|r| r.mean_error)
            .sum::<f64>()
            / successful as f64;

        info!("\nAverage Statistics:");
        info!("  Match percentage: {:.1}%", avg_match_pct);
        info!("  Mean error: {:.2}s", avg_error);

        let mut success_results: Vec<_> = results.iter().filter(|r| r.status == "Success").collect();
        success_results.sort_by(|a, b| cmp_f64(b.match_percentage, a.match_percentage));

        info!("\nBest 5 Albums:");
        for (i, result) in success_results.iter().take(5).enumerate() {
            info!(
                "  {}. {} - {} ({:.1}%, {} via {})",
                i + 1,
                result.artist,
                result.album,
                result.match_percentage,
                result.confidence,
                result.matching_stage
            );
            info!("     MusicBrainz: {}", result.musicbrainz_url);
        }

        info!("\nWorst 5 Albums:");
        for (i, result) in success_results.iter().rev().take(5).enumerate() {
            info!(
                "  {}. {} - {} ({:.1}%, {} via {})",
                i + 1,
                result.artist,
                result.album,
                result.match_percentage,
                result.confidence,
                result.matching_stage
            );
            info!("     MusicBrainz: {}", result.musicbrainz_url);
        }
    }

    info!("\nDone!");

    print_cache_statistics(&cache_config, &*mb_client.get_stats());

    Ok(())
}

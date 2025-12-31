//! Album Matcher Service
//!
//! **[PLAN026/PLAN030]** Album matching via am28 algorithm
//!
//! Provides album identification by matching detected track boundaries
//! against MusicBrainz release editions.
//!
//! # Algorithm Overview
//! 1. Extract metadata from audio file (ID3 tags + path)
//! 2. Search MusicBrainz for candidate releases
//! 3. Group releases into editions (by track count/duration pattern)
//! 4. Test each edition through Stages 2-5:
//!    - Stage 2: Silence detection parameter optimization (180 combinations)
//!    - Stage 3: Over-segmentation assembly (DP algorithm)
//!    - Stage 4: Quiet spot detection (RMS profiling)
//!    - Stage 5: Extra track merging
//! 5. Select best matching edition with artist verification

use std::path::{Path, PathBuf};
use tokio::task::spawn_blocking;
use tracing::{debug, info, warn};

use super::constants::{
    DEFAULT_MIN_DURATION_SECS, DEFAULT_THRESHOLD_DB, EARLY_EXIT_GRACE_PERIOD_SECS,
    MATCH_TOLERANCE_SECS, MIN_ARTIST_SIMILARITY, MIN_DURATION_VALUES, STAGE4_PENALTY_PERCENT,
    THRESHOLD_VALUES,
};
use super::editions::{
    calculate_name_distance, filter_and_sort_editions, filter_editions_by_file_duration,
    group_into_editions,
};
use super::metadata::extract_and_reconcile_metadata;
use super::orchestrator::{run_orchestration, OrchestratorConfig};
use super::silence_detection::precompute_silence_cache;
use super::single_track::SingleTrackDiscriminator;
use super::stages::stage4::RmsProfile;
use super::types::{AlbumMatchResult, MatchedTrack, MatchingStage, SilenceCache};
use crate::services::{MBError, MusicBrainzClient};
use crate::workflow::memory_tracker;

/// Album matching error types
#[derive(Debug)]
pub enum AlbumMatchError {
    /// Audio decoding failed
    DecodeError(String),
    /// Metadata extraction failed
    MetadataError(String),
    /// MusicBrainz API error
    MusicBrainzError(String),
    /// No candidates found
    NoCandidates,
    /// Internal processing error
    InternalError(String),
    /// IO error
    IoError(String),
    /// Task join error
    TaskJoinError(String),
    /// Single track detected (not an album)
    SingleTrackDetected {
        /// Confidence level (0.0-1.0)
        confidence: f64,
        /// Detection stage (pre_decode or post_decode)
        stage: String,
    },
}

impl std::fmt::Display for AlbumMatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlbumMatchError::DecodeError(s) => write!(f, "Decode error: {}", s),
            AlbumMatchError::MetadataError(s) => write!(f, "Metadata error: {}", s),
            AlbumMatchError::MusicBrainzError(s) => write!(f, "MusicBrainz error: {}", s),
            AlbumMatchError::NoCandidates => write!(f, "No MusicBrainz candidates found"),
            AlbumMatchError::InternalError(s) => write!(f, "Internal error: {}", s),
            AlbumMatchError::IoError(s) => write!(f, "IO error: {}", s),
            AlbumMatchError::TaskJoinError(s) => write!(f, "Task join error: {}", s),
            AlbumMatchError::SingleTrackDetected { confidence, stage } => {
                write!(
                    f,
                    "Single track detected (confidence: {:.2}, stage: {})",
                    confidence, stage
                )
            }
        }
    }
}

impl From<MBError> for AlbumMatchError {
    fn from(err: MBError) -> Self {
        AlbumMatchError::MusicBrainzError(err.to_string())
    }
}

impl std::error::Error for AlbumMatchError {}

/// Configuration for album matching
///
/// **[PLAN030]** Extended configuration with stage controls and parameter grid.
#[derive(Debug, Clone)]
pub struct AlbumMatcherConfig {
    // --- Core Parameters ---
    /// Default silence threshold for track detection (dB)
    pub default_threshold_db: f64,
    /// Default minimum silence duration (seconds)
    pub default_min_duration_secs: f64,
    /// Track duration tolerance for matching (seconds)
    pub match_tolerance_secs: f64,
    /// Minimum artist similarity (Jaro-Winkler) to accept match
    pub min_artist_similarity: f64,

    // --- Cache Configuration ---
    /// Enable MusicBrainz API caching
    pub enable_cache: bool,
    /// Cache directory path
    pub cache_dir: Option<PathBuf>,

    // --- Stage Configuration (PLAN030) ---
    /// Enable Stage 3: Over-segmentation assembly
    pub enable_stage3: bool,
    /// Enable Stage 4: Quiet spot detection
    pub enable_stage4: bool,
    /// Enable Stage 5: Extra track merging
    pub enable_stage5: bool,
    /// Penalty percentage for Stage 4 results (lower reliability)
    pub stage4_penalty_percent: f64,

    // --- Early-Exit Configuration (PLAN030) ---
    /// Enable early exit on 100% match
    pub enable_early_exit: bool,
    /// Grace period after 100% match (seconds)
    pub early_exit_grace_secs: u64,

    // --- Parameter Grid Override (PLAN030) ---
    /// Custom threshold values (dB) - uses defaults if None
    pub threshold_values: Option<Vec<f64>>,
    /// Custom min duration values (seconds) - uses defaults if None
    pub min_duration_values: Option<Vec<f64>>,
}

impl Default for AlbumMatcherConfig {
    fn default() -> Self {
        Self {
            default_threshold_db: DEFAULT_THRESHOLD_DB,
            default_min_duration_secs: DEFAULT_MIN_DURATION_SECS,
            match_tolerance_secs: MATCH_TOLERANCE_SECS,
            min_artist_similarity: MIN_ARTIST_SIMILARITY,
            enable_cache: true,
            cache_dir: None,
            enable_stage3: true,
            enable_stage4: true,
            enable_stage5: true,
            stage4_penalty_percent: STAGE4_PENALTY_PERCENT,
            enable_early_exit: true,
            early_exit_grace_secs: EARLY_EXIT_GRACE_PERIOD_SECS,
            threshold_values: None,
            min_duration_values: None,
        }
    }
}

impl AlbumMatcherConfig {
    /// Get threshold values (custom or default)
    pub fn threshold_values(&self) -> &[f64] {
        self.threshold_values
            .as_deref()
            .unwrap_or(&THRESHOLD_VALUES)
    }

    /// Get min duration values (custom or default)
    pub fn min_duration_values(&self) -> &[f64] {
        self.min_duration_values
            .as_deref()
            .unwrap_or(&MIN_DURATION_VALUES)
    }

    /// Get total parameter combinations
    pub fn total_combinations(&self) -> usize {
        self.threshold_values().len() * self.min_duration_values().len()
    }
}

/// Album matcher service
///
/// **[PLAN026/PLAN030]** Orchestrates album identification through am28 stages.
pub struct AlbumMatcher {
    /// Configuration
    config: AlbumMatcherConfig,
    /// MusicBrainz API client
    mb_client: MusicBrainzClient,
    /// Optional database pool for MusicBrainz query caching
    db_pool: Option<sqlx::SqlitePool>,
}

impl AlbumMatcher {
    /// Create new album matcher with default configuration
    pub fn new() -> Result<Self, AlbumMatchError> {
        Self::with_config(AlbumMatcherConfig::default())
    }

    /// Create new album matcher with custom configuration
    pub fn with_config(config: AlbumMatcherConfig) -> Result<Self, AlbumMatchError> {
        let mb_client = MusicBrainzClient::new()
            .map_err(|e| AlbumMatchError::MusicBrainzError(e.to_string()))?;

        Ok(Self {
            config,
            mb_client,
            db_pool: None,
        })
    }

    /// Create new album matcher with provided MusicBrainz client
    pub fn with_client(config: AlbumMatcherConfig, mb_client: MusicBrainzClient) -> Self {
        Self {
            config,
            mb_client,
            db_pool: None,
        }
    }

    /// Create new album matcher with database pool for caching
    pub fn with_pool(
        config: AlbumMatcherConfig,
        mb_client: MusicBrainzClient,
        db_pool: sqlx::SqlitePool,
    ) -> Self {
        Self {
            config,
            mb_client,
            db_pool: Some(db_pool),
        }
    }

    /// Match an album file against MusicBrainz
    ///
    /// **[REQ-ALG-004..014]** Full album matching workflow:
    /// 1. Extract metadata (ID3 tags + path)
    /// 2. Pre-decode single-track check
    /// 3. Decode audio and compute silence cache
    /// 4. Post-decode single-track check
    /// 5. Search MusicBrainz for candidate releases
    /// 6. Group into editions and filter by name similarity
    /// 7. Run multi-stage matching (Stages 2-5)
    /// 8. Verify artist match and return result
    ///
    /// # Arguments
    /// * `audio_path` - Path to the audio file
    /// * `artist_hint` - Artist name override (uses metadata if None)
    /// * `album_hint` - Album name override (uses metadata if None)
    ///
    /// # Returns
    /// AlbumMatchResult with match details or error status
    pub async fn match_album(
        &self,
        audio_path: &Path,
        artist_hint: Option<&str>,
        album_hint: Option<&str>,
    ) -> Result<AlbumMatchResult, AlbumMatchError> {
        info!("Starting album match for: {}", audio_path.display());

        // Step 1: Extract metadata from file
        let metadata = extract_and_reconcile_metadata(audio_path);
        let artist = artist_hint
            .map(|s| s.to_string())
            .unwrap_or(metadata.artist.clone());
        let album = album_hint
            .map(|s| s.to_string())
            .unwrap_or(metadata.album.clone());

        debug!("Metadata: artist={}, album={}", artist, album);

        // Step 2: Pre-decode single-track analysis
        let pre_analysis = SingleTrackDiscriminator::analyze_pre_decode(audio_path, None);

        if pre_analysis.is_likely_single_track && pre_analysis.final_score >= 2.0 {
            info!(
                "Pre-decode single-track detection: score={:.2}, likely single track",
                pre_analysis.final_score
            );

            // Build detailed component breakdown
            let mut details = format!(
                "Single track detected (pre-decode: score={:.2}, threshold=2.0)\n  Components:",
                pre_analysis.final_score
            );
            details.push_str(&format!("\n    - Filename: {:.2}", pre_analysis.filename_score));
            if let Some(ref pattern) = pre_analysis.filename_match {
                details.push_str(&format!(" (matched: {})", pattern));
            }
            details.push_str(&format!(
                "\n    - Directory files: {:.2} ({} audio files)",
                pre_analysis.dir_count_score, pre_analysis.dir_audio_files
            ));
            details.push_str(&format!("\n    - ID3 track tag: {:.2}", pre_analysis.id3_track_score));
            if let Some(ref track_info) = pre_analysis.id3_track_info {
                details.push_str(&format!(" ({})", track_info));
            }
            details.push_str(&format!("\n    - Duration: {:.2}", pre_analysis.duration_score));
            if let Some(duration_mins) = pre_analysis.duration_mins {
                details.push_str(&format!(" ({:.2} mins)", duration_mins));
            }

            return Ok(AlbumMatchResult::no_match(details));
        }

        // Step 3: PARALLEL decode + MusicBrainz lookup
        // am28 pattern: Run CPU-bound decode and network I/O concurrently
        info!(
            "Starting parallel: decode + MusicBrainz lookup for artist={}, album={}",
            artist, album
        );

        let file_path_owned = audio_path.to_path_buf();
        let threshold_values = self.config.threshold_values().to_vec();
        let min_duration_values = self.config.min_duration_values().to_vec();
        let num_min_durations = min_duration_values.len();

        // Spawn decode task (CPU-bound via spawn_blocking)
        let decode_handle = spawn_blocking(move || {
            decode_and_analyze(&file_path_owned, &threshold_values, &min_duration_values)
        });

        // Spawn MusicBrainz lookup (network I/O)
        // Use cached version if database pool available
        let mb_handle = {
            let artist = artist.clone();
            let album = album.clone();
            let db_pool = self.db_pool.clone();
            let mb_client = &self.mb_client;

            async move {
                if let Some(pool) = db_pool {
                    mb_client.comprehensive_search_cached(&artist, &album, Some(50), &pool).await
                } else {
                    mb_client.comprehensive_search(&artist, &album, Some(50)).await
                }
            }
        };

        // Wait for both to complete concurrently
        let (decode_result, mb_result) = tokio::join!(decode_handle, mb_handle);

        // Process decode result
        let (samples, sample_rate, silence_cache, rms_profile) =
            decode_result.map_err(|e| AlbumMatchError::TaskJoinError(e.to_string()))??;

        let _tracking_id = memory_tracker::track_allocation(samples.len(), "album_matcher decode");

        let total_samples = samples.len();
        let duration_secs = total_samples as f64 / sample_rate as f64;
        // **[BUG FIX]** Calculate file duration in milliseconds for pre-filtering
        let file_duration_ms = (duration_secs * 1000.0) as u64;

        debug!(
            "Decoded {} samples at {}Hz ({:.1}s, {}ms)",
            total_samples, sample_rate, duration_secs, file_duration_ms
        );

        // Step 4: Post-decode single-track check using silence gap count
        // Get silence gaps from default parameters
        let default_idx = 6 * num_min_durations + 7; // Approximate middle of grid
        let default_durations = silence_cache
            .get(default_idx.min(silence_cache.len().saturating_sub(1)))
            .cloned()
            .unwrap_or_default();
        let silence_gap_count = default_durations.len().saturating_sub(1);

        let mut post_analysis = pre_analysis;
        let duration_mins = duration_secs / 60.0;
        SingleTrackDiscriminator::update_post_decode(
            &mut post_analysis,
            duration_mins,
            silence_gap_count,
        );

        if post_analysis.is_likely_single_track && post_analysis.final_score >= 2.5 {
            info!(
                "Post-decode single-track detection: score={:.2}, likely single track",
                post_analysis.final_score
            );

            // Build detailed component breakdown
            let mut details = format!(
                "Single track detected (post-decode: score={:.2}, threshold=2.5)\n  Components:",
                post_analysis.final_score
            );
            details.push_str(&format!("\n    - Filename: {:.2}", post_analysis.filename_score));
            if let Some(ref pattern) = post_analysis.filename_match {
                details.push_str(&format!(" (matched: {})", pattern));
            }
            details.push_str(&format!(
                "\n    - Directory files: {:.2} ({} audio files)",
                post_analysis.dir_count_score, post_analysis.dir_audio_files
            ));
            details.push_str(&format!("\n    - ID3 track tag: {:.2}", post_analysis.id3_track_score));
            if let Some(ref track_info) = post_analysis.id3_track_info {
                details.push_str(&format!(" ({})", track_info));
            }
            details.push_str(&format!("\n    - Duration: {:.2}", post_analysis.duration_score));
            if let Some(duration_mins) = post_analysis.duration_mins {
                details.push_str(&format!(" ({:.2} mins)", duration_mins));
            }
            if let Some(silence_score) = post_analysis.silence_gap_score {
                details.push_str(&format!("\n    - Silence gaps: {:.2}", silence_score));
                if let Some(gap_count) = post_analysis.silence_gap_count {
                    details.push_str(&format!(" ({} gaps)", gap_count));
                }
            }

            // **[IMPROVEMENT#1]** Preserve samples for fallback reuse
            return Ok(AlbumMatchResult::no_match_with_audio(
                details,
                samples,
                sample_rate,
            ));
        }

        // Step 5: Process MusicBrainz result (already fetched in parallel)
        let releases = mb_result?;

        if releases.is_empty() {
            // **[IMPROVEMENT#2]** Enhanced logging for diagnosis
            warn!(
                file = %audio_path.display(),
                artist = %artist,
                album = %album,
                "No MusicBrainz candidates found"
            );
            // **[IMPROVEMENT#1]** Preserve samples for fallback reuse
            return Ok(AlbumMatchResult::no_match_with_audio(
                "No MusicBrainz candidates found".to_string(),
                samples,
                sample_rate,
            ));
        }

        info!("Found {} MusicBrainz releases", releases.len());

        // Step 6: Group into editions and filter by name similarity
        let editions = group_into_editions(&releases);
        let editions = filter_and_sort_editions(editions, &artist, &album, 20);

        // **[BUG FIX]** Pre-filter editions by file duration to eliminate impossible matches
        let editions = filter_editions_by_file_duration(editions, file_duration_ms);

        if editions.is_empty() {
            // **[IMPROVEMENT#2]** Enhanced logging for diagnosis
            warn!(
                file = %audio_path.display(),
                candidate_count = releases.len(),
                "No valid editions after filtering"
            );
            // **[IMPROVEMENT#1]** Preserve samples for fallback reuse
            return Ok(AlbumMatchResult::no_match_with_audio(
                "No valid editions found".to_string(),
                samples,
                sample_rate,
            ));
        }

        info!(
            "Testing {} editions (top by name similarity)",
            editions.len()
        );

        // Step 7: Run multi-stage matching orchestration
        let orchestrator_config = OrchestratorConfig {
            // **[PHASE1]** Lowered from 80.0% to 65.0% to capture high-confidence matches
            // that were incorrectly rejected (22 files with 67-79% confidence).
            // See IMPROVEMENT_PLAN_audio_decode_album_matching.md Phase 1.1
            min_match_percentage: 65.0,
            tolerance_secs: self.config.match_tolerance_secs,
            early_exit: self.config.enable_early_exit,
            early_exit_grace: self.config.early_exit_grace_secs as usize,
            quiet_spot_window_secs: 5.0,
            max_merge_tracks: 3,
            name_similarity_weight: 0.4, // Default: 40% name similarity, 60% match percentage
            file_duration_ms, // **[BUG FIX]** Pass actual file duration for validation
        };

        let result = run_orchestration(
            &silence_cache,
            &rms_profile,
            total_samples,
            &editions,
            &orchestrator_config,
        );

        // Log stage-by-stage performance for diagnostics
        eprintln!("\n=== Stage-by-Stage Match Percentages ===");
        if let Some(best_s2) = result.stage_results.stage2.first() {
            eprintln!("  Stage 2 (Parameter Grid): {:.1}% (edition: {})",
                best_s2.best_percentage, best_s2.edition.title);
        } else {
            eprintln!("  Stage 2 (Parameter Grid): No results");
        }
        if let Some(best_s3) = result.stage_results.stage3.first() {
            eprintln!("  Stage 3 (DP Assembly): {:.1}% (edition: {})",
                best_s3.best_percentage, best_s3.edition.title);
        } else {
            eprintln!("  Stage 3 (DP Assembly): No results");
        }
        if let Some(best_s4) = result.stage_results.stage4.first() {
            eprintln!("  Stage 4 (RMS Quiet Spot): {:.1}% penalized, {:.1}% raw (edition: {})",
                best_s4.penalized_percentage, best_s4.raw_percentage, best_s4.edition.title);
        } else {
            eprintln!("  Stage 4 (RMS Quiet Spot): No results");
        }
        if let Some(best_s5) = result.stage_results.stage5.first() {
            eprintln!("  Stage 5 (Extra Merging): {:.1}% (edition: {})",
                best_s5.percentage, best_s5.edition.title);
        } else {
            eprintln!("  Stage 5 (Extra Merging): No results");
        }
        eprintln!("  Winning Stage: {:?} ({:.1}%)", result.winning_stage, result.match_percentage);
        eprintln!("========================================\n");

        // Log track-by-track duration comparison
        if let Some(edition) = result.matched_edition.as_ref() {
            eprintln!("\n=== Track-by-Track Duration Comparison ===");
            eprintln!("Album: {} - {}", edition.artist, edition.title);
            eprintln!("Tracks: {}, Detected: {}", edition.track_count, result.detected_durations.len());
            eprintln!("\n{:>3} {:>10} {:>10} {:>10} {:>10} {:>10}",
                "Trk", "Expected", "@ Offset", "Detected", "Error", "Status");
            eprintln!("{}", "-".repeat(66));

            // Calculate cumulative offsets for detected passages
            let mut cumulative_offset = 0.0;
            let mut total_expected = 0.0;
            let mut total_detected = 0.0;

            for (i, (&expected, &detected)) in edition.track_durations.iter()
                .zip(result.detected_durations.iter())
                .enumerate() {
                let offset = cumulative_offset;
                cumulative_offset += detected;

                let error = (detected - expected).abs();
                let error_sign = if detected > expected { "+" } else { "-" };
                let status = if error <= self.config.match_tolerance_secs {
                    "✓"
                } else {
                    "✗"
                };
                eprintln!("{:>3} {:>9.2}s {:>9.2}s {:>9.2}s {:>9}s {:>10}",
                    i + 1, expected, offset, detected, format!("{}{:.2}", error_sign, error), status);

                total_expected += expected;
                total_detected += detected;
            }

            // Show totals row
            eprintln!("{}", "-".repeat(66));
            eprintln!("{:>3} {:>10} {:>10} {:>10} {:>10} {:>10}",
                "", "TOTALS", "",
                format!("{:.2}s", total_expected),
                format!("{:.2}s", total_detected),
                "");

            // Show any extra detected tracks
            if result.detected_durations.len() > edition.track_count {
                eprintln!("\nExtra detected tracks:");
                for (i, &detected) in result.detected_durations.iter()
                    .skip(edition.track_count)
                    .enumerate() {
                    let offset = cumulative_offset;
                    cumulative_offset += detected;
                    eprintln!("{:>3} {:>9}  {:>9.2}s {:>9.2}s {:>9}  {:>10}",
                        edition.track_count + i + 1, "-", offset, detected, "-", "EXTRA");
                }
            }

            eprintln!("==========================================\n");
        }

        // Step 8: Verify artist match and build result
        let artist_similarity = result
            .matched_edition
            .as_ref()
            .map(|e| {
                let sim = calculate_name_distance(&e.artist, &e.title, &artist, &album);
                // Extract just the artist similarity (rough approximation)
                sim
            })
            .unwrap_or(0.0);

        let artist_verified = artist_similarity >= self.config.min_artist_similarity;

        // Determine confidence level
        let confidence = if result.success && result.match_percentage >= 100.0 && artist_verified {
            "Excellent"
        } else if result.success && result.match_percentage >= 90.0 && artist_verified {
            "Good"
        } else if result.success && result.match_percentage >= 80.0 {
            "Fair"
        } else {
            "Poor"
        };

        // Build matched tracks list
        let tracks = build_matched_tracks(&result, self.config.match_tolerance_secs);

        // Calculate mean error
        let mean_error = if !result.track_errors.is_empty() {
            result.track_errors.iter().sum::<f64>() / result.track_errors.len() as f64
        } else {
            0.0
        };

        // Count matched tracks (within tolerance)
        let matched_count = result
            .track_errors
            .iter()
            .filter(|&e| *e <= self.config.match_tolerance_secs)
            .count();

        // **[Top-5 Ranking]** Generate ranked candidates with passage comparison tables
        let ranked_candidates = super::orchestrator::rank_top_candidates(
            &result.stage_results,
            self.config.match_tolerance_secs,
            5, // Top 5 candidates
            file_duration_ms, // **[BUG FIX]** Pass file duration for validated scoring
        );

        let album_result = AlbumMatchResult {
            matched: result.success,
            release_mbid: result
                .matched_edition
                .as_ref()
                .map(|e| e.release_mbid.clone()),
            matched_artist: result.matched_edition.as_ref().map(|e| e.artist.clone()),
            matched_album: result.matched_edition.as_ref().map(|e| e.title.clone()),
            matching_stage: Some(result.winning_stage),
            match_percentage: result.match_percentage,
            mean_error_seconds: mean_error,
            matched_track_count: matched_count,
            expected_track_count: result
                .matched_edition
                .as_ref()
                .map(|e| e.track_count)
                .unwrap_or(0),
            detected_track_count: result.detected_durations.len(),
            tracks,
            confidence: confidence.to_string(),
            artist_verified,
            artist_similarity,
            best_threshold_db: None, // Could extract from stage results if needed
            best_min_duration_secs: None,
            status: if result.success {
                format!("Matched via {:?}", result.winning_stage)
            } else {
                "No match found".to_string()
            },
            // **[IMPROVEMENT#1]** Preserve samples for successful matches too (may be useful for future features)
            decoded_audio: Some((samples, sample_rate)),
            // **[Top-5 Ranking]** Include ranked candidates with passage comparison tables
            ranked_candidates,
        };

        // **[Top-5 Ranking]** Display ranked candidates with passage comparison tables
        if !album_result.ranked_candidates.is_empty() {
            // Format duration as HH:MM:SS.SS
            let hours = (duration_secs / 3600.0).floor() as u32;
            let minutes = ((duration_secs % 3600.0) / 60.0).floor() as u32;
            let seconds = duration_secs % 60.0;
            let duration_formatted = format!("{}:{:02}:{:05.2}", hours, minutes, seconds);

            eprintln!("\nFile: {}", audio_path.display());
            eprintln!("Total Duration: {} ({:.2}s)", duration_formatted, duration_secs);
            eprintln!("\n=== Top {} Candidate Rankings ===", album_result.ranked_candidates.len());

            for candidate in &album_result.ranked_candidates {
                eprintln!("\n--- Rank #{}: {} - {} ---", candidate.rank, candidate.artist, candidate.title);
                eprintln!("  Release MBID: {}", candidate.release_mbid);
                eprintln!("  Track Count: {}", candidate.track_count);
                eprintln!("  Match %: {:.1}%", candidate.match_percentage);
                eprintln!("  Final Score: {:.4}", candidate.final_score);

                // Score breakdown with weighting
                eprintln!("    ├─ Duration Score: {:.4} × 0.30 = {:.4}",
                    candidate.duration_score, candidate.duration_score * 0.30);
                eprintln!("    ├─ Quality Score:  {:.4} × 0.45 = {:.4}",
                    candidate.quality_score, candidate.quality_score * 0.45);
                eprintln!("    ├─ Name Score:     {:.4} × 0.25 = {:.4}",
                    candidate.name_score, candidate.name_score * 0.25);
                eprintln!("    └─ Base Score:     {:.4} × {:.4} (penalty) = {:.4}",
                    (candidate.duration_score * 0.30) + (candidate.quality_score * 0.45) + (candidate.name_score * 0.25),
                    candidate.track_count_penalty,
                    candidate.final_score);

                eprintln!("  Stage: {:?}", candidate.stage);
                eprintln!("  Mean Error: {:.2}s", candidate.mean_error);

                eprintln!("\n  {:>3} {:<30} {:>10} {:>10} {:>10} {:>10} {:>10}",
                    "Trk", "Track Title", "Expected", "@ Offset", "Detected", "Error", "Status");
                eprintln!("  {}", "-".repeat(106));

                let mut total_expected = 0.0;
                let mut total_detected = 0.0;

                for passage in &candidate.passage_comparison {
                    let status = if passage.within_tolerance { "✓" } else { "✗" };
                    let error_sign = if passage.detected_duration > passage.expected_duration { "+" } else { "-" };
                    eprintln!("  {:>3} {:<30} {:>9.2}s {:>9.2}s {:>9.2}s {:>9}s {:>10}",
                        passage.track_number,
                        passage.track_title,
                        passage.expected_duration,
                        passage.detected_start_offset,
                        passage.detected_duration,
                        format!("{}{:.2}", error_sign, passage.error),
                        status
                    );
                    total_expected += passage.expected_duration;
                    total_detected += passage.detected_duration;
                }

                // Format totals as HH:MM:SS.SS
                let format_time = |secs: f64| {
                    let h = (secs / 3600.0).floor() as u32;
                    let m = ((secs % 3600.0) / 60.0).floor() as u32;
                    let s = secs % 60.0;
                    format!("{}:{:02}:{:05.2}", h, m, s)
                };

                eprintln!("  {}", "-".repeat(106));
                eprintln!("  {:>3} {:<30} {:>10} {:>10} {:>10}",
                    "", "TOTALS",
                    format_time(total_expected),
                    "",
                    format_time(total_detected)
                );
            }
            eprintln!("\n=========================================\n");
        }

        info!(
            "Album match complete: matched={}, stage={:?}, percentage={:.1}%",
            album_result.matched, album_result.matching_stage, album_result.match_percentage
        );

        Ok(album_result)
    }

    /// Get configuration
    pub fn config(&self) -> &AlbumMatcherConfig {
        &self.config
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Decode audio file and compute analysis caches
///
/// Runs synchronously (intended for spawn_blocking)
fn decode_and_analyze(
    path: &Path,
    threshold_values: &[f64],
    min_duration_values: &[f64],
) -> Result<(Vec<f32>, u32, SilenceCache, RmsProfile), AlbumMatchError> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    // Open file
    let file = std::fs::File::open(path).map_err(|e| AlbumMatchError::IoError(e.to_string()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    // Probe format
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AlbumMatchError::DecodeError(e.to_string()))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or(AlbumMatchError::DecodeError("No default track".into()))?;

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or(AlbumMatchError::DecodeError("No sample rate".into()))?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AlbumMatchError::DecodeError(e.to_string()))?;

    let track_id = track.id;
    let mut samples = Vec::new();

    // Decode all packets
    loop {
        match format.next_packet() {
            Ok(packet) if packet.track_id() == track_id => {
                if let Ok(decoded) = decoder.decode(&packet) {
                    let spec = decoded.spec();
                    let channels = spec.channels.count();

                    let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, *spec);
                    sample_buf.copy_interleaved_ref(decoded);

                    // Mix to mono
                    for chunk in sample_buf.samples().chunks(channels) {
                        let mono: f32 = chunk.iter().sum::<f32>() / channels as f32;
                        samples.push(mono);
                    }
                }
            }
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(_) => continue,
            _ => continue,
        }
    }

    if samples.is_empty() {
        return Err(AlbumMatchError::DecodeError(
            "No audio samples decoded".into(),
        ));
    }

    // Compute silence cache
    let silence_cache =
        precompute_silence_cache(&samples, sample_rate, threshold_values, min_duration_values);

    // Compute RMS profile for Stage 4
    let rms_profile = RmsProfile::from_samples(&samples, sample_rate, 50.0);

    Ok((samples, sample_rate, silence_cache, rms_profile))
}

/// Build matched tracks list from orchestration result
fn build_matched_tracks(
    result: &super::orchestrator::OrchestrationResult,
    tolerance_secs: f64,
) -> Vec<MatchedTrack> {
    let edition = match &result.matched_edition {
        Some(e) => e,
        None => return Vec::new(),
    };

    let mut tracks = Vec::new();

    for (i, (detected, expected_ms)) in result
        .detected_durations
        .iter()
        .zip(edition.durations.iter())
        .enumerate()
    {
        let expected_secs = *expected_ms as f64 / 1000.0;
        let error = (detected - expected_secs).abs();

        let recording_mbid = edition.recording_mbids.get(i).cloned().unwrap_or_default();
        let track_title = edition.track_titles.get(i).cloned().unwrap_or_else(|| format!("Track {}", i + 1));

        tracks.push(MatchedTrack {
            track_number: i + 1,
            disc_number: 1, // Simplified - could be enhanced for multi-disc
            recording_mbid,
            title: track_title,
            detected_duration: *detected,
            expected_duration: expected_secs,
            timing_error: error,
            within_tolerance: error <= tolerance_secs,
        });
    }

    tracks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_album_matcher_config_defaults() {
        let config = AlbumMatcherConfig::default();
        // Uses constants from constants.rs
        assert_eq!(config.default_threshold_db, DEFAULT_THRESHOLD_DB);
        assert_eq!(config.default_min_duration_secs, DEFAULT_MIN_DURATION_SECS);
        assert_eq!(config.match_tolerance_secs, MATCH_TOLERANCE_SECS);
        assert_eq!(config.min_artist_similarity, MIN_ARTIST_SIMILARITY);
        // New stage configuration
        assert!(config.enable_stage3);
        assert!(config.enable_stage4);
        assert!(config.enable_stage5);
        assert_eq!(config.stage4_penalty_percent, STAGE4_PENALTY_PERCENT);
        // Early exit
        assert!(config.enable_early_exit);
        assert_eq!(config.early_exit_grace_secs, EARLY_EXIT_GRACE_PERIOD_SECS);
    }

    #[test]
    fn test_album_matcher_creation() {
        let matcher = AlbumMatcher::new().expect("Failed to create matcher");
        assert_eq!(matcher.config().match_tolerance_secs, MATCH_TOLERANCE_SECS);
    }

    #[test]
    fn test_config_total_combinations() {
        let config = AlbumMatcherConfig::default();
        assert_eq!(config.total_combinations(), 180);
        assert_eq!(
            config.threshold_values().len() * config.min_duration_values().len(),
            180
        );
    }

    #[test]
    fn test_config_custom_values_override() {
        let config = AlbumMatcherConfig {
            threshold_values: Some(vec![-50.0, -55.0]),
            min_duration_values: Some(vec![0.5, 1.0, 1.5]),
            ..Default::default()
        };
        // Custom values used
        assert_eq!(config.threshold_values().len(), 2);
        assert_eq!(config.min_duration_values().len(), 3);
        assert_eq!(config.total_combinations(), 6);
    }

    // =========================================================================
    // PLAN030 Increment 13: Additional Tests
    // =========================================================================

    /// TC-U-013-01: Verify config defaults
    #[test]
    fn test_config_defaults_complete() {
        let config = AlbumMatcherConfig::default();

        // Core defaults - uses MATCH_TOLERANCE_SECS from constants
        assert_eq!(config.match_tolerance_secs, MATCH_TOLERANCE_SECS);
        assert_eq!(config.min_artist_similarity, MIN_ARTIST_SIMILARITY);

        // Stage enables
        assert!(config.enable_stage3);
        assert!(config.enable_stage4);
        assert!(config.enable_stage5);

        // Early exit
        assert!(config.enable_early_exit);

        // Grid size
        assert_eq!(config.total_combinations(), 180);
    }

    /// TC-U-013-03: Verify error type conversion
    #[test]
    fn test_error_display() {
        let errors = vec![
            AlbumMatchError::DecodeError("test".into()),
            AlbumMatchError::MetadataError("test".into()),
            AlbumMatchError::MusicBrainzError("test".into()),
            AlbumMatchError::NoCandidates,
            AlbumMatchError::InternalError("test".into()),
            AlbumMatchError::IoError("test".into()),
            AlbumMatchError::TaskJoinError("test".into()),
            AlbumMatchError::SingleTrackDetected {
                confidence: 0.9,
                stage: "pre_decode".into(),
            },
        ];

        for error in errors {
            // All errors should have Display implementation
            let msg = format!("{}", error);
            assert!(!msg.is_empty());
        }
    }

    /// TC-U-013-04: Verify helper function for building matched tracks
    #[test]
    fn test_build_matched_tracks_empty() {
        use super::super::orchestrator::OrchestrationResult;
        use super::super::types::MatchingStage;

        let result = OrchestrationResult {
            winning_stage: MatchingStage::Stage2,
            matched_edition: None,
            match_percentage: 0.0,
            detected_durations: vec![],
            track_errors: vec![],
            success: false,
            stage_results: Default::default(),
        };

        let tracks = build_matched_tracks(&result, 3.0);
        assert!(tracks.is_empty());
    }
}

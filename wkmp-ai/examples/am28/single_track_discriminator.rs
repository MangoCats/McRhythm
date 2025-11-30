//! # Single-Track Discriminator
//!
//! Layered detection system to identify files containing individual tracks vs full albums.
//!
//! ## Problem
//! Album matcher is designed for full-album MP3 files (all tracks concatenated).
//! If a user accidentally processes individual track files, results will be invalid.
//! This module detects such files before expensive processing.
//!
//! ## 5-Layer Detection System
//!
//! ### Layer 1: Filename Pattern (pre-decode)
//! - Check for track number prefix (e.g., "01 - Song.mp3", "12_Track.mp3")
//! - Score: +1.0 if pattern found
//!
//! ### Layer 2: Directory File Count (pre-decode)
//! - Count audio files in parent directory
//! - Score: +0.8 if >= 4 files (likely individual tracks)
//! - Score: +0.3 if 2-3 files
//! - Score: -0.5 if 1 file (album file)
//!
//! ### Layer 3: ID3 Track Number/Total Tags (pre-decode)
//! - Check for ID3 track number and total fields
//! - Score: +1.0 if track total > 1 (definitive)
//! - Score: +0.4 if track number present but no total
//!
//! ### Layer 4: Duration Heuristics (pre or post-decode)
//! - Single tracks typically < 8 min, albums > 20 min
//! - Score: +0.7 if < 8 min (short)
//! - Score: +0.4 if 8-20 min (suspicious)
//! - Score: -0.3 if > 20 min (album length)
//!
//! ### Layer 5: Silence Gap Count (post-decode)
//! - Albums have multiple tracks = multiple silence gaps (typically 3+)
//! - Score: +1.0 if < 3 gaps (definitive)
//! - Score: -0.5 if >= 3 gaps (album)
//!
//! ## Scoring
//! - Pre-decode score: sum of layers 1-4
//! - Final score: sum of all 5 layers
//! - Threshold: score >= 1.5 = likely single track
//!
//! ## Usage
//! ```ignore
//! // Pre-decode analysis
//! let mut analysis = SingleTrackDiscriminator::analyze_pre_decode(&path, None);
//! SingleTrackDiscriminator::log_pre_decode(&album_id, &analysis, &path);
//!
//! // Post-decode update
//! let duration_mins = file_duration_secs / 60.0;
//! let gap_count = detected_tracks.len().saturating_sub(1);
//! SingleTrackDiscriminator::update_post_decode(&mut analysis, duration_mins, gap_count);
//! SingleTrackDiscriminator::log_post_decode(&album_id, &analysis);
//!
//! if analysis.is_likely_single_track {
//!     warn!("Single track detected - results may be invalid");
//! }
//! ```

use crate::constants::*;
use crate::types::SingleTrackAnalysis;
use lofty::prelude::*;
use lofty::probe::Probe;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;
use tracing::{info, warn};

/// Regex for detecting track number prefix in filenames
static TRACK_NUMBER_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(SINGLE_TRACK_FILENAME_PATTERN).expect("Invalid track number regex"));

// =============================================================================
// Single-Track Discriminator Implementation
// =============================================================================

/// Single-Track Discriminator - layered detection of individual tracks vs albums
pub(crate) struct SingleTrackDiscriminator;

impl SingleTrackDiscriminator {
    /// Layer 1: Check for track number prefix in filename
    ///
    /// Detects patterns like "01 - Song.mp3", "12_Track.mp3", "8. Title.mp3"
    ///
    /// # Returns
    /// (score, matched_pattern) tuple
    /// - score: +1.0 if pattern found, 0.0 otherwise
    /// - matched_pattern: The matched text if found
    fn check_filename_pattern(path: &Path) -> (f64, Option<String>) {
        if let Some(filename) = path.file_stem() {
            let filename_str = filename.to_string_lossy();
            if let Some(captures) = TRACK_NUMBER_REGEX.captures(&filename_str) {
                let matched = captures.get(0).map(|m| m.as_str().to_string());
                return (SCORE_FILENAME_PATTERN, matched);
            }
        }
        (0.0, None)
    }

    /// Layer 2: Count audio files in parent directory
    ///
    /// Many audio files in same directory suggests individual tracks rather than albums.
    ///
    /// # Returns
    /// (score, audio_file_count) tuple
    /// - score: +0.8 if >= 4 files, +0.3 if 2-3, -0.5 if 1, 0.0 if 0 or error
    /// - audio_file_count: Number of audio files found
    fn check_directory_files(path: &Path) -> (f64, usize) {
        let parent = match path.parent() {
            Some(p) => p,
            None => return (0.0, 0),
        };

        let audio_count = std::fs::read_dir(parent)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0);

        let score = if audio_count >= SINGLE_TRACK_DIR_FILE_THRESHOLD {
            SCORE_DIR_FILES_HIGH
        } else if audio_count >= 2 {
            SCORE_DIR_FILES_MEDIUM
        } else if audio_count == 1 {
            SCORE_DIR_FILES_SINGLE
        } else {
            0.0
        };

        (score, audio_count)
    }

    /// Layer 3: Check ID3 track number tag
    ///
    /// ID3 track total field is definitive when present.
    /// If track total > 1, file is definitely a single track.
    ///
    /// # Returns
    /// (score, track_info) tuple
    /// - score: +1.0 if total > 1, +0.4 if number only, 0.0 otherwise
    /// - track_info: "num/total" or "num/?" if present
    fn check_id3_track_tag(path: &Path) -> (f64, Option<String>) {
        let tagged_file = match Probe::open(path).and_then(|p| p.read()) {
            Ok(f) => f,
            Err(_) => return (0.0, None),
        };

        let tag = match tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())
        {
            Some(t) => t,
            None => return (0.0, None),
        };

        let track_number = tag.track();
        let track_total = tag.track_total();

        match (track_number, track_total) {
            (Some(num), Some(total)) if total > SINGLE_TRACK_ID3_TOTAL_THRESHOLD => {
                (SCORE_ID3_TRACK_TOTAL, Some(format!("{}/{}", num, total)))
            }
            (Some(num), Some(total)) => (0.0, Some(format!("{}/{}", num, total))),
            (Some(num), None) => (SCORE_ID3_TRACK_NUMBER_ONLY, Some(format!("{}/?", num))),
            _ => (0.0, None),
        }
    }

    /// Layer 4: Check duration (from ID3 or provided decoded duration)
    ///
    /// Single tracks typically < 8 min, albums typically > 20 min.
    ///
    /// # Arguments
    /// * `duration_mins` - Duration in minutes
    ///
    /// # Returns
    /// Score based on duration thresholds:
    /// - +0.7 if < 8 min (short, likely single track)
    /// - +0.4 if 8-20 min (suspicious)
    /// - -0.3 if > 20 min (album length)
    fn check_duration(duration_mins: f64) -> f64 {
        if duration_mins < SINGLE_TRACK_TYPICAL_DURATION_MINS {
            SCORE_DURATION_SHORT
        } else if duration_mins < SINGLE_TRACK_MIN_ALBUM_DURATION_MINS {
            SCORE_DURATION_SUSPICIOUS
        } else {
            SCORE_DURATION_ALBUM_LENGTH
        }
    }

    /// Layer 5: Check silence gap count (post-decode)
    ///
    /// Albums have multiple tracks = multiple silence gaps (typically 3+).
    /// Files with few gaps are likely single tracks.
    ///
    /// # Arguments
    /// * `gap_count` - Number of silence gaps detected (N tracks = N-1 gaps)
    ///
    /// # Returns
    /// Score based on gap count:
    /// - +1.0 if < 3 gaps (definitive single track)
    /// - -0.5 if >= 3 gaps (album with multiple tracks)
    fn check_silence_gaps(gap_count: usize) -> f64 {
        if gap_count < SINGLE_TRACK_MIN_EXPECTED_GAPS {
            SCORE_SILENCE_GAPS_FEW
        } else {
            SCORE_SILENCE_GAPS_MANY
        }
    }

    /// Perform pre-decode analysis (layers 1-3, and layer 4 if duration available)
    ///
    /// Analyzes file metadata that doesn't require decoding audio.
    ///
    /// # Arguments
    /// * `path` - Path to audio file
    /// * `duration_mins` - Optional duration from ID3 tags (if available)
    ///
    /// # Returns
    /// SingleTrackAnalysis with pre-decode layers populated
    ///
    /// # Layers Included
    /// 1. Filename pattern check
    /// 2. Directory file count
    /// 3. ID3 track number/total
    /// 4. Duration (if provided)
    pub(crate) fn analyze_pre_decode(
        path: &Path,
        duration_mins: Option<f64>,
    ) -> SingleTrackAnalysis {
        let mut analysis = SingleTrackAnalysis::new();

        // Layer 1: Filename pattern
        let (score, matched) = Self::check_filename_pattern(path);
        analysis.filename_score = score;
        analysis.filename_match = matched;

        // Layer 2: Directory file count
        let (score, count) = Self::check_directory_files(path);
        analysis.dir_count_score = score;
        analysis.dir_audio_files = count;

        // Layer 3: ID3 track tag
        let (score, info) = Self::check_id3_track_tag(path);
        analysis.id3_track_score = score;
        analysis.id3_track_info = info;

        // Layer 4: Duration (if available from ID3)
        if let Some(mins) = duration_mins {
            analysis.duration_score = Self::check_duration(mins);
            analysis.duration_mins = Some(mins);
        }

        analysis.update_aggregates();
        analysis
    }

    /// Update analysis with post-decode information
    ///
    /// Adds layer 5 (silence gaps) and updates duration if not already set.
    ///
    /// # Arguments
    /// * `analysis` - Mutable reference to existing analysis
    /// * `duration_mins` - Decoded duration in minutes (more accurate than ID3)
    /// * `gap_count` - Number of silence gaps detected (N tracks = N-1 gaps)
    pub(crate) fn update_post_decode(
        analysis: &mut SingleTrackAnalysis,
        duration_mins: f64,
        gap_count: usize,
    ) {
        // Update duration if not already set or if decoded is more accurate
        if analysis.duration_mins.is_none() {
            analysis.duration_score = Self::check_duration(duration_mins);
        }
        analysis.duration_mins = Some(duration_mins);

        // Layer 5: Silence gaps
        analysis.silence_gap_score = Some(Self::check_silence_gaps(gap_count));
        analysis.silence_gap_count = Some(gap_count);

        analysis.update_aggregates();
    }

    /// Log pre-decode analysis results
    ///
    /// Outputs detailed layer-by-layer scores and pre-decode summary.
    ///
    /// # Arguments
    /// * `album_id` - Album identifier for logging (e.g., "A1")
    /// * `analysis` - Analysis results
    /// * `path` - File path (for filename display)
    pub(crate) fn log_pre_decode(album_id: &str, analysis: &SingleTrackAnalysis, path: &Path) {
        info!(
            "[{}] 🔍 Single-track analysis (pre-decode) for {:?}:",
            album_id,
            path.file_name().unwrap_or_default()
        );

        // Layer 1
        if let Some(ref matched) = analysis.filename_match {
            info!(
                "[{}]     Filename pattern: {:+.2} (matched \"{}\")",
                album_id, analysis.filename_score, matched
            );
        } else {
            info!(
                "[{}]     Filename pattern: {:+.2} (no track number prefix)",
                album_id, analysis.filename_score
            );
        }

        // Layer 2
        info!(
            "[{}]     Directory files:  {:+.2} ({} audio files in dir, threshold={})",
            album_id,
            analysis.dir_count_score,
            analysis.dir_audio_files,
            SINGLE_TRACK_DIR_FILE_THRESHOLD
        );

        // Layer 3
        if let Some(ref info) = analysis.id3_track_info {
            info!(
                "[{}]     ID3 track tag:    {:+.2} (track {})",
                album_id, analysis.id3_track_score, info
            );
        } else {
            info!(
                "[{}]     ID3 track tag:    {:+.2} (no track tag)",
                album_id, analysis.id3_track_score
            );
        }

        // Layer 4
        if let Some(mins) = analysis.duration_mins {
            info!(
                "[{}]     Duration hint:    {:+.2} ({:.2} min, album threshold={} min)",
                album_id, analysis.duration_score, mins, SINGLE_TRACK_MIN_ALBUM_DURATION_MINS
            );
        } else {
            info!(
                "[{}]     Duration hint:    N/A (will check after decode)",
                album_id
            );
        }

        // Pre-decode summary
        info!(
            "[{}]     PRE-DECODE TOTAL: {:.2} ({} confidence{})",
            album_id,
            analysis.pre_decode_score,
            analysis.confidence,
            if analysis.is_likely_single_track {
                " - LIKELY SINGLE TRACK"
            } else {
                ""
            }
        );
    }

    /// Log post-decode analysis update
    ///
    /// Outputs layer 5 results and final summary with warning if single track detected.
    ///
    /// # Arguments
    /// * `album_id` - Album identifier for logging (e.g., "A1")
    /// * `analysis` - Analysis results (after update_post_decode)
    pub(crate) fn log_post_decode(album_id: &str, analysis: &SingleTrackAnalysis) {
        info!(
            "[{}] 🔍 Single-track analysis (post-decode update):",
            album_id
        );

        // Layer 4 (if updated)
        if let Some(mins) = analysis.duration_mins {
            info!(
                "[{}]     Decoded duration: {:+.2} ({:.2} min)",
                album_id, analysis.duration_score, mins
            );
        }

        // Layer 5
        if let Some(score) = analysis.silence_gap_score {
            let count = analysis.silence_gap_count.unwrap_or(0);
            info!(
                "[{}]     Silence gaps:     {:+.2} ({} gaps detected, threshold={})",
                album_id, score, count, SINGLE_TRACK_MIN_EXPECTED_GAPS
            );
        }

        // Final summary
        info!(
            "[{}]     FINAL TOTAL:      {:.2} ({} confidence)",
            album_id, analysis.final_score, analysis.confidence
        );

        if analysis.is_likely_single_track {
            warn!("[{}]     ⚠️ SINGLE TRACK DETECTED - Results may be invalid (score={:.2} >= threshold={})",
                album_id, analysis.final_score, SINGLE_TRACK_SCORE_THRESHOLD);
        }
    }
}

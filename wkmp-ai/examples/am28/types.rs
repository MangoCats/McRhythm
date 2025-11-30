//! # Shared Data Structures
//!
//! Common types used across multiple modules in the album matcher.
//!
//! This module contains all shared data structures extracted from album_matcher_28.rs,
//! organized by functional domain.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

// =============================================================================
// Cache Types (PLAN026 - MusicBrainz Caching)
// =============================================================================

/// Cache mode configuration
#[derive(Debug, Clone, Copy)]
pub(crate) enum CacheMode {
    /// Live API only, no caching
    Disabled,
    /// Cache hits use cache, misses query API and store (default)
    ReadWrite,
    /// Cache only, error on miss (for algorithm tuning)
    ReadOnly,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub(crate) struct CacheConfig {
    pub(crate) mode: CacheMode,
    pub(crate) cache_dir: PathBuf,
}

/// Cached search response
#[derive(Serialize, Deserialize)]
pub(crate) struct CachedSearch {
    /// Original query string (for verification)
    pub(crate) query: String,
    /// ISO 8601 timestamp
    pub(crate) timestamp: String,
    /// Search response data
    pub(crate) response: MBSearchResponse,
}

/// Cached release details
#[derive(Serialize, Deserialize)]
pub(crate) struct CachedRelease {
    /// Release MBID (for verification)
    pub(crate) mbid: String,
    /// ISO 8601 timestamp
    pub(crate) timestamp: String,
    /// Release details data
    pub(crate) details: MBReleaseDetails,
}

/// Cache metadata (cache/musicbrainz/metadata.json)
#[derive(Serialize, Deserialize)]
pub(crate) struct CacheMetadata {
    /// Cache version identifier
    pub(crate) version: String,
    /// ISO 8601 timestamp of cache creation
    pub(crate) created: String,
    /// Number of cached search responses
    pub(crate) search_count: usize,
    /// Number of cached release details
    pub(crate) release_count: usize,
    /// ISO 8601 timestamp of last update
    pub(crate) last_updated: String,
}

/// Cache statistics tracking
#[derive(Debug, Default)]
pub(crate) struct CacheStats {
    pub(crate) search_hits: AtomicU64,
    pub(crate) search_misses: AtomicU64,
    pub(crate) release_hits: AtomicU64,
    pub(crate) release_misses: AtomicU64,
}

impl CacheStats {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn record_search_hit(&self) {
        self.search_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_search_miss(&self) {
        self.search_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_release_hit(&self) {
        self.release_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_release_miss(&self) {
        self.release_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn total_hits(&self) -> u64 {
        self.search_hits.load(Ordering::Relaxed) + self.release_hits.load(Ordering::Relaxed)
    }

    pub(crate) fn total_misses(&self) -> u64 {
        self.search_misses.load(Ordering::Relaxed) + self.release_misses.load(Ordering::Relaxed)
    }

    pub(crate) fn hit_rate(&self) -> f64 {
        let hits = self.total_hits();
        let total = hits + self.total_misses();
        if total == 0 {
            0.0
        } else {
            (hits as f64) / (total as f64)
        }
    }
}

// =============================================================================
// MusicBrainz API Types
// =============================================================================

/// MusicBrainz search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MBSearchResponse {
    /// List of releases matching the search query
    pub(crate) releases: Vec<MBRelease>,
}

/// A MusicBrainz release (album) from search results
///
/// Note: Some fields exist in the MusicBrainz JSON response but are not currently
/// used by our matching algorithm. They are retained for API completeness.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct MBRelease {
    /// MusicBrainz release ID (MBID)
    pub(crate) id: String,
    /// Release title
    pub(crate) title: String,
    /// Artist credits for this release
    #[serde(rename = "artist-credit")]
    pub(crate) artist_credit: Option<Vec<MBArtistCredit>>,
    /// Country of release (e.g., "US", "GB")
    pub(crate) country: Option<String>,
    /// Release status (e.g., "Official", "Bootleg")
    pub(crate) status: Option<String>,
    /// Physical packaging type (e.g., "Jewel Case")
    pub(crate) packaging: Option<String>,
}

/// Artist credit entry linking an artist to a release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MBArtistCredit {
    /// The artist information
    pub(crate) artist: Option<MBArtist>,
}

/// MusicBrainz artist information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MBArtist {
    /// Artist name
    pub(crate) name: String,
}

/// Detailed release information including track listings
///
/// Note: Some fields exist in the MusicBrainz JSON response but are not currently
/// used by our matching algorithm. They are retained for API completeness.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct MBReleaseDetails {
    /// MusicBrainz release ID (MBID)
    pub(crate) id: String,
    /// Release title
    pub(crate) title: String,
    /// Media (discs) in this release
    pub(crate) media: Vec<MBMedia>,
}

/// A single medium (disc) within a release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MBMedia {
    /// Tracks on this medium
    pub(crate) tracks: Vec<MBTrack>,
    /// Format type (e.g., "CD", "Vinyl", "Digital Media")
    pub(crate) format: Option<String>,
}

/// MusicBrainz recording information (linked to a track)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MBRecording {
    /// MusicBrainz Recording MBID
    pub(crate) id: String,
    /// Recording title
    pub(crate) title: Option<String>,
}

/// A single track on a medium
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MBTrack {
    /// Track length in milliseconds
    pub(crate) length: Option<u32>,
    /// Linked recording (contains Recording MBID)
    pub(crate) recording: Option<MBRecording>,
}

// =============================================================================
// Edition Grouping Types
// =============================================================================

/// A specific MusicBrainz release ID within an edition group
///
/// Multiple MBIDs can represent the same edition (e.g., US vs UK release).
#[derive(Debug, Clone)]
pub(crate) struct EditionMBID {
    /// MusicBrainz release ID
    pub(crate) mbid: String,
    /// Country of release
    pub(crate) country: Option<String>,
    /// Release status (Official, Bootleg, etc.)
    pub(crate) status: Option<String>,
    /// Whether this is a CD release (preferred format)
    pub(crate) is_cd: bool,
}

/// A unique album edition identified by track count and duration pattern
///
/// Groups multiple MBIDs that represent the same physical release.
#[derive(Debug, Clone)]
pub(crate) struct Edition {
    /// Number of tracks in this edition
    pub(crate) track_count: usize,
    /// Track durations in seconds
    pub(crate) durations: Vec<u32>,
    /// Recording MBIDs for each track (same order as durations)
    pub(crate) recording_mbids: Vec<String>,
    /// All MBIDs representing this edition
    pub(crate) mbids: Vec<EditionMBID>,
    /// Signature for deduplication (e.g., "107,125,135,...")
    pub(crate) duration_signature: String,
    /// Artist name from MusicBrainz
    pub(crate) artist: String,
    /// Album title from MusicBrainz
    pub(crate) album: String,
    /// Rank 1-N based on name similarity to source (1 = best match)
    pub(crate) name_distance_rank: usize,
    /// Overall name distance score (lower = better match)
    pub(crate) name_distance_score: f64,
}

// =============================================================================
// Track Matching Types
// =============================================================================

/// Track matching comparison result
#[derive(Debug, Clone, Serialize)]
pub(crate) struct TrackMatch {
    /// Duration detected from audio analysis (seconds)
    pub(crate) detected_duration: f64,
    /// Expected duration from MusicBrainz (seconds)
    pub(crate) expected_duration: u32,
    /// Absolute difference between detected and expected (seconds)
    pub(crate) error: f64,
    /// Whether error is within MATCH_TOLERANCE_SECS
    pub(crate) matches: bool,
}

/// A detected track with no corresponding MusicBrainz entry
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExtraTrack {
    /// 1-based index in detected tracks
    pub(crate) track_index: usize,
    /// Duration in seconds
    pub(crate) duration: f64,
    /// Description of why this is extra
    pub(crate) description: String,
}

/// Pre-computed silence detection cache
///
/// Matrix of track durations for all parameter combinations:
/// - Outer Vec: Indexed by (threshold_idx * num_min_durations + min_duration_idx)
/// - Inner Vec: Track durations in seconds for that parameter combination
///
/// Computed once per album by `precompute_silence_cache()`, then reused for all
/// editions during Stage 2 testing. Avoids re-scanning audio for each parameter set.
pub(crate) type SilenceCache = Vec<Vec<f64>>;

/// Over-segmented candidate collected during Stage 2 for Stage 3 assembly
///
/// Represents a segmentation with more tracks than expected, which may
/// assemble into a correct match via dynamic programming.
#[derive(Debug, Clone)]
pub(crate) struct OverSegmentedCandidate {
    /// Detected track durations in seconds
    pub(crate) durations: Vec<f64>,
    /// Silence detection threshold used (dB)
    pub(crate) threshold_db: f64,
    /// Minimum silence duration used (seconds)
    pub(crate) min_duration_secs: f64,
    /// Number of tracks detected
    pub(crate) track_count: usize,
}

/// Result of testing a segmentation against expected track durations
#[derive(Debug, Clone)]
pub(crate) struct CandidateTestResult {
    /// Percentage of expected tracks that matched (0-100)
    pub(crate) percentage: f64,
    /// Number of tracks within tolerance
    pub(crate) matched_count: usize,
    /// Per-track comparison results
    pub(crate) matches: Vec<TrackMatch>,
    /// MusicBrainz release ID of best match
    pub(crate) mbid: String,
    /// Expected track durations from MusicBrainz (seconds)
    pub(crate) expected_durations: Vec<u32>,
    /// Mean absolute error across matched tracks (seconds)
    pub(crate) mean_error: f64,
    /// Detected track durations from audio analysis (seconds)
    pub(crate) detected_durations: Vec<f64>,
}

/// Stage 2 results for single-edition processing (Run 15)
#[derive(Debug)]
pub(crate) struct SingleEditionStage2Results {
    pub(crate) best_result: Option<CandidateTestResult>,
    pub(crate) over_segmented_candidates: Vec<OverSegmentedCandidate>,
    pub(crate) best_threshold: Option<f64>,
    pub(crate) best_min_duration: Option<f64>,
}

/// Result of testing a single edition through all stages (Run 15 parallel processing)
#[derive(Clone)]
pub(crate) struct EditionTestResult {
    /// Index of the edition in the editions list
    pub(crate) edition_idx: usize,
    /// Best percentage achieved for this edition
    pub(crate) best_percentage: f64,
    /// Best result found across all stages
    pub(crate) best_result: Option<CandidateTestResult>,
    /// Which stage produced the best result
    pub(crate) best_stage: &'static str,
    /// Best threshold from stage 2 (if applicable)
    pub(crate) best_threshold: Option<f64>,
    /// Best min_duration from stage 2 (if applicable)
    pub(crate) best_min_duration: Option<f64>,
    /// Expected durations from this edition
    pub(crate) expected_durations: Vec<u32>,
    /// Log messages accumulated during processing
    pub(crate) log_messages: Vec<String>,
}

// =============================================================================
// AcoustID Fingerprinting Types (Run 24)
// =============================================================================

/// AcoustID API response structure
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AcoustIDLookupResponse {
    pub(crate) status: String,
    pub(crate) results: Vec<AcoustIDLookupResult>,
}

/// AcoustID lookup result
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AcoustIDLookupResult {
    pub(crate) id: String,
    pub(crate) score: f64,
    pub(crate) recordings: Option<Vec<AcoustIDLookupRecording>>,
}

/// AcoustID recording information
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AcoustIDLookupRecording {
    /// MusicBrainz Recording MBID
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) title: Option<String>,
}

/// Response from AcoustID track/list_by_mbid endpoint
///
/// Used to check if a MusicBrainz Recording MBID exists in AcoustID database.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AcoustIDTrackByMbidResponse {
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) tracks: Vec<AcoustIDTrackInfo>,
}

/// Track info from AcoustID track lookup
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AcoustIDTrackInfo {
    /// AcoustID track ID (fingerprint ID)
    pub(crate) id: String,
}

/// Result of checking if an MBID exists in AcoustID database
#[derive(Debug, Clone)]
pub(crate) enum MbidLookupResult {
    /// MBID has fingerprints in AcoustID (count of associated tracks/fingerprints)
    ExistsWithFingerprints(usize),
    /// MBID not found in AcoustID database
    NotInDatabase,
    /// Lookup failed (error message)
    LookupFailed(String),
}

/// Result of AcoustID verification for a single track
#[derive(Debug, Clone, Serialize)]
pub(crate) struct TrackVerification {
    /// 1-based track index
    pub(crate) track_index: usize,
    /// Expected Recording MBID from MusicBrainz edition
    pub(crate) expected_recording_mbid: String,
    /// Recording MBID(s) returned by AcoustID lookup (may have multiple matches)
    pub(crate) acoustid_recording_mbids: Vec<String>,
    /// AcoustID confidence score (0.0-1.0, highest match)
    pub(crate) acoustid_score: f64,
    /// Whether expected MBID was found in AcoustID results
    pub(crate) mbid_match: bool,
    /// Status: "matched", "mismatched", "no_acoustid_match", "fingerprint_failed", "skipped"
    pub(crate) status: String,
}

/// Summary of AcoustID verification for an album
#[derive(Debug, Clone, Serialize)]
pub(crate) struct AcoustIDVerificationSummary {
    /// Total tracks in edition
    pub(crate) total_tracks: usize,
    /// Tracks where expected MBID matched AcoustID
    pub(crate) matched_count: usize,
    /// Tracks where AcoustID returned different MBID(s)
    pub(crate) mismatched_count: usize,
    /// Tracks where AcoustID found no matches
    pub(crate) no_match_count: usize,
    /// Tracks where fingerprinting failed
    pub(crate) fingerprint_failed_count: usize,
    /// Per-track verification results
    pub(crate) track_verifications: Vec<TrackVerification>,
}

// =============================================================================
// Output Types
// =============================================================================

/// Complete validation result for an album matching attempt
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ValidationResult {
    /// Path to the source audio file
    pub(crate) album_path: String,
    /// Artist name (from ID3 or path)
    pub(crate) artist: String,
    /// Album name (from ID3 or path)
    pub(crate) album: String,
    /// MusicBrainz release ID of best match
    pub(crate) mbid: String,
    /// Full MusicBrainz URL for the release
    pub(crate) musicbrainz_url: String,
    /// Number of tracks in matched MusicBrainz release
    pub(crate) expected_track_count: usize,
    /// Number of tracks detected in audio file
    pub(crate) detected_track_count: usize,
    /// Whether detected count equals expected count
    pub(crate) perfect_count_match: bool,
    /// Per-track comparison results
    pub(crate) track_matches: Vec<TrackMatch>,
    /// Detected tracks with no MusicBrainz match
    pub(crate) extra_tracks: Vec<ExtraTrack>,
    /// Count of tracks within tolerance
    pub(crate) matched_tracks_count: usize,
    /// Percentage of expected tracks that matched
    pub(crate) match_percentage: f64,
    /// Mean absolute error across matched tracks (seconds)
    pub(crate) mean_error: f64,
    /// Status message describing result
    pub(crate) status: String,
    /// Stage that produced best match (e.g., "album_extractor_2_optimization")
    pub(crate) matching_stage: String,
    /// Optimal silence threshold (dB) if found
    pub(crate) best_threshold_db: Option<f64>,
    /// Optimal minimum silence duration (seconds) if found
    pub(crate) best_min_duration_secs: Option<f64>,
    /// Confidence level: "Excellent", "Good", "Fair", "Poor"
    pub(crate) confidence: String,
    /// Matched artist name from MusicBrainz (may differ from source)
    pub(crate) matched_artist: String,
    /// Matched album name from MusicBrainz (may differ from source)
    pub(crate) matched_album: String,
    /// True if matched artist significantly differs from source artist
    ///
    /// This indicates the match may be incorrect (wrong artist with similar album name).
    pub(crate) artist_mismatch: bool,
    /// Jaro-Winkler similarity score between source and matched artist (0.0-1.0)
    pub(crate) artist_similarity: f64,
    /// AcoustID verification results (Run 24: chromaprint/AcoustID validation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) acoustid_verification: Option<AcoustIDVerificationSummary>,
}

impl ValidationResult {
    /// Create an error/failure ValidationResult with common defaults
    ///
    /// Used when matching fails before finding any MusicBrainz candidates.
    /// Sets all numeric fields to zero/empty and status to provided message.
    ///
    /// # Arguments
    /// * `album_path` - Path to the source audio file
    /// * `artist` - Artist name (from ID3 or path)
    /// * `album` - Album name (from ID3 or path)
    /// * `detected_track_count` - Number of tracks detected in audio file
    /// * `status` - Error message describing why matching failed
    ///
    /// # Returns
    /// ValidationResult with all fields populated with failure defaults
    pub(crate) fn error(
        album_path: &std::path::Path,
        artist: &str,
        album: &str,
        detected_track_count: usize,
        status: String,
    ) -> Self {
        Self {
            album_path: album_path.to_string_lossy().to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            mbid: String::new(),
            musicbrainz_url: String::new(),
            expected_track_count: 0,
            detected_track_count,
            perfect_count_match: false,
            track_matches: Vec::new(),
            extra_tracks: Vec::new(),
            matched_tracks_count: 0,
            match_percentage: 0.0,
            mean_error: 0.0,
            status,
            matching_stage: "None".to_string(),
            best_threshold_db: None,
            best_min_duration_secs: None,
            confidence: "Poor".to_string(),
            matched_artist: String::new(),
            matched_album: String::new(),
            artist_mismatch: false,
            artist_similarity: 0.0,
            acoustid_verification: None,
        }
    }
}

// =============================================================================
// Metadata Extraction and Reconciliation Types
// =============================================================================

/// ID3 metadata extracted from audio file tags
///
/// Contains all relevant ID3v2 tag fields for album matching, including
/// MusicBrainz IDs if present in tags.
#[derive(Debug, Clone)]
pub(crate) struct ID3Metadata {
    /// Artist name from ID3 tag.
    pub(crate) artist: Option<String>,
    /// Album name from ID3 tag.
    pub(crate) album: Option<String>,
    /// Release date from ID3 tag.
    pub(crate) date: Option<String>,
    /// Genre from ID3 tag.
    pub(crate) genre: Option<String>,
    /// MusicBrainz album ID if present in tags.
    pub(crate) musicbrainz_albumid: Option<String>,
    /// MusicBrainz artist ID if present in tags.
    pub(crate) musicbrainz_artistid: Option<String>,
    /// Comment field (may contain track count info).
    pub(crate) comment: Option<String>,
    /// All ID3 tags as key-value pairs.
    pub(crate) all_tags: std::collections::HashMap<String, String>,
}

/// Metadata after reconciling ID3 tags with path-derived information
///
/// Resolves conflicts between ID3 tags and file path metadata using heuristics.
/// Provides primary and alternate values for fallback searches.
#[derive(Debug, Clone)]
pub(crate) struct ReconciledMetadata {
    /// Final artist name to use for search.
    pub(crate) artist: String,
    /// Final album name to use for search.
    pub(crate) album: String,
    /// Strategy used to resolve conflicts.
    pub(crate) strategy: ReconciliationStrategy,
    /// Confidence level in the reconciled metadata.
    pub(crate) confidence: MetadataConfidence,
    /// Source of the artist name.
    pub(crate) artist_source: MetadataSource,
    /// Source of the album name.
    pub(crate) album_source: MetadataSource,
    /// Alternative artist name for fallback searches.
    pub(crate) alternate_artist: Option<String>,
    /// Alternative album name for fallback searches.
    pub(crate) alternate_album: Option<String>,
    /// Whether MusicBrainz IDs were found in tags.
    pub(crate) has_musicbrainz_ids: bool,
    /// Estimated track count from ID3 comment field.
    pub(crate) estimated_track_count: Option<usize>,
}

/// Strategy used to reconcile ID3 tags with path-derived metadata
#[derive(Debug, Clone, Copy)]
pub(crate) enum ReconciliationStrategy {
    /// ID3 and path agree on values.
    DirectMatch,
    /// Partial overlap, chose one source.
    PartialMatch,
    /// Sources disagree, applied heuristics.
    Conflict,
    /// One source missing, used other.
    GapFill,
    /// ID3 tags missing, used path only.
    PathOnly,
}

/// Confidence level in reconciled metadata
#[derive(Debug, Clone, Copy)]
pub(crate) enum MetadataConfidence {
    /// DirectMatch or strong heuristic confidence.
    High,
    /// PartialMatch or moderate heuristic confidence.
    Medium,
    /// Conflict or GapFill.
    Low,
}

/// Source of a metadata field
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum MetadataSource {
    /// Value from ID3 tags.
    ID3,
    /// Value derived from file path.
    Path,
    /// Both sources agree.
    Both,
    /// Sources conflict (heuristic was applied).
    Conflict,
}

// =============================================================================
// Single-Track Discriminator Types
// =============================================================================

/// Confidence level for single-track detection
///
/// Indicates how confident we are that a file contains a single track
/// rather than a full album.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SingleTrackConfidence {
    /// High confidence (score >= 2.0 or definitive indicator)
    High,
    /// Medium confidence (score >= 1.0)
    Medium,
    /// Low confidence (score >= 0.5)
    Low,
    /// Unlikely to be single track (score < 0.5)
    Unlikely,
}

impl std::fmt::Display for SingleTrackConfidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SingleTrackConfidence::High => write!(f, "High"),
            SingleTrackConfidence::Medium => write!(f, "Medium"),
            SingleTrackConfidence::Low => write!(f, "Low"),
            SingleTrackConfidence::Unlikely => write!(f, "Unlikely"),
        }
    }
}

/// Results from single-track analysis
///
/// Aggregates scores from 5 detection layers to determine if file contains
/// a single track vs full album.
///
/// ## Layers
/// 1. Filename pattern (track number prefix like "01 -")
/// 2. Directory file count (many files = individual tracks)
/// 3. ID3 track number/total tags
/// 4. Duration heuristics (single tracks typically < 8 min, albums > 20 min)
/// 5. Silence gap count (post-decode, albums have 3+ gaps)
///
/// ## Scoring
/// - Each layer contributes positive score (single track indicator) or
///   negative score (counter-indicator)
/// - Pre-decode score: layers 1-4 (before audio decode)
/// - Final score: all 5 layers (after audio decode)
/// - Threshold: score >= 1.5 = likely single track
#[derive(Debug, Clone)]
pub(crate) struct SingleTrackAnalysis {
    // Layer 1: Filename pattern
    pub(crate) filename_score: f64,
    pub(crate) filename_match: Option<String>, // The matched pattern, if any

    // Layer 2: Directory file count
    pub(crate) dir_count_score: f64,
    pub(crate) dir_audio_files: usize,

    // Layer 3: ID3 track tags
    pub(crate) id3_track_score: f64,
    pub(crate) id3_track_info: Option<String>, // e.g., "4/12" or "4/?"

    // Layer 4: Duration
    pub(crate) duration_score: f64,
    pub(crate) duration_mins: Option<f64>,

    // Layer 5: Silence gaps (post-decode only)
    pub(crate) silence_gap_score: Option<f64>, // None until post-decode
    pub(crate) silence_gap_count: Option<usize>,

    // Aggregated results
    pub(crate) pre_decode_score: f64,
    pub(crate) final_score: f64,
    pub(crate) confidence: SingleTrackConfidence,
    pub(crate) is_likely_single_track: bool,
}

impl SingleTrackAnalysis {
    /// Create new analysis with all scores initialized to zero
    pub(crate) fn new() -> Self {
        Self {
            filename_score: 0.0,
            filename_match: None,
            dir_count_score: 0.0,
            dir_audio_files: 0,
            id3_track_score: 0.0,
            id3_track_info: None,
            duration_score: 0.0,
            duration_mins: None,
            silence_gap_score: None,
            silence_gap_count: None,
            pre_decode_score: 0.0,
            final_score: 0.0,
            confidence: SingleTrackConfidence::Unlikely,
            is_likely_single_track: false,
        }
    }

    /// Compute confidence level from aggregated score
    ///
    /// # Thresholds
    /// - High: score >= 2.0
    /// - Medium: score >= 1.0
    /// - Low: score >= 0.5
    /// - Unlikely: score < 0.5
    pub(crate) fn compute_confidence(score: f64) -> SingleTrackConfidence {
        if score >= 2.0 {
            SingleTrackConfidence::High
        } else if score >= 1.0 {
            SingleTrackConfidence::Medium
        } else if score >= 0.5 {
            SingleTrackConfidence::Low
        } else {
            SingleTrackConfidence::Unlikely
        }
    }

    /// Update pre_decode_score, final_score, confidence, and is_likely_single_track
    ///
    /// Called after each layer updates to recalculate aggregates.
    /// Pre-decode includes layers 1-4, final includes all 5 layers.
    pub(crate) fn update_aggregates(&mut self) {
        self.pre_decode_score =
            self.filename_score + self.dir_count_score + self.id3_track_score + self.duration_score;

        self.final_score = self.pre_decode_score + self.silence_gap_score.unwrap_or(0.0);
        self.confidence = Self::compute_confidence(self.final_score);
        self.is_likely_single_track =
            self.final_score >= crate::constants::SINGLE_TRACK_SCORE_THRESHOLD;
    }
}

//! Album Matching Types
//!
//! **[PLAN026/PLAN030]** Types for album matching results
//!
//! These types bridge the am28 matching algorithm output to the
//! ContentTypeClassifier's ClassificationResult format.
//!
//! ## Type Organization
//!
//! - **Core Types**: MatchingStage, MatchedTrack, Edition, AlbumMatchResult
//! - **Cache Types**: CacheMode, CacheConfig, CachedSearch, CachedRelease
//! - **MusicBrainz API Types**: MBSearchResponse, MBRelease, MBReleaseDetails, etc.
//! - **Stage Result Types**: CandidateTestResult, EditionTestResult
//! - **Metadata Types**: ID3Metadata, ReconciledMetadata
//! - **Single-Track Types**: SingleTrackConfidence, SingleTrackAnalysis

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Matching stage identifier
///
/// Indicates which stage of the am28 algorithm produced the match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchingStage {
    /// Stage 2: Parameter grid search (silence detection optimization)
    Stage2,
    /// Stage 3: Dynamic programming assembly for over-segmented tracks
    Stage3,
    /// Stage 4: Edition-guided quiet spot detection
    Stage4,
    /// Stage 5: Extra track merging
    Stage5,
}

impl MatchingStage {
    /// Convert to database string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchingStage::Stage2 => "stage2",
            MatchingStage::Stage3 => "stage3",
            MatchingStage::Stage4 => "stage4",
            MatchingStage::Stage5 => "stage5",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "stage2" => Some(MatchingStage::Stage2),
            "stage3" => Some(MatchingStage::Stage3),
            "stage4" => Some(MatchingStage::Stage4),
            "stage5" => Some(MatchingStage::Stage5),
            _ => None,
        }
    }
}

impl std::fmt::Display for MatchingStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A matched track within an album
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedTrack {
    /// 1-based track index
    pub track_number: usize,
    /// Disc number (1 for single-disc releases)
    pub disc_number: usize,
    /// MusicBrainz Recording MBID
    pub recording_mbid: String,
    /// Track title from MusicBrainz
    pub title: String,
    /// Detected duration in seconds
    pub detected_duration: f64,
    /// Expected duration from MusicBrainz (seconds)
    pub expected_duration: f64,
    /// Timing error (seconds, absolute)
    pub timing_error: f64,
    /// Whether timing is within tolerance
    pub within_tolerance: bool,
}

/// A MusicBrainz edition (specific release)
///
/// Groups releases by track count and duration pattern.
/// An edition represents a unique combination of track count and duration
/// signature, which may correspond to multiple MusicBrainz release MBIDs.
#[derive(Debug, Clone)]
pub struct Edition {
    /// MusicBrainz Release MBID
    pub release_mbid: String,
    /// Album title from MusicBrainz
    pub title: String,
    /// Artist name from MusicBrainz
    pub artist: String,
    /// Artist credit string
    pub artist_credit: Option<String>,
    /// Release country
    pub country: Option<String>,
    /// Release status (Official, Bootleg, etc.)
    pub status: Option<String>,
    /// Number of tracks
    pub track_count: usize,
    /// Expected track durations (seconds)
    pub track_durations: Vec<f64>,
    /// Recording MBIDs for each track
    pub recording_mbids: Vec<String>,
    // --- Fields added from am28 (PLAN030) ---
    /// Track durations in milliseconds (from MusicBrainz)
    pub durations: Vec<u32>,
    /// Rank 1-N based on name similarity to source (1 = best match)
    pub name_distance_rank: Option<usize>,
    /// Overall name distance score (lower = better match)
    pub name_distance_score: Option<f64>,
}

/// Result of album matching
#[derive(Debug, Clone, Serialize)]
pub struct AlbumMatchResult {
    /// Whether a match was found
    pub matched: bool,
    /// MusicBrainz Release MBID (if matched)
    pub release_mbid: Option<String>,
    /// Artist name from MusicBrainz
    pub matched_artist: Option<String>,
    /// Album title from MusicBrainz
    pub matched_album: Option<String>,
    /// Which stage produced the match
    pub matching_stage: Option<MatchingStage>,
    /// Match percentage (0-100)
    pub match_percentage: f64,
    /// Mean timing error (seconds)
    pub mean_error_seconds: f64,
    /// Number of tracks matched within tolerance
    pub matched_track_count: usize,
    /// Total expected tracks
    pub expected_track_count: usize,
    /// Total detected tracks
    pub detected_track_count: usize,
    /// Per-track matching details
    pub tracks: Vec<MatchedTrack>,
    /// Confidence level string: "Excellent", "Good", "Fair", "Poor"
    pub confidence: String,
    /// Whether artist verification passed
    pub artist_verified: bool,
    /// Jaro-Winkler similarity between source and matched artist
    pub artist_similarity: f64,
    /// Best silence detection threshold (dB) if applicable
    pub best_threshold_db: Option<f64>,
    /// Best minimum silence duration (seconds) if applicable
    pub best_min_duration_secs: Option<f64>,
    /// Status message
    pub status: String,
}

impl AlbumMatchResult {
    /// Create a "no match" result
    pub fn no_match(status: String) -> Self {
        Self {
            matched: false,
            release_mbid: None,
            matched_artist: None,
            matched_album: None,
            matching_stage: None,
            match_percentage: 0.0,
            mean_error_seconds: 0.0,
            matched_track_count: 0,
            expected_track_count: 0,
            detected_track_count: 0,
            tracks: Vec::new(),
            confidence: "Poor".to_string(),
            artist_verified: false,
            artist_similarity: 0.0,
            best_threshold_db: None,
            best_min_duration_secs: None,
            status,
        }
    }

    /// Check if this is a high-confidence match (>=80% match percentage)
    pub fn is_high_confidence(&self) -> bool {
        self.matched && self.match_percentage >= 80.0
    }

    /// Check if this is a full album match (100% of tracks matched)
    pub fn is_full_album(&self) -> bool {
        self.matched
            && self.matched_track_count == self.expected_track_count
            && self.match_percentage >= 100.0
    }

    /// Check if this is a partial album match
    pub fn is_partial_album(&self) -> bool {
        self.matched && !self.is_full_album()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matching_stage_roundtrip() {
        let stages = [
            MatchingStage::Stage2,
            MatchingStage::Stage3,
            MatchingStage::Stage4,
            MatchingStage::Stage5,
        ];

        for stage in stages {
            let s = stage.as_str();
            let parsed = MatchingStage::from_str(s);
            assert_eq!(parsed, Some(stage));
        }
    }

    #[test]
    fn test_album_match_result_no_match() {
        let result = AlbumMatchResult::no_match("No MusicBrainz candidates found".to_string());
        assert!(!result.matched);
        assert!(!result.is_high_confidence());
        assert!(!result.is_full_album());
    }

    #[test]
    fn test_edition_with_new_fields() {
        let edition = Edition {
            release_mbid: "test-mbid".to_string(),
            title: "Test Album".to_string(),
            artist: "Test Artist".to_string(),
            artist_credit: None,
            country: Some("US".to_string()),
            status: Some("Official".to_string()),
            track_count: 10,
            track_durations: vec![180.0, 200.0, 220.0],
            recording_mbids: vec!["rec1".to_string(), "rec2".to_string(), "rec3".to_string()],
            durations: vec![180000, 200000, 220000],
            name_distance_rank: Some(1),
            name_distance_score: Some(0.95),
        };
        assert_eq!(edition.track_count, 10);
        assert_eq!(edition.durations.len(), 3);
        assert_eq!(edition.name_distance_rank, Some(1));
    }

    #[test]
    fn test_cache_mode_default() {
        let config = CacheConfig::default();
        assert!(matches!(config.mode, CacheMode::ReadWrite));
    }

    #[test]
    fn test_candidate_test_result() {
        let result = CandidateTestResult {
            percentage: 100.0,
            detected_durations: vec![180.5, 200.2],
            matched_count: 2,
            expected_count: 2,
            errors: vec![0.5, 0.2],
        };
        assert_eq!(result.matched_count, result.expected_count);
    }
}

// =============================================================================
// Cache Types (PLAN030 - MusicBrainz Caching)
// =============================================================================

/// Cache mode configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheMode {
    /// Live API only, no caching
    Disabled,
    /// Cache hits use cache, misses query API and store (default)
    ReadWrite,
    /// Cache only, error on miss (for algorithm tuning)
    ReadOnly,
}

impl Default for CacheMode {
    fn default() -> Self {
        CacheMode::ReadWrite
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Cache operation mode
    pub mode: CacheMode,
    /// Directory for cache files
    pub cache_dir: PathBuf,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            mode: CacheMode::ReadWrite,
            cache_dir: PathBuf::from(".cache/musicbrainz"),
        }
    }
}

/// Cached search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSearch {
    /// Original query string (for verification)
    pub query: String,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Search response data
    pub response: MBSearchResponse,
}

/// Cached release details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRelease {
    /// Release MBID (for verification)
    pub mbid: String,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Release details data
    pub details: MBReleaseDetails,
}

/// Cache metadata (cache/musicbrainz/metadata.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// Cache version identifier
    pub version: String,
    /// ISO 8601 timestamp of cache creation
    pub created: String,
    /// Number of cached search responses
    pub search_count: usize,
    /// Number of cached release details
    pub release_count: usize,
    /// ISO 8601 timestamp of last update
    pub last_updated: String,
}

/// Cache statistics tracking
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Search cache hits
    pub search_hits: AtomicU64,
    /// Search cache misses
    pub search_misses: AtomicU64,
    /// Release cache hits
    pub release_hits: AtomicU64,
    /// Release cache misses
    pub release_misses: AtomicU64,
}

impl CacheStats {
    /// Create new cache stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a search cache hit
    pub fn record_search_hit(&self) {
        self.search_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a search cache miss
    pub fn record_search_miss(&self) {
        self.search_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a release cache hit
    pub fn record_release_hit(&self) {
        self.release_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a release cache miss
    pub fn record_release_miss(&self) {
        self.release_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total cache hits
    pub fn total_hits(&self) -> u64 {
        self.search_hits.load(Ordering::Relaxed) + self.release_hits.load(Ordering::Relaxed)
    }

    /// Get total cache misses
    pub fn total_misses(&self) -> u64 {
        self.search_misses.load(Ordering::Relaxed) + self.release_misses.load(Ordering::Relaxed)
    }

    /// Calculate cache hit rate (0.0-1.0)
    pub fn hit_rate(&self) -> f64 {
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
pub struct MBSearchResponse {
    /// List of releases matching the search query
    pub releases: Vec<MBRelease>,
}

/// A MusicBrainz release (album) from search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MBRelease {
    /// MusicBrainz release ID (MBID)
    pub id: String,
    /// Release title
    pub title: String,
    /// Artist credits for this release
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<MBArtistCredit>>,
    /// Country of release (e.g., "US", "GB")
    pub country: Option<String>,
    /// Release status (e.g., "Official", "Bootleg")
    pub status: Option<String>,
    /// Physical packaging type (e.g., "Jewel Case")
    pub packaging: Option<String>,
}

/// Artist credit entry linking an artist to a release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MBArtistCredit {
    /// The credited name (may differ from artist.name)
    pub name: String,
    /// The artist information
    pub artist: MBArtist,
}

/// MusicBrainz artist information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MBArtist {
    /// Artist name
    pub name: String,
}

/// Detailed release information including track listings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MBReleaseDetails {
    /// MusicBrainz release ID (MBID)
    pub id: String,
    /// Release title
    pub title: String,
    /// Media (discs) in this release
    pub media: Vec<MBMedia>,
    /// Release status (e.g., "Official", "Bootleg")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Release country code (e.g., "US", "GB")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Release date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Barcode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barcode: Option<String>,
}

/// A single medium (disc) within a release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MBMedia {
    /// Tracks on this medium
    pub tracks: Vec<MBTrack>,
    /// Format type (e.g., "CD", "Vinyl", "Digital Media")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Number of tracks on this medium
    #[serde(default)]
    pub track_count: usize,
}

/// MusicBrainz recording information (linked to a track)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MBRecording {
    /// MusicBrainz Recording MBID
    pub id: String,
    /// Recording title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Recording length in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    /// Artist credits for this recording
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_credit: Option<Vec<MBArtistCredit>>,
}

/// A single track on a medium
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MBTrack {
    /// Track ID
    #[serde(default)]
    pub id: String,
    /// Track number as string (e.g., "1", "A1")
    #[serde(default)]
    pub number: String,
    /// Track title
    #[serde(default)]
    pub title: String,
    /// Track length in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    /// Track position on the medium
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
    /// Linked recording (contains Recording MBID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording: Option<MBRecording>,
}

// =============================================================================
// Stage Result Types
// =============================================================================

/// Result from testing a single parameter combination
#[derive(Debug, Clone)]
pub struct CandidateTestResult {
    /// Percentage of expected tracks that matched (0-100)
    pub percentage: f64,
    /// Detected track durations from audio analysis (seconds)
    pub detected_durations: Vec<f64>,
    /// Number of tracks within tolerance
    pub matched_count: usize,
    /// Total expected tracks
    pub expected_count: usize,
    /// Per-track timing errors (seconds)
    pub errors: Vec<f64>,
}

/// Track matching comparison result
#[derive(Debug, Clone, Serialize)]
pub struct TrackMatch {
    /// Duration detected from audio analysis (seconds)
    pub detected_duration: f64,
    /// Expected duration from MusicBrainz (seconds)
    pub expected_duration: u32,
    /// Absolute difference between detected and expected (seconds)
    pub error: f64,
    /// Whether error is within tolerance
    pub matches: bool,
}

/// A detected track with no corresponding MusicBrainz entry
#[derive(Debug, Clone, Serialize)]
pub struct ExtraTrack {
    /// 1-based index in detected tracks
    pub track_index: usize,
    /// Duration in seconds
    pub duration: f64,
    /// Description of why this is extra
    pub description: String,
}

/// Pre-computed silence detection cache
///
/// Matrix of track durations for all parameter combinations.
/// Outer Vec indexed by (threshold_idx * num_min_durations + min_duration_idx).
/// Inner Vec contains track durations in seconds for that parameter combination.
pub type SilenceCache = Vec<Vec<f64>>;

/// Over-segmented candidate collected during Stage 2 for Stage 3 assembly
#[derive(Debug, Clone)]
pub struct OverSegmentedCandidate {
    /// Detected track durations in seconds
    pub durations: Vec<f64>,
    /// Silence detection threshold used (dB)
    pub threshold_db: f64,
    /// Minimum silence duration used (seconds)
    pub min_duration_secs: f64,
    /// Number of tracks detected
    pub track_count: usize,
}

/// Result of testing a single edition through all stages
#[derive(Debug, Clone)]
pub struct EditionTestResult {
    /// Index of the edition in the editions list
    pub edition_idx: usize,
    /// Best percentage achieved for this edition
    pub best_percentage: f64,
    /// Best result found across all stages
    pub best_result: Option<CandidateTestResult>,
    /// Which stage produced the best result
    pub best_stage: String,
    /// Best threshold from stage 2 (if applicable)
    pub best_threshold: Option<f64>,
    /// Best min_duration from stage 2 (if applicable)
    pub best_min_duration: Option<f64>,
    /// Expected durations from this edition (milliseconds)
    pub expected_durations: Vec<u32>,
    /// Log messages accumulated during processing
    pub log_messages: Vec<String>,
}

// =============================================================================
// Metadata Types
// =============================================================================

/// ID3 metadata extracted from audio file tags
#[derive(Debug, Clone, Default)]
pub struct ID3Metadata {
    /// Artist name from ID3 tag
    pub artist: Option<String>,
    /// Album name from ID3 tag
    pub album: Option<String>,
    /// Release date from ID3 tag
    pub date: Option<String>,
    /// Genre from ID3 tag
    pub genre: Option<String>,
    /// MusicBrainz album ID if present in tags
    pub musicbrainz_albumid: Option<String>,
    /// MusicBrainz artist ID if present in tags
    pub musicbrainz_artistid: Option<String>,
    /// Comment field (may contain track count info)
    pub comment: Option<String>,
    /// All ID3 tags as key-value pairs
    pub all_tags: HashMap<String, String>,
}

/// Metadata after reconciling ID3 tags with path-derived information
#[derive(Debug, Clone)]
pub struct ReconciledMetadata {
    /// Final artist name to use for search
    pub artist: String,
    /// Final album name to use for search
    pub album: String,
    /// Strategy used to resolve conflicts
    pub strategy: ReconciliationStrategy,
    /// Confidence level in the reconciled metadata
    pub confidence: MetadataConfidence,
    /// Source of the artist name
    pub artist_source: MetadataSource,
    /// Source of the album name
    pub album_source: MetadataSource,
    /// Alternative artist name for fallback searches
    pub alternate_artist: Option<String>,
    /// Alternative album name for fallback searches
    pub alternate_album: Option<String>,
    /// Whether MusicBrainz IDs were found in tags
    pub has_musicbrainz_ids: bool,
    /// Estimated track count from ID3 comment field
    pub estimated_track_count: Option<usize>,
}

/// Strategy used to reconcile ID3 tags with path-derived metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationStrategy {
    /// ID3 and path agree on values
    DirectMatch,
    /// Partial overlap, chose one source
    PartialMatch,
    /// Sources disagree, applied heuristics
    Conflict,
    /// One source missing, used other
    GapFill,
    /// ID3 tags missing, used path only
    PathOnly,
}

/// Confidence level in reconciled metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataConfidence {
    /// DirectMatch or strong heuristic confidence
    High,
    /// PartialMatch or moderate heuristic confidence
    Medium,
    /// Conflict or GapFill
    Low,
}

/// Source of a metadata field
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataSource {
    /// Value from ID3 tags
    ID3,
    /// Value derived from file path
    Path,
    /// Both sources agree
    Both,
    /// Sources conflict (heuristic was applied)
    Conflict,
}

// =============================================================================
// Single-Track Discriminator Types
// =============================================================================

/// Confidence level for single-track detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingleTrackConfidence {
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
#[derive(Debug, Clone)]
pub struct SingleTrackAnalysis {
    // Layer 1: Filename pattern
    /// Score from filename pattern analysis
    pub filename_score: f64,
    /// The matched pattern, if any
    pub filename_match: Option<String>,

    // Layer 2: Directory file count
    /// Score from directory analysis
    pub dir_count_score: f64,
    /// Number of audio files in directory
    pub dir_audio_files: usize,

    // Layer 3: ID3 track tags
    /// Score from ID3 tags
    pub id3_track_score: f64,
    /// Track info (e.g., "4/12" or "4/?")
    pub id3_track_info: Option<String>,

    // Layer 4: Duration
    /// Score from duration analysis
    pub duration_score: f64,
    /// Duration in minutes
    pub duration_mins: Option<f64>,

    // Layer 5: Silence gaps (post-decode only)
    /// Score from silence gap analysis (None until post-decode)
    pub silence_gap_score: Option<f64>,
    /// Number of silence gaps detected
    pub silence_gap_count: Option<usize>,

    // Aggregated results
    /// Pre-decode aggregate score
    pub pre_decode_score: f64,
    /// Final aggregate score (includes post-decode)
    pub final_score: f64,
    /// Confidence level
    pub confidence: SingleTrackConfidence,
    /// Whether this is likely a single track
    pub is_likely_single_track: bool,
}

impl Default for SingleTrackAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

impl SingleTrackAnalysis {
    /// Create new analysis with all scores initialized to zero
    pub fn new() -> Self {
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
    pub fn compute_confidence(score: f64) -> SingleTrackConfidence {
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
    pub fn update_aggregates(&mut self, single_track_threshold: f64) {
        self.pre_decode_score =
            self.filename_score + self.dir_count_score + self.id3_track_score + self.duration_score;

        self.final_score = self.pre_decode_score + self.silence_gap_score.unwrap_or(0.0);
        self.confidence = Self::compute_confidence(self.final_score);
        self.is_likely_single_track = self.final_score >= single_track_threshold;
    }
}

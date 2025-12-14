//! Content Type Classification Service
//!
//! **[PLAN026]** Step 6: Content Type Determination
//!
//! Classifies audio files into content types based on:
//! - Duration-based triage
//! - Silence detection for segment estimation
//! - AcoustID fingerprint matching (single-song path)
//! - MusicBrainz release matching (album path)
//!
//! # Content Types
//! - `SINGLE_SONG` - Single track with MusicBrainz Recording MBID
//! - `FULL_ALBUM` - Complete album with MusicBrainz Release MBID
//! - `PARTIAL_ALBUM` - Partial album match (some tracks identified)
//! - `MULTIPLE_SONGS` - Multiple unrelated songs in one file
//! - `NOT_IN_MUSICBRAINZ` - Audio not found in MusicBrainz database
//! - `IDENTIFICATION_FAILED` - Unable to determine content type
//!
//! # Algorithm
//! Per refactor1126.md Step 6:
//! 1. Duration-based triage: <12min → single; 12-25min → dual; >25min → album
//! 2. Quick silence scan for segment estimation
//! 3. Single-song path via Chromaprint → AcoustID
//! 4. Album path via metadata → MusicBrainz Release → edition matching

use crate::fusion::identity_resolver::IdentityResolver;
use crate::services::acoustid_client::{AcoustIDRecording, AcoustIDResponse};
use crate::services::metadata_extractor::AudioMetadata;
use crate::services::recording_matcher::{RecordingCandidate, RecordingMatcher};
use crate::types::{Fusion, IdentityExtraction};
use crate::utils::string_similarity::jaro_winkler_similarity;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fmt;
use std::sync::Arc;

/// Threshold below which both artist AND title mismatch indicates AcoustID error
/// Based on cross-validation analysis: 6/25 disagreements are double-mismatch cases
/// where AcoustID returns completely wrong recordings
const DOUBLE_MISMATCH_THRESHOLD: f64 = 0.70;

/// Content type classification for audio files
///
/// **[REQ-CLASS-001..006]** Six classification outcomes per PLAN026
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    /// Single track with MusicBrainz Recording MBID
    /// Confidence ≥ 0.80 from AcoustID match
    SingleSong,

    /// Complete album with MusicBrainz Release MBID
    /// All tracks match expected durations within tolerance
    FullAlbum,

    /// Partial album match
    /// Some tracks identified but not complete match
    PartialAlbum,

    /// Multiple unrelated songs in one file
    /// Different artists/releases per detected segment
    MultipleSongs,

    /// Audio content not found in MusicBrainz database
    /// Valid audio but no MB match after exhaustive search
    NotInMusicbrainz,

    /// Unable to determine content type
    /// Processing error or ambiguous results
    IdentificationFailed,
}

impl ContentType {
    /// Convert to database string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::SingleSong => "SINGLE_SONG",
            ContentType::FullAlbum => "FULL_ALBUM",
            ContentType::PartialAlbum => "PARTIAL_ALBUM",
            ContentType::MultipleSongs => "MULTIPLE_SONGS",
            ContentType::NotInMusicbrainz => "NOT_IN_MUSICBRAINZ",
            ContentType::IdentificationFailed => "IDENTIFICATION_FAILED",
        }
    }

    /// Parse from database string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "SINGLE_SONG" => Some(ContentType::SingleSong),
            "FULL_ALBUM" => Some(ContentType::FullAlbum),
            "PARTIAL_ALBUM" => Some(ContentType::PartialAlbum),
            "MULTIPLE_SONGS" => Some(ContentType::MultipleSongs),
            "NOT_IN_MUSICBRAINZ" => Some(ContentType::NotInMusicbrainz),
            "IDENTIFICATION_FAILED" => Some(ContentType::IdentificationFailed),
            _ => None,
        }
    }

    /// Check if this is a completed status (no further processing needed)
    pub fn is_completed(&self) -> bool {
        matches!(
            self,
            ContentType::NotInMusicbrainz | ContentType::IdentificationFailed
        )
    }

    /// Check if this content type requires passage segmentation
    pub fn requires_segmentation(&self) -> bool {
        matches!(
            self,
            ContentType::FullAlbum | ContentType::PartialAlbum | ContentType::MultipleSongs
        )
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Duration-based triage result
///
/// **[REQ-ALG-001]** Per refactor1126.md:
/// - <12 min → single-song path
/// - 12-25 min → dual-path (try both)
/// - >25 min → album path
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriagePath {
    /// File < 12 minutes - likely single song
    SingleSong,
    /// File 12-25 minutes - could be single or album
    DualPath,
    /// File > 25 minutes - likely album
    Album,
}

impl TriagePath {
    /// Triage based on duration in seconds
    ///
    /// # Arguments
    /// * `duration_seconds` - Total file duration in seconds
    ///
    /// # Returns
    /// Appropriate triage path based on duration thresholds
    pub fn from_duration(duration_seconds: f64) -> Self {
        const SINGLE_THRESHOLD_SECS: f64 = 12.0 * 60.0; // 12 minutes
        const ALBUM_THRESHOLD_SECS: f64 = 25.0 * 60.0; // 25 minutes

        if duration_seconds < SINGLE_THRESHOLD_SECS {
            TriagePath::SingleSong
        } else if duration_seconds > ALBUM_THRESHOLD_SECS {
            TriagePath::Album
        } else {
            TriagePath::DualPath
        }
    }
}

/// Match confidence level for content type classification
///
/// **[AMB-01, AMB-03]** Per refactor1126.md Algorithm Constants
/// Named MatchConfidence to avoid conflict with passage_song_matcher::ConfidenceLevel
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MatchConfidence {
    /// 95%+ match
    Excellent,
    /// 75-94% match
    Good,
    /// 50-74% match
    Fair,
    /// <50% match
    Poor,
}

impl MatchConfidence {
    /// Create from numeric confidence value (0.0-1.0)
    pub fn from_value(confidence: f64) -> Self {
        if confidence >= 0.95 {
            MatchConfidence::Excellent
        } else if confidence >= 0.75 {
            MatchConfidence::Good
        } else if confidence >= 0.50 {
            MatchConfidence::Fair
        } else {
            MatchConfidence::Poor
        }
    }

    /// Convert to database string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchConfidence::Excellent => "Excellent",
            MatchConfidence::Good => "Good",
            MatchConfidence::Fair => "Fair",
            MatchConfidence::Poor => "Poor",
        }
    }

    /// Check if confidence meets high threshold (≥80%)
    pub fn is_high_confidence(&self) -> bool {
        matches!(self, MatchConfidence::Excellent | MatchConfidence::Good)
    }
}

impl fmt::Display for MatchConfidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Result of content type classification
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// Determined content type
    pub content_type: ContentType,
    /// Classification confidence level
    pub confidence: MatchConfidence,
    /// Numeric confidence value (0.0-1.0)
    pub confidence_value: f64,
    /// MusicBrainz Release MBID (for album types)
    pub release_mbid: Option<String>,
    /// MusicBrainz Recording MBID (for single song)
    pub recording_mbid: Option<String>,
    /// Match percentage for album matching
    pub match_percentage: Option<f64>,
    /// Artist verification result
    pub artist_verified: bool,
    /// Which matching stage produced this result (for albums)
    pub matching_stage: Option<String>,
}

impl ClassificationResult {
    /// Create a single song classification result
    pub fn single_song(recording_mbid: String, confidence: f64) -> Self {
        Self {
            content_type: ContentType::SingleSong,
            confidence: MatchConfidence::from_value(confidence),
            confidence_value: confidence,
            release_mbid: None,
            recording_mbid: Some(recording_mbid),
            match_percentage: Some(100.0),
            artist_verified: true,
            matching_stage: Some("acoustid".to_string()),
        }
    }

    /// Create a single song classification result with custom matching stage
    ///
    /// **[SSI-MB-010]** Supports MusicBrainz fallback and multi-source fusion
    pub fn single_song_with_stage(recording_mbid: String, confidence: f64, stage: &str) -> Self {
        Self {
            content_type: ContentType::SingleSong,
            confidence: MatchConfidence::from_value(confidence),
            confidence_value: confidence,
            release_mbid: None,
            recording_mbid: Some(recording_mbid),
            match_percentage: Some(100.0),
            artist_verified: true,
            matching_stage: Some(stage.to_string()),
        }
    }

    /// Create a not-in-musicbrainz result
    pub fn not_in_musicbrainz() -> Self {
        Self {
            content_type: ContentType::NotInMusicbrainz,
            confidence: MatchConfidence::Fair,
            confidence_value: 0.5,
            release_mbid: None,
            recording_mbid: None,
            match_percentage: None,
            artist_verified: false,
            matching_stage: None,
        }
    }

    /// Create an identification failed result
    pub fn identification_failed() -> Self {
        Self {
            content_type: ContentType::IdentificationFailed,
            confidence: MatchConfidence::Poor,
            confidence_value: 0.0,
            release_mbid: None,
            recording_mbid: None,
            match_percentage: None,
            artist_verified: false,
            matching_stage: None,
        }
    }
}

/// High confidence threshold for classification acceptance
pub const HIGH_CONFIDENCE_THRESHOLD: f64 = 0.80;

/// Result of validating AcoustID result against ID3 metadata
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the validation passed (metadata agrees)
    pub passed: bool,
    /// Artist similarity score (0.0-1.0)
    pub artist_similarity: f64,
    /// Title similarity score (0.0-1.0)
    pub title_similarity: f64,
    /// The recording that was validated
    pub recording: Option<AcoustIDRecording>,
}

impl ValidationResult {
    /// Check if this is a double-mismatch (both artist AND title below threshold)
    ///
    /// Double-mismatch indicates likely AcoustID database error rather than
    /// genuine match. Based on cross-validation: 6/25 disagreements are this type.
    pub fn is_double_mismatch(&self) -> bool {
        self.artist_similarity < DOUBLE_MISMATCH_THRESHOLD
            && self.title_similarity < DOUBLE_MISMATCH_THRESHOLD
    }
}

/// Validate an AcoustID response against ID3 metadata
///
/// **[SSI-VAL-030]** Double-mismatch rejection rule
///
/// Compares the best AcoustID recording against ID3 metadata using
/// Jaro-Winkler similarity. If both artist AND title have similarity
/// below DOUBLE_MISMATCH_THRESHOLD (0.70), the match is rejected as
/// a likely AcoustID database error.
///
/// # Arguments
/// * `response` - AcoustID lookup response
/// * `id3_artist` - Artist from ID3 tags
/// * `id3_title` - Title from ID3 tags
///
/// # Returns
/// ValidationResult indicating whether the match should be accepted
pub fn validate_acoustid_against_id3(
    response: &AcoustIDResponse,
    id3_artist: Option<&str>,
    id3_title: Option<&str>,
) -> ValidationResult {
    // Find best scoring result with recordings
    let best_result = response
        .results
        .iter()
        .filter(|r| r.score >= HIGH_CONFIDENCE_THRESHOLD)
        .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));

    let Some(result) = best_result else {
        return ValidationResult {
            passed: false,
            artist_similarity: 0.0,
            title_similarity: 0.0,
            recording: None,
        };
    };

    let Some(recordings) = &result.recordings else {
        return ValidationResult {
            passed: false,
            artist_similarity: 0.0,
            title_similarity: 0.0,
            recording: None,
        };
    };

    let Some(recording) = recordings.first() else {
        return ValidationResult {
            passed: false,
            artist_similarity: 0.0,
            title_similarity: 0.0,
            recording: None,
        };
    };

    // Extract artist name from AcoustID response
    let acoustid_artist = recording
        .artists
        .as_ref()
        .and_then(|artists| artists.first())
        .map(|a| a.name.as_str());

    let acoustid_title = recording.title.as_deref();

    // Calculate similarities
    // When ID3 metadata is missing, we can't validate - return 1.0 to avoid false rejection
    let artist_similarity = match (id3_artist, acoustid_artist) {
        (Some(id3), Some(aid)) => jaro_winkler_similarity(id3, aid),
        (None, _) => 1.0, // ID3 missing = can't validate, assume no conflict
        (_, None) => 1.0, // AcoustID missing = can't validate, assume no conflict
    };

    let title_similarity = match (id3_title, acoustid_title) {
        (Some(id3), Some(aid)) => jaro_winkler_similarity(id3, aid),
        (None, _) => 1.0, // ID3 missing = can't validate, assume no conflict
        (_, None) => 1.0, // AcoustID missing = can't validate, assume no conflict
    };

    // Double-mismatch check: only trigger if BOTH have valid comparisons
    // If either ID3 artist or title is missing, we can't reliably detect double-mismatch
    let can_validate_artist = id3_artist.is_some() && acoustid_artist.is_some();
    let can_validate_title = id3_title.is_some() && acoustid_title.is_some();

    let is_double_mismatch = can_validate_artist
        && can_validate_title
        && artist_similarity < DOUBLE_MISMATCH_THRESHOLD
        && title_similarity < DOUBLE_MISMATCH_THRESHOLD;

    if is_double_mismatch {
        tracing::warn!(
            id3_artist = ?id3_artist,
            id3_title = ?id3_title,
            acoustid_artist = ?acoustid_artist,
            acoustid_title = ?acoustid_title,
            artist_sim = artist_similarity,
            title_sim = title_similarity,
            "DOUBLE-MISMATCH: Rejecting AcoustID result as likely database error"
        );
    }

    ValidationResult {
        passed: !is_double_mismatch,
        artist_similarity,
        title_similarity,
        recording: Some(recording.clone()),
    }
}

/// Classification error types
#[derive(Debug)]
pub enum ClassificationError {
    /// Fingerprinting failed
    FingerprintError(String),
    /// AcoustID lookup failed
    AcoustIdError(String),
    /// MusicBrainz lookup failed
    MusicBrainzError(String),
    /// No match found
    NoMatch,
}

impl std::fmt::Display for ClassificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassificationError::FingerprintError(s) => write!(f, "Fingerprint error: {}", s),
            ClassificationError::AcoustIdError(s) => write!(f, "AcoustID error: {}", s),
            ClassificationError::MusicBrainzError(s) => write!(f, "MusicBrainz error: {}", s),
            ClassificationError::NoMatch => write!(f, "No match found"),
        }
    }
}

impl std::error::Error for ClassificationError {}

/// Content type classifier service
///
/// **[PLAN026]** Orchestrates Step 6: Content Type Determination
///
/// Uses duration-based triage to route files to appropriate classification paths:
/// - Single-song path: Chromaprint → AcoustID → MusicBrainz recording search fallback
/// - Album path: Metadata → MusicBrainz Release → Edition matching (Stages 2-5)
pub struct ContentTypeClassifier {
    /// AcoustID client for single-song fingerprint matching
    acoustid_client: Arc<super::acoustid_client::AcoustIDClient>,
    /// Fingerprinter for generating Chromaprint fingerprints
    fingerprinter: super::fingerprinter::Fingerprinter,
    /// Album matcher for album path classification (Stages 2-5)
    album_matcher: crate::matching::AlbumMatcher,
    /// **[SSI-MB-010]** Recording matcher for MusicBrainz fallback
    recording_matcher: RecordingMatcher,
    /// Database pool for recording cache (optional)
    db_pool: Option<SqlitePool>,
}

impl ContentTypeClassifier {
    /// Create new classifier with AcoustID client
    pub fn new(acoustid_client: Arc<super::acoustid_client::AcoustIDClient>) -> Self {
        let mb_client = Arc::new(
            super::musicbrainz_client::MusicBrainzClient::new()
                .expect("Failed to create MusicBrainzClient"),
        );
        Self {
            acoustid_client,
            fingerprinter: super::fingerprinter::Fingerprinter::new(),
            album_matcher: crate::matching::AlbumMatcher::new()
                .expect("Failed to create AlbumMatcher"),
            recording_matcher: RecordingMatcher::new(mb_client),
            db_pool: None,
        }
    }

    /// Create classifier with database pool for caching
    ///
    /// **[SSI-INT-030]** Enables recording cache for MusicBrainz queries
    pub fn with_db(acoustid_client: Arc<super::acoustid_client::AcoustIDClient>, db_pool: SqlitePool) -> Self {
        let mb_client = Arc::new(
            super::musicbrainz_client::MusicBrainzClient::new()
                .expect("Failed to create MusicBrainzClient"),
        );
        Self {
            acoustid_client,
            fingerprinter: super::fingerprinter::Fingerprinter::new(),
            album_matcher: crate::matching::AlbumMatcher::new()
                .expect("Failed to create AlbumMatcher"),
            recording_matcher: RecordingMatcher::new(mb_client),
            db_pool: Some(db_pool),
        }
    }

    /// Set database pool for recording cache
    pub fn set_db_pool(&mut self, pool: SqlitePool) {
        self.db_pool = Some(pool);
    }

    /// Triage file based on duration
    ///
    /// **[REQ-ALG-001]** Duration-based routing:
    /// - <12 min → single-song path
    /// - 12-25 min → dual-path (try both)
    /// - >25 min → album path
    pub fn triage(&self, duration_seconds: f64) -> TriagePath {
        TriagePath::from_duration(duration_seconds)
    }

    /// Classify using single-song path (AcoustID)
    ///
    /// **[REQ-ALG-002, REQ-ALG-003]** Single-song classification:
    /// 1. Generate Chromaprint fingerprint
    /// 2. Query AcoustID API
    /// 3. Return SINGLE_SONG if confidence ≥ 0.80
    pub async fn classify_single_song(
        &self,
        audio_path: &std::path::Path,
        duration_seconds: f64,
    ) -> Result<ClassificationResult, ClassificationError> {
        // Step 1: Generate Chromaprint fingerprint
        let fingerprint = self
            .fingerprinter
            .fingerprint_file(audio_path)
            .map_err(|e| ClassificationError::FingerprintError(e.to_string()))?;

        // Step 2: Query AcoustID
        let response = self
            .acoustid_client
            .lookup(&fingerprint, duration_seconds as u64)
            .await
            .map_err(|e| ClassificationError::AcoustIdError(e.to_string()))?;

        // Step 3: Find best match with sufficient confidence
        let best_match = response
            .results
            .iter()
            .filter(|r| r.score >= HIGH_CONFIDENCE_THRESHOLD)
            .filter_map(|r| {
                r.recordings
                    .as_ref()
                    .and_then(|recs| recs.first().map(|rec| (r.score, rec.id.clone())))
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        match best_match {
            Some((score, recording_mbid)) => {
                Ok(ClassificationResult::single_song(recording_mbid, score))
            }
            None => {
                // No high-confidence match found
                if response.results.is_empty() {
                    Ok(ClassificationResult::not_in_musicbrainz())
                } else {
                    // Low confidence matches exist but don't meet threshold
                    Err(ClassificationError::NoMatch)
                }
            }
        }
    }

    /// Classify using single-song path with MusicBrainz fallback and ID3 validation
    ///
    /// **[SSI-MB-010, SSI-VAL-030]** Enhanced single-song classification:
    /// 1. Try AcoustID first
    /// 2. Validate AcoustID result against ID3 metadata (double-mismatch check)
    /// 3. If validation fails, go directly to MusicBrainz fallback
    /// 4. If high confidence AND validation passes, return result
    /// 5. If low confidence or failure, try MusicBrainz recording search
    /// 6. Fuse results if both available
    ///
    /// # Arguments
    /// * `audio_path` - Path to audio file
    /// * `duration_seconds` - Audio duration for AcoustID
    /// * `metadata` - Optional metadata for MusicBrainz fallback and validation
    pub async fn classify_single_song_with_fallback(
        &self,
        audio_path: &std::path::Path,
        duration_seconds: f64,
        metadata: Option<&AudioMetadata>,
    ) -> Result<ClassificationResult, ClassificationError> {
        // Step 1: Generate Chromaprint fingerprint and query AcoustID
        let fingerprint = self
            .fingerprinter
            .fingerprint_file(audio_path)
            .map_err(|e| ClassificationError::FingerprintError(e.to_string()))?;

        let response = self
            .acoustid_client
            .lookup(&fingerprint, duration_seconds as u64)
            .await
            .map_err(|e| ClassificationError::AcoustIdError(e.to_string()))?;

        // Step 2: Validate AcoustID result against ID3 metadata (double-mismatch check)
        let validation = if let Some(meta) = metadata {
            validate_acoustid_against_id3(
                &response,
                meta.artist.as_deref(),
                meta.title.as_deref(),
            )
        } else {
            // No metadata to validate against - assume pass
            ValidationResult {
                passed: true,
                artist_similarity: 1.0,
                title_similarity: 1.0,
                recording: None,
            }
        };

        // Step 3: If double-mismatch detected, skip AcoustID entirely
        if validation.is_double_mismatch() {
            tracing::info!(
                artist_sim = validation.artist_similarity,
                title_sim = validation.title_similarity,
                "Double-mismatch detected - skipping AcoustID, using MusicBrainz fallback"
            );
            // Go directly to MusicBrainz fallback
            return self.try_mb_fallback_only(metadata, duration_seconds).await;
        }

        // Step 4: Find best match from validated response
        let best_match = response
            .results
            .iter()
            .filter(|r| r.score >= HIGH_CONFIDENCE_THRESHOLD)
            .filter_map(|r| {
                r.recordings
                    .as_ref()
                    .and_then(|recs| recs.first().map(|rec| (r.score, rec.id.clone())))
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Step 5: If AcoustID succeeds with high confidence, use it
        if let Some((score, recording_mbid)) = best_match {
            tracing::debug!(
                confidence = score,
                mbid = %recording_mbid,
                artist_sim = validation.artist_similarity,
                title_sim = validation.title_similarity,
                "AcoustID match validated against ID3 metadata"
            );
            return Ok(ClassificationResult::single_song(recording_mbid, score));
        }

        // Step 6: No high-confidence match - try MusicBrainz fallback
        self.try_mb_fallback_only(metadata, duration_seconds).await
    }

    /// Try MusicBrainz fallback only (after AcoustID failure or rejection)
    async fn try_mb_fallback_only(
        &self,
        metadata: Option<&AudioMetadata>,
        duration_seconds: f64,
    ) -> Result<ClassificationResult, ClassificationError> {
        if let Some(meta) = metadata {
            if let (Some(artist), Some(title)) = (&meta.artist, &meta.title) {
                let mb_result = self
                    .try_musicbrainz_fallback(artist, title, Some(duration_seconds))
                    .await;

                if let Ok(candidates) = mb_result {
                    if let Some(best) = candidates.first() {
                        tracing::info!(
                            artist = %artist,
                            title = %title,
                            mbid = %best.mbid,
                            similarity = best.similarity,
                            "MusicBrainz fallback succeeded"
                        );
                        return Ok(ClassificationResult::single_song_with_stage(
                            best.mbid.clone(),
                            best.similarity,
                            "musicbrainz_fallback",
                        ));
                    }
                }
            }
        }

        Ok(ClassificationResult::not_in_musicbrainz())
    }

    /// Legacy classify_single_song_with_fallback without double-mismatch check
    ///
    /// **Deprecated:** Use classify_single_song_with_fallback instead
    #[allow(dead_code)]
    async fn classify_single_song_with_fallback_legacy(
        &self,
        audio_path: &std::path::Path,
        duration_seconds: f64,
        metadata: Option<&AudioMetadata>,
    ) -> Result<ClassificationResult, ClassificationError> {
        // Step 1: Try AcoustID
        let acoustid_result = self.classify_single_song(audio_path, duration_seconds).await;

        // Step 2: If AcoustID succeeds with high confidence, use it
        if let Ok(ref result) = acoustid_result {
            if result.confidence_value >= HIGH_CONFIDENCE_THRESHOLD {
                tracing::debug!(
                    confidence = result.confidence_value,
                    mbid = ?result.recording_mbid,
                    "AcoustID high confidence match, using directly"
                );
                return Ok(result.clone());
            }
        }

        // Step 3: Try MusicBrainz recording search as fallback
        if let Some(meta) = metadata {
            if let (Some(artist), Some(title)) = (&meta.artist, &meta.title) {
                let mb_result = self
                    .try_musicbrainz_fallback(artist, title, Some(duration_seconds))
                    .await;

                if let Ok(candidates) = mb_result {
                    if let Some(best) = candidates.first() {
                        // Have both AcoustID (low) and MB result - fuse them
                        if let Ok(acoustid) = &acoustid_result {
                            if acoustid.recording_mbid.is_some() {
                                return Ok(self.fuse_results(acoustid, best));
                            }
                        }
                        // Only MB result available
                        tracing::info!(
                            artist = %artist,
                            title = %title,
                            mbid = %best.mbid,
                            similarity = best.similarity,
                            "MusicBrainz fallback succeeded"
                        );
                        return Ok(ClassificationResult::single_song_with_stage(
                            best.mbid.clone(),
                            best.similarity,
                            "musicbrainz_fallback",
                        ));
                    }
                }
            }
        }

        // Step 4: Return AcoustID result if available (even low confidence)
        if let Ok(result) = acoustid_result {
            return Ok(result);
        }

        // Step 5: No match found
        Ok(ClassificationResult::not_in_musicbrainz())
    }

    /// Try MusicBrainz recording search
    ///
    /// Uses cache if database pool is available.
    async fn try_musicbrainz_fallback(
        &self,
        artist: &str,
        title: &str,
        duration_secs: Option<f64>,
    ) -> Result<Vec<RecordingCandidate>, ClassificationError> {
        tracing::debug!(
            artist = %artist,
            title = %title,
            duration = ?duration_secs,
            "Trying MusicBrainz recording search fallback"
        );

        let candidates = if let Some(pool) = &self.db_pool {
            self.recording_matcher
                .search_with_cache(artist, title, duration_secs, pool)
                .await
        } else {
            self.recording_matcher
                .search(artist, title, duration_secs)
                .await
        };

        candidates.map_err(|e| ClassificationError::MusicBrainzError(e.to_string()))
    }

    /// Fuse AcoustID and MusicBrainz results
    ///
    /// **[SSI-FUS-010]** Bayesian-style fusion:
    /// - If both agree on MBID: boost confidence using 1 - (1-c1)(1-c2)
    /// - If disagree: prefer higher confidence
    fn fuse_results(
        &self,
        acoustid: &ClassificationResult,
        mb: &RecordingCandidate,
    ) -> ClassificationResult {
        // If they agree on MBID, boost confidence
        if acoustid.recording_mbid.as_ref() == Some(&mb.mbid) {
            let combined = 1.0 - (1.0 - acoustid.confidence_value) * (1.0 - mb.similarity);
            tracing::info!(
                mbid = %mb.mbid,
                acoustid_conf = acoustid.confidence_value,
                mb_conf = mb.similarity,
                combined = combined,
                "Sources agree - boosting confidence via Bayesian fusion"
            );
            return ClassificationResult::single_song_with_stage(
                mb.mbid.clone(),
                combined,
                "multi_source_fusion",
            );
        }

        // If they disagree, prefer higher confidence
        if mb.similarity > acoustid.confidence_value {
            tracing::info!(
                acoustid_mbid = ?acoustid.recording_mbid,
                mb_mbid = %mb.mbid,
                "Sources disagree - preferring MusicBrainz (higher confidence)"
            );
            ClassificationResult::single_song_with_stage(
                mb.mbid.clone(),
                mb.similarity,
                "musicbrainz_preferred",
            )
        } else {
            tracing::info!(
                acoustid_mbid = ?acoustid.recording_mbid,
                mb_mbid = %mb.mbid,
                "Sources disagree - preferring AcoustID (higher confidence)"
            );
            acoustid.clone()
        }
    }

    /// Full multi-source fusion using IdentityResolver
    ///
    /// **[SSI-FUS-020]** Combines all available sources via IdentityResolver:
    /// - Source 1: AcoustID fingerprint matching
    /// - Source 2: MusicBrainz recording search
    /// - Source 3: ID3 metadata implicit confidence (when sources agree)
    ///
    /// Uses Bayesian posterior: `P = 1 - (1-c1)(1-c2)...(1-cN)`
    pub async fn classify_with_full_fusion(
        &self,
        audio_path: &std::path::Path,
        duration_seconds: f64,
        metadata: Option<&AudioMetadata>,
    ) -> Result<ClassificationResult, ClassificationError> {
        let mut sources: Vec<IdentityExtraction> = Vec::new();

        // Source 1: AcoustID
        if let Ok(acoustid_result) = self.classify_single_song(audio_path, duration_seconds).await {
            if let Some(mbid) = &acoustid_result.recording_mbid {
                sources.push(IdentityExtraction {
                    recording_mbid: mbid.clone(),
                    confidence: acoustid_result.confidence_value as f32,
                    source: "AcoustID".to_string(),
                });
            }
        }

        // Source 2: MusicBrainz recording search
        if let Some(meta) = metadata {
            if let (Some(artist), Some(title)) = (&meta.artist, &meta.title) {
                if let Ok(candidates) = self
                    .try_musicbrainz_fallback(artist, title, Some(duration_seconds))
                    .await
                {
                    if let Some(best) = candidates.first() {
                        sources.push(IdentityExtraction {
                            recording_mbid: best.mbid.clone(),
                            confidence: best.similarity as f32,
                            source: "MusicBrainz".to_string(),
                        });
                    }
                }
            }
        }

        // Source 3: ID3 metadata as implicit confidence when sources agree
        if sources.len() >= 2 {
            let first_mbid = &sources[0].recording_mbid;
            let all_agree = sources.iter().all(|s| &s.recording_mbid == first_mbid);

            if all_agree {
                // ID3 metadata corroborates the match
                sources.push(IdentityExtraction {
                    recording_mbid: first_mbid.clone(),
                    confidence: 0.70, // ID3 metadata implicit confidence
                    source: "ID3".to_string(),
                });
            }
        }

        // Fuse all sources using IdentityResolver
        if sources.is_empty() {
            return Ok(ClassificationResult::not_in_musicbrainz());
        }

        let resolver = IdentityResolver::new();
        match resolver.fuse(sources).await {
            Ok(fusion_result) => {
                if let Some(mbid) = fusion_result.output.recording_mbid {
                    let confidence = fusion_result.confidence as f64;
                    tracing::info!(
                        mbid = %mbid,
                        confidence,
                        sources = ?fusion_result.sources,
                        "Full multi-source fusion completed"
                    );
                    Ok(ClassificationResult::single_song_with_stage(
                        mbid,
                        confidence,
                        "identity_resolver_fusion",
                    ))
                } else {
                    Ok(ClassificationResult::not_in_musicbrainz())
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "IdentityResolver fusion failed");
                Ok(ClassificationResult::identification_failed())
            }
        }
    }

    /// Classify using album path (am28 Stages 2-5)
    ///
    /// **[REQ-ALG-004..014]** Album classification:
    /// 1. Extract metadata and search MusicBrainz
    /// 2. Test editions through Stages 2-5
    /// 3. Return FULL_ALBUM or PARTIAL_ALBUM based on match quality
    ///
    /// # Arguments
    /// * `audio_path` - Path to the audio file
    /// * `artist_hint` - Artist name from metadata (optional)
    /// * `album_hint` - Album name from metadata (optional)
    pub async fn classify_album(
        &self,
        audio_path: &std::path::Path,
        artist_hint: Option<&str>,
        album_hint: Option<&str>,
    ) -> ClassificationResult {
        match self
            .album_matcher
            .match_album(audio_path, artist_hint, album_hint)
            .await
        {
            Ok(result) => {
                if result.is_full_album() {
                    ClassificationResult {
                        content_type: ContentType::FullAlbum,
                        confidence: MatchConfidence::from_value(result.match_percentage / 100.0),
                        confidence_value: result.match_percentage / 100.0,
                        release_mbid: result.release_mbid,
                        recording_mbid: None,
                        match_percentage: Some(result.match_percentage),
                        artist_verified: result.artist_verified,
                        matching_stage: result.matching_stage.map(|s| s.as_str().to_string()),
                    }
                } else if result.is_partial_album() {
                    ClassificationResult {
                        content_type: ContentType::PartialAlbum,
                        confidence: MatchConfidence::from_value(result.match_percentage / 100.0),
                        confidence_value: result.match_percentage / 100.0,
                        release_mbid: result.release_mbid,
                        recording_mbid: None,
                        match_percentage: Some(result.match_percentage),
                        artist_verified: result.artist_verified,
                        matching_stage: result.matching_stage.map(|s| s.as_str().to_string()),
                    }
                } else {
                    // No album match found
                    ClassificationResult::not_in_musicbrainz()
                }
            }
            Err(_) => ClassificationResult::identification_failed(),
        }
    }

    /// Full classification workflow based on triage
    ///
    /// **[REQ-STEP-006]** Step 6: Content Type Determination
    pub async fn classify(
        &self,
        audio_path: &std::path::Path,
        duration_seconds: f64,
    ) -> ClassificationResult {
        let triage = self.triage(duration_seconds);

        match triage {
            TriagePath::SingleSong => {
                // Try single-song path only
                match self
                    .classify_single_song(audio_path, duration_seconds)
                    .await
                {
                    Ok(result) => result,
                    Err(_) => ClassificationResult::not_in_musicbrainz(),
                }
            }
            TriagePath::DualPath => {
                // Try single-song first, fall through to album if no match
                match self
                    .classify_single_song(audio_path, duration_seconds)
                    .await
                {
                    Ok(result) => result,
                    Err(_) => {
                        // Single-song failed, try album path
                        self.classify_album(audio_path, None, None).await
                    }
                }
            }
            TriagePath::Album => {
                // Album path only (file too long for single-song)
                self.classify_album(audio_path, None, None).await
            }
        }
    }

    /// Full classification with metadata hints
    ///
    /// **[REQ-STEP-006]** Step 6: Content Type Determination with metadata
    ///
    /// Enhanced version of classify() that accepts artist/album hints
    /// for improved MusicBrainz search accuracy.
    pub async fn classify_with_metadata(
        &self,
        audio_path: &std::path::Path,
        duration_seconds: f64,
        artist_hint: Option<&str>,
        album_hint: Option<&str>,
    ) -> ClassificationResult {
        let triage = self.triage(duration_seconds);

        match triage {
            TriagePath::SingleSong => {
                match self
                    .classify_single_song(audio_path, duration_seconds)
                    .await
                {
                    Ok(result) => result,
                    Err(_) => ClassificationResult::not_in_musicbrainz(),
                }
            }
            TriagePath::DualPath => {
                match self
                    .classify_single_song(audio_path, duration_seconds)
                    .await
                {
                    Ok(result) => result,
                    Err(_) => {
                        self.classify_album(audio_path, artist_hint, album_hint)
                            .await
                    }
                }
            }
            TriagePath::Album => {
                self.classify_album(audio_path, artist_hint, album_hint)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_str_roundtrip() {
        let types = [
            ContentType::SingleSong,
            ContentType::FullAlbum,
            ContentType::PartialAlbum,
            ContentType::MultipleSongs,
            ContentType::NotInMusicbrainz,
            ContentType::IdentificationFailed,
        ];

        for ct in types {
            let s = ct.as_str();
            let parsed = ContentType::from_str(s);
            assert_eq!(parsed, Some(ct), "Roundtrip failed for {:?}", ct);
        }
    }

    #[test]
    fn test_triage_path_from_duration() {
        // < 12 minutes = single song
        assert_eq!(TriagePath::from_duration(300.0), TriagePath::SingleSong); // 5 min
        assert_eq!(TriagePath::from_duration(600.0), TriagePath::SingleSong); // 10 min

        // 12-25 minutes = dual path
        assert_eq!(TriagePath::from_duration(720.0), TriagePath::DualPath); // 12 min
        assert_eq!(TriagePath::from_duration(1200.0), TriagePath::DualPath); // 20 min

        // > 25 minutes = album
        assert_eq!(TriagePath::from_duration(1501.0), TriagePath::Album); // 25+ min
        assert_eq!(TriagePath::from_duration(3600.0), TriagePath::Album); // 60 min
    }

    #[test]
    fn test_match_confidence_from_value() {
        assert_eq!(
            MatchConfidence::from_value(0.98),
            MatchConfidence::Excellent
        );
        assert_eq!(
            MatchConfidence::from_value(0.95),
            MatchConfidence::Excellent
        );
        assert_eq!(MatchConfidence::from_value(0.80), MatchConfidence::Good);
        assert_eq!(MatchConfidence::from_value(0.60), MatchConfidence::Fair);
        assert_eq!(MatchConfidence::from_value(0.40), MatchConfidence::Poor);
    }

    #[test]
    fn test_match_confidence_high_threshold() {
        assert!(MatchConfidence::Excellent.is_high_confidence());
        assert!(MatchConfidence::Good.is_high_confidence());
        assert!(!MatchConfidence::Fair.is_high_confidence());
        assert!(!MatchConfidence::Poor.is_high_confidence());
    }

    #[test]
    fn test_content_type_is_completed() {
        assert!(ContentType::NotInMusicbrainz.is_completed());
        assert!(ContentType::IdentificationFailed.is_completed());
        assert!(!ContentType::SingleSong.is_completed());
        assert!(!ContentType::FullAlbum.is_completed());
    }

    #[test]
    fn test_content_type_requires_segmentation() {
        assert!(ContentType::FullAlbum.requires_segmentation());
        assert!(ContentType::PartialAlbum.requires_segmentation());
        assert!(ContentType::MultipleSongs.requires_segmentation());
        assert!(!ContentType::SingleSong.requires_segmentation());
        assert!(!ContentType::NotInMusicbrainz.requires_segmentation());
    }

    // Tests for double-mismatch validation

    fn make_test_response(artist: &str, title: &str, score: f64) -> AcoustIDResponse {
        use crate::services::acoustid_client::{AcoustIDArtist, AcoustIDRecording, AcoustIDResult};

        AcoustIDResponse {
            status: "ok".to_string(),
            results: vec![AcoustIDResult {
                id: "test-id".to_string(),
                score,
                recordings: Some(vec![AcoustIDRecording {
                    id: "mbid-123".to_string(),
                    title: Some(title.to_string()),
                    artists: Some(vec![AcoustIDArtist {
                        id: "artist-mbid".to_string(),
                        name: artist.to_string(),
                    }]),
                    duration: Some(180),
                }]),
            }],
        }
    }

    #[test]
    fn test_validation_passes_when_metadata_matches() {
        let response = make_test_response("The Beatles", "Yesterday", 0.95);
        let result = validate_acoustid_against_id3(&response, Some("The Beatles"), Some("Yesterday"));

        assert!(result.passed);
        assert!(!result.is_double_mismatch());
        assert!(result.artist_similarity > 0.9);
        assert!(result.title_similarity > 0.9);
    }

    #[test]
    fn test_validation_passes_with_normalized_match() {
        // "The Beatles" should match "Beatles" after normalization
        let response = make_test_response("Beatles", "Yesterday", 0.95);
        let result = validate_acoustid_against_id3(&response, Some("The Beatles"), Some("Yesterday"));

        assert!(result.passed);
        assert!(!result.is_double_mismatch());
        assert!(result.artist_similarity > 0.9);
    }

    #[test]
    fn test_validation_fails_on_double_mismatch() {
        // Boston vs Metro Station - completely different
        let response = make_test_response("Metro Station", "Shake It", 0.99);
        let result = validate_acoustid_against_id3(&response, Some("Boston"), Some("Rock & Roll Band"));

        assert!(!result.passed);
        assert!(result.is_double_mismatch());
        assert!(result.artist_similarity < 0.7);
        assert!(result.title_similarity < 0.7);
    }

    #[test]
    fn test_validation_passes_on_single_mismatch() {
        // Same artist, different title - NOT a double mismatch
        let response = make_test_response("The Beatles", "Hey Jude", 0.95);
        let result = validate_acoustid_against_id3(
            &response,
            Some("The Beatles"),
            Some("Yesterday"), // Different title
        );

        // Artist matches, so not a double mismatch
        assert!(result.passed);
        assert!(!result.is_double_mismatch());
        assert!(result.artist_similarity > 0.9);
        assert!(result.title_similarity < 0.7);
    }

    #[test]
    fn test_validation_with_no_id3_metadata() {
        let response = make_test_response("Any Artist", "Any Title", 0.95);
        let result = validate_acoustid_against_id3(&response, None, None);

        // No ID3 metadata means we can't validate - assume pass
        assert!(result.passed);
        assert!(!result.is_double_mismatch());
    }

    #[test]
    fn test_validation_result_is_double_mismatch() {
        let result = ValidationResult {
            passed: false,
            artist_similarity: 0.5,
            title_similarity: 0.5,
            recording: None,
        };
        assert!(result.is_double_mismatch());

        let result = ValidationResult {
            passed: true,
            artist_similarity: 0.9,
            title_similarity: 0.5,
            recording: None,
        };
        assert!(!result.is_double_mismatch()); // Artist matches
    }
}

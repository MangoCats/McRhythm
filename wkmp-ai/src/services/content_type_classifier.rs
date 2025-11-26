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

use serde::{Deserialize, Serialize};
use std::fmt;

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
        const ALBUM_THRESHOLD_SECS: f64 = 25.0 * 60.0;  // 25 minutes

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

/// Classification error types
#[derive(Debug)]
pub enum ClassificationError {
    /// Fingerprinting failed
    FingerprintError(String),
    /// AcoustID lookup failed
    AcoustIdError(String),
    /// No match found
    NoMatch,
}

impl std::fmt::Display for ClassificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassificationError::FingerprintError(s) => write!(f, "Fingerprint error: {}", s),
            ClassificationError::AcoustIdError(s) => write!(f, "AcoustID error: {}", s),
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
/// - Single-song path: Chromaprint → AcoustID → SINGLE_SONG
/// - Album path: Metadata → MusicBrainz Release → Edition matching (Stages 2-5)
pub struct ContentTypeClassifier {
    /// AcoustID client for single-song fingerprint matching
    acoustid_client: std::sync::Arc<super::acoustid_client::AcoustIDClient>,
    /// Fingerprinter for generating Chromaprint fingerprints
    fingerprinter: super::fingerprinter::Fingerprinter,
    /// Album matcher for album path classification (Stages 2-5)
    album_matcher: crate::matching::AlbumMatcher,
}

impl ContentTypeClassifier {
    /// Create new classifier with AcoustID client
    pub fn new(acoustid_client: std::sync::Arc<super::acoustid_client::AcoustIDClient>) -> Self {
        Self {
            acoustid_client,
            fingerprinter: super::fingerprinter::Fingerprinter::new(),
            album_matcher: crate::matching::AlbumMatcher::new(),
        }
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
                r.recordings.as_ref().and_then(|recs| {
                    recs.first().map(|rec| (r.score, rec.id.clone()))
                })
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
                        matching_stage: result
                            .matching_stage
                            .map(|s| s.as_str().to_string()),
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
                        matching_stage: result
                            .matching_stage
                            .map(|s| s.as_str().to_string()),
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
                match self.classify_single_song(audio_path, duration_seconds).await {
                    Ok(result) => result,
                    Err(_) => ClassificationResult::not_in_musicbrainz(),
                }
            }
            TriagePath::DualPath => {
                // Try single-song first, fall through to album if no match
                match self.classify_single_song(audio_path, duration_seconds).await {
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
                match self.classify_single_song(audio_path, duration_seconds).await {
                    Ok(result) => result,
                    Err(_) => ClassificationResult::not_in_musicbrainz(),
                }
            }
            TriagePath::DualPath => {
                match self.classify_single_song(audio_path, duration_seconds).await {
                    Ok(result) => result,
                    Err(_) => {
                        self.classify_album(audio_path, artist_hint, album_hint).await
                    }
                }
            }
            TriagePath::Album => {
                self.classify_album(audio_path, artist_hint, album_hint).await
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
        assert_eq!(TriagePath::from_duration(720.0), TriagePath::DualPath);  // 12 min
        assert_eq!(TriagePath::from_duration(1200.0), TriagePath::DualPath); // 20 min

        // > 25 minutes = album
        assert_eq!(TriagePath::from_duration(1501.0), TriagePath::Album);    // 25+ min
        assert_eq!(TriagePath::from_duration(3600.0), TriagePath::Album);    // 60 min
    }

    #[test]
    fn test_match_confidence_from_value() {
        assert_eq!(MatchConfidence::from_value(0.98), MatchConfidence::Excellent);
        assert_eq!(MatchConfidence::from_value(0.95), MatchConfidence::Excellent);
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
}

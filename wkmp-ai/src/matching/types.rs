//! Album Matching Types
//!
//! **[PLAN026]** Types for album matching results
//!
//! These types bridge the am28 matching algorithm output to the
//! ContentTypeClassifier's ClassificationResult format.

use serde::{Deserialize, Serialize};

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
}

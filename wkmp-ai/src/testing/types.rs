//! Core types for the closed-loop testing system
//!
//! **[PLAN031 Increment 1]** Testing module foundation types
//!
//! Defines enums and structs used throughout the testing infrastructure:
//! - Classification results (TP/FP/TN/FN)
//! - Failure categories for pattern analysis
//! - Batch configuration
//! - Reset modes for database management

use serde::{Deserialize, Serialize};

/// Classification of a test result against ground truth
///
/// **[SPEC031-EV-020]** Success/failure classification for single songs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Classification {
    /// Assigned MBID matches ground truth
    TruePositive,
    /// Assigned MBID differs from ground truth
    FalsePositive,
    /// No MBID assigned, ground truth is NULL (correctly rejected)
    TrueNegative,
    /// No MBID assigned, but ground truth exists (missed identification)
    FalseNegative,
}

impl Classification {
    /// Returns true if this is a successful classification
    pub fn is_success(&self) -> bool {
        matches!(self, Classification::TruePositive | Classification::TrueNegative)
    }

    /// Returns true if this is a failure classification
    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }
}

impl std::fmt::Display for Classification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Classification::TruePositive => write!(f, "TRUE_POSITIVE"),
            Classification::FalsePositive => write!(f, "FALSE_POSITIVE"),
            Classification::TrueNegative => write!(f, "TRUE_NEGATIVE"),
            Classification::FalseNegative => write!(f, "FALSE_NEGATIVE"),
        }
    }
}

/// Album-level classification result
///
/// **[SPEC031-EV-020]** Success/failure classification for albums
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlbumClassification {
    /// Album MBID + all track MBIDs correct
    AlbumMatch,
    /// Album MBID correct, some tracks wrong
    PartialMatch,
    /// Album MBID incorrect
    WrongAlbum,
    /// No album identification (failure if ground truth exists)
    NoMatch,
}

impl AlbumClassification {
    /// Returns true if this is a full success
    pub fn is_success(&self) -> bool {
        matches!(self, AlbumClassification::AlbumMatch)
    }
}

impl std::fmt::Display for AlbumClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlbumClassification::AlbumMatch => write!(f, "ALBUM_MATCH"),
            AlbumClassification::PartialMatch => write!(f, "PARTIAL_MATCH"),
            AlbumClassification::WrongAlbum => write!(f, "WRONG_ALBUM"),
            AlbumClassification::NoMatch => write!(f, "NO_MATCH"),
        }
    }
}

/// Category of failure for pattern analysis
///
/// **[SPEC031-FA-010]** Failure categorization for root cause analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureCategory {
    // Identification failures
    /// Got an MBID but it's wrong
    WrongRecording,
    /// Correct song, wrong album release
    WrongAlbum,
    /// Similar song name, different artist
    WrongArtist,
    /// Should have found MBID but didn't
    MissedIdentification,

    // Confidence failures
    /// High confidence but wrong
    OverconfidentWrong,
    /// Low confidence but actually correct
    UnderconfidentRight,

    // Data source failures
    /// AcoustID returned wrong result
    AcoustIdMismatch,
    /// MusicBrainz search returned no/wrong results
    MetadataSearchFailed,
    /// ID3 metadata was misleading
    ID3TagsUnreliable,

    // Edge cases
    /// Multiple correct MBIDs exist (remasters, etc.)
    MultipleValidMatches,
    /// Can't verify correctness
    NoGroundTruth,
    /// Processing took too long
    Timeout,
    /// External service failure
    ApiError,
}

impl std::fmt::Display for FailureCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureCategory::WrongRecording => write!(f, "WRONG_RECORDING"),
            FailureCategory::WrongAlbum => write!(f, "WRONG_ALBUM"),
            FailureCategory::WrongArtist => write!(f, "WRONG_ARTIST"),
            FailureCategory::MissedIdentification => write!(f, "MISSED_IDENTIFICATION"),
            FailureCategory::OverconfidentWrong => write!(f, "OVERCONFIDENT_WRONG"),
            FailureCategory::UnderconfidentRight => write!(f, "UNDERCONFIDENT_RIGHT"),
            FailureCategory::AcoustIdMismatch => write!(f, "ACOUSTID_MISMATCH"),
            FailureCategory::MetadataSearchFailed => write!(f, "METADATA_SEARCH_FAILED"),
            FailureCategory::ID3TagsUnreliable => write!(f, "ID3_TAGS_UNRELIABLE"),
            FailureCategory::MultipleValidMatches => write!(f, "MULTIPLE_VALID_MATCHES"),
            FailureCategory::NoGroundTruth => write!(f, "NO_GROUND_TRUTH"),
            FailureCategory::Timeout => write!(f, "TIMEOUT"),
            FailureCategory::ApiError => write!(f, "API_ERROR"),
        }
    }
}

/// Database reset mode for testing
///
/// **[SPEC031-DB-010]** Reset modes for test isolation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResetMode {
    /// Clear entire database, fresh start (preserves ground_truth)
    Full,
    /// Clear only test-related data, keep ground truth and settings
    TestDataOnly,
    /// Clear specific batch results only
    BatchOnly {
        /// Batch ID to clear
        batch_id: String,
    },
    /// No reset, build on existing data
    Incremental,
}

impl Default for ResetMode {
    fn default() -> Self {
        ResetMode::Incremental
    }
}

/// Test batch configuration
///
/// **[SPEC031-BM-010]** Batch configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Number of files per batch
    pub batch_size: usize,

    /// Stop after N consecutive failures (Phase 1)
    ///
    /// Per ISSUE-H-001 resolution: "consecutive" means failures in a row,
    /// counter resets on success.
    pub failure_threshold: usize,

    /// Minimum accuracy to proceed (Phase 2+)
    pub accuracy_threshold: f64,

    /// Enable parallel processing (Phase 3)
    pub parallel_workers: usize,

    /// Maximum time per file before timeout (seconds)
    pub timeout_secs: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            failure_threshold: 3,
            accuracy_threshold: 0.90,
            parallel_workers: 1,
            timeout_secs: 60,
        }
    }
}

impl BatchConfig {
    /// Create Phase 1 configuration (small batches, accuracy focus)
    pub fn phase1() -> Self {
        Self {
            batch_size: 10,
            failure_threshold: 3,
            accuracy_threshold: 0.85,
            parallel_workers: 1,
            timeout_secs: 60,
        }
    }

    /// Create Phase 2 configuration (medium batches, validation)
    pub fn phase2() -> Self {
        Self {
            batch_size: 50,
            failure_threshold: usize::MAX, // Don't stop on failures
            accuracy_threshold: 0.90,
            parallel_workers: 1,
            timeout_secs: 60,
        }
    }

    /// Create Phase 3 configuration (large batches, speed optimization)
    pub fn phase3(workers: usize) -> Self {
        Self {
            batch_size: 500,
            failure_threshold: usize::MAX,
            accuracy_threshold: 0.90,
            parallel_workers: workers,
            timeout_secs: 120,
        }
    }
}

/// Test phase in the progression strategy
///
/// **[SPEC031-BM-020]** Three-phase progression
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestPhase {
    /// Small batches (5-10 files), accuracy focus
    Phase1,
    /// Medium batches (50-100 files), validation
    Phase2,
    /// Large batches (500+ files), speed optimization
    Phase3,
}

impl std::fmt::Display for TestPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestPhase::Phase1 => write!(f, "Phase 1 (Small Batches)"),
            TestPhase::Phase2 => write!(f, "Phase 2 (Medium Batches)"),
            TestPhase::Phase3 => write!(f, "Phase 3 (Large Batches)"),
        }
    }
}

impl Default for TestPhase {
    fn default() -> Self {
        TestPhase::Phase1
    }
}

/// Reason for stopping a batch
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    /// All files processed successfully
    Completed,
    /// Failure threshold reached (Phase 1)
    FailureThreshold,
    /// Accuracy below threshold
    AccuracyBelowThreshold,
    /// User cancelled
    Cancelled,
    /// Error occurred
    Error(String),
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopReason::Completed => write!(f, "Completed"),
            StopReason::FailureThreshold => write!(f, "Failure threshold reached"),
            StopReason::AccuracyBelowThreshold => write!(f, "Accuracy below threshold"),
            StopReason::Cancelled => write!(f, "Cancelled"),
            StopReason::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification_is_success() {
        assert!(Classification::TruePositive.is_success());
        assert!(Classification::TrueNegative.is_success());
        assert!(!Classification::FalsePositive.is_success());
        assert!(!Classification::FalseNegative.is_success());
    }

    #[test]
    fn test_classification_is_failure() {
        assert!(!Classification::TruePositive.is_failure());
        assert!(!Classification::TrueNegative.is_failure());
        assert!(Classification::FalsePositive.is_failure());
        assert!(Classification::FalseNegative.is_failure());
    }

    #[test]
    fn test_batch_config_defaults() {
        let config = BatchConfig::default();
        assert_eq!(config.batch_size, 10);
        assert_eq!(config.failure_threshold, 3);
        assert!((config.accuracy_threshold - 0.90).abs() < 0.001);
        assert_eq!(config.parallel_workers, 1);
        assert_eq!(config.timeout_secs, 60);
    }

    #[test]
    fn test_batch_config_phases() {
        let p1 = BatchConfig::phase1();
        assert_eq!(p1.batch_size, 10);
        assert_eq!(p1.failure_threshold, 3);

        let p2 = BatchConfig::phase2();
        assert_eq!(p2.batch_size, 50);
        assert_eq!(p2.failure_threshold, usize::MAX);

        let p3 = BatchConfig::phase3(4);
        assert_eq!(p3.batch_size, 500);
        assert_eq!(p3.parallel_workers, 4);
    }

    #[test]
    fn test_reset_mode_default() {
        let mode = ResetMode::default();
        assert_eq!(mode, ResetMode::Incremental);
    }
}

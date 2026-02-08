//! Fallback Handler Service
//!
//! **[PLAN026]** Increment 7: Fallback & Edge Cases
//!
//! Handles files that fail automated identification:
//! - MusicBrainz lookup failures (no candidates, API errors)
//! - Artist verification failures
//! - All matching stages exhausted without success
//! - Edge cases (corrupted metadata, very short files)

use anyhow::Result;
use sqlx::SqlitePool;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::content_type_classifier::{ClassificationResult, ContentType, MatchConfidence};

/// Fallback strategies for failed identification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Accept file with metadata-only classification (no MusicBrainz match)
    MetadataOnly,
    /// Mark for manual review
    ManualReview,
    /// Retry with relaxed matching parameters
    RelaxedRetry,
    /// Skip file entirely (unprocessable)
    Skip,
}

impl FallbackStrategy {
    /// Convert to database string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            FallbackStrategy::MetadataOnly => "metadata_only",
            FallbackStrategy::ManualReview => "manual_review",
            FallbackStrategy::RelaxedRetry => "relaxed_retry",
            FallbackStrategy::Skip => "skip",
        }
    }
}

/// Reason why fallback was triggered
#[derive(Debug, Clone)]
pub enum FallbackReason {
    /// No MusicBrainz candidates found for search query
    NoCandidates {
        /// Search query that yielded no results
        query: String,
    },
    /// MusicBrainz API returned an error
    ApiError {
        /// Error message from the API
        error: String,
    },
    /// Artist verification failed (similarity below threshold)
    ArtistVerificationFailed {
        /// Artist name from file metadata
        expected: String,
        /// Artist name from MusicBrainz
        found: String,
        /// Computed similarity score (0.0-1.0)
        similarity: f64,
    },
    /// All matching stages exhausted without valid match
    AllStagesExhausted {
        /// Number of matching stages attempted
        stages_tried: usize,
    },
    /// File too short for meaningful analysis
    FileTooShort {
        /// File duration in seconds
        duration_secs: f64,
    },
    /// Metadata extraction failed
    MetadataExtractionFailed {
        /// Error description
        error: String,
    },
    /// Single track detected in expected album
    SingleTrackDetected,
    /// Audio decoding failed
    DecodingFailed {
        /// Decoding error description
        error: String,
    },
}

impl FallbackReason {
    /// Convert to user-friendly description
    pub fn description(&self) -> String {
        match self {
            FallbackReason::NoCandidates { query } => {
                format!("No MusicBrainz releases found for: {}", query)
            }
            FallbackReason::ApiError { error } => {
                format!("MusicBrainz API error: {}", error)
            }
            FallbackReason::ArtistVerificationFailed {
                expected,
                found,
                similarity,
            } => {
                format!(
                    "Artist mismatch: expected '{}', found '{}' (similarity: {:.1}%)",
                    expected,
                    found,
                    similarity * 100.0
                )
            }
            FallbackReason::AllStagesExhausted { stages_tried } => {
                format!(
                    "All {} matching stages failed to find valid match",
                    stages_tried
                )
            }
            FallbackReason::FileTooShort { duration_secs } => {
                format!("File too short for analysis ({:.1}s)", duration_secs)
            }
            FallbackReason::MetadataExtractionFailed { error } => {
                format!("Failed to extract metadata: {}", error)
            }
            FallbackReason::SingleTrackDetected => {
                "Single track detected when album expected".to_string()
            }
            FallbackReason::DecodingFailed { error } => {
                format!("Audio decoding failed: {}", error)
            }
        }
    }

    /// Get recommended fallback strategy
    pub fn recommended_strategy(&self) -> FallbackStrategy {
        match self {
            // Can still use local metadata
            FallbackReason::NoCandidates { .. }
            | FallbackReason::ApiError { .. }
            | FallbackReason::AllStagesExhausted { .. } => FallbackStrategy::MetadataOnly,

            // Needs human decision
            FallbackReason::ArtistVerificationFailed { .. }
            | FallbackReason::SingleTrackDetected => FallbackStrategy::ManualReview,

            // Might work with different parameters
            FallbackReason::MetadataExtractionFailed { .. } => FallbackStrategy::RelaxedRetry,

            // Unrecoverable
            FallbackReason::FileTooShort { .. } | FallbackReason::DecodingFailed { .. } => {
                FallbackStrategy::Skip
            }
        }
    }
}

/// Result of fallback handling
#[derive(Debug)]
pub struct FallbackResult {
    /// Strategy that was applied
    pub strategy: FallbackStrategy,
    /// Original reason for fallback
    pub reason: FallbackReason,
    /// Updated classification result (if applicable)
    pub classification: Option<ClassificationResult>,
    /// Whether file should be queued for manual review
    pub needs_manual_review: bool,
    /// User-facing status message
    pub status_message: String,
}

/// Fallback handler service
///
/// **[PLAN026]** Manages files that fail automated identification
pub struct FallbackHandler {
    /// Database connection pool
    pool: SqlitePool,
    /// Minimum file duration to attempt processing (seconds)
    min_duration_secs: f64,
    /// Artist similarity threshold for relaxed matching
    relaxed_artist_threshold: f64,
}

impl FallbackHandler {
    /// Create new fallback handler
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            min_duration_secs: 10.0, // Files shorter than 10s are skipped
            relaxed_artist_threshold: 0.30, // 30% similarity for relaxed matching
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(
        pool: SqlitePool,
        min_duration_secs: f64,
        relaxed_artist_threshold: f64,
    ) -> Self {
        Self {
            pool,
            min_duration_secs,
            relaxed_artist_threshold,
        }
    }

    /// Handle a failed identification
    ///
    /// **[REQ-FALLBACK-001]** Determines appropriate fallback strategy and
    /// returns updated classification if possible.
    pub async fn handle_failure(
        &self,
        file_id: Uuid,
        reason: FallbackReason,
        original_metadata: Option<&FileMetadata>,
    ) -> Result<FallbackResult> {
        let strategy = reason.recommended_strategy();

        info!(
            file_id = %file_id,
            reason = ?reason,
            strategy = strategy.as_str(),
            "Handling identification failure"
        );

        let result = match strategy {
            FallbackStrategy::MetadataOnly => {
                self.handle_metadata_only(file_id, &reason, original_metadata)
                    .await?
            }
            FallbackStrategy::ManualReview => self.handle_manual_review(file_id, &reason).await?,
            FallbackStrategy::RelaxedRetry => self.handle_relaxed_retry(file_id, &reason).await?,
            FallbackStrategy::Skip => self.handle_skip(file_id, &reason).await?,
        };

        // Update file record with fallback information
        self.update_file_fallback_status(file_id, &result).await?;

        Ok(result)
    }

    /// Handle metadata-only fallback
    ///
    /// Uses local ID3 tags/filename to classify without MusicBrainz match.
    async fn handle_metadata_only(
        &self,
        file_id: Uuid,
        reason: &FallbackReason,
        metadata: Option<&FileMetadata>,
    ) -> Result<FallbackResult> {
        debug!(
            file_id = %file_id,
            "Applying metadata-only fallback"
        );

        // Create classification based on available metadata
        let classification = if let Some(meta) = metadata {
            Some(ClassificationResult {
                content_type: meta.inferred_content_type(),
                confidence: MatchConfidence::Poor, // No MusicBrainz verification
                confidence_value: 0.0,
                recording_mbid: None,
                release_mbid: None,
                match_percentage: None,
                artist_verified: false,
                matching_stage: Some("fallback_metadata".to_string()),
            })
        } else {
            Some(ClassificationResult {
                content_type: ContentType::NotInMusicbrainz,
                confidence: MatchConfidence::Poor,
                confidence_value: 0.0,
                recording_mbid: None,
                release_mbid: None,
                match_percentage: None,
                artist_verified: false,
                matching_stage: Some("fallback_none".to_string()),
            })
        };

        Ok(FallbackResult {
            strategy: FallbackStrategy::MetadataOnly,
            reason: reason.clone(),
            classification,
            needs_manual_review: false,
            status_message: format!(
                "File classified using local metadata only. {}",
                reason.description()
            ),
        })
    }

    /// Handle manual review fallback
    ///
    /// Marks file for human review without automated classification.
    async fn handle_manual_review(
        &self,
        file_id: Uuid,
        reason: &FallbackReason,
    ) -> Result<FallbackResult> {
        debug!(
            file_id = %file_id,
            "Marking file for manual review"
        );

        let classification = ClassificationResult {
            content_type: ContentType::IdentificationFailed,
            confidence: MatchConfidence::Poor,
            confidence_value: 0.0,
            recording_mbid: None,
            release_mbid: None,
            match_percentage: None,
            artist_verified: false,
            matching_stage: Some("fallback_manual_review".to_string()),
        };

        Ok(FallbackResult {
            strategy: FallbackStrategy::ManualReview,
            reason: reason.clone(),
            classification: Some(classification),
            needs_manual_review: true,
            status_message: format!("File requires manual review. {}", reason.description()),
        })
    }

    /// Handle relaxed retry fallback
    ///
    /// Suggests retry with less strict matching parameters.
    async fn handle_relaxed_retry(
        &self,
        file_id: Uuid,
        reason: &FallbackReason,
    ) -> Result<FallbackResult> {
        debug!(
            file_id = %file_id,
            "Suggesting relaxed retry"
        );

        // For now, mark as needing retry - actual retry would be handled by caller
        let classification = ClassificationResult {
            content_type: ContentType::IdentificationFailed,
            confidence: MatchConfidence::Poor,
            confidence_value: 0.0,
            recording_mbid: None,
            release_mbid: None,
            match_percentage: None,
            artist_verified: false,
            matching_stage: Some("fallback_relaxed_retry".to_string()),
        };

        Ok(FallbackResult {
            strategy: FallbackStrategy::RelaxedRetry,
            reason: reason.clone(),
            classification: Some(classification),
            needs_manual_review: false,
            status_message: format!(
                "File may benefit from retry with relaxed parameters. {}",
                reason.description()
            ),
        })
    }

    /// Handle skip fallback
    ///
    /// Marks file as unprocessable.
    async fn handle_skip(&self, file_id: Uuid, reason: &FallbackReason) -> Result<FallbackResult> {
        warn!(
            file_id = %file_id,
            reason = %reason.description(),
            "Skipping unprocessable file"
        );

        let classification = ClassificationResult {
            content_type: ContentType::IdentificationFailed,
            confidence: MatchConfidence::Poor,
            confidence_value: 0.0,
            recording_mbid: None,
            release_mbid: None,
            match_percentage: None,
            artist_verified: false,
            matching_stage: Some("fallback_skip".to_string()),
        };

        Ok(FallbackResult {
            strategy: FallbackStrategy::Skip,
            reason: reason.clone(),
            classification: Some(classification),
            needs_manual_review: false,
            status_message: format!("File cannot be processed. {}", reason.description()),
        })
    }

    /// Update file record with fallback status
    async fn update_file_fallback_status(
        &self,
        file_id: Uuid,
        result: &FallbackResult,
    ) -> Result<()> {
        let content_type = result
            .classification
            .as_ref()
            .map(|c| c.content_type.as_str())
            .unwrap_or("IDENTIFICATION_FAILED");

        let status = &result.status_message;

        sqlx::query(
            r#"
            UPDATE files
            SET content_type = ?,
                fallback_strategy = ?,
                fallback_reason = ?,
                needs_manual_review = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE guid = ?
            "#,
        )
        .bind(content_type)
        .bind(result.strategy.as_str())
        .bind(status)
        .bind(if result.needs_manual_review { 1 } else { 0 })
        .bind(file_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if file duration is too short
    pub fn is_too_short(&self, duration_secs: f64) -> bool {
        duration_secs < self.min_duration_secs
    }

    /// Get relaxed artist similarity threshold
    pub fn relaxed_artist_threshold(&self) -> f64 {
        self.relaxed_artist_threshold
    }
}

/// File metadata for fallback classification
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Artist name from ID3/filename
    pub artist: Option<String>,
    /// Album name from ID3/filename
    pub album: Option<String>,
    /// Title from ID3/filename
    pub title: Option<String>,
    /// Track number from ID3
    pub track_number: Option<u32>,
    /// Total tracks from ID3
    pub total_tracks: Option<u32>,
    /// Duration in seconds
    pub duration_secs: f64,
}

impl FileMetadata {
    /// Infer content type from available metadata
    pub fn inferred_content_type(&self) -> ContentType {
        // If we have track information suggesting multiple tracks
        if let (Some(track), Some(total)) = (self.track_number, self.total_tracks) {
            if total > 1 && track <= total {
                // Looks like part of an album
                return ContentType::PartialAlbum;
            }
        }

        // If duration suggests album (>25 minutes)
        if self.duration_secs > 25.0 * 60.0 {
            return ContentType::FullAlbum;
        }

        // If duration suggests single song (<12 minutes)
        if self.duration_secs < 12.0 * 60.0 {
            return ContentType::SingleSong;
        }

        // Ambiguous - could be either
        ContentType::NotInMusicbrainz
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_strategy_strings() {
        assert_eq!(FallbackStrategy::MetadataOnly.as_str(), "metadata_only");
        assert_eq!(FallbackStrategy::ManualReview.as_str(), "manual_review");
        assert_eq!(FallbackStrategy::RelaxedRetry.as_str(), "relaxed_retry");
        assert_eq!(FallbackStrategy::Skip.as_str(), "skip");
    }

    #[test]
    fn test_fallback_reason_strategies() {
        let no_candidates = FallbackReason::NoCandidates {
            query: "test".to_string(),
        };
        assert_eq!(
            no_candidates.recommended_strategy(),
            FallbackStrategy::MetadataOnly
        );

        let too_short = FallbackReason::FileTooShort { duration_secs: 5.0 };
        assert_eq!(too_short.recommended_strategy(), FallbackStrategy::Skip);

        let artist_failed = FallbackReason::ArtistVerificationFailed {
            expected: "Artist A".to_string(),
            found: "Artist B".to_string(),
            similarity: 0.3,
        };
        assert_eq!(
            artist_failed.recommended_strategy(),
            FallbackStrategy::ManualReview
        );
    }

    #[test]
    fn test_file_metadata_inferred_type() {
        // Short file -> single song
        let short_file = FileMetadata {
            artist: Some("Artist".to_string()),
            album: None,
            title: Some("Track".to_string()),
            track_number: None,
            total_tracks: None,
            duration_secs: 180.0, // 3 minutes
        };
        assert_eq!(short_file.inferred_content_type(), ContentType::SingleSong);

        // Long file -> full album
        let long_file = FileMetadata {
            artist: Some("Artist".to_string()),
            album: Some("Album".to_string()),
            title: None,
            track_number: None,
            total_tracks: None,
            duration_secs: 45.0 * 60.0, // 45 minutes
        };
        assert_eq!(long_file.inferred_content_type(), ContentType::FullAlbum);

        // Track metadata -> partial album
        let track_file = FileMetadata {
            artist: Some("Artist".to_string()),
            album: Some("Album".to_string()),
            title: Some("Track 3".to_string()),
            track_number: Some(3),
            total_tracks: Some(12),
            duration_secs: 240.0,
        };
        assert_eq!(
            track_file.inferred_content_type(),
            ContentType::PartialAlbum
        );
    }

    #[tokio::test]
    async fn test_fallback_handler_creation() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        let handler = FallbackHandler::new(pool);
        assert_eq!(handler.min_duration_secs, 10.0);
        assert_eq!(handler.relaxed_artist_threshold, 0.30);
    }
}

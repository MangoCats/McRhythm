//! ID3 Metadata Extractor (Tier 1)
//!
//! Extracts ID3 metadata tags from audio files using the `lofty` crate.
//! Supports ID3v2, ID3v1, and various other tag formats (APE, Vorbis, MP4, etc.).
//!
//! # Implementation
//! - TASK-005: ID3 Extractor (PLAN024)
//! - Confidence: 0.6 (user-editable metadata, may be incomplete or incorrect)
//!
//! # Architecture
//! Implements `SourceExtractor` trait for integration with parallel extraction pipeline.
//! Gracefully handles missing tags by returning None for unavailable fields.

use crate::types::{
    ConfidenceValue, ExtractionError, ExtractionResult, MetadataExtraction, PassageContext,
    SourceExtractor,
};
use async_trait::async_trait;
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag};
use std::path::Path;
use tracing::{debug, warn};

/// ID3 Metadata Extractor
///
/// Extracts metadata from audio file tags using lofty crate.
/// Supports ID3v2, ID3v1, APE, Vorbis Comments, MP4, and other formats.
///
/// # Confidence
/// Base confidence: 0.6
/// - User-editable metadata (may be incomplete or incorrect)
/// - No verification against authoritative sources
/// - Higher than genre mapping (0.5) but lower than MusicBrainz (0.9)
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::extractors::id3_extractor::ID3Extractor;
/// use wkmp_ai::types::{SourceExtractor, PassageContext};
///
/// let extractor = ID3Extractor::new();
/// let result = extractor.extract(&passage_ctx).await?;
///
/// if let Some(metadata) = result.metadata {
///     if let Some(title) = metadata.title {
///         println!("Title: {} (confidence: {})", title.value, title.confidence);
///     }
/// }
/// ```
pub struct ID3Extractor {
    /// Base confidence for ID3 metadata
    base_confidence: f32,
}

impl ID3Extractor {
    /// Create new ID3 extractor with default confidence (0.6)
    pub fn new() -> Self {
        Self {
            base_confidence: 0.6,
        }
    }

    /// Create new ID3 extractor with custom confidence
    pub fn with_confidence(confidence: f32) -> Self {
        Self {
            base_confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Extract metadata from audio file
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    ///
    /// # Returns
    /// Metadata extraction with confidence-scored fields
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed
    fn extract_metadata(&self, file_path: &Path) -> Result<MetadataExtraction, ExtractionError> {
        // Probe file to determine format
        let tagged_file = Probe::open(file_path)
            .map_err(|e| ExtractionError::Io(std::io::Error::other(e)))?
            .read()
            .map_err(|e| {
                ExtractionError::Parse(format!("Failed to read audio file tags: {}", e))
            })?;

        // Get primary tag (ID3v2 preferred, falls back to others)
        let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

        let Some(tag) = tag else {
            debug!(file = ?file_path, "No tags found in audio file");
            return Ok(MetadataExtraction::default());
        };

        // Extract standard fields
        let title = self.extract_title(tag);
        let artist = self.extract_artist(tag);
        let album = self.extract_album(tag);
        let recording_mbid = self.extract_musicbrainz_recording_id(tag);

        // Extract additional fields
        let mut additional = std::collections::HashMap::new();

        // Genre
        if let Some(genre) = tag.genre() {
            additional.insert(
                "genre".to_string(),
                ConfidenceValue::new(genre.to_string(), self.base_confidence, "ID3"),
            );
        }

        // Year/Date
        if let Some(year) = tag.year() {
            additional.insert(
                "year".to_string(),
                ConfidenceValue::new(year.to_string(), self.base_confidence, "ID3"),
            );
        }

        // Track number
        if let Some(track) = tag.track() {
            additional.insert(
                "track_number".to_string(),
                ConfidenceValue::new(track.to_string(), self.base_confidence, "ID3"),
            );
        }

        // Album artist
        if let Some(album_artist) = tag.get_string(&ItemKey::AlbumArtist) {
            additional.insert(
                "album_artist".to_string(),
                ConfidenceValue::new(album_artist.to_string(), self.base_confidence, "ID3"),
            );
        }

        // Composer
        if let Some(composer) = tag.get_string(&ItemKey::Composer) {
            additional.insert(
                "composer".to_string(),
                ConfidenceValue::new(composer.to_string(), self.base_confidence, "ID3"),
            );
        }

        Ok(MetadataExtraction {
            title,
            artist,
            album,
            recording_mbid,
            additional,
        })
    }

    fn extract_title(&self, tag: &Tag) -> Option<ConfidenceValue<String>> {
        tag.title().map(|title| {
            ConfidenceValue::new(title.to_string(), self.base_confidence, "ID3")
        })
    }

    fn extract_artist(&self, tag: &Tag) -> Option<ConfidenceValue<String>> {
        tag.artist().map(|artist| {
            ConfidenceValue::new(artist.to_string(), self.base_confidence, "ID3")
        })
    }

    fn extract_album(&self, tag: &Tag) -> Option<ConfidenceValue<String>> {
        tag.album().map(|album| {
            ConfidenceValue::new(album.to_string(), self.base_confidence, "ID3")
        })
    }

    fn extract_musicbrainz_recording_id(&self, tag: &Tag) -> Option<ConfidenceValue<String>> {
        // MusicBrainz Recording ID stored in TXXX frame (ID3v2) or custom tag
        // Try various common keys
        let mbid_keys = [
            ItemKey::MusicBrainzRecordingId,
            ItemKey::Unknown("MUSICBRAINZ_TRACKID".to_string()),
            ItemKey::Unknown("MusicBrainz Recording Id".to_string()),
        ];

        for key in &mbid_keys {
            if let Some(mbid) = tag.get_string(key) {
                // Validate MBID format (UUID)
                if is_valid_mbid(mbid) {
                    debug!(mbid = %mbid, "Found MusicBrainz Recording ID in ID3 tags");
                    // Higher confidence when MBID is present (0.9 - authoritative)
                    return Some(ConfidenceValue::new(
                        mbid.to_string(),
                        0.9,
                        "ID3-MBID",
                    ));
                } else {
                    warn!(mbid = %mbid, "Invalid MusicBrainz Recording ID format in ID3 tags");
                }
            }
        }

        None
    }
}

impl Default for ID3Extractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceExtractor for ID3Extractor {
    fn name(&self) -> &'static str {
        "ID3"
    }

    fn base_confidence(&self) -> f32 {
        self.base_confidence
    }

    async fn extract(&self, ctx: &PassageContext) -> Result<ExtractionResult, ExtractionError> {
        debug!(
            passage_id = %ctx.passage_id,
            file_path = ?ctx.file_path,
            "Extracting ID3 metadata"
        );

        let metadata = self.extract_metadata(&ctx.file_path)?;

        // Log what was extracted
        let field_count = [
            metadata.title.is_some(),
            metadata.artist.is_some(),
            metadata.album.is_some(),
            metadata.recording_mbid.is_some(),
        ]
        .iter()
        .filter(|&&present| present)
        .count()
            + metadata.additional.len();

        debug!(
            passage_id = %ctx.passage_id,
            field_count = field_count,
            has_mbid = metadata.recording_mbid.is_some(),
            "ID3 extraction complete"
        );

        Ok(ExtractionResult {
            metadata: Some(metadata),
            identity: None,      // ID3 extractor doesn't perform identity resolution
            musical_flavor: None, // Musical flavor comes from other extractors
        })
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Validate MusicBrainz ID format (UUID v4)
///
/// MBIDs are 36-character UUIDs: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
fn is_valid_mbid(mbid: &str) -> bool {
    if mbid.len() != 36 {
        return false;
    }

    // Check UUID format: 8-4-4-4-12 hex digits with hyphens
    let parts: Vec<&str> = mbid.split('-').collect();
    if parts.len() != 5 {
        return false;
    }

    if parts[0].len() != 8
        || parts[1].len() != 4
        || parts[2].len() != 4
        || parts[3].len() != 4
        || parts[4].len() != 12
    {
        return false;
    }

    // Verify all characters are hex digits or hyphens
    mbid.chars()
        .all(|c| c.is_ascii_hexdigit() || c == '-')
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn test_extractor_name() {
        let extractor = ID3Extractor::new();
        assert_eq!(extractor.name(), "ID3");
    }

    #[test]
    fn test_default_confidence() {
        let extractor = ID3Extractor::new();
        assert_eq!(extractor.base_confidence(), 0.6);
    }

    #[test]
    fn test_custom_confidence() {
        let extractor = ID3Extractor::with_confidence(0.8);
        assert_eq!(extractor.base_confidence(), 0.8);
    }

    #[test]
    fn test_confidence_clamping() {
        let extractor = ID3Extractor::with_confidence(1.5);
        assert_eq!(extractor.base_confidence(), 1.0, "Should clamp to 1.0");

        let extractor = ID3Extractor::with_confidence(-0.5);
        assert_eq!(extractor.base_confidence(), 0.0, "Should clamp to 0.0");
    }

    #[test]
    fn test_valid_mbid_format() {
        // Valid UUIDs
        assert!(is_valid_mbid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_valid_mbid("c87e8e0f-5b43-4f7e-9a5b-d7c3f7d5e8a0"));

        // Invalid formats
        assert!(!is_valid_mbid("550e8400-e29b-41d4-a716")); // Too short
        assert!(!is_valid_mbid("not-a-uuid"));
        assert!(!is_valid_mbid("550e8400e29b41d4a716446655440000")); // No hyphens
        assert!(!is_valid_mbid("550e8400-e29b-41d4-a716-446655440000-extra")); // Too long
    }

    #[tokio::test]
    async fn test_extract_nonexistent_file() {
        let extractor = ID3Extractor::new();
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/nonexistent/file.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: None,
            sample_rate: Some(44100),
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        let result = extractor.extract(&ctx).await;
        assert!(result.is_err(), "Should fail for nonexistent file");
    }

    #[test]
    fn test_default_trait() {
        let extractor = ID3Extractor::default();
        assert_eq!(extractor.base_confidence(), 0.6);
    }

    // Note: Testing with real audio files requires test fixtures
    // Integration tests with test audio files should be added separately
}

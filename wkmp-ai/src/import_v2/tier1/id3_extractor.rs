// PLAN023 Tier 1: ID3 Metadata Extractor
//
// Extracts metadata from ID3v2/ID3v1 tags using lofty crate.
// Supports MP3, FLAC, and other common audio formats.
//
// Requirements: REQ-AI-030 (ID3 Metadata Extraction)
// Architecture: Tier 1 (Source Extractor)

use crate::import_v2::types::{
    ExtractionSource, ExtractorResult, ImportError, ImportResult, MetadataBundle, MetadataField,
};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use std::path::Path;
use tracing::{debug, warn};
use uuid::Uuid;  // REQ-TD-004: For MBID parsing

/// ID3 metadata extractor using lofty
///
/// **[P1-4]** Extracts title, artist, album, release date, track number, duration
pub struct ID3Extractor {
    /// Confidence level for ID3 tag data (0.0 to 1.0)
    /// ID3 tags are user-edited, so moderate confidence (0.70)
    confidence: f64,
}

impl Default for ID3Extractor {
    fn default() -> Self {
        Self { confidence: 0.70 }
    }
}

impl ID3Extractor {
    /// Create new ID3 extractor with custom confidence
    pub fn new(confidence: f64) -> Self {
        Self { confidence }
    }

    /// Extract metadata from audio file tags
    ///
    /// **[REQ-AI-030]** Extract metadata from ID3v2/ID3v1 tags
    ///
    /// # Returns
    /// * `Ok(ExtractorResult<MetadataBundle>)` - Metadata fields with source tracking
    /// * `Err(ImportError)` - File not found, unsupported format, or read error
    ///
    /// # Algorithm
    /// 1. Probe file to detect format
    /// 2. Read primary tag (ID3v2 preferred over ID3v1)
    /// 3. Extract standard fields: title, artist, album, date, track, duration
    /// 4. Parse duration from audio properties
    /// 5. Assign confidence based on tag type (ID3v2 > ID3v1)
    pub fn extract(&self, file_path: &Path) -> ImportResult<ExtractorResult<MetadataBundle>> {
        debug!("Extracting ID3 metadata from: {}", file_path.display());

        // Probe file to detect format
        let tagged_file = Probe::open(file_path)
            .map_err(|e| {
                ImportError::ExtractionFailed(format!("Failed to open file: {}", e))
            })?
            .read()
            .map_err(|e| {
                ImportError::ExtractionFailed(format!("Failed to read file tags: {}", e))
            })?;

        // Get primary tag (prefer ID3v2 over ID3v1)
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())
            .ok_or_else(|| {
                ImportError::ExtractionFailed("No tags found in file".to_string())
            })?;

        let mut bundle = MetadataBundle::default();

        // Extract title
        if let Some(title) = tag.title() {
            bundle.title.push(MetadataField {
                value: title.to_string(),
                confidence: self.confidence,
                source: ExtractionSource::ID3Metadata,
            });
            debug!("  Title: {}", title);
        }

        // Extract artist
        if let Some(artist) = tag.artist() {
            bundle.artist.push(MetadataField {
                value: artist.to_string(),
                confidence: self.confidence,
                source: ExtractionSource::ID3Metadata,
            });
            debug!("  Artist: {}", artist);
        }

        // Extract album
        if let Some(album) = tag.album() {
            bundle.album.push(MetadataField {
                value: album.to_string(),
                confidence: self.confidence,
                source: ExtractionSource::ID3Metadata,
            });
            debug!("  Album: {}", album);
        }

        // Extract release date (year)
        if let Some(year) = tag.year() {
            bundle.release_date.push(MetadataField {
                value: year.to_string(),
                confidence: self.confidence,
                source: ExtractionSource::ID3Metadata,
            });
            debug!("  Year: {}", year);
        }

        // Extract track number
        if let Some(track) = tag.track() {
            bundle.track_number.push(MetadataField {
                value: track,
                confidence: self.confidence,
                source: ExtractionSource::ID3Metadata,
            });
            debug!("  Track: {}", track);
        }

        // Extract duration from audio properties (AudioFile trait provides properties())
        let duration_ms = tagged_file.properties().duration().as_millis() as u32;
        bundle.duration_ms.push(MetadataField {
            value: duration_ms,
            confidence: 1.0, // Duration is always accurate from audio properties
            source: ExtractionSource::ID3Metadata,
        });
        debug!("  Duration: {} ms", duration_ms);

        debug!(
            "Extracted {} metadata fields from ID3 tags",
            bundle.title.len()
                + bundle.artist.len()
                + bundle.album.len()
                + bundle.release_date.len()
                + bundle.track_number.len()
                + bundle.duration_ms.len()
        );

        Ok(ExtractorResult {
            data: bundle,
            confidence: self.confidence,
            source: ExtractionSource::ID3Metadata,
        })
    }

    /// Extract MusicBrainz Recording ID from ID3 UFID frame
    ///
    /// Searches for UFID (Unique File Identifier) frame with owner "http://musicbrainz.org"
    /// and extracts the MBID if present.
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    ///
    /// # Returns
    /// * `Ok(Some(ExtractorResult<Vec<MBIDCandidate>>>>` - MBID found and parsed successfully
    /// * `Ok(None)` - No UFID frame found or MBID parsing failed
    /// * `Err(ImportError)` - File not found or read error
    ///
    /// # Notes
    /// - MusicBrainz UFID owner: "http://musicbrainz.org"
    /// - MBID format: UUID string (e.g., "84614f2a-8768-46ca-93e0-d9a5ee001cce")
    /// - Confidence: 0.85 (higher than regular ID3 fields - less user-editable)
    pub fn extract_mbid(
        &self,
        file_path: &Path,
    ) -> ImportResult<Option<ExtractorResult<Vec<crate::import_v2::types::MBIDCandidate>>>> {
        
        

        debug!("Extracting MBID from UFID frame: {}", file_path.display());

        // Probe file
        let tagged_file = Probe::open(file_path)
            .map_err(|e| {
                ImportError::ExtractionFailed(format!("Failed to open file: {}", e))
            })?
            .read()
            .map_err(|e| {
                ImportError::ExtractionFailed(format!("Failed to read file tags: {}", e))
            })?;

        // Get primary tag
        let tag = match tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
            Some(t) => t,
            None => {
                debug!("  No tags found, cannot extract UFID");
                return Ok(None);
            }
        };

        // lofty doesn't provide direct UFID access through Accessor trait
        // We need to check if the tag is ID3v2 and access frames directly
        use lofty::tag::TagType;
        if tag.tag_type() != TagType::Id3v2 {
            debug!("  Not an ID3v2 tag, skipping UFID extraction");
            return Ok(None);
        }

        // REQ-TD-004: Extract MBID from UFID frame using id3 crate
        // lofty doesn't expose UFID frames, so we use id3 crate for MP3 files

        // Check if this is an MP3 file (UFID frames only in MP3)
        if file_path.extension().and_then(|s| s.to_str()) != Some("mp3") {
            debug!("  Not an MP3 file, UFID extraction only supported for MP3");
            return Ok(None);
        }

        // Use id3 crate to access UFID frames
        match self.extract_mbid_from_ufid_mp3(file_path) {
            Some(mbid) => {
                debug!("  Successfully extracted MBID from UFID: {}", mbid);

                // Create MBIDCandidate with high confidence (0.95 - less user-editable than regular ID3)
                use crate::import_v2::types::MBIDCandidate;
                let candidate = MBIDCandidate {
                    mbid,
                    confidence: 0.95,  // High confidence for UFID frames
                    sources: vec![ExtractionSource::ID3Metadata],
                };

                Ok(Some(ExtractorResult {
                    data: vec![candidate],
                    source: ExtractionSource::ID3Metadata,
                    confidence: 0.95,
                }))
            }
            None => {
                debug!("  No MusicBrainz UFID frame found");
                Ok(None)
            }
        }
    }

    /// Extract MBID from MP3 UFID frame using id3 crate
    ///
    /// REQ-TD-004: MusicBrainz Recording ID extraction
    ///
    /// # Arguments
    /// * `file_path` - Path to MP3 file
    ///
    /// # Returns
    /// * `Some(Uuid)` - MBID if valid UFID frame found
    /// * `None` - No UFID frame or invalid MBID format
    fn extract_mbid_from_ufid_mp3(&self, file_path: &Path) -> Option<Uuid> {
        use id3::Tag;

        // Read ID3 tags using id3 crate
        let tag = match Tag::read_from_path(file_path) {
            Ok(t) => t,
            Err(e) => {
                debug!("  Failed to read ID3 tags: {}", e);
                return None;
            }
        };

        // Search for MusicBrainz UFID frame
        // Note: id3 crate uses different frame ID naming
        for frame in tag.frames() {
            // UFID frames have ID "UFID"
            if frame.id() == "UFID" {
                if let id3::Content::Unknown(data) = frame.content() {
                    // UFID frame structure: owner\0identifier
                    // Find the null byte that separates owner from identifier
                    if let Some(null_pos) = data.data.iter().position(|&b| b == 0) {
                        let owner = String::from_utf8_lossy(&data.data[..null_pos]);

                        // Check if this is a MusicBrainz UFID
                        if owner == "http://musicbrainz.org" {
                            // Extract identifier (after null byte)
                            let identifier = &data.data[null_pos + 1..];
                            let mbid_str = String::from_utf8_lossy(identifier);

                            if let Ok(uuid) = Uuid::parse_str(&mbid_str) {
                                debug!("  Extracted MBID from UFID: {}", uuid);
                                return Some(uuid);
                            } else {
                                debug!("  Invalid MBID format in UFID: {}", mbid_str);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract genre from audio file tags
    ///
    /// # Returns
    /// * `Ok(Some(String))` - Genre string if present
    /// * `Ok(None)` - No genre tag found
    /// * `Err(ImportError)` - File not found or read error
    pub fn extract_genre(&self, file_path: &Path) -> ImportResult<Option<String>> {
        debug!("Extracting genre from: {}", file_path.display());

        // Probe file
        let tagged_file = Probe::open(file_path)
            .map_err(|e| {
                ImportError::ExtractionFailed(format!("Failed to open file: {}", e))
            })?
            .read()
            .map_err(|e| {
                ImportError::ExtractionFailed(format!("Failed to read file tags: {}", e))
            })?;

        // Get primary tag
        let tag = match tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
            Some(t) => t,
            None => {
                warn!("No tags found in file: {}", file_path.display());
                return Ok(None);
            }
        };

        // Extract genre
        if let Some(genre) = tag.genre() {
            debug!("  Genre: {}", genre);
            Ok(Some(genre.to_string()))
        } else {
            debug!("  No genre tag found");
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // [P1-4] ID3 Extractor Tests
    // ============================================================================

    #[test]
    fn test_extractor_default_confidence() {
        let extractor = ID3Extractor::default();
        assert_eq!(extractor.confidence, 0.70, "Default confidence should be 0.70");
    }

    #[test]
    fn test_extractor_custom_confidence() {
        let extractor = ID3Extractor::new(0.85);
        assert_eq!(extractor.confidence, 0.85, "Custom confidence should be preserved");
    }

    #[test]
    fn test_extract_nonexistent_file() {
        let extractor = ID3Extractor::default();
        let result = extractor.extract(Path::new("nonexistent_file_12345.mp3"));
        assert!(
            result.is_err(),
            "Extracting from nonexistent file should fail"
        );
    }

    #[test]
    fn test_extract_genre_nonexistent_file() {
        let extractor = ID3Extractor::default();
        let result = extractor.extract_genre(Path::new("nonexistent_file_12345.mp3"));
        assert!(
            result.is_err(),
            "Extracting genre from nonexistent file should fail"
        );
    }

    // Note: Full integration tests with real MP3 files require test fixtures.
    // Creating minimal valid MP3 files programmatically is complex and brittle.
    //
    // These tests are validated through:
    // 1. Manual testing with real audio files during development
    // 2. Integration tests that use real audio files from test fixtures
    // 3. The SongWorkflowEngine integration tests (P1-5) which test the full pipeline
    //
    // Unit tests above verify:
    // - Default confidence is correct (0.70)
    // - Custom confidence works
    // - Nonexistent files return errors appropriately
    // - The extractor can be instantiated correctly
}

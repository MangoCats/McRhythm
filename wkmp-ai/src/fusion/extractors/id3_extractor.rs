// ID3 Metadata Extractor
//
// PLAN023: REQ-AI-031 - Extract metadata from ID3 tags
// Confidence: 0.7-0.9 (depends on tag quality)

use crate::fusion::extractors::Extractor;
use crate::fusion::{
    ExtractionResult, MetadataExtraction, IdentityExtraction, Confidence,
};
use anyhow::{Result, Context};
use async_trait::async_trait;
use std::path::Path;
use lofty::config::ParseOptions;
use lofty::file::TaggedFileExt;
use lofty::prelude::{Accessor, AudioFile};
use lofty::probe::Probe;
use lofty::tag::ItemKey;
use tracing::{debug, warn};

pub struct Id3Extractor;

impl Default for Id3Extractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Id3Extractor {
    pub fn new() -> Self {
        Self
    }

    /// Determine confidence based on tag completeness
    fn calculate_confidence(&self, title: &Option<String>, artist: &Option<String>, mbid: &Option<String>) -> Confidence {
        let mut score: f64 = 0.0;

        // Title presence: +0.3
        if title.as_ref().map(|t| !t.trim().is_empty()).unwrap_or(false) {
            score += 0.3;
        }

        // Artist presence: +0.2
        if artist.as_ref().map(|a| !a.trim().is_empty()).unwrap_or(false) {
            score += 0.2;
        }

        // MBID presence: +0.4 (high value for identity resolution)
        if mbid.as_ref().map(|m| !m.trim().is_empty()).unwrap_or(false) {
            score += 0.4;
        }

        // Base confidence: 0.5 (even with minimal tags)
        score += 0.5;

        score.min(0.9) // Cap at 0.9 (ID3 tags can have typos/errors)
    }
}

#[async_trait]
impl Extractor for Id3Extractor {
    fn source_id(&self) -> &'static str {
        "ID3"
    }

    async fn extract(
        &self,
        file_path: &Path,
        _start_seconds: f64,
        _end_seconds: f64,
    ) -> Result<ExtractionResult> {
        debug!("Extracting ID3 tags from: {}", file_path.display());

        // Parse audio file tags using lofty
        let tagged_file = Probe::open(file_path)
            .context("Failed to open audio file")?
            .options(ParseOptions::new().read_properties(true))
            .read()
            .context("Failed to read audio file tags")?;

        // Extract primary tag (prefers ID3v2 > Vorbis > APE > ID3v1)
        let tag = match tagged_file.primary_tag() {
            Some(t) => t,
            None => {
                warn!("No tags found in file: {}", file_path.display());
                return Ok(ExtractionResult {
                    source: self.source_id().to_string(),
                    confidence: 0.5,
                    timestamp: chrono::Utc::now().timestamp(),
                    metadata: None,
                    flavor: None,
                    identity: None,
                });
            }
        };

        // Extract fields
        let title = tag.title().map(|t| t.to_string());
        let artist = tag.artist().map(|a| a.to_string());
        let album = tag.album().map(|a| a.to_string());

        // Extract MusicBrainz Recording ID (if present)
        let recording_mbid = tag
            .get_string(&ItemKey::MusicBrainzRecordingId)
            .map(|s| s.to_string());

        // Get duration from audio properties
        let duration_seconds = tagged_file.properties().duration().as_secs_f64();

        // Calculate confidence
        let confidence = self.calculate_confidence(&title, &artist, &recording_mbid);

        // Field-specific confidence (same as overall for ID3)
        let title_confidence = if title.is_some() { Some(confidence) } else { None };
        let artist_confidence = if artist.is_some() { Some(confidence) } else { None };

        // Build extraction result
        let metadata = Some(MetadataExtraction {
            title: title.clone(),
            artist: artist.clone(),
            album,
            duration_seconds: Some(duration_seconds),
            title_confidence,
            artist_confidence,
        });

        let identity = recording_mbid.map(|mbid| IdentityExtraction {
            recording_mbid: mbid,
            confidence,
            context: None,
        });

        debug!(
            "ID3 extraction complete: title={:?}, artist={:?}, mbid={:?}, confidence={:.2}",
            title, artist, identity.as_ref().map(|i| &i.recording_mbid), confidence
        );

        Ok(ExtractionResult {
            source: self.source_id().to_string(),
            confidence,
            timestamp: chrono::Utc::now().timestamp(),
            metadata,
            flavor: None, // ID3 doesn't provide musical flavor directly
            identity,
        })
    }

    fn confidence_range(&self) -> (Confidence, Confidence) {
        (0.5, 0.9) // ID3 tags: moderate to high confidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_calculation() {
        let extractor = Id3Extractor::new();

        // Full tags (title + artist + mbid)
        let conf = extractor.calculate_confidence(
            &Some("Song Title".to_string()),
            &Some("Artist Name".to_string()),
            &Some("mbid-uuid-here".to_string()),
        );
        assert!((conf - 0.9).abs() < 0.01, "Full tags should yield 0.9 confidence");

        // No MBID
        let conf = extractor.calculate_confidence(
            &Some("Song Title".to_string()),
            &Some("Artist Name".to_string()),
            &None,
        );
        assert!((conf - 0.9).abs() < 0.01, "Should be capped at 0.9: {}", conf);

        // Only title
        let conf = extractor.calculate_confidence(
            &Some("Song Title".to_string()),
            &None,
            &None,
        );
        assert!((conf - 0.8).abs() < 0.01, "Title only should yield 0.8");

        // No tags
        let conf = extractor.calculate_confidence(&None, &None, &None);
        assert!((conf - 0.5).abs() < 0.01, "No tags should yield 0.5");
    }

    #[test]
    fn test_source_id() {
        let extractor = Id3Extractor::new();
        assert_eq!(extractor.source_id(), "ID3");
    }

    #[test]
    fn test_confidence_range() {
        let extractor = Id3Extractor::new();
        assert_eq!(extractor.confidence_range(), (0.5, 0.9));
    }
}

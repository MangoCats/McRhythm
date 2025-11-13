// Tier 1 Extractors - Parallel Source Extraction
//
// PLAN023: REQ-AI-031, REQ-AI-041 - Multi-source extraction with parallel execution
// 7 extractors: ID3, Chromaprint, AcoustID, MusicBrainz, Audio-derived, ID3Genre, (Essentia optional)

use crate::fusion::{ExtractionResult, Confidence};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

pub mod audio_extractor;
pub mod id3_extractor;
pub mod chromaprint_analyzer;
pub mod acoustid_client;
pub mod musicbrainz_client;
pub mod audio_derived_extractor;
pub mod genre_mapping;
// pub mod essentia_analyzer; // Deferred to future increment

// Re-export audio extraction helper
pub use audio_extractor::extract_passage_audio;

/// Extractor trait - all Tier 1 extractors implement this
#[async_trait]
pub trait Extractor: Send + Sync {
    /// Extractor identifier (e.g., "ID3", "AcoustID", "MusicBrainz")
    fn source_id(&self) -> &'static str;

    /// Extract data from audio passage
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    /// * `start_seconds` - Passage start time in seconds
    /// * `end_seconds` - Passage end time in seconds
    ///
    /// # Returns
    /// * `Ok(ExtractionResult)` - Extracted data with confidence scores
    /// * `Err(_)` - Extraction failed (logged but doesn't abort import)
    async fn extract(
        &self,
        file_path: &Path,
        start_seconds: f64,
        end_seconds: f64,
    ) -> Result<ExtractionResult>;

    /// Check if extractor is available (dependencies met, API keys configured, etc.)
    fn is_available(&self) -> bool {
        true // Default: assume available
    }

    /// Get expected confidence range for this extractor
    fn confidence_range(&self) -> (Confidence, Confidence) {
        (0.0, 1.0) // Default: full range
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_range_default() {
        struct DummyExtractor;

        #[async_trait]
        impl Extractor for DummyExtractor {
            fn source_id(&self) -> &'static str {
                "Dummy"
            }

            async fn extract(
                &self,
                _file_path: &Path,
                _start_seconds: f64,
                _end_seconds: f64,
            ) -> Result<ExtractionResult> {
                unimplemented!()
            }
        }

        let extractor = DummyExtractor;
        assert_eq!(extractor.confidence_range(), (0.0, 1.0));
        assert!(extractor.is_available());
    }
}

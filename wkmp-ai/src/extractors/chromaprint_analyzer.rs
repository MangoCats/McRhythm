//! Chromaprint Analyzer (Tier 1)
//!
//! Generates acoustic fingerprints using Chromaprint library.
//! Fingerprints are used for identity resolution via AcoustID.
//!
//! # Implementation
//! - TASK-006: Chromaprint Analyzer (PLAN024)
//! - Confidence: 1.0 (deterministic algorithm, 100% reproducible)
//!
//! # Architecture
//! Implements `SourceExtractor` trait for integration with parallel extraction pipeline.
//! Produces base64-encoded fingerprints for downstream AcoustID lookup (TASK-007).

use crate::ffi::chromaprint::ChromaprintContext;
use crate::types::{
    ExtractionError, ExtractionResult, IdentityExtraction, PassageContext, SourceExtractor,
};
use async_trait::async_trait;
use tracing::{debug, warn};

/// Chromaprint Analyzer
///
/// Generates acoustic fingerprints from audio samples using Chromaprint library.
/// Fingerprints are base64-encoded strings used for AcoustID lookup.
///
/// # Confidence
/// Base confidence: 1.0
/// - Deterministic algorithm (same input → same output)
/// - Industry-standard fingerprinting (AcoustID/MusicBrainz ecosystem)
/// - No uncertainty in fingerprint generation itself
/// - Note: Confidence of MBID lookup from AcoustID is separate (TASK-007)
///
/// # Requirements
/// - Audio samples in f32 format (mono or stereo)
/// - Sample rate (typically 44100 Hz)
/// - Number of channels (1 or 2)
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::extractors::chromaprint_analyzer::ChromaprintAnalyzer;
/// use wkmp_ai::types::{SourceExtractor, PassageContext};
///
/// let analyzer = ChromaprintAnalyzer::new();
/// let result = analyzer.extract(&passage_ctx).await?;
///
/// if let Some(identity) = result.identity {
///     println!("Fingerprint: {}", identity.recording_mbid);  // base64 fingerprint
/// }
/// ```
pub struct ChromaprintAnalyzer {
    /// Base confidence for fingerprint generation (always 1.0)
    base_confidence: f32,
}

impl ChromaprintAnalyzer {
    /// Create new Chromaprint analyzer with default confidence (1.0)
    pub fn new() -> Self {
        Self {
            base_confidence: 1.0,
        }
    }

    /// Generate fingerprint from audio samples (synchronous, runs on blocking pool)
    ///
    /// **[AIA-PERF-047]** Static method for spawn_blocking compatibility
    ///
    /// # Arguments
    /// * `samples` - Audio samples in f32 format (interleaved if stereo)
    /// * `sample_rate` - Sample rate in Hz
    /// * `num_channels` - Number of channels (1=mono, 2=stereo)
    /// * `base_confidence` - Base confidence value to return
    ///
    /// # Returns
    /// Base64-encoded fingerprint string
    ///
    /// # Errors
    /// Returns error if:
    /// - Chromaprint library not available
    /// - Audio format invalid (unsupported sample rate/channels)
    /// - Fingerprint generation fails (not enough audio data)
    fn generate_fingerprint_sync(
        samples: &[f32],
        sample_rate: u32,
        num_channels: u8,
        _base_confidence: f32,
    ) -> Result<String, ExtractionError> {
        // Validate inputs
        if samples.is_empty() {
            return Err(ExtractionError::Internal(
                "No audio samples provided".to_string(),
            ));
        }

        // Create Chromaprint context
        let mut ctx = ChromaprintContext::new().map_err(|e| {
            ExtractionError::NotAvailable(format!("Chromaprint library not available: {}", e))
        })?;

        // Generate fingerprint using high-level API
        // This handles: validation, start, f32→i16 conversion, feed, finish, get_fingerprint
        let fingerprint = ctx
            .generate_fingerprint(samples, sample_rate, num_channels)
            .map_err(|e| ExtractionError::Internal(format!("Fingerprint generation failed: {}", e)))?;

        debug!(
            fingerprint_length = fingerprint.len(),
            sample_count = samples.len(),
            sample_rate = sample_rate,
            channels = num_channels,
            "Fingerprint generated successfully"
        );

        Ok(fingerprint)
    }
}

impl Default for ChromaprintAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceExtractor for ChromaprintAnalyzer {
    fn name(&self) -> &'static str {
        "Chromaprint"
    }

    fn base_confidence(&self) -> f32 {
        self.base_confidence
    }

    async fn extract(&self, ctx: &PassageContext) -> Result<ExtractionResult, ExtractionError> {
        debug!(
            passage_id = %ctx.passage_id,
            file_path = ?ctx.file_path,
            "Extracting Chromaprint fingerprint"
        );

        // Check if audio samples are available
        let Some(ref samples) = ctx.audio_samples else {
            warn!(
                passage_id = %ctx.passage_id,
                "No audio samples available for fingerprinting"
            );
            return Err(ExtractionError::Internal(
                "No audio samples available".to_string(),
            ));
        };

        let Some(sample_rate) = ctx.sample_rate else {
            return Err(ExtractionError::Internal(
                "Sample rate not specified".to_string(),
            ));
        };

        let Some(num_channels) = ctx.num_channels else {
            return Err(ExtractionError::Internal(
                "Number of channels not specified".to_string(),
            ));
        };

        // Generate fingerprint on blocking thread pool
        // **[AIA-PERF-047]** CPU-intensive Chromaprint fingerprinting runs on blocking pool
        let samples_clone = samples.clone();
        let base_confidence = self.base_confidence;
        let fingerprint = tokio::task::spawn_blocking(move || {
            Self::generate_fingerprint_sync(&samples_clone, sample_rate, num_channels, base_confidence)
        })
        .await
        .map_err(|e| ExtractionError::Internal(format!("Fingerprint task panicked: {}", e)))??;

        debug!(
            passage_id = %ctx.passage_id,
            fingerprint_length = fingerprint.len(),
            "Chromaprint extraction complete"
        );

        // Return identity extraction with fingerprint
        // Note: This is NOT a MusicBrainz Recording MBID yet
        // The fingerprint will be sent to AcoustID in TASK-007
        // For now, we store the fingerprint in the recording_mbid field
        // as a temporary placeholder (will be replaced by actual MBID from AcoustID)
        Ok(ExtractionResult {
            metadata: None,
            identity: Some(IdentityExtraction {
                recording_mbid: fingerprint,
                confidence: self.base_confidence,
                source: "Chromaprint".to_string(),
            }),
            musical_flavor: None,
        })
    }
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
    fn test_analyzer_name() {
        let analyzer = ChromaprintAnalyzer::new();
        assert_eq!(analyzer.name(), "Chromaprint");
    }

    #[test]
    fn test_default_confidence() {
        let analyzer = ChromaprintAnalyzer::new();
        assert_eq!(analyzer.base_confidence(), 1.0);
    }

    #[test]
    fn test_default_trait() {
        let analyzer = ChromaprintAnalyzer::default();
        assert_eq!(analyzer.base_confidence(), 1.0);
    }

    #[test]
    fn test_generate_fingerprint_empty_samples() {
        let analyzer = ChromaprintAnalyzer::new();
        let result = ChromaprintAnalyzer::generate_fingerprint_sync(&[], 44100, 2, analyzer.base_confidence);
        assert!(
            result.is_err(),
            "Should fail for empty audio samples"
        );
        assert!(matches!(result.unwrap_err(), ExtractionError::Internal(_)));
    }

    #[test]
    fn test_generate_fingerprint_invalid_channels() {
        let analyzer = ChromaprintAnalyzer::new();
        let samples = vec![0.0f32; 44100]; // 1 second of silence

        let result = ChromaprintAnalyzer::generate_fingerprint_sync(&samples, 44100, 0, analyzer.base_confidence);
        assert!(result.is_err(), "Should fail for 0 channels");

        let result = ChromaprintAnalyzer::generate_fingerprint_sync(&samples, 44100, 3, analyzer.base_confidence);
        assert!(result.is_err(), "Should fail for 3 channels");
    }

    #[tokio::test]
    async fn test_extract_missing_samples() {
        let analyzer = ChromaprintAnalyzer::new();
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: None, // No samples
            sample_rate: Some(44100),
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        let result = analyzer.extract(&ctx).await;
        assert!(result.is_err(), "Should fail when no audio samples provided");
    }

    #[tokio::test]
    async fn test_extract_missing_sample_rate() {
        let analyzer = ChromaprintAnalyzer::new();
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: Some(vec![0.0f32; 44100]),
            sample_rate: None, // Missing
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        let result = analyzer.extract(&ctx).await;
        assert!(result.is_err(), "Should fail when sample rate not specified");
    }

    #[tokio::test]
    async fn test_extract_missing_channels() {
        let analyzer = ChromaprintAnalyzer::new();
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: Some(vec![0.0f32; 44100]),
            sample_rate: Some(44100),
            num_channels: None, // Missing
            import_session_id: Uuid::new_v4(),
        };

        let result = analyzer.extract(&ctx).await;
        assert!(result.is_err(), "Should fail when channels not specified");
    }

    // Note: Testing actual fingerprint generation requires:
    // 1. libchromaprint installed on system
    // 2. Real audio data (not silence)
    // These tests are covered in integration tests with test audio files
}

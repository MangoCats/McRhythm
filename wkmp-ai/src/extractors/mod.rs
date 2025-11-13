//! Tier 1 Source Extractors
//!
//! Implements 7 independent extractors for parallel execution.
//! Each extractor implements the `SourceExtractor` trait from `types` module.
//!
//! # Architecture
//! Per PLAN024 3-tier hybrid fusion:
//! - **Tier 1:** Extract raw data from multiple sources (THIS MODULE)
//! - **Tier 2:** Fuse extracted data with confidence weighting
//! - **Tier 3:** Validate fused results
//!
//! # Extractors
//! 1. **id3_extractor** - Extract ID3 metadata tags
//! 2. **chromaprint_analyzer** - Generate Chromaprint fingerprints
//! 3. **acoustid_client** - Query fingerprint → Recording MBID
//! 4. **musicbrainz_client** - Query MBID → Recording metadata
//! 5. **essentia_analyzer** - Extract musical features (optional, command execution)
//! 6. **audio_derived_extractor** - Algorithmic feature extraction (DSP)
//! 7. **id3_genre_mapper** - Map ID3 genre → musical flavor characteristics
//!
//! # Parallel Execution
//! All extractors run independently and report failures via per-passage error isolation.
//! Failed extractors do not block other extractors or the overall import process.
//!
//! # Implementation Status
//! - ✅ TASK-004: Base traits defined
//! - ⏳ TASK-005: ID3 Extractor
//! - ⏳ TASK-006: Chromaprint Analyzer
//! - ⏳ TASK-007: AcoustID Client
//! - ⏳ TASK-008: MusicBrainz Client
//! - ⏳ TASK-009: Essentia Analyzer
//! - ⏳ TASK-010: AudioDerived Extractor
//! - ⏳ TASK-011: ID3 Genre Mapper

// Module declarations (implemented extractors)
pub mod id3_extractor;           // TASK-005 ✅
pub mod chromaprint_analyzer;    // TASK-006 ✅
pub mod acoustid_client;         // TASK-007 ✅
pub mod musicbrainz_client;      // TASK-008 ✅
pub mod essentia_analyzer;       // TASK-009 ✅
pub mod audio_derived_extractor; // TASK-010 ✅
pub mod id3_genre_mapper;        // TASK-011 ✅

// All 7 Tier 1 extractors complete! ✅

use crate::types::{ExtractionResult, PassageContext, SourceExtractor};
#[cfg(test)]
use crate::types::ExtractionError;
use futures::future::join_all;
use std::sync::Arc;
use tracing::{debug, warn};

/// Parallel extractor executor
///
/// Runs all extractors concurrently and collects results.
/// Per-passage error isolation: individual extractor failures do not fail the entire batch.
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::extractors::ParallelExtractor;
/// use wkmp_ai::types::PassageContext;
///
/// let extractors: Vec<Arc<dyn SourceExtractor>> = vec![
///     Arc::new(ID3Extractor),
///     Arc::new(ChromaprintAnalyzer::new()),
///     // ... more extractors
/// ];
///
/// let executor = ParallelExtractor::new(extractors);
/// let results = executor.extract_all(&passage_ctx).await;
///
/// // Results contains successful extractions, failures logged but not propagated
/// for result in results {
///     println!("Extracted from {}: {:?}", result.extractor_name, result.data);
/// }
/// ```
pub struct ParallelExtractor {
    extractors: Vec<Arc<dyn SourceExtractor>>,
}

impl ParallelExtractor {
    /// Create new parallel extractor with given extractors
    pub fn new(extractors: Vec<Arc<dyn SourceExtractor>>) -> Self {
        Self { extractors }
    }

    /// Extract from all sources concurrently
    ///
    /// # Arguments
    /// * `ctx` - Passage context for extraction
    ///
    /// # Returns
    /// Vec of successful extraction results with extractor names.
    /// Failures are logged but not returned (per-passage error isolation).
    pub async fn extract_all(&self, ctx: &PassageContext) -> Vec<ExtractionOutput> {
        let futures = self.extractors.iter().map(|extractor| {
            let extractor = Arc::clone(extractor);
            let ctx = ctx.clone();
            async move {
                let name = extractor.name();
                match extractor.extract(&ctx).await {
                    Ok(result) => {
                        debug!(
                            extractor = name,
                            passage_id = %ctx.passage_id,
                            "Extraction successful"
                        );
                        Some(ExtractionOutput {
                            extractor_name: name.to_string(),
                            data: result,
                            confidence: extractor.base_confidence(),
                        })
                    }
                    Err(e) => {
                        warn!(
                            extractor = name,
                            passage_id = %ctx.passage_id,
                            error = %e,
                            "Extraction failed (per-passage error isolation)"
                        );
                        None
                    }
                }
            }
        });

        join_all(futures)
            .await
            .into_iter()
            .flatten()
            .collect()
    }

    /// Get extractor count
    pub fn count(&self) -> usize {
        self.extractors.len()
    }
}

/// Extraction output with extractor metadata
#[derive(Debug, Clone)]
pub struct ExtractionOutput {
    /// Name of extractor that produced this output
    pub extractor_name: String,
    /// Extracted data
    pub data: ExtractionResult,
    /// Base confidence of extractor
    pub confidence: f32,
}

// ============================================================================
// Mock Extractor for Testing
// ============================================================================

#[cfg(test)]
pub mod mock {
    use super::*;
    use async_trait::async_trait;

    /// Mock extractor for testing
    pub struct MockExtractor {
        pub name: &'static str,
        pub confidence: f32,
        pub should_fail: bool,
    }

    impl MockExtractor {
        pub fn new(name: &'static str, confidence: f32) -> Self {
            Self {
                name,
                confidence,
                should_fail: false,
            }
        }

        pub fn failing(name: &'static str) -> Self {
            Self {
                name,
                confidence: 0.0,
                should_fail: true,
            }
        }
    }

    #[async_trait]
    impl SourceExtractor for MockExtractor {
        fn name(&self) -> &'static str {
            self.name
        }

        fn base_confidence(&self) -> f32 {
            self.confidence
        }

        async fn extract(
            &self,
            _ctx: &PassageContext,
        ) -> Result<ExtractionResult, ExtractionError> {
            if self.should_fail {
                Err(ExtractionError::Internal("Mock failure".to_string()))
            } else {
                Ok(ExtractionResult::default())
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PassageContext;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_parallel_extractor_success() {
        let extractors: Vec<Arc<dyn SourceExtractor>> = vec![
            Arc::new(mock::MockExtractor::new("Extractor1", 0.8)),
            Arc::new(mock::MockExtractor::new("Extractor2", 0.9)),
            Arc::new(mock::MockExtractor::new("Extractor3", 0.7)),
        ];

        let executor = ParallelExtractor::new(extractors);

        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: None,
            sample_rate: Some(44100),
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        let results = executor.extract_all(&ctx).await;

        assert_eq!(results.len(), 3, "All 3 extractors should succeed");
        assert_eq!(
            results[0].extractor_name, "Extractor1",
            "Results should preserve order"
        );
    }

    #[tokio::test]
    async fn test_parallel_extractor_partial_failure() {
        let extractors: Vec<Arc<dyn SourceExtractor>> = vec![
            Arc::new(mock::MockExtractor::new("Success1", 0.8)),
            Arc::new(mock::MockExtractor::failing("Failure1")),
            Arc::new(mock::MockExtractor::new("Success2", 0.9)),
            Arc::new(mock::MockExtractor::failing("Failure2")),
        ];

        let executor = ParallelExtractor::new(extractors);

        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: None,
            sample_rate: Some(44100),
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        let results = executor.extract_all(&ctx).await;

        assert_eq!(
            results.len(),
            2,
            "Only 2 extractors should succeed (per-passage error isolation)"
        );
        assert_eq!(results[0].extractor_name, "Success1");
        assert_eq!(results[1].extractor_name, "Success2");
    }

    #[test]
    fn test_extractor_count() {
        let extractors: Vec<Arc<dyn SourceExtractor>> = vec![
            Arc::new(mock::MockExtractor::new("Test1", 0.8)),
            Arc::new(mock::MockExtractor::new("Test2", 0.9)),
        ];

        let executor = ParallelExtractor::new(extractors);
        assert_eq!(executor.count(), 2);
    }
}

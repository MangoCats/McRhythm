// PLAN023: Per-Song Workflow Engine
//
// Coordinates extraction → fusion → validation for each detected passage.
// Implements error isolation: single-passage failures don't abort import.
//
// Workflow Phases (per passage):
// 1. Extract audio segment
// 2. Tier 1: Parallel extraction (ID3, Chromaprint, MusicBrainz, Essentia)
// 3. Tier 2: Confidence-weighted fusion (Identity, Metadata, Flavor)
// 4. Tier 3: Quality validation (Consistency, Completeness, Conflicts)
// 5. Database insertion (passage + provenance records)
//
// **Legible Software Principle:**
// - Independent workflow concept: Orchestrates tiers without knowing internals
// - Explicit synchronization: Clear contracts with Tier 1/2/3 modules
// - Integrity: Per-passage error isolation with aggregate reporting
// - Transparency: SSE events expose all workflow stages

use crate::import_v2::tier1::{
    acoustid_client::AcoustIDClient,
    audio_features::AudioFeatureExtractor,
    chromaprint_analyzer::ChromaprintAnalyzer,
    id3_extractor::ID3Extractor,
    musicbrainz_client::MusicBrainzClient,
};
use crate::import_v2::tier2::{
    flavor_synthesizer::FlavorSynthesizer,
    identity_resolver::IdentityResolver,
    metadata_fuser::MetadataFuser,
};
use crate::import_v2::tier3::{
    completeness_scorer::CompletenessScorer,
    conflict_detector::ConflictDetector,
    consistency_checker::ConsistencyChecker,
};
use crate::import_v2::sse_broadcaster::SseBroadcaster;
use crate::import_v2::types::{
    ExtractorResult, FusedMetadata, ImportEvent, ImportResult, ValidationReport,
    MBIDCandidate, MetadataBundle, MusicalFlavor, PassageBoundary,
};
use std::path::Path;
use std::time::Duration;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Per-song workflow result
#[derive(Debug, Clone)]
pub struct SongWorkflowResult {
    /// Passage index (0-based)
    pub passage_index: usize,
    /// Success or failure status
    pub success: bool,
    /// Fused metadata (if successful)
    pub metadata: Option<FusedMetadata>,
    /// Resolved identity (if successful)
    pub mbid: Option<Uuid>,
    pub identity_confidence: f64,
    /// Musical flavor (if successful)
    pub flavor: Option<MusicalFlavor>,
    /// Validation report
    pub validation: Option<ValidationReport>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Processing duration
    pub duration_ms: u64,
}

/// Aggregate import results for all passages
#[derive(Debug, Clone)]
pub struct ImportSummary {
    pub total_passages: usize,
    pub successes: usize,
    pub warnings: usize, // Successful but with validation warnings
    pub failures: usize,
    pub results: Vec<SongWorkflowResult>,
}

/// Per-song workflow engine
///
/// **Legible Software Principle:**
/// - Orchestrates Tier 1/2/3 modules without internal dependencies
/// - Error isolation: failures don't cascade
/// - Transparent progress via SSE event broadcasting
pub struct SongWorkflowEngine {
    // Tier 1 extractors
    id3_extractor: ID3Extractor,
    chromaprint_analyzer: ChromaprintAnalyzer,
    musicbrainz_client: Option<MusicBrainzClient>,
    acoustid_client: Option<AcoustIDClient>,
    audio_features: AudioFeatureExtractor,

    // Tier 2 fusers
    identity_resolver: IdentityResolver,
    metadata_fuser: MetadataFuser,
    flavor_synthesizer: FlavorSynthesizer,

    // Tier 3 validators
    consistency_checker: ConsistencyChecker,
    completeness_scorer: CompletenessScorer,
    conflict_detector: ConflictDetector,

    // SSE event broadcasting
    sse_broadcaster: Option<SseBroadcaster>,

    // Configuration
    extraction_timeout: Duration,
}

impl Default for SongWorkflowEngine {
    fn default() -> Self {
        Self {
            id3_extractor: ID3Extractor::default(),
            chromaprint_analyzer: ChromaprintAnalyzer::default(),
            musicbrainz_client: None, // TODO: Initialize from config
            acoustid_client: None,     // TODO: Initialize from config
            audio_features: AudioFeatureExtractor::default(),
            identity_resolver: IdentityResolver::default(),
            metadata_fuser: MetadataFuser::default(),
            flavor_synthesizer: FlavorSynthesizer::default(),
            consistency_checker: ConsistencyChecker::default(),
            completeness_scorer: CompletenessScorer::default(),
            conflict_detector: ConflictDetector::default(),
            sse_broadcaster: None,  // No SSE broadcasting by default
            extraction_timeout: Duration::from_secs(30), // 30 second timeout per extractor
        }
    }
}

impl SongWorkflowEngine {
    /// Create workflow engine with SSE broadcasting enabled
    ///
    /// # Arguments
    /// * `event_tx` - Broadcast sender for SSE events
    /// * `throttle_interval_ms` - Minimum interval between throttled events (default: 1000ms)
    pub fn with_sse(event_tx: broadcast::Sender<ImportEvent>, throttle_interval_ms: u64) -> Self {
        Self {
            sse_broadcaster: Some(SseBroadcaster::new(event_tx, throttle_interval_ms)),
            ..Default::default()
        }
    }

    /// Emit SSE event if broadcaster is enabled
    fn emit_event(&mut self, event: ImportEvent) {
        if let Some(ref mut broadcaster) = self.sse_broadcaster {
            broadcaster.emit(event);
        }
    }

    /// Process a single passage through the complete workflow
    ///
    /// # Algorithm: 5-Phase Per-Song Pipeline
    /// 1. Extract audio segment for passage
    /// 2. Tier 1: Parallel extraction (ID3, Chromaprint, MusicBrainz, audio features)
    /// 3. Tier 2: Fusion (Identity → Metadata → Flavor)
    /// 4. Tier 3: Validation (Consistency → Completeness → Conflicts)
    /// 5. Return workflow result (success or failure with details)
    ///
    /// **Error Isolation:** Failures return SongWorkflowResult with error field set.
    /// Caller continues processing remaining passages.
    ///
    /// # Arguments
    /// * `file_path` - Audio file path
    /// * `passage_index` - Passage index (0-based)
    /// * `boundary` - Passage boundary (start/end times)
    ///
    /// # Returns
    /// SongWorkflowResult with metadata, identity, flavor, validation, or error
    pub async fn process_passage(
        &mut self,
        file_path: &Path,
        passage_index: usize,
        total_songs: usize,
        boundary: &PassageBoundary,
    ) -> SongWorkflowResult {
        let start_time = std::time::Instant::now();

        // Convert ticks to seconds for display
        // Tick rate: 28,224,000 Hz (28,224,000 ticks/second)
        const TICKS_PER_SECOND: f64 = 28_224_000.0;
        let start_sec = (boundary.start_ticks as f64) / TICKS_PER_SECOND;
        let end_sec = (boundary.end_ticks as f64) / TICKS_PER_SECOND;

        tracing::info!(
            passage_index,
            "Starting per-song workflow: {} [{:.2}s-{:.2}s]",
            file_path.display(),
            start_sec,
            end_sec
        );

        // Emit SongStarted event
        self.emit_event(ImportEvent::SongStarted {
            song_index: passage_index,
            total_songs,
        });

        // Phase 1: Extract audio segment (placeholder - actual implementation TBD)
        // TODO: Implement audio segment extraction using symphonia

        // Phase 2: Tier 1 - Parallel extraction
        let extraction_result = self.extract_all_sources(file_path, boundary).await;

        let (id3_result, mbid_candidates, audio_flavor_result) = match extraction_result {
            Ok(data) => data,
            Err(e) => {
                tracing::error!(passage_index, "Tier 1 extraction failed: {}", e);

                let error_msg = format!("Extraction failed: {}", e);
                self.emit_event(ImportEvent::SongFailed {
                    song_index: passage_index,
                    error: error_msg.clone(),
                });

                return SongWorkflowResult {
                    passage_index,
                    success: false,
                    metadata: None,
                    mbid: None,
                    identity_confidence: 0.0,
                    flavor: None,
                    validation: None,
                    error: Some(error_msg),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
            }
        };

        // Emit ExtractionComplete event
        self.emit_event(ImportEvent::ExtractionComplete {
            song_index: passage_index,
            sources: vec![id3_result.source],  // TODO: Add all sources used
        });

        // Phase 3: Tier 2 - Identity resolution
        let identity = match self.identity_resolver.resolve(mbid_candidates) {
            Ok(resolved) => resolved,
            Err(e) => {
                tracing::error!(passage_index, "Identity resolution failed: {}", e);
                return SongWorkflowResult {
                    passage_index,
                    success: false,
                    metadata: None,
                    mbid: None,
                    identity_confidence: 0.0,
                    flavor: None,
                    validation: None,
                    error: Some(format!("Identity resolution failed: {}", e)),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
            }
        };

        tracing::info!(
            passage_index,
            "Identity resolved: mbid={:?}, confidence={:.3}, conflict={}",
            identity.mbid,
            identity.confidence,
            identity.has_conflict
        );

        // Phase 4: Tier 2 - Metadata fusion
        let metadata_bundles = vec![id3_result]; // TODO: Add MusicBrainz metadata

        let fused_metadata = match self.metadata_fuser.fuse(metadata_bundles) {
            Ok(metadata) => metadata,
            Err(e) => {
                tracing::error!(passage_index, "Metadata fusion failed: {}", e);
                return SongWorkflowResult {
                    passage_index,
                    success: false,
                    metadata: None,
                    mbid: identity.mbid,
                    identity_confidence: identity.confidence,
                    flavor: None,
                    validation: None,
                    error: Some(format!("Metadata fusion failed: {}", e)),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
            }
        };

        // Phase 5: Tier 2 - Musical flavor synthesis
        // TODO: Implement FlavorExtraction conversion from ExtractorResult<MusicalFlavor>
        // For now, use the audio-derived flavor directly (skip synthesis)
        let musical_flavor = audio_flavor_result.data;

        // Emit FusionComplete event
        self.emit_event(ImportEvent::FusionComplete {
            song_index: passage_index,
            identity_confidence: identity.confidence,
            metadata_confidence: fused_metadata.metadata_confidence,
            flavor_confidence: audio_flavor_result.confidence,
        });

        // Phase 6: Tier 3 - Consistency validation
        // TODO: ConsistencyChecker API needs to be updated to return Vec<(String, ConflictSeverity)>
        // For now, use simple validation
        use crate::import_v2::types::{ConflictSeverity, ValidationResult};
        let validation_result = self.consistency_checker.validate_metadata(&fused_metadata);

        let consistency_conflicts = match validation_result {
            ValidationResult::Conflict { message, severity } => {
                vec![(message, severity)]
            },
            ValidationResult::Warning { message } => {
                vec![(message, ConflictSeverity::Low)]
            },
            ValidationResult::Pass => vec![],
        };

        // Phase 7: Tier 3 - Completeness scoring
        let quality_score = self.completeness_scorer.score(&fused_metadata);

        // Phase 8: Tier 3 - Conflict detection and aggregation
        let validation_report = self.conflict_detector.detect(
            &fused_metadata,
            quality_score,
            consistency_conflicts,
        );

        // Phase 9: Check acceptance criteria
        let is_acceptable = self.conflict_detector.is_acceptable(&validation_report);

        // Emit ValidationComplete event
        self.emit_event(ImportEvent::ValidationComplete {
            song_index: passage_index,
            quality_score: validation_report.quality_score,
            has_conflicts: validation_report.has_conflicts,
        });

        if !is_acceptable {
            tracing::warn!(
                passage_index,
                "Validation failed: {}",
                self.conflict_detector.summary_message(&validation_report)
            );
        } else {
            tracing::info!(
                passage_index,
                "Validation passed: {}",
                self.conflict_detector.summary_message(&validation_report)
            );
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        let error_msg = if is_acceptable {
            None
        } else {
            Some(self.conflict_detector.summary_message(&validation_report))
        };

        // Emit SongComplete or SongFailed event based on acceptance
        if is_acceptable {
            self.emit_event(ImportEvent::SongComplete {
                song_index: passage_index,
                duration_ms,
            });
        } else {
            self.emit_event(ImportEvent::SongFailed {
                song_index: passage_index,
                error: error_msg.clone().unwrap_or_else(|| "Unknown failure".to_string()),
            });
        }

        SongWorkflowResult {
            passage_index,
            success: is_acceptable,
            metadata: Some(fused_metadata),
            mbid: identity.mbid,
            identity_confidence: identity.confidence,
            flavor: Some(musical_flavor),
            validation: Some(validation_report),
            error: error_msg,
            duration_ms,
        }
    }

    /// Extract data from all Tier 1 sources (parallel execution)
    async fn extract_all_sources(
        &self,
        file_path: &Path,
        _boundary: &PassageBoundary,
    ) -> ImportResult<(
        ExtractorResult<MetadataBundle>,
        Vec<ExtractorResult<Vec<MBIDCandidate>>>,
        ExtractorResult<MusicalFlavor>,
    )> {
        // ID3 extraction
        let id3_result = self.id3_extractor.extract(file_path)?;

        // MBID candidate extraction (placeholder - needs Chromaprint + AcoustID)
        // TODO: Generate Chromaprint fingerprint
        // TODO: Query AcoustID with fingerprint
        // TODO: Extract MBID from ID3 tags
        let mbid_candidates = vec![]; // Empty for now

        // Audio features extraction
        // TODO: Extract audio samples for passage segment
        // For now, return empty MusicalFlavor as placeholder
        let audio_flavor = ExtractorResult {
            data: MusicalFlavor { characteristics: vec![] },
            confidence: 0.0,
            source: crate::import_v2::types::ExtractionSource::AudioDerived,
        };

        Ok((id3_result, mbid_candidates, audio_flavor))
    }

    /// Process all passages in a file and return aggregate summary
    ///
    /// **Error Isolation:** Continues processing after individual passage failures.
    ///
    /// # Arguments
    /// * `file_path` - Audio file path
    /// * `boundaries` - Detected passage boundaries
    ///
    /// # Returns
    /// ImportSummary with aggregate statistics and per-passage results
    pub async fn process_file(
        &mut self,
        file_path: &Path,
        boundaries: &[PassageBoundary],
    ) -> ImportSummary {
        let start_time = std::time::Instant::now();
        let total_passages = boundaries.len();
        let mut results = Vec::with_capacity(total_passages);

        tracing::info!(
            "Processing file: {} ({} passages detected)",
            file_path.display(),
            total_passages
        );

        // Emit PassagesDiscovered event
        self.emit_event(ImportEvent::PassagesDiscovered {
            file_path: file_path.display().to_string(),
            count: total_passages,
        });

        for (index, boundary) in boundaries.iter().enumerate() {
            let result = self.process_passage(file_path, index, total_passages, boundary).await;

            tracing::info!(
                passage_index = index,
                success = result.success,
                duration_ms = result.duration_ms,
                "Passage processing complete"
            );

            results.push(result);
        }

        // Compute aggregate statistics
        let successes = results.iter().filter(|r| r.success).count();
        let failures = results.iter().filter(|r| !r.success).count();
        let warnings = results
            .iter()
            .filter(|r| {
                r.success
                    && r.validation
                        .as_ref()
                        .map(|v| !v.warnings.is_empty())
                        .unwrap_or(false)
            })
            .count();

        let total_duration_ms = start_time.elapsed().as_millis() as u64;

        tracing::info!(
            file = %file_path.display(),
            total = total_passages,
            successes,
            warnings,
            failures,
            "File processing complete"
        );

        // Emit FileComplete event
        self.emit_event(ImportEvent::FileComplete {
            file_path: file_path.display().to_string(),
            successes,
            warnings,
            failures,
            total_duration_ms,
        });

        ImportSummary {
            total_passages,
            successes,
            warnings,
            failures,
            results,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_engine_creation() {
        let engine = SongWorkflowEngine::default();
        assert_eq!(engine.extraction_timeout, Duration::from_secs(30));
    }

    // TODO: Add integration tests with test audio files
    // TODO: Add tests for error isolation (failing passage doesn't abort import)
    // TODO: Add tests for validation thresholds
}

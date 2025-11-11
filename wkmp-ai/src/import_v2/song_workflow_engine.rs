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
    audio_loader::AudioLoader,
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

/// Per-song workflow result
#[derive(Debug, Clone)]
pub struct SongWorkflowResult {
    /// Passage index (0-based)
    pub passage_index: usize,
    /// Success or failure status
    pub success: bool,
    /// Fused metadata (if successful)
    pub metadata: Option<FusedMetadata>,
    /// Resolved identity (full structure with candidates)
    pub identity: Option<crate::import_v2::types::ResolvedIdentity>,
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
    audio_loader: AudioLoader,
    id3_extractor: ID3Extractor,
    chromaprint_analyzer: ChromaprintAnalyzer,
    musicbrainz_client: Option<MusicBrainzClient>,
    acoustid_client: Option<AcoustIDClient>,
    audio_features: AudioFeatureExtractor,

    // Tier 2 fusers
    identity_resolver: IdentityResolver,
    metadata_fuser: MetadataFuser,
    #[allow(dead_code)]
    flavor_synthesizer: FlavorSynthesizer,

    // Tier 3 validators
    consistency_checker: ConsistencyChecker,
    completeness_scorer: CompletenessScorer,
    conflict_detector: ConflictDetector,

    // SSE event broadcasting
    sse_broadcaster: Option<SseBroadcaster>,

    // Configuration
    #[allow(dead_code)]
    extraction_timeout: Duration,
}

impl Default for SongWorkflowEngine {
    fn default() -> Self {
        Self {
            audio_loader: AudioLoader::default(),
            id3_extractor: ID3Extractor::default(),
            chromaprint_analyzer: ChromaprintAnalyzer::default(),
            musicbrainz_client: None, // Call init_clients() after construction
            acoustid_client: None,     // Call init_clients() after construction
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

    /// Initialize API clients from configuration
    ///
    /// **Configuration Priority:** Database → ENV → TOML
    ///
    /// # Arguments
    /// * `db` - Database connection pool
    /// * `toml_config` - TOML configuration
    ///
    /// # Effects
    /// - Sets `acoustid_client` if AcoustID API key is configured
    /// - Sets `musicbrainz_client` (always, uses standard user-agent)
    ///
    /// # Errors
    /// - Returns error if AcoustID API key is configured but invalid
    /// - Logs warning if AcoustID API key is not configured (optional)
    ///
    /// # Traceability
    /// [APIK-RES-010] - Multi-tier configuration resolution
    pub async fn init_clients(
        &mut self,
        db: &sqlx::Pool<sqlx::Sqlite>,
        toml_config: &wkmp_common::config::TomlConfig,
    ) -> wkmp_common::Result<()> {
        use tracing::{info, warn};

        // Initialize AcoustID client (optional - may not be configured)
        match AcoustIDClient::from_config(db, toml_config).await {
            Ok(client) => {
                info!("AcoustID client initialized successfully");
                self.acoustid_client = Some(client);
            }
            Err(e) => {
                warn!("AcoustID client not initialized: {}. Fingerprinting will be unavailable.", e);
                self.acoustid_client = None;
            }
        }

        // Initialize MusicBrainz client (always available - uses standard user-agent)
        match MusicBrainzClient::from_config(db, toml_config).await {
            Ok(client) => {
                info!("MusicBrainz client initialized successfully");
                self.musicbrainz_client = Some(client);
            }
            Err(e) => {
                warn!("MusicBrainz client initialization failed: {}", e);
                self.musicbrainz_client = None;
            }
        }

        Ok(())
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
    /// * `session_id` - Import session ID for event correlation (REQ-TD-006)
    /// * `file_path` - Audio file path
    /// * `passage_index` - Passage index (0-based)
    /// * `boundary` - Passage boundary (start/end times)
    ///
    /// # Returns
    /// SongWorkflowResult with metadata, identity, flavor, validation, or error
    pub async fn process_passage(
        &mut self,
        session_id: uuid::Uuid,
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
            session_id,
            song_index: passage_index,
            total_songs,
        });

        // Phase 1: Audio segment extraction already implemented via AudioLoader::load_segment()
        // REQ-TD-002: Audio segment extraction is functional (see extract_all_sources)

        // Phase 2: Tier 1 - Parallel extraction
        let extraction_result = self.extract_all_sources(file_path, boundary).await;

        let (id3_result, mbid_candidates, audio_flavor_result) = match extraction_result {
            Ok(data) => data,
            Err(e) => {
                tracing::error!(passage_index, "Tier 1 extraction failed: {}", e);

                let error_msg = format!("Extraction failed: {}", e);
                self.emit_event(ImportEvent::SongFailed {
                    session_id,
                    song_index: passage_index,
                    error: error_msg.clone(),
                });

                return SongWorkflowResult {
                    passage_index,
                    success: false,
                    metadata: None,
                    identity: None,
                    flavor: None,
                    validation: None,
                    error: Some(error_msg),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
            }
        };

        // Emit ExtractionComplete event
        // Collect all sources used (ID3 + AcoustID if candidates exist + audio features)
        let mut sources = vec![id3_result.source, audio_flavor_result.source];

        // Add AcoustID source if candidates were found
        if !mbid_candidates.is_empty() {
            // AcoustID source is recorded in the first candidate list
            if let Some(first_candidate_list) = mbid_candidates.first() {
                sources.push(first_candidate_list.source);
            }
        }

        self.emit_event(ImportEvent::ExtractionComplete {
            session_id,
            song_index: passage_index,
            sources,
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
                    identity: None,
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
        let mut metadata_bundles = vec![id3_result];

        // Query MusicBrainz API if client is available and MBID was resolved
        if let (Some(ref mb_client), Some(mbid)) = (&self.musicbrainz_client, identity.mbid) {
            match mb_client.lookup(mbid).await {
                Ok(mb_result) => {
                    tracing::debug!(
                        "MusicBrainz returned metadata: title={} artist={} album={}",
                        mb_result.data.title.first().map(|f| f.value.as_str()).unwrap_or("N/A"),
                        mb_result.data.artist.first().map(|f| f.value.as_str()).unwrap_or("N/A"),
                        mb_result.data.album.first().map(|f| f.value.as_str()).unwrap_or("N/A")
                    );
                    metadata_bundles.push(mb_result);
                }
                Err(e) => {
                    // Non-fatal: continue with other sources
                    tracing::warn!(
                        "MusicBrainz lookup failed for mbid={} (non-fatal, continuing with other sources): {}",
                        mbid,
                        e
                    );
                }
            }
        }

        let fused_metadata = match self.metadata_fuser.fuse(metadata_bundles) {
            Ok(metadata) => metadata,
            Err(e) => {
                tracing::error!(passage_index, "Metadata fusion failed: {}", e);
                return SongWorkflowResult {
                    passage_index,
                    success: false,
                    metadata: None,
                    identity: Some(identity),
                    flavor: None,
                    validation: None,
                    error: Some(format!("Metadata fusion failed: {}", e)),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
            }
        };

        // Phase 5: Tier 2 - Musical flavor synthesis
        // REQ-TD-007: Combine multiple flavor sources for robust analysis
        tracing::debug!("Phase 5: Synthesizing musical flavor from all sources");

        use crate::import_v2::types::FlavorExtraction;
        let mut flavor_sources = Vec::new();

        // Add audio-derived flavor (always available from Phase 3)
        flavor_sources.push(FlavorExtraction {
            flavor: audio_flavor_result.data.clone(),
            confidence: audio_flavor_result.confidence,
            source: audio_flavor_result.source,
        });

        // Future: Add AcousticBrainz flavor here when implemented
        // if let Some(acousticbrainz_flavor) = acousticbrainz_flavor_result {
        //     flavor_sources.push(FlavorExtraction {
        //         flavor: acousticbrainz_flavor.data,
        //         confidence: acousticbrainz_flavor.confidence,
        //         source: acousticbrainz_flavor.source,
        //     });
        // }

        // Synthesize combined flavor
        let synthesized = match self.flavor_synthesizer.synthesize(flavor_sources) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Flavor synthesis failed: {}", e);
                // Return error result
                return SongWorkflowResult {
                    passage_index,
                    success: false,
                    metadata: None,
                    identity: Some(identity),
                    flavor: None,
                    validation: None,
                    error: Some(format!("Flavor synthesis failed: {}", e)),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
            }
        };

        let musical_flavor = synthesized.flavor;
        let flavor_confidence = synthesized.flavor_confidence;

        tracing::info!(
            "Flavor synthesis complete: confidence={:.2}, completeness={:.2}, sources={}",
            flavor_confidence,
            synthesized.flavor_completeness,
            synthesized.sources_used.len()
        );

        // Emit FusionComplete event
        self.emit_event(ImportEvent::FusionComplete {
            session_id,
            song_index: passage_index,
            identity_confidence: identity.confidence,
            metadata_confidence: fused_metadata.metadata_confidence,
            flavor_confidence, // REQ-TD-007: Use synthesized confidence
        });

        // Phase 6: Tier 3 - Consistency validation
        let (_consistency_warnings, consistency_conflicts) =
            self.consistency_checker.validate_metadata_detailed(&fused_metadata);

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
            session_id,
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
                session_id,
                song_index: passage_index,
                duration_ms,
            });
        } else {
            self.emit_event(ImportEvent::SongFailed {
                session_id,
                song_index: passage_index,
                error: error_msg.clone().unwrap_or_else(|| "Unknown failure".to_string()),
            });
        }

        SongWorkflowResult {
            passage_index,
            success: is_acceptable,
            metadata: Some(fused_metadata),
            identity: Some(identity),
            flavor: Some(musical_flavor),
            validation: Some(validation_report),
            error: error_msg,
            duration_ms,
        }
    }

    /// Extract data from all Tier 1 sources (parallel execution)
    ///
    /// **[P1-5]** Fully integrated audio processing pipeline:
    /// 1. Load audio segment for passage boundary (AudioLoader with resampling)
    /// 2. Generate Chromaprint fingerprint (ChromaprintAnalyzer)
    /// 3. Extract ID3 metadata (ID3Extractor)
    /// 4. Extract audio-derived musical flavor (AudioFeatureExtractor)
    ///
    /// # Algorithm
    /// - Convert passage boundary ticks to sample offsets
    /// - Load audio segment (stereo, resampled to 44.1kHz)
    /// - Convert stereo → mono for Chromaprint (mix channels)
    /// - Generate acoustic fingerprint
    /// - Extract audio features for flavor
    ///
    /// # Returns
    /// Tuple of (ID3 metadata, MBID candidates, musical flavor)
    async fn extract_all_sources(
        &self,
        file_path: &Path,
        boundary: &PassageBoundary,
    ) -> ImportResult<(
        ExtractorResult<MetadataBundle>,
        Vec<ExtractorResult<Vec<MBIDCandidate>>>,
        ExtractorResult<MusicalFlavor>,
    )> {
        // **Phase 1: Load audio segment**
        // AudioLoader expects ticks (i64), not seconds
        // Tick rate: 28,224,000 Hz (28,224,000 ticks/second)
        const TICKS_PER_SECOND: f64 = 28_224_000.0;
        let start_sec = (boundary.start_ticks as f64) / TICKS_PER_SECOND;
        let end_sec = (boundary.end_ticks as f64) / TICKS_PER_SECOND;

        tracing::debug!(
            "Loading audio segment: {} to {} ticks ({:.2}s to {:.2}s, {:.2}s duration)",
            boundary.start_ticks,
            boundary.end_ticks,
            start_sec,
            end_sec,
            end_sec - start_sec
        );

        // Load audio segment (stereo, resampled to 44.1kHz)
        let audio_segment = self.audio_loader.load_segment(
            file_path,
            boundary.start_ticks,
            boundary.end_ticks,
        ).map_err(|e| crate::import_v2::types::ImportError::AudioProcessingFailed(
            format!("Failed to load audio segment: {}", e)
        ))?;

        let duration_ms = ((end_sec - start_sec) * 1000.0) as u32;

        tracing::debug!(
            "Loaded {} stereo samples at {} Hz ({} frames, {:.2}s)",
            audio_segment.samples.len(),
            audio_segment.sample_rate,
            audio_segment.samples.len() / 2,
            audio_segment.samples.len() as f64 / (audio_segment.sample_rate as f64 * 2.0)
        );

        // **Phase 2: Convert stereo to mono for Chromaprint**
        // Chromaprint requires mono audio, so mix L+R channels
        let mono_samples = Self::stereo_to_mono(&audio_segment.samples);

        tracing::debug!(
            "Converted to mono: {} samples",
            mono_samples.len()
        );

        // **Phase 3: Generate Chromaprint fingerprint**
        let fingerprint_result = self.chromaprint_analyzer.analyze(
            &mono_samples,
            duration_ms,
        )?;

        tracing::debug!(
            "Generated Chromaprint fingerprint: {} bytes, confidence: {:.3}",
            fingerprint_result.data.len(),
            fingerprint_result.confidence
        );

        // **Phase 4: Extract ID3 metadata**
        let id3_result = self.id3_extractor.extract(file_path)?;

        tracing::debug!(
            "Extracted ID3 metadata: {} fields",
            id3_result.data.title.len()
                + id3_result.data.artist.len()
                + id3_result.data.album.len()
        );

        // **Phase 5: Extract audio-derived musical flavor**
        // Use stereo samples for feature extraction
        // Note: AudioFeatureExtractor only takes samples, not sample rate
        let audio_flavor = self.audio_features.extract(&audio_segment.samples)?;

        tracing::debug!(
            "Extracted audio flavor: {} characteristics, confidence: {:.3}",
            audio_flavor.data.characteristics.len(),
            audio_flavor.confidence
        );

        // **Phase 6: Assemble MBID candidates**
        let mut mbid_candidates = vec![];

        // Query AcoustID API if client is available
        if let Some(ref acoustid_client) = self.acoustid_client {
            // Convert duration from ms to seconds for AcoustID API
            let duration_secs = duration_ms / 1000;

            match acoustid_client.lookup(&fingerprint_result.data, duration_secs).await {
                Ok(acoustid_result) => {
                    tracing::debug!(
                        "AcoustID returned {} candidates",
                        acoustid_result.data.len()
                    );
                    mbid_candidates.push(acoustid_result);
                }
                Err(e) => {
                    // Non-fatal: continue with other sources
                    tracing::warn!(
                        "AcoustID lookup failed (non-fatal, continuing with other sources): {}",
                        e
                    );
                }
            }
        }

        // Extract MBID from ID3 UFID frame (if present)
        // Note: Currently limited by lofty crate - UFID frames not exposed in public API
        // When lofty adds UFID support, this will automatically enable ID3 MBID extraction
        if let Ok(Some(id3_mbid_result)) = self.id3_extractor.extract_mbid(file_path) {
            tracing::debug!(
                "ID3 UFID returned {} MBID candidates",
                id3_mbid_result.data.len()
            );
            mbid_candidates.push(id3_mbid_result);
        }

        Ok((id3_result, mbid_candidates, audio_flavor))
    }

    /// Convert stereo samples to mono by averaging L+R channels
    ///
    /// **Algorithm:** For each stereo frame (L, R), output mono sample = (L + R) / 2
    ///
    /// # Arguments
    /// * `stereo_samples` - Interleaved L/R samples [L0, R0, L1, R1, ...]
    ///
    /// # Returns
    /// Mono samples [M0, M1, M2, ...] where Mi = (Li + Ri) / 2
    fn stereo_to_mono(stereo_samples: &[f32]) -> Vec<f32> {
        let num_frames = stereo_samples.len() / 2;
        let mut mono = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            let left = stereo_samples[i * 2];
            let right = stereo_samples[i * 2 + 1];
            mono.push((left + right) / 2.0);
        }

        mono
    }

    /// Process all passages in a file and return aggregate summary
    ///
    /// **Error Isolation:** Continues processing after individual passage failures.
    ///
    /// # Arguments
    /// * `session_id` - Import session ID for event correlation (REQ-TD-006)
    /// * `file_path` - Audio file path
    /// * `boundaries` - Detected passage boundaries
    ///
    /// # Returns
    /// ImportSummary with aggregate statistics and per-passage results
    pub async fn process_file(
        &mut self,
        session_id: uuid::Uuid,
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
            session_id,
            file_path: file_path.display().to_string(),
            count: total_passages,
        });

        for (index, boundary) in boundaries.iter().enumerate() {
            let result = self.process_passage(session_id, file_path, index, total_passages, boundary).await;

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
            session_id,
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

//! Pipeline Orchestrator
//!
//! Orchestrates the complete 3-tier hybrid fusion pipeline for passage processing.
//! Coordinates extraction (Tier 1), fusion (Tier 2), and validation (Tier 3).
//!
//! # Architecture
//! - **Phase 0**: Boundary detection (identifies passages in audio file)
//! - **Phase 1**: Extraction (Tier 1 - source extractors)
//! - **Phase 2**: Fusion (Tier 2 - data fusers)
//! - **Phase 3**: Validation (Tier 3 - quality validators)
//!
//! # Error Handling
//! - Per-passage error isolation: extractor failures don't fail entire file
//! - Graceful degradation: continues with available data if some sources fail
//! - Detailed error reporting via WorkflowEvent::Error
//!
//! # Example
//! ```rust,ignore
//! let pipeline = Pipeline::new(config);
//! let passages = pipeline.process_file(Path::new("audio.flac")).await?;
//! ```

use super::{FusedPassage, PassageBoundary, ProcessedPassage, WorkflowEvent};
use crate::extractors::acoustid_client::AcoustIDClient;
use crate::extractors::audio_derived_extractor::AudioDerivedExtractor;
use crate::extractors::chromaprint_analyzer::ChromaprintAnalyzer;
use crate::extractors::essentia_analyzer::EssentiaAnalyzer;
use crate::extractors::id3_extractor::ID3Extractor;
use crate::extractors::id3_genre_mapper::ID3GenreMapper;
use crate::extractors::musicbrainz_client::MusicBrainzClient;
use crate::fusion::{FlavorSynthesizer, IdentityResolver, MetadataFuser};
use crate::types::{
    ExtractionResult, Fusion, PassageContext, SourceExtractor, Validation, ValidationResult,
};
use crate::validators::{CompletenessScorer, ConsistencyValidator, QualityScorer};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Pipeline configuration
#[derive(Clone)]
pub struct PipelineConfig {
    /// AcoustID API key for fingerprint lookups
    pub acoustid_api_key: Option<String>,
    /// **[AIA-SEC-030]** Skip AcoustID for this session (set when user chooses to skip)
    pub acoustid_skip: bool,
    /// Enable MusicBrainz lookups (requires network access)
    pub enable_musicbrainz: bool,
    /// Enable Essentia audio analysis (requires Essentia library)
    pub enable_essentia: bool,
    /// Enable audio-derived feature extraction
    pub enable_audio_derived: bool,
    /// Minimum passage quality score to accept (0.0-1.0)
    pub min_quality_threshold: f32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            acoustid_api_key: None,
            acoustid_skip: false,
            enable_musicbrainz: true,
            enable_essentia: true,
            enable_audio_derived: true,
            min_quality_threshold: 0.5,
        }
    }
}

/// Pipeline orchestrator for 3-tier hybrid fusion
pub struct Pipeline {
    config: PipelineConfig,
    event_tx: Option<mpsc::Sender<WorkflowEvent>>,
}

impl Pipeline {
    /// Create new pipeline with configuration
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            config,
            event_tx: None,
        }
    }

    /// Create pipeline with event channel for progress reporting
    pub fn with_events(config: PipelineConfig, event_tx: mpsc::Sender<WorkflowEvent>) -> Self {
        Self {
            config,
            event_tx: Some(event_tx),
        }
    }

    /// Process entire audio file (detect boundaries + process all passages)
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    ///
    /// # Returns
    /// * Vec of successfully processed passages (failures are logged but don't stop processing)
    pub async fn process_file(&self, file_path: &Path) -> Result<Vec<ProcessedPassage>> {
        info!("Pipeline processing file: {:?}", file_path);

        // Emit file started event
        self.emit_event(WorkflowEvent::FileStarted {
            file_path: file_path.to_string_lossy().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })
        .await;

        // Phase 0: Detect passage boundaries with audio caching
        // **[AIA-PERF-046]** Decode once, reuse audio for all passage extractors
        let file_audio = super::boundary_detector::detect_boundaries_with_audio(file_path)
            .await
            .context("Failed to detect passage boundaries")?;

        info!("Detected {} passages", file_audio.boundaries.len());

        // Emit boundary events
        for (i, boundary) in file_audio.boundaries.iter().enumerate() {
            self.emit_event(WorkflowEvent::BoundaryDetected {
                passage_index: i,
                start_time: boundary.start_time,
                end_time: boundary.end_time,
                confidence: boundary.confidence,
            })
            .await;
        }

        // Process each passage sequentially with cached audio
        let mut processed_passages = Vec::new();
        let total_passages = file_audio.boundaries.len();

        for (i, boundary) in file_audio.boundaries.iter().enumerate() {
            self.emit_event(WorkflowEvent::PassageStarted {
                passage_index: i,
                total_passages,
            })
            .await;

            match self.process_passage_with_audio(file_path, boundary, i, &file_audio).await {
                Ok(passage) => {
                    self.emit_event(WorkflowEvent::PassageCompleted {
                        passage_index: i,
                        quality_score: passage.validation.score as f64,
                        validation_status: format!("{:?}", passage.validation.status),
                    })
                    .await;

                    processed_passages.push(passage);
                }
                Err(e) => {
                    error!("Failed to process passage {}: {:?}", i, e);
                    self.emit_event(WorkflowEvent::Error {
                        passage_index: Some(i),
                        message: format!("Passage processing failed: {:?}", e),
                    })
                    .await;
                    // Continue with next passage (per-passage error isolation)
                }
            }
        }

        info!(
            "File processing complete: {} of {} passages successful",
            processed_passages.len(),
            total_passages
        );

        self.emit_event(WorkflowEvent::FileCompleted {
            file_path: file_path.to_string_lossy().to_string(),
            passages_processed: processed_passages.len(),
            timestamp: chrono::Utc::now().timestamp(),
        })
        .await;

        Ok(processed_passages)
    }

    /// Process single passage with cached audio
    ///
    /// **[AIA-PERF-046]** Uses pre-decoded audio to eliminate re-decoding for extractors
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    /// * `boundary` - Passage boundary (start/end times)
    /// * `passage_index` - Index for progress reporting
    /// * `file_audio` - Cached decoded audio from boundary detection
    ///
    /// # Returns
    /// * Processed passage with fusion and validation results
    async fn process_passage_with_audio(
        &self,
        file_path: &Path,
        boundary: &PassageBoundary,
        passage_index: usize,
        file_audio: &super::FileAudioData,
    ) -> Result<ProcessedPassage> {
        debug!(
            "Processing passage {} ({} - {} ticks) with cached audio",
            passage_index, boundary.start_time, boundary.end_time
        );

        // Extract passage-specific audio samples from cached file audio
        let passage_samples = super::boundary_detector::extract_passage_samples(file_audio, boundary);

        debug!(
            "Extracted {} samples for passage {} ({:.1} MB)",
            passage_samples.len(),
            passage_index,
            (passage_samples.len() * std::mem::size_of::<f32>()) as f64 / 1_048_576.0
        );

        // **[PLAN024 Option 3]** Two-Pass Pipeline for MusicBrainz Integration
        //
        // Pass 1: Parallel extraction (all extractors, MusicBrainz returns NotAvailable)
        // Fusion: Bayesian confidence selection of Recording MBID
        // Pass 2: MusicBrainz extraction with fused MBID (if available)
        // Re-fusion: Merge MusicBrainz metadata into final result

        // PASS 1: Extraction (Tier 1) with audio samples
        debug!("Pass 1: Running all extractors with cached audio");
        let mut extraction_results = self.extract_with_audio(
            file_path,
            boundary,
            passage_index,
            &passage_samples,
            file_audio.sample_rate,
            file_audio.num_channels,
        ).await?;

        // PASS 1 FUSION: Fuse to obtain Recording MBID
        debug!("Pass 1 Fusion: Fusing extraction results to obtain MBID");
        let pass1_fusion = self.fuse(&extraction_results, passage_index).await?;

        // PASS 2: MusicBrainz with fused MBID (if available and enabled)
        if self.config.enable_musicbrainz {
            if let Some(ref mbid_cv) = pass1_fusion.metadata.recording_mbid {
                let mbid = &mbid_cv.value;

                info!(
                    passage_index = passage_index,
                    mbid = %mbid,
                    confidence = mbid_cv.confidence,
                    source = %mbid_cv.source,
                    "Pass 2: Running MusicBrainz with fused MBID"
                );

                // Create passage context for MusicBrainz
                let ctx = PassageContext {
                    passage_id: Uuid::new_v4(),
                    file_id: Uuid::new_v4(),
                    file_path: PathBuf::from(file_path),
                    start_time_ticks: boundary.start_time,
                    end_time_ticks: boundary.end_time,
                    audio_samples: None,
                    sample_rate: None,
                    num_channels: None,
                    import_session_id: Uuid::new_v4(),
                };

                self.emit_extraction_progress(passage_index, "MusicBrainz-Pass2", "running")
                    .await;

                match MusicBrainzClient::new().extract_with_mbid(mbid, &ctx).await {
                    Ok(musicbrainz_result) => {
                        info!(
                            passage_index = passage_index,
                            "Pass 2: MusicBrainz extraction successful"
                        );

                        // Add MusicBrainz result to extraction results
                        extraction_results.push(musicbrainz_result);

                        self.emit_extraction_progress(passage_index, "MusicBrainz-Pass2", "completed")
                            .await;
                    }
                    Err(e) => {
                        warn!(
                            passage_index = passage_index,
                            error = ?e,
                            "Pass 2: MusicBrainz extraction failed (non-fatal)"
                        );
                        self.emit_extraction_progress(passage_index, "MusicBrainz-Pass2", "failed")
                            .await;
                    }
                }
            } else {
                debug!(
                    passage_index = passage_index,
                    "Pass 2: Skipping MusicBrainz (no MBID from Pass 1 fusion)"
                );
            }
        }

        // PASS 2 FUSION: Re-fuse with MusicBrainz data (if Pass 2 ran)
        debug!("Pass 2 Fusion: Re-fusing with all extraction results");
        let final_fusion = self.fuse(&extraction_results, passage_index).await?;

        // Phase 3: Validation (Tier 3)
        let validation_result = self.validate(&final_fusion, passage_index).await?;

        Ok(ProcessedPassage {
            boundary: boundary.clone(),
            extractions: extraction_results,
            fusion: final_fusion,
            validation: validation_result,
        })
    }

    /// Process single passage through 3-tier pipeline (deprecated)
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    /// * `boundary` - Passage boundary (start/end times)
    /// * `passage_index` - Index for progress reporting
    ///
    /// # Returns
    /// * Processed passage with fusion and validation results
    #[deprecated(note = "Use process_passage_with_audio() to avoid re-decoding audio")]
    async fn process_passage(
        &self,
        file_path: &Path,
        boundary: &PassageBoundary,
        passage_index: usize,
    ) -> Result<ProcessedPassage> {
        debug!(
            "Processing passage {} ({} - {} ticks)",
            passage_index, boundary.start_time, boundary.end_time
        );

        // **[PLAN024 Option 3]** Two-Pass Pipeline for MusicBrainz Integration
        //
        // Pass 1: Parallel extraction (all extractors, MusicBrainz returns NotAvailable)
        // Fusion: Bayesian confidence selection of Recording MBID
        // Pass 2: MusicBrainz extraction with fused MBID (if available)
        // Re-fusion: Merge MusicBrainz metadata into final result

        // PASS 1: Extraction (Tier 1)
        debug!("Pass 1: Running all extractors");
        let mut extraction_results = self.extract(file_path, boundary, passage_index).await?;

        // PASS 1 FUSION: Fuse to obtain Recording MBID
        debug!("Pass 1 Fusion: Fusing extraction results to obtain MBID");
        let pass1_fusion = self.fuse(&extraction_results, passage_index).await?;

        // PASS 2: MusicBrainz with fused MBID (if available and enabled)
        if self.config.enable_musicbrainz {
            if let Some(ref mbid_cv) = pass1_fusion.metadata.recording_mbid {
                let mbid = &mbid_cv.value;

                info!(
                    passage_index = passage_index,
                    mbid = %mbid,
                    confidence = mbid_cv.confidence,
                    source = %mbid_cv.source,
                    "Pass 2: Running MusicBrainz with fused MBID"
                );

                // Create passage context for MusicBrainz
                let ctx = PassageContext {
                    passage_id: Uuid::new_v4(),
                    file_id: Uuid::new_v4(),
                    file_path: PathBuf::from(file_path),
                    start_time_ticks: boundary.start_time,
                    end_time_ticks: boundary.end_time,
                    audio_samples: None,
                    sample_rate: None,
                    num_channels: None,
                    import_session_id: Uuid::new_v4(),
                };

                self.emit_extraction_progress(passage_index, "MusicBrainz-Pass2", "running")
                    .await;

                match MusicBrainzClient::new().extract_with_mbid(mbid, &ctx).await {
                    Ok(musicbrainz_result) => {
                        info!(
                            passage_index = passage_index,
                            "Pass 2: MusicBrainz extraction successful"
                        );

                        // Add MusicBrainz result to extraction results
                        extraction_results.push(musicbrainz_result);

                        self.emit_extraction_progress(passage_index, "MusicBrainz-Pass2", "completed")
                            .await;
                    }
                    Err(e) => {
                        warn!(
                            passage_index = passage_index,
                            error = ?e,
                            "Pass 2: MusicBrainz extraction failed (non-fatal)"
                        );
                        self.emit_extraction_progress(passage_index, "MusicBrainz-Pass2", "failed")
                            .await;
                    }
                }
            } else {
                debug!(
                    passage_index = passage_index,
                    "Pass 2: Skipping MusicBrainz (no MBID from Pass 1 fusion)"
                );
            }
        }

        // PASS 2 FUSION: Re-fuse with MusicBrainz data (if Pass 2 ran)
        debug!("Pass 2 Fusion: Re-fusing with all extraction results");
        let final_fusion = self.fuse(&extraction_results, passage_index).await?;

        // Phase 3: Validation (Tier 3)
        let validation_result = self.validate(&final_fusion, passage_index).await?;

        Ok(ProcessedPassage {
            boundary: boundary.clone(),
            extractions: extraction_results,
            fusion: final_fusion,
            validation: validation_result,
        })
    }

    /// Phase 1: Run all enabled extractors with cached audio
    ///
    /// **[AIA-PERF-046]** Provides pre-decoded audio to Chromaprint/AudioDerived extractors
    async fn extract_with_audio(
        &self,
        file_path: &Path,
        boundary: &PassageBoundary,
        passage_index: usize,
        audio_samples: &[f32],
        sample_rate: u32,
        num_channels: u8,
    ) -> Result<Vec<ExtractionResult>> {
        debug!("Phase 1: Extraction for passage {} with cached audio", passage_index);

        // Create passage context with cached audio samples
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(), // In production, this would come from database
            file_path: PathBuf::from(file_path),
            start_time_ticks: boundary.start_time,
            end_time_ticks: boundary.end_time,
            audio_samples: Some(audio_samples.to_vec()),
            sample_rate: Some(sample_rate),
            num_channels: Some(num_channels),
            import_session_id: Uuid::new_v4(),
        };

        self.extract_common(&ctx, passage_index).await
    }

    /// Phase 1: Run all enabled extractors (deprecated)
    #[deprecated(note = "Use extract_with_audio() to avoid re-decoding audio")]
    async fn extract(
        &self,
        file_path: &Path,
        boundary: &PassageBoundary,
        passage_index: usize,
    ) -> Result<Vec<ExtractionResult>> {
        debug!("Phase 1: Extraction for passage {}", passage_index);

        // Create passage context for extractors
        // Note: audio_samples are None - extractors load audio themselves if needed
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(), // In production, this would come from database
            file_path: PathBuf::from(file_path),
            start_time_ticks: boundary.start_time,
            end_time_ticks: boundary.end_time,
            audio_samples: None, // Extractors load audio themselves if needed
            sample_rate: None,
            num_channels: None,
            import_session_id: Uuid::new_v4(),
        };

        self.extract_common(&ctx, passage_index).await
    }

    /// Common extractor logic (called by both extract methods)
    async fn extract_common(
        &self,
        ctx: &PassageContext,
        passage_index: usize,
    ) -> Result<Vec<ExtractionResult>> {

        let mut results = Vec::new();

        // Extractor 1: ID3 tags
        self.emit_extraction_progress(passage_index, "ID3", "running")
            .await;
        match ID3Extractor::new().extract(&ctx).await {
            Ok(extraction) => {
                results.push(extraction);
                self.emit_extraction_progress(passage_index, "ID3", "completed")
                    .await;
            }
            Err(e) => {
                warn!("ID3 extraction failed: {}", e);
                self.emit_extraction_progress(passage_index, "ID3", "failed")
                    .await;
            }
        }

        // Extractor 2: Chromaprint fingerprint
        self.emit_extraction_progress(passage_index, "Chromaprint", "running")
            .await;
        match ChromaprintAnalyzer::new().extract(&ctx).await {
            Ok(extraction) => {
                results.push(extraction);
                self.emit_extraction_progress(passage_index, "Chromaprint", "completed")
                    .await;
            }
            Err(e) => {
                warn!("Chromaprint extraction failed: {}", e);
                self.emit_extraction_progress(passage_index, "Chromaprint", "failed")
                    .await;
            }
        }

        // Extractor 3: AcoustID (if API key provided and not skipped)
        // **[AIA-SEC-030]** Skip if user has chosen to skip AcoustID for this session
        if let Some(ref api_key) = self.config.acoustid_api_key {
            if !self.config.acoustid_skip {
                self.emit_extraction_progress(passage_index, "AcoustID", "running")
                    .await;
                match AcoustIDClient::new(api_key.clone()).extract(&ctx).await {
                    Ok(extraction) => {
                        results.push(extraction);
                        self.emit_extraction_progress(passage_index, "AcoustID", "completed")
                            .await;
                    }
                    Err(e) => {
                        // **[AIA-SEC-030]** Check if error is invalid API key
                        if crate::extractors::acoustid_client::is_invalid_api_key_error(&e) {
                            // Emit event to prompt user for API key
                            self.emit_event(WorkflowEvent::AcoustIDKeyInvalid {
                                error_message: e.to_string(),
                            })
                            .await;
                            warn!(
                                "AcoustID API key invalid: {}. User will be prompted for valid key or to skip.",
                                e
                            );
                        } else {
                            warn!("AcoustID extraction failed: {}", e);
                        }
                        self.emit_extraction_progress(passage_index, "AcoustID", "failed")
                            .await;
                    }
                }
            } else {
                // **[AIA-SEC-030]** User has chosen to skip AcoustID
                debug!(
                    passage_index = passage_index,
                    "AcoustID extraction skipped (user chose to skip AcoustID functionality)"
                );
            }
        } else {
            // No API key configured - skip AcoustID
            debug!(
                passage_index = passage_index,
                "AcoustID extraction skipped (no API key configured)"
            );
        }

        // Extractor 4: MusicBrainz
        // **[PLAN024 Option 3]** MusicBrainz is now handled in Pass 2 (after fusion)
        // This extractor is intentionally skipped in Pass 1 because it requires
        // Recording MBID which comes from fusion of ID3/AcoustID results.
        // See process_passage() for Pass 2 implementation.

        // Extractor 5: Essentia (if enabled)
        if self.config.enable_essentia {
            self.emit_extraction_progress(passage_index, "Essentia", "running")
                .await;
            match EssentiaAnalyzer::new().extract(&ctx).await {
                Ok(extraction) => {
                    results.push(extraction);
                    self.emit_extraction_progress(passage_index, "Essentia", "completed")
                        .await;
                }
                Err(e) => {
                    warn!("Essentia extraction failed: {}", e);
                    self.emit_extraction_progress(passage_index, "Essentia", "failed")
                        .await;
                }
            }
        }

        // Extractor 6: Audio-derived features (if enabled)
        if self.config.enable_audio_derived {
            self.emit_extraction_progress(passage_index, "AudioDerived", "running")
                .await;
            match AudioDerivedExtractor::new().extract(&ctx).await {
                Ok(extraction) => {
                    results.push(extraction);
                    self.emit_extraction_progress(passage_index, "AudioDerived", "completed")
                        .await;
                }
                Err(e) => {
                    warn!("Audio-derived extraction failed: {}", e);
                    self.emit_extraction_progress(passage_index, "AudioDerived", "failed")
                        .await;
                }
            }
        }

        // Extractor 7: ID3 Genre Mapper (derives flavor from ID3 genre tags)
        self.emit_extraction_progress(passage_index, "ID3GenreMapper", "running")
            .await;
        match ID3GenreMapper::new().extract(&ctx).await {
            Ok(extraction) => {
                results.push(extraction);
                self.emit_extraction_progress(passage_index, "ID3GenreMapper", "completed")
                    .await;
            }
            Err(e) => {
                warn!("ID3 genre mapping failed: {}", e);
                self.emit_extraction_progress(passage_index, "ID3GenreMapper", "failed")
                    .await;
            }
        }

        if results.is_empty() {
            anyhow::bail!("All extractors failed - no data available for fusion");
        }

        let total_extractors = 3 // Always run: ID3, Chromaprint, ID3GenreMapper
            + if self.config.acoustid_api_key.is_some() { 1 } else { 0 }
            // MusicBrainz excluded - runs in Pass 2 (see process_passage)
            + if self.config.enable_essentia { 1 } else { 0 }
            + if self.config.enable_audio_derived { 1 } else { 0 };

        info!(
            "Pass 1 extraction complete: {} of {} extractors succeeded (MusicBrainz deferred to Pass 2)",
            results.len(),
            total_extractors
        );

        Ok(results)
    }

    /// Phase 2: Fuse extraction results
    async fn fuse(
        &self,
        extraction_results: &[ExtractionResult],
        passage_index: usize,
    ) -> Result<FusedPassage> {
        debug!("Phase 2: Fusion for passage {}", passage_index);

        self.emit_event(WorkflowEvent::FusionStarted { passage_index })
            .await;

        // Collect extraction data by type from all results
        let mut identity_extractions = Vec::new();
        let mut metadata_extractions = Vec::new();
        let mut flavor_extractions = Vec::new();

        for result in extraction_results {
            if let Some(ref identity) = result.identity {
                identity_extractions.push(identity.clone());
            }
            if let Some(ref metadata) = result.metadata {
                metadata_extractions.push(metadata.clone());
            }
            if let Some(ref flavor) = result.musical_flavor {
                flavor_extractions.push(flavor.clone());
            }
        }

        // Fuser 1: Identity Resolver
        let identity_fuser = IdentityResolver::new();
        let identity_result = identity_fuser
            .fuse(identity_extractions)
            .await
            .context("Identity fusion failed")?;

        // Fuser 2: Metadata Fuser
        let metadata_fuser = MetadataFuser::new();
        let metadata_result = metadata_fuser
            .fuse(metadata_extractions)
            .await
            .context("Metadata fusion failed")?;

        // Fuser 3: Flavor Synthesizer
        let flavor_fuser = FlavorSynthesizer::new();
        let flavor_result = flavor_fuser
            .fuse(flavor_extractions)
            .await
            .context("Flavor fusion failed")?;

        info!("Fusion complete for passage {}", passage_index);

        Ok(FusedPassage {
            identity: identity_result.output,
            metadata: metadata_result.output,
            flavor: flavor_result.output,
        })
    }

    /// Phase 3: Validate fused data
    async fn validate(
        &self,
        fused_passage: &FusedPassage,
        passage_index: usize,
    ) -> Result<ValidationResult> {
        debug!("Phase 3: Validation for passage {}", passage_index);

        self.emit_event(WorkflowEvent::ValidationStarted { passage_index })
            .await;

        // Validator 1: Consistency Validator
        let consistency_validator = ConsistencyValidator::new();
        let consistency_result = consistency_validator
            .validate(fused_passage)
            .await
            .context("Consistency validation failed")?;

        debug!(
            "Consistency validation: {:?} (score: {:.2})",
            consistency_result.status, consistency_result.score
        );

        // Validator 2: Completeness Scorer
        let completeness_scorer = CompletenessScorer::new();
        let completeness_result = completeness_scorer
            .validate(fused_passage)
            .await
            .context("Completeness scoring failed")?;

        debug!(
            "Completeness scoring: {:?} (score: {:.2})",
            completeness_result.status, completeness_result.score
        );

        // Validator 3: Quality Scorer (final comprehensive assessment)
        let quality_scorer = QualityScorer::new();
        let quality_result = quality_scorer
            .validate(fused_passage)
            .await
            .context("Quality scoring failed")?;

        info!(
            "Validation complete: {:?} (quality: {:.1}%)",
            quality_result.status,
            quality_result.score * 100.0
        );

        // Return the quality result (most comprehensive)
        // Note: consistency and completeness insights are captured in quality scorer
        Ok(quality_result)
    }

    /// Emit workflow event if channel configured
    async fn emit_event(&self, event: WorkflowEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(event).await;
        }
    }

    /// Emit extraction progress event
    async fn emit_extraction_progress(
        &self,
        passage_index: usize,
        extractor: &str,
        status: &str,
    ) {
        self.emit_event(WorkflowEvent::ExtractionProgress {
            passage_index,
            extractor: extractor.to_string(),
            status: status.to_string(),
        })
        .await;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PipelineConfig::default();
        assert!(config.acoustid_api_key.is_none());
        assert!(config.enable_musicbrainz);
        assert!(config.enable_essentia);
        assert!(config.enable_audio_derived);
        assert_eq!(config.min_quality_threshold, 0.5);
    }

    #[test]
    fn test_pipeline_creation() {
        let config = PipelineConfig::default();
        let pipeline = Pipeline::new(config);
        assert!(pipeline.event_tx.is_none());
    }

    #[test]
    fn test_pipeline_with_events() {
        let config = PipelineConfig::default();
        let (tx, _rx) = mpsc::channel(10);
        let pipeline = Pipeline::with_events(config, tx);
        assert!(pipeline.event_tx.is_some());
    }
}

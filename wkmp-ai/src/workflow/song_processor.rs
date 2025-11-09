// Song Processor - Per-Song Workflow Orchestration
//
// PLAN023: REQ-AI-010 - Complete per-song sequential processing pipeline
// Phases: 0 (boundary) → 1-6 (extract, fuse, validate) → Store

use super::{PassageBoundary, ProcessedPassage, WorkflowEvent};
use crate::fusion::extractors::{
    acoustid_client::AcoustIdClient, audio_derived_extractor::AudioDerivedExtractor, id3_extractor::Id3Extractor, musicbrainz_client::MusicBrainzClient,
    Extractor,
};
use crate::fusion::fusers;
use crate::fusion::ExtractionResult;
use anyhow::Result;
use sqlx::SqlitePool;
use std::path::Path;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Song processor configuration
pub struct SongProcessorConfig {
    pub acoustid_api_key: String,
    pub enable_musicbrainz: bool,
    pub enable_audio_derived: bool,
    pub enable_database_storage: bool,
}

/// Song processor orchestrates complete workflow
pub struct SongProcessor {
    config: SongProcessorConfig,
    event_tx: mpsc::Sender<WorkflowEvent>,
    db: Option<SqlitePool>,
}

impl SongProcessor {
    /// Create new song processor without database
    pub fn new(config: SongProcessorConfig, event_tx: mpsc::Sender<WorkflowEvent>) -> Self {
        Self {
            config,
            event_tx,
            db: None,
        }
    }

    /// Create new song processor with database
    pub fn with_database(
        config: SongProcessorConfig,
        event_tx: mpsc::Sender<WorkflowEvent>,
        db: SqlitePool,
    ) -> Self {
        Self {
            config,
            event_tx,
            db: Some(db),
        }
    }

    /// Process entire audio file (all passages)
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    ///
    /// # Returns
    /// * Vec of processed passages
    pub async fn process_file(&self, file_path: &Path) -> Result<Vec<ProcessedPassage>> {
        info!("Processing file: {:?}", file_path);

        // Emit file started event
        self.emit_event(WorkflowEvent::FileStarted {
            file_path: file_path.to_string_lossy().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })
        .await;

        // Phase 0: Detect passage boundaries
        let boundaries = super::boundary_detector::detect_boundaries(file_path).await?;
        info!("Detected {} passages", boundaries.len());

        // Emit boundary events (SPEC017: ticks in event, converted to seconds in event_bridge)
        for (i, boundary) in boundaries.iter().enumerate() {
            self.emit_event(WorkflowEvent::BoundaryDetected {
                passage_index: i,
                start_time: boundary.start_time,
                end_time: boundary.end_time,
                confidence: boundary.confidence,
            })
            .await;
        }

        // Process each passage sequentially
        let mut processed_passages = Vec::new();
        let import_session_id = uuid::Uuid::new_v4().to_string();

        for (i, boundary) in boundaries.into_iter().enumerate() {
            match self.process_passage(file_path, &boundary, i, processed_passages.len() + 1).await {
                Ok(passage) => {
                    // Store to database if enabled
                    if self.config.enable_database_storage {
                        if let Some(db) = &self.db {
                            match super::storage::store_passage(
                                db,
                                &file_path.to_string_lossy(),
                                &passage,
                                &import_session_id,
                            )
                            .await
                            {
                                Ok(passage_id) => {
                                    debug!("Stored passage {} to database: {}", i, passage_id);
                                }
                                Err(e) => {
                                    warn!("Failed to store passage {} to database: {}", i, e);
                                    self.emit_event(WorkflowEvent::Error {
                                        passage_index: Some(i),
                                        message: format!("Database storage failed: {}", e),
                                    })
                                    .await;
                                }
                            }
                        } else {
                            warn!("Database storage enabled but no database connection available");
                        }
                    }

                    processed_passages.push(passage);
                }
                Err(e) => {
                    error!("Failed to process passage {}: {}", i, e);
                    self.emit_event(WorkflowEvent::Error {
                        passage_index: Some(i),
                        message: format!("Passage processing failed: {}", e),
                    })
                    .await;
                }
            }
        }

        // Emit file completed event
        self.emit_event(WorkflowEvent::FileCompleted {
            file_path: file_path.to_string_lossy().to_string(),
            passages_processed: processed_passages.len(),
            timestamp: chrono::Utc::now().timestamp(),
        })
        .await;

        Ok(processed_passages)
    }

    /// Process single passage through complete pipeline
    async fn process_passage(
        &self,
        file_path: &Path,
        boundary: &PassageBoundary,
        passage_index: usize,
        total_passages: usize,
    ) -> Result<ProcessedPassage> {
        // Convert ticks to seconds for logging
        let start_seconds = boundary.start_time as f64 / super::TICK_RATE as f64;
        let end_seconds = boundary.end_time as f64 / super::TICK_RATE as f64;

        info!(
            "Processing passage {} of {} ({:.1}s - {:.1}s)",
            passage_index + 1,
            total_passages,
            start_seconds,
            end_seconds
        );

        self.emit_event(WorkflowEvent::PassageStarted {
            passage_index,
            total_passages,
        })
        .await;

        // Phase 1-3: Extraction (parallel extractors) - pass ticks, convert in run_extractors
        let extractions = self
            .run_extractors(file_path, boundary.start_time, boundary.end_time, passage_index)
            .await?;

        debug!("Completed {} extractions", extractions.len());

        // Phase 4: Fusion
        self.emit_event(WorkflowEvent::FusionStarted { passage_index })
            .await;

        let fusion = fusers::fuse_extractions(extractions.clone()).await?;

        info!(
            "Fusion complete: MBID={:?}, title={:?}, {} flavor characteristics",
            fusion.identity.recording_mbid,
            fusion.metadata.title,
            fusion.flavor.characteristics.len()
        );

        // Phase 5-6: Validation
        self.emit_event(WorkflowEvent::ValidationStarted { passage_index })
            .await;

        let validation = self.run_validation(&extractions, &fusion)?;

        info!(
            "Validation complete: status={:?}, quality={:.1}%",
            validation.status,
            validation.quality_score
        );

        self.emit_event(WorkflowEvent::PassageCompleted {
            passage_index,
            quality_score: validation.quality_score,
            validation_status: format!("{:?}", validation.status),
        })
        .await;

        Ok(ProcessedPassage {
            boundary: boundary.clone(),
            extractions,
            fusion,
            validation,
        })
    }

    /// Run all extractors in parallel
    async fn run_extractors(
        &self,
        file_path: &Path,
        start_ticks: i64,
        end_ticks: i64,
        passage_index: usize,
    ) -> Result<Vec<ExtractionResult>> {
        // Convert SPEC017 ticks to seconds for extractors that need time ranges
        let start_seconds = start_ticks as f64 / super::TICK_RATE as f64;
        let end_seconds = end_ticks as f64 / super::TICK_RATE as f64;

        let mut extractions = Vec::new();

        // 1. ID3 Extractor (always enabled)
        self.emit_event(WorkflowEvent::ExtractionProgress {
            passage_index,
            extractor: "ID3".to_string(),
            status: "running".to_string(),
        })
        .await;

        let id3_extractor = Id3Extractor::new();
        match id3_extractor.extract(file_path, start_seconds, end_seconds).await {
            Ok(result) => {
                debug!("ID3 extraction: confidence={:.2}", result.confidence);
                extractions.push(result);
            }
            Err(e) => {
                warn!("ID3 extraction failed: {}", e);
            }
        }

        // 2. Chromaprint + AcoustID (if API key provided)
        if !self.config.acoustid_api_key.is_empty() {
            self.emit_event(WorkflowEvent::ExtractionProgress {
                passage_index,
                extractor: "AcoustID".to_string(),
                status: "running".to_string(),
            })
            .await;

            let acoustid_client = AcoustIdClient::new(self.config.acoustid_api_key.clone());
            match acoustid_client.extract(file_path, start_seconds, end_seconds).await {
                Ok(result) => {
                    debug!("AcoustID extraction: confidence={:.2}", result.confidence);
                    extractions.push(result);
                }
                Err(e) => {
                    warn!("AcoustID extraction failed: {}", e);
                }
            }
        }

        // 3. MusicBrainz (if enabled and we have an MBID)
        if self.config.enable_musicbrainz {
            // Check if any extractor found an MBID
            let mbid = extractions
                .iter()
                .find_map(|e| e.identity.as_ref().map(|id| id.recording_mbid.clone()));

            if let Some(mbid) = mbid {
                self.emit_event(WorkflowEvent::ExtractionProgress {
                    passage_index,
                    extractor: "MusicBrainz".to_string(),
                    status: "running".to_string(),
                })
                .await;

                let mb_client = MusicBrainzClient::new();
                match mb_client.fetch_by_mbid(&mbid).await {
                    Ok(result) => {
                        debug!("MusicBrainz extraction: confidence={:.2}", result.confidence);
                        extractions.push(result);
                    }
                    Err(e) => {
                        warn!("MusicBrainz extraction failed: {}", e);
                    }
                }
            }
        }

        // 4. Audio-Derived Features (if enabled)
        if self.config.enable_audio_derived {
            self.emit_event(WorkflowEvent::ExtractionProgress {
                passage_index,
                extractor: "AudioDerived".to_string(),
                status: "running".to_string(),
            })
            .await;

            let audio_extractor = AudioDerivedExtractor::new();
            match audio_extractor.extract(file_path, start_seconds, end_seconds).await {
                Ok(result) => {
                    debug!("Audio-derived extraction: confidence={:.2}", result.confidence);
                    extractions.push(result);
                }
                Err(e) => {
                    warn!("Audio-derived extraction failed: {}", e);
                }
            }
        }

        Ok(extractions)
    }

    /// Run validation checks using Tier 3 validation pipeline
    fn run_validation(
        &self,
        extractions: &[ExtractionResult],
        fusion: &crate::fusion::FusionResult,
    ) -> Result<crate::fusion::ValidationResult> {
        // Use the async validation pipeline from fusion::validators
        // Note: run_validation is sync, so we use block_in_place for the async call
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                crate::fusion::validators::validate_fusion(fusion, extractions).await
            })
        })
    }

    /// Emit workflow event
    async fn emit_event(&self, event: WorkflowEvent) {
        if let Err(e) = self.event_tx.send(event).await {
            error!("Failed to emit workflow event: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_song_processor_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let config = SongProcessorConfig {
            acoustid_api_key: String::new(),
            enable_musicbrainz: false,
            enable_audio_derived: false,
            enable_database_storage: false,
        };

        let processor = SongProcessor::new(config, tx);
        assert!(processor.config.acoustid_api_key.is_empty());
        assert!(processor.db.is_none());
    }
}

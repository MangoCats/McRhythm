// PLAN024 Sprint 2: Session-level import workflow orchestration
//
// Concept: Orchestrate complete import session using import_v2 SongWorkflowEngine
// Replaces: services::WorkflowOrchestrator (legacy)
//
// This orchestrator manages the full session lifecycle:
// - Session state transitions
// - Progress tracking and SSE broadcasting
// - File discovery and boundary detection
// - Per-passage workflow execution via SongWorkflowEngine
// - Error handling and cancellation

use crate::db::files::{calculate_file_hash, save_file, AudioFile};
use crate::import_v2::db_repository::ImportRepository;
use crate::import_v2::song_workflow_engine::SongWorkflowEngine;
use crate::import_v2::types::ImportEvent;
use crate::models::{ImportSession, ImportState};
use crate::services::FileScanner;
use anyhow::Result;
use chrono::{DateTime, Utc};
use lofty::file::TaggedFileExt;
use lofty::prelude::AudioFile as LoftyAudioFile;
use lofty::probe::Probe;
use sqlx::{Pool, Sqlite};
use std::path::Path;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// Session-level import orchestrator using import_v2 architecture
///
/// **PLAN024 Sprint 2**: Replaces legacy WorkflowOrchestrator with import_v2
pub struct SessionOrchestrator {
    /// Database connection pool
    db: Pool<Sqlite>,
    /// Broadcast channel for ImportEvent SSE
    event_tx: broadcast::Sender<ImportEvent>,
    /// Song workflow engine for per-passage processing
    engine: SongWorkflowEngine,
    /// Database repository for ProcessedPassage persistence
    repository: ImportRepository,
}

impl SessionOrchestrator {
    /// Create new session orchestrator
    ///
    /// # Arguments
    /// * `db` - Database connection pool
    /// * `event_tx` - Broadcast sender for SSE events
    /// * `throttle_interval_ms` - SSE throttle interval (default: 1000ms)
    pub fn new(
        db: Pool<Sqlite>,
        event_tx: broadcast::Sender<ImportEvent>,
        throttle_interval_ms: u64,
    ) -> Self {
        let engine = SongWorkflowEngine::with_sse(event_tx.clone(), throttle_interval_ms);
        let repository = ImportRepository::new(db.clone());

        Self {
            db,
            event_tx,
            engine,
            repository,
        }
    }

    /// Initialize API clients from configuration
    ///
    /// Must be called after construction before executing workflow
    ///
    /// # Arguments
    /// * `toml_config` - TOML configuration (fallback for API keys)
    pub async fn init_clients(
        &mut self,
        toml_config: &wkmp_common::config::TomlConfig,
    ) -> wkmp_common::Result<()> {
        self.engine.init_clients(&self.db, toml_config).await
    }

    /// Execute complete import workflow for a session
    ///
    /// # Arguments
    /// * `session` - Import session from database
    /// * `cancel_token` - Cancellation token for aborting workflow
    ///
    /// # Returns
    /// Updated session with final state (Completed, Failed, or Cancelled)
    ///
    /// # Phases
    /// 1. Scanning - Discover audio files
    /// 2. Boundary Detection - Identify passage boundaries
    /// 3. Per-Passage Processing - Run SongWorkflowEngine for each passage
    /// 4. Completion - Finalize session
    pub async fn execute_import(
        &mut self,
        mut session: ImportSession,
        cancel_token: CancellationToken,
    ) -> Result<ImportSession> {
        tracing::info!(
            session_id = %session.session_id,
            root_folder = %session.root_folder,
            "Starting import workflow (import_v2)"
        );

        // Emit session started event
        let _ = self.event_tx.send(ImportEvent::SessionStarted {
            session_id: session.session_id,
            root_folder: session.root_folder.clone(),
        });

        // Phase 1: Scanning - Discover audio files
        session.transition_to(ImportState::Scanning);
        session.update_progress(0, 0, "Scanning for audio files...".to_string());
        crate::db::sessions::save_session(&self.db, &session).await?;

        tracing::info!(session_id = %session.session_id, "Phase 1: SCANNING");

        let file_scanner = FileScanner::new();
        let scan_result = file_scanner.scan_with_stats(Path::new(&session.root_folder))?;

        tracing::info!(
            session_id = %session.session_id,
            files_found = scan_result.files.len(),
            total_size_mb = scan_result.total_size / 1_000_000,
            "File scan completed"
        );

        // Check for cancellation
        if cancel_token.is_cancelled() {
            tracing::info!(
                session_id = %session.session_id,
                "Import cancelled during scanning phase"
            );
            session.transition_to(ImportState::Cancelled);
            session.update_progress(0, scan_result.files.len(), "Import cancelled by user".to_string());
            crate::db::sessions::save_session(&self.db, &session).await?;
            return Ok(session);
        }

        // If no files found, mark as completed
        if scan_result.files.is_empty() {
            tracing::warn!(
                session_id = %session.session_id,
                "No audio files found in directory"
            );
            session.transition_to(ImportState::Completed);
            session.update_progress(0, 0, "No audio files found".to_string());
            crate::db::sessions::save_session(&self.db, &session).await?;
            return Ok(session);
        }

        // Phase 2: Boundary Detection - Identify passage boundaries
        session.transition_to(ImportState::Segmenting);
        session.update_progress(0, scan_result.files.len(), "Detecting passage boundaries...".to_string());
        crate::db::sessions::save_session(&self.db, &session).await?;

        tracing::info!(session_id = %session.session_id, "Phase 2: BOUNDARY DETECTION");

        use crate::import_v2::tier2::boundary_fuser::{BoundaryFuser, FusedBoundary};
        use crate::import_v2::types::{BoundaryDetectionMethod, ExtractionSource, ExtractorResult, PassageBoundary};

        let boundary_fuser = BoundaryFuser::default();
        let mut total_passages = 0;
        // Track (file_id, file_path, boundaries) for Phase 3 processing
        let mut passage_boundaries: Vec<(uuid::Uuid, std::path::PathBuf, Vec<FusedBoundary>)> = Vec::new();

        for (file_idx, file_path) in scan_result.files.iter().enumerate() {
            // Generate UUID for this file
            let file_id = uuid::Uuid::new_v4();

            // **CRITICAL FIX**: Save file metadata to database BEFORE processing passages
            // This ensures the foreign key parent exists when passages are saved
            if let Err(e) = self.save_file_metadata(&file_id, file_path).await {
                tracing::warn!(
                    session_id = %session.session_id,
                    file_id = %file_id,
                    file = %file_path.display(),
                    error = %e,
                    "Failed to save file metadata (non-fatal, continuing)"
                );
            }

            // Check for cancellation
            if cancel_token.is_cancelled() {
                tracing::info!(
                    session_id = %session.session_id,
                    "Import cancelled during boundary detection"
                );
                session.transition_to(ImportState::Cancelled);
                session.update_progress(file_idx, scan_result.files.len(), "Import cancelled by user".to_string());
                crate::db::sessions::save_session(&self.db, &session).await?;
                return Ok(session);
            }

            tracing::debug!(
                session_id = %session.session_id,
                file = %file_path.display(),
                progress = format!("{}/{}", file_idx + 1, scan_result.files.len()),
                "Detecting boundaries"
            );

            session.update_progress(
                file_idx,
                scan_result.files.len(),
                format!("Detecting boundaries in file {}/{}", file_idx + 1, scan_result.files.len()),
            );

            // Extract duration from audio file metadata
            // Use ID3Extractor to get accurate duration from audio properties
            use crate::import_v2::tier1::id3_extractor::ID3Extractor;
            let id3_extractor = ID3Extractor::default();

            let duration_ms = match id3_extractor.extract(&file_path) {
                Ok(result) => {
                    // Extract duration from MetadataBundle
                    result.data.duration_ms.first()
                        .map(|field| field.value as f64)
                        .unwrap_or(180_000.0) // Fallback to 3 minutes if duration not found
                }
                Err(e) => {
                    tracing::warn!(
                        session_id = %session.session_id,
                        file = %file_path.display(),
                        error = %e,
                        "Failed to extract metadata, using default duration"
                    );
                    180_000.0 // Fallback to 3 minutes (in milliseconds)
                }
            };

            let duration_secs = duration_ms / 1000.0;

            // Strategy 1: Silence-based detection using actual SilenceDetector
            // REQ-TD-001: Replace stub with functional boundary detection
            tracing::debug!(
                session_id = %session.session_id,
                file = %file_path.display(),
                "Loading audio for silence detection"
            );

            // Load full audio file for boundary detection
            use crate::import_v2::tier1::audio_loader::AudioLoader;
            let audio_loader = AudioLoader::default();

            let file_boundaries = match audio_loader.load_full(&file_path) {
                Ok(audio_segment) => {
                    // Convert stereo to mono by averaging channels (SilenceDetector expects mono)
                    let mono_samples: Vec<f32> = audio_segment
                        .samples
                        .chunks_exact(2)
                        .map(|stereo| (stereo[0] + stereo[1]) / 2.0)
                        .collect();

                    tracing::debug!(
                        session_id = %session.session_id,
                        file = %file_path.display(),
                        samples = mono_samples.len(),
                        sample_rate = audio_segment.sample_rate,
                        "Audio loaded, running silence detection"
                    );

                    // Get silence detection configuration from database (with defaults)
                    let silence_threshold_db = sqlx::query_scalar::<_, String>(
                        "SELECT value FROM settings WHERE key = 'import.boundary_detection.silence_threshold_db'"
                    )
                    .fetch_optional(&self.db)
                    .await?
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(-60.0);

                    let min_silence_duration_sec = sqlx::query_scalar::<_, String>(
                        "SELECT value FROM settings WHERE key = 'import.boundary_detection.min_silence_duration_sec'"
                    )
                    .fetch_optional(&self.db)
                    .await?
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.5);

                    tracing::debug!(
                        session_id = %session.session_id,
                        threshold_db = silence_threshold_db,
                        min_duration_sec = min_silence_duration_sec,
                        "Silence detection configuration"
                    );

                    // Initialize silence detector with configuration
                    use crate::services::silence_detector::SilenceDetector;
                    let silence_detector = SilenceDetector::new()
                        .with_threshold_db(silence_threshold_db as f32)
                        .and_then(|d| d.with_min_duration(min_silence_duration_sec as f32))
                        .map_err(|e| anyhow::anyhow!("Failed to create silence detector: {}", e))?;

                    // Detect silence regions
                    let silence_regions = silence_detector
                        .detect(&mono_samples, audio_segment.sample_rate as usize)
                        .map_err(|e| anyhow::anyhow!("Silence detection failed: {}", e))?;

                    tracing::debug!(
                        session_id = %session.session_id,
                        file = %file_path.display(),
                        silence_regions = silence_regions.len(),
                        "Silence detection complete"
                    );

                    // Convert silence regions to passage boundaries
                    if silence_regions.is_empty() {
                        // No silence detected - entire file is one passage
                        vec![PassageBoundary {
                            start_ticks: 0,
                            end_ticks: (duration_secs * 28_224_000.0) as i64,
                            confidence: 0.8,
                            detection_method: BoundaryDetectionMethod::SilenceDetection,
                        }]
                    } else {
                        // Create passages between silence regions
                        let mut boundaries = Vec::new();
                        let mut last_end_sec = 0.0f32;

                        for silence in &silence_regions {
                            // Passage before this silence
                            if silence.start_seconds > last_end_sec {
                                let start_ticks = (last_end_sec as f64 * 28_224_000.0) as i64;
                                let end_ticks = (silence.start_seconds as f64 * 28_224_000.0) as i64;

                                boundaries.push(PassageBoundary {
                                    start_ticks,
                                    end_ticks,
                                    confidence: 0.8,
                                    detection_method: BoundaryDetectionMethod::SilenceDetection,
                                });
                            }
                            last_end_sec = silence.end_seconds;
                        }

                        // Final passage after last silence
                        if (last_end_sec as f64) < duration_secs {
                            let start_ticks = (last_end_sec as f64 * 28_224_000.0) as i64;
                            let end_ticks = (duration_secs * 28_224_000.0) as i64;

                            boundaries.push(PassageBoundary {
                                start_ticks,
                                end_ticks,
                                confidence: 0.8,
                                detection_method: BoundaryDetectionMethod::SilenceDetection,
                            });
                        }

                        boundaries
                    }
                }
                Err(e) => {
                    // Audio loading failed - fallback to single passage spanning duration
                    tracing::warn!(
                        session_id = %session.session_id,
                        file = %file_path.display(),
                        error = %e,
                        "Failed to load audio for boundary detection, using fallback (single passage)"
                    );

                    vec![PassageBoundary {
                        start_ticks: 0,
                        end_ticks: (duration_secs * 28_224_000.0) as i64,
                        confidence: 0.5, // Lower confidence for fallback
                        detection_method: BoundaryDetectionMethod::SilenceDetection,
                    }]
                }
            };

            tracing::info!(
                session_id = %session.session_id,
                file = %file_path.display(),
                passages = file_boundaries.len(),
                "Boundary detection complete"
            );

            // Wrap in ExtractorResult
            let extractor_result = ExtractorResult {
                source: ExtractionSource::AudioDerived,
                confidence: 0.8, // Confidence for silence-based detection
                data: file_boundaries.clone(),
            };

            // Fuse boundaries (currently just one method, but infrastructure ready for multiple)
            let fused_boundaries = boundary_fuser.fuse(vec![extractor_result])?;

            tracing::debug!(
                session_id = %session.session_id,
                file = %file_path.display(),
                passages_found = fused_boundaries.len(),
                "Boundary detection complete for file"
            );

            // Emit PassagesDiscovered event
            let _ = self.event_tx.send(ImportEvent::PassagesDiscovered {
                session_id: session.session_id,
                file_path: file_path.display().to_string(),
                count: fused_boundaries.len(),
            });

            total_passages += fused_boundaries.len();

            // Store boundaries for Phase 3 processing (with file UUID)
            passage_boundaries.push((file_id, file_path.clone(), fused_boundaries));
        }

        tracing::info!(
            session_id = %session.session_id,
            files_processed = scan_result.files.len(),
            passages_discovered = total_passages,
            "Boundary detection complete"
        );

        // Phase 3: Per-Passage Processing - Run workflow for each passage
        // Using Extracting state as this phase does metadata extraction + fingerprinting + analysis
        session.transition_to(ImportState::Extracting);
        session.update_progress(0, total_passages, "Processing passages...".to_string());
        crate::db::sessions::save_session(&self.db, &session).await?;

        tracing::info!(
            session_id = %session.session_id,
            total_passages = total_passages,
            "Phase 3: PER-PASSAGE PROCESSING"
        );

        let mut passage_idx = 0;
        let mut successes = 0;
        let mut warnings = 0;
        let mut failures = 0;

        for (file_id, file_path, boundaries) in passage_boundaries {
            for boundary in boundaries {
                // Check for cancellation
                if cancel_token.is_cancelled() {
                    tracing::info!(
                        session_id = %session.session_id,
                        "Import cancelled during passage processing"
                    );
                    session.transition_to(ImportState::Cancelled);
                    session.update_progress(passage_idx, total_passages, "Import cancelled by user".to_string());
                    crate::db::sessions::save_session(&self.db, &session).await?;
                    return Ok(session);
                }

                tracing::debug!(
                    session_id = %session.session_id,
                    file = %file_path.display(),
                    passage_idx = passage_idx,
                    "Processing passage {}/{}", passage_idx + 1, total_passages
                );

                session.update_progress(
                    passage_idx,
                    total_passages,
                    format!("Processing passage {}/{}", passage_idx + 1, total_passages),
                );

                // Emit SongStarted event
                let _ = self.event_tx.send(ImportEvent::SongStarted {
                    session_id: session.session_id,
                    song_index: passage_idx,
                    total_songs: total_passages,
                });

                // Convert FusedBoundary to PassageBoundary for workflow engine
                let passage_boundary = PassageBoundary {
                    start_ticks: boundary.start_ticks,
                    end_ticks: boundary.end_ticks,
                    confidence: boundary.confidence,
                    detection_method: boundary.methods_used.first()
                        .copied()
                        .unwrap_or(BoundaryDetectionMethod::SilenceDetection),
                };

                // Process passage through workflow
                let start_time = std::time::Instant::now();
                let result = self.engine.process_passage(
                    session.session_id,
                    &file_path,
                    passage_idx,
                    total_passages,
                    &passage_boundary,
                ).await;

                let duration_ms = start_time.elapsed().as_millis() as u64;

                // SongWorkflowResult has a success field, not wrapped in Result
                if result.success {
                    successes += 1;

                    // Check if there are warnings in validation
                    if let Some(ref validation) = result.validation {
                        if !validation.warnings.is_empty() {
                            warnings += 1;
                        }
                    }

                    tracing::info!(
                        session_id = %session.session_id,
                        passage_idx = passage_idx,
                        duration_ms = duration_ms,
                        "Passage processing succeeded"
                    );

                    // Emit SongComplete event
                    let _ = self.event_tx.send(ImportEvent::SongComplete {
                        session_id: session.session_id,
                        song_index: passage_idx,
                        duration_ms,
                    });

                    // Phase 4: Database Persistence
                    // **[PLAN024 Sprint 2]** Save processed passage to database

                    // Convert SongWorkflowResult to ProcessedPassage
                    // Note: SongWorkflowResult currently returns partial data - we construct
                    // the required types from available fields
                    let processed = self.build_processed_passage(
                        result,
                        &passage_boundary,
                        duration_ms,
                    );

                    // Use file_id from boundary detection phase
                    match self.repository.save_processed_passage(&file_id, &processed, &session.session_id).await {
                        Ok(passage_id) => {
                            tracing::debug!(
                                session_id = %session.session_id,
                                passage_id = %passage_id,
                                "Passage persisted to database"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                session_id = %session.session_id,
                                passage_idx = passage_idx,
                                error = %e,
                                "Failed to persist passage to database (non-fatal, continuing)"
                            );
                            // Database errors are non-fatal - workflow continues
                        }
                    }
                } else {
                    failures += 1;
                    let error_msg = result.error.as_deref().unwrap_or("Unknown error");

                    tracing::warn!(
                        session_id = %session.session_id,
                        passage_idx = passage_idx,
                        error = error_msg,
                        "Passage processing failed"
                    );

                    // Emit SongFailed event
                    let _ = self.event_tx.send(ImportEvent::SongFailed {
                        session_id: session.session_id,
                        song_index: passage_idx,
                        error: error_msg.to_string(),
                    });
                }

                passage_idx += 1;
            }

            // Emit FileComplete event after processing all passages in a file
            let _ = self.event_tx.send(ImportEvent::FileComplete {
                session_id: session.session_id,
                file_path: file_path.display().to_string(),
                successes,
                warnings,
                failures,
                total_duration_ms: 0, // TODO: Track file-level duration
            });
        }

        tracing::info!(
            session_id = %session.session_id,
            total_passages = total_passages,
            successes = successes,
            warnings = warnings,
            failures = failures,
            "Per-passage processing complete"
        );

        // Phase 4: Completion - Finalize session
        session.transition_to(ImportState::Completed);
        session.update_progress(
            total_passages,
            total_passages,
            format!("Import completed - {} successes, {} warnings, {} failures", successes, warnings, failures),
        );
        crate::db::sessions::save_session(&self.db, &session).await?;

        tracing::info!(
            session_id = %session.session_id,
            files_processed = scan_result.files.len(),
            passages_processed = total_passages,
            successes = successes,
            warnings = warnings,
            failures = failures,
            "Import workflow completed (Phase 1-3: Scanning + Boundary Detection + Per-Passage Processing)"
        );

        Ok(session)
    }

    /// Handle workflow failure
    ///
    /// Transitions session to Failed state and broadcasts failure event
    pub async fn handle_failure(
        &self,
        mut session: ImportSession,
        error: &anyhow::Error,
    ) -> Result<()> {
        tracing::error!(
            session_id = %session.session_id,
            error = %error,
            "Import workflow failed"
        );

        session.transition_to(ImportState::Failed);
        session.progress.current_operation = format!("Import failed: {}", error);
        crate::db::sessions::save_session(&self.db, &session).await?;

        let _ = self.event_tx.send(ImportEvent::SessionFailed {
            session_id: session.session_id,
            error: error.to_string(),
        });

        Ok(())
    }

    /// Build ProcessedPassage from SongWorkflowResult
    ///
    /// **[PLAN024 Sprint 2]** Convert workflow result into database-persistable structure
    ///
    /// # Workarounds
    /// - SynthesizedFlavor: Wrapped from raw MusicalFlavor (until FlavorSynthesizer integration)
    ///
    /// # Arguments
    /// * `result` - Workflow execution result
    /// * `boundary` - Passage boundary information
    /// * `duration_ms` - Workflow execution duration
    fn build_processed_passage(
        &self,
        result: crate::import_v2::song_workflow_engine::SongWorkflowResult,
        boundary: &crate::import_v2::types::PassageBoundary,
        duration_ms: u64,
    ) -> crate::import_v2::types::ProcessedPassage {
        use crate::import_v2::types::{ProcessedPassage, SynthesizedFlavor, ExtractionSource};

        // Extract identity (guaranteed to exist on success)
        let identity = result.identity.expect("Identity must exist on successful workflow result");

        // Extract metadata (guaranteed to exist on success per workflow implementation)
        let metadata = result.metadata.expect("Metadata must exist on successful workflow result");

        // Construct SynthesizedFlavor from raw MusicalFlavor
        // Note: FlavorSynthesizer integrated in song_workflow_engine.rs (REQ-TD-007)
        let flavor = result.flavor.map(|musical_flavor| {
            SynthesizedFlavor {
                flavor: musical_flavor,
                flavor_confidence: 0.8,  // Placeholder - actual confidence from synthesizer
                flavor_completeness: 1.0,  // Assume complete for audio-derived flavor
                sources_used: vec![ExtractionSource::AudioDerived],
            }
        }).expect("Flavor must exist on successful workflow result");

        // Extract validation (guaranteed to exist on success)
        let validation = result.validation.expect("Validation must exist on successful workflow result");

        ProcessedPassage {
            identity,
            metadata,
            flavor,
            boundary: boundary.clone(),
            validation,
            import_duration_ms: duration_ms,
            import_timestamp: chrono::Utc::now().to_rfc3339(),
            import_version: "PLAN024-v1".to_string(),
        }
    }

    /// Extract file metadata and save to database
    ///
    /// **Resolves FOREIGN KEY constraint failures** by ensuring parent file record exists
    /// before passages are inserted.
    ///
    /// # Arguments
    /// * `file_id` - UUID for this file
    /// * `file_path` - Path to audio file
    ///
    /// # Returns
    /// * `Ok(())` - File successfully saved to database
    /// * `Err(anyhow::Error)` - File metadata extraction or database save failed
    async fn save_file_metadata(
        &self,
        file_id: &uuid::Uuid,
        file_path: &Path,
    ) -> Result<()> {
        // Calculate file hash
        let hash = calculate_file_hash(file_path)
            .unwrap_or_else(|e| {
                tracing::warn!(
                    file_id = %file_id,
                    file = %file_path.display(),
                    error = %e,
                    "Failed to calculate file hash, using empty string"
                );
                String::new()
            });

        // Get file size
        let file_size_bytes = std::fs::metadata(file_path)
            .ok()
            .map(|m| m.len() as i64);

        // Get modification time (use current time as fallback)
        let modification_time = std::fs::metadata(file_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|st| DateTime::from_timestamp(
                st.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64,
                0
            ))
            .unwrap_or_else(|| Utc::now());

        // Extract audio properties using lofty
        let (duration_ticks, format, sample_rate, channels) = match Probe::open(file_path)
            .and_then(|probe| probe.read())
        {
            Ok(tagged_file) => {
                let props = tagged_file.properties();

                // Duration in ticks (tick rate: 28,224,000 Hz per SPEC017)
                let duration_secs = props.duration().as_secs_f64();
                let duration_ticks = (duration_secs * 28_224_000.0) as i64;

                // Format (e.g., "MP3", "FLAC", "AAC")
                let format = format!("{:?}", tagged_file.file_type());

                // Sample rate and channels
                let sample_rate = props.sample_rate().map(|sr| sr as i32);
                let channels = props.channels().map(|ch| ch as i32);

                (Some(duration_ticks), Some(format), sample_rate, channels)
            }
            Err(e) => {
                tracing::warn!(
                    file_id = %file_id,
                    file = %file_path.display(),
                    error = %e,
                    "Failed to extract audio properties"
                );
                (None, None, None, None)
            }
        };

        // Create AudioFile struct
        let audio_file = AudioFile {
            guid: *file_id,
            path: file_path.display().to_string(),
            hash,
            duration_ticks,
            format,
            sample_rate,
            channels,
            file_size_bytes,
            modification_time,
        };

        // Save to database
        save_file(&self.db, &audio_file).await?;

        tracing::debug!(
            file_id = %file_id,
            file = %file_path.display(),
            "File metadata saved to database"
        );

        Ok(())
    }
}

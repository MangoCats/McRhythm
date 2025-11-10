//! Import workflow orchestrator
//!
//! **[AIA-WF-010]** Coordinates import workflow through all states
//!
//! # State Progression
//! SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING → COMPLETED
//!
//! # Architecture
//! This orchestrator implements a state machine for the audio file import workflow.
//! Each state is handled by a dedicated `phase_*` method:
//!
//! - **SCANNING** (line ~185): Scan filesystem for audio files
//! - **EXTRACTING** (line ~390): Extract ID3 metadata from files
//! - **FINGERPRINTING** (line ~533): Generate chromaprint fingerprints + AcoustID lookup
//!   - **[AIA-PERF-040]** Chromaprint generation parallelized (3-4x speedup)
//! - **SEGMENTING** (line ~907): Detect silence boundaries and segment passages
//! - **ANALYZING** (line ~1059): Extract audio-derived features (RMS, spectral)
//! - **FLAVORING** (line ~1157): Fetch AcousticBrainz/Essentia musical flavor vectors
//!
//! # Future Refactoring
//! This 1,459-line file could be split into separate modules per state for better
//! maintainability (see technical debt review for details).

use crate::models::{ImportSession, ImportState};
use crate::services::{
    AcousticBrainzClient, AcoustIDClient, AmplitudeAnalyzer, EssentiaClient, FileScanner,
    Fingerprinter, MetadataExtractor, MusicBrainzClient,
};
use anyhow::Result;
use rayon::prelude::*;
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;
use wkmp_common::events::{EventBus, WkmpEvent};

/// Workflow orchestrator service
pub struct WorkflowOrchestrator {
    db: SqlitePool,
    event_bus: EventBus,
    file_scanner: FileScanner,
    metadata_extractor: MetadataExtractor,
    fingerprinter: Fingerprinter,
    amplitude_analyzer: AmplitudeAnalyzer,
    mb_client: Option<MusicBrainzClient>,
    acoustid_client: Option<AcoustIDClient>,
    acousticbrainz_client: Option<AcousticBrainzClient>,
    essentia_client: Option<EssentiaClient>,
}

impl WorkflowOrchestrator {
    pub fn new(db: SqlitePool, event_bus: EventBus, acoustid_api_key: Option<String>) -> Self {
        // Initialize API clients (can fail, so wrapped in Option)
        let mb_client = MusicBrainzClient::new().ok();

        // Initialize AcoustID client with provided API key (if available)
        let acoustid_client = acoustid_api_key
            .and_then(|key| {
                if key.is_empty() {
                    tracing::warn!("AcoustID API key is empty, fingerprinting disabled");
                    None
                } else {
                    match AcoustIDClient::new(key, db.clone()) {
                        Ok(client) => {
                            tracing::info!("AcoustID client initialized with configured API key");
                            Some(client)
                        }
                        Err(e) => {
                            tracing::error!("Failed to initialize AcoustID client: {}", e);
                            None
                        }
                    }
                }
            });

        let acousticbrainz_client = AcousticBrainzClient::new().ok();
        let essentia_client = EssentiaClient::new().ok();

        // Log Essentia availability
        if essentia_client.is_some() {
            tracing::info!("Essentia available for local musical flavor extraction");
        } else {
            tracing::warn!("Essentia not available - install essentia_streaming_extractor_music for fallback analysis");
        }

        Self {
            db,
            event_bus,
            file_scanner: FileScanner::new(),
            metadata_extractor: MetadataExtractor::new(),
            fingerprinter: Fingerprinter::new(),
            amplitude_analyzer: AmplitudeAnalyzer::default(),
            mb_client,
            acoustid_client,
            acousticbrainz_client,
            essentia_client,
        }
    }

    /// Execute complete import workflow
    ///
    /// **[AIA-WF-010]** Progress through all states
    /// **[AIA-ASYNC-010]** Respects cancellation token
    pub async fn execute_import(
        &self,
        mut session: ImportSession,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Result<ImportSession> {
        let start_time = std::time::Instant::now();

        tracing::info!(
            session_id = %session.session_id,
            root_folder = %session.root_folder,
            "Starting import workflow"
        );

        // Broadcast session started event
        self.event_bus.emit_lossy(WkmpEvent::ImportSessionStarted {
            session_id: session.session_id,
            root_folder: session.root_folder.clone(),
            timestamp: Utc::now(),
        });

        // Phase 1: SCANNING - Discover audio files
        session = self.phase_scanning(session, start_time, &cancel_token).await?;
        if cancel_token.is_cancelled() {
            return Ok(session); // Return early with Cancelled state
        }

        // Phase 2: EXTRACTING - Extract metadata
        session = self.phase_extracting(session, start_time, &cancel_token).await?;
        if cancel_token.is_cancelled() {
            return Ok(session); // Return early with Cancelled state
        }

        // Phase 3: FINGERPRINTING - Audio fingerprinting (stub)
        session = self.phase_fingerprinting(session, start_time, &cancel_token).await?;
        if cancel_token.is_cancelled() {
            return Ok(session); // Return early with Cancelled state
        }

        // Phase 4: SEGMENTING - Passage detection (stub)
        session = self.phase_segmenting(session, start_time, &cancel_token).await?;
        if cancel_token.is_cancelled() {
            return Ok(session); // Return early with Cancelled state
        }

        // Phase 5: ANALYZING - Amplitude analysis (stub)
        session = self.phase_analyzing(session, start_time, &cancel_token).await?;
        if cancel_token.is_cancelled() {
            return Ok(session); // Return early with Cancelled state
        }

        // Phase 6: FLAVORING - Musical flavor extraction (stub)
        session = self.phase_flavoring(session, start_time, &cancel_token).await?;

        // Phase 7: COMPLETED
        session.transition_to(ImportState::Completed);
        session.update_progress(
            session.progress.total,
            session.progress.total,
            "Import completed successfully".to_string(),
        );

        crate::db::sessions::save_session(&self.db, &session).await?;

        // Clean up temporary mapping tables
        if let Err(e) = sqlx::query("DELETE FROM temp_file_songs")
            .execute(&self.db)
            .await
        {
            tracing::warn!("Failed to clean up temp_file_songs table: {}", e);
        }
        if let Err(e) = sqlx::query("DELETE FROM temp_file_albums")
            .execute(&self.db)
            .await
        {
            tracing::warn!("Failed to clean up temp_file_albums table: {}", e);
        }

        let duration_seconds = start_time.elapsed().as_secs();

        tracing::info!(
            session_id = %session.session_id,
            duration_seconds,
            "Import workflow completed successfully"
        );

        // Broadcast completion event
        self.event_bus.emit_lossy(WkmpEvent::ImportSessionCompleted {
            session_id: session.session_id,
            files_processed: session.progress.total,
            duration_seconds,
            timestamp: Utc::now(),
        });

        Ok(session)
    }

    /// Execute import workflow using PLAN024 pipeline
    ///
    /// **[PLAN024]** Modern 3-tier hybrid fusion pipeline
    /// **[AIA-ASYNC-010]** Respects cancellation token
    ///
    /// # Workflow
    /// 1. SCANNING: File discovery (reuses legacy phase_scanning)
    /// 2. PROCESSING: PLAN024 3-tier pipeline (replaces 5 legacy phases)
    /// 3. COMPLETED: Import finished
    pub async fn execute_import_plan024(
        &self,
        mut session: ImportSession,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Result<ImportSession> {
        let start_time = std::time::Instant::now();

        tracing::info!(
            session_id = %session.session_id,
            root_folder = %session.root_folder,
            "Starting PLAN024 import workflow"
        );

        // Broadcast session started event
        self.event_bus.emit_lossy(WkmpEvent::ImportSessionStarted {
            session_id: session.session_id,
            root_folder: session.root_folder.clone(),
            timestamp: Utc::now(),
        });

        // Phase 1: SCANNING - Discover audio files (reuse legacy implementation)
        session = self.phase_scanning(session, start_time, &cancel_token).await?;
        if cancel_token.is_cancelled() {
            return Ok(session);
        }

        // Phase 2: PROCESSING - PLAN024 3-tier hybrid fusion pipeline
        session = self.phase_processing_plan024(session, start_time, &cancel_token).await?;
        if cancel_token.is_cancelled() {
            return Ok(session);
        }

        // Phase 3: COMPLETED
        session.transition_to(ImportState::Completed);
        session.update_progress(
            session.progress.total,
            session.progress.total,
            "Import completed successfully with PLAN024 pipeline".to_string(),
        );

        crate::db::sessions::save_session(&self.db, &session).await?;

        let duration_seconds = start_time.elapsed().as_secs();

        tracing::info!(
            session_id = %session.session_id,
            duration_seconds,
            "PLAN024 import workflow completed successfully"
        );

        // Broadcast completion event
        self.event_bus.emit_lossy(WkmpEvent::ImportSessionCompleted {
            session_id: session.session_id,
            files_processed: session.progress.total,
            duration_seconds,
            timestamp: Utc::now(),
        });

        Ok(session)
    }

    // ============================================================================
    // PHASE 1: SCANNING
    // ============================================================================

    /// Phase 1: SCANNING - File discovery and database persistence
    /// **[AIA-ASYNC-010]** Checks cancellation token during file processing
    async fn phase_scanning(
        &self,
        mut session: ImportSession,
        start_time: std::time::Instant,
        cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<ImportSession> {
        session.transition_to(ImportState::Scanning);
        session.update_progress(0, 0, "Scanning for audio files...".to_string());
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(session_id = %session.session_id, "Phase 1: SCANNING");

        let scan_result = self
            .file_scanner
            .scan_with_stats(Path::new(&session.root_folder))?;

        tracing::info!(
            session_id = %session.session_id,
            files_found = scan_result.files.len(),
            total_size_mb = scan_result.total_size / 1_000_000,
            "File scan completed"
        );

        session.update_progress(
            0,
            scan_result.files.len(),
            format!("Found {} audio files, saving to database...", scan_result.files.len()),
        );
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        // Save discovered files to database
        let mut saved_count = 0;
        let total_files = scan_result.files.len();
        let scan_start_time = std::time::Instant::now();

        for (file_index, file_path) in scan_result.files.iter().enumerate() {
            // **[AIA-ASYNC-010]** Check for cancellation every file
            if cancel_token.is_cancelled() {
                let files_processed = file_index;
                tracing::info!(
                    session_id = %session.session_id,
                    files_processed = files_processed,
                    "Import cancelled during scanning phase"
                );
                session.transition_to(ImportState::Cancelled);
                session.progress.current_file = None;
                session.update_progress(
                    files_processed,
                    total_files,
                    "Import cancelled by user".to_string(),
                );
                crate::db::sessions::save_session(&self.db, &session).await?;
                return Ok(session);
            }

            // Get file metadata (fast - needed for modification time)
            let metadata = match std::fs::metadata(file_path) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(
                        session_id = %session.session_id,
                        file = %file_path.display(),
                        error = %e,
                        "Failed to read file metadata, skipping"
                    );
                    continue;
                }
            };
            let mod_time = metadata.modified()?;
            let mod_time_utc = chrono::DateTime::<Utc>::from(mod_time);

            // Create relative path from root folder (fast)
            let root_path = Path::new(&session.root_folder);
            let relative_path = file_path.strip_prefix(root_path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();

            // **[OPTIMIZATION]** Update progress BEFORE expensive operations (hash calculation)
            // This ensures UI shows activity even during slow hash operations
            // **[REQ-AIA-UI-004]** Set current file being processed
            session.progress.current_file = Some(relative_path.clone());

            // Calculate ETA based on files processed so far
            let files_processed = file_index + 1;
            let elapsed = scan_start_time.elapsed().as_secs_f64();
            let eta_message = if files_processed > 5 && elapsed > 1.0 {
                let avg_time_per_file = elapsed / files_processed as f64;
                let files_remaining = total_files - files_processed;
                let eta_seconds = (files_remaining as f64 * avg_time_per_file) as u64;
                let eta_minutes = eta_seconds / 60;
                let eta_secs = eta_seconds % 60;
                format!(" (ETA: {}m {}s)", eta_minutes, eta_secs)
            } else {
                String::new()
            };

            session.update_progress(
                files_processed,
                total_files,
                format!("Scanning file {} of {}: {}{}", files_processed, total_files, relative_path, eta_message),
            );
            crate::db::sessions::save_session(&self.db, &session).await?;
            self.broadcast_progress(&session, start_time);

            // **[OPTIMIZATION]** Check if file exists and is unchanged
            // Skip expensive hash calculation for unchanged files (95% speedup on re-scans)
            if let Ok(Some(existing)) = crate::db::files::load_file_by_path(&self.db, &relative_path).await {
                if existing.modification_time == mod_time_utc {
                    // File unchanged since last import - skip hashing entirely
                    tracing::debug!(
                        session_id = %session.session_id,
                        file = %relative_path,
                        "File unchanged (same modification time), skipping hash calculation"
                    );
                    saved_count += 1;
                    continue;
                }
                // File modified - log and fall through to hash calculation
                tracing::debug!(
                    session_id = %session.session_id,
                    file = %relative_path,
                    old_mtime = %existing.modification_time,
                    new_mtime = %mod_time_utc,
                    "File modification time changed, recalculating hash"
                );
            }

            // Calculate file hash (only for new or modified files)
            let hash = match crate::db::files::calculate_file_hash(file_path) {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!(
                        session_id = %session.session_id,
                        file = %file_path.display(),
                        error = %e,
                        "Failed to hash file, skipping"
                    );
                    continue;
                }
            };

            // Check for duplicate by hash (different file path, same content)
            if let Ok(Some(existing)) = crate::db::files::load_file_by_hash(&self.db, &hash).await {
                tracing::debug!(
                    session_id = %session.session_id,
                    new_path = %relative_path,
                    existing_path = %existing.path,
                    "Duplicate file detected (different path, same hash)"
                );
                saved_count += 1;
                continue;
            }

            // Extract audio metadata (format, sample_rate, channels, file_size_bytes)
            let metadata = match self.metadata_extractor.extract(file_path) {
                Ok(meta) => Some(meta),
                Err(e) => {
                    tracing::warn!(
                        session_id = %session.session_id,
                        file = %relative_path,
                        error = %e,
                        "Failed to extract metadata, saving file without metadata"
                    );
                    None
                }
            };

            // Create audio file record
            let mut audio_file = crate::db::files::AudioFile::new(
                relative_path.clone(),
                hash,
                mod_time_utc,
            );

            // Populate metadata fields if extraction succeeded
            if let Some(meta) = metadata {
                audio_file.format = Some(meta.format);
                audio_file.sample_rate = meta.sample_rate.map(|sr| sr as i32);
                audio_file.channels = meta.channels.map(|ch| ch as i32);
                audio_file.file_size_bytes = Some(meta.file_size_bytes as i64);
            }

            // Save to database
            if let Err(e) = crate::db::files::save_file(&self.db, &audio_file).await {
                tracing::warn!(
                    session_id = %session.session_id,
                    file = %relative_path,
                    error = %e,
                    "Failed to save file to database"
                );
            } else {
                saved_count += 1;
                tracing::debug!(
                    session_id = %session.session_id,
                    file = %relative_path,
                    file_id = %audio_file.guid,
                    "File saved to database"
                );
            }
        }

        session.update_progress(
            saved_count,
            saved_count,
            format!("Saved {} files to database", saved_count),
        );
        session.progress.total = saved_count;

        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(
            session_id = %session.session_id,
            files_saved = saved_count,
            "File scanning and database persistence completed"
        );

        Ok(session)
    }

    // ============================================================================
    // PHASE 2: EXTRACTING
    // ============================================================================

    /// Phase 2: EXTRACTING - Metadata extraction and persistence
    async fn phase_extracting(
        &self,
        mut session: ImportSession,
        start_time: std::time::Instant,
        _cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<ImportSession> {
        session.transition_to(ImportState::Extracting);
        session.update_progress(0, session.progress.total, "Extracting metadata...".to_string());
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(
            session_id = %session.session_id,
            total_files = session.progress.total,
            "Phase 2: EXTRACTING"
        );

        // Load all files from database
        let files = crate::db::files::load_all_files(&self.db).await?;
        let root_folder = session.root_folder.clone();
        let root_path = Path::new(&root_folder);

        let mut extracted_count = 0;
        let mut skipped_count = 0;
        for file in &files {
            // **[OPTIMIZATION]** Skip extraction if file already has metadata (duration indicates success)
            // REQ-F-003: Changed from file.duration to file.duration_ticks
            if file.duration_ticks.is_some() {
                skipped_count += 1;
                tracing::debug!(
                    session_id = %session.session_id,
                    file = %file.path,
                    "Skipping extraction - metadata already exists"
                );

                continue;
            }

            // Construct absolute path
            let file_path = root_path.join(&file.path);

            // Extract metadata using lofty
            match self.metadata_extractor.extract(&file_path) {
                Ok(metadata) => {
                    tracing::debug!(
                        session_id = %session.session_id,
                        file = %file.path,
                        title = ?metadata.title,
                        artist = ?metadata.artist,
                        album = ?metadata.album,
                        duration = ?metadata.duration_seconds,
                        "Metadata extracted"
                    );

                    // Update file duration if available
                    // REQ-F-003: Convert seconds to ticks before storing
                    if let Some(duration_seconds) = metadata.duration_seconds {
                        let duration_ticks = wkmp_common::timing::seconds_to_ticks(duration_seconds);
                        if let Err(e) = crate::db::files::update_file_duration(&self.db, file.guid, duration_ticks).await {
                            tracing::warn!(
                                session_id = %session.session_id,
                                file = %file.path,
                                error = %e,
                                "Failed to update file duration"
                            );
                        }
                    }

                    // Load passages for this file and update their metadata
                    match crate::db::passages::load_passages_for_file(&self.db, file.guid).await {
                        Ok(passages) => {
                            for passage in passages {
                                if let Err(e) = crate::db::passages::update_passage_metadata(
                                    &self.db,
                                    passage.guid,
                                    metadata.title.clone(),
                                    metadata.artist.clone(),
                                    metadata.album.clone(),
                                ).await {
                                    tracing::warn!(
                                        session_id = %session.session_id,
                                        passage_id = %passage.guid,
                                        error = %e,
                                        "Failed to update passage metadata"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                session_id = %session.session_id,
                                file = %file.path,
                                error = %e,
                                "Failed to load passages for file"
                            );
                        }
                    }

                    extracted_count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        session_id = %session.session_id,
                        file = %file.path,
                        error = %e,
                        "Failed to extract metadata"
                    );
                    extracted_count += 1; // Still count as processed
                }
            }

            // Update progress for every file processed
            let total_processed = extracted_count + skipped_count;
            session.progress.current_file = Some(file.path.clone());
            session.update_progress(
                total_processed,
                files.len(),
                format!("Extracting metadata from file {} of {}", total_processed, files.len()),
            );
            crate::db::sessions::save_session(&self.db, &session).await?;
            self.broadcast_progress(&session, start_time);
        }

        let total_processed = extracted_count + skipped_count;
        session.update_progress(
            total_processed,
            total_processed,
            format!("Extracted {} / Skipped {} unchanged files", extracted_count, skipped_count),
        );
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(
            session_id = %session.session_id,
            extracted_count,
            skipped_count,
            "Metadata extraction completed"
        );

        Ok(session)
    }

    // ============================================================================
    // PHASE 3: FINGERPRINTING
    // ============================================================================

    /// Phase 3: FINGERPRINTING - Audio fingerprinting (stub)
    async fn phase_fingerprinting(
        &self,
        mut session: ImportSession,
        start_time: std::time::Instant,
        _cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<ImportSession> {
        session.transition_to(ImportState::Fingerprinting);

        // **[REQ-AIA-UI-003]** Initialize sub-task counters for fingerprinting phase
        use crate::models::import_session::SubTaskStatus;
        if let Some(phase) = session.progress.get_phase_mut(crate::models::ImportState::Fingerprinting) {
            phase.subtasks = vec![
                SubTaskStatus::new("Chromaprint"),
                SubTaskStatus::new("AcoustID"),
                SubTaskStatus::new("MusicBrainz"),
            ];
        }

        session.update_progress(
            0,
            session.progress.total,
            "Fingerprinting audio files...".to_string(),
        );
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(
            session_id = %session.session_id,
            "Phase 3: FINGERPRINTING"
        );

        // Load all files from database
        let files = crate::db::files::load_all_files(&self.db).await?;
        let root_folder = session.root_folder.clone();
        let root_path = Path::new(&root_folder);

        // **[AIA-PERF-040]** Phase 1: Parallel Chromaprint fingerprint generation (3-4x speedup)
        // **[AIA-PERF-050]** With real-time progress tracking and UI updates
        tracing::info!("Generating fingerprints in parallel...");
        let fingerprint_start = std::time::Instant::now();

        // Thread-safe progress counters
        let processed_count = Arc::new(AtomicUsize::new(0));
        let success_count = Arc::new(AtomicUsize::new(0));
        let failure_count = Arc::new(AtomicUsize::new(0));
        let total_files = files.len();

        // Clone data needed by both parallel work and progress monitoring
        let progress_counter = processed_count.clone();
        let progress_success = success_count.clone();
        let progress_failure = failure_count.clone();
        let progress_session = session.clone();
        let progress_db = self.db.clone();
        let progress_event_bus = self.event_bus.clone();
        let progress_start_time = Arc::new(fingerprint_start); // Share start time via Arc

        // Spawn background task for periodic progress updates
        let progress_task = tokio::spawn(async move {
            tracing::debug!("Progress monitoring task started");
            let mut last_count = 0;
            let update_interval = std::time::Duration::from_secs(2); // Update every 2 seconds

            loop {
                tokio::time::sleep(update_interval).await;

                let current_count = progress_counter.load(Ordering::Relaxed);
                let current_success = progress_success.load(Ordering::Relaxed);
                let current_failure = progress_failure.load(Ordering::Relaxed);

                tracing::debug!(
                    "Progress check: {}/{} (last: {})",
                    current_count, total_files, last_count
                );

                if current_count == total_files {
                    tracing::debug!("Progress monitoring: All files processed, exiting");
                    break; // Finished
                }

                if current_count > last_count {
                    let elapsed = progress_start_time.elapsed().as_secs_f64();
                    let rate = if elapsed > 0.0 {
                        current_count as f64 / elapsed
                    } else {
                        0.0
                    };
                    let remaining = total_files - current_count;
                    let eta_secs = if rate > 0.0 {
                        (remaining as f64 / rate) as u64
                    } else {
                        0
                    };

                    tracing::info!(
                        "Fingerprinting progress: {}/{} ({:.1}%) | Rate: {:.1} files/sec | ETA: {}s | Success: {} | Failed: {}",
                        current_count,
                        total_files,
                        (current_count as f64 / total_files as f64) * 100.0,
                        rate,
                        eta_secs,
                        current_success,
                        current_failure
                    );

                    // Update session progress and broadcast
                    let mut updated_session = progress_session.clone();
                    updated_session.update_progress(
                        current_count,
                        total_files,
                        format!("Fingerprinting: {}/{} ({:.0}%)", current_count, total_files, (current_count as f64 / total_files as f64) * 100.0)
                    );

                    // Save to database (non-blocking, best-effort)
                    let _ = crate::db::sessions::save_session(&progress_db, &updated_session).await;

                    // Broadcast progress event via SSE
                    let elapsed_secs = elapsed as u64;
                    progress_event_bus.emit_lossy(WkmpEvent::ImportProgressUpdate {
                        session_id: updated_session.session_id,
                        state: format!("{:?}", updated_session.state),
                        current: current_count,
                        total: total_files,
                        percentage: (current_count as f32 / total_files as f32) * 100.0,
                        current_operation: format!("Fingerprinting {}/{} ({:.1} files/sec)", current_count, total_files, rate),
                        elapsed_seconds: elapsed_secs,
                        estimated_remaining_seconds: Some(eta_secs),
                        phases: vec![], // Not tracking phases in parallel section
                        current_file: None, // Parallel processing, no single "current" file
                        timestamp: chrono::Utc::now(),
                    });

                    last_count = current_count;
                }
            }

            tracing::debug!("Progress monitoring task completed");
        });

        // Move blocking Rayon work to separate thread pool to avoid blocking tokio runtime
        let fingerprinter = self.fingerprinter.clone();
        let files_for_processing = files.clone();
        let root_path_owned = root_path.to_path_buf();
        let processed_counter = processed_count.clone();
        let success_counter = success_count.clone();
        let failure_counter = failure_count.clone();

        tracing::debug!("Starting parallel fingerprinting in spawn_blocking");

        let fingerprint_results: Vec<(usize, Option<String>)> = tokio::task::spawn_blocking(move || {
            tracing::debug!("Rayon parallel fingerprinting starting");
            let results: Vec<(usize, Option<String>)> = files_for_processing
                .par_iter()
                .enumerate()
                .map(|(idx, file)| {
                    if idx % 100 == 0 {
                        tracing::debug!("Processing file {}/{}", idx, files_for_processing.len());
                    }

                    let file_path = root_path_owned.join(&file.path);
                    let fingerprint = fingerprinter.fingerprint_file(&file_path).ok();

                    // Update counters
                    processed_counter.fetch_add(1, Ordering::Relaxed);
                    if fingerprint.is_some() {
                        success_counter.fetch_add(1, Ordering::Relaxed);
                    } else {
                        failure_counter.fetch_add(1, Ordering::Relaxed);
                    }

                    (idx, fingerprint)
                })
                .collect();

            tracing::debug!("Rayon parallel fingerprinting completed: {} results", results.len());
            results
        })
        .await
        .expect("Fingerprinting task panicked");

        tracing::debug!("spawn_blocking completed, waiting for progress task");

        // Wait for progress task to finish
        let _ = progress_task.await;

        let final_success = success_count.load(Ordering::Relaxed);
        let final_failure = failure_count.load(Ordering::Relaxed);
        let elapsed = fingerprint_start.elapsed();
        let rate = total_files as f64 / elapsed.as_secs_f64();

        tracing::info!(
            "Parallel fingerprinting completed in {:?} | Total: {} | Success: {} | Failed: {} | Rate: {:.1} files/sec",
            elapsed,
            total_files,
            final_success,
            final_failure,
            rate
        );

        // Update counters for Chromaprint phase
        let chromaprint_success = fingerprint_results.iter().filter(|(_, fp)| fp.is_some()).count();
        let chromaprint_failed = fingerprint_results.len() - chromaprint_success;
        if let Some(phase) = session.progress.get_phase_mut(crate::models::ImportState::Fingerprinting) {
            if let Some(subtask) = phase.subtasks.iter_mut().find(|s| s.name == "Chromaprint") {
                subtask.success_count = chromaprint_success;
                subtask.failure_count = chromaprint_failed;
            }
        }

        tracing::info!(
            "Chromaprint results: {} succeeded, {} failed",
            chromaprint_success,
            chromaprint_failed
        );

        // **[AIA-PERF-040]** Phase 2: Sequential API calls and database writes
        // (rate-limited, cannot parallelize)
        let mut processed_count = 0;

        for (idx, file) in files.iter().enumerate() {
            // **[REQ-AIA-UI-004]** Set current file
            session.progress.current_file = Some(file.path.clone());

            // Get pre-generated fingerprint from parallel phase
            let fingerprint = match &fingerprint_results[idx].1 {
                Some(fp) => fp.clone(),
                None => {
                    tracing::warn!("Skipping {} (fingerprinting failed)", file.path);
                    processed_count += 1;
                    continue;
                }
            };

            // REQ-F-003: Convert from ticks to seconds for AcoustID API
            let duration = if let Some(ticks) = file.duration_ticks {
                wkmp_common::timing::ticks_to_seconds(ticks) as u64
            } else {
                120  // Default 120 seconds if duration unknown
            };

            // Query AcoustID if client available
            if let Some(ref acoustid) = self.acoustid_client {
                match acoustid.lookup(&fingerprint, duration).await {
                    Ok(response) => {
                        // Process top result
                        if let Some(top_result) = response.results.first() {
                            if let Some(ref recordings) = top_result.recordings {
                                if let Some(recording) = recordings.first() {
                                    // **[REQ-AIA-UI-003]** Increment AcoustID success counter (found)
                                    if let Some(phase) = session.progress.get_phase_mut(crate::models::ImportState::Fingerprinting) {
                                        if let Some(subtask) = phase.subtasks.iter_mut().find(|s| s.name == "AcoustID") {
                                            subtask.success_count += 1;
                                        }
                                    }

                                    // Query MusicBrainz for detailed metadata
                                    if let Some(ref mb) = self.mb_client {
                                        match mb.lookup_recording(&recording.id).await {
                                            Ok(mb_recording) => {
                                            // **[REQ-AIA-UI-003]** Increment MusicBrainz success counter
                                            if let Some(phase) = session.progress.get_phase_mut(crate::models::ImportState::Fingerprinting) {
                                                if let Some(subtask) = phase.subtasks.iter_mut().find(|s| s.name == "MusicBrainz") {
                                                    subtask.success_count += 1;
                                                }
                                            }
                                            // Save song with MusicBrainz title (may update existing if recording_mbid exists)
                                            let song = crate::db::songs::Song::new(
                                                recording.id.clone(),
                                                Some(mb_recording.title.clone())
                                            );
                                            if let Err(e) = crate::db::songs::save_song(&self.db, &song).await {
                                                tracing::error!(
                                                    song_id = %song.guid,
                                                    recording_mbid = %recording.id,
                                                    file_path = %file.path,
                                                    error = %e,
                                                    "FK constraint failed when saving song"
                                                );
                                                return Err(e);
                                            }

                                            // Load the song back to get the actual guid (may differ if ON CONFLICT UPDATE occurred)
                                            let song = match crate::db::songs::load_song_by_mbid(&self.db, &recording.id).await? {
                                                Some(s) => s,
                                                None => {
                                                    tracing::error!(
                                                        recording_mbid = %recording.id,
                                                        file_path = %file.path,
                                                        "Song not found after save"
                                                    );
                                                    continue;
                                                }
                                            };

                                            // Save artists and link to song
                                            for artist_credit in &mb_recording.artist_credit {
                                                let artist = crate::db::artists::Artist::new(
                                                    artist_credit.artist.id.clone(),
                                                    artist_credit.artist.name.clone(),
                                                );
                                                if let Err(e) = crate::db::artists::save_artist(&self.db, &artist).await {
                                                    tracing::error!(
                                                        artist_id = %artist.guid,
                                                        artist_mbid = %artist_credit.artist.id,
                                                        file_path = %file.path,
                                                        error = %e,
                                                        "FK constraint failed when saving artist"
                                                    );
                                                    return Err(e);
                                                }

                                                // Load artist back to get actual guid (may differ if ON CONFLICT UPDATE occurred)
                                                let artist = match crate::db::artists::load_artist_by_mbid(&self.db, &artist_credit.artist.id).await? {
                                                    Some(a) => a,
                                                    None => {
                                                        tracing::warn!(
                                                            artist_mbid = %artist_credit.artist.id,
                                                            file_path = %file.path,
                                                            "Artist not found after save"
                                                        );
                                                        continue;
                                                    }
                                                };

                                                // Link song to artist (equal weight)
                                                let weight = 1.0 / mb_recording.artist_credit.len() as f64;
                                                if let Err(e) = crate::db::artists::link_song_to_artist(&self.db, song.guid, artist.guid, weight).await {
                                                    tracing::error!(
                                                        song_id = %song.guid,
                                                        artist_id = %artist.guid,
                                                        file_path = %file.path,
                                                        error = %e,
                                                        "FK constraint failed when linking song to artist"
                                                    );
                                                    return Err(e);
                                                }
                                            }

                                            // Save album if available
                                            if let Some(ref releases) = mb_recording.releases {
                                                if let Some(release) = releases.first() {
                                                    let album = crate::db::albums::Album::new(
                                                        release.id.clone(),
                                                        release.title.clone(),
                                                    );
                                                    if let Err(e) = crate::db::albums::save_album(&self.db, &album).await {
                                                        tracing::error!(
                                                            album_id = %album.guid,
                                                            album_mbid = %release.id,
                                                            file_path = %file.path,
                                                            error = %e,
                                                            "FK constraint failed when saving album"
                                                        );
                                                        return Err(e);
                                                    }

                                                    // Load album back to get actual guid (may differ if ON CONFLICT UPDATE occurred)
                                                    let album = match crate::db::albums::load_album_by_mbid(&self.db, &release.id).await? {
                                                        Some(a) => a,
                                                        None => {
                                                            tracing::warn!(
                                                                album_mbid = %release.id,
                                                                file_path = %file.path,
                                                                "Album not found after save"
                                                            );
                                                            continue;
                                                        }
                                                    };

                                                    // Store file → album mapping for later passage linking
                                                    // (passages don't exist yet - they're created in segmenting phase)
                                                    if let Err(e) = sqlx::query(
                                                        "INSERT INTO temp_file_albums (file_id, album_id) VALUES (?, ?)
                                                         ON CONFLICT(file_id, album_id) DO NOTHING"
                                                    )
                                                    .bind(file.guid.to_string())
                                                    .bind(album.guid.to_string())
                                                    .execute(&self.db)
                                                    .await
                                                    {
                                                        tracing::error!(
                                                            file_id = %file.guid,
                                                            album_id = %album.guid,
                                                            file_path = %file.path,
                                                            error = %e,
                                                            "FK constraint failed when inserting into temp_file_albums"
                                                        );
                                                        return Err(e.into());
                                                    }
                                                }
                                            }

                                            // Save work if available
                                            if let Some(ref relations) = mb_recording.relations {
                                                for relation in relations {
                                                    if relation.relation_type == "performance" || relation.relation_type == "cover" {
                                                        if let Some(ref work) = relation.work {
                                                            let db_work = crate::db::works::Work::new(
                                                                work.id.clone(),
                                                                work.title.clone(),
                                                            );
                                                            if let Err(e) = crate::db::works::save_work(&self.db, &db_work).await {
                                                                tracing::error!(
                                                                    work_id = %db_work.guid,
                                                                    work_mbid = %work.id,
                                                                    file_path = %file.path,
                                                                    error = %e,
                                                                    "FK constraint failed when saving work"
                                                                );
                                                                return Err(e);
                                                            }

                                                            // Load work back to get actual guid (may differ if ON CONFLICT UPDATE occurred)
                                                            let db_work = match crate::db::works::load_work_by_mbid(&self.db, &work.id).await? {
                                                                Some(w) => w,
                                                                None => {
                                                                    tracing::warn!(
                                                                        work_mbid = %work.id,
                                                                        file_path = %file.path,
                                                                        "Work not found after save"
                                                                    );
                                                                    continue;
                                                                }
                                                            };

                                                            // Link song to work
                                                            if let Err(e) = crate::db::works::link_song_to_work(&self.db, song.guid, db_work.guid).await {
                                                                tracing::error!(
                                                                    song_id = %song.guid,
                                                                    work_id = %db_work.guid,
                                                                    file_path = %file.path,
                                                                    error = %e,
                                                                    "FK constraint failed when linking song to work"
                                                                );
                                                                return Err(e);
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            // Store file → song mapping for later passage linking
                                            // (passages don't exist yet - they're created in segmenting phase)
                                            if let Err(e) = sqlx::query(
                                                "INSERT INTO temp_file_songs (file_id, song_id) VALUES (?, ?)
                                                 ON CONFLICT(file_id) DO UPDATE SET song_id = excluded.song_id"
                                            )
                                            .bind(file.guid.to_string())
                                            .bind(song.guid.to_string())
                                            .execute(&self.db)
                                            .await
                                            {
                                                tracing::error!(
                                                    file_id = %file.guid,
                                                    song_id = %song.guid,
                                                    file_path = %file.path,
                                                    error = %e,
                                                    "FK constraint failed when inserting into temp_file_songs"
                                                );
                                                return Err(e.into());
                                            }

                                            tracing::info!(
                                                file = %file.path,
                                                recording_mbid = %recording.id,
                                                "Successfully fingerprinted and linked to MusicBrainz"
                                            );
                                            }
                                            Err(e) => {
                                                // Log MusicBrainz lookup error
                                                tracing::warn!(
                                                    recording_mbid = %recording.id,
                                                    file = %file.path,
                                                    error = ?e,
                                                    "MusicBrainz lookup failed"
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    // **[REQ-AIA-UI-003]** Increment AcoustID failure counter (no recording in result)
                                    if let Some(phase) = session.progress.get_phase_mut(crate::models::ImportState::Fingerprinting) {
                                        if let Some(subtask) = phase.subtasks.iter_mut().find(|s| s.name == "AcoustID") {
                                            subtask.failure_count += 1;
                                        }
                                    }
                                }
                            } else {
                                // **[REQ-AIA-UI-003]** Increment AcoustID failure counter (no recordings field)
                                if let Some(phase) = session.progress.get_phase_mut(crate::models::ImportState::Fingerprinting) {
                                    if let Some(subtask) = phase.subtasks.iter_mut().find(|s| s.name == "AcoustID") {
                                        subtask.failure_count += 1;
                                    }
                                }
                            }
                        } else {
                            // **[REQ-AIA-UI-003]** Increment AcoustID failure counter (no results)
                            if let Some(phase) = session.progress.get_phase_mut(crate::models::ImportState::Fingerprinting) {
                                if let Some(subtask) = phase.subtasks.iter_mut().find(|s| s.name == "AcoustID") {
                                    subtask.failure_count += 1;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("AcoustID lookup failed for {}: {}", file.path, e);
                        // **[REQ-AIA-UI-003]** Increment AcoustID failure counter (lookup error)
                        if let Some(phase) = session.progress.get_phase_mut(crate::models::ImportState::Fingerprinting) {
                            if let Some(subtask) = phase.subtasks.iter_mut().find(|s| s.name == "AcoustID") {
                                subtask.failure_count += 1;
                            }
                        }
                    }
                }
            }

            processed_count += 1;

            // Update progress on every file (per-file progress indicator)
            session.progress.current_file = Some(file.path.clone());
            session.update_progress(
                processed_count,
                files.len(),
                format!("Fingerprinting file {} of {}", processed_count, files.len()),
            );
            crate::db::sessions::save_session(&self.db, &session).await?;
            self.broadcast_progress(&session, start_time);
        }

        // Final progress update
        session.update_progress(processed_count, files.len(), "Fingerprinting completed".to_string());
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        Ok(session)
    }

    // ============================================================================
    // PHASE 4: SEGMENTING
    // ============================================================================

    /// Phase 4: SEGMENTING - Passage creation
    async fn phase_segmenting(
        &self,
        mut session: ImportSession,
        start_time: std::time::Instant,
        _cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<ImportSession> {
        session.transition_to(ImportState::Segmenting);
        session.update_progress(
            0,
            session.progress.total,
            "Creating passages...".to_string(),
        );
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(
            session_id = %session.session_id,
            "Phase 4: SEGMENTING"
        );

        // Load all files from database
        let files = crate::db::files::load_all_files(&self.db).await?;

        tracing::info!(
            session_id = %session.session_id,
            file_count = files.len(),
            "Creating passages for files"
        );

        let mut passages_created = 0;
        for file in &files {
            // For now, create one passage per file (entire file)
            // In production, this would:
            // 1. Run silence detection to find boundaries
            // 2. Create multiple passages per file if silence detected
            // 3. Use detected lead-in/lead-out timing

            // Get file duration (default to 180 seconds if not set)
            // REQ-F-003: Convert from ticks to seconds for passage creation
            let duration_sec = if let Some(ticks) = file.duration_ticks {
                wkmp_common::timing::ticks_to_seconds(ticks)
            } else {
                180.0  // Default 180 seconds if duration unknown
            };

            // Create passage spanning entire file
            let passage = crate::db::passages::Passage::new(
                file.guid,
                0.0,          // start_sec
                duration_sec, // end_sec
            );

            // Save passage to database
            if let Err(e) = crate::db::passages::save_passage(&self.db, &passage).await {
                tracing::warn!(
                    session_id = %session.session_id,
                    file = %file.path,
                    error = %e,
                    "Failed to save passage to database"
                );
            } else {
                passages_created += 1;
                tracing::debug!(
                    session_id = %session.session_id,
                    file = %file.path,
                    passage_id = %passage.guid,
                    duration_sec,
                    "Passage created"
                );

                // Link passage to song if fingerprinting identified one
                if let Ok(Some(row)) = sqlx::query_as::<_, (String,)>(
                    "SELECT song_id FROM temp_file_songs WHERE file_id = ?"
                )
                .bind(file.guid.to_string())
                .fetch_optional(&self.db)
                .await
                {
                    if let Ok(song_guid) = Uuid::parse_str(&row.0) {
                        if let Err(e) = crate::db::songs::link_passage_to_song(
                            &self.db,
                            passage.guid,
                            song_guid,
                            passage.start_time_ticks,
                            passage.end_time_ticks,
                        ).await {
                            tracing::warn!(
                                session_id = %session.session_id,
                                file = %file.path,
                                error = %e,
                                "Failed to link passage to song"
                            );
                        }
                    }
                }

                // Link passage to albums if fingerprinting identified any
                if let Ok(rows) = sqlx::query_as::<_, (String,)>(
                    "SELECT album_id FROM temp_file_albums WHERE file_id = ?"
                )
                .bind(file.guid.to_string())
                .fetch_all(&self.db)
                .await
                {
                    for row in rows {
                        if let Ok(album_guid) = Uuid::parse_str(&row.0) {
                            if let Err(e) = crate::db::albums::link_passage_to_album(
                                &self.db,
                                passage.guid,
                                album_guid,
                            ).await {
                                tracing::warn!(
                                    session_id = %session.session_id,
                                    file = %file.path,
                                    error = %e,
                                    "Failed to link passage to album"
                                );
                            }
                        }
                    }
                }
            }

            // Update progress for every file processed
            session.progress.current_file = Some(file.path.clone());
            session.update_progress(
                passages_created,
                files.len(),
                format!("Creating passage {} of {}", passages_created, files.len()),
            );
            crate::db::sessions::save_session(&self.db, &session).await?;
            self.broadcast_progress(&session, start_time);
        }

        session.update_progress(
            passages_created,
            passages_created,
            format!("Created {} passages", passages_created),
        );
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(
            session_id = %session.session_id,
            passages_created,
            "Passage creation completed"
        );

        Ok(session)
    }

    // ============================================================================
    // PHASE 5: ANALYZING
    // ============================================================================

    /// Phase 5: ANALYZING - Amplitude analysis (stub)
    async fn phase_analyzing(
        &self,
        mut session: ImportSession,
        start_time: std::time::Instant,
        _cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<ImportSession> {
        session.transition_to(ImportState::Analyzing);
        session.update_progress(
            0,
            session.progress.total,
            "Analyzing amplitude profiles...".to_string(),
        );
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(
            session_id = %session.session_id,
            "Phase 5: ANALYZING"
        );

        // Load all files from database
        let files = crate::db::files::load_all_files(&self.db).await?;
        let root_folder = session.root_folder.clone();
        let root_path = Path::new(&root_folder);

        let mut analyzed_count = 0;

        for file in &files {
            // Construct absolute path
            let file_path = root_path.join(&file.path);

            // Load passages for this file
            let passages = crate::db::passages::load_passages_for_file(&self.db, file.guid).await?;

            for passage in passages {
                // Calculate passage timing in seconds
                let start_sec = wkmp_common::timing::ticks_to_seconds(passage.start_time_ticks);
                let end_sec = wkmp_common::timing::ticks_to_seconds(passage.end_time_ticks);

                // Analyze amplitude profile
                match self.amplitude_analyzer.analyze_file(&file_path, start_sec, end_sec).await {
                    Ok(analysis) => {
                        // Calculate lead-in and lead-out start times relative to passage start
                        let lead_in_start_sec = start_sec + analysis.lead_in_duration;
                        let lead_out_start_sec = end_sec - analysis.lead_out_duration;

                        // Convert to ticks
                        let lead_in_start_ticks = Some(wkmp_common::timing::seconds_to_ticks(lead_in_start_sec));
                        let lead_out_start_ticks = Some(wkmp_common::timing::seconds_to_ticks(lead_out_start_sec));

                        // Update passage timing in database
                        crate::db::passages::update_passage_timing(
                            &self.db,
                            passage.guid,
                            lead_in_start_ticks,
                            lead_out_start_ticks,
                        ).await?;

                        tracing::debug!(
                            passage_id = %passage.guid,
                            lead_in_duration = analysis.lead_in_duration,
                            lead_out_duration = analysis.lead_out_duration,
                            "Amplitude analysis completed"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            file = %file.path,
                            passage_id = %passage.guid,
                            error = %e,
                            "Amplitude analysis failed, using defaults"
                        );
                        // Continue with other passages
                    }
                }
            }

            analyzed_count += 1;

            // Update progress for every file processed
            session.progress.current_file = Some(file.path.clone());
            session.update_progress(
                analyzed_count,
                files.len(),
                format!("Analyzing amplitude profile for file {} of {}", analyzed_count, files.len()),
            );
            crate::db::sessions::save_session(&self.db, &session).await?;
            self.broadcast_progress(&session, start_time);
        }

        // Final progress update
        session.update_progress(analyzed_count, files.len(), "Amplitude analysis completed".to_string());
        crate::db::sessions::save_session(&self.db, &session).await?;

        Ok(session)
    }

    // ============================================================================
    // PHASE 6: FLAVORING
    // ============================================================================

    /// Phase 6: FLAVORING - Musical flavor extraction via AcousticBrainz
    async fn phase_flavoring(
        &self,
        mut session: ImportSession,
        start_time: std::time::Instant,
        _cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<ImportSession> {
        session.transition_to(ImportState::Flavoring);
        session.update_progress(
            0,
            session.progress.total,
            "Extracting musical flavors...".to_string(),
        );
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(
            session_id = %session.session_id,
            "Phase 6: FLAVORING"
        );

        // Load all files from database
        let files = crate::db::files::load_all_files(&self.db).await?;
        let root_folder = session.root_folder.clone();
        let root_path = Path::new(&root_folder);

        let mut processed_count = 0;
        let mut acousticbrainz_count = 0;
        let mut essentia_count = 0;
        let mut not_found_count = 0;

        for file in &files {
            // Construct absolute path
            let file_path = root_path.join(&file.path);
            // Load passages for this file
            let passages = crate::db::passages::load_passages_for_file(&self.db, file.guid).await?;

            for passage in passages {
                // Get recording MBID from passage_songs linking table
                let recording_mbid = match self.get_passage_recording_mbid(&passage.guid).await {
                    Ok(Some(mbid)) => mbid,
                    Ok(None) => {
                        tracing::debug!(
                            passage_id = %passage.guid,
                            "No MusicBrainz recording linked to passage, skipping"
                        );
                        processed_count += 1;
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!(
                            passage_id = %passage.guid,
                            error = %e,
                            "Failed to get recording MBID"
                        );
                        processed_count += 1;
                        continue;
                    }
                };

                // Query AcousticBrainz if client available
                if let Some(ref ab_client) = self.acousticbrainz_client {
                    match ab_client.get_flavor_vector(&recording_mbid).await {
                        Ok(flavor) => {
                            // Serialize to JSON
                            match flavor.to_json() {
                                Ok(flavor_json) => {
                                    // Store in database
                                    if let Err(e) = crate::db::passages::update_passage_flavor(
                                        &self.db,
                                        passage.guid,
                                        flavor_json,
                                    ).await {
                                        tracing::warn!(
                                            passage_id = %passage.guid,
                                            error = %e,
                                            "Failed to save flavor vector"
                                        );
                                    } else {
                                        acousticbrainz_count += 1;
                                        tracing::debug!(
                                            passage_id = %passage.guid,
                                            recording_mbid = %recording_mbid,
                                            key = ?flavor.key,
                                            bpm = ?flavor.bpm,
                                            "Musical flavor extracted from AcousticBrainz"
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        passage_id = %passage.guid,
                                        error = %e,
                                        "Failed to serialize flavor vector"
                                    );
                                }
                            }
                        }
                        Err(crate::services::ABError::RecordingNotFound(_)) => {
                            // Try Essentia fallback for local analysis
                            if let Some(ref essentia) = self.essentia_client {
                                tracing::debug!(
                                    passage_id = %passage.guid,
                                    recording_mbid = %recording_mbid,
                                    "Recording not found in AcousticBrainz, trying Essentia fallback"
                                );

                                match essentia.analyze_file(&file_path).await {
                                    Ok(flavor) => {
                                        // Store Essentia-generated flavor vector
                                        match flavor.to_json() {
                                            Ok(flavor_json) => {
                                                if let Err(e) = crate::db::passages::update_passage_flavor(
                                                    &self.db,
                                                    passage.guid,
                                                    flavor_json,
                                                ).await {
                                                    tracing::warn!(
                                                        passage_id = %passage.guid,
                                                        error = %e,
                                                        "Failed to save Essentia flavor vector"
                                                    );
                                                } else {
                                                    essentia_count += 1;
                                                    tracing::info!(
                                                        passage_id = %passage.guid,
                                                        key = ?flavor.key,
                                                        bpm = ?flavor.bpm,
                                                        "Musical flavor extracted via Essentia fallback"
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                tracing::warn!(
                                                    passage_id = %passage.guid,
                                                    error = %e,
                                                    "Failed to serialize Essentia flavor vector"
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        not_found_count += 1;
                                        tracing::warn!(
                                            passage_id = %passage.guid,
                                            file = %file.path,
                                            error = %e,
                                            "Essentia analysis failed, no flavor data available"
                                        );
                                    }
                                }
                            } else {
                                not_found_count += 1;
                                tracing::debug!(
                                    passage_id = %passage.guid,
                                    recording_mbid = %recording_mbid,
                                    "Recording not found in AcousticBrainz and Essentia not available"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                passage_id = %passage.guid,
                                recording_mbid = %recording_mbid,
                                error = %e,
                                "AcousticBrainz lookup failed"
                            );
                        }
                    }
                } else {
                    tracing::warn!("AcousticBrainz client not available");
                }

                processed_count += 1;

                // Update progress for every passage processed
                session.progress.current_file = Some(file.path.clone());
                session.update_progress(
                    processed_count,
                    processed_count, // Use processed as total since we don't know passage count upfront
                    format!(
                        "Extracting musical flavor for passage {} (AB: {}, Essentia: {}, unavailable: {})",
                        processed_count, acousticbrainz_count, essentia_count, not_found_count
                    ),
                );
                crate::db::sessions::save_session(&self.db, &session).await?;
                self.broadcast_progress(&session, start_time);
            }
        }

        // Final progress update
        tracing::info!(
            session_id = %session.session_id,
            processed = processed_count,
            acousticbrainz = acousticbrainz_count,
            essentia = essentia_count,
            not_found = not_found_count,
            "Phase 6: FLAVORING completed"
        );

        session.update_progress(
            processed_count,
            processed_count,
            format!(
                "Flavor extraction: {} AcousticBrainz, {} Essentia, {} unavailable",
                acousticbrainz_count, essentia_count, not_found_count
            ),
        );
        crate::db::sessions::save_session(&self.db, &session).await?;

        Ok(session)
    }

    // ============================================================================
    // HELPER FUNCTIONS
    // ============================================================================

    /// Get recording MBID for a passage via passage_songs linking table
    async fn get_passage_recording_mbid(&self, passage_id: &uuid::Uuid) -> Result<Option<String>> {
        let row = sqlx::query(
            r#"
            SELECT s.recording_mbid
            FROM passage_songs ps
            INNER JOIN songs s ON ps.song_id = s.guid
            WHERE ps.passage_id = ?
            LIMIT 1
            "#,
        )
        .bind(passage_id.to_string())
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(row) => {
                let mbid: String = row.get("recording_mbid");
                Ok(Some(mbid))
            }
            None => Ok(None),
        }
    }

    // ============================================================================
    // PLAN024: Pipeline Integration
    // ============================================================================

    /// Phase 2 (PLAN024): PROCESSING - 3-tier hybrid fusion pipeline
    ///
    /// Replaces legacy EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING
    /// with unified per-file pipeline processing.
    ///
    /// **Architecture:**
    /// - Per file: Detect boundaries → process each passage through 3-tier pipeline
    /// - Tier 1: Extraction (7 extractors in parallel)
    /// - Tier 2: Fusion (3 fusers - identity, metadata, flavor)
    /// - Tier 3: Validation (3 validators - consistency, completeness, quality)
    async fn phase_processing_plan024(
        &self,
        mut session: ImportSession,
        start_time: std::time::Instant,
        cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<ImportSession> {
        use crate::workflow::{Pipeline, PipelineConfig};
        use tokio::sync::mpsc;

        session.transition_to(ImportState::Processing);
        session.update_progress(0, 0, "Initializing PLAN024 pipeline...".to_string());
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(
            session_id = %session.session_id,
            "Phase 2 (PLAN024): PROCESSING - 3-tier hybrid fusion pipeline"
        );

        // Load AcoustID API key from database (for pipeline configuration)
        let acoustid_api_key = match crate::db::settings::get_acoustid_api_key(&self.db).await {
            Ok(key) => key,
            Err(e) => {
                tracing::warn!("Failed to load AcoustID API key: {}", e);
                None
            }
        };

        // Create event channel for pipeline SSE broadcasting
        let (event_tx, mut event_rx) = mpsc::channel(100);

        // Configure PLAN024 pipeline
        let pipeline_config = PipelineConfig {
            acoustid_api_key: acoustid_api_key.clone(),
            enable_musicbrainz: true,
            enable_essentia: self.essentia_client.is_some(),
            enable_audio_derived: true,
            min_quality_threshold: 0.5, // Default minimum quality
        };

        let pipeline = Pipeline::with_events(pipeline_config, event_tx);

        // Spawn task to bridge pipeline events to EventBus (SSE)
        let _event_bus = self.event_bus.clone();
        let session_id = session.session_id;
        tokio::spawn(async move {
            use crate::workflow::WorkflowEvent;

            while let Some(event) = event_rx.recv().await {
                // Convert workflow events to WkmpEvent for SSE broadcasting
                match event {
                    WorkflowEvent::FileStarted { file_path, timestamp: _ } => {
                        tracing::debug!(session_id = %session_id, file = %file_path, "File processing started");
                    }
                    WorkflowEvent::PassageCompleted { passage_index, quality_score, validation_status } => {
                        tracing::debug!(
                            session_id = %session_id,
                            passage_index,
                            quality_score,
                            validation_status,
                            "Passage completed"
                        );
                    }
                    WorkflowEvent::FileCompleted { file_path, passages_processed, timestamp: _ } => {
                        tracing::info!(
                            session_id = %session_id,
                            file = %file_path,
                            passages = passages_processed,
                            "File processing completed"
                        );
                    }
                    WorkflowEvent::Error { passage_index, message } => {
                        tracing::warn!(
                            session_id = %session_id,
                            passage_index = ?passage_index,
                            error = %message,
                            "Pipeline error"
                        );
                    }
                    _ => {
                        // Other events (BoundaryDetected, ExtractionProgress, etc.)
                        tracing::trace!(session_id = %session_id, event = ?event, "Pipeline event");
                    }
                }
            }
        });

        // Load all files from database
        let files = crate::db::files::load_all_files(&self.db).await?;
        let total_files = files.len();

        tracing::info!(
            session_id = %session.session_id,
            file_count = total_files,
            "Processing files through PLAN024 pipeline"
        );

        session.update_progress(
            0,
            total_files,
            format!("Processing {} files through hybrid fusion pipeline", total_files),
        );
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        let mut files_processed = 0;
        let import_session_id = session.session_id.to_string();

        for file in &files {
            // Check cancellation
            if cancel_token.is_cancelled() {
                tracing::info!(
                    session_id = %session.session_id,
                    files_processed,
                    "Import cancelled during processing phase"
                );
                session.transition_to(ImportState::Cancelled);
                session.update_progress(
                    files_processed,
                    total_files,
                    "Import cancelled by user".to_string(),
                );
                crate::db::sessions::save_session(&self.db, &session).await?;
                return Ok(session);
            }

            let file_path_str = file.path.clone();
            session.progress.current_file = Some(file_path_str.clone());
            session.update_progress(
                files_processed,
                total_files,
                format!("Processing file {} of {}: {}", files_processed + 1, total_files, file_path_str),
            );
            crate::db::sessions::save_session(&self.db, &session).await?;
            self.broadcast_progress(&session, start_time);

            // Process file through PLAN024 pipeline
            let file_path = std::path::Path::new(&file_path_str);
            match pipeline.process_file(file_path).await {
                Ok(processed_passages) => {
                    tracing::info!(
                        session_id = %session.session_id,
                        file = %file_path_str,
                        passages = processed_passages.len(),
                        "Pipeline processing completed successfully"
                    );

                    // Store all passages to database
                    match crate::workflow::storage::store_passages_batch(
                        &self.db,
                        &file_path_str,
                        &processed_passages,
                        &import_session_id,
                    )
                    .await
                    {
                        Ok(passage_ids) => {
                            tracing::info!(
                                session_id = %session.session_id,
                                file = %file_path_str,
                                passages_stored = passage_ids.len(),
                                "Passages stored to database successfully"
                            );

                            // Link passages to songs/artists/albums based on fused identity
                            for (passage_id_str, processed_passage) in passage_ids.iter().zip(&processed_passages) {
                                let passage_guid = match Uuid::parse_str(passage_id_str) {
                                    Ok(guid) => guid,
                                    Err(e) => {
                                        tracing::error!(
                                            session_id = %session.session_id,
                                            passage_id = %passage_id_str,
                                            error = %e,
                                            "Invalid passage GUID, skipping linking"
                                        );
                                        continue;
                                    }
                                };

                                // Extract recording MBID from fusion results
                                let recording_mbid = if let Some(ref mbid_cv) = processed_passage.fusion.metadata.recording_mbid {
                                    mbid_cv.value.clone()
                                } else {
                                    // No MBID - cannot link to song
                                    tracing::debug!(
                                        session_id = %session.session_id,
                                        passage_id = %passage_id_str,
                                        "No recording MBID found, skipping song linking"
                                    );
                                    continue;
                                };

                                // Look up or create song
                                let song = match crate::db::songs::load_song_by_mbid(&self.db, &recording_mbid).await {
                                    Ok(Some(existing_song)) => {
                                        tracing::debug!(
                                            session_id = %session.session_id,
                                            song_id = %existing_song.guid,
                                            recording_mbid = %recording_mbid,
                                            "Found existing song"
                                        );
                                        existing_song
                                    }
                                    Ok(None) => {
                                        // Create new song from fusion results
                                        let title = processed_passage.fusion.metadata.title.as_ref().map(|cv| cv.value.clone());
                                        let new_song = crate::db::songs::Song::new(recording_mbid.clone(), title.clone());

                                        if let Err(e) = crate::db::songs::save_song(&self.db, &new_song).await {
                                            tracing::error!(
                                                session_id = %session.session_id,
                                                recording_mbid = %recording_mbid,
                                                error = %e,
                                                "Failed to create new song, skipping"
                                            );
                                            continue;
                                        }

                                        tracing::info!(
                                            session_id = %session.session_id,
                                            song_id = %new_song.guid,
                                            title = ?title,
                                            recording_mbid = %recording_mbid,
                                            "Created new song"
                                        );

                                        new_song
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            session_id = %session.session_id,
                                            recording_mbid = %recording_mbid,
                                            error = %e,
                                            "Failed to query for existing song, skipping"
                                        );
                                        continue;
                                    }
                                };

                                // Link passage to song
                                if let Err(e) = crate::db::songs::link_passage_to_song(
                                    &self.db,
                                    passage_guid,
                                    song.guid,
                                    processed_passage.boundary.start_time,
                                    processed_passage.boundary.end_time,
                                )
                                .await
                                {
                                    tracing::error!(
                                        session_id = %session.session_id,
                                        passage_id = %passage_id_str,
                                        song_id = %song.guid,
                                        error = %e,
                                        "Failed to link passage to song"
                                    );
                                } else {
                                    tracing::debug!(
                                        session_id = %session.session_id,
                                        passage_id = %passage_id_str,
                                        song_id = %song.guid,
                                        "Linked passage to song"
                                    );
                                }

                                // Link to artist(s) if available
                                if let Some(ref artist_cv) = processed_passage.fusion.metadata.artist {
                                    let artist_name = &artist_cv.value;

                                    // Extract artist MBID from fusion metadata (MusicBrainz extractor)
                                    let artist_mbid = if let Some(mbid_cv) = processed_passage.fusion.metadata.additional.get("artist_mbid") {
                                        // Single artist - use actual MBID from MusicBrainz
                                        mbid_cv.value.clone()
                                    } else if let Some(mbids_cv) = processed_passage.fusion.metadata.additional.get("artist_mbids") {
                                        // Multiple artists - use first MBID (primary artist)
                                        mbids_cv.value.split(',').next().unwrap_or("").to_string()
                                    } else {
                                        // No MusicBrainz MBID available - fall back to name-based ID
                                        format!("name:{}", artist_name)
                                    };

                                    match crate::db::artists::load_artist_by_mbid(&self.db, &artist_mbid).await {
                                        Ok(Some(existing_artist)) => {
                                            // Link song to existing artist
                                            if let Err(e) = crate::db::artists::link_song_to_artist(
                                                &self.db,
                                                song.guid,
                                                existing_artist.guid,
                                                1.0, // Full weight (single artist)
                                            )
                                            .await
                                            {
                                                tracing::error!(
                                                    session_id = %session.session_id,
                                                    song_id = %song.guid,
                                                    artist_id = %existing_artist.guid,
                                                    error = %e,
                                                    "Failed to link song to artist"
                                                );
                                            }
                                        }
                                        Ok(None) => {
                                            // Create new artist
                                            let new_artist = crate::db::artists::Artist::new(artist_mbid.clone(), artist_name.clone());

                                            if let Err(e) = crate::db::artists::save_artist(&self.db, &new_artist).await {
                                                tracing::error!(
                                                    session_id = %session.session_id,
                                                    artist_name = %artist_name,
                                                    error = %e,
                                                    "Failed to create artist"
                                                );
                                            } else {
                                                // Link song to new artist
                                                if let Err(e) = crate::db::artists::link_song_to_artist(
                                                    &self.db,
                                                    song.guid,
                                                    new_artist.guid,
                                                    1.0,
                                                )
                                                .await
                                                {
                                                    tracing::error!(
                                                        session_id = %session.session_id,
                                                        song_id = %song.guid,
                                                        artist_id = %new_artist.guid,
                                                        error = %e,
                                                        "Failed to link song to new artist"
                                                    );
                                                } else {
                                                    tracing::debug!(
                                                        session_id = %session.session_id,
                                                        song_id = %song.guid,
                                                        artist_name = %artist_name,
                                                        "Created and linked new artist"
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                session_id = %session.session_id,
                                                artist_name = %artist_name,
                                                error = %e,
                                                "Failed to query for existing artist"
                                            );
                                        }
                                    }
                                }

                                // Link to album if available
                                if let Some(ref album_cv) = processed_passage.fusion.metadata.album {
                                    let album_title = &album_cv.value;

                                    // Extract release MBID from fusion metadata (MusicBrainz extractor)
                                    let album_mbid = if let Some(mbid_cv) = processed_passage.fusion.metadata.additional.get("release_mbid") {
                                        // Use actual release MBID from MusicBrainz
                                        mbid_cv.value.clone()
                                    } else {
                                        // No MusicBrainz MBID available - fall back to title-based ID
                                        format!("title:{}", album_title)
                                    };

                                    match crate::db::albums::load_album_by_mbid(&self.db, &album_mbid).await {
                                        Ok(Some(existing_album)) => {
                                            // Link passage to existing album
                                            if let Err(e) = crate::db::albums::link_passage_to_album(
                                                &self.db,
                                                passage_guid,
                                                existing_album.guid,
                                            )
                                            .await
                                            {
                                                tracing::error!(
                                                    session_id = %session.session_id,
                                                    passage_id = %passage_id_str,
                                                    album_id = %existing_album.guid,
                                                    error = %e,
                                                    "Failed to link passage to album"
                                                );
                                            }
                                        }
                                        Ok(None) => {
                                            // Create new album
                                            let new_album = crate::db::albums::Album::new(album_mbid.clone(), album_title.clone());

                                            if let Err(e) = crate::db::albums::save_album(&self.db, &new_album).await {
                                                tracing::error!(
                                                    session_id = %session.session_id,
                                                    album_title = %album_title,
                                                    error = %e,
                                                    "Failed to create album"
                                                );
                                            } else {
                                                // Link passage to new album
                                                if let Err(e) = crate::db::albums::link_passage_to_album(
                                                    &self.db,
                                                    passage_guid,
                                                    new_album.guid,
                                                )
                                                .await
                                                {
                                                    tracing::error!(
                                                        session_id = %session.session_id,
                                                        passage_id = %passage_id_str,
                                                        album_id = %new_album.guid,
                                                        error = %e,
                                                        "Failed to link passage to new album"
                                                    );
                                                } else {
                                                    tracing::debug!(
                                                        session_id = %session.session_id,
                                                        passage_id = %passage_id_str,
                                                        album_title = %album_title,
                                                        "Created and linked new album"
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                session_id = %session.session_id,
                                                album_title = %album_title,
                                                error = %e,
                                                "Failed to query for existing album"
                                            );
                                        }
                                    }
                                }
                            }

                            tracing::info!(
                                session_id = %session.session_id,
                                file = %file_path_str,
                                passages = passage_ids.len(),
                                "Completed passage-to-song/artist/album linking"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                session_id = %session.session_id,
                                file = %file_path_str,
                                error = %e,
                                "Failed to store passages to database"
                            );
                            // Continue processing other files (per-file error isolation)
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        session_id = %session.session_id,
                        file = %file_path_str,
                        error = %e,
                        "Pipeline processing failed for file"
                    );
                    // Continue processing other files (per-file error isolation)
                }
            }

            files_processed += 1;
        }

        // Final progress update
        session.update_progress(
            files_processed,
            total_files,
            format!("PLAN024 pipeline completed - {} files processed", files_processed),
        );
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(
            session_id = %session.session_id,
            files_processed,
            "PLAN024 processing phase completed"
        );

        Ok(session)
    }

    /// Handle workflow failure
    pub async fn handle_failure(
        &self,
        mut session: ImportSession,
        error: &anyhow::Error,
    ) -> Result<ImportSession> {
        tracing::error!(
            session_id = %session.session_id,
            error = %error,
            "Import workflow failed"
        );

        session.transition_to(ImportState::Failed);
        session.update_progress(
            session.progress.current,
            session.progress.total,
            format!("Import failed: {}", error),
        );

        crate::db::sessions::save_session(&self.db, &session).await?;

        // Broadcast failure event
        self.event_bus.emit_lossy(WkmpEvent::ImportSessionFailed {
            session_id: session.session_id,
            error_message: error.to_string(),
            files_processed: session.progress.current,
            timestamp: Utc::now(),
        });

        Ok(session)
    }

    /// Broadcast progress update event
    ///
    /// **[AIA-MS-010]** SSE event streaming
    fn broadcast_progress(&self, session: &ImportSession, start_time: std::time::Instant) {
        let elapsed_seconds = start_time.elapsed().as_secs();

        self.event_bus.emit_lossy(WkmpEvent::ImportProgressUpdate {
            session_id: session.session_id,
            state: format!("{:?}", session.state),
            current: session.progress.current,
            total: session.progress.total,
            percentage: session.progress.percentage as f32,
            current_operation: session.progress.current_operation.clone(),
            elapsed_seconds,
            estimated_remaining_seconds: session.progress.estimated_remaining_seconds,
            // **[REQ-AIA-UI-001]** Convert phase tracking to event data
            phases: session.progress.phases.iter().map(|p| p.into()).collect(),
            // **[REQ-AIA-UI-004]** Include current file being processed
            current_file: session.progress.current_file.clone(),
            timestamp: Utc::now(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        // Would need a database pool for real test
        // This is a placeholder
        assert!(true);
    }
}

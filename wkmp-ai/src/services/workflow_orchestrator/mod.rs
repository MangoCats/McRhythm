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
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;
use wkmp_common::events::{EventBus, WkmpEvent};

// Phase modules (internal implementation)
mod phase_scanning;
mod phase_extraction;
mod phase_fingerprinting;
mod phase_segmenting;
mod phase_analyzing;
mod phase_flavoring;

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
    /// Create new workflow orchestrator
    ///
    /// # Arguments
    /// * `db` - Database connection pool
    /// * `event_bus` - Event bus for progress updates
    /// * `acoustid_api_key` - Optional AcoustID API key for fingerprinting
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
                            tracing::error!("Failed to initialize AcoustID client: {:?}", e);
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
                                            error = ?e,
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
                                                error = ?e,
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
                                            error = ?e,
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
                                        error = ?e,
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
                                                    error = ?e,
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
                                                    error = ?e,
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
                                                        error = ?e,
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
                                                error = ?e,
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
                                                    error = ?e,
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
                                                    error = ?e,
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
                                                        error = ?e,
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
                                                error = ?e,
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
                                error = ?e,
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
                        error = ?e,  // Debug format shows full error chain
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
            error = ?error,
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

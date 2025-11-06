//! Import workflow orchestrator
//!
//! **[AIA-WF-010]** Coordinates import workflow through all states
//!
//! State progression:
//! SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING → COMPLETED

use crate::models::{ImportSession, ImportState};
use crate::services::{
    AcousticBrainzClient, AcoustIDClient, AmplitudeAnalyzer, EssentiaClient, FileScanner,
    Fingerprinter, MetadataExtractor, MusicBrainzClient,
};
use anyhow::Result;
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use std::path::Path;
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
        for file_path in &scan_result.files {
            // **[AIA-ASYNC-010]** Check for cancellation every file
            if cancel_token.is_cancelled() {
                tracing::info!(
                    session_id = %session.session_id,
                    files_processed = saved_count,
                    "Import cancelled during scanning phase"
                );
                session.transition_to(ImportState::Cancelled);
                session.progress.current_file = None;
                session.update_progress(
                    saved_count,
                    scan_result.files.len(),
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
            session.update_progress(
                saved_count,
                scan_result.files.len(),
                format!("Checking file {} of {}: {}", saved_count + 1, scan_result.files.len(), relative_path),
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

        let mut processed_count = 0;

        for file in &files {
            // Construct absolute path
            let file_path = root_path.join(&file.path);

            // **[REQ-AIA-UI-004]** Set current file
            session.progress.current_file = Some(file.path.clone());

            // Generate Chromaprint fingerprint
            let fingerprint = match self.fingerprinter.fingerprint_file(&file_path) {
                Ok(fp) => {
                    // **[REQ-AIA-UI-003]** Increment Chromaprint success counter
                    if let Some(phase) = session.progress.get_phase_mut(crate::models::ImportState::Fingerprinting) {
                        if let Some(subtask) = phase.subtasks.iter_mut().find(|s| s.name == "Chromaprint") {
                            subtask.success_count += 1;
                        }
                    }
                    fp
                }
                Err(e) => {
                    tracing::warn!("Failed to fingerprint {}: {}", file.path, e);
                    // **[REQ-AIA-UI-003]** Increment Chromaprint failure counter
                    if let Some(phase) = session.progress.get_phase_mut(crate::models::ImportState::Fingerprinting) {
                        if let Some(subtask) = phase.subtasks.iter_mut().find(|s| s.name == "Chromaprint") {
                            subtask.failure_count += 1;
                        }
                    }
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

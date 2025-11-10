//! Phase 3: FINGERPRINTING
//!
//! Audio fingerprinting with parallel Chromaprint generation and sequential API lookups

use super::WorkflowOrchestrator;
use crate::models::{ImportSession, ImportState};
use anyhow::Result;
use rayon::prelude::*;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use wkmp_common::events::WkmpEvent;

impl WorkflowOrchestrator {
    /// Phase 3: FINGERPRINTING - Audio fingerprinting (stub)
    pub(super) async fn phase_fingerprinting(
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
        let fingerprinter = self.fingerprinter;
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
}

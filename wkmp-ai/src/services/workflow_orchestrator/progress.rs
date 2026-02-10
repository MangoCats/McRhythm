//! Progress broadcasting and worker activity tracking
//!
//! Extracted from mod.rs for maintainability.
//! Contains SSE event broadcasting, phase statistics conversion,
//! and real-time worker activity tracking for UI display.

use crate::models::{ImportSession, ImportState};
use chrono::Utc;
use wkmp_common::events::{WkmpEvent, WorkerActivity};

use super::WorkflowOrchestrator;

impl WorkflowOrchestrator {
    /// Handle workflow failure
    pub async fn handle_failure(
        &self,
        mut session: ImportSession,
        error: &anyhow::Error,
    ) -> anyhow::Result<ImportSession> {
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
    pub(super) fn broadcast_progress(&self, session: &ImportSession, start_time: std::time::Instant) {
        self.broadcast_progress_with_stats(session, start_time, vec![]);
    }

    /// **[PLAN024]** SSE event streaming with phase-specific statistics
    pub(super) fn broadcast_progress_with_stats(
        &self,
        session: &ImportSession,
        start_time: std::time::Instant,
        phase_statistics: Vec<wkmp_common::events::PhaseStatistics>,
    ) {
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
            // **[PLAN024]** Phase-specific statistics per wkmp-ai_refinement.md
            phase_statistics,
            timestamp: Utc::now(),
        });
    }

    /// **[PLAN024]** Convert ImportStatistics to PhaseStatistics for SSE events
    /// **[PLAN031 Task 2.5]** Made async for tokio::sync lock compatibility
    pub(super) async fn convert_statistics_to_sse(&self) -> Vec<wkmp_common::events::PhaseStatistics> {
        use wkmp_common::events::PhaseStatistics;

        // **[PLAN031 Task 2.5]** Extract statistics data in block scope to drop guards before await
        tracing::debug!("Acquiring all statistics Mutex locks for SSE conversion");
        let (
            scanning,
            processing,
            filename_matching,
            hashing,
            extracting,
            segmenting,
            fingerprinting,
            song_matching,
            recording,
            amplitude,
            flavoring,
            passages_complete,
            files_complete,
        ) = {
            let s1 = self.statistics.scanning.lock().unwrap().clone();
            let s2 = self.statistics.processing.lock().unwrap().clone();
            let s3 = self.statistics.filename_matching.lock().unwrap().clone();
            let s4 = self.statistics.hashing.lock().unwrap().clone();
            let s5 = self.statistics.extracting.lock().unwrap().clone();
            let s6 = self.statistics.segmenting.lock().unwrap().clone();
            let s7 = self.statistics.fingerprinting.lock().unwrap().clone();
            let s8 = self.statistics.song_matching.lock().unwrap().clone();
            let s9 = self.statistics.recording.lock().unwrap().clone();
            let s10 = self.statistics.amplitude.lock().unwrap().clone();
            let s11 = self.statistics.flavoring.lock().unwrap().clone();
            let s12 = self.statistics.passages_complete.lock().unwrap().clone();
            let s13 = self.statistics.files_complete.lock().unwrap().clone();
            tracing::debug!("All statistics Mutex locks acquired and data cloned");
            (s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13)
        }; // Guards dropped here

        // **[AIA-UI-010]** Get current worker activities with elapsed time calculation
        // **[PLAN031 Task 2.5]** Use async read lock (safe to await after guards dropped)
        let worker_activities: Vec<WorkerActivity> = self
            .worker_activities
            .read()
            .await
            .values()
            .map(|activity| {
                let mut activity = activity.clone();
                // Calculate elapsed_ms from phase_started_at
                if let Some(started_at) = activity.phase_started_at {
                    activity.elapsed_ms = Some((Utc::now() - started_at).num_milliseconds() as u64);
                }
                activity
            })
            .collect();

        tracing::trace!(
            worker_count = worker_activities.len(),
            "Worker activities collected for SSE"
        );

        // **[File Processing Status]** Get current file processing states
        let mut file_statuses: Vec<wkmp_common::events::FileProcessingStatus> = self
            .file_processing_states
            .read()
            .await
            .values()
            .cloned()
            .collect();

        // Sort by file_index for display
        file_statuses.sort_by_key(|f| f.file_index);

        let result = vec![
            PhaseStatistics::Scanning {
                potential_files_found: scanning.potential_files_found,
                is_scanning: scanning.is_scanning,
                audio_files: scanning.audio_files,
                image_files: scanning.image_files,
                other_files: scanning.other_files,
                total_files: scanning.total_files,
                magic_byte_analyzed: scanning.magic_byte_analyzed,
                audio_confirmed: scanning.audio_confirmed,
                image_confirmed: scanning.image_confirmed,
                other_confirmed: scanning.other_confirmed,
                audio_unrecognized_ext: scanning.audio_unrecognized_ext,
                image_unrecognized_ext: scanning.image_unrecognized_ext,
                misleading_extension: scanning.misleading_extension,
            },
            PhaseStatistics::Processing {
                completed: processing.completed,
                started: processing.started,
                total: processing.total,
                workers: worker_activities,
                max_workers: *self.max_workers.read().await,
                files: file_statuses,
            },
            PhaseStatistics::FilenameMatching {
                completed_filenames_found: filename_matching.completed_filenames_found,
            },
            PhaseStatistics::Hashing {
                hashes_computed: hashing.hashes_computed,
                matches_found: hashing.matches_found,
            },
            PhaseStatistics::Extracting {
                successful_extractions: extracting.successful_extractions,
                failures: extracting.failures,
            },
            PhaseStatistics::Segmenting {
                files_processed: segmenting.files_processed,
                potential_passages: segmenting.potential_passages,
                finalized_passages: segmenting.finalized_passages,
                songs_identified: segmenting.songs_identified,
            },
            PhaseStatistics::Fingerprinting {
                passages_fingerprinted: fingerprinting.passages_fingerprinted,
                successful_matches: fingerprinting.successful_matches,
            },
            PhaseStatistics::SongMatching {
                high_confidence: song_matching.high_confidence,
                medium_confidence: song_matching.medium_confidence,
                low_confidence: song_matching.low_confidence,
                no_confidence: song_matching.no_confidence,
            },
            PhaseStatistics::Recording {
                recorded_passages: recording.recorded_passages.clone(),
            },
            PhaseStatistics::Amplitude {
                analyzed_passages: amplitude.analyzed_passages.clone(),
            },
            PhaseStatistics::Flavoring {
                pre_existing: flavoring.pre_existing,
                acousticbrainz: flavoring.acousticbrainz,
                essentia: flavoring.essentia,
                failed: flavoring.failed,
            },
            PhaseStatistics::PassagesComplete {
                passages_completed: passages_complete.passages_completed,
            },
            PhaseStatistics::FilesComplete {
                files_completed: files_complete.files_completed,
            },
        ];

        // Log before dropping mutex guards (which happens when function returns)
        tracing::debug!("Releasing all statistics Mutex locks for SSE conversion");
        result
    }

    /// **[AIA-UI-010]** Update worker activity (current phase)
    /// **[PLAN031 Task 2.5]** Made async for tokio::sync::RwLock
    pub(super) async fn set_worker_phase(
        &self,
        file_path: &std::path::Path,
        root_folder: &std::path::Path,
        file_index: usize,
        phase_number: u8,
        phase_name: &str,
    ) {
        let worker_key = file_index.to_string();
        let relative_path = file_path
            .strip_prefix(root_folder)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| file_path.display().to_string());

        tracing::trace!(
            worker_id = %worker_key,
            file_index = file_index,
            phase_number = phase_number,
            phase_name = phase_name,
            "Setting worker phase"
        );

        let activity = WorkerActivity {
            worker_id: worker_key.clone(),
            file_path: Some(relative_path.clone()),
            file_index: Some(file_index),
            phase_number: Some(phase_number),
            phase_name: Some(phase_name.to_string()),
            phase_started_at: Some(Utc::now()),
            elapsed_ms: None,
            passage_start_seconds: None,
            passage_end_seconds: None,
        };

        // **[PLAN031 Task 2.5]** Use async write lock
        self.worker_activities
            .write()
            .await
            .insert(worker_key, activity);
    }

    /// **[AIA-UI-010]** Update worker activity with passage timing (for passage-level phases)
    /// **[PLAN031 Task 2.5]** Made async for tokio::sync::RwLock
    #[allow(dead_code)] // Scaffolded for fine-grained passage-level UI tracking
    pub(super) async fn set_worker_phase_with_passage(
        &self,
        file_path: &std::path::Path,
        root_folder: &std::path::Path,
        file_index: usize,
        phase_number: u8,
        phase_name: &str,
        passage_start_seconds: f64,
        passage_end_seconds: f64,
    ) {
        let worker_key = file_index.to_string();
        let relative_path = file_path
            .strip_prefix(root_folder)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| file_path.display().to_string());

        tracing::trace!(
            worker_id = %worker_key,
            file_index = file_index,
            phase_number = phase_number,
            phase_name = phase_name,
            passage_start = passage_start_seconds,
            passage_end = passage_end_seconds,
            "Setting worker phase with passage timing"
        );

        let activity = WorkerActivity {
            worker_id: worker_key.clone(),
            file_path: Some(relative_path.clone()),
            file_index: Some(file_index),
            phase_number: Some(phase_number),
            phase_name: Some(phase_name.to_string()),
            phase_started_at: Some(Utc::now()),
            elapsed_ms: None,
            passage_start_seconds: Some(passage_start_seconds),
            passage_end_seconds: Some(passage_end_seconds),
        };

        // **[PLAN031 Task 2.5]** Use async write lock
        self.worker_activities
            .write()
            .await
            .insert(worker_key, activity);
    }

    /// **[AIA-UI-010]** Clear worker activity (worker now idle)
    /// **[PLAN031 Task 2.5]** Made async for tokio::sync::RwLock
    pub(super) async fn clear_worker_phase(&self, file_index: usize) {
        let worker_key = file_index.to_string();
        // **[PLAN031 Task 2.5]** Use async write lock
        self.worker_activities.write().await.remove(&worker_key);
    }
}

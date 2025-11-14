//! Phase 1: SCANNING
//!
//! File system scanning for audio file discovery
//!
//! **[AIA-WF-010]** SCANNING phase discovers audio files and creates basic file records
//!
//! **PLAN024 Architecture:**
//! - SCANNING: File discovery only (path, modification time, session linkage)
//! - PROCESSING: Per-file pipeline handles hashing, metadata, segmentation, etc.
//!
//! **Legacy Architecture (Deprecated):**
//! - SCANNING: File discovery + batch hashing + batch metadata extraction
//! - EXTRACTING/FINGERPRINTING/etc.: Separate batch phases
//!
//! This module implements the PLAN024 approach: minimal file discovery only.

use super::WorkflowOrchestrator;
use crate::models::{ImportSession, ImportState};
use anyhow::Result;
use chrono::Utc;
use std::path::Path;

impl WorkflowOrchestrator {
    /// Phase 1: SCANNING - Discover audio files and create basic file records
    ///
    /// **[AIA-WF-010]** Filesystem traversal
    /// **[AIA-ASYNC-010]** Respects cancellation token
    ///
    /// **PLAN024 Approach:**
    /// - Scans filesystem for audio files
    /// - Creates file records with: path, modification_time, session_id
    /// - Does NOT extract metadata, hash files, or do any processing
    /// - Processing happens per-file in Phase 2 (PROCESSING)
    ///
    /// # Returns
    /// Updated session with file count in progress.total
    pub(super) async fn phase_scanning(
        &self,
        mut session: ImportSession,
        start_time: std::time::Instant,
        cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<ImportSession> {
        session.transition_to(ImportState::Scanning);
        session.update_progress(0, 0, "Scanning for audio files...".to_string());
        crate::db::sessions::save_session(&self.db, &session).await?;

        // **[PLAN024]** Set scanning to active
        {
            let mut scan_stats = self.statistics.scanning.lock().unwrap();
            scan_stats.is_scanning = true;
            scan_stats.potential_files_found = 0;
        }

        // Broadcast initial scanning state
        let phase_statistics = self.convert_statistics_to_sse();
        self.broadcast_progress_with_stats(&session, start_time, phase_statistics);

        tracing::info!(session_id = %session.session_id, "Phase 1: SCANNING (file discovery only)");

        // Scan filesystem for audio files
        let scan_result = self
            .file_scanner
            .scan_with_stats_and_progress(
                Path::new(&session.root_folder),
                |file_count| {
                    // **[PLAN024]** Update scanning statistics during scan
                    {
                        let mut scan_stats = self.statistics.scanning.lock().unwrap();
                        scan_stats.potential_files_found = file_count;
                    }

                    tracing::debug!(
                        session_id = %session.session_id,
                        files_found = file_count,
                        "File discovery progress"
                    );
                },
            )?;

        tracing::info!(
            session_id = %session.session_id,
            files_found = scan_result.files.len(),
            total_size_mb = scan_result.total_size / 1_000_000,
            "File scan completed"
        );

        // Create basic file records in database
        // NOTE: We only store path and modification time here
        // Hashing, metadata extraction, etc. happens in per-file pipeline
        let root_path = Path::new(&session.root_folder);
        let mut file_records = Vec::new();

        for file_path in &scan_result.files {
            // Check cancellation
            if cancel_token.is_cancelled() {
                tracing::info!(
                    session_id = %session.session_id,
                    files_created = file_records.len(),
                    "Import cancelled during file record creation"
                );
                session.transition_to(ImportState::Cancelled);
                session.update_progress(
                    file_records.len(),
                    scan_result.files.len(),
                    "Import cancelled by user".to_string(),
                );
                crate::db::sessions::save_session(&self.db, &session).await?;
                return Ok(session);
            }

            // Get file metadata (modification time)
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

            let mod_time = match metadata.modified() {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!(
                        session_id = %session.session_id,
                        file = %file_path.display(),
                        error = %e,
                        "Failed to get modification time, skipping"
                    );
                    continue;
                }
            };
            let mod_time_utc = chrono::DateTime::<Utc>::from(mod_time);

            // Create relative path
            let relative_path = file_path
                .strip_prefix(root_path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();

            // Create minimal file record (no hash, no metadata yet)
            // Hash and metadata will be computed in per-file pipeline
            // NOTE: No session_id - files table doesn't track sessions per SPEC031 zero-conf
            let mut audio_file = crate::db::files::AudioFile::new(
                relative_path,
                String::new(), // Hash will be computed in Phase 2
                mod_time_utc,
            );

            // Set file size (other fields will be populated in per-file pipeline)
            audio_file.file_size_bytes = Some(metadata.len() as i64);

            file_records.push(audio_file);
        }

        // Batch save file records to database
        if !file_records.is_empty() {
            crate::db::files::save_files_batch(&self.db, &file_records).await?;
        }

        let files_found = file_records.len();

        // **[PLAN024]** Mark scanning complete
        {
            let mut scan_stats = self.statistics.scanning.lock().unwrap();
            scan_stats.is_scanning = false;
            scan_stats.potential_files_found = files_found;
        }

        // Update progress with final scan count
        session.update_progress(
            files_found,
            files_found,
            format!("{} audio files found", files_found),
        );
        session.progress.total = files_found; // Set total for PROCESSING phase
        crate::db::sessions::save_session(&self.db, &session).await?;

        // Broadcast final scanning state
        let phase_statistics = self.convert_statistics_to_sse();
        self.broadcast_progress_with_stats(&session, start_time, phase_statistics);

        tracing::info!(
            session_id = %session.session_id,
            files_found,
            "SCANNING phase complete - file records created, ready for per-file processing"
        );

        Ok(session)
    }
}

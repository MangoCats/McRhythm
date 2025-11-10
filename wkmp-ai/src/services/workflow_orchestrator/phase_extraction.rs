//! Phase 2: EXTRACTING
//!
//! ID3 metadata extraction and database persistence

use super::WorkflowOrchestrator;
use crate::models::{ImportSession, ImportState};
use anyhow::Result;
use std::path::Path;

impl WorkflowOrchestrator {
    /// Phase 2: EXTRACTING - Metadata extraction and persistence
    pub(super) async fn phase_extracting(
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
}

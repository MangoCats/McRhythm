//! Phase 4: SEGMENTING
//!
//! Passage creation and entity linking

use super::WorkflowOrchestrator;
use crate::models::{ImportSession, ImportState};
use anyhow::Result;
use uuid::Uuid;

impl WorkflowOrchestrator {
    /// Phase 4: SEGMENTING - Passage creation
    pub(super) async fn phase_segmenting(
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
}

//! Phase 6: FLAVORING
//!
//! Musical flavor extraction via AcousticBrainz and Essentia

use super::WorkflowOrchestrator;
use crate::models::{ImportSession, ImportState};
use anyhow::Result;
use sqlx::Row;
use std::path::Path;
use uuid::Uuid;

impl WorkflowOrchestrator {
    /// Phase 6: FLAVORING - Musical flavor extraction via AcousticBrainz
    pub(super) async fn phase_flavoring(
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
    async fn get_passage_recording_mbid(&self, passage_id: &Uuid) -> Result<Option<String>> {
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
}

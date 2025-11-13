//! Phase 5: ANALYZING (DEPRECATED)
//!
//! **[AIA-WF-020]** DEPRECATED: Batch-phase amplitude analysis
//!
//! Replaced by PLAN024 per-file pipeline architecture where each file
//! goes through amplitude analysis as part of its individual pipeline
//! (Phase 8 in the 10-phase per-file sequence)

use super::WorkflowOrchestrator;
use crate::models::{ImportSession, ImportState};
use anyhow::Result;
use std::path::Path;

impl WorkflowOrchestrator {
    /// Phase 5: ANALYZING - Amplitude analysis
    ///
    /// **DEPRECATED:** Use `phase_processing_per_file()` instead
    ///
    /// **[AIA-WF-020]** Batch phases DEPRECATED as of PLAN024
    #[deprecated(since = "0.1.0", note = "Use phase_processing_per_file() with per-file pipeline")]
    pub(super) async fn phase_analyzing(
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
}

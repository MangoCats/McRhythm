//! PLAN025 segmentation-first pipeline (DEPRECATED)
//!
//! Extracted from mod.rs for maintainability.
//! Contains the deprecated batch-phase processing pipeline.

use crate::models::{ImportSession, ImportState};
use crate::services::{
    AcoustIDClient, AcousticBrainzClient, ProgressManager, WriteQueue,
};
use anyhow::Result;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;
use wkmp_common::events::EventBus;

use super::{SegmentBoundary, WorkflowOrchestrator};

#[allow(deprecated)]
impl WorkflowOrchestrator {
    /// - Tier 1: Extraction (7 extractors in parallel)
    /// - Tier 2: Fusion (3 fusers - identity, metadata, flavor)
    /// - Tier 3: Validation (3 validators - consistency, completeness, quality)
    /// Phase 2: PROCESSING - PLAN024 3-tier hybrid fusion pipeline
    ///
    /// **DEPRECATED:** Use `phase_processing_per_file()` instead
    ///
    /// **[AIA-WF-020]** Batch-phase processing DEPRECATED as of corrective implementation
    #[deprecated(
        since = "0.1.0",
        note = "Use phase_processing_per_file() with per-file pipeline"
    )]
    pub(super) async fn phase_processing_plan025(
        &self,
        mut session: ImportSession,
        start_time: std::time::Instant,
        cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<ImportSession> {
        use futures::stream::{self, StreamExt};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        session.transition_to(ImportState::Processing);
        session.update_progress(0, 0, "Initializing PLAN025 pipeline...".to_string());
        crate::db::sessions::save_session(&self.db, &session).await?;
        self.broadcast_progress(&session, start_time);

        tracing::info!(
            session_id = %session.session_id,
            "Phase 2 (PLAN025): PROCESSING - Segmentation-first per-file pipeline with 4 workers"
        );

        // Load all files from database
        let files = crate::db::files::load_all_files(&self.db).await?;
        let total_files = files.len();

        // **[PLAN028]** Initialize performance optimization components
        tracing::debug!(
            session_id = %session.session_id,
            total_files,
            "Initializing PLAN028 ProgressManager and WriteQueue"
        );
        {
            let mut pm = self.progress_manager.lock();
            *pm = Some(ProgressManager::new(
                session.session_id,
                self.event_bus.clone(),
                self.db.clone(),
                total_files,
            ));
        }
        {
            let mut wq = self.write_queue.lock();
            *wq = Some(Arc::new(WriteQueue::new(self.db.clone())));
        }

        tracing::info!(
            session_id = %session.session_id,
            file_count = total_files,
            "Processing files through PLAN025 pipeline (4 parallel workers)"
        );

        session.update_progress(
            0,
            total_files,
            format!(
                "Processing {} files through segmentation-first pipeline",
                total_files
            ),
        );
        // **[PLAN028]** Use ProgressManager instead of direct database write + SSE broadcast
        // Clone to avoid holding lock across await
        let pm_clone = self.progress_manager.lock().clone();
        if let Some(pm) = pm_clone {
            pm.update_progress(
                0,
                format!(
                    "Processing {} files through segmentation-first pipeline",
                    total_files
                ),
            )
            .await?;
        }

        // Thread-safe progress counter
        let files_processed = Arc::new(AtomicUsize::new(0));
        let files_processed_clone = files_processed.clone();

        // Clone data for workers
        let db = self.db.clone();
        let event_bus = self.event_bus.clone();
        let acoustid_client = self.acoustid_client.clone();
        let acousticbrainz_client = self.acousticbrainz_client.clone();
        let session_id = session.session_id;
        let root_folder = session.root_folder.clone();

        // **[REQ-PIPE-020]** Per-file processing with 4 parallel workers
        // Using futures::stream::buffer_unordered(4) for concurrency
        let results: Vec<Result<usize>> = stream::iter(files.into_iter().enumerate())
            .map(|(index, file)| {
                let db = db.clone();
                let event_bus = event_bus.clone();
                let acoustid_client = acoustid_client.clone();
                let acousticbrainz_client = acousticbrainz_client.clone();
                let files_processed = files_processed_clone.clone();
                let cancel_token = cancel_token.clone();
                let root_folder = root_folder.clone();

                async move {
                    // Check cancellation before processing
                    if cancel_token.is_cancelled() {
                        return Ok(index);
                    }

                    let file_path = std::path::Path::new(&root_folder).join(&file.path);

                    tracing::debug!(
                        session_id = %session_id,
                        file_index = index,
                        file = %file.path,
                        "Worker starting file processing"
                    );

                    // Process file through PLAN025 pipeline
                    match Self::process_file_plan025(
                        &db,
                        &event_bus,
                        session_id,
                        &file_path,
                        &file,
                        acoustid_client.clone(),
                        acousticbrainz_client.clone(),
                    )
                    .await
                    {
                        Ok(passages_created) => {
                            tracing::info!(
                                session_id = %session_id,
                                file = %file.path,
                                passages = passages_created,
                                "File processing completed successfully"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                session_id = %session_id,
                                file = %file.path,
                                error = ?e,
                                "File processing failed"
                            );
                            // Continue processing other files (per-file error isolation)
                        }
                    }

                    // Update progress counter
                    let current = files_processed.fetch_add(1, Ordering::Relaxed) + 1;

                    if current % 10 == 0 || current == total_files {
                        tracing::info!(
                            session_id = %session_id,
                            progress = format!("{}/{}", current, total_files),
                            "Pipeline progress update"
                        );
                    }

                    Ok(index)
                }
            })
            .buffer_unordered(4) // **[REQ-PIPE-020]** 4 concurrent workers
            .collect()
            .await;

        // Check if cancelled during processing
        if cancel_token.is_cancelled() {
            let processed = files_processed.load(Ordering::Relaxed);
            tracing::info!(
                session_id = %session.session_id,
                files_processed = processed,
                "Import cancelled during PLAN025 processing phase"
            );
            session.transition_to(ImportState::Cancelled);
            session.update_progress(
                processed,
                total_files,
                "Import cancelled by user".to_string(),
            );
            // **[PLAN028]** Force sync and shutdown on cancellation
            // Clone to avoid holding lock across await
            let pm_clone = self.progress_manager.lock().clone();
            if let Some(pm) = pm_clone {
                pm.update_progress(processed, "Import cancelled by user".to_string())
                    .await?;
                pm.force_sync().await?;
                pm.shutdown();
            }
            let wq_clone = self.write_queue.lock().clone();
            if let Some(wq) = wq_clone {
                wq.shutdown().await?;
            }
            return Ok(session);
        }

        let final_count = files_processed.load(Ordering::Relaxed);
        let successful = results.iter().filter(|r| r.is_ok()).count();
        let failed = results.iter().filter(|r| r.is_err()).count();

        tracing::info!(
            session_id = %session.session_id,
            total = total_files,
            successful,
            failed,
            "PLAN025 processing phase completed"
        );

        // Final progress update
        session.update_progress(
            final_count,
            total_files,
            format!(
                "PLAN025 pipeline completed - {} files processed",
                final_count
            ),
        );

        // **[PLAN028]** Force final sync and shutdown
        // Clone to avoid holding lock across await
        let pm_clone = self.progress_manager.lock().clone();
        if let Some(pm) = pm_clone {
            pm.update_progress(
                final_count,
                format!(
                    "PLAN025 pipeline completed - {} files processed",
                    final_count
                ),
            )
            .await?;
            pm.force_sync().await?;
            pm.shutdown();
        }
        let wq_clone = self.write_queue.lock().clone();
        if let Some(wq) = wq_clone {
            wq.shutdown().await?;
        }

        // **[PLAN029]** Log pool statistics at import completion
        self.log_pool_stats();

        Ok(session)
    }

    /// Process single file through PLAN025 pipeline
    ///
    /// **[REQ-PIPE-010]** Segmentation-first sequence:
    /// Verify → Extract → Hash → **SEGMENT** → Match → Fingerprint → Identify → Amplitude → Flavor → DB
    ///
    /// # Phase 1 Implementation (Critical)
    /// - Implements segmentation BEFORE fingerprinting
    /// - Stubs new components (PatternAnalyzer, ContextualMatcher, ConfidenceAssessor)
    /// - Uses whole-file fingerprinting temporarily (per-segment in Phase 3)
    ///
    /// # Returns
    /// Number of passages created for this file
    async fn process_file_plan025(
        db: &SqlitePool,
        _event_bus: &EventBus,
        session_id: Uuid,
        file_path: &std::path::Path,
        file: &crate::db::files::AudioFile,
        acoustid_client: Option<Arc<AcoustIDClient>>,
        acousticbrainz_client: Option<Arc<AcousticBrainzClient>>,
    ) -> Result<usize> {
        tracing::debug!(
            session_id = %session_id,
            file = ?file_path,
            "Starting PLAN025 per-file pipeline"
        );

        // Step 1: Verify file exists
        if !file_path.exists() {
            anyhow::bail!("File not found: {:?}", file_path);
        }

        // Step 2: Extract metadata
        tracing::debug!(
            session_id = %session_id,
            file = ?file_path,
            "Step 2: Extracting metadata"
        );

        let metadata_extractor = crate::services::MetadataExtractor::new();
        let audio_metadata = match metadata_extractor.extract(file_path) {
            Ok(metadata) => {
                tracing::debug!(
                    session_id = %session_id,
                    artist = ?metadata.artist,
                    title = ?metadata.title,
                    album = ?metadata.album,
                    duration = ?metadata.duration_seconds,
                    "Metadata extracted successfully"
                );
                Some(metadata)
            }
            Err(e) => {
                tracing::warn!(
                    session_id = %session_id,
                    file = ?file_path,
                    error = ?e,
                    "Failed to extract metadata, continuing without it"
                );
                None
            }
        };

        // Step 3: Compute file hash (already done in SCANNING phase, skip for now)

        // **[REQ-PIPE-010]** Step 4: SEGMENT - Silence detection BEFORE fingerprinting
        tracing::debug!(
            session_id = %session_id,
            file = ?file_path,
            "Step 4: SEGMENTING (before fingerprinting)"
        );

        // Load audio for silence detection
        // For Phase 1, create one passage per file (stub)
        // Phase 1 implementation will use SilenceDetector in Phase 1b
        let duration_sec_f64 = if let Some(ticks) = file.duration_ticks {
            wkmp_common::timing::ticks_to_seconds(ticks)
        } else {
            180.0 // Default 180 seconds
        };

        let segments = vec![SegmentBoundary {
            start_seconds: 0.0,
            end_seconds: duration_sec_f64 as f32,
        }];

        tracing::debug!(
            session_id = %session_id,
            segments = segments.len(),
            "Segmentation complete"
        );

        // **[PLAN025 Phase 2]** Step 5: Pattern Analysis + Contextual Matching
        tracing::debug!(
            session_id = %session_id,
            "Step 5: Pattern analysis and contextual matching"
        );

        // Convert segments to PatternAnalyzer format
        let pattern_segments: Vec<crate::services::Segment> = segments
            .iter()
            .map(|s| crate::services::Segment::new(s.start_seconds, s.end_seconds))
            .collect();

        // Run PatternAnalyzer
        let pattern_analyzer = crate::services::PatternAnalyzer::new();
        let pattern_metadata = pattern_analyzer.analyze(&pattern_segments)?;

        tracing::info!(
            session_id = %session_id,
            track_count = pattern_metadata.track_count,
            source_media = pattern_metadata.likely_source_media.as_str(),
            gap_pattern = pattern_metadata.gap_pattern.as_str(),
            confidence = pattern_metadata.confidence,
            "Pattern analysis complete"
        );

        // **[PLAN025 Phase 2 Integration]** Step 5: Contextual Matching
        tracing::debug!(
            session_id = %session_id,
            has_metadata = audio_metadata.is_some(),
            "Step 5: Contextual matching"
        );

        // Track best MBID candidate from contextual matching
        let mut best_mbid: Option<String> = None;

        let metadata_score: f32 = if let Some(ref metadata) = audio_metadata {
            // Try to create contextual matcher
            let contextual_matcher = match crate::services::ContextualMatcher::new() {
                Ok(matcher) => Some(matcher),
                Err(e) => {
                    tracing::warn!(
                        session_id = %session_id,
                        error = ?e,
                        "Failed to create contextual matcher"
                    );
                    None
                }
            };

            // Attempt contextual matching if matcher was created successfully
            if let Some(matcher) = contextual_matcher {
                let match_candidates = if pattern_metadata.track_count == 1 {
                    // Single-segment: match by artist + title
                    matcher
                        .match_single_segment(
                            metadata.artist.as_deref().unwrap_or(""),
                            metadata.title.as_deref().unwrap_or(""),
                            metadata.duration_seconds.map(|d| d as f32),
                        )
                        .await
                } else {
                    // Multi-segment: match by album structure
                    matcher
                        .match_multi_segment(
                            metadata.album.as_deref().unwrap_or(""),
                            metadata.artist.as_deref().unwrap_or(""),
                            &pattern_metadata,
                        )
                        .await
                };

                match match_candidates {
                    Ok(candidates) if !candidates.is_empty() => {
                        if let Some(top_candidate) = candidates.first() {
                            // Store top MBID for potential flavor extraction
                            best_mbid = Some(top_candidate.recording_mbid.clone());

                            tracing::info!(
                                session_id = %session_id,
                                candidate_count = candidates.len(),
                                top_score = top_candidate.match_score,
                                mbid = %top_candidate.recording_mbid,
                                "Contextual matching found candidates"
                            );

                            top_candidate.match_score
                        } else {
                            0.0
                        }
                    }
                    Ok(_) => {
                        tracing::debug!(
                            session_id = %session_id,
                            "Contextual matching found no candidates"
                        );
                        0.0
                    }
                    Err(e) => {
                        tracing::warn!(
                            session_id = %session_id,
                            error = ?e,
                            "Contextual matching failed"
                        );
                        0.0
                    }
                }
            } else {
                // Contextual matcher creation failed
                0.0
            }
        } else {
            tracing::debug!(
                session_id = %session_id,
                "No metadata available for contextual matching"
            );
            0.0
        };

        // **[PLAN025 Phase 3 Integration]** Step 6: Per-Segment Fingerprinting
        tracing::debug!(
            session_id = %session_id,
            segment_count = segments.len(),
            "Step 6: Per-segment fingerprinting"
        );

        // Generate fingerprints for each segment
        let fingerprinter = crate::services::Fingerprinter::new();
        let mut segment_fingerprints = Vec::new();

        for (idx, segment) in segments.iter().enumerate() {
            match fingerprinter.fingerprint_segment(
                file_path,
                segment.start_seconds,
                segment.end_seconds,
            ) {
                Ok(fingerprint) => {
                    tracing::debug!(
                        session_id = %session_id,
                        segment_index = idx,
                        fingerprint_len = fingerprint.len(),
                        "Segment fingerprint generated"
                    );
                    segment_fingerprints.push(fingerprint);
                }
                Err(e) => {
                    tracing::warn!(
                        session_id = %session_id,
                        segment_index = idx,
                        error = ?e,
                        "Failed to fingerprint segment, continuing with others"
                    );
                    // Continue with other segments - per-file error isolation
                }
            }
        }

        tracing::info!(
            session_id = %session_id,
            total_segments = segments.len(),
            fingerprints_generated = segment_fingerprints.len(),
            "Per-segment fingerprinting complete"
        );

        // Query AcoustID API with per-segment fingerprints (rate-limited 3 req/s)
        let fingerprint_score = if let Some(client) = acoustid_client {
            if segment_fingerprints.is_empty() {
                0.0 // No fingerprints generated
            } else {
                // Query AcoustID for each segment fingerprint
                let mut acoustid_scores = Vec::new();

                for (idx, (fingerprint, segment)) in
                    segment_fingerprints.iter().zip(segments.iter()).enumerate()
                {
                    let duration_seconds = (segment.end_seconds - segment.start_seconds) as u64;

                    match client.lookup(fingerprint, duration_seconds).await {
                        Ok(response) => {
                            if let Some(result) = response.results.first() {
                                let score = result.score as f32;
                                acoustid_scores.push(score);

                                // If we don't have an MBID yet, try to get one from AcoustID
                                if best_mbid.is_none() {
                                    if let Some(recordings) = &result.recordings {
                                        if let Some(recording) = recordings.first() {
                                            best_mbid = Some(recording.id.clone());
                                            tracing::debug!(
                                                session_id = %session_id,
                                                mbid = %recording.id,
                                                "Using MBID from AcoustID result"
                                            );
                                        }
                                    }
                                }

                                tracing::debug!(
                                    session_id = %session_id,
                                    segment_index = idx,
                                    score = score,
                                    recordings = result.recordings.as_ref().map(|r| r.len()).unwrap_or(0),
                                    "AcoustID match found for segment"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                session_id = %session_id,
                                segment_index = idx,
                                error = ?e,
                                "AcoustID lookup failed for segment, continuing"
                            );
                            // Continue with other segments - per-file error isolation
                        }
                    }
                }

                // Aggregate scores: average of all successful matches
                if acoustid_scores.is_empty() {
                    tracing::debug!(
                        session_id = %session_id,
                        "No AcoustID matches found for any segment"
                    );
                    0.0
                } else {
                    let avg_score =
                        acoustid_scores.iter().sum::<f32>() / acoustid_scores.len() as f32;
                    tracing::info!(
                        session_id = %session_id,
                        matches = acoustid_scores.len(),
                        avg_score = avg_score,
                        "AcoustID per-segment lookup complete"
                    );
                    avg_score
                }
            }
        } else {
            tracing::debug!(
                session_id = %session_id,
                "AcoustID client not available (no API key), using score 0.0"
            );
            0.0 // No AcoustID client available
        };

        // **[PLAN025 Phase 2]** Step 7: Evidence-based confidence assessment
        tracing::debug!(
            session_id = %session_id,
            "Step 7: Confidence assessment"
        );

        let confidence_assessor = crate::services::ConfidenceAssessor::new();
        let evidence = crate::services::Evidence {
            metadata_score,
            fingerprint_score,
            duration_match: 0.0, // No duration matching yet
        };

        let confidence_result = if pattern_metadata.track_count == 1 {
            confidence_assessor.assess_single_segment(evidence)?
        } else {
            confidence_assessor.assess_multi_segment(evidence)?
        };

        tracing::info!(
            session_id = %session_id,
            confidence = confidence_result.confidence,
            decision = confidence_result.decision.as_str(),
            "Confidence assessment complete"
        );

        // Handle decision
        match confidence_result.decision {
            crate::services::Decision::Accept => {
                tracing::info!(
                    session_id = %session_id,
                    "Decision: ACCEPT - Creating passages with MBID"
                );
            }
            crate::services::Decision::Review => {
                tracing::warn!(
                    session_id = %session_id,
                    confidence = confidence_result.confidence,
                    "Decision: REVIEW - Manual review required (logged, no UI yet)"
                );
            }
            crate::services::Decision::Reject => {
                tracing::warn!(
                    session_id = %session_id,
                    confidence = confidence_result.confidence,
                    "Decision: REJECT - Creating zero-song passages (graceful degradation)"
                );
            }
        }

        // **[PLAN025 Integration]** Step 8: Amplitude Analysis
        tracing::debug!(
            session_id = %session_id,
            segment_count = segments.len(),
            "Step 8: Amplitude analysis"
        );

        // Analyze amplitude for each segment to detect lead-in/lead-out timing
        let amplitude_params = crate::models::AmplitudeParameters::default();
        let amplitude_analyzer = crate::services::AmplitudeAnalyzer::new(amplitude_params);
        let mut amplitude_results = Vec::new();

        for (idx, segment) in segments.iter().enumerate() {
            // Old workflow (legacy): disable yielding
            match amplitude_analyzer
                .analyze_file(
                    file_path,
                    segment.start_seconds as f64,
                    segment.end_seconds as f64,
                    0,
                )
                .await
            {
                Ok(result) => {
                    tracing::debug!(
                        session_id = %session_id,
                        segment_index = idx,
                        lead_in = result.lead_in_duration,
                        lead_out = result.lead_out_duration,
                        peak_rms = result.peak_rms,
                        "Amplitude analysis complete for segment"
                    );
                    amplitude_results.push(Some(result));
                }
                Err(e) => {
                    tracing::warn!(
                        session_id = %session_id,
                        segment_index = idx,
                        error = ?e,
                        "Amplitude analysis failed for segment, continuing"
                    );
                    amplitude_results.push(None);
                }
            }
        }

        tracing::info!(
            session_id = %session_id,
            total_segments = segments.len(),
            analyzed = amplitude_results.iter().filter(|r| r.is_some()).count(),
            "Amplitude analysis complete"
        );

        // **[PLAN025 Integration]** Step 9: Musical Flavor Extraction
        //
        // **HIGH-LEVEL FEATURE EXTRACTION**
        // We extract HIGH-LEVEL musical characteristics from AcousticBrainz:
        // - Musical key and scale (e.g., "C major")
        // - Tempo (BPM)
        // - Danceability score
        // - Spectral features (brightness, energy)
        // - Harmonic complexity (dissonance)
        // - Dynamic range
        //
        // These are AGGREGATED features computed by Essentia, not raw audio data.
        // The AcousticBrainz "low-level" endpoint name is misleading - it provides
        // high-level musical descriptors suitable for passage selection.
        tracing::debug!(
            session_id = %session_id,
            has_mbid = best_mbid.is_some(),
            decision = confidence_result.decision.as_str(),
            "Step 9: Musical flavor extraction (high-level features)"
        );

        // Only query AcousticBrainz for Accept decisions with confirmed MBID
        let musical_flavor = if matches!(
            confidence_result.decision,
            crate::services::Decision::Accept
        ) {
            if let Some(ref mbid) = best_mbid {
                if let Some(ref ab_client) = acousticbrainz_client {
                    match ab_client.lookup_lowlevel(mbid).await {
                        Ok(lowlevel_data) => {
                            // Extract high-level musical features from AcousticBrainz data
                            let flavor = crate::services::MusicalFlavorVector::from_acousticbrainz(
                                &lowlevel_data,
                            );

                            // Convert to JSON for database storage
                            match flavor.to_json() {
                                Ok(json) => {
                                    tracing::info!(
                                        session_id = %session_id,
                                        mbid = %mbid,
                                        has_key = flavor.key.is_some(),
                                        has_bpm = flavor.bpm.is_some(),
                                        "Musical flavor extracted successfully"
                                    );
                                    Some(json)
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        session_id = %session_id,
                                        error = ?e,
                                        "Failed to serialize flavor vector"
                                    );
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            tracing::debug!(
                                session_id = %session_id,
                                mbid = %mbid,
                                error = ?e,
                                "AcousticBrainz lookup failed (recording may not be in database)"
                            );
                            None
                        }
                    }
                } else {
                    tracing::debug!(
                        session_id = %session_id,
                        "AcousticBrainz client not available"
                    );
                    None
                }
            } else {
                tracing::debug!(
                    session_id = %session_id,
                    "No MBID available for flavor extraction"
                );
                None
            }
        } else {
            tracing::debug!(
                session_id = %session_id,
                decision = confidence_result.decision.as_str(),
                "Skipping flavor extraction (not Accept decision)"
            );
            None
        };

        if musical_flavor.is_some() {
            tracing::info!(
                session_id = %session_id,
                "Musical flavor will be stored in passage"
            );
        }

        // Step 10: DB - Store passages
        let mut passages_created = 0;
        for (idx, segment) in segments.iter().enumerate() {
            let mut passage = crate::db::passages::Passage::new(
                file.guid,
                segment.start_seconds as f64,
                segment.end_seconds as f64,
            );

            // Populate metadata fields if available
            if let Some(ref metadata) = audio_metadata {
                passage.artist = metadata.artist.clone();
                passage.title = metadata.title.clone();
                passage.album = metadata.album.clone();
            }

            // Populate musical flavor vector if available
            if let Some(ref flavor_json) = musical_flavor {
                passage.musical_flavor_vector = Some(flavor_json.clone());
            }

            // Populate lead-in/lead-out timing from amplitude analysis
            if let Some(Some(ref amplitude_result)) = amplitude_results.get(idx) {
                use wkmp_common::timing::seconds_to_ticks;

                // Calculate lead-in start: passage start + lead-in duration
                let lead_in_start =
                    passage.start_time_ticks + seconds_to_ticks(amplitude_result.lead_in_duration);

                // Calculate lead-out start: passage end - lead-out duration
                let lead_out_start =
                    passage.end_time_ticks - seconds_to_ticks(amplitude_result.lead_out_duration);

                // Ensure values stay within passage boundaries (database constraints)
                if lead_in_start >= passage.start_time_ticks
                    && lead_in_start <= passage.end_time_ticks
                {
                    passage.lead_in_start_ticks = Some(lead_in_start);
                }

                if lead_out_start >= passage.start_time_ticks
                    && lead_out_start <= passage.end_time_ticks
                {
                    passage.lead_out_start_ticks = Some(lead_out_start);
                }

                tracing::debug!(
                    session_id = %session_id,
                    segment_index = idx,
                    lead_in_start_ticks = ?passage.lead_in_start_ticks,
                    lead_out_start_ticks = ?passage.lead_out_start_ticks,
                    "Populated amplitude-based timing"
                );
            }

            if let Err(e) = crate::db::passages::save_passage(db, &passage).await {
                tracing::warn!(
                    session_id = %session_id,
                    file = ?file_path,
                    error = ?e,
                    "Failed to save passage"
                );
            } else {
                passages_created += 1;
                tracing::debug!(
                    session_id = %session_id,
                    passage_id = %passage.guid,
                    artist = ?passage.artist,
                    title = ?passage.title,
                    "Passage created with metadata"
                );
            }
        }

        tracing::debug!(
            session_id = %session_id,
            file = ?file_path,
            passages = passages_created,
            "PLAN025 per-file pipeline completed"
        );

        Ok(passages_created)
    }
}

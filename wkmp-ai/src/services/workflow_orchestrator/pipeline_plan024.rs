//! PLAN024 per-file pipeline (production)
//!
//! Extracted from mod.rs for maintainability.
//! Contains the production per-file pipeline with two paths:
//! - Album files (multi-passage segmentation)
//! - Single-track files (direct resolution)

use anyhow::Result;
use uuid::Uuid;
use wkmp_common::events::{FileState, WkmpEvent};

use super::WorkflowOrchestrator;

impl WorkflowOrchestrator {
    /// **[PLAN031 Fix 7]** Process file through 10-phase pipeline with late audio decoding
    ///
    /// **Architecture Change:** Audio decoding moved from pre-pipeline to Phase 4
    /// - Phase 1: Filename matching (early-exit if already processed)
    /// - Phase 2: Hash computation (early-exit if duplicate)
    /// - Phase 3: Metadata extraction
    /// - **Phase 4: Decode audio + Segmentation** (decode happens HERE, not before Phase 1)
    /// - Phases 5, 8: Reuse decoded samples from Phase 4
    ///
    /// **Benefits:**
    /// - Files that early-exit never get decoded (save CPU + memory)
    /// - Hash computation reads compressed bytes only (not 200 MB decoded samples)
    /// - Single decode per file (not pre-decode + hash read)
    pub async fn process_file_plan024(
        &self,
        file_path: &std::path::Path,
        root_folder: &std::path::Path,
        file_index: usize,
    ) -> Result<()> {
        tracing::info!(
            file = ?file_path,
            "Starting PLAN024 10-phase per-file pipeline (late audio decode)"
        );

        // Phase 1: Filename Matching
        self.set_worker_phase(file_path, root_folder, file_index, 1, "Filename Matching")
            .await;
        let phase1_start = std::time::Instant::now();
        tracing::debug!(file = ?file_path, "Phase 1: Filename Matching - START");

        // Calculate relative path from root folder
        let path_start = std::time::Instant::now();
        let relative_path = file_path
            .strip_prefix(root_folder)
            .map_err(|e| anyhow::anyhow!("File path not under root folder: {}", e))?;
        let path_elapsed = path_start.elapsed();

        let db_start = std::time::Instant::now();
        let filename_matcher = crate::services::FilenameMatcher::new(self.db.clone());
        let match_result = filename_matcher.check_file(relative_path).await?;
        let db_elapsed = db_start.elapsed();

        tracing::info!(
            phase = "Filename Matching",
            duration_ms = phase1_start.elapsed().as_millis(),
            path_us = path_elapsed.as_micros(),
            db_ms = db_elapsed.as_millis(),
            file_index = file_index,
            "Phase 1 completed"
        );

        let file_id = match match_result {
            crate::services::MatchResult::AlreadyProcessed(guid) => {
                // **[PLAN024]** Track completed filenames (early exit)
                self.statistics.increment_completed_filenames();

                tracing::info!(
                    file = ?file_path,
                    file_id = %guid,
                    "File already processed (INGEST COMPLETE/NO AUDIO/DUPLICATE HASH), skipping all remaining phases"
                );
                return Ok(());
            }
            crate::services::MatchResult::Reuse(guid) => {
                tracing::debug!(file_id = %guid, "Reusing existing file record (incomplete processing)");
                guid
            }
            crate::services::MatchResult::New => {
                // **[PLAN031 Fix 7]** Create minimal file record (like bulk insert)
                // Only path and modification time - hash/metadata/etc populated in later phases
                let fs_start = std::time::Instant::now();
                let metadata = std::fs::metadata(file_path)?;
                let modification_time = metadata
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs() as i64;
                let fs_elapsed = fs_start.elapsed();

                let insert_start = std::time::Instant::now();
                let guid = filename_matcher
                    .create_file_record(relative_path, modification_time)
                    .await?;
                let insert_elapsed = insert_start.elapsed();

                tracing::debug!(
                    file_id = %guid,
                    fs_us = fs_elapsed.as_micros(),
                    insert_ms = insert_elapsed.as_millis(),
                    "Created new minimal file record"
                );
                guid
            }
        };

        // Phase 2: Hash Deduplication
        self.set_worker_phase(file_path, root_folder, file_index, 2, "Hash Deduplication")
            .await;
        let phase2_start = std::time::Instant::now();
        tracing::debug!(file = ?file_path, file_id = %file_id, "Phase 2: Hash Deduplication");
        let hash_deduplicator = crate::services::HashDeduplicator::new(self.db.clone());
        let hash_result = hash_deduplicator
            .process_file_hash(file_id, file_path)
            .await?;

        // **[PLAN024]** Track hash computation
        self.statistics.increment_hashes_computed();

        match hash_result {
            crate::services::HashResult::Duplicate {
                hash,
                original_file_id,
            } => {
                // **[PLAN024]** Track hash match (early exit)
                self.statistics.increment_hash_matches();

                tracing::info!(
                    phase = "Hash Deduplication",
                    duration_ms = phase2_start.elapsed().as_millis(),
                    file_index = file_index,
                    file = ?file_path,
                    file_id = %file_id,
                    hash,
                    original_file_id = %original_file_id,
                    "Duplicate hash found, skipping pipeline"
                );

                // **[File Processing Status]** Mark as duplicate hash
                {
                    let mut states = self.file_processing_states.write().await;
                    if let Some(file_state) = states.get_mut(&file_index) {
                        file_state.state = FileState::DuplicateHash;
                    }
                }

                return Ok(());
            }
            crate::services::HashResult::Unique(hash) => {
                tracing::info!(
                    phase = "Hash Deduplication",
                    duration_ms = phase2_start.elapsed().as_millis(),
                    file_index = file_index,
                    hash,
                    "Phase 2 completed - hash unique"
                );
            }
        }

        // Phase 3: Metadata Extraction & Merging
        self.set_worker_phase(file_path, root_folder, file_index, 3, "Metadata Extraction")
            .await;
        let phase3_start = std::time::Instant::now();
        tracing::debug!(file = ?file_path, file_id = %file_id, "Phase 3: Metadata Extraction & Merging");
        let metadata_merger = crate::services::MetadataMerger::new(self.db.clone());
        let merged_metadata = metadata_merger
            .extract_and_merge(file_id, file_path)
            .await?;

        // **[PLAN024]** Track metadata extraction
        let successful = merged_metadata.title.is_some()
            || merged_metadata.artist.is_some()
            || merged_metadata.album.is_some();
        self.statistics.record_metadata_extraction(successful);

        tracing::info!(
            phase = "Metadata Extraction",
            duration_ms = phase3_start.elapsed().as_millis(),
            file_index = file_index,
            successful = successful,
            has_title = merged_metadata.title.is_some(),
            has_artist = merged_metadata.artist.is_some(),
            has_album = merged_metadata.album.is_some(),
            "Phase 3 completed"
        );

        // **[PLAN032]** Phase 3.5: Single-Track Detection
        // Check if file is a single song (not a full album) BEFORE decoding audio
        // This allows bypassing album segmentation for individual track files
        let duration_mins = if merged_metadata.duration_ticks > 0 {
            Some(merged_metadata.duration_ticks as f64 / 28_224_000.0 / 60.0) // ticks to minutes
        } else {
            None
        };

        let single_track_analysis = crate::matching::SingleTrackDiscriminator::analyze_pre_decode(
            file_path,
            duration_mins
        );

        // Log single-track analysis results
        crate::matching::SingleTrackDiscriminator::log_pre_decode(
            &format!("F{}", file_index),
            &single_track_analysis,
            file_path
        );

        // **[PLAN032]** Branch based on single-track detection
        // Threshold: final_score >= 2.0 indicates high confidence single-track file
        const SINGLE_TRACK_BRANCH_THRESHOLD: f64 = 2.0;
        if single_track_analysis.is_likely_single_track
            && single_track_analysis.final_score >= SINGLE_TRACK_BRANCH_THRESHOLD
        {
            tracing::info!(
                file = ?file_path,
                file_index = file_index,
                score = single_track_analysis.final_score,
                "Single-track detected (score >= {}), switching to single-song resolution path",
                SINGLE_TRACK_BRANCH_THRESHOLD
            );

            // Decode audio for single-song processing
            let decode_start = std::time::Instant::now();
            let decoded = tokio::task::spawn_blocking({
                let file_path = file_path.to_path_buf();
                move || crate::utils::decode_audio_file(&file_path)
            })
            .await?
            .map_err(|e| anyhow::anyhow!("Audio decoding failed: {}", e))?;
            let decode_elapsed = decode_start.elapsed();

            tracing::info!(
                file = ?file_path,
                duration = format!("{:.2}s", decoded.duration_seconds),
                decode_ms = decode_elapsed.as_millis(),
                "Audio decoded for single-song resolution"
            );

            // Calculate duration in ticks
            const TICKS_PER_SECOND: i64 = 28_224_000;
            let duration_seconds = decoded.samples.len() as f64 / decoded.sample_rate as f64;
            let duration_ticks = (duration_seconds * TICKS_PER_SECOND as f64) as i64;

            // Route to single-song resolution pipeline
            return self.resolve_single_song(
                file_id,
                file_path,
                root_folder,
                file_index,
                &decoded,
                &merged_metadata,
                duration_ticks,
            ).await;
        }

        // Continue with album segmentation pipeline (file is NOT a single track)
        tracing::debug!(
            file = ?file_path,
            file_index = file_index,
            score = single_track_analysis.final_score,
            "Not single-track (score < {}), continuing with album pipeline",
            SINGLE_TRACK_BRANCH_THRESHOLD
        );

        // Acquire album semaphore to limit concurrent album processing.
        // This prevents album files from monopolizing all worker slots.
        let _album_permit = self.album_semaphore.acquire().await
            .map_err(|e| anyhow::anyhow!("Album semaphore closed: {}", e))?;
        tracing::debug!(
            file_index = file_index,
            "Album semaphore acquired"
        );

        // **[PLAN031 Fix 7]** Phase 4: Audio Decode + Passage Segmentation
        // Audio is decoded HERE (not before Phase 1) after early-exit opportunities
        self.set_worker_phase(
            file_path,
            root_folder,
            file_index,
            4,
            "Passage Segmentation",
        )
        .await;
        let phase4_start = std::time::Instant::now();
        tracing::debug!(file = ?file_path, file_id = %file_id, "Phase 4: Decoding audio + Passage Segmentation");

        // Decode audio file to mono f32 PCM
        tracing::debug!(file = ?file_path, "Decoding audio (first and only decode)");
        let decode_start = std::time::Instant::now();
        let mut decoded = tokio::task::spawn_blocking({
            let file_path = file_path.to_path_buf();
            move || crate::utils::decode_audio_file(&file_path)
        })
        .await?
        .map_err(|e| anyhow::anyhow!("Audio decoding failed: {}", e))?;
        let decode_elapsed = decode_start.elapsed();

        tracing::info!(
            file = ?file_path,
            sample_rate = decoded.sample_rate,
            channels = decoded.channels,
            duration = format!("{:.2}s", decoded.duration_seconds),
            samples = decoded.samples.len(),
            decode_ms = decode_elapsed.as_millis(),
            "Audio decoded successfully"
        );

        // Calculate duration in ticks from sample count
        const TICKS_PER_SECOND: i64 = 28_224_000;
        let duration_seconds = decoded.samples.len() as f64 / decoded.sample_rate as f64;
        let duration_ticks = (duration_seconds * TICKS_PER_SECOND as f64) as i64;

        // Segment audio into passages
        tracing::debug!(file = ?file_path, "Starting passage segmentation");
        let segment_start = std::time::Instant::now();
        let passage_segmenter = crate::services::PassageSegmenter::new(self.db.clone());
        let segment_result = passage_segmenter
            .segment_file(
                file_id,
                file_path,
                &decoded.samples,
                decoded.sample_rate as usize,
                duration_ticks,
            )
            .await?;
        let segment_elapsed = segment_start.elapsed();

        tracing::info!(
            file = ?file_path,
            segment_ms = segment_elapsed.as_millis(),
            "Passage segmentation completed"
        );

        let passages = match segment_result {
            crate::services::SegmentResult::NoAudio => {
                // **[PLAN024]** Track segmentation (no audio - early exit)
                self.statistics.record_segmentation(0, 0, 0);

                tracing::info!(
                    phase = "Passage Segmentation",
                    duration_ms = phase4_start.elapsed().as_millis(),
                    file_index = file_index,
                    file = ?file_path,
                    file_id = %file_id,
                    "No audio detected, skipping pipeline"
                );

                // **[File Processing Status]** Mark as no audio
                {
                    let mut states = self.file_processing_states.write().await;
                    if let Some(file_state) = states.get_mut(&file_index) {
                        file_state.state = FileState::NoAudio;
                    }
                }

                return Ok(());
            }
            crate::services::SegmentResult::Passages(boundaries) => {
                tracing::info!(
                    phase = "Passage Segmentation",
                    duration_ms = phase4_start.elapsed().as_millis(),
                    file_index = file_index,
                    passage_count = boundaries.len(),
                    "Phase 4 completed"
                );
                boundaries
            }
        };

        // **[SPEC-EMBID-001]** Album Edition Matching (Stage 1)
        // AlbumMatcher resolves per-passage MBIDs from MusicBrainz album editions.
        // When matched, AlbumMatcher's track boundaries OVERRIDE Phase 4 passages
        // (AlbumMatcher has multi-stage silence detection; Phase 4 may find fewer boundaries).
        // Falls back to AcoustID (Stage 2) only if album matching fails.
        let mut passages = passages; // Make mutable for AlbumMatcher boundary override
        let preresolved: Vec<Option<crate::services::passage_song_matcher::MbidResolution>> = {
            use crate::matching::{AlbumMatcher, AlbumMatcherConfig, ConfidenceTier};

            let artist_hint = merged_metadata.artist.as_deref();
            let album_hint = merged_metadata.album.as_deref();

            let album_result = match crate::services::MusicBrainzClient::new() {
                Ok(client) => {
                    let matcher = AlbumMatcher::with_pool(
                        AlbumMatcherConfig::default(),
                        client,
                        self.db.clone(),
                    );
                    matcher
                        .match_album(file_path, artist_hint, album_hint)
                        .await
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "MusicBrainzClient unavailable for album matching");
                    Err(crate::matching::AlbumMatchError::MusicBrainzError(
                        e.to_string(),
                    ))
                }
            };

            match album_result {
                Ok(ref result) if result.matched => {
                    // Derive passage boundaries from AlbumMatcher's detected track durations
                    const TICKS_PER_SECOND: f64 = 28_224_000.0;
                    let mut album_passages = Vec::with_capacity(result.tracks.len());
                    let mut cursor_ticks: i64 = 0;
                    for track in &result.tracks {
                        let duration_ticks = (track.detected_duration * TICKS_PER_SECOND) as i64;
                        album_passages.push(
                            crate::services::PassageBoundary::new(
                                cursor_ticks,
                                cursor_ticks + duration_ticks,
                            ),
                        );
                        cursor_ticks += duration_ticks;
                    }

                    tracing::info!(
                        release_mbid = ?result.release_mbid,
                        artist = ?result.matched_artist,
                        album = ?result.matched_album,
                        match_pct = result.match_percentage,
                        tracks = result.tracks.len(),
                        phase4_passages = passages.len(),
                        "AlbumMatcher: matched — using edition MBIDs and track boundaries"
                    );

                    // Override Phase 4 passages with AlbumMatcher boundaries
                    passages = album_passages;

                    result
                        .tracks
                        .iter()
                        .map(|track| {
                            Some(crate::services::passage_song_matcher::MbidResolution {
                                mbid: track.recording_mbid.clone(),
                                tier: ConfidenceTier::Tier3,
                                score: (result.match_percentage / 100.0) as f32,
                                source: "AlbumMatcher".to_string(),
                            })
                        })
                        .collect()
                }
                Ok(_) => {
                    tracing::info!("AlbumMatcher: no match — falling back to AcoustID");
                    vec![None; passages.len()]
                }
                Err(e) => {
                    tracing::warn!(error = %e, "AlbumMatcher failed — falling back to AcoustID");
                    vec![None; passages.len()]
                }
            }
        };

        // Phase 5: Per-Passage Fingerprinting (skipped when AlbumMatcher resolved all passages)
        let needs_fingerprinting = preresolved.iter().any(|r| r.is_none());

        let fingerprint_results = if needs_fingerprinting {
            self.set_worker_phase(file_path, root_folder, file_index, 5, "Fingerprinting")
                .await;
            let phase5_start = std::time::Instant::now();
            tracing::debug!(
                file = ?file_path,
                file_id = %file_id,
                passage_count = passages.len(),
                "Phase 5: Per-Passage Fingerprinting"
            );

            // Get API key from database settings
            let api_key: Option<String> =
                sqlx::query_scalar("SELECT value FROM settings WHERE key = 'acoustid_api_key'")
                    .fetch_optional(&self.db)
                    .await?;

            let passage_fingerprinter =
                crate::services::PassageFingerprinter::new(api_key, self.db.clone())?;
            let fp_results = passage_fingerprinter
                .fingerprint_passages(file_path, &passages)
                .await?;

            // **[PLAN024]** Track fingerprinting
            let (passages_fingerprinted, successful_matches) = match &fp_results {
                crate::services::FingerprintResult::Success(candidates) => {
                    (passages.len(), candidates.len())
                }
                _ => (passages.len(), 0),
            };
            self.statistics
                .record_fingerprinting(passages_fingerprinted, successful_matches);

            tracing::info!(
                phase = "Fingerprinting",
                duration_ms = phase5_start.elapsed().as_millis(),
                file_index = file_index,
                passages_fingerprinted = passages_fingerprinted,
                successful_matches = successful_matches,
                "Phase 5 completed"
            );

            fp_results
        } else {
            // All passages pre-resolved by AlbumMatcher — skip AcoustID
            tracing::info!(
                file = ?file_path,
                passages = passages.len(),
                "Phase 5 skipped: all passages pre-resolved by AlbumMatcher"
            );
            self.set_worker_phase(
                file_path,
                root_folder,
                file_index,
                5,
                "Fingerprinting (skipped)",
            )
            .await;
            self.statistics.record_fingerprinting(0, 0);
            crate::services::FingerprintResult::Failed("Skipped: AlbumMatcher resolved all passages".to_string())
        };

        // Phase 6: Song Matching (with pre-resolved MBIDs from cascade)
        self.set_worker_phase(file_path, root_folder, file_index, 6, "Song Matching")
            .await;
        let phase6_start = std::time::Instant::now();
        tracing::debug!(
            file = ?file_path,
            file_id = %file_id,
            "Phase 6: Song Matching"
        );
        let passage_song_matcher = crate::services::PassageSongMatcher::new();
        let song_match_result = passage_song_matcher.match_passages_with_preresolved(
            &passages,
            &fingerprint_results,
            &merged_metadata,
            &preresolved,
        );

        // **[PLAN024]** Track song matching
        self.statistics.record_song_matching(
            song_match_result.stats.high_confidence,
            song_match_result.stats.medium_confidence,
            song_match_result.stats.low_confidence,
            song_match_result.stats.zero_song,
        );

        tracing::info!(
            phase = "Song Matching",
            duration_ms = phase6_start.elapsed().as_millis(),
            file_index = file_index,
            matches = song_match_result.matches.len(),
            high_conf = song_match_result.stats.high_confidence,
            medium_conf = song_match_result.stats.medium_confidence,
            low_conf = song_match_result.stats.low_confidence,
            zero_song = song_match_result.stats.zero_song,
            "Phase 6 completed"
        );

        // **[PLAN024]** Update segmenting stats with finalized passages
        {
            let mut seg_stats = self.statistics.segmenting.lock().unwrap();
            seg_stats.files_processed += 1;
            seg_stats.potential_passages += passages.len();
            seg_stats.finalized_passages += song_match_result.matches.len();
            seg_stats.songs_identified += song_match_result
                .matches
                .iter()
                .filter(|m| m.mbid.is_some())
                .count();
        }

        // Phase 7: Recording
        self.set_worker_phase(file_path, root_folder, file_index, 7, "Recording")
            .await;
        let phase7_start = std::time::Instant::now();
        tracing::debug!(
            file = ?file_path,
            file_id = %file_id,
            "Phase 7: Recording"
        );
        let passage_recorder = crate::services::PassageRecorder::new(self.db.clone());
        let recording_result = passage_recorder
            .record_passages(file_id, &song_match_result.matches)
            .await?;

        tracing::info!(
            phase = "Recording",
            duration_ms = phase7_start.elapsed().as_millis(),
            file_index = file_index,
            passages_recorded = recording_result.passages.len(),
            songs_created = recording_result.stats.songs_created,
            "Phase 7 completed"
        );

        // **[PLAN024]** Track recording (Phase 7)
        for passage_record in &recording_result.passages {
            let song_title = if let Some(ref song_id) = passage_record.song_id {
                // Query database for song title
                sqlx::query_scalar::<_, String>("SELECT title FROM songs WHERE guid = ?")
                    .bind(song_id.to_string())
                    .fetch_optional(&self.db)
                    .await?
            } else {
                None
            };

            let file_path_str = relative_path.to_string_lossy().to_string();
            self.statistics
                .add_recorded_passage(song_title, file_path_str);
        }

        // Phase 8: Amplitude Analysis
        self.set_worker_phase(file_path, root_folder, file_index, 8, "Amplitude Analysis")
            .await;
        let phase8_start = std::time::Instant::now();
        tracing::debug!(
            file = ?file_path,
            file_id = %file_id,
            "Phase 8: Amplitude Analysis"
        );
        let passage_amplitude_analyzer =
            crate::services::PassageAmplitudeAnalyzer::new(self.db.clone()).await?;
        let amplitude_result = passage_amplitude_analyzer
            .analyze_passages_with_audio(
                &decoded.samples,
                decoded.sample_rate,
                &recording_result.passages,
            )
            .await?;

        // Release decoded audio memory (~500MB for 50-minute album)
        decoded.clear();

        tracing::info!(
            phase = "Amplitude Analysis",
            duration_ms = phase8_start.elapsed().as_millis(),
            file_index = file_index,
            passages_analyzed = amplitude_result.passages.len(),
            "Phase 8 completed"
        );

        // **[PLAN024]** Track amplitude analysis (Phase 8)
        for passage_timing in &amplitude_result.passages {
            // Query passage details from database
            let passage_info: Option<(i64, i64, Option<String>)> = sqlx::query_as(
                "SELECT p.start_time_ticks, p.end_time_ticks, s.title
                 FROM passages p
                 LEFT JOIN songs s ON p.song_id = s.guid
                 WHERE p.guid = ?",
            )
            .bind(passage_timing.passage_id.to_string())
            .fetch_optional(&self.db)
            .await?;

            if let Some((start_ticks, end_ticks, song_title)) = passage_info {
                let passage_length_seconds =
                    (end_ticks - start_ticks) as f64 / TICKS_PER_SECOND as f64;

                // **[SPEC032]** lead_in_start_ticks and lead_out_start_ticks are stored as ABSOLUTE positions
                // (relative to file start). Compute durations by subtracting passage boundaries.
                // **[SPEC002]** Lead-in and lead-out durations are NON-NEGATIVE by definition
                // **Note:** For very short passages (near minimum_passage_audio_duration_ticks), these may be NULL

                // Lead-in duration = absolute lead-in position - passage start position (or 0 if NULL)
                let lead_in_duration_ticks = passage_timing
                    .lead_in_start_ticks
                    .map(|ticks| (ticks - start_ticks).max(0))
                    .unwrap_or(0);
                let lead_in_ms = (lead_in_duration_ticks * 1000 / TICKS_PER_SECOND) as u64;

                // Lead-out duration = passage end position - absolute lead-out position (or 0 if NULL)
                let lead_out_duration_ticks = passage_timing
                    .lead_out_start_ticks
                    .map(|ticks| (end_ticks - ticks).max(0))
                    .unwrap_or(0);
                let lead_out_ms = (lead_out_duration_ticks * 1000 / TICKS_PER_SECOND) as u64;

                self.statistics.add_analyzed_passage(
                    song_title,
                    passage_length_seconds,
                    lead_in_ms,
                    lead_out_ms,
                );

                self.statistics.increment_passages_completed();
            }
        }

        // Phase 9: Flavoring
        self.set_worker_phase(file_path, root_folder, file_index, 9, "Flavor Fetching")
            .await;
        let phase9_start = std::time::Instant::now();
        tracing::debug!(
            file = ?file_path,
            file_id = %file_id,
            "Phase 9: Flavoring"
        );
        let passage_flavor_fetcher = crate::services::PassageFlavorFetcher::new(self.db.clone())?;
        let flavor_result = passage_flavor_fetcher
            .fetch_flavors(file_path, &recording_result.passages, self.essentia_client.as_ref())
            .await?;

        // **[PLAN032]** Emit FlavorLookup analysis log events for each song
        let passage_total = flavor_result.songs.len() as u32;
        for (idx, song_result) in flavor_result.songs.iter().enumerate() {
            // Query song title and recording_mbid from database
            let song_info: Option<(String, String)> = sqlx::query_as(
                "SELECT title, recording_mbid FROM songs WHERE guid = ?"
            )
            .bind(song_result.song_id.to_string())
            .fetch_optional(&self.db)
            .await?;

            let (song_title, recording_mbid) = song_info
                .map(|(t, m)| (Some(t), Some(m)))
                .unwrap_or((None, None));

            let source_str = song_result.flavor_source.as_str();
            let message = if song_result.success {
                format!(
                    "Flavor fetched from {} for: {}",
                    source_str,
                    song_title.as_deref().unwrap_or("Unknown")
                )
            } else {
                format!(
                    "Flavor lookup failed for: {}",
                    song_title.as_deref().unwrap_or("Unknown")
                )
            };

            let log_type = if song_result.success {
                wkmp_common::events::AnalysisLogType::Success
            } else {
                wkmp_common::events::AnalysisLogType::Warning
            };

            self.event_bus.emit_lossy(WkmpEvent::AnalysisLog(
                wkmp_common::events::AnalysisLogEntry {
                    timestamp: chrono::Utc::now(),
                    file_index: file_index as u32,
                    total_files: 1, // TODO: Pass from batch orchestration
                    file_path: file_path.to_string_lossy().to_string(),
                    message_type: log_type,
                    message,
                    details: Some(wkmp_common::events::AnalysisLogDetails::FlavorLookup {
                        passage_index: idx as u32 + 1,
                        passage_total,
                        song_title,
                        source: source_str.to_string(),
                        success: song_result.success,
                        recording_mbid,
                    }),
                },
            ));
        }

        tracing::info!(
            phase = "Flavor Fetching",
            duration_ms = phase9_start.elapsed().as_millis(),
            file_index = file_index,
            songs_processed = flavor_result.stats.songs_processed,
            essentia = flavor_result.stats.essentia_count,
            "Phase 9 completed"
        );

        // **[PLAN024]** Track flavoring (Phase 9)
        for _ in 0..flavor_result.stats.essentia_count {
            self.statistics.record_flavoring(false, Some("essentia"));
        }
        for _ in 0..flavor_result.stats.failed_count {
            self.statistics.record_flavoring(false, None);
        }
        // Pre-existing flavors are those songs_processed but not in the other categories
        let pre_existing_count = flavor_result
            .stats
            .songs_processed
            .saturating_sub(flavor_result.stats.essentia_count)
            .saturating_sub(flavor_result.stats.failed_count);
        for _ in 0..pre_existing_count {
            self.statistics.record_flavoring(true, None);
        }

        // Phase 10: Finalization
        self.set_worker_phase(file_path, root_folder, file_index, 10, "Finalization")
            .await;
        let phase10_start = std::time::Instant::now();
        tracing::debug!(
            file = ?file_path,
            file_id = %file_id,
            "Phase 10: Finalization"
        );
        let passage_finalizer = crate::services::PassageFinalizer::new(self.db.clone());
        let finalization_result = passage_finalizer.finalize(file_id).await?;

        if finalization_result.success {
            // **[PLAN024]** Track file completion (Phase 10)
            self.statistics.increment_files_completed();

            tracing::info!(
                phase = "Finalization",
                duration_ms = phase10_start.elapsed().as_millis(),
                file_index = file_index,
                file = ?file_path,
                file_id = %file_id,
                passages = finalization_result.passages_validated,
                "Phase 10 completed - File ingested successfully"
            );

            // Enhanced import logging (album file)
            let _ = crate::services::import_logger::log_file_import_details(
                &self.db,
                &file_id,
                file_path,
            )
            .await;
        } else {
            tracing::error!(
                phase = "Finalization",
                duration_ms = phase10_start.elapsed().as_millis(),
                file_index = file_index,
                file = ?file_path,
                file_id = %file_id,
                errors = ?finalization_result.errors,
                "Phase 10 failed - Finalization validation errors"
            );
            anyhow::bail!(
                "Finalization failed with {} validation errors: {:?}",
                finalization_result.errors.len(),
                finalization_result.errors
            );
        }

        // Clear worker phase tracking when done (whether success or failure)
        self.clear_worker_phase().await;

        Ok(())
    }

    /// Resolve single-song file via direct AcoustID lookup
    ///
    /// **[PLAN032]** Single-song resolution path for files detected as individual tracks
    /// (not full albums). Bypasses album matching/segmentation for simpler, faster processing.
    ///
    /// # Algorithm
    /// 1. Create single passage covering entire file duration
    /// 2. Generate Chromaprint fingerprint for whole file
    /// 3. Query AcoustID for Recording MBID
    /// 4. Fall back to metadata-based MusicBrainz search if AcoustID fails
    /// 5. Create database entries via existing recording pipeline
    ///
    /// # Arguments
    /// * `file_id` - UUID of the file record in database
    /// * `file_path` - Absolute path to audio file
    /// * `root_folder` - Root import folder for relative path calculation
    /// * `file_index` - Index for progress tracking
    /// * `decoded` - Already-decoded audio samples (from Phase 4)
    /// * `merged_metadata` - Already-extracted metadata (from Phase 3)
    /// * `duration_ticks` - File duration in ticks
    ///
    /// # Returns
    /// * `Ok(())` on success, `Err` on failure
    async fn resolve_single_song(
        &self,
        file_id: Uuid,
        file_path: &std::path::Path,
        root_folder: &std::path::Path,
        file_index: usize,
        _decoded: &crate::utils::DecodedAudio, // For future use if inline audio processing needed
        merged_metadata: &crate::services::MergedMetadata,
        duration_ticks: i64,
    ) -> Result<()> {
        const TICKS_PER_SECOND: i64 = 28_224_000;

        tracing::info!(
            file = ?file_path,
            file_id = %file_id,
            "Single-song resolution: Bypassing album segmentation"
        );

        let relative_path = file_path.strip_prefix(root_folder)
            .map_err(|e| anyhow::anyhow!("File path not under root folder: {}", e))?;

        // Phase 4b: Create single passage covering entire file
        self.set_worker_phase(file_path, root_folder, file_index, 4, "Single-Song Passage")
            .await;
        let phase4b_start = std::time::Instant::now();

        // Single passage from start (0) to end (duration_ticks)
        let passages = vec![crate::services::PassageBoundary::new(0, duration_ticks)];

        tracing::info!(
            phase = "Single-Song Passage",
            duration_ms = phase4b_start.elapsed().as_millis(),
            file_index = file_index,
            "Created single passage covering entire file ({:.2}s)",
            duration_ticks as f64 / TICKS_PER_SECOND as f64
        );

        // Phase 4c: MBID Identification Cascade (metadata-first)
        // **[SPEC-EMBID-001]** Try embedded MBID → ContextualMatcher before AcoustID
        let cascade = crate::services::MbidIdentificationCascade::new();
        let preresolved_mbid = cascade.resolve_single_track(merged_metadata).await;
        let preresolved = vec![preresolved_mbid.clone()];

        // Phase 5: Fingerprinting (skip if Stage 0 embedded MBID found)
        let fingerprint_results = if preresolved_mbid
            .as_ref()
            .map_or(false, |r| r.tier.is_stage0())
        {
            // Stage 0 MBID is authoritative — skip AcoustID fingerprinting
            tracing::info!(
                file_index = file_index,
                mbid = %preresolved_mbid.as_ref().unwrap().mbid,
                tier = %preresolved_mbid.as_ref().unwrap().tier.name(),
                "Stage 0 MBID found, skipping AcoustID fingerprinting"
            );
            self.statistics.record_fingerprinting(1, 0);
            crate::services::FingerprintResult::Skipped
        } else {
            self.set_worker_phase(file_path, root_folder, file_index, 5, "Fingerprinting")
                .await;
            let phase5_start = std::time::Instant::now();
            tracing::debug!(
                file = ?file_path,
                file_id = %file_id,
                "Phase 5: Single-song Fingerprinting"
            );

            // Get API key from database settings
            let api_key: Option<String> =
                sqlx::query_scalar("SELECT value FROM settings WHERE key = 'acoustid_api_key'")
                    .fetch_optional(&self.db)
                    .await?;

            let passage_fingerprinter =
                crate::services::PassageFingerprinter::new(api_key, self.db.clone())?;
            let fp_results = passage_fingerprinter
                .fingerprint_passages(file_path, &passages)
                .await?;

            // Track fingerprinting statistics
            let (passages_fingerprinted, successful_matches) = match &fp_results {
                crate::services::FingerprintResult::Success(candidates) => {
                    (passages.len(), candidates.len())
                }
                _ => (passages.len(), 0),
            };
            self.statistics
                .record_fingerprinting(passages_fingerprinted, successful_matches);

            tracing::info!(
                phase = "Single-Song Fingerprinting",
                duration_ms = phase5_start.elapsed().as_millis(),
                file_index = file_index,
                successful_matches = successful_matches,
                "Phase 5 completed"
            );

            fp_results
        };

        // Phase 6: Song Matching (with pre-resolved MBIDs from cascade)
        self.set_worker_phase(file_path, root_folder, file_index, 6, "Song Matching")
            .await;
        let phase6_start = std::time::Instant::now();
        tracing::debug!(
            file = ?file_path,
            file_id = %file_id,
            "Phase 6: Single-song Matching"
        );

        let passage_song_matcher = crate::services::PassageSongMatcher::new();
        let song_match_result = passage_song_matcher.match_passages_with_preresolved(
            &passages,
            &fingerprint_results,
            merged_metadata,
            &preresolved,
        );

        // Track song matching statistics
        self.statistics.record_song_matching(
            song_match_result.stats.high_confidence,
            song_match_result.stats.medium_confidence,
            song_match_result.stats.low_confidence,
            song_match_result.stats.zero_song,
        );

        tracing::info!(
            phase = "Single-Song Matching",
            duration_ms = phase6_start.elapsed().as_millis(),
            file_index = file_index,
            matches = song_match_result.matches.len(),
            high_conf = song_match_result.stats.high_confidence,
            medium_conf = song_match_result.stats.medium_confidence,
            "Phase 6 completed"
        );

        // Update segmentation stats for single-song
        {
            let mut seg_stats = self.statistics.segmenting.lock().unwrap();
            seg_stats.files_processed += 1;
            seg_stats.potential_passages += 1; // Single passage
            seg_stats.finalized_passages += song_match_result.matches.len();
            seg_stats.songs_identified += song_match_result
                .matches
                .iter()
                .filter(|m| m.mbid.is_some())
                .count();
        }

        // Phase 7: Recording
        self.set_worker_phase(file_path, root_folder, file_index, 7, "Recording")
            .await;
        let phase7_start = std::time::Instant::now();
        tracing::debug!(
            file = ?file_path,
            file_id = %file_id,
            "Phase 7: Recording (single-song)"
        );
        let passage_recorder = crate::services::PassageRecorder::new(self.db.clone());
        let recording_result = passage_recorder
            .record_passages(file_id, &song_match_result.matches)
            .await?;

        tracing::info!(
            phase = "Recording",
            duration_ms = phase7_start.elapsed().as_millis(),
            file_index = file_index,
            passages_recorded = recording_result.passages.len(),
            songs_created = recording_result.stats.songs_created,
            "Phase 7 completed"
        );

        // Track recording statistics
        for passage_record in &recording_result.passages {
            let song_title = if let Some(ref song_id) = passage_record.song_id {
                sqlx::query_scalar::<_, String>("SELECT title FROM songs WHERE guid = ?")
                    .bind(song_id.to_string())
                    .fetch_optional(&self.db)
                    .await?
            } else {
                None
            };
            let file_path_str = relative_path.to_string_lossy().to_string();
            self.statistics
                .add_recorded_passage(song_title, file_path_str);
        }

        // Phase 8: Amplitude Analysis
        self.set_worker_phase(file_path, root_folder, file_index, 8, "Amplitude Analysis")
            .await;
        let phase8_start = std::time::Instant::now();
        tracing::debug!(
            file = ?file_path,
            file_id = %file_id,
            "Phase 8: Amplitude Analysis (single-song)"
        );
        let passage_amplitude_analyzer =
            crate::services::PassageAmplitudeAnalyzer::new(self.db.clone()).await?;
        let amplitude_result = passage_amplitude_analyzer
            .analyze_passages(file_path, &recording_result.passages)
            .await?;

        tracing::info!(
            phase = "Amplitude Analysis",
            duration_ms = phase8_start.elapsed().as_millis(),
            file_index = file_index,
            passages_analyzed = amplitude_result.passages.len(),
            "Phase 8 completed"
        );

        // Track amplitude statistics
        for passage_timing in &amplitude_result.passages {
            let passage_info: Option<(i64, i64, Option<String>)> = sqlx::query_as(
                "SELECT p.start_time_ticks, p.end_time_ticks, s.title
                 FROM passages p
                 LEFT JOIN songs s ON p.song_id = s.guid
                 WHERE p.guid = ?",
            )
            .bind(passage_timing.passage_id.to_string())
            .fetch_optional(&self.db)
            .await?;

            if let Some((start_ticks, end_ticks, song_title)) = passage_info {
                let passage_length_seconds =
                    (end_ticks - start_ticks) as f64 / TICKS_PER_SECOND as f64;
                let lead_in_duration_ticks = passage_timing
                    .lead_in_start_ticks
                    .map(|ticks| (ticks - start_ticks).max(0))
                    .unwrap_or(0);
                let lead_in_ms = (lead_in_duration_ticks * 1000 / TICKS_PER_SECOND) as u64;
                let lead_out_duration_ticks = passage_timing
                    .lead_out_start_ticks
                    .map(|ticks| (end_ticks - ticks).max(0))
                    .unwrap_or(0);
                let lead_out_ms = (lead_out_duration_ticks * 1000 / TICKS_PER_SECOND) as u64;

                self.statistics.add_analyzed_passage(
                    song_title,
                    passage_length_seconds,
                    lead_in_ms,
                    lead_out_ms,
                );
                self.statistics.increment_passages_completed();
            }
        }

        // Phase 9: Flavoring
        self.set_worker_phase(file_path, root_folder, file_index, 9, "Flavor Fetching")
            .await;
        let phase9_start = std::time::Instant::now();
        tracing::debug!(
            file = ?file_path,
            file_id = %file_id,
            "Phase 9: Flavoring (single-song)"
        );
        let passage_flavor_fetcher = crate::services::PassageFlavorFetcher::new(self.db.clone())?;
        let flavor_result = passage_flavor_fetcher
            .fetch_flavors(file_path, &recording_result.passages, self.essentia_client.as_ref())
            .await?;

        // **[PLAN032]** Emit FlavorLookup analysis log events for each song (single-song path)
        let passage_total = flavor_result.songs.len() as u32;
        for (idx, song_result) in flavor_result.songs.iter().enumerate() {
            // Query song title and recording_mbid from database
            let song_info: Option<(String, String)> = sqlx::query_as(
                "SELECT title, recording_mbid FROM songs WHERE guid = ?"
            )
            .bind(song_result.song_id.to_string())
            .fetch_optional(&self.db)
            .await?;

            let (song_title, recording_mbid) = song_info
                .map(|(t, m)| (Some(t), Some(m)))
                .unwrap_or((None, None));

            let source_str = song_result.flavor_source.as_str();
            let message = if song_result.success {
                format!(
                    "Flavor fetched from {} for: {}",
                    source_str,
                    song_title.as_deref().unwrap_or("Unknown")
                )
            } else {
                format!(
                    "Flavor lookup failed for: {}",
                    song_title.as_deref().unwrap_or("Unknown")
                )
            };

            let log_type = if song_result.success {
                wkmp_common::events::AnalysisLogType::Success
            } else {
                wkmp_common::events::AnalysisLogType::Warning
            };

            self.event_bus.emit_lossy(WkmpEvent::AnalysisLog(
                wkmp_common::events::AnalysisLogEntry {
                    timestamp: chrono::Utc::now(),
                    file_index: file_index as u32,
                    total_files: 1, // TODO: Pass from batch orchestration
                    file_path: file_path.to_string_lossy().to_string(),
                    message_type: log_type,
                    message,
                    details: Some(wkmp_common::events::AnalysisLogDetails::FlavorLookup {
                        passage_index: idx as u32 + 1,
                        passage_total,
                        song_title,
                        source: source_str.to_string(),
                        success: song_result.success,
                        recording_mbid,
                    }),
                },
            ));
        }

        tracing::info!(
            phase = "Flavor Fetching",
            duration_ms = phase9_start.elapsed().as_millis(),
            file_index = file_index,
            songs_processed = flavor_result.stats.songs_processed,
            "Phase 9 completed"
        );

        // Track flavoring statistics
        for _ in 0..flavor_result.stats.essentia_count {
            self.statistics.record_flavoring(false, Some("essentia"));
        }
        for _ in 0..flavor_result.stats.failed_count {
            self.statistics.record_flavoring(false, None);
        }

        // Phase 10: Finalization
        self.set_worker_phase(file_path, root_folder, file_index, 10, "Finalization")
            .await;
        let phase10_start = std::time::Instant::now();
        tracing::debug!(
            file = ?file_path,
            file_id = %file_id,
            "Phase 10: Finalization (single-song)"
        );
        let passage_finalizer = crate::services::PassageFinalizer::new(self.db.clone());
        let finalization_result = passage_finalizer.finalize(file_id).await?;

        if finalization_result.success {
            self.statistics.increment_files_completed();
            tracing::info!(
                phase = "Finalization",
                duration_ms = phase10_start.elapsed().as_millis(),
                file_index = file_index,
                file = ?file_path,
                file_id = %file_id,
                passages = finalization_result.passages_validated,
                "Phase 10 completed - Single-song ingested successfully"
            );

            // Enhanced import logging (single-track file)
            let _ = crate::services::import_logger::log_file_import_details(
                &self.db,
                &file_id,
                file_path,
            )
            .await;
        } else {
            tracing::error!(
                phase = "Finalization",
                duration_ms = phase10_start.elapsed().as_millis(),
                file_index = file_index,
                file = ?file_path,
                errors = ?finalization_result.errors,
                "Phase 10 failed - Finalization validation errors"
            );
            anyhow::bail!(
                "Single-song finalization failed with {} errors: {:?}",
                finalization_result.errors.len(),
                finalization_result.errors
            );
        }

        // Clear worker phase tracking
        self.clear_worker_phase().await;

        Ok(())
    }
}

//! Passage Segmentation for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-011] Segmentation (Phase 4)
//!
//! Detects passage boundaries via silence detection and identifies NO AUDIO files.
//! Uses settings from database for thresholds and durations.

use sqlx::{Pool, Sqlite};
use std::path::Path;
use uuid::Uuid;
use wkmp_common::{Error, Result};

use super::silence_detector::{SilenceDetector, SilenceRegion};
use crate::db::settings;

/// SPEC017: 28,224,000 ticks per second
const TICKS_PER_SECOND: i64 = 28_224_000;

/// Passage boundary in ticks
#[derive(Debug, Clone, PartialEq)]
pub struct PassageBoundary {
    /// Start time in ticks
    pub start_ticks: i64,
    /// End time in ticks
    pub end_ticks: i64,
}

impl PassageBoundary {
    /// Create new passage boundary
    pub fn new(start_ticks: i64, end_ticks: i64) -> Self {
        Self {
            start_ticks,
            end_ticks,
        }
    }

    /// Calculate duration in ticks
    pub fn duration_ticks(&self) -> i64 {
        self.end_ticks - self.start_ticks
    }
}

/// Segmentation result
#[derive(Debug, Clone, PartialEq)]
pub enum SegmentResult {
    /// File has valid audio passages
    Passages(Vec<PassageBoundary>),
    /// File has <100ms non-silence (NO AUDIO)
    NoAudio,
}

/// Passage Segmenter
///
/// **Traceability:** [REQ-SPEC032-011] (Phase 4: SEGMENTING)
pub struct PassageSegmenter {
    db: Pool<Sqlite>,
}

impl PassageSegmenter {
    /// Create new passage segmenter
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    /// Segment audio file into passages
    ///
    /// **Algorithm:**
    /// 1. Load settings from database (silence_threshold_dB, silence_min_duration_ticks, minimum_passage_audio_duration_ticks)
    /// 2. Decode audio to PCM (use symphonia or metadata extractor's approach)
    /// 3. Detect silence regions using SilenceDetector
    /// 4. Calculate total non-silence duration
    /// 5. If total non-silence < minimum_passage_audio_duration_ticks (100ms default):
    ///    - Return SegmentResult::NoAudio
    ///    - Update files.status = 'NO AUDIO'
    /// 6. Otherwise:
    ///    - Convert silence regions to passage boundaries (regions between silences)
    ///    - Return SegmentResult::Passages with boundaries in ticks
    ///
    /// **Traceability:** [REQ-SPEC032-011]
    ///
    /// **Note:** This is a simplified implementation that requires audio decoding.
    /// Full implementation would use symphonia to decode audio to PCM samples.
    /// For now, we'll create the service structure with placeholder logic.
    pub async fn segment_file(
        &self,
        file_id: Uuid,
        _file_path: &Path,
        samples: &[f32],
        sample_rate: usize,
        duration_ticks: i64,
    ) -> Result<SegmentResult> {
        tracing::debug!(
            file_id = %file_id,
            sample_rate,
            sample_count = samples.len(),
            duration_ticks,
            "Segmenting audio file"
        );

        // Load settings from database
        let silence_threshold_db = settings::get_silence_threshold_db(&self.db).await?;
        let silence_min_duration_ticks = settings::get_silence_min_duration_ticks(&self.db).await?;
        let minimum_passage_audio_duration_ticks =
            settings::get_minimum_passage_audio_duration_ticks(&self.db).await?;

        // Convert thresholds to formats needed by SilenceDetector
        let silence_threshold_db_f32 = -(silence_threshold_db as f32); // Negate: settings use positive dB, detector uses negative
        let silence_min_duration_sec =
            silence_min_duration_ticks as f32 / TICKS_PER_SECOND as f32;

        tracing::debug!(
            silence_threshold_db,
            silence_min_duration_ticks,
            minimum_passage_audio_duration_ticks,
            "Loaded segmentation settings"
        );

        // Configure silence detector
        let detector = SilenceDetector::new()
            .with_threshold_db(silence_threshold_db_f32)
            .map_err(|e| Error::Internal(format!("Invalid silence threshold: {}", e)))?
            .with_min_duration(silence_min_duration_sec)
            .map_err(|e| Error::Internal(format!("Invalid min duration: {}", e)))?;

        // Detect silence regions
        let silence_regions = detector
            .detect(samples, sample_rate)
            .map_err(|e| Error::Internal(format!("Silence detection failed: {}", e)))?;

        tracing::debug!(
            silence_region_count = silence_regions.len(),
            "Detected silence regions"
        );

        // Convert silence regions to passage boundaries (regions between silences)
        let passages = self.silence_to_passages(&silence_regions, duration_ticks);

        // Calculate total non-silence duration from passages
        let total_non_silence_ticks: i64 = passages.iter().map(|p| p.duration_ticks()).sum();

        tracing::debug!(
            passage_count = passages.len(),
            total_non_silence_ticks,
            minimum_passage_audio_duration_ticks,
            "Calculated passage durations"
        );

        // Check for NO AUDIO condition
        if total_non_silence_ticks < minimum_passage_audio_duration_ticks {
            tracing::info!(
                file_id = %file_id,
                total_non_silence_ticks,
                "File has <100ms non-silence, marking as NO AUDIO"
            );

            // Update file status to NO AUDIO
            sqlx::query(
                "UPDATE files SET status = 'NO AUDIO', updated_at = CURRENT_TIMESTAMP WHERE guid = ?",
            )
            .bind(file_id.to_string())
            .execute(&self.db)
            .await?;

            return Ok(SegmentResult::NoAudio);
        }

        Ok(SegmentResult::Passages(passages))
    }

    /// Convert silence regions to passage boundaries
    ///
    /// **Algorithm:**
    /// - Passages are regions between silence
    /// - If no silence: entire file is one passage (0 to duration_ticks)
    /// - Otherwise: passages are gaps between silence regions
    fn silence_to_passages(
        &self,
        silence_regions: &[SilenceRegion],
        duration_ticks: i64,
    ) -> Vec<PassageBoundary> {
        if silence_regions.is_empty() {
            // No silence detected - entire file is one passage
            return vec![PassageBoundary::new(0, duration_ticks)];
        }

        let mut passages = Vec::new();
        let mut current_position_ticks = 0;

        for silence in silence_regions {
            let silence_start_ticks = (silence.start_seconds * TICKS_PER_SECOND as f32) as i64;
            let silence_end_ticks = (silence.end_seconds * TICKS_PER_SECOND as f32) as i64;

            // Passage before this silence
            if current_position_ticks < silence_start_ticks {
                passages.push(PassageBoundary::new(
                    current_position_ticks,
                    silence_start_ticks,
                ));
            }

            current_position_ticks = silence_end_ticks;
        }

        // Passage after last silence (if any)
        if current_position_ticks < duration_ticks {
            passages.push(PassageBoundary::new(current_position_ticks, duration_ticks));
        }

        passages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Setup in-memory test database with files and settings tables
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create files table
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration_ticks INTEGER,
                modification_time TIMESTAMP NOT NULL,
                status TEXT DEFAULT 'PENDING',
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create settings table
        sqlx::query(
            "CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert default settings
        sqlx::query("INSERT INTO settings (key, value) VALUES ('silence_threshold_dB', '35.0')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO settings (key, value) VALUES ('silence_min_duration_ticks', '8467200')") // 300ms
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO settings (key, value) VALUES ('minimum_passage_audio_duration_ticks', '2822400')") // 100ms
            .execute(&pool)
            .await
            .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_segmenter_creation() {
        let pool = setup_test_db().await;
        let _segmenter = PassageSegmenter::new(pool);
        // Just verify it can be created without panic
    }

    #[tokio::test]
    async fn test_segment_file_no_audio() {
        let pool = setup_test_db().await;
        let segmenter = PassageSegmenter::new(pool.clone());

        let file_id = Uuid::new_v4();

        // Create audio: 600ms total with only brief moments of sound
        // Pattern: 500ms silence, 1ms sound, 99ms silence (total 600ms)
        // Total non-silence: ~1ms < 100ms threshold → should be NO AUDIO
        let sample_rate = 44100;
        let mut samples = Vec::new();

        // 500ms of silence
        for _ in 0..(sample_rate / 2) {
            samples.push(0.0);
        }

        // 1ms of loud sound (above -35dB threshold)
        for _ in 0..(sample_rate / 1000) {
            samples.push(0.9);
        }

        // 99ms of silence
        for _ in 0..(sample_rate * 99 / 1000) {
            samples.push(0.0);
        }

        let duration_ticks = ((samples.len() as f32 / sample_rate as f32) * TICKS_PER_SECOND as f32) as i64;

        // Insert test file
        sqlx::query(
            "INSERT INTO files (guid, path, hash, duration_ticks, modification_time, status) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, 'PROCESSING')",
        )
        .bind(file_id.to_string())
        .bind("/test/file.mp3")
        .bind("test_hash")
        .bind(duration_ticks)
        .execute(&pool)
        .await
        .unwrap();

        let result = segmenter
            .segment_file(
                file_id,
                Path::new("/test/file.mp3"),
                &samples,
                sample_rate,
                duration_ticks,
            )
            .await
            .unwrap();

        // Should be marked as NO AUDIO (only ~1ms of non-silence < 100ms threshold)
        assert_eq!(result, SegmentResult::NoAudio);

        // Verify file status updated
        let status: String = sqlx::query_scalar("SELECT status FROM files WHERE guid = ?")
            .bind(file_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(status, "NO AUDIO");
    }

    #[tokio::test]
    async fn test_segment_file_single_passage() {
        let pool = setup_test_db().await;
        let segmenter = PassageSegmenter::new(pool.clone());

        let file_id = Uuid::new_v4();

        // Insert test file
        sqlx::query(
            "INSERT INTO files (guid, path, hash, duration_ticks, modification_time, status) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, 'PROCESSING')",
        )
        .bind(file_id.to_string())
        .bind("/test/file2.mp3")
        .bind("test_hash2")
        .bind(28224000) // 1 second
        .execute(&pool)
        .await
        .unwrap();

        // Create audio with continuous sound (no silence)
        let sample_rate = 44100;
        let samples: Vec<f32> = vec![0.5; sample_rate]; // 1 second of sound

        let result = segmenter
            .segment_file(
                file_id,
                Path::new("/test/file2.mp3"),
                &samples,
                sample_rate,
                28224000, // 1 second
            )
            .await
            .unwrap();

        // Should have one passage (entire file)
        match result {
            SegmentResult::Passages(passages) => {
                assert_eq!(passages.len(), 1);
                assert_eq!(passages[0].start_ticks, 0);
                assert_eq!(passages[0].duration_ticks(), 28224000);
            }
            SegmentResult::NoAudio => panic!("Expected passages, got NoAudio"),
        }
    }

    #[tokio::test]
    async fn test_silence_to_passages_no_silence() {
        let pool = setup_test_db().await;
        let segmenter = PassageSegmenter::new(pool);

        let passages = segmenter.silence_to_passages(&[], 100000);

        assert_eq!(passages.len(), 1);
        assert_eq!(passages[0].start_ticks, 0);
        assert_eq!(passages[0].end_ticks, 100000);
    }

    #[tokio::test]
    async fn test_silence_to_passages_with_silence() {
        let pool = setup_test_db().await;
        let segmenter = PassageSegmenter::new(pool);

        // Silence from 1.0s to 2.0s in a 5-second file
        let silence_regions = vec![SilenceRegion::new(1.0, 2.0)];

        let passages = segmenter.silence_to_passages(&silence_regions, 5 * TICKS_PER_SECOND);

        // Should have 2 passages: 0-1s and 2-5s
        assert_eq!(passages.len(), 2);
        assert_eq!(passages[0].start_ticks, 0);
        assert_eq!(passages[0].end_ticks, TICKS_PER_SECOND); // 1 second
        assert_eq!(passages[1].start_ticks, 2 * TICKS_PER_SECOND);
        assert_eq!(passages[1].end_ticks, 5 * TICKS_PER_SECOND);
    }

    #[tokio::test]
    async fn test_passage_boundary_duration() {
        let passage = PassageBoundary::new(1000, 5000);
        assert_eq!(passage.duration_ticks(), 4000);
    }
}

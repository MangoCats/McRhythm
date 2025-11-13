//! Passage Recording for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-014] Recording (Phase 7)
//!
//! Writes passages to database and creates song/artist relationships.
//! Uses atomic transactions to ensure data consistency.

use sqlx::{Pool, Sqlite};
use uuid::Uuid;
use wkmp_common::{Error, Result};

use super::passage_segmenter::PassageBoundary;
use super::passage_song_matcher::{ConfidenceLevel, PassageSongMatch};

/// Recording result for a passage
#[derive(Debug, Clone)]
pub struct PassageRecord {
    /// Passage GUID (database primary key)
    pub passage_id: Uuid,
    /// Song GUID (None for zero-song passages)
    pub song_id: Option<Uuid>,
    /// Whether song was newly created
    pub song_created: bool,
}

/// Recording result
#[derive(Debug, Clone)]
pub struct RecordingResult {
    /// Recorded passages
    pub passages: Vec<PassageRecord>,
    /// Statistics
    pub stats: RecordingStats,
}

/// Recording statistics
#[derive(Debug, Clone)]
pub struct RecordingStats {
    /// Total passages recorded
    pub passages_recorded: usize,
    /// Passages with songs
    pub passages_with_songs: usize,
    /// Zero-song passages
    pub zero_song_passages: usize,
    /// New songs created
    pub songs_created: usize,
    /// Existing songs reused
    pub songs_reused: usize,
}

/// Passage Recorder
///
/// **Traceability:** [REQ-SPEC032-014] (Phase 7: RECORDING)
pub struct PassageRecorder {
    db: Pool<Sqlite>,
}

impl PassageRecorder {
    /// Create new passage recorder
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    /// Record passages to database
    ///
    /// **Algorithm:**
    /// 1. Begin atomic transaction
    /// 2. For each passage match:
    ///    a. If has MBID: Get or create song
    ///    b. Create passage record (song_id = NULL for zero-song)
    ///    c. Set passage.status = 'PENDING' (awaiting Phase 8 amplitude analysis)
    /// 3. Commit transaction
    /// 4. Return recording result with statistics
    ///
    /// **Traceability:** [REQ-SPEC032-014]
    pub async fn record_passages(
        &self,
        file_id: Uuid,
        matches: &[PassageSongMatch],
    ) -> Result<RecordingResult> {
        tracing::debug!(
            file_id = %file_id,
            passage_count = matches.len(),
            "Recording passages to database"
        );

        let mut tx = self.db.begin().await?;
        let mut passages = Vec::new();
        let mut stats = RecordingStats {
            passages_recorded: 0,
            passages_with_songs: 0,
            zero_song_passages: 0,
            songs_created: 0,
            songs_reused: 0,
        };

        for (idx, match_item) in matches.iter().enumerate() {
            // Get or create song if MBID present
            let (song_id, song_created) = if let Some(ref mbid) = match_item.mbid {
                let (id, created) = self.get_or_create_song(&mut tx, mbid, &match_item.title).await?;
                if created {
                    stats.songs_created += 1;
                } else {
                    stats.songs_reused += 1;
                }
                stats.passages_with_songs += 1;
                (Some(id), created)
            } else {
                // Zero-song passage
                stats.zero_song_passages += 1;
                (None, false)
            };

            // Create passage record
            let passage_id = Uuid::new_v4();

            sqlx::query(
                r#"
                INSERT INTO passages (
                    guid, file_id, start_time_ticks, end_time_ticks,
                    song_id, title, status,
                    created_at, updated_at
                )
                VALUES (?, ?, ?, ?, ?, ?, 'PENDING', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                "#
            )
            .bind(passage_id.to_string())
            .bind(file_id.to_string())
            .bind(match_item.passage.start_ticks)
            .bind(match_item.passage.end_ticks)
            .bind(song_id.as_ref().map(|id| id.to_string()))
            .bind(match_item.title.as_ref())
            .execute(&mut *tx)
            .await?;

            tracing::debug!(
                passage_id = %passage_id,
                passage_idx = idx,
                song_id = ?song_id,
                confidence = %match_item.confidence.as_str(),
                "Recorded passage"
            );

            passages.push(PassageRecord {
                passage_id,
                song_id,
                song_created,
            });

            stats.passages_recorded += 1;
        }

        // Commit transaction
        tx.commit().await?;

        tracing::info!(
            file_id = %file_id,
            passages_recorded = stats.passages_recorded,
            songs_created = stats.songs_created,
            zero_song_passages = stats.zero_song_passages,
            "Recording complete"
        );

        Ok(RecordingResult { passages, stats })
    }

    /// Get existing song or create new one
    ///
    /// Returns (song_id, created)
    async fn get_or_create_song(
        &self,
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        mbid: &str,
        title: &Option<String>,
    ) -> Result<(Uuid, bool)> {
        // Check if song already exists
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT guid FROM songs WHERE recording_mbid = ?"
        )
        .bind(mbid)
        .fetch_optional(&mut **tx)
        .await?;

        if let Some((guid_str,)) = existing {
            let song_id = Uuid::parse_str(&guid_str)
                .map_err(|e| Error::Internal(format!("Invalid song GUID: {}", e)))?;

            tracing::debug!(
                song_id = %song_id,
                mbid,
                "Reusing existing song"
            );

            return Ok((song_id, false));
        }

        // Create new song
        let song_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO songs (
                guid, recording_mbid, base_probability,
                min_cooldown, ramping_cooldown, status,
                created_at, updated_at
            )
            VALUES (?, ?, 1.0, 604800, 1209600, 'PENDING', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            "#
        )
        .bind(song_id.to_string())
        .bind(mbid)
        .execute(&mut **tx)
        .await?;

        tracing::debug!(
            song_id = %song_id,
            mbid,
            title = ?title,
            "Created new song"
        );

        Ok((song_id, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Setup in-memory test database with passages and songs tables
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create files table
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create songs table
        sqlx::query(
            r#"
            CREATE TABLE songs (
                guid TEXT PRIMARY KEY,
                recording_mbid TEXT NOT NULL,
                base_probability REAL NOT NULL DEFAULT 1.0,
                min_cooldown INTEGER NOT NULL DEFAULT 604800,
                ramping_cooldown INTEGER NOT NULL DEFAULT 1209600,
                status TEXT DEFAULT 'PENDING',
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create passages table
        sqlx::query(
            r#"
            CREATE TABLE passages (
                guid TEXT PRIMARY KEY,
                file_id TEXT NOT NULL,
                start_time_ticks INTEGER NOT NULL,
                end_time_ticks INTEGER NOT NULL,
                song_id TEXT,
                title TEXT,
                status TEXT DEFAULT 'PENDING',
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_recorder_creation() {
        let pool = setup_test_db().await;
        let _recorder = PassageRecorder::new(pool);
        // Just verify it can be created
    }

    #[tokio::test]
    async fn test_record_single_passage_with_song() {
        let pool = setup_test_db().await;
        let recorder = PassageRecorder::new(pool.clone());

        let file_id = Uuid::new_v4();

        // Insert test file
        sqlx::query("INSERT INTO files (guid, path, hash) VALUES (?, ?, ?)")
            .bind(file_id.to_string())
            .bind("/test/file.mp3")
            .bind("test_hash")
            .execute(&pool)
            .await
            .unwrap();

        let matches = vec![PassageSongMatch {
            passage: PassageBoundary::new(0, 100000),
            mbid: Some("mbid-123".to_string()),
            confidence: ConfidenceLevel::High,
            score: 0.95,
            title: Some("Test Song".to_string()),
        }];

        let result = recorder.record_passages(file_id, &matches).await.unwrap();

        assert_eq!(result.stats.passages_recorded, 1);
        assert_eq!(result.stats.passages_with_songs, 1);
        assert_eq!(result.stats.zero_song_passages, 0);
        assert_eq!(result.stats.songs_created, 1);

        // Verify passage was written
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM passages WHERE file_id = ?")
            .bind(file_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(count, 1);

        // Verify song was created
        let song_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM songs WHERE recording_mbid = ?")
            .bind("mbid-123")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(song_count, 1);
    }

    #[tokio::test]
    async fn test_record_zero_song_passage() {
        let pool = setup_test_db().await;
        let recorder = PassageRecorder::new(pool.clone());

        let file_id = Uuid::new_v4();

        sqlx::query("INSERT INTO files (guid, path, hash) VALUES (?, ?, ?)")
            .bind(file_id.to_string())
            .bind("/test/file.mp3")
            .bind("test_hash")
            .execute(&pool)
            .await
            .unwrap();

        let matches = vec![PassageSongMatch {
            passage: PassageBoundary::new(0, 100000),
            mbid: None,
            confidence: ConfidenceLevel::None,
            score: 0.0,
            title: None,
        }];

        let result = recorder.record_passages(file_id, &matches).await.unwrap();

        assert_eq!(result.stats.passages_recorded, 1);
        assert_eq!(result.stats.passages_with_songs, 0);
        assert_eq!(result.stats.zero_song_passages, 1);
        assert_eq!(result.stats.songs_created, 0);

        // Verify passage has NULL song_id
        let song_id: Option<String> = sqlx::query_scalar(
            "SELECT song_id FROM passages WHERE file_id = ?"
        )
        .bind(file_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(song_id, None);
    }

    #[tokio::test]
    async fn test_record_reuse_existing_song() {
        let pool = setup_test_db().await;
        let recorder = PassageRecorder::new(pool.clone());

        let file_id = Uuid::new_v4();

        sqlx::query("INSERT INTO files (guid, path, hash) VALUES (?, ?, ?)")
            .bind(file_id.to_string())
            .bind("/test/file.mp3")
            .bind("test_hash")
            .execute(&pool)
            .await
            .unwrap();

        // Record first passage (creates song)
        let matches1 = vec![PassageSongMatch {
            passage: PassageBoundary::new(0, 100000),
            mbid: Some("mbid-reuse".to_string()),
            confidence: ConfidenceLevel::High,
            score: 0.95,
            title: Some("Test Song".to_string()),
        }];

        let result1 = recorder.record_passages(file_id, &matches1).await.unwrap();
        assert_eq!(result1.stats.songs_created, 1);

        // Record second passage (reuses song)
        let matches2 = vec![PassageSongMatch {
            passage: PassageBoundary::new(100000, 200000),
            mbid: Some("mbid-reuse".to_string()),
            confidence: ConfidenceLevel::High,
            score: 0.92,
            title: Some("Test Song".to_string()),
        }];

        let result2 = recorder.record_passages(file_id, &matches2).await.unwrap();
        assert_eq!(result2.stats.songs_created, 0);
        assert_eq!(result2.stats.songs_reused, 1);

        // Verify only one song exists
        let song_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM songs WHERE recording_mbid = ?")
            .bind("mbid-reuse")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(song_count, 1);

        // Verify both passages reference the same song
        let passages: Vec<(String,)> = sqlx::query_as(
            "SELECT song_id FROM passages WHERE file_id = ? ORDER BY start_time_ticks"
        )
        .bind(file_id.to_string())
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(passages.len(), 2);
        assert_eq!(passages[0].0, passages[1].0); // Same song_id
    }
}

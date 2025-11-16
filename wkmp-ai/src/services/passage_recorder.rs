//! Passage Recording for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-014] Recording (Phase 7)
//!
//! Writes passages to database and creates song/artist relationships.
//! Uses atomic transactions to ensure data consistency.
//! **[ARCH-ERRH-070]** Retry logic for transient database lock errors.

use sqlx::{Pool, Sqlite};
use uuid::Uuid;
use wkmp_common::{Error, Result};
use crate::utils::{retry_on_lock, begin_monitored};
use std::collections::HashMap;

use super::passage_song_matcher::PassageSongMatch;

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
    /// 1. Pre-fetch all unique MBIDs from passage matches
    /// 2. Batch query existing songs (OUTSIDE transaction - reduces lock time)
    /// 3. Begin atomic transaction
    /// 4. Insert new songs (INSERT-only, no SELECT)
    /// 5. Insert passages (INSERT-only, no SELECT)
    /// 6. Commit transaction
    /// 7. Return recording result with statistics
    ///
    /// **Performance Optimization:**
    /// - Song queries moved OUTSIDE transaction to reduce connection hold time
    /// - Transaction only contains fast INSERT operations
    /// - Target: <100ms transaction hold time (vs. 9,500ms in previous implementation)
    ///
    /// **Traceability:** [REQ-SPEC032-014]
    /// **[ARCH-ERRH-070]** Retry logic for transient database lock errors
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

        // Get max lock wait time from settings (default 5000ms)
        let max_wait_ms: i64 = sqlx::query_scalar(
            "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'ai_database_max_lock_wait_ms'"
        )
        .fetch_optional(&self.db)
        .await?
        .unwrap_or(5000);

        // Step 1: Pre-fetch all unique MBIDs from passage matches (OUTSIDE transaction)
        let unique_mbids: Vec<String> = matches
            .iter()
            .filter_map(|m| m.mbid.as_ref())
            .map(|mbid| mbid.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        tracing::debug!(
            unique_mbid_count = unique_mbids.len(),
            "Pre-fetching existing songs"
        );

        // Step 2: Batch query existing songs (OUTSIDE transaction)
        let existing_songs = Self::batch_query_existing_songs(&self.db, &unique_mbids).await?;

        tracing::debug!(
            existing_song_count = existing_songs.len(),
            songs_to_create = unique_mbids.len() - existing_songs.len(),
            "Existing songs queried"
        );

        // Wrap transaction in retry logic
        let matches_vec = matches.to_vec();
        let existing_songs_map = existing_songs; // Already a HashMap

        // **[IMPL001]** Wrap entire retry_on_lock in unconstrained() to prevent Tokio from
        // interrupting both connection acquisition AND transaction execution.
        // Without this, Tokio can switch tasks during pool.begin().await, causing 14+ second
        // waits when all 20 pool connections are held by CPU-intensive tasks.
        tokio::task::unconstrained(
            retry_on_lock(
                "passage recording",
                max_wait_ms as u64,
                || {
                    let matches_ref = &matches_vec;
                    let db_ref = &self.db;
                    let existing_ref = &existing_songs_map;
                    async move {
                        // Step 3: Begin transaction (all song lookups already done)
                        tracing::debug!("Beginning monitored transaction for passage recording");
                        let mut tx = begin_monitored(db_ref, "passage_recorder::record").await?;
                        tracing::debug!("Transaction acquired, starting passage recording loop");
                    let mut passages = Vec::new();
                    let mut stats = RecordingStats {
                        passages_recorded: 0,
                        passages_with_songs: 0,
                        zero_song_passages: 0,
                        songs_created: 0,
                        songs_reused: 0,
                    };

                    // Track newly created songs in this transaction
                    let mut newly_created_songs: HashMap<String, Uuid> = HashMap::new();

                    // **[PERF-FIX]** Prepare all passage data first, then batch insert
                    // This avoids 20+ individual .await points that can yield
                    let mut passage_data = Vec::new();

                    for (idx, match_item) in matches_ref.iter().enumerate() {
                        // Get or create song if MBID present
                        let (song_id, song_created) = if let Some(ref mbid) = match_item.mbid {
                            // Check existing songs first
                            if let Some(&existing_id) = existing_ref.get(mbid) {
                                stats.songs_reused += 1;
                                stats.passages_with_songs += 1;
                                (Some(existing_id), false)
                            }
                            // Check if we already created this song in this transaction
                            else if let Some(&created_id) = newly_created_songs.get(mbid) {
                                stats.songs_reused += 1;
                                stats.passages_with_songs += 1;
                                (Some(created_id), false)
                            }
                            // Create new song (INSERT only, no SELECT)
                            else {
                                let song_id = Uuid::new_v4();

                                tracing::trace!(
                                    song_id = %song_id,
                                    mbid,
                                    "Executing INSERT for new song"
                                );

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
                                .execute(&mut **tx.inner_mut())
                                .await
                                .map_err(|e| {
                                    tracing::error!(
                                        song_id = %song_id,
                                        mbid,
                                        error = %e,
                                        "Song INSERT failed"
                                    );
                                    wkmp_common::Error::Database(e)
                                })?;

                                tracing::debug!(
                                    song_id = %song_id,
                                    mbid,
                                    title = ?match_item.title,
                                    "Created new song"
                                );

                                newly_created_songs.insert(mbid.clone(), song_id);
                                stats.songs_created += 1;
                                stats.passages_with_songs += 1;
                                (Some(song_id), true)
                            }
                        } else {
                            // Zero-song passage
                            stats.zero_song_passages += 1;
                            (None, false)
                        };

                        // Prepare passage data (don't insert yet)
                        let passage_id = Uuid::new_v4();

                        passage_data.push((
                            passage_id,
                            song_id,
                            song_created,
                            match_item.clone(),
                            idx,
                        ));

                        stats.passages_recorded += 1;
                    }

                    // **[PERF-FIX]** Batch insert all passages with a single multi-row INSERT
                    // This reduces from 20+ .await points to just 1
                    if !passage_data.is_empty() {
                        tracing::debug!(
                            passage_count = passage_data.len(),
                            "Batch inserting passages"
                        );

                        // Build multi-row INSERT statement
                        let values_clause = passage_data
                            .iter()
                            .map(|_| "(?, ?, ?, ?, ?, ?, 'PENDING', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                            .collect::<Vec<_>>()
                            .join(", ");

                        let insert_sql = format!(
                            r#"
                            INSERT INTO passages (
                                guid, file_id, start_time_ticks, end_time_ticks,
                                song_id, title, status,
                                created_at, updated_at
                            )
                            VALUES {}
                            "#,
                            values_clause
                        );

                        let mut query = sqlx::query(&insert_sql);

                        for (passage_id, song_id, _, match_item, _) in &passage_data {
                            query = query
                                .bind(passage_id.to_string())
                                .bind(file_id.to_string())
                                .bind(match_item.passage.start_ticks)
                                .bind(match_item.passage.end_ticks)
                                .bind(song_id.as_ref().map(|id| id.to_string()))
                                .bind(match_item.title.as_ref());
                        }

                        query
                            .execute(&mut **tx.inner_mut())
                            .await
                            .map_err(|e| {
                                tracing::error!(
                                    error = %e,
                                    passage_count = passage_data.len(),
                                    "Batch passage INSERT failed"
                                );
                                wkmp_common::Error::Database(e)
                            })?;

                        tracing::debug!(
                            passage_count = passage_data.len(),
                            "Batch insert complete"
                        );

                        // Build result records
                        for (passage_id, song_id, song_created, match_item, idx) in passage_data {
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
                        }
                    }

                    // Commit transaction (logs connection release timing)
                    tracing::debug!("Committing transaction for passage recording");
                    tx.commit().await.map_err(|e| {
                        tracing::error!(
                            error = %e,
                            "Transaction COMMIT failed"
                        );
                        e
                    })?;
                    tracing::debug!("Database transaction committed for passage recording");

                    tracing::info!(
                        file_id = %file_id,
                        passages_recorded = stats.passages_recorded,
                        songs_created = stats.songs_created,
                        zero_song_passages = stats.zero_song_passages,
                        "Recording complete"
                    );

                    Ok(RecordingResult { passages, stats })
                    }
                }
            )
        ).await // Close unconstrained() wrapper around retry_on_lock
    }

    /// Batch query existing songs by MBIDs
    ///
    /// **Performance:** Runs OUTSIDE transaction to avoid holding connection during query.
    /// Returns HashMap of MBID -> song_id for reuse.
    async fn batch_query_existing_songs(
        db: &Pool<Sqlite>,
        mbids: &[String],
    ) -> Result<HashMap<String, Uuid>> {
        if mbids.is_empty() {
            return Ok(HashMap::new());
        }

        // Build parameterized query with placeholders
        let placeholders = mbids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query_sql = format!(
            "SELECT guid, recording_mbid FROM songs WHERE recording_mbid IN ({})",
            placeholders
        );

        let mut query = sqlx::query_as::<_, (String, String)>(&query_sql);
        for mbid in mbids {
            query = query.bind(mbid);
        }

        let rows = query.fetch_all(db).await?;

        let mut map = HashMap::new();
        for (guid_str, mbid) in rows {
            let song_id = Uuid::parse_str(&guid_str)
                .map_err(|e| Error::Internal(format!("Invalid song GUID: {}", e)))?;
            map.insert(mbid, song_id);
        }

        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Setup in-memory test database with passages and songs tables
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create settings table
        sqlx::query(
            r#"
            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert ai_database_max_lock_wait_ms setting
        sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_max_lock_wait_ms', '5000')")
            .execute(&pool)
            .await
            .unwrap();

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

//! Passage Finalization for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-017] Finalization (Phase 10)
//!
//! Finalizes import by validating all passages are complete and marking file as 'INGEST COMPLETE'.

use sqlx::{Pool, Sqlite};
use uuid::Uuid;
use wkmp_common::Result;

/// Finalization result
#[derive(Debug, Clone)]
pub struct FinalizationResult {
    /// File GUID
    pub file_id: Uuid,
    /// Number of passages validated
    pub passages_validated: usize,
    /// Whether finalization succeeded
    pub success: bool,
    /// Validation errors (if any)
    pub errors: Vec<String>,
}

/// Passage Finalizer
///
/// **Traceability:** [REQ-SPEC032-017] (Phase 10: FINALIZATION)
pub struct PassageFinalizer {
    db: Pool<Sqlite>,
}

impl PassageFinalizer {
    /// Create new passage finalizer
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    /// Finalize import for a file
    ///
    /// **Algorithm:**
    /// 1. Validate all passages have status = 'INGEST COMPLETE'
    /// 2. Validate all songs have status = 'FLAVOR READY' (or NULL for zero-song passages)
    /// 3. If validation passes:
    ///    a. Mark files.status = 'INGEST COMPLETE'
    ///    b. Update files.updated_at = CURRENT_TIMESTAMP
    /// 4. Return finalization result
    ///
    /// **Traceability:** [REQ-SPEC032-017]
    pub async fn finalize(&self, file_id: Uuid) -> Result<FinalizationResult> {
        tracing::debug!(
            file_id = %file_id,
            "Finalizing import"
        );

        let mut errors = Vec::new();

        // Validate all passages have status = 'INGEST COMPLETE'
        let pending_passages: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM passages WHERE file_id = ? AND status != 'INGEST COMPLETE'"
        )
        .bind(file_id.to_string())
        .fetch_one(&self.db)
        .await?;

        if pending_passages > 0 {
            let error_msg = format!(
                "Validation failed: {} passages do not have status = 'INGEST COMPLETE'",
                pending_passages
            );
            tracing::error!(file_id = %file_id, pending_passages, "{}", error_msg);
            errors.push(error_msg);
        }

        // Validate all songs have status = 'FLAVOR READY' (for passages with song_id)
        let pending_songs: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT songs.guid)
            FROM passages
            JOIN songs ON passages.song_id = songs.guid
            WHERE passages.file_id = ?
              AND songs.status != 'FLAVOR READY'
            "#
        )
        .bind(file_id.to_string())
        .fetch_one(&self.db)
        .await?;

        if pending_songs > 0 {
            let error_msg = format!(
                "Validation failed: {} songs do not have status = 'FLAVOR READY'",
                pending_songs
            );
            tracing::error!(file_id = %file_id, pending_songs, "{}", error_msg);
            errors.push(error_msg);
        }

        // Get total passage count for result
        let passages_validated: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM passages WHERE file_id = ?"
        )
        .bind(file_id.to_string())
        .fetch_one(&self.db)
        .await?;

        // If validation passed, mark file as complete
        let success = errors.is_empty();
        if success {
            sqlx::query(
                r#"
                UPDATE files
                SET status = 'INGEST COMPLETE',
                    updated_at = CURRENT_TIMESTAMP
                WHERE guid = ?
                "#
            )
            .bind(file_id.to_string())
            .execute(&self.db)
            .await?;

            tracing::info!(
                file_id = %file_id,
                passages = passages_validated,
                "Import finalization complete"
            );
        } else {
            tracing::error!(
                file_id = %file_id,
                error_count = errors.len(),
                "Import finalization failed"
            );
        }

        Ok(FinalizationResult {
            file_id,
            passages_validated: passages_validated as usize,
            success,
            errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Setup in-memory test database
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create files table
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                status TEXT DEFAULT 'PENDING',
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
    async fn test_finalizer_creation() {
        let pool = setup_test_db().await;
        let _finalizer = PassageFinalizer::new(pool);
        // Just verify it can be created
    }

    #[tokio::test]
    async fn test_finalize_success() {
        let pool = setup_test_db().await;
        let finalizer = PassageFinalizer::new(pool.clone());

        let file_id = Uuid::new_v4();
        let song_id = Uuid::new_v4();

        // Insert test file
        sqlx::query("INSERT INTO files (guid, path, hash, status) VALUES (?, ?, ?, 'PROCESSING')")
            .bind(file_id.to_string())
            .bind("/test/file.mp3")
            .bind("test_hash")
            .execute(&pool)
            .await
            .unwrap();

        // Insert test song with FLAVOR READY
        sqlx::query("INSERT INTO songs (guid, recording_mbid, status) VALUES (?, ?, 'FLAVOR READY')")
            .bind(song_id.to_string())
            .bind("mbid-123")
            .execute(&pool)
            .await
            .unwrap();

        // Insert test passage with INGEST COMPLETE
        sqlx::query(
            "INSERT INTO passages (guid, file_id, start_time_ticks, end_time_ticks, song_id, status) VALUES (?, ?, ?, ?, ?, 'INGEST COMPLETE')"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(file_id.to_string())
        .bind(0)
        .bind(100000)
        .bind(song_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let result = finalizer.finalize(file_id).await.unwrap();

        assert!(result.success);
        assert_eq!(result.passages_validated, 1);
        assert_eq!(result.errors.len(), 0);

        // Verify file status updated
        let file_status: String = sqlx::query_scalar("SELECT status FROM files WHERE guid = ?")
            .bind(file_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(file_status, "INGEST COMPLETE");
    }

    #[tokio::test]
    async fn test_finalize_pending_passage() {
        let pool = setup_test_db().await;
        let finalizer = PassageFinalizer::new(pool.clone());

        let file_id = Uuid::new_v4();

        // Insert test file
        sqlx::query("INSERT INTO files (guid, path, hash, status) VALUES (?, ?, ?, 'PROCESSING')")
            .bind(file_id.to_string())
            .bind("/test/file.mp3")
            .bind("test_hash")
            .execute(&pool)
            .await
            .unwrap();

        // Insert test passage with PENDING status
        sqlx::query(
            "INSERT INTO passages (guid, file_id, start_time_ticks, end_time_ticks, status) VALUES (?, ?, ?, ?, 'PENDING')"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(file_id.to_string())
        .bind(0)
        .bind(100000)
        .execute(&pool)
        .await
        .unwrap();

        let result = finalizer.finalize(file_id).await.unwrap();

        assert!(!result.success);
        assert_eq!(result.passages_validated, 1);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("passages do not have status = 'INGEST COMPLETE'"));

        // Verify file status NOT updated
        let file_status: String = sqlx::query_scalar("SELECT status FROM files WHERE guid = ?")
            .bind(file_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(file_status, "PROCESSING");
    }

    #[tokio::test]
    async fn test_finalize_zero_song_passage() {
        let pool = setup_test_db().await;
        let finalizer = PassageFinalizer::new(pool.clone());

        let file_id = Uuid::new_v4();

        // Insert test file
        sqlx::query("INSERT INTO files (guid, path, hash, status) VALUES (?, ?, ?, 'PROCESSING')")
            .bind(file_id.to_string())
            .bind("/test/file.mp3")
            .bind("test_hash")
            .execute(&pool)
            .await
            .unwrap();

        // Insert test passage with INGEST COMPLETE and NULL song_id (zero-song passage)
        sqlx::query(
            "INSERT INTO passages (guid, file_id, start_time_ticks, end_time_ticks, song_id, status) VALUES (?, ?, ?, ?, NULL, 'INGEST COMPLETE')"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(file_id.to_string())
        .bind(0)
        .bind(100000)
        .execute(&pool)
        .await
        .unwrap();

        let result = finalizer.finalize(file_id).await.unwrap();

        // Should succeed (zero-song passages don't need songs to have FLAVOR READY)
        assert!(result.success);
        assert_eq!(result.passages_validated, 1);
        assert_eq!(result.errors.len(), 0);
    }
}

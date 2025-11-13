//! Filename Matching for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-008] Filename Matching (Phase 1)
//!
//! Checks if a file path already exists in the database to determine whether
//! this is a new file, an existing file that should be updated, or a file that
//! has already been processed.

use sqlx::{Pool, Sqlite};
use std::path::Path;
use uuid::Uuid;
use wkmp_common::{Error, Result};

/// Filename matching result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchResult {
    /// File not found in database - create new fileId
    New,
    /// File exists with status INGEST COMPLETE - skip processing
    AlreadyProcessed(Uuid),
    /// File exists but not yet processed - reuse fileId and update metadata
    Reuse(Uuid),
}

/// Filename Matcher
///
/// **Traceability:** [REQ-SPEC032-008] (Phase 1: Filename Matching)
pub struct FilenameMatcher {
    db: Pool<Sqlite>,
}

impl FilenameMatcher {
    /// Create new filename matcher
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    /// Check if file path exists in database
    ///
    /// **Algorithm:**
    /// 1. Query database for file by path
    /// 2. If not found: Return MatchResult::New
    /// 3. If found with status "INGEST COMPLETE": Return MatchResult::AlreadyProcessed(guid)
    /// 4. If found with other status: Return MatchResult::Reuse(guid)
    ///
    /// **Traceability:** [REQ-SPEC032-008]
    pub async fn check_file(&self, file_path: &Path) -> Result<MatchResult> {
        // Convert path to string (relative to root folder, forward slashes)
        let path_str = file_path
            .to_str()
            .ok_or_else(|| Error::InvalidInput("Invalid UTF-8 in file path".to_string()))?
            .replace('\\', "/");

        tracing::debug!(path = %path_str, "Checking if file exists in database");

        // Query database for file by path
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT guid, status FROM files WHERE path = ?",
        )
        .bind(&path_str)
        .fetch_optional(&self.db)
        .await?;

        match row {
            None => {
                tracing::debug!(path = %path_str, "File not found in database");
                Ok(MatchResult::New)
            }
            Some((guid_str, status)) => {
                let guid = Uuid::parse_str(&guid_str)
                    .map_err(|e| Error::Internal(format!("Invalid UUID in database: {}", e)))?;

                if status == "INGEST COMPLETE" {
                    tracing::debug!(
                        path = %path_str,
                        guid = %guid,
                        "File already processed (INGEST COMPLETE)"
                    );
                    Ok(MatchResult::AlreadyProcessed(guid))
                } else {
                    tracing::debug!(
                        path = %path_str,
                        guid = %guid,
                        status = %status,
                        "File exists, reusing fileId"
                    );
                    Ok(MatchResult::Reuse(guid))
                }
            }
        }
    }

    /// Create new file record in database
    ///
    /// **Parameters:**
    /// - `file_path`: File path relative to root folder
    /// - `modification_time`: File last modified timestamp (Unix timestamp)
    ///
    /// **Returns:** New file UUID
    ///
    /// **Traceability:** [REQ-SPEC032-008]
    pub async fn create_file_record(
        &self,
        file_path: &Path,
        modification_time: i64,
    ) -> Result<Uuid> {
        let guid = Uuid::new_v4();
        let path_str = file_path
            .to_str()
            .ok_or_else(|| Error::InvalidInput("Invalid UTF-8 in file path".to_string()))?
            .replace('\\', "/");

        // Create file record with PENDING status and temporary hash
        // Hash will be updated in Phase 2
        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, modification_time, status)
            VALUES (?, ?, 'PENDING', datetime(?, 'unixepoch'), 'PENDING')
            "#,
        )
        .bind(guid.to_string())
        .bind(&path_str)
        .bind(modification_time)
        .execute(&self.db)
        .await?;

        tracing::debug!(
            path = %path_str,
            guid = %guid,
            "Created new file record"
        );

        Ok(guid)
    }

    /// Update file status
    ///
    /// **Traceability:** [REQ-SPEC032-008]
    pub async fn update_file_status(&self, guid: Uuid, status: &str) -> Result<()> {
        sqlx::query("UPDATE files SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE guid = ?")
            .bind(status)
            .bind(guid.to_string())
            .execute(&self.db)
            .await?;

        tracing::debug!(guid = %guid, status = %status, "Updated file status");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Setup in-memory test database with files table
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create files table matching production schema
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration_ticks INTEGER,
                format TEXT,
                sample_rate INTEGER,
                channels INTEGER,
                file_size_bytes INTEGER,
                modification_time TIMESTAMP NOT NULL,
                status TEXT DEFAULT 'PENDING',
                matching_hashes TEXT,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_check_file_new() {
        let pool = setup_test_db().await;
        let matcher = FilenameMatcher::new(pool);

        let result = matcher
            .check_file(Path::new("music/test.mp3"))
            .await
            .unwrap();

        assert_eq!(result, MatchResult::New);
    }

    #[tokio::test]
    async fn test_check_file_already_processed() {
        let pool = setup_test_db().await;
        let matcher = FilenameMatcher::new(pool.clone());

        // Insert a file with INGEST COMPLETE status
        let guid = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, modification_time, status)
            VALUES (?, 'music/test.mp3', 'abc123', datetime('now'), 'INGEST COMPLETE')
            "#,
        )
        .bind(guid.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let result = matcher
            .check_file(Path::new("music/test.mp3"))
            .await
            .unwrap();

        assert_eq!(result, MatchResult::AlreadyProcessed(guid));
    }

    #[tokio::test]
    async fn test_check_file_reuse() {
        let pool = setup_test_db().await;
        let matcher = FilenameMatcher::new(pool.clone());

        // Insert a file with PENDING status
        let guid = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, modification_time, status)
            VALUES (?, 'music/test.mp3', 'abc123', datetime('now'), 'PENDING')
            "#,
        )
        .bind(guid.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let result = matcher
            .check_file(Path::new("music/test.mp3"))
            .await
            .unwrap();

        assert_eq!(result, MatchResult::Reuse(guid));
    }

    #[tokio::test]
    async fn test_create_file_record() {
        let pool = setup_test_db().await;
        let matcher = FilenameMatcher::new(pool.clone());

        let guid = matcher
            .create_file_record(Path::new("music/test.mp3"), 1234567890)
            .await
            .unwrap();

        // Verify file was created
        let (stored_guid, path, status): (String, String, String) = sqlx::query_as(
            "SELECT guid, path, status FROM files WHERE guid = ?",
        )
        .bind(guid.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(stored_guid, guid.to_string());
        assert_eq!(path, "music/test.mp3");
        assert_eq!(status, "PENDING");
    }

    #[tokio::test]
    async fn test_update_file_status() {
        let pool = setup_test_db().await;
        let matcher = FilenameMatcher::new(pool.clone());

        // Create a file record
        let guid = matcher
            .create_file_record(Path::new("music/test.mp3"), 1234567890)
            .await
            .unwrap();

        // Update status
        matcher
            .update_file_status(guid, "PROCESSING")
            .await
            .unwrap();

        // Verify status was updated
        let status: String =
            sqlx::query_scalar("SELECT status FROM files WHERE guid = ?")
                .bind(guid.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(status, "PROCESSING");
    }

    #[tokio::test]
    async fn test_path_normalization_windows() {
        let pool = setup_test_db().await;
        let matcher = FilenameMatcher::new(pool.clone());

        // Create file with forward slashes
        let guid = matcher
            .create_file_record(Path::new("music/test.mp3"), 1234567890)
            .await
            .unwrap();

        // Check with backslashes (Windows path)
        let result = matcher
            .check_file(Path::new("music\\test.mp3"))
            .await
            .unwrap();

        assert_eq!(result, MatchResult::Reuse(guid));
    }
}

//! Hash Deduplication for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-009] Hash Deduplication (Phase 2)
//!
//! Calculates SHA-256 hash of file content, detects duplicate hashes,
//! and creates bidirectional links between duplicate files.

use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use uuid::Uuid;
use wkmp_common::{Error, Result};
use crate::utils::{retry_on_lock, begin_monitored};

/// Hash deduplication result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashResult {
    /// Unique hash - continue processing
    Unique(String),
    /// Duplicate hash found - stop processing, link to original
    Duplicate {
        hash: String,
        original_file_id: Uuid,
    },
}

/// Hash Deduplicator
///
/// **Traceability:** [REQ-SPEC032-009] (Phase 2: Hash Deduplication)
pub struct HashDeduplicator {
    db: Pool<Sqlite>,
}

impl HashDeduplicator {
    /// Create new hash deduplicator
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    /// Calculate SHA-256 hash of file
    ///
    /// **Algorithm:**
    /// 1. Read file content in chunks (1MB at a time for memory efficiency)
    /// 2. Calculate SHA-256 hash
    /// 3. Return hex-encoded hash string
    ///
    /// **Traceability:** [REQ-SPEC032-009]
    pub async fn calculate_hash(&self, file_path: &Path) -> Result<String> {
        let path_buf = file_path.to_path_buf();
        tracing::debug!(path = %path_buf.display(), "Calculating SHA-256 hash");

        // Use tokio::task::spawn_blocking for CPU-intensive hash calculation
        let file_path_clone = path_buf.clone();
        let hash = tokio::task::spawn_blocking(move || -> Result<String> {
            use std::fs::File;
            use std::io::Read;

            let mut file = File::open(&file_path_clone).map_err(|e| {
                Error::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to open file for hashing: {}", e),
                ))
            })?;

            let mut hasher = Sha256::new();
            let mut buffer = vec![0u8; 1024 * 1024]; // 1MB chunks

            loop {
                let bytes_read = file.read(&mut buffer).map_err(|e| {
                    Error::Io(std::io::Error::new(
                        e.kind(),
                        format!("Failed to read file for hashing: {}", e),
                    ))
                })?;

                if bytes_read == 0 {
                    break;
                }

                hasher.update(&buffer[..bytes_read]);
            }

            let hash_bytes = hasher.finalize();
            let hash_hex = format!("{:x}", hash_bytes);

            Ok(hash_hex)
        })
        .await
        .map_err(|e| Error::Internal(format!("Hash calculation task failed: {}", e)))??;

        tracing::debug!(
            path = %path_buf.display(),
            hash_len = hash.len(),
            "Calculated hash"
        );

        Ok(hash)
    }

    /// Check for duplicate hash in database
    ///
    /// **Algorithm:**
    /// 1. Query database for other files with same hash
    /// 2. If no duplicates: Return HashResult::Unique(hash)
    /// 3. If duplicate found: Return HashResult::Duplicate with original file ID
    ///
    /// **Traceability:** [REQ-SPEC032-009]
    pub async fn check_duplicate(&self, hash: &str, current_file_id: Uuid) -> Result<HashResult> {
        tracing::debug!(hash = %hash, file_id = %current_file_id, "Checking for duplicate hash");

        // Query database for files with same hash (excluding current file)
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT guid FROM files WHERE hash = ? AND guid != ? LIMIT 1",
        )
        .bind(hash)
        .bind(current_file_id.to_string())
        .fetch_optional(&self.db)
        .await?;

        match row {
            None => {
                tracing::debug!(hash = %hash, "No duplicate hash found");
                Ok(HashResult::Unique(hash.to_string()))
            }
            Some((original_guid_str,)) => {
                let original_file_id = Uuid::parse_str(&original_guid_str)
                    .map_err(|e| Error::Internal(format!("Invalid UUID in database: {}", e)))?;

                tracing::info!(
                    hash = %hash,
                    current_file_id = %current_file_id,
                    original_file_id = %original_file_id,
                    "Duplicate hash detected"
                );

                Ok(HashResult::Duplicate {
                    hash: hash.to_string(),
                    original_file_id,
                })
            }
        }
    }

    /// Update file hash in database
    ///
    /// **Traceability:** [REQ-SPEC032-009]
    pub async fn update_file_hash(&self, file_id: Uuid, hash: &str) -> Result<()> {
        sqlx::query(
            "UPDATE files SET hash = ?, updated_at = CURRENT_TIMESTAMP WHERE guid = ?",
        )
        .bind(hash)
        .bind(file_id.to_string())
        .execute(&self.db)
        .await?;

        tracing::debug!(file_id = %file_id, hash = %hash, "Updated file hash");

        Ok(())
    }

    /// Create bidirectional duplicate link
    ///
    /// **Algorithm:**
    /// 1. Read matching_hashes JSON arrays from both files
    /// 2. Add current_file_id to original's matching_hashes
    /// 3. Add original_file_id to current's matching_hashes
    /// 4. Update both files in database
    /// 5. Set current file status to DUPLICATE HASH
    ///
    /// **Traceability:** [REQ-SPEC032-009]
    pub async fn link_duplicates(
        &self,
        current_file_id: Uuid,
        original_file_id: Uuid,
    ) -> Result<()> {
        tracing::debug!(
            current = %current_file_id,
            original = %original_file_id,
            "Creating bidirectional duplicate link"
        );

        // Get max lock wait time from settings (default 5000ms)
        let max_wait_ms: i64 = sqlx::query_scalar(
            "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'ai_database_max_lock_wait_ms'"
        )
        .fetch_optional(&self.db)
        .await?
        .unwrap_or(5000);

        // Wrap in retry logic with unconstrained execution
        let db_ref = &self.db;
        tokio::task::unconstrained(
            retry_on_lock(
                "hash deduplicator link",
                max_wait_ms as u64,
                || async {
                    // Start monitored transaction
                    let mut tx = begin_monitored(db_ref, "hash_deduplicator::link_duplicates").await?;

        // Read original file's matching_hashes
        let original_matches: Option<String> = sqlx::query_scalar(
            "SELECT matching_hashes FROM files WHERE guid = ?",
        )
        .bind(original_file_id.to_string())
        .fetch_one(&mut **tx.inner_mut())
        .await?;

        let mut original_array: Vec<String> = match original_matches {
            Some(json) if !json.is_empty() => {
                serde_json::from_str(&json).unwrap_or_else(|_| Vec::new())
            }
            _ => Vec::new(),
        };

        // Add current file to original's array (if not already present)
        let current_str = current_file_id.to_string();
        if !original_array.contains(&current_str) {
            original_array.push(current_str.clone());
        }

        // Read current file's matching_hashes
        let current_matches: Option<String> = sqlx::query_scalar(
            "SELECT matching_hashes FROM files WHERE guid = ?",
        )
        .bind(current_file_id.to_string())
        .fetch_one(&mut **tx.inner_mut())
        .await?;

        let mut current_array: Vec<String> = match current_matches {
            Some(json) if !json.is_empty() => {
                serde_json::from_str(&json).unwrap_or_else(|_| Vec::new())
            }
            _ => Vec::new(),
        };

        // Add original file to current's array (if not already present)
        let original_str = original_file_id.to_string();
        if !current_array.contains(&original_str) {
            current_array.push(original_str.clone());
        }

        // Update original file
        let original_json = serde_json::to_string(&original_array)
            .map_err(|e| Error::Internal(format!("Failed to serialize JSON: {}", e)))?;

        sqlx::query(
            "UPDATE files SET matching_hashes = ?, updated_at = CURRENT_TIMESTAMP WHERE guid = ?",
        )
        .bind(&original_json)
        .bind(original_file_id.to_string())
        .execute(&mut **tx.inner_mut())
        .await?;

        // Update current file (set matching_hashes AND status to DUPLICATE HASH)
        let current_json = serde_json::to_string(&current_array)
            .map_err(|e| Error::Internal(format!("Failed to serialize JSON: {}", e)))?;

        sqlx::query(
            "UPDATE files SET matching_hashes = ?, status = 'DUPLICATE HASH', updated_at = CURRENT_TIMESTAMP WHERE guid = ?",
        )
        .bind(&current_json)
        .bind(current_file_id.to_string())
        .execute(&mut **tx.inner_mut())
        .await?;

        // Commit transaction (logs connection release timing)
        tx.commit().await?;

        tracing::info!(
            current = %current_file_id,
            original = %original_file_id,
            "Bidirectional duplicate link created"
        );

        Ok(())
                }
            )
        ).await // Close unconstrained() wrapper around retry_on_lock
    }

    /// Process file hash (calculate, check duplicate, update database)
    ///
    /// **Complete Phase 2 workflow:**
    /// 1. Calculate SHA-256 hash
    /// 2. Update file record with hash
    /// 3. Check for duplicate
    /// 4. If duplicate: Create bidirectional link, return Duplicate
    /// 5. If unique: Return Unique
    ///
    /// **Traceability:** [REQ-SPEC032-009]
    pub async fn process_file_hash(
        &self,
        file_id: Uuid,
        file_path: &Path,
    ) -> Result<HashResult> {
        // Calculate hash
        let hash = self.calculate_hash(file_path).await?;

        // Update file record with hash
        self.update_file_hash(file_id, &hash).await?;

        // Check for duplicate
        let result = self.check_duplicate(&hash, file_id).await?;

        // If duplicate, create bidirectional link
        if let HashResult::Duplicate {
            original_file_id, ..
        } = &result
        {
            self.link_duplicates(file_id, *original_file_id).await?;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Setup in-memory test database with files table
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create settings table (required by hash deduplicator operations)
        wkmp_common::db::init::create_settings_table(&pool)
            .await
            .unwrap();

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
    async fn test_calculate_hash() {
        let pool = setup_test_db().await;
        let deduplicator = HashDeduplicator::new(pool);

        // Create temporary file with known content
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();
        temp_file.flush().unwrap();

        let hash = deduplicator
            .calculate_hash(temp_file.path())
            .await
            .unwrap();

        // Verify hash is SHA-256 hex string (64 characters)
        assert_eq!(hash.len(), 64);

        // Calculate expected hash
        let expected_hash = format!("{:x}", Sha256::digest(b"test content"));
        assert_eq!(hash, expected_hash);
    }

    #[tokio::test]
    async fn test_check_duplicate_unique() {
        let pool = setup_test_db().await;
        let deduplicator = HashDeduplicator::new(pool.clone());

        let file_id = Uuid::new_v4();
        let hash = "abc123";

        let result = deduplicator.check_duplicate(hash, file_id).await.unwrap();

        assert_eq!(result, HashResult::Unique(hash.to_string()));
    }

    #[tokio::test]
    async fn test_check_duplicate_found() {
        let pool = setup_test_db().await;
        let deduplicator = HashDeduplicator::new(pool.clone());

        // Insert original file with hash
        let original_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, modification_time)
            VALUES (?, 'music/original.mp3', 'abc123', datetime('now'))
            "#,
        )
        .bind(original_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Check for duplicate with different file
        let current_id = Uuid::new_v4();
        let result = deduplicator
            .check_duplicate("abc123", current_id)
            .await
            .unwrap();

        match result {
            HashResult::Duplicate {
                hash,
                original_file_id,
            } => {
                assert_eq!(hash, "abc123");
                assert_eq!(original_file_id, original_id);
            }
            _ => panic!("Expected Duplicate, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_update_file_hash() {
        let pool = setup_test_db().await;
        let deduplicator = HashDeduplicator::new(pool.clone());

        // Insert file with temporary hash
        let file_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, modification_time)
            VALUES (?, 'music/test.mp3', 'PENDING', datetime('now'))
            "#,
        )
        .bind(file_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Update hash
        deduplicator
            .update_file_hash(file_id, "abc123")
            .await
            .unwrap();

        // Verify hash was updated
        let stored_hash: String =
            sqlx::query_scalar("SELECT hash FROM files WHERE guid = ?")
                .bind(file_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(stored_hash, "abc123");
    }

    #[tokio::test]
    async fn test_link_duplicates() {
        let pool = setup_test_db().await;
        let deduplicator = HashDeduplicator::new(pool.clone());

        // Insert two files
        let original_id = Uuid::new_v4();
        let duplicate_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, modification_time)
            VALUES (?, 'music/original.mp3', 'abc123', datetime('now'))
            "#,
        )
        .bind(original_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, modification_time)
            VALUES (?, 'music/duplicate.mp3', 'abc123', datetime('now'))
            "#,
        )
        .bind(duplicate_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Create bidirectional link
        deduplicator
            .link_duplicates(duplicate_id, original_id)
            .await
            .unwrap();

        // Verify original file has duplicate in matching_hashes
        let original_matches: String = sqlx::query_scalar(
            "SELECT matching_hashes FROM files WHERE guid = ?",
        )
        .bind(original_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        let original_array: Vec<String> = serde_json::from_str(&original_matches).unwrap();
        assert!(original_array.contains(&duplicate_id.to_string()));

        // Verify duplicate file has original in matching_hashes
        let duplicate_matches: String = sqlx::query_scalar(
            "SELECT matching_hashes FROM files WHERE guid = ?",
        )
        .bind(duplicate_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        let duplicate_array: Vec<String> = serde_json::from_str(&duplicate_matches).unwrap();
        assert!(duplicate_array.contains(&original_id.to_string()));

        // Verify duplicate file has DUPLICATE HASH status
        let status: String =
            sqlx::query_scalar("SELECT status FROM files WHERE guid = ?")
                .bind(duplicate_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(status, "DUPLICATE HASH");
    }

    #[tokio::test]
    async fn test_process_file_hash_unique() {
        let pool = setup_test_db().await;
        let deduplicator = HashDeduplicator::new(pool.clone());

        // Create temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"unique content").unwrap();
        temp_file.flush().unwrap();

        // Insert file record
        let file_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, modification_time)
            VALUES (?, 'music/test.mp3', 'PENDING', datetime('now'))
            "#,
        )
        .bind(file_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Process hash
        let result = deduplicator
            .process_file_hash(file_id, temp_file.path())
            .await
            .unwrap();

        match result {
            HashResult::Unique(hash) => {
                assert_eq!(hash.len(), 64); // SHA-256 hex string
            }
            _ => panic!("Expected Unique, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_process_file_hash_duplicate() {
        let pool = setup_test_db().await;
        let deduplicator = HashDeduplicator::new(pool.clone());

        // Create temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"duplicate content").unwrap();
        temp_file.flush().unwrap();

        // Calculate hash for content
        let expected_hash = format!("{:x}", Sha256::digest(b"duplicate content"));

        // Insert original file with same hash
        let original_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, modification_time)
            VALUES (?, 'music/original.mp3', ?, datetime('now'))
            "#,
        )
        .bind(original_id.to_string())
        .bind(&expected_hash)
        .execute(&pool)
        .await
        .unwrap();

        // Insert duplicate file record
        let duplicate_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, modification_time)
            VALUES (?, 'music/duplicate.mp3', 'PENDING', datetime('now'))
            "#,
        )
        .bind(duplicate_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Process hash (should detect duplicate and create link)
        let result = deduplicator
            .process_file_hash(duplicate_id, temp_file.path())
            .await
            .unwrap();

        match result {
            HashResult::Duplicate {
                hash,
                original_file_id,
            } => {
                assert_eq!(hash, expected_hash);
                assert_eq!(original_file_id, original_id);
            }
            _ => panic!("Expected Duplicate, got {:?}", result),
        }

        // Verify bidirectional link was created
        let duplicate_status: String =
            sqlx::query_scalar("SELECT status FROM files WHERE guid = ?")
                .bind(duplicate_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(duplicate_status, "DUPLICATE HASH");
    }
}

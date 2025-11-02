//! File database operations
//!
//! **[AIA-DB-010]** Audio file persistence and deduplication

use anyhow::Result;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// Audio file record
#[derive(Debug, Clone)]
pub struct AudioFile {
    pub guid: Uuid,
    pub path: String,
    pub hash: String,
    pub duration: Option<f64>,
    pub modification_time: DateTime<Utc>,
}

impl AudioFile {
    /// Create new audio file record
    pub fn new(path: String, hash: String, modification_time: DateTime<Utc>) -> Self {
        Self {
            guid: Uuid::new_v4(),
            path,
            hash,
            duration: None,
            modification_time,
        }
    }
}

/// Calculate SHA-256 hash of file contents
///
/// **[AIA-INT-010]** File deduplication via SHA-256
pub fn calculate_file_hash(file_path: &Path) -> Result<String> {
    let contents = fs::read(file_path)?;
    let hash = Sha256::digest(&contents);
    Ok(format!("{:x}", hash))
}

/// Save audio file to database
pub async fn save_file(pool: &SqlitePool, file: &AudioFile) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO files (guid, path, hash, duration, modification_time, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        ON CONFLICT(path) DO UPDATE SET
            hash = excluded.hash,
            duration = excluded.duration,
            modification_time = excluded.modification_time,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(file.guid.to_string())
    .bind(&file.path)
    .bind(&file.hash)
    .bind(file.duration)
    .bind(file.modification_time.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(())
}

/// Load audio file by path
pub async fn load_file_by_path(pool: &SqlitePool, path: &str) -> Result<Option<AudioFile>> {
    let row = sqlx::query(
        r#"
        SELECT guid, path, hash, duration, modification_time
        FROM files
        WHERE path = ?
        "#,
    )
    .bind(path)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let guid_str: String = row.get("guid");
            let guid = Uuid::parse_str(&guid_str)?;

            let mod_time_str: String = row.get("modification_time");
            let modification_time = DateTime::parse_from_rfc3339(&mod_time_str)?
                .with_timezone(&Utc);

            Ok(Some(AudioFile {
                guid,
                path: row.get("path"),
                hash: row.get("hash"),
                duration: row.get("duration"),
                modification_time,
            }))
        }
        None => Ok(None),
    }
}

/// Load audio file by hash (for deduplication)
pub async fn load_file_by_hash(pool: &SqlitePool, hash: &str) -> Result<Option<AudioFile>> {
    let row = sqlx::query(
        r#"
        SELECT guid, path, hash, duration, modification_time
        FROM files
        WHERE hash = ?
        LIMIT 1
        "#,
    )
    .bind(hash)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let guid_str: String = row.get("guid");
            let guid = Uuid::parse_str(&guid_str)?;

            let mod_time_str: String = row.get("modification_time");
            let modification_time = DateTime::parse_from_rfc3339(&mod_time_str)?
                .with_timezone(&Utc);

            Ok(Some(AudioFile {
                guid,
                path: row.get("path"),
                hash: row.get("hash"),
                duration: row.get("duration"),
                modification_time,
            }))
        }
        None => Ok(None),
    }
}

/// Count total files in database
pub async fn count_files(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

/// Load all files from database
pub async fn load_all_files(pool: &SqlitePool) -> Result<Vec<AudioFile>> {
    let rows = sqlx::query(
        r#"
        SELECT guid, path, hash, duration, modification_time
        FROM files
        ORDER BY path
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut files = Vec::new();
    for row in rows {
        let guid_str: String = row.get("guid");
        let guid = Uuid::parse_str(&guid_str)?;

        let mod_time_str: String = row.get("modification_time");
        let modification_time = DateTime::parse_from_rfc3339(&mod_time_str)?
            .with_timezone(&Utc);

        files.push(AudioFile {
            guid,
            path: row.get("path"),
            hash: row.get("hash"),
            duration: row.get("duration"),
            modification_time,
        });
    }

    Ok(files)
}

/// Update file duration
pub async fn update_file_duration(pool: &SqlitePool, file_id: Uuid, duration: f64) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE files
        SET duration = ?, updated_at = CURRENT_TIMESTAMP
        WHERE guid = ?
        "#,
    )
    .bind(duration)
    .bind(file_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load_file() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        // Initialize schema for test database
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
        wkmp_common::db::init::create_files_table(&pool).await.unwrap();

        let file = AudioFile::new(
            "test/music/track01.mp3".to_string(),
            "abc123def456".to_string(),
            Utc::now(),
        );

        save_file(&pool, &file).await.expect("Failed to save file");

        let loaded = load_file_by_path(&pool, "test/music/track01.mp3")
            .await
            .expect("Failed to load file")
            .expect("File not found");

        assert_eq!(loaded.path, file.path);
        assert_eq!(loaded.hash, file.hash);
    }

    #[tokio::test]
    async fn test_deduplication_by_hash() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        // Initialize schema for test database
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
        wkmp_common::db::init::create_files_table(&pool).await.unwrap();

        let file = AudioFile::new(
            "test/music/track01.mp3".to_string(),
            "samehash123".to_string(),
            Utc::now(),
        );

        save_file(&pool, &file).await.expect("Failed to save file");

        // Look up by hash
        let duplicate = load_file_by_hash(&pool, "samehash123")
            .await
            .expect("Failed to load file")
            .expect("File not found");

        assert_eq!(duplicate.hash, "samehash123");
    }
}

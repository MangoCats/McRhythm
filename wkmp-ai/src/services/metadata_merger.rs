//! Metadata Merging for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-010] Metadata Extraction (Phase 3)
//!
//! Extracts metadata from audio files and merges with existing database metadata.
//! Merge strategy: New values overwrite old, old values preserved if new is NULL.

use sqlx::{Pool, Sqlite};
use std::path::Path;
use uuid::Uuid;
use wkmp_common::{Error, Result};

use super::metadata_extractor::{AudioMetadata, MetadataError, MetadataExtractor};

/// Merged metadata result
#[derive(Debug, Clone)]
pub struct MergedMetadata {
    /// Artist (merged)
    pub artist: Option<String>,
    /// Title (merged)
    pub title: Option<String>,
    /// Album (merged)
    pub album: Option<String>,
    /// Track number (merged)
    pub track_number: Option<u32>,
    /// Year (merged)
    pub year: Option<u32>,
    /// Duration in ticks (from file properties)
    pub duration_ticks: i64,
    /// Audio format
    pub format: String,
    /// Sample rate (Hz)
    pub sample_rate: Option<u32>,
    /// Channels
    pub channels: Option<u8>,
    /// File size (bytes)
    pub file_size_bytes: u64,
}

/// Metadata Merger
///
/// **Traceability:** [REQ-SPEC032-010] (Phase 3: Metadata Extraction & Merging)
pub struct MetadataMerger {
    db: Pool<Sqlite>,
    extractor: MetadataExtractor,
}

impl MetadataMerger {
    /// Create new metadata merger
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self {
            db,
            extractor: MetadataExtractor::new(),
        }
    }

    /// Extract and merge metadata
    ///
    /// **Algorithm:**
    /// 1. Extract metadata from audio file using lofty
    /// 2. If file_id exists: Read existing metadata from database
    /// 3. Merge: new_value if Some, else old_value
    /// 4. Update file record with merged metadata
    /// 5. Return merged metadata
    ///
    /// **Merge Strategy (per SPEC032):**
    /// - New values overwrite old values
    /// - Old values preserved if new is NULL
    ///
    /// **Traceability:** [REQ-SPEC032-010]
    pub async fn extract_and_merge(
        &self,
        file_id: Uuid,
        file_path: &Path,
    ) -> Result<MergedMetadata> {
        tracing::debug!(
            file_id = %file_id,
            path = %file_path.display(),
            "Extracting and merging metadata"
        );

        // Extract metadata from file
        let new_metadata = self
            .extractor
            .extract(file_path)
            .map_err(|e| Error::Internal(format!("Metadata extraction failed: {}", e)))?;

        // Read existing metadata from database (if any)
        let existing: Option<(
            Option<String>, // artist
            Option<String>, // title
            Option<String>, // album
            Option<i64>,    // track_number
            Option<i64>,    // year
        )> = sqlx::query_as(
            "SELECT artist, title, album, track_number, year FROM files WHERE guid = ?",
        )
        .bind(file_id.to_string())
        .fetch_optional(&self.db)
        .await?;

        // Merge metadata: new overwrites old, old preserved if new is NULL
        let (merged_artist, merged_title, merged_album, merged_track, merged_year) =
            if let Some((old_artist, old_title, old_album, old_track, old_year)) = existing {
                (
                    new_metadata.artist.or(old_artist),
                    new_metadata.title.or(old_title),
                    new_metadata.album.or(old_album),
                    new_metadata
                        .track_number
                        .map(|t| t as i64)
                        .or(old_track),
                    new_metadata.year.map(|y| y as i64).or(old_year),
                )
            } else {
                (
                    new_metadata.artist,
                    new_metadata.title,
                    new_metadata.album,
                    new_metadata.track_number.map(|t| t as i64),
                    new_metadata.year.map(|y| y as i64),
                )
            };

        // Convert duration to ticks (SPEC017: 28,224,000 ticks per second)
        let duration_ticks = new_metadata
            .duration_seconds
            .map(|s| (s * 28_224_000.0) as i64)
            .unwrap_or(0);

        // Update file record with merged metadata
        sqlx::query(
            r#"
            UPDATE files
            SET
                artist = ?,
                title = ?,
                album = ?,
                track_number = ?,
                year = ?,
                duration_ticks = ?,
                format = ?,
                sample_rate = ?,
                channels = ?,
                file_size_bytes = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE guid = ?
            "#,
        )
        .bind(&merged_artist)
        .bind(&merged_title)
        .bind(&merged_album)
        .bind(merged_track)
        .bind(merged_year)
        .bind(duration_ticks)
        .bind(&new_metadata.format)
        .bind(new_metadata.sample_rate.map(|r| r as i64))
        .bind(new_metadata.channels.map(|c| c as i64))
        .bind(new_metadata.file_size_bytes as i64)
        .bind(file_id.to_string())
        .execute(&self.db)
        .await?;

        tracing::debug!(
            file_id = %file_id,
            artist = ?merged_artist,
            title = ?merged_title,
            duration_ticks = duration_ticks,
            "Merged metadata updated"
        );

        Ok(MergedMetadata {
            artist: merged_artist,
            title: merged_title,
            album: merged_album,
            track_number: merged_track.map(|t| t as u32),
            year: merged_year.map(|y| y as u32),
            duration_ticks,
            format: new_metadata.format,
            sample_rate: new_metadata.sample_rate,
            channels: new_metadata.channels,
            file_size_bytes: new_metadata.file_size_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Setup in-memory test database with files table
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                artist TEXT,
                title TEXT,
                album TEXT,
                track_number INTEGER,
                year INTEGER,
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

    // Note: Full integration tests with real audio files require test fixtures
    // Unit tests focus on database merge logic

    #[tokio::test]
    async fn test_metadata_merger_creation() {
        let pool = setup_test_db().await;
        let _merger = MetadataMerger::new(pool);
        // Just verify it can be created without panic
    }
}


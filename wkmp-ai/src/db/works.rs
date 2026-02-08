//! Work database operations
//!
//! **[AIA-DB-010]** Work persistence (MusicBrainz musical works)

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Work record (MusicBrainz musical work / composition)
///
/// PLAN026: Extended with am28 integration fields
#[derive(Debug, Clone)]
pub struct Work {
    /// Unique identifier (UUID)
    pub guid: Uuid,
    /// MusicBrainz Work MBID
    pub work_mbid: String,
    /// Work title (composition name)
    pub title: String,
    // PLAN026: am28 integration fields
    /// Composer/author name(s)
    pub composer: Option<String>,
    /// International Standard Musical Work Code
    pub iswc: Option<String>,
    /// Work type: 'Song', 'Symphony', 'Opera', etc.
    pub work_type: Option<String>,
    /// Base selection probability (0.0-1.0+, default 1.0)
    pub base_probability: f64,
    /// Minimum cooldown period in seconds
    pub min_cooldown: i64,
    /// Ramping cooldown period in seconds
    pub ramping_cooldown: i64,
    /// ISO 8601 timestamp when last played
    pub last_played_at: Option<String>,
}

impl Work {
    /// Create new work from MusicBrainz work MBID
    pub fn new(work_mbid: String, title: String) -> Self {
        Self {
            guid: Uuid::new_v4(),
            work_mbid,
            title,
            composer: None,
            iswc: None,
            work_type: None,
            base_probability: 1.0,
            min_cooldown: 259200,     // 3 days in seconds
            ramping_cooldown: 604800, // 7 days in seconds
            last_played_at: None,
        }
    }

    /// Create new work with all am28 fields
    pub fn new_with_details(
        work_mbid: String,
        title: String,
        composer: Option<String>,
        iswc: Option<String>,
        work_type: Option<String>,
    ) -> Self {
        Self {
            guid: Uuid::new_v4(),
            work_mbid,
            title,
            composer,
            iswc,
            work_type,
            base_probability: 1.0,
            min_cooldown: 259200,
            ramping_cooldown: 604800,
            last_played_at: None,
        }
    }
}

/// Save work to database
pub async fn save_work(pool: &SqlitePool, work: &Work) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO works (
            guid, work_mbid, title, composer, iswc, work_type,
            base_probability, min_cooldown, ramping_cooldown,
            last_played_at, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        ON CONFLICT(work_mbid) DO UPDATE SET
            title = excluded.title,
            composer = excluded.composer,
            iswc = excluded.iswc,
            work_type = excluded.work_type,
            base_probability = excluded.base_probability,
            min_cooldown = excluded.min_cooldown,
            ramping_cooldown = excluded.ramping_cooldown,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(work.guid.to_string())
    .bind(&work.work_mbid)
    .bind(&work.title)
    .bind(&work.composer)
    .bind(&work.iswc)
    .bind(&work.work_type)
    .bind(work.base_probability)
    .bind(work.min_cooldown)
    .bind(work.ramping_cooldown)
    .bind(&work.last_played_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load work by MBID
pub async fn load_work_by_mbid(pool: &SqlitePool, work_mbid: &str) -> Result<Option<Work>> {
    let row = sqlx::query(
        r#"
        SELECT guid, work_mbid, title, composer, iswc, work_type,
               base_probability, min_cooldown, ramping_cooldown, last_played_at
        FROM works
        WHERE work_mbid = ?
        "#,
    )
    .bind(work_mbid)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let guid_str: String = row.get("guid");

            Ok(Some(Work {
                guid: Uuid::parse_str(&guid_str)?,
                work_mbid: row.get("work_mbid"),
                title: row.get("title"),
                composer: row.get("composer"),
                iswc: row.get("iswc"),
                work_type: row.get("work_type"),
                base_probability: row.get("base_probability"),
                min_cooldown: row.get("min_cooldown"),
                ramping_cooldown: row.get("ramping_cooldown"),
                last_played_at: row.get("last_played_at"),
            }))
        }
        None => Ok(None),
    }
}

/// Link song to work
pub async fn link_song_to_work(pool: &SqlitePool, song_id: Uuid, work_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE songs
        SET work_id = ?, updated_at = CURRENT_TIMESTAMP
        WHERE guid = ?
        "#,
    )
    .bind(work_id.to_string())
    .bind(song_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load_work() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        // Initialize schema for test database (including self-healing schema sync)
        wkmp_common::db::init::init_database_schema(&pool)
            .await
            .unwrap();

        let work = Work::new("work-mbid-999".to_string(), "Test Composition".to_string());

        save_work(&pool, &work).await.expect("Failed to save work");

        let loaded = load_work_by_mbid(&pool, "work-mbid-999")
            .await
            .expect("Failed to load work")
            .expect("Work not found");

        assert_eq!(loaded.work_mbid, "work-mbid-999");
        assert_eq!(loaded.title, "Test Composition");
    }
}

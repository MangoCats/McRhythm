//! Album database operations
//!
//! **[AIA-DB-010]** Album persistence (MusicBrainz releases)

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Album record (MusicBrainz release)
#[derive(Debug, Clone)]
pub struct Album {
    pub guid: Uuid,
    pub album_mbid: String,
    pub title: String,
    pub release_date: Option<String>,
}

impl Album {
    /// Create new album from MusicBrainz release MBID
    pub fn new(album_mbid: String, title: String) -> Self {
        Self {
            guid: Uuid::new_v4(),
            album_mbid,
            title,
            release_date: None,
        }
    }
}

/// Save album to database
pub async fn save_album(pool: &SqlitePool, album: &Album) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO albums (guid, album_mbid, title, release_date, created_at, updated_at)
        VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        ON CONFLICT(album_mbid) DO UPDATE SET
            title = excluded.title,
            release_date = excluded.release_date,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(album.guid.to_string())
    .bind(&album.album_mbid)
    .bind(&album.title)
    .bind(&album.release_date)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load album by MBID
pub async fn load_album_by_mbid(pool: &SqlitePool, album_mbid: &str) -> Result<Option<Album>> {
    let row = sqlx::query(
        r#"
        SELECT guid, album_mbid, title, release_date
        FROM albums
        WHERE album_mbid = ?
        "#,
    )
    .bind(album_mbid)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let guid_str: String = row.get("guid");

            Ok(Some(Album {
                guid: Uuid::parse_str(&guid_str)?,
                album_mbid: row.get("album_mbid"),
                title: row.get("title"),
                release_date: row.get("release_date"),
            }))
        }
        None => Ok(None),
    }
}

/// Link passage to album
pub async fn link_passage_to_album(
    pool: &SqlitePool,
    passage_id: Uuid,
    album_id: Uuid,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO passage_albums (passage_id, album_id, created_at)
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(passage_id, album_id) DO NOTHING
        "#,
    )
    .bind(passage_id.to_string())
    .bind(album_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load_album() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        // Initialize schema for test database
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
        wkmp_common::db::init::create_albums_table(&pool).await.unwrap();

        let album = Album::new("release-mbid-789".to_string(), "Test Album".to_string());

        save_album(&pool, &album).await.expect("Failed to save album");

        let loaded = load_album_by_mbid(&pool, "release-mbid-789")
            .await
            .expect("Failed to load album")
            .expect("Album not found");

        assert_eq!(loaded.album_mbid, "release-mbid-789");
        assert_eq!(loaded.title, "Test Album");
    }
}

//! Artist database operations
//!
//! **[AIA-DB-010]** Artist persistence (MusicBrainz artists)

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Artist record (MusicBrainz artist)
#[derive(Debug, Clone)]
pub struct Artist {
    pub guid: Uuid,
    pub artist_mbid: String,
    pub name: String,
    pub base_probability: f64,
    pub min_cooldown: i64,
    pub ramping_cooldown: i64,
    pub last_played_at: Option<String>,
}

impl Artist {
    /// Create new artist from MusicBrainz artist MBID
    pub fn new(artist_mbid: String, name: String) -> Self {
        Self {
            guid: Uuid::new_v4(),
            artist_mbid,
            name,
            base_probability: 1.0,
            min_cooldown: 7200,   // 2 hours in seconds
            ramping_cooldown: 14400, // 4 hours in seconds
            last_played_at: None,
        }
    }
}

/// Save artist to database
pub async fn save_artist(pool: &SqlitePool, artist: &Artist) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO artists (
            guid, artist_mbid, name, base_probability, min_cooldown, ramping_cooldown,
            last_played_at, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        ON CONFLICT(artist_mbid) DO UPDATE SET
            name = excluded.name,
            base_probability = excluded.base_probability,
            min_cooldown = excluded.min_cooldown,
            ramping_cooldown = excluded.ramping_cooldown,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(artist.guid.to_string())
    .bind(&artist.artist_mbid)
    .bind(&artist.name)
    .bind(artist.base_probability)
    .bind(artist.min_cooldown)
    .bind(artist.ramping_cooldown)
    .bind(&artist.last_played_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load artist by MBID
pub async fn load_artist_by_mbid(pool: &SqlitePool, artist_mbid: &str) -> Result<Option<Artist>> {
    let row = sqlx::query(
        r#"
        SELECT guid, artist_mbid, name, base_probability, min_cooldown,
               ramping_cooldown, last_played_at
        FROM artists
        WHERE artist_mbid = ?
        "#,
    )
    .bind(artist_mbid)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let guid_str: String = row.get("guid");

            Ok(Some(Artist {
                guid: Uuid::parse_str(&guid_str)?,
                artist_mbid: row.get("artist_mbid"),
                name: row.get("name"),
                base_probability: row.get("base_probability"),
                min_cooldown: row.get("min_cooldown"),
                ramping_cooldown: row.get("ramping_cooldown"),
                last_played_at: row.get("last_played_at"),
            }))
        }
        None => Ok(None),
    }
}

/// Link song to artist with weight
pub async fn link_song_to_artist(
    pool: &SqlitePool,
    song_id: Uuid,
    artist_id: Uuid,
    weight: f64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO song_artists (song_id, artist_id, weight, created_at)
        VALUES (?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(song_id, artist_id) DO UPDATE SET
            weight = excluded.weight
        "#,
    )
    .bind(song_id.to_string())
    .bind(artist_id.to_string())
    .bind(weight)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load_artist() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        crate::db::schema::initialize_schema(&pool)
            .await
            .expect("Schema initialization failed");

        let artist = Artist::new("artist-mbid-456".to_string(), "Test Artist".to_string());

        save_artist(&pool, &artist).await.expect("Failed to save artist");

        let loaded = load_artist_by_mbid(&pool, "artist-mbid-456")
            .await
            .expect("Failed to load artist")
            .expect("Artist not found");

        assert_eq!(loaded.artist_mbid, "artist-mbid-456");
        assert_eq!(loaded.name, "Test Artist");
    }
}

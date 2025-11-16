//! Artist database operations
//!
//! **[AIA-DB-010]** Artist persistence (MusicBrainz artists)

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Artist record (MusicBrainz artist)
#[derive(Debug, Clone)]
pub struct Artist {
    /// Unique identifier (UUID)
    pub guid: Uuid,
    /// MusicBrainz Artist MBID
    pub artist_mbid: String,
    /// Artist name
    pub name: String,
    /// Base selection probability (0.0-1.0+, default 1.0)
    pub base_probability: f64,
    /// Minimum cooldown period in seconds
    pub min_cooldown: i64,
    /// Ramping cooldown period in seconds
    pub ramping_cooldown: i64,
    /// ISO 8601 timestamp when last played
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

/// **[PLAN026]** Batch query existing artists by MBIDs (outside transaction)
///
/// Pre-fetches all existing artists for given artist MBIDs to minimize
/// transaction duration. Returns HashMap for O(1) lookup during batch insert.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `artist_mbids` - MBIDs to query
///
/// # Returns
/// HashMap mapping artist_mbid → Artist (only for artists that exist)
pub async fn batch_query_existing_artists(
    pool: &SqlitePool,
    artist_mbids: &[String],
) -> Result<std::collections::HashMap<String, Artist>> {
    use futures::TryStreamExt;

    if artist_mbids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    // Build IN clause with placeholders
    let placeholders = (0..artist_mbids.len())
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let query_str = format!(
        r#"
        SELECT guid, artist_mbid, name, base_probability, min_cooldown,
               ramping_cooldown, last_played_at
        FROM artists
        WHERE artist_mbid IN ({})
        "#,
        placeholders
    );

    let mut query = sqlx::query(&query_str);
    for mbid in artist_mbids {
        query = query.bind(mbid);
    }

    let mut artists = std::collections::HashMap::new();
    let mut rows = query.fetch(pool);

    while let Some(row) = rows.try_next().await? {
        let guid_str: String = row.get("guid");
        let artist_mbid: String = row.get("artist_mbid");

        let artist = Artist {
            guid: Uuid::parse_str(&guid_str)?,
            artist_mbid: artist_mbid.clone(),
            name: row.get("name"),
            base_probability: row.get("base_probability"),
            min_cooldown: row.get("min_cooldown"),
            ramping_cooldown: row.get("ramping_cooldown"),
            last_played_at: row.get("last_played_at"),
        };

        artists.insert(artist_mbid, artist);
    }

    Ok(artists)
}

/// **[PLAN026]** Batch insert/update artists within a transaction
///
/// Inserts or updates multiple artists in a single database transaction.
/// Uses ON CONFLICT to upsert existing artists.
///
/// # Arguments
/// * `tx` - Database transaction (caller manages transaction lifecycle)
/// * `artists` - Artists to insert/update
///
/// # Returns
/// Number of artists processed
pub async fn batch_save_artists(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    artists: &[Artist],
) -> Result<usize> {
    for artist in artists {
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
        .execute(&mut **tx)
        .await?;
    }

    Ok(artists.len())
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

        // Initialize schema for test database
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
        wkmp_common::db::init::create_artists_table(&pool).await.unwrap();

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

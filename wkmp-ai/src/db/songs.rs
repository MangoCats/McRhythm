//! Song database operations
//!
//! **[AIA-DB-010]** Song persistence (MusicBrainz recordings)

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Song record (MusicBrainz recording)
#[derive(Debug, Clone)]
pub struct Song {
    pub guid: Uuid,
    pub recording_mbid: String,
    pub title: Option<String>,
    pub work_id: Option<Uuid>,
    pub related_songs: Option<String>,
    pub lyrics: Option<String>,
    pub base_probability: f64,
    pub min_cooldown: i64,
    pub ramping_cooldown: i64,
    pub last_played_at: Option<String>,
}

impl Song {
    /// Create new song from MusicBrainz recording MBID and optional title
    pub fn new(recording_mbid: String, title: Option<String>) -> Self {
        Self {
            guid: Uuid::new_v4(),
            recording_mbid,
            title,
            work_id: None,
            related_songs: None,
            lyrics: None,
            base_probability: 1.0,
            min_cooldown: 604800,      // 7 days in seconds
            ramping_cooldown: 1209600,  // 14 days in seconds
            last_played_at: None,
        }
    }
}

/// Save song to database
pub async fn save_song(pool: &SqlitePool, song: &Song) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO songs (
            guid, recording_mbid, title, work_id, related_songs, lyrics,
            base_probability, min_cooldown, ramping_cooldown, last_played_at,
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        ON CONFLICT(recording_mbid) DO UPDATE SET
            title = excluded.title,
            work_id = excluded.work_id,
            related_songs = excluded.related_songs,
            lyrics = excluded.lyrics,
            base_probability = excluded.base_probability,
            min_cooldown = excluded.min_cooldown,
            ramping_cooldown = excluded.ramping_cooldown,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(song.guid.to_string())
    .bind(&song.recording_mbid)
    .bind(&song.title)
    .bind(song.work_id.map(|id| id.to_string()))
    .bind(&song.related_songs)
    .bind(&song.lyrics)
    .bind(song.base_probability)
    .bind(song.min_cooldown)
    .bind(song.ramping_cooldown)
    .bind(&song.last_played_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load song by recording MBID
pub async fn load_song_by_mbid(pool: &SqlitePool, recording_mbid: &str) -> Result<Option<Song>> {
    let row = sqlx::query(
        r#"
        SELECT guid, recording_mbid, title, work_id, related_songs, lyrics,
               base_probability, min_cooldown, ramping_cooldown, last_played_at
        FROM songs
        WHERE recording_mbid = ?
        "#,
    )
    .bind(recording_mbid)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let guid_str: String = row.get("guid");
            let work_id_str: Option<String> = row.get("work_id");

            Ok(Some(Song {
                guid: Uuid::parse_str(&guid_str)?,
                recording_mbid: row.get("recording_mbid"),
                title: row.get("title"),
                work_id: work_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                related_songs: row.get("related_songs"),
                lyrics: row.get("lyrics"),
                base_probability: row.get("base_probability"),
                min_cooldown: row.get("min_cooldown"),
                ramping_cooldown: row.get("ramping_cooldown"),
                last_played_at: row.get("last_played_at"),
            }))
        }
        None => Ok(None),
    }
}

/// Link passage to song
pub async fn link_passage_to_song(
    pool: &SqlitePool,
    passage_id: Uuid,
    song_id: Uuid,
    start_time_ticks: i64,
    end_time_ticks: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO passage_songs (passage_id, song_id, start_time_ticks, end_time_ticks, created_at)
        VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(passage_id, song_id) DO UPDATE SET
            start_time_ticks = excluded.start_time_ticks,
            end_time_ticks = excluded.end_time_ticks
        "#,
    )
    .bind(passage_id.to_string())
    .bind(song_id.to_string())
    .bind(start_time_ticks)
    .bind(end_time_ticks)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load_song() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        // Initialize schema for test database
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS songs (
                guid TEXT PRIMARY KEY,
                recording_mbid TEXT UNIQUE NOT NULL,
                title TEXT,
                work_id TEXT,
                related_songs TEXT,
                lyrics TEXT,
                base_probability REAL NOT NULL DEFAULT 1.0,
                min_cooldown INTEGER NOT NULL DEFAULT 604800,
                ramping_cooldown INTEGER NOT NULL DEFAULT 1209600,
                last_played_at TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(&pool).await.unwrap();

        let song = Song::new("recording-mbid-123".to_string(), Some("Test Song".to_string()));

        save_song(&pool, &song).await.expect("Failed to save song");

        let loaded = load_song_by_mbid(&pool, "recording-mbid-123")
            .await
            .expect("Failed to load song")
            .expect("Song not found");

        assert_eq!(loaded.recording_mbid, "recording-mbid-123");
        assert_eq!(loaded.base_probability, 1.0);
    }
}

//! Load song timeline from passage_songs table
//!
//! Provides database queries to build song timelines for passages.
//!
//! **Traceability:**
//! - [DB-PS-010] passage_songs table schema
//! - [ARCH-SNGC-041] Song Timeline Data Structure
//! - [REV002] Event-driven architecture update

use crate::error::Result;
use crate::playback::song_timeline::{SongTimeline, SongTimelineEntry};
use sqlx::{Pool, Row, Sqlite};
use tracing::{debug, warn};
use uuid::Uuid;

/// Load song timeline for a passage
///
/// Queries `passage_songs` table and builds a sorted timeline of songs
/// within the specified passage.
///
/// **Algorithm:**
/// 1. Query all passage_songs rows for the given passage_id
/// 2. Parse song_id (handle NULL for gaps)
/// 3. Create timeline entries sorted by start_time_ms
/// 4. Return SongTimeline (empty if no songs)
///
/// **Traceability:** [DB-PS-010] passage_songs table schema
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `passage_id` - UUID of passage to load timeline for
///
/// # Returns
/// * `Ok(SongTimeline)` - Timeline (may be empty if passage has no songs)
/// * `Err` - Database error
///
/// # Examples
/// ```no_run
/// use wkmp_ap::db::passage_songs::load_song_timeline;
/// use uuid::Uuid;
/// # use sqlx::{Pool, Sqlite};
///
/// # async fn example(pool: &Pool<Sqlite>) {
/// let passage_id = Uuid::new_v4();
/// let timeline = load_song_timeline(pool, passage_id).await.unwrap();
///
/// // Timeline may be empty if passage has no songs
/// if timeline.is_empty() {
///     println!("Passage is one continuous gap (no song boundaries)");
/// } else {
///     println!("Passage has {} song entries", timeline.len());
/// }
/// # }
/// ```
pub async fn load_song_timeline(
    pool: &Pool<Sqlite>,
    passage_id: Uuid,
) -> Result<SongTimeline> {
    debug!("Loading song timeline for passage {}", passage_id);

    // Query passage_songs table
    // Note: passage_songs table may not exist in all databases (graceful fallback)
    // Use raw query instead of query! macro to avoid compile-time database checks
    let query_result = sqlx::query(
        r#"
        SELECT
            song_guid,
            start_time_ms,
            end_time_ms
        FROM passage_songs
        WHERE passage_guid = ?
        ORDER BY start_time_ms ASC
        "#,
    )
    .bind(passage_id.to_string())
    .fetch_all(pool)
    .await;

    let rows = match query_result {
        Ok(rows) => rows,
        Err(e) => {
            // Check if error is due to missing table
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("no such table") || err_str.contains("passage_songs") {
                warn!(
                    "passage_songs table not found - returning empty timeline for passage {}",
                    passage_id
                );
                return Ok(SongTimeline::new(vec![]));
            } else {
                // Other database error - propagate
                return Err(e.into());
            }
        }
    };

    // Convert rows to timeline entries
    let entries: Vec<SongTimelineEntry> = rows
        .into_iter()
        .filter_map(|row| {
            // Parse song_guid (may be NULL for gaps)
            let song_guid: Option<String> = row.try_get("song_guid").ok().flatten();

            // If song_guid is present but invalid, filter out the entry
            let song_id = match song_guid {
                Some(s) => match Uuid::parse_str(&s) {
                    Ok(uuid) => Some(uuid),
                    Err(_) => {
                        warn!("Invalid UUID in passage_songs.song_guid: {} - filtering out entry", s);
                        return None; // Filter out entries with invalid UUIDs
                    }
                },
                None => None, // NULL song_guid is valid (represents a gap)
            };

            // Get time values
            let start_time_ms: i64 = match row.try_get("start_time_ms") {
                Ok(v) => v,
                Err(e) => {
                    warn!("Failed to parse start_time_ms: {}", e);
                    return None;
                }
            };

            let end_time_ms: i64 = match row.try_get("end_time_ms") {
                Ok(v) => v,
                Err(e) => {
                    warn!("Failed to parse end_time_ms: {}", e);
                    return None;
                }
            };

            // Validate time range
            if start_time_ms < 0 || end_time_ms < 0 {
                warn!(
                    "Invalid time range in passage_songs: start={}, end={}",
                    start_time_ms, end_time_ms
                );
                return None;
            }

            if end_time_ms <= start_time_ms {
                warn!(
                    "Invalid time range in passage_songs: end ({}) <= start ({})",
                    end_time_ms, start_time_ms
                );
                return None;
            }

            Some(SongTimelineEntry {
                song_id,
                start_time_ms: start_time_ms as u64,
                end_time_ms: end_time_ms as u64,
            })
        })
        .collect();

    debug!(
        "Loaded {} song entries for passage {}",
        entries.len(),
        passage_id
    );

    Ok(SongTimeline::new(entries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn create_test_db() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Create passage_songs table
        sqlx::query(
            r#"
            CREATE TABLE passage_songs (
                guid TEXT PRIMARY KEY NOT NULL,
                passage_guid TEXT NOT NULL,
                song_guid TEXT,
                start_time_ms INTEGER NOT NULL,
                end_time_ms INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_empty_passage_songs() {
        let pool = create_test_db().await;
        let passage_id = Uuid::new_v4();

        let timeline = load_song_timeline(&pool, passage_id).await.unwrap();

        assert_eq!(timeline.len(), 0);
        assert!(timeline.is_empty());
    }

    #[tokio::test]
    async fn test_single_song() {
        let pool = create_test_db().await;
        let passage_id = Uuid::new_v4();
        let song_id = Uuid::new_v4();

        // Insert one song
        sqlx::query(
            r#"
            INSERT INTO passage_songs (guid, passage_guid, song_guid, start_time_ms, end_time_ms)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(passage_id.to_string())
        .bind(song_id.to_string())
        .bind(0)
        .bind(10000)
        .execute(&pool)
        .await
        .unwrap();

        let timeline = load_song_timeline(&pool, passage_id).await.unwrap();

        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline.get_current_song(5000), Some(song_id));
    }

    #[tokio::test]
    async fn test_multiple_songs() {
        let pool = create_test_db().await;
        let passage_id = Uuid::new_v4();
        let song1 = Uuid::new_v4();
        let song2 = Uuid::new_v4();
        let song3 = Uuid::new_v4();

        // Insert songs (in random order to test sorting)
        sqlx::query(
            r#"
            INSERT INTO passage_songs (guid, passage_guid, song_guid, start_time_ms, end_time_ms)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(passage_id.to_string())
        .bind(song2.to_string())
        .bind(10000)
        .bind(20000)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            INSERT INTO passage_songs (guid, passage_guid, song_guid, start_time_ms, end_time_ms)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(passage_id.to_string())
        .bind(song1.to_string())
        .bind(0)
        .bind(10000)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            INSERT INTO passage_songs (guid, passage_guid, song_guid, start_time_ms, end_time_ms)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(passage_id.to_string())
        .bind(song3.to_string())
        .bind(20000)
        .bind(30000)
        .execute(&pool)
        .await
        .unwrap();

        let timeline = load_song_timeline(&pool, passage_id).await.unwrap();

        assert_eq!(timeline.len(), 3);

        // Verify songs are in correct order despite insertion order
        assert_eq!(timeline.get_current_song(5000), Some(song1));
        assert_eq!(timeline.get_current_song(15000), Some(song2));
        assert_eq!(timeline.get_current_song(25000), Some(song3));
    }

    #[tokio::test]
    async fn test_gap_with_null_song_guid() {
        let pool = create_test_db().await;
        let passage_id = Uuid::new_v4();

        // Insert gap (NULL song_guid)
        sqlx::query(
            r#"
            INSERT INTO passage_songs (guid, passage_guid, song_guid, start_time_ms, end_time_ms)
            VALUES (?, ?, NULL, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(passage_id.to_string())
        .bind(0)
        .bind(5000)
        .execute(&pool)
        .await
        .unwrap();

        let timeline = load_song_timeline(&pool, passage_id).await.unwrap();

        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline.get_current_song(2000), None); // Gap has no song
    }

    #[tokio::test]
    async fn test_invalid_uuid_filtered_out() {
        let pool = create_test_db().await;
        let passage_id = Uuid::new_v4();

        // Insert entry with invalid UUID
        sqlx::query(
            r#"
            INSERT INTO passage_songs (guid, passage_guid, song_guid, start_time_ms, end_time_ms)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(passage_id.to_string())
        .bind("not-a-valid-uuid")
        .bind(0)
        .bind(5000)
        .execute(&pool)
        .await
        .unwrap();

        let timeline = load_song_timeline(&pool, passage_id).await.unwrap();

        // Invalid entry should be filtered out
        assert_eq!(timeline.len(), 0);
    }

    #[tokio::test]
    async fn test_invalid_time_range_filtered_out() {
        let pool = create_test_db().await;
        let passage_id = Uuid::new_v4();
        let song_id = Uuid::new_v4();

        // Insert entry where end_time <= start_time (invalid)
        sqlx::query(
            r#"
            INSERT INTO passage_songs (guid, passage_guid, song_guid, start_time_ms, end_time_ms)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(passage_id.to_string())
        .bind(song_id.to_string())
        .bind(10000)
        .bind(5000) // end < start (invalid!)
        .execute(&pool)
        .await
        .unwrap();

        let timeline = load_song_timeline(&pool, passage_id).await.unwrap();

        // Invalid entry should be filtered out
        assert_eq!(timeline.len(), 0);
    }

    #[tokio::test]
    async fn test_missing_table_returns_empty() {
        // Create pool without passage_songs table
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let passage_id = Uuid::new_v4();

        // Should return empty timeline, not error
        let timeline = load_song_timeline(&pool, passage_id).await.unwrap();

        assert_eq!(timeline.len(), 0);
        assert!(timeline.is_empty());
    }
}

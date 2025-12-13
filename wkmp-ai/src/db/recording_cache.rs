//! Recording cache database operations
//!
//! Caches MusicBrainz recording lookup results to prevent redundant API calls.
//! Uses normalized artist/title as lookup key with 7-day TTL.
//!
//! **[SSI-INT-030]** Recording cache to prevent redundant queries
//!
//! ## Schema
//! The recording_cache table is created automatically via SPEC031 data-driven schema:
//! ```sql
//! CREATE TABLE IF NOT EXISTS recording_cache (
//!     id INTEGER PRIMARY KEY AUTOINCREMENT,
//!     artist_normalized TEXT NOT NULL,
//!     title_normalized TEXT NOT NULL,
//!     mbid TEXT,  -- NULL if not found
//!     confidence REAL,
//!     cached_at TEXT NOT NULL DEFAULT (datetime('now')),
//!     expires_at TEXT NOT NULL,
//!     UNIQUE(artist_normalized, title_normalized)
//! );
//! ```

use sqlx::SqlitePool;

/// Cache time-to-live in days
pub const CACHE_TTL_DAYS: i64 = 7;

/// Cached recording lookup result
#[derive(Debug, Clone)]
pub struct CachedRecording {
    /// Recording MBID (None if not found in MusicBrainz)
    pub mbid: Option<String>,
    /// Confidence/similarity score from original lookup
    pub confidence: Option<f64>,
    /// When this entry was cached
    pub cached_at: String,
}

/// Ensure recording_cache table exists
///
/// Creates the table if it doesn't exist (SPEC031 data-driven schema).
pub async fn ensure_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS recording_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            artist_normalized TEXT NOT NULL,
            title_normalized TEXT NOT NULL,
            mbid TEXT,
            confidence REAL,
            cached_at TEXT NOT NULL DEFAULT (datetime('now')),
            expires_at TEXT NOT NULL,
            UNIQUE(artist_normalized, title_normalized)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for fast lookups
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_recording_cache_lookup
        ON recording_cache(artist_normalized, title_normalized)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get cached recording by normalized artist/title
///
/// Returns None if not cached or cache entry has expired.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `artist_normalized` - Normalized artist name
/// * `title_normalized` - Normalized recording title
pub async fn get_cached(
    pool: &SqlitePool,
    artist_normalized: &str,
    title_normalized: &str,
) -> Result<Option<CachedRecording>, sqlx::Error> {
    let row: Option<(Option<String>, Option<f64>, String)> = sqlx::query_as(
        r#"
        SELECT mbid, confidence, cached_at FROM recording_cache
        WHERE artist_normalized = ? AND title_normalized = ?
        AND datetime(expires_at) > datetime('now')
        "#,
    )
    .bind(artist_normalized)
    .bind(title_normalized)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(mbid, confidence, cached_at)| CachedRecording {
        mbid,
        confidence,
        cached_at,
    }))
}

/// Cache recording lookup result
///
/// Inserts or updates cache entry with 7-day TTL.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `artist_normalized` - Normalized artist name
/// * `title_normalized` - Normalized recording title
/// * `mbid` - Recording MBID (None if not found)
/// * `confidence` - Similarity/confidence score
pub async fn cache_result(
    pool: &SqlitePool,
    artist_normalized: &str,
    title_normalized: &str,
    mbid: Option<&str>,
    confidence: Option<f64>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO recording_cache (artist_normalized, title_normalized, mbid, confidence, expires_at)
        VALUES (?, ?, ?, ?, datetime('now', '+7 days'))
        ON CONFLICT(artist_normalized, title_normalized) DO UPDATE SET
            mbid = excluded.mbid,
            confidence = excluded.confidence,
            cached_at = datetime('now'),
            expires_at = excluded.expires_at
        "#,
    )
    .bind(artist_normalized)
    .bind(title_normalized)
    .bind(mbid)
    .bind(confidence)
    .execute(pool)
    .await?;

    tracing::trace!(
        artist = %artist_normalized,
        title = %title_normalized,
        mbid = ?mbid,
        "Cached recording lookup result"
    );

    Ok(())
}

/// Clear expired cache entries
///
/// Removes entries where expires_at has passed.
pub async fn clear_expired(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM recording_cache
        WHERE datetime(expires_at) <= datetime('now')
        "#,
    )
    .execute(pool)
    .await?;

    let count = result.rows_affected();
    if count > 0 {
        tracing::info!(count, "Cleared expired recording cache entries");
    }

    Ok(count)
}

/// Get cache statistics
///
/// Returns (total_entries, valid_entries, expired_entries)
pub async fn get_stats(pool: &SqlitePool) -> Result<(i64, i64, i64), sqlx::Error> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM recording_cache")
        .fetch_one(pool)
        .await?;

    let valid: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM recording_cache WHERE datetime(expires_at) > datetime('now')",
    )
    .fetch_one(pool)
    .await?;

    let expired = total.0 - valid.0;

    Ok((total.0, valid.0, expired))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        ensure_table(&pool).await.expect("Failed to create table");
        pool
    }

    #[tokio::test]
    async fn test_cache_and_retrieve() {
        let pool = setup_test_db().await;

        // Cache a result
        cache_result(&pool, "beatles", "hey jude", Some("test-mbid"), Some(0.95))
            .await
            .expect("Failed to cache");

        // Retrieve it
        let cached = get_cached(&pool, "beatles", "hey jude")
            .await
            .expect("Failed to get cached");

        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.mbid, Some("test-mbid".to_string()));
        assert!((cached.confidence.unwrap() - 0.95).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_cache_not_found() {
        let pool = setup_test_db().await;

        // Cache a "not found" result (mbid = None)
        cache_result(&pool, "unknown", "song", None, None)
            .await
            .expect("Failed to cache");

        // Retrieve it
        let cached = get_cached(&pool, "unknown", "song")
            .await
            .expect("Failed to get cached");

        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert!(cached.mbid.is_none());
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let pool = setup_test_db().await;

        // Try to get non-existent entry
        let cached = get_cached(&pool, "nonexistent", "entry")
            .await
            .expect("Failed to get cached");

        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_update() {
        let pool = setup_test_db().await;

        // Cache initial result
        cache_result(&pool, "artist", "title", Some("mbid-1"), Some(0.80))
            .await
            .expect("Failed to cache");

        // Update with better result
        cache_result(&pool, "artist", "title", Some("mbid-2"), Some(0.95))
            .await
            .expect("Failed to update cache");

        // Should have updated value
        let cached = get_cached(&pool, "artist", "title")
            .await
            .expect("Failed to get cached")
            .expect("Should have cached entry");

        assert_eq!(cached.mbid, Some("mbid-2".to_string()));
        assert!((cached.confidence.unwrap() - 0.95).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let pool = setup_test_db().await;

        // Add some entries
        cache_result(&pool, "artist1", "title1", Some("mbid1"), Some(0.9))
            .await
            .expect("Failed to cache");
        cache_result(&pool, "artist2", "title2", Some("mbid2"), Some(0.8))
            .await
            .expect("Failed to cache");

        let (total, valid, expired) = get_stats(&pool).await.expect("Failed to get stats");

        assert_eq!(total, 2);
        assert_eq!(valid, 2);
        assert_eq!(expired, 0);
    }
}

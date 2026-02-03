//! AcoustID response cache database operations
//!
//! Caches AcoustID API responses to avoid rate limiting and network delays.
//! Uses SHA256 hash of fingerprint+duration as lookup key since fingerprints are long.
//!
//! **[SSI-CACHE-020]** AcoustID response cache for test iteration speedup
//!
//! ## Schema
//! ```sql
//! CREATE TABLE IF NOT EXISTS acoustid_cache (
//!     id INTEGER PRIMARY KEY AUTOINCREMENT,
//!     fingerprint_hash TEXT NOT NULL UNIQUE,
//!     mbid TEXT,  -- NULL if no match found
//!     score REAL NOT NULL,
//!     artist TEXT,
//!     title TEXT,
//!     cached_at TEXT NOT NULL DEFAULT (datetime('now')),
//!     expires_at TEXT NOT NULL
//! );
//! ```

use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

/// Cache time-to-live in days (730 days = 2 years)
/// AcoustID data is stable, long TTL maximizes cache hits
pub const CACHE_TTL_DAYS: i64 = 730;

/// Cached AcoustID lookup result
#[derive(Debug, Clone)]
pub struct CachedAcoustID {
    /// Recording MBID (None if no match found)
    pub mbid: Option<String>,
    /// AcoustID confidence score (0.0-1.0)
    pub score: f64,
    /// Artist name from AcoustID
    pub artist: Option<String>,
    /// Recording title from AcoustID
    pub title: Option<String>,
    /// When this entry was cached
    pub cached_at: String,
}

/// Compute hash key from fingerprint and duration
///
/// Uses SHA256 to create a fixed-length key from the long fingerprint string.
pub fn compute_key(fingerprint: &str, duration: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(fingerprint.as_bytes());
    hasher.update(b":");
    hasher.update(duration.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Ensure acoustid_cache table exists
pub async fn ensure_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS acoustid_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fingerprint_hash TEXT NOT NULL UNIQUE,
            mbid TEXT,
            score REAL NOT NULL,
            artist TEXT,
            title TEXT,
            cached_at TEXT NOT NULL DEFAULT (datetime('now')),
            expires_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for fast lookups
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_acoustid_cache_hash
        ON acoustid_cache(fingerprint_hash)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get cached AcoustID result by fingerprint and duration
///
/// Returns None if not cached or cache entry has expired.
pub async fn get_cached(
    pool: &SqlitePool,
    fingerprint: &str,
    duration: u64,
) -> Result<Option<CachedAcoustID>, sqlx::Error> {
    let key = compute_key(fingerprint, duration);

    let row: Option<(Option<String>, f64, Option<String>, Option<String>, String)> =
        sqlx::query_as(
            r#"
            SELECT mbid, score, artist, title, cached_at FROM acoustid_cache
            WHERE fingerprint_hash = ?
            AND datetime(expires_at) > datetime('now')
            "#,
        )
        .bind(&key)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|(mbid, score, artist, title, cached_at)| CachedAcoustID {
        mbid,
        score,
        artist,
        title,
        cached_at,
    }))
}

/// Cache AcoustID lookup result
pub async fn cache_result(
    pool: &SqlitePool,
    fingerprint: &str,
    duration: u64,
    mbid: Option<&str>,
    score: f64,
    artist: Option<&str>,
    title: Option<&str>,
) -> Result<(), sqlx::Error> {
    let key = compute_key(fingerprint, duration);

    sqlx::query(
        r#"
        INSERT INTO acoustid_cache (fingerprint_hash, mbid, score, artist, title, expires_at)
        VALUES (?, ?, ?, ?, ?, datetime('now', '+730 days'))
        ON CONFLICT(fingerprint_hash) DO UPDATE SET
            mbid = excluded.mbid,
            score = excluded.score,
            artist = excluded.artist,
            title = excluded.title,
            cached_at = datetime('now'),
            expires_at = excluded.expires_at
        "#,
    )
    .bind(&key)
    .bind(mbid)
    .bind(score)
    .bind(artist)
    .bind(title)
    .execute(pool)
    .await?;

    tracing::trace!(
        mbid = ?mbid,
        score = score,
        artist = ?artist,
        title = ?title,
        "Cached AcoustID result"
    );

    Ok(())
}

/// Get cache statistics
///
/// Returns (total_entries, valid_entries, expired_entries)
pub async fn get_stats(pool: &SqlitePool) -> Result<(i64, i64, i64), sqlx::Error> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM acoustid_cache")
        .fetch_one(pool)
        .await?;

    let valid: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM acoustid_cache WHERE datetime(expires_at) > datetime('now')",
    )
    .fetch_one(pool)
    .await?;

    let expired = total.0 - valid.0;

    Ok((total.0, valid.0, expired))
}

/// Clear expired cache entries
pub async fn clear_expired(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM acoustid_cache
        WHERE datetime(expires_at) <= datetime('now')
        "#,
    )
    .execute(pool)
    .await?;

    let count = result.rows_affected();
    if count > 0 {
        tracing::info!(count, "Cleared expired AcoustID cache entries");
    }

    Ok(count)
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

    #[test]
    fn test_compute_key() {
        let key1 = compute_key("AQAD123", 180);
        let key2 = compute_key("AQAD123", 180);
        let key3 = compute_key("AQAD123", 181);
        let key4 = compute_key("AQAD456", 180);

        assert_eq!(key1, key2); // Same inputs = same key
        assert_ne!(key1, key3); // Different duration = different key
        assert_ne!(key1, key4); // Different fingerprint = different key
        assert_eq!(key1.len(), 64); // SHA256 hex = 64 chars
    }

    #[tokio::test]
    async fn test_cache_and_retrieve() {
        let pool = setup_test_db().await;

        // Cache a result
        cache_result(
            &pool,
            "AQAD123fingerprint",
            180,
            Some("mbid-123"),
            0.95,
            Some("The Beatles"),
            Some("Yesterday"),
        )
        .await
        .expect("Failed to cache");

        // Retrieve it
        let cached = get_cached(&pool, "AQAD123fingerprint", 180)
            .await
            .expect("Failed to get cached");

        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.mbid, Some("mbid-123".to_string()));
        assert!((cached.score - 0.95).abs() < 0.001);
        assert_eq!(cached.artist, Some("The Beatles".to_string()));
        assert_eq!(cached.title, Some("Yesterday".to_string()));
    }

    #[tokio::test]
    async fn test_cache_no_match() {
        let pool = setup_test_db().await;

        // Cache a "no match" result
        cache_result(&pool, "AQAD999", 120, None, 0.0, None, None)
            .await
            .expect("Failed to cache");

        let cached = get_cached(&pool, "AQAD999", 120)
            .await
            .expect("Failed to get cached");

        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert!(cached.mbid.is_none());
        assert!((cached.score - 0.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let pool = setup_test_db().await;

        let cached = get_cached(&pool, "nonexistent", 100)
            .await
            .expect("Failed to get cached");

        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let pool = setup_test_db().await;

        cache_result(&pool, "fp1", 100, Some("mbid1"), 0.9, None, None)
            .await
            .expect("Failed to cache");
        cache_result(&pool, "fp2", 200, Some("mbid2"), 0.8, None, None)
            .await
            .expect("Failed to cache");

        let (total, valid, expired) = get_stats(&pool).await.expect("Failed to get stats");

        assert_eq!(total, 2);
        assert_eq!(valid, 2);
        assert_eq!(expired, 0);
    }
}

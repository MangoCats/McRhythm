//! MusicBrainz release query caching
//!
//! Caches album matching MusicBrainz queries to prevent redundant API calls.
//! Two separate caches for the two-phase album matching process:
//!
//! 1. **Release Search Cache**: Artist/album → list of candidate release MBIDs
//! 2. **Release Details Cache**: Release MBID → full track listing with durations
//!
//! ## Schema
//! ```sql
//! -- Phase 1: Search results cache
//! CREATE TABLE IF NOT EXISTS release_search_cache (
//!     artist_normalized TEXT NOT NULL,
//!     album_normalized TEXT NOT NULL,
//!     release_ids_json TEXT NOT NULL,  -- JSON array of MBIDs
//!     cached_at TEXT NOT NULL DEFAULT (datetime('now')),
//!     expires_at TEXT NOT NULL,
//!     UNIQUE(artist_normalized, album_normalized)
//! );
//!
//! -- Phase 2: Release details cache
//! CREATE TABLE IF NOT EXISTS release_details_cache (
//!     mbid TEXT PRIMARY KEY,
//!     details_json TEXT NOT NULL,  -- Full MBReleaseDetails as JSON
//!     cached_at TEXT NOT NULL DEFAULT (datetime('now')),
//!     expires_at TEXT NOT NULL
//! );
//! ```
//!
//! ## Rationale
//! Album matching is the most expensive operation in the import pipeline:
//! - Searches for up to 50 candidate releases per album file
//! - Fetches full track listings for each candidate (large payloads)
//! - Network latency and rate limiting compound the cost
//!
//! Caching eliminates these redundant queries for:
//! - Duplicate albums (same album in multiple formats/locations)
//! - Common albums across music libraries
//! - Test re-runs and development iterations
//!
//! ## Cache Strategy
//! - **TTL**: 90 days (shorter than recording cache due to release edition churn)
//! - **Normalization**: Lowercase, whitespace-normalized artist/album names
//! - **Size**: Release details can be 10-50KB each (many tracks × metadata)

use sqlx::SqlitePool;

/// Cache time-to-live in days (90 days)
/// Shorter than recording cache due to potential release edition updates
pub const CACHE_TTL_DAYS: i64 = 90;

/// Normalize string for cache lookup (lowercase, trim, collapse whitespace)
fn normalize_for_cache(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Cached release search result
#[derive(Debug, Clone)]
pub struct CachedReleaseSearch {
    /// List of release MBIDs from search
    pub release_ids: Vec<String>,
    /// When this entry was cached
    pub cached_at: String,
}

/// Cached release details
#[derive(Debug, Clone)]
pub struct CachedReleaseDetails {
    /// Full release details as JSON string
    pub details_json: String,
    /// When this entry was cached
    pub cached_at: String,
}

/// Ensure release cache tables exist
pub async fn ensure_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Release search cache
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS release_search_cache (
            artist_normalized TEXT NOT NULL,
            album_normalized TEXT NOT NULL,
            release_ids_json TEXT NOT NULL,
            cached_at TEXT NOT NULL DEFAULT (datetime('now')),
            expires_at TEXT NOT NULL,
            UNIQUE(artist_normalized, album_normalized)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_release_search_lookup
        ON release_search_cache(artist_normalized, album_normalized)
        "#,
    )
    .execute(pool)
    .await?;

    // Release details cache
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS release_details_cache (
            mbid TEXT PRIMARY KEY,
            details_json TEXT NOT NULL,
            cached_at TEXT NOT NULL DEFAULT (datetime('now')),
            expires_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_release_details_expires
        ON release_details_cache(expires_at)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

// =============================================================================
// Release Search Cache
// =============================================================================

/// Get cached release search results
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `artist` - Artist name (will be normalized)
/// * `album` - Album name (will be normalized)
pub async fn get_search_cached(
    pool: &SqlitePool,
    artist: &str,
    album: &str,
) -> Result<Option<CachedReleaseSearch>, sqlx::Error> {
    let artist_norm = normalize_for_cache(artist);
    let album_norm = normalize_for_cache(album);

    let row: Option<(String, String)> = sqlx::query_as(
        r#"
        SELECT release_ids_json, cached_at FROM release_search_cache
        WHERE artist_normalized = ? AND album_normalized = ?
        AND datetime(expires_at) > datetime('now')
        "#,
    )
    .bind(&artist_norm)
    .bind(&album_norm)
    .fetch_optional(pool)
    .await?;

    if let Some((release_ids_json, cached_at)) = row {
        // Deserialize release IDs from JSON array
        match serde_json::from_str::<Vec<String>>(&release_ids_json) {
            Ok(release_ids) => Ok(Some(CachedReleaseSearch {
                release_ids,
                cached_at,
            })),
            Err(e) => {
                tracing::warn!(
                    artist = %artist,
                    album = %album,
                    error = %e,
                    "Failed to deserialize cached release search, treating as cache miss"
                );
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

/// Cache release search results
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `artist` - Artist name (will be normalized)
/// * `album` - Album name (will be normalized)
/// * `release_ids` - List of release MBIDs from search
pub async fn cache_search(
    pool: &SqlitePool,
    artist: &str,
    album: &str,
    release_ids: &[String],
) -> Result<(), sqlx::Error> {
    let artist_norm = normalize_for_cache(artist);
    let album_norm = normalize_for_cache(album);

    let release_ids_json = match serde_json::to_string(release_ids) {
        Ok(json) => json,
        Err(e) => {
            tracing::warn!(
                artist = %artist,
                album = %album,
                error = %e,
                "Failed to serialize release IDs for caching"
            );
            return Ok(()); // Non-fatal
        }
    };

    sqlx::query(
        r#"
        INSERT INTO release_search_cache (artist_normalized, album_normalized, release_ids_json, expires_at)
        VALUES (?, ?, ?, datetime('now', '+' || ? || ' days'))
        ON CONFLICT(artist_normalized, album_normalized) DO UPDATE SET
            release_ids_json = excluded.release_ids_json,
            cached_at = datetime('now'),
            expires_at = datetime('now', '+' || ? || ' days')
        "#,
    )
    .bind(&artist_norm)
    .bind(&album_norm)
    .bind(&release_ids_json)
    .bind(CACHE_TTL_DAYS)
    .bind(CACHE_TTL_DAYS)
    .execute(pool)
    .await?;

    Ok(())
}

// =============================================================================
// Release Details Cache
// =============================================================================

/// Get cached release details
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `mbid` - Release MBID
pub async fn get_details_cached(
    pool: &SqlitePool,
    mbid: &str,
) -> Result<Option<CachedReleaseDetails>, sqlx::Error> {
    let row: Option<(String, String)> = sqlx::query_as(
        r#"
        SELECT details_json, cached_at FROM release_details_cache
        WHERE mbid = ?
        AND datetime(expires_at) > datetime('now')
        "#,
    )
    .bind(mbid)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(details_json, cached_at)| CachedReleaseDetails {
        details_json,
        cached_at,
    }))
}

/// Cache release details
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `mbid` - Release MBID
/// * `details_json` - Full release details as JSON string
pub async fn cache_details(
    pool: &SqlitePool,
    mbid: &str,
    details_json: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO release_details_cache (mbid, details_json, expires_at)
        VALUES (?, ?, datetime('now', '+' || ? || ' days'))
        ON CONFLICT(mbid) DO UPDATE SET
            details_json = excluded.details_json,
            cached_at = datetime('now'),
            expires_at = datetime('now', '+' || ? || ' days')
        "#,
    )
    .bind(mbid)
    .bind(details_json)
    .bind(CACHE_TTL_DAYS)
    .bind(CACHE_TTL_DAYS)
    .execute(pool)
    .await?;

    Ok(())
}

/// Clean up expired cache entries
///
/// Returns tuple: (search entries deleted, details entries deleted)
pub async fn cleanup_expired(pool: &SqlitePool) -> Result<(u64, u64), sqlx::Error> {
    let search_result = sqlx::query(
        r#"
        DELETE FROM release_search_cache
        WHERE datetime(expires_at) <= datetime('now')
        "#,
    )
    .execute(pool)
    .await?;

    let details_result = sqlx::query(
        r#"
        DELETE FROM release_details_cache
        WHERE datetime(expires_at) <= datetime('now')
        "#,
    )
    .execute(pool)
    .await?;

    Ok((search_result.rows_affected(), details_result.rows_affected()))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        ensure_tables(&pool).await.unwrap();
        pool
    }

    #[test]
    fn test_normalization() {
        assert_eq!(normalize_for_cache("The Beatles"), "the beatles");
        assert_eq!(normalize_for_cache("  Abbey  Road  "), "abbey road");
        assert_eq!(normalize_for_cache("UPPERCASE"), "uppercase");
    }

    #[tokio::test]
    async fn test_search_cache_miss() {
        let pool = create_test_pool().await;
        let result = get_search_cached(&pool, "Unknown Artist", "Unknown Album")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_search_cache_hit() {
        let pool = create_test_pool().await;
        let artist = "The Beatles";
        let album = "Abbey Road";
        let release_ids = vec![
            "mbid-1".to_string(),
            "mbid-2".to_string(),
        ];

        // Cache the search
        cache_search(&pool, artist, album, &release_ids)
            .await
            .unwrap();

        // Retrieve from cache (case-insensitive)
        let cached = get_search_cached(&pool, "the beatles", "abbey road")
            .await
            .unwrap();
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.release_ids, release_ids);
    }

    #[tokio::test]
    async fn test_details_cache_hit() {
        let pool = create_test_pool().await;
        let mbid = "test-release-mbid";
        let details_json = r#"{"title":"Test Album","tracks":[{"title":"Track 1"}]}"#;

        // Cache the details
        cache_details(&pool, mbid, details_json).await.unwrap();

        // Retrieve from cache
        let cached = get_details_cached(&pool, mbid).await.unwrap();
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.details_json, details_json);
    }
}

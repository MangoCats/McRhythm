//! MusicBrainz MBID details cache
//!
//! Caches full recording metadata lookups by MBID to prevent redundant API calls.
//! Used by MusicBrainz-Pass2 extractor which fetches complete recording details
//! after Pass 1 fusion identifies the MBID.
//!
//! ## Schema
//! ```sql
//! CREATE TABLE IF NOT EXISTS mbid_details_cache (
//!     mbid TEXT PRIMARY KEY,
//!     details_json TEXT NOT NULL,  -- Full ExtractionResult as JSON
//!     cached_at TEXT NOT NULL DEFAULT (datetime('now')),
//!     expires_at TEXT NOT NULL
//! );
//! ```
//!
//! ## Rationale
//! MusicBrainz-Pass2 is the single biggest bottleneck in the import pipeline (46% of
//! extraction time in tests). Since recording metadata is stable, caching MBID lookups
//! eliminates redundant API calls for:
//! - Duplicate recordings across test runs
//! - Same recordings in different albums
//! - Repeated processing of the same library
//!
//! ## Cache Strategy
//! - **TTL**: 730 days (2 years) - MusicBrainz recording metadata rarely changes
//! - **Key**: Recording MBID (UUID)
//! - **Value**: Complete ExtractionResult serialized as JSON
//! - **Invalidation**: Automatic via expires_at timestamp

use crate::types::ExtractionResult;
use sqlx::SqlitePool;

/// Cache time-to-live in days (730 days = 2 years)
/// MusicBrainz recording metadata is very stable, long TTL reduces API calls
pub const CACHE_TTL_DAYS: i64 = 730;

/// Cached MBID details lookup result
#[derive(Debug, Clone)]
pub struct CachedMBIDDetails {
    /// Full extraction result with recording metadata
    pub extraction_result: ExtractionResult,
    /// When this entry was cached
    pub cached_at: String,
}

/// Ensure mbid_details_cache table exists
///
/// Creates the table if it doesn't exist.
pub async fn ensure_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS mbid_details_cache (
            mbid TEXT PRIMARY KEY,
            details_json TEXT NOT NULL,
            cached_at TEXT NOT NULL DEFAULT (datetime('now')),
            expires_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for expiration cleanup queries
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_mbid_cache_expires
        ON mbid_details_cache(expires_at)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get cached MBID details
///
/// Returns None if not cached or cache entry has expired.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `mbid` - MusicBrainz Recording MBID
pub async fn get_cached(
    pool: &SqlitePool,
    mbid: &str,
) -> Result<Option<CachedMBIDDetails>, sqlx::Error> {
    let row: Option<(String, String)> = sqlx::query_as(
        r#"
        SELECT details_json, cached_at FROM mbid_details_cache
        WHERE mbid = ?
        AND datetime(expires_at) > datetime('now')
        "#,
    )
    .bind(mbid)
    .fetch_optional(pool)
    .await?;

    if let Some((details_json, cached_at)) = row {
        // Deserialize ExtractionResult from JSON
        match serde_json::from_str::<ExtractionResult>(&details_json) {
            Ok(extraction_result) => Ok(Some(CachedMBIDDetails {
                extraction_result,
                cached_at,
            })),
            Err(e) => {
                tracing::warn!(
                    mbid = %mbid,
                    error = %e,
                    "Failed to deserialize cached MBID details, treating as cache miss"
                );
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

/// Cache MBID details result
///
/// Stores extraction result in cache with 2-year TTL.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `mbid` - MusicBrainz Recording MBID
/// * `extraction_result` - Complete extraction result to cache
pub async fn cache_result(
    pool: &SqlitePool,
    mbid: &str,
    extraction_result: &ExtractionResult,
) -> Result<(), sqlx::Error> {
    // Serialize ExtractionResult to JSON
    let details_json = match serde_json::to_string(extraction_result) {
        Ok(json) => json,
        Err(e) => {
            tracing::warn!(
                mbid = %mbid,
                error = %e,
                "Failed to serialize ExtractionResult for caching"
            );
            return Ok(()); // Non-fatal - just skip caching
        }
    };

    sqlx::query(
        r#"
        INSERT INTO mbid_details_cache (mbid, details_json, expires_at)
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
/// Removes all entries where expires_at < now.
/// Returns number of entries deleted.
pub async fn cleanup_expired(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM mbid_details_cache
        WHERE datetime(expires_at) <= datetime('now')
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ConfidenceValue, IdentityExtraction, MetadataExtraction};

    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        ensure_table(&pool).await.unwrap();
        pool
    }

    fn create_test_extraction_result() -> ExtractionResult {
        ExtractionResult {
            identity: Some(IdentityExtraction {
                recording_mbid: "test-mbid-123".to_string(),
                confidence: 0.9,
                source: "Test".to_string(),
            }),
            metadata: Some(MetadataExtraction {
                title: Some(ConfidenceValue::new("Test Song".to_string(), 0.9, "Test")),
                artist: Some(ConfidenceValue::new("Test Artist".to_string(), 0.9, "Test")),
                ..Default::default()
            }),
            musical_flavor: None,
        }
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let pool = create_test_pool().await;
        let result = get_cached(&pool, "nonexistent-mbid").await.unwrap();
        assert!(result.is_none(), "Should be cache miss for nonexistent MBID");
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let pool = create_test_pool().await;
        let mbid = "test-mbid-123";
        let extraction = create_test_extraction_result();

        // Cache the result
        cache_result(&pool, mbid, &extraction).await.unwrap();

        // Retrieve from cache
        let cached = get_cached(&pool, mbid).await.unwrap();
        assert!(cached.is_some(), "Should be cache hit");

        let cached = cached.unwrap();
        assert_eq!(
            cached.extraction_result.identity.as_ref().unwrap().recording_mbid,
            "test-mbid-123".to_string()
        );
    }

    #[tokio::test]
    async fn test_cache_update() {
        let pool = create_test_pool().await;
        let mbid = "test-mbid-456";

        // Cache first result
        let mut extraction1 = create_test_extraction_result();
        extraction1.metadata.as_mut().unwrap().title = Some(ConfidenceValue::new("First Title".to_string(), 0.9, "Test"));
        cache_result(&pool, mbid, &extraction1).await.unwrap();

        // Cache updated result (should replace)
        let mut extraction2 = create_test_extraction_result();
        extraction2.metadata.as_mut().unwrap().title = Some(ConfidenceValue::new("Updated Title".to_string(), 0.9, "Test"));
        cache_result(&pool, mbid, &extraction2).await.unwrap();

        // Verify updated value
        let cached = get_cached(&pool, mbid).await.unwrap().unwrap();
        assert_eq!(
            cached.extraction_result.metadata.as_ref().unwrap().title.as_ref().unwrap().value,
            "Updated Title".to_string()
        );
    }
}

// AcousticBrainz caching layer
//!
//! **Purpose:** Cache AcousticBrainz API responses in database to avoid re-querying
//!
//! **Performance:**
//! - Cache hit: <1ms database query
//! - Cache miss: ~1000ms API query + rate limiting
//!
//! **Coverage:** 91.1% hit rate expected for popular music (per coverage analysis)
//!
//! **Architecture:** Wraps AcousticBrainzClient, implements same interface with caching

use sqlx::{Pool, Sqlite};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, warn};

use super::acousticbrainz_client::{
    ABError, ABLowLevel, MusicalFlavorVector,
};

/// Caching layer errors
#[derive(Debug, Error)]
pub enum CacheError {
    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    /// AcousticBrainz API error (after cache miss)
    #[error("AcousticBrainz API error: {0}")]
    ApiError(#[from] ABError),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// AcousticBrainz client with database caching
///
/// **Usage Pattern:**
/// ```rust,ignore
/// let cache = AcousticBrainzCache::new(db_pool)?;
/// let flavor = cache.get_flavor_vector(recording_mbid).await?;
/// ```
///
/// **Implementation:**
/// - Check cache first (acousticbrainz_cache table)
/// - On cache hit: Return cached data (<1ms)
/// - On cache miss: Return error (AcousticBrainz API offline since 2022)
/// - Cache data is static and never expires
pub struct AcousticBrainzCache {
    /// Database connection pool
    db: Pool<Sqlite>,
}

impl AcousticBrainzCache {
    /// Create new cached AcousticBrainz client
    ///
    /// Note: AcousticBrainz API is offline since 2022. This cache serves
    /// pre-existing cached data only. Cache misses return an error immediately.
    pub fn new(db: Pool<Sqlite>) -> Result<Self, CacheError> {
        Ok(Self { db })
    }

    /// Lookup low-level data by recording MBID (with caching)
    ///
    /// **Algorithm:**
    /// 1. Check database cache
    /// 2. If found: Parse JSON and return
    /// 3. If not found: Return error (API offline since 2022)
    ///
    /// **Error Handling:**
    /// - Cache lookup errors are logged but not fatal (fall through to error)
    /// - Cache misses return RecordingNotFound error
    pub async fn lookup_lowlevel(&self, recording_mbid: &str) -> Result<ABLowLevel, CacheError> {
        debug!(mbid = %recording_mbid, "Checking AcousticBrainz cache");

        // Try cache first
        match self.lookup_cache(recording_mbid).await {
            Ok(Some(cached_data)) => {
                debug!(mbid = %recording_mbid, "Cache hit");
                return Ok(cached_data);
            }
            Ok(None) => {
                debug!(mbid = %recording_mbid, "Cache miss");
            }
            Err(e) => {
                warn!(
                    mbid = %recording_mbid,
                    error = ?e,
                    "Cache lookup failed, falling back to API"
                );
            }
        }

        // Cache miss: AcousticBrainz API is offline since 2022, return error immediately
        // instead of timing out against the dead service
        Err(CacheError::ApiError(ABError::RecordingNotFound(format!(
            "AcousticBrainz cache miss for {} — API disabled (service offline since 2022)",
            recording_mbid
        ))))
    }

    /// Get musical flavor vector for recording (with caching)
    ///
    /// Convenience method that queries and extracts flavor vector
    pub async fn get_flavor_vector(
        &self,
        recording_mbid: &str,
    ) -> Result<MusicalFlavorVector, CacheError> {
        let lowlevel = self.lookup_lowlevel(recording_mbid).await?;
        Ok(MusicalFlavorVector::from_acousticbrainz(&lowlevel))
    }

    /// Lookup data from cache
    ///
    /// Returns:
    /// - Ok(Some(data)) if cache hit
    /// - Ok(None) if cache miss
    /// - Err if database error
    async fn lookup_cache(
        &self,
        recording_mbid: &str,
    ) -> Result<Option<ABLowLevel>, CacheError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT lowlevel_json FROM acousticbrainz_cache WHERE recording_mbid = ?",
        )
        .bind(recording_mbid)
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some((json,)) => {
                let lowlevel: ABLowLevel = serde_json::from_str(&json)?;
                Ok(Some(lowlevel))
            }
            None => Ok(None),
        }
    }

    /// Store data in cache
    ///
    /// Extracts has_tonal, has_rhythm flags for fast filtering
    async fn store_in_cache(
        &self,
        recording_mbid: &str,
        lowlevel: &ABLowLevel,
    ) -> Result<(), CacheError> {
        let json = serde_json::to_string(lowlevel)?;
        let has_tonal = lowlevel.tonal.is_some();
        let has_rhythm = lowlevel.rhythm.is_some();

        // Extract Essentia version if available
        let essentia_version = lowlevel
            .metadata
            .version
            .as_ref()
            .and_then(|v| v.essentia.as_ref())
            .map(|s| s.as_str());

        sqlx::query(
            r#"
            INSERT INTO acousticbrainz_cache
                (recording_mbid, lowlevel_json, has_tonal, has_rhythm, essentia_version, fetched_at)
            VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(recording_mbid) DO UPDATE SET
                lowlevel_json = excluded.lowlevel_json,
                has_tonal = excluded.has_tonal,
                has_rhythm = excluded.has_rhythm,
                essentia_version = excluded.essentia_version,
                fetched_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(recording_mbid)
        .bind(json)
        .bind(has_tonal)
        .bind(has_rhythm)
        .bind(essentia_version)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get cache statistics
    ///
    /// Returns (total_cached, has_tonal_and_rhythm, has_tonal_only, has_rhythm_only, has_neither)
    pub async fn cache_stats(&self) -> Result<(i64, i64, i64, i64, i64), CacheError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM acousticbrainz_cache")
            .fetch_one(&self.db)
            .await?;

        let both: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM acousticbrainz_cache WHERE has_tonal = 1 AND has_rhythm = 1",
        )
        .fetch_one(&self.db)
        .await?;

        let tonal_only: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM acousticbrainz_cache WHERE has_tonal = 1 AND has_rhythm = 0",
        )
        .fetch_one(&self.db)
        .await?;

        let rhythm_only: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM acousticbrainz_cache WHERE has_tonal = 0 AND has_rhythm = 1",
        )
        .fetch_one(&self.db)
        .await?;

        let neither: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM acousticbrainz_cache WHERE has_tonal = 0 AND has_rhythm = 0",
        )
        .fetch_one(&self.db)
        .await?;

        Ok((total, both, tonal_only, rhythm_only, neither))
    }

    /// Clear cache entries older than specified duration
    ///
    /// **Note:** AcousticBrainz data is static (ceased 2022), so cache invalidation
    /// is typically not needed. This method is provided for testing and maintenance.
    pub async fn clear_old_cache(&self, older_than: Duration) -> Result<u64, CacheError> {
        let seconds = older_than.as_secs() as i64;

        let result = sqlx::query(
            "DELETE FROM acousticbrainz_cache WHERE fetched_at < datetime('now', ? || ' seconds')",
        )
        .bind(-seconds)
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Setup in-memory test database with cache table
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create cache table (from migration 008)
        sqlx::query(
            r#"
            CREATE TABLE acousticbrainz_cache (
                recording_mbid TEXT PRIMARY KEY NOT NULL,
                lowlevel_json TEXT NOT NULL,
                has_tonal BOOLEAN NOT NULL DEFAULT 0,
                has_rhythm BOOLEAN NOT NULL DEFAULT 0,
                fetched_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                essentia_version TEXT,
                CHECK (length(recording_mbid) = 36),
                CHECK (json_valid(lowlevel_json))
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_cache_creation() {
        let pool = setup_test_db().await;
        let _cache = AcousticBrainzCache::new(pool).unwrap();
        // Just verify it can be created
    }

    #[tokio::test]
    async fn test_cache_stats_empty() {
        let pool = setup_test_db().await;
        let cache = AcousticBrainzCache::new(pool).unwrap();

        let (total, both, tonal_only, rhythm_only, neither) =
            cache.cache_stats().await.unwrap();

        assert_eq!(total, 0);
        assert_eq!(both, 0);
        assert_eq!(tonal_only, 0);
        assert_eq!(rhythm_only, 0);
        assert_eq!(neither, 0);
    }

    #[tokio::test]
    async fn test_lookup_cache_miss() {
        let pool = setup_test_db().await;
        let cache = AcousticBrainzCache::new(pool).unwrap();

        let result = cache
            .lookup_cache("00000000-0000-0000-0000-000000000000")
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let pool = setup_test_db().await;
        let cache = AcousticBrainzCache::new(pool).unwrap();

        // Create minimal test data
        let lowlevel = ABLowLevel {
            metadata: super::super::acousticbrainz_client::ABMetadata {
                version: Some(super::super::acousticbrainz_client::ABVersion {
                    essentia: Some("2.1".to_string()),
                }),
                audio_properties: None,
            },
            tonal: Some(super::super::acousticbrainz_client::ABTonal {
                key_key: Some("C".to_string()),
                key_scale: Some("major".to_string()),
                key_strength: Some(0.85),
                chords_key: None,
                chords_scale: None,
            }),
            rhythm: Some(super::super::acousticbrainz_client::ABRhythm {
                bpm: Some(120.0),
                onset_rate: None,
                danceability: Some(0.7),
            }),
            lowlevel: None,
        };

        let mbid = "12345678-1234-1234-1234-123456789012";

        // Store in cache
        cache.store_in_cache(mbid, &lowlevel).await.unwrap();

        // Retrieve from cache
        let cached = cache.lookup_cache(mbid).await.unwrap();
        assert!(cached.is_some());

        let cached_data = cached.unwrap();
        assert!(cached_data.tonal.is_some());
        assert!(cached_data.rhythm.is_some());
        assert_eq!(
            cached_data.tonal.unwrap().key_key,
            Some("C".to_string())
        );
        assert_eq!(cached_data.rhythm.unwrap().bpm, Some(120.0));

        // Verify stats
        let (total, both, tonal_only, rhythm_only, neither) =
            cache.cache_stats().await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(both, 1);
        assert_eq!(tonal_only, 0);
        assert_eq!(rhythm_only, 0);
        assert_eq!(neither, 0);
    }

    #[tokio::test]
    async fn test_upsert_duplicate() {
        let pool = setup_test_db().await;
        let cache = AcousticBrainzCache::new(pool).unwrap();

        let lowlevel = ABLowLevel {
            metadata: super::super::acousticbrainz_client::ABMetadata {
                version: None,
                audio_properties: None,
            },
            tonal: None,
            rhythm: None,
            lowlevel: None,
        };

        let mbid = "12345678-1234-1234-1234-123456789012";

        // Store twice
        cache.store_in_cache(mbid, &lowlevel).await.unwrap();
        cache.store_in_cache(mbid, &lowlevel).await.unwrap();

        // Should still have only 1 entry
        let (total, _, _, _, _) = cache.cache_stats().await.unwrap();
        assert_eq!(total, 1);
    }

    #[tokio::test]
    async fn test_clear_old_cache() {
        let pool = setup_test_db().await;
        let cache = AcousticBrainzCache::new(pool).unwrap();

        let lowlevel = ABLowLevel {
            metadata: super::super::acousticbrainz_client::ABMetadata {
                version: None,
                audio_properties: None,
            },
            tonal: None,
            rhythm: None,
            lowlevel: None,
        };

        // Store entry
        cache
            .store_in_cache("12345678-1234-1234-1234-123456789012", &lowlevel)
            .await
            .unwrap();

        // Clearing entries older than 1 hour should not remove it
        let removed = cache
            .clear_old_cache(Duration::from_secs(3600))
            .await
            .unwrap();
        assert_eq!(removed, 0);

        let (total, _, _, _, _) = cache.cache_stats().await.unwrap();
        assert_eq!(total, 1);
    }
}

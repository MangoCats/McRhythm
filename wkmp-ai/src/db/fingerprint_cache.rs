//! Fingerprint cache database operations
//!
//! Caches Chromaprint fingerprints by file hash to avoid expensive re-computation.
//! Fingerprints are deterministic for a given audio file, so they never expire.
//!
//! **[SSI-CACHE-010]** Fingerprint cache for test iteration speedup
//!
//! ## Schema
//! ```sql
//! CREATE TABLE IF NOT EXISTS fingerprint_cache (
//!     id INTEGER PRIMARY KEY AUTOINCREMENT,
//!     file_hash TEXT NOT NULL UNIQUE,
//!     fingerprint TEXT NOT NULL,
//!     duration_seconds REAL NOT NULL,
//!     cached_at TEXT NOT NULL DEFAULT (datetime('now'))
//! );
//! ```

use sqlx::SqlitePool;

/// Cached fingerprint result
#[derive(Debug, Clone)]
pub struct CachedFingerprint {
    /// Chromaprint fingerprint string
    pub fingerprint: String,
    /// Audio duration in seconds
    pub duration_seconds: f64,
    /// When this entry was cached
    pub cached_at: String,
}

/// Ensure fingerprint_cache table exists
pub async fn ensure_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS fingerprint_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_hash TEXT NOT NULL UNIQUE,
            fingerprint TEXT NOT NULL,
            duration_seconds REAL NOT NULL,
            cached_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for fast lookups
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_fingerprint_cache_hash
        ON fingerprint_cache(file_hash)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get cached fingerprint by file hash
///
/// Returns None if not cached.
pub async fn get_cached(
    pool: &SqlitePool,
    file_hash: &str,
) -> Result<Option<CachedFingerprint>, sqlx::Error> {
    let row: Option<(String, f64, String)> = sqlx::query_as(
        r#"
        SELECT fingerprint, duration_seconds, cached_at FROM fingerprint_cache
        WHERE file_hash = ?
        "#,
    )
    .bind(file_hash)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(fingerprint, duration_seconds, cached_at)| CachedFingerprint {
        fingerprint,
        duration_seconds,
        cached_at,
    }))
}

/// Cache fingerprint result
///
/// Fingerprints never change for a given file, so no expiry needed.
pub async fn cache_result(
    pool: &SqlitePool,
    file_hash: &str,
    fingerprint: &str,
    duration_seconds: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO fingerprint_cache (file_hash, fingerprint, duration_seconds)
        VALUES (?, ?, ?)
        ON CONFLICT(file_hash) DO UPDATE SET
            fingerprint = excluded.fingerprint,
            duration_seconds = excluded.duration_seconds,
            cached_at = datetime('now')
        "#,
    )
    .bind(file_hash)
    .bind(fingerprint)
    .bind(duration_seconds)
    .execute(pool)
    .await?;

    tracing::trace!(
        file_hash = %file_hash,
        fingerprint_len = fingerprint.len(),
        duration = duration_seconds,
        "Cached fingerprint"
    );

    Ok(())
}

/// Get cache statistics
///
/// Returns total number of cached fingerprints
pub async fn get_stats(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM fingerprint_cache")
        .fetch_one(pool)
        .await?;

    Ok(count.0)
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

        // Cache a fingerprint
        cache_result(&pool, "abc123hash", "AQAD123fingerprint", 180.5)
            .await
            .expect("Failed to cache");

        // Retrieve it
        let cached = get_cached(&pool, "abc123hash")
            .await
            .expect("Failed to get cached");

        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.fingerprint, "AQAD123fingerprint");
        assert!((cached.duration_seconds - 180.5).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let pool = setup_test_db().await;

        let cached = get_cached(&pool, "nonexistent")
            .await
            .expect("Failed to get cached");

        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let pool = setup_test_db().await;

        cache_result(&pool, "hash1", "fp1", 100.0)
            .await
            .expect("Failed to cache");
        cache_result(&pool, "hash2", "fp2", 200.0)
            .await
            .expect("Failed to cache");

        let count = get_stats(&pool).await.expect("Failed to get stats");
        assert_eq!(count, 2);
    }
}

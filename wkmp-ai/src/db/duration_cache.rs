//! Accurate Duration Cache
//!
//! Caches accurate audio file durations obtained via full decode (frame counting).
//! This is necessary because VBR MP3 files without Xing/VBRI headers cannot have
//! their duration accurately determined from metadata alone.
//!
//! **[SSI-DUR-010]** Accurate duration cache for Stage 2 matching
//!
//! ## Background
//! - Lofty (metadata library) assumes 32kbps for VBR MP3s without headers → wrong durations
//! - Full decode (counting frames) gives accurate duration but is slow (~10s per file)
//! - This cache stores accurate durations to avoid repeated full decodes
//!
//! ## Schema
//! ```sql
//! CREATE TABLE IF NOT EXISTS duration_cache (
//!     id INTEGER PRIMARY KEY AUTOINCREMENT,
//!     file_hash TEXT NOT NULL UNIQUE,
//!     accurate_duration_secs REAL NOT NULL,
//!     sample_rate INTEGER NOT NULL,
//!     frame_count INTEGER NOT NULL,
//!     method TEXT NOT NULL,  -- 'full_decode', 'lofty_verified', 'ffprobe'
//!     cached_at TEXT NOT NULL DEFAULT (datetime('now'))
//! );
//! ```

use sqlx::SqlitePool;

/// Cached accurate duration result
#[derive(Debug, Clone)]
pub struct CachedDuration {
    /// Accurate duration in seconds (from full decode)
    pub duration_secs: f64,
    /// Sample rate used for calculation
    pub sample_rate: u32,
    /// Number of audio frames decoded
    pub frame_count: u64,
    /// Method used to obtain duration
    pub method: String,
    /// When this entry was cached
    pub cached_at: String,
}

/// Method used to obtain accurate duration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationMethod {
    /// Full decode via symphonia (most accurate)
    FullDecode,
    /// Lofty duration verified against expected range
    LoftyVerified,
    /// Duration from FFprobe
    Ffprobe,
    /// Frame counting without full decode
    FrameCount,
}

impl DurationMethod {
    /// Convert to string for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            DurationMethod::FullDecode => "full_decode",
            DurationMethod::LoftyVerified => "lofty_verified",
            DurationMethod::Ffprobe => "ffprobe",
            DurationMethod::FrameCount => "frame_count",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "full_decode" => Some(DurationMethod::FullDecode),
            "lofty_verified" => Some(DurationMethod::LoftyVerified),
            "ffprobe" => Some(DurationMethod::Ffprobe),
            "frame_count" => Some(DurationMethod::FrameCount),
            _ => None,
        }
    }
}

/// Ensure duration_cache table exists
pub async fn ensure_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS duration_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_hash TEXT NOT NULL UNIQUE,
            accurate_duration_secs REAL NOT NULL,
            sample_rate INTEGER NOT NULL,
            frame_count INTEGER NOT NULL,
            method TEXT NOT NULL,
            cached_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for fast lookups
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_duration_cache_hash
        ON duration_cache(file_hash)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get cached accurate duration by file hash
///
/// Returns None if not cached.
pub async fn get_cached(
    pool: &SqlitePool,
    file_hash: &str,
) -> Result<Option<CachedDuration>, sqlx::Error> {
    let row: Option<(f64, i64, i64, String, String)> = sqlx::query_as(
        r#"
        SELECT accurate_duration_secs, sample_rate, frame_count, method, cached_at
        FROM duration_cache
        WHERE file_hash = ?
        "#,
    )
    .bind(file_hash)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(duration_secs, sample_rate, frame_count, method, cached_at)| CachedDuration {
        duration_secs,
        sample_rate: sample_rate as u32,
        frame_count: frame_count as u64,
        method,
        cached_at,
    }))
}

/// Cache accurate duration result
pub async fn cache_result(
    pool: &SqlitePool,
    file_hash: &str,
    duration_secs: f64,
    sample_rate: u32,
    frame_count: u64,
    method: DurationMethod,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO duration_cache (file_hash, accurate_duration_secs, sample_rate, frame_count, method)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(file_hash) DO UPDATE SET
            accurate_duration_secs = excluded.accurate_duration_secs,
            sample_rate = excluded.sample_rate,
            frame_count = excluded.frame_count,
            method = excluded.method,
            cached_at = datetime('now')
        "#,
    )
    .bind(file_hash)
    .bind(duration_secs)
    .bind(sample_rate as i64)
    .bind(frame_count as i64)
    .bind(method.as_str())
    .execute(pool)
    .await?;

    tracing::trace!(
        file_hash = %file_hash,
        duration = duration_secs,
        method = method.as_str(),
        "Cached accurate duration"
    );

    Ok(())
}

/// Get cache statistics
///
/// Returns (total_entries, full_decode_count, lofty_verified_count)
pub async fn get_stats(pool: &SqlitePool) -> Result<(i64, i64, i64), sqlx::Error> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM duration_cache")
        .fetch_one(pool)
        .await?;

    let full_decode: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM duration_cache WHERE method = 'full_decode'",
    )
    .fetch_one(pool)
    .await?;

    let lofty_verified: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM duration_cache WHERE method = 'lofty_verified'",
    )
    .fetch_one(pool)
    .await?;

    Ok((total.0, full_decode.0, lofty_verified.0))
}

/// Batch insert multiple duration cache entries
///
/// Efficient for populating cache from existing data
pub async fn batch_insert(
    pool: &SqlitePool,
    entries: &[(String, f64, u32, u64, DurationMethod)],
) -> Result<usize, sqlx::Error> {
    let mut inserted = 0;

    for (file_hash, duration_secs, sample_rate, frame_count, method) in entries {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO duration_cache
            (file_hash, accurate_duration_secs, sample_rate, frame_count, method)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(file_hash)
        .bind(duration_secs)
        .bind(*sample_rate as i64)
        .bind(*frame_count as i64)
        .bind(method.as_str())
        .execute(pool)
        .await?;

        if result.rows_affected() > 0 {
            inserted += 1;
        }
    }

    Ok(inserted)
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

        // Cache a duration
        cache_result(&pool, "abc123hash", 312.5, 44100, 11961, DurationMethod::FullDecode)
            .await
            .expect("Failed to cache");

        // Retrieve it
        let cached = get_cached(&pool, "abc123hash")
            .await
            .expect("Failed to get cached");

        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert!((cached.duration_secs - 312.5).abs() < 0.001);
        assert_eq!(cached.sample_rate, 44100);
        assert_eq!(cached.frame_count, 11961);
        assert_eq!(cached.method, "full_decode");
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

        cache_result(&pool, "hash1", 100.0, 44100, 1000, DurationMethod::FullDecode)
            .await
            .expect("Failed to cache");
        cache_result(&pool, "hash2", 200.0, 44100, 2000, DurationMethod::LoftyVerified)
            .await
            .expect("Failed to cache");

        let (total, full_decode, lofty_verified) = get_stats(&pool).await.expect("Failed to get stats");
        assert_eq!(total, 2);
        assert_eq!(full_decode, 1);
        assert_eq!(lofty_verified, 1);
    }

    #[tokio::test]
    async fn test_batch_insert() {
        let pool = setup_test_db().await;

        let entries = vec![
            ("hash1".to_string(), 100.0, 44100u32, 1000u64, DurationMethod::FullDecode),
            ("hash2".to_string(), 200.0, 44100u32, 2000u64, DurationMethod::FullDecode),
            ("hash3".to_string(), 300.0, 44100u32, 3000u64, DurationMethod::LoftyVerified),
        ];

        let inserted = batch_insert(&pool, &entries).await.expect("Failed to batch insert");
        assert_eq!(inserted, 3);

        let (total, _, _) = get_stats(&pool).await.expect("Failed to get stats");
        assert_eq!(total, 3);
    }
}

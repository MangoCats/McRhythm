//! Accurate Duration Service
//!
//! Provides accurate audio file durations via full decode with caching.
//! Essential for Stage 2 matching where duration validation is critical.
//!
//! **[SSI-DUR-020]** Accurate duration computation service
//!
//! ## Why This Exists
//! VBR MP3 files without Xing/VBRI headers have incorrect duration in metadata.
//! Lofty (and similar libraries) assume 32kbps minimum bitrate, causing 3-5x
//! duration overestimates. Full decode counts actual frames for accurate duration.
//!
//! ## Performance
//! - Full decode: ~10 seconds per file (depends on file size)
//! - Cache lookup: ~1ms
//! - Lofty metadata: ~50ms but often wrong for VBR MP3s

use crate::db::duration_cache::{self, CachedDuration, DurationMethod};
use crate::utils::audio_decoder::decode_audio_file;
use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::path::Path;

/// Accurate duration result
#[derive(Debug, Clone)]
pub struct AccurateDuration {
    /// Duration in seconds
    pub duration_secs: f64,
    /// Sample rate
    pub sample_rate: u32,
    /// Number of frames decoded
    pub frame_count: u64,
    /// Method used to obtain duration
    pub method: DurationMethod,
    /// Whether this came from cache
    pub from_cache: bool,
}

/// Get accurate duration for an audio file
///
/// Checks cache first; if not cached, performs full decode and caches result.
///
/// # Arguments
/// * `pool` - Database pool for cache access
/// * `file_path` - Path to audio file
/// * `file_hash` - Hash of the file (for cache key)
///
/// # Returns
/// * `AccurateDuration` with duration in seconds
pub async fn get_accurate_duration(
    pool: &SqlitePool,
    file_path: &Path,
    file_hash: &str,
) -> Result<AccurateDuration> {
    // Check cache first
    if let Some(cached) = duration_cache::get_cached(pool, file_hash).await? {
        return Ok(AccurateDuration {
            duration_secs: cached.duration_secs,
            sample_rate: cached.sample_rate,
            frame_count: cached.frame_count,
            method: DurationMethod::from_str(&cached.method).unwrap_or(DurationMethod::FullDecode),
            from_cache: true,
        });
    }

    // Not cached - perform full decode
    tracing::debug!(path = %file_path.display(), "Computing accurate duration via full decode");

    let decoded = decode_audio_file(file_path)
        .with_context(|| format!("Failed to decode: {}", file_path.display()))?;

    let duration_secs = decoded.duration_seconds;
    let sample_rate = decoded.sample_rate;
    let frame_count = decoded.samples.len() as u64;

    // Cache the result
    duration_cache::cache_result(
        pool,
        file_hash,
        duration_secs,
        sample_rate,
        frame_count,
        DurationMethod::FullDecode,
    )
    .await?;

    tracing::debug!(
        path = %file_path.display(),
        duration = duration_secs,
        "Cached accurate duration"
    );

    Ok(AccurateDuration {
        duration_secs,
        sample_rate,
        frame_count,
        method: DurationMethod::FullDecode,
        from_cache: false,
    })
}

/// Check if a lofty duration is suspicious (likely wrong)
///
/// VBR MP3s without headers show ~3-5x inflated durations because lofty
/// assumes 32kbps minimum bitrate.
///
/// # Arguments
/// * `lofty_duration` - Duration from lofty metadata
/// * `file_size_bytes` - File size in bytes
///
/// # Returns
/// * `true` if duration is likely wrong and needs full decode
pub fn is_duration_suspicious(lofty_duration: f64, file_size_bytes: u64) -> bool {
    if lofty_duration <= 0.0 {
        return true;
    }

    // Calculate implied bitrate
    let implied_bitrate_kbps = (file_size_bytes as f64 * 8.0) / (lofty_duration * 1000.0);

    // If implied bitrate is ~32kbps (minimum MP3), duration is likely wrong
    // Most VBR MP3s are 128-320kbps average
    if implied_bitrate_kbps < 40.0 {
        return true;
    }

    // If duration > 10 minutes for a file < 50MB, suspicious for pop music
    // (50MB at 128kbps = ~50 minutes, so < 50MB with > 10min = low bitrate)
    if lofty_duration > 600.0 && file_size_bytes < 50_000_000 {
        let min_reasonable_bitrate = (file_size_bytes as f64 * 8.0) / (600.0 * 1000.0);
        if implied_bitrate_kbps < min_reasonable_bitrate {
            return true;
        }
    }

    false
}

/// Get accurate duration, using lofty if trustworthy
///
/// This is a smart wrapper that:
/// 1. Checks cache first
/// 2. If not cached and lofty duration looks reasonable, uses it
/// 3. If not cached and lofty duration is suspicious, does full decode
///
/// # Arguments
/// * `pool` - Database pool for cache access
/// * `file_path` - Path to audio file
/// * `file_hash` - Hash of the file (for cache key)
/// * `lofty_duration` - Duration from lofty (may be wrong)
///
/// # Returns
/// * `AccurateDuration` with duration in seconds
pub async fn get_duration_smart(
    pool: &SqlitePool,
    file_path: &Path,
    file_hash: &str,
    lofty_duration: f64,
) -> Result<AccurateDuration> {
    // Check cache first
    if let Some(cached) = duration_cache::get_cached(pool, file_hash).await? {
        return Ok(AccurateDuration {
            duration_secs: cached.duration_secs,
            sample_rate: cached.sample_rate,
            frame_count: cached.frame_count,
            method: DurationMethod::from_str(&cached.method).unwrap_or(DurationMethod::FullDecode),
            from_cache: true,
        });
    }

    // Get file size
    let file_size = std::fs::metadata(file_path)
        .with_context(|| format!("Failed to get file size: {}", file_path.display()))?
        .len();

    // Check if lofty duration is suspicious
    if is_duration_suspicious(lofty_duration, file_size) {
        tracing::debug!(
            path = %file_path.display(),
            lofty_duration,
            file_size,
            "Lofty duration suspicious, using full decode"
        );
        return get_accurate_duration(pool, file_path, file_hash).await;
    }

    // Lofty duration looks reasonable - cache and use it
    let sample_rate = 44100; // Assume standard rate for verified durations
    let frame_count = (lofty_duration * sample_rate as f64) as u64;

    duration_cache::cache_result(
        pool,
        file_hash,
        lofty_duration,
        sample_rate,
        frame_count,
        DurationMethod::LoftyVerified,
    )
    .await?;

    Ok(AccurateDuration {
        duration_secs: lofty_duration,
        sample_rate,
        frame_count,
        method: DurationMethod::LoftyVerified,
        from_cache: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_duration_suspicious() {
        // Normal file: 5MB, 3 minutes -> ~222 kbps (reasonable)
        assert!(!is_duration_suspicious(180.0, 5_000_000));

        // Suspicious: 5MB, 25 minutes -> ~27 kbps (too low)
        assert!(is_duration_suspicious(1500.0, 5_000_000));

        // Suspicious: 5MB, 24 minutes -> 32 kbps (minimum assumption)
        assert!(is_duration_suspicious(1453.6, 5_824_117));

        // Normal: 5MB, 5.2 minutes -> ~128 kbps
        assert!(!is_duration_suspicious(312.5, 5_824_117));

        // Edge case: zero duration
        assert!(is_duration_suspicious(0.0, 5_000_000));

        // Edge case: very small file with long duration
        assert!(is_duration_suspicious(600.0, 1_000_000));
    }
}

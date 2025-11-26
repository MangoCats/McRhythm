//! # Timing Utilities
//!
//! Stagger delay calculation for concurrent album processing.
//!
//! ## Features
//! - **Stagger calculation**: Delay calculation for concurrent album processing
//!
//! ## Related Modules
//! - `query_stats`: QueryStats and spawn_heartbeat_task
//! - `constants`: STAGGER_MULTIPLIER, MAX_CONCURRENT_ALBUMS, MB_RATE_LIMIT_MS

use crate::constants::*;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

/// Calculate stagger delay for concurrent album processing
///
/// For the initial batch of MAX_CONCURRENT_ALBUMS albums, staggers their start times
/// to reduce MusicBrainz API rate limit contention. Albums beyond the initial batch
/// start immediately when their slot becomes available.
///
/// # Arguments
/// * `album_idx` - Album index (0-based)
///
/// # Returns
/// Tuple of (should_stagger, delay_ms, stagger_position):
/// * `should_stagger` - Whether this album should wait before starting
/// * `delay_ms` - Delay in milliseconds
/// * `stagger_position` - Position in stagger sequence (0-5 for MAX_CONCURRENT_ALBUMS=6)
///
/// # Algorithm
/// ```ignore
/// stagger_position = album_idx % MAX_CONCURRENT_ALBUMS
/// delay_ms = stagger_position × STAGGER_MULTIPLIER × MB_RATE_LIMIT_MS
/// ```
///
/// # Example
/// With MAX_CONCURRENT_ALBUMS=6, STAGGER_MULTIPLIER=30, MB_RATE_LIMIT_MS=1550:
/// - Album 0: 0s delay (position 0)
/// - Album 1: 46.5s delay (1 × 30 × 1550ms)
/// - Album 2: 93s delay (2 × 30 × 1550ms)
/// - Album 6: 0s delay (position 0, starts when Album 0 finishes)
pub(crate) fn calculate_stagger_delay(album_idx: usize) -> (bool, u64, usize) {
    let stagger_position = album_idx % MAX_CONCURRENT_ALBUMS;
    let should_stagger = STAGGER_MULTIPLIER > 0
        && stagger_position > 0
        && album_idx < MAX_CONCURRENT_ALBUMS;

    let delay_ms = if should_stagger {
        stagger_position as u64 * STAGGER_MULTIPLIER * MB_RATE_LIMIT_MS
    } else {
        0
    };

    (should_stagger, delay_ms, stagger_position)
}

/// Apply stagger delay with logging
///
/// Convenience function that combines calculate_stagger_delay() with actual sleep
/// and informative logging.
///
/// # Arguments
/// * `album_idx` - Album index (0-based)
///
/// # Example
/// ```ignore
/// apply_stagger_delay(1).await;  // Waits 46.5s and logs reason
/// ```
pub(crate) async fn apply_stagger_delay(album_idx: usize) {
    let (should_stagger, delay_ms, stagger_position) = calculate_stagger_delay(album_idx);

    if should_stagger {
        let delay_secs = delay_ms / 1000;
        info!(
            "[A{}] Staggered start: waiting {}s ({} position × {} multiplier × {}ms rate limit)...",
            album_idx + 1,
            delay_secs,
            stagger_position,
            STAGGER_MULTIPLIER,
            MB_RATE_LIMIT_MS
        );
        sleep(Duration::from_millis(delay_ms)).await;
    }
}

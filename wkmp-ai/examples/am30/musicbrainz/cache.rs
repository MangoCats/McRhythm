//! # MusicBrainz Cache Implementation
//!
//! File-based cache with 3 modes: Disabled, ReadWrite, ReadOnly.
//!
//! ## Features
//! - **3 cache modes**: Disabled, ReadWrite (build cache), ReadOnly (cache-only testing)
//! - **SHA-256 query hashing**: Deterministic cache keys for search queries
//! - **Human-readable JSON**: Pretty-printed for manual inspection
//! - **Cache statistics**: Hit/miss tracking with reporting
//! - **Metadata tracking**: Cache version, creation date, last updated timestamp
//!
//! ## File Structure
//! ```text
//! cache/
//!   ├── metadata.json        # Cache statistics and version info
//!   ├── searches/            # Search query results (SHA-256 hashed filenames)
//!   │   └── abc123def456.json
//!   └── releases/            # Release details (MBID filenames)
//!       └── 550e8400-e29b-41d4-a716-446655440000.json
//! ```
//!
//! ## Requirements Coverage
//! - REQ-CACHE-020: Search query caching (hash_query, load_cached_search, store_search_cache)
//! - REQ-CACHE-030: Release details caching (load_cached_release, store_release_cache)
//! - REQ-CACHE-050: Directory structure creation
//! - REQ-CACHE-080: Statistics reporting (print_cache_statistics)
//! - REQ-CACHE-090: Error handling (corrupted cache, read/write failures)
//! - REQ-CACHE-130: Human-readable JSON format
//!
//! ## Related Modules
//! - `types`: CacheMode, CacheConfig, CacheStats, CachedSearch, CachedRelease, CacheMetadata
//! - `musicbrainz::api`: MBClient (primary consumer)

use crate::types::*;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::atomic::Ordering;

// =============================================================================
// Cache Key Generation
// =============================================================================

/// Hash query string for cache key using SHA-256 (first 16 hex chars)
///
/// Creates deterministic cache keys for MusicBrainz search queries by hashing
/// the query string with SHA-256 and taking the first 16 hex characters.
///
/// # Arguments
/// * `query` - Search query string (e.g., "artist:Pink Floyd AND release:The Wall")
///
/// # Returns
/// 16-character hex string (e.g., "abc123def4567890")
///
/// # Example
/// ```ignore
/// let key = hash_query("artist:Pink Floyd AND release:The Wall");
/// assert_eq!(key.len(), 16);
/// // cache/searches/abc123def4567890.json
/// ```
///
/// # Requirements
/// REQ-CACHE-020: Search Query Caching
pub(crate) fn hash_query(query: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}

// =============================================================================
// Search Query Cache Operations
// =============================================================================

/// Load cached search response from disk
///
/// Attempts to load a previously cached MusicBrainz search response using the
/// SHA-256 hash of the query string as the cache key.
///
/// # Arguments
/// * `cache_dir` - Root cache directory (e.g., "./cache")
/// * `key` - Hashed query key (16-char hex string from hash_query)
///
/// # Returns
/// - `Ok(Some(response))` - Cache hit, found cached response
/// - `Ok(None)` - Cache miss, file doesn't exist
/// - `Err(message)` - Cache file corrupted or read failure
///
/// # Error Handling
/// REQ-CACHE-090: Reports cache file read failures and parse errors separately
///
/// # Requirements
/// REQ-CACHE-020: Search Query Caching
/// REQ-CACHE-090: Error Handling
pub(crate) fn load_cached_search(
    cache_dir: &Path,
    key: &str,
) -> Result<Option<MBSearchResponse>, String> {
    let search_cache_dir = cache_dir.join("searches");
    let cache_file = search_cache_dir.join(format!("{}.json", key));

    if !cache_file.exists() {
        return Ok(None); // Cache miss
    }

    // REQ-CACHE-090: Handle cache file read failures
    let contents = std::fs::read_to_string(&cache_file)
        .map_err(|e| format!("Cache file read failure: {}", e))?;

    // REQ-CACHE-090: Handle corrupted cache (parse failure)
    let cached: CachedSearch = serde_json::from_str(&contents)
        .map_err(|e| format!("Cache file parse failure (corrupted): {}", e))?;

    Ok(Some(cached.response))
}

/// Store search response in cache
///
/// Writes a MusicBrainz search response to disk cache with metadata (query, timestamp).
/// Creates cache directory structure if it doesn't exist.
///
/// # Arguments
/// * `cache_dir` - Root cache directory
/// * `key` - Hashed query key (from hash_query)
/// * `query` - Original query string (stored for debugging/inspection)
/// * `response` - MusicBrainz search response to cache
///
/// # Returns
/// - `Ok(())` - Cache write successful
/// - `Err(message)` - Directory creation, serialization, or write failure
///
/// # File Format
/// Pretty-printed JSON with structure:
/// ```json
/// {
///   "query": "artist:Pink Floyd AND release:The Wall",
///   "timestamp": "2025-01-15T10:30:45Z",
///   "response": { ... }
/// }
/// ```
///
/// # Requirements
/// REQ-CACHE-020: Search Query Caching
/// REQ-CACHE-050: Cache directory creation
/// REQ-CACHE-130: Human-readable JSON format
pub(crate) fn store_search_cache(
    cache_dir: &Path,
    key: &str,
    query: &str,
    response: &MBSearchResponse,
) -> Result<(), String> {
    let search_cache_dir = cache_dir.join("searches");

    // REQ-CACHE-050: Ensure cache directory exists
    std::fs::create_dir_all(&search_cache_dir)
        .map_err(|e| format!("Cache directory creation failure: {}", e))?;

    let cache_file = search_cache_dir.join(format!("{}.json", key));

    let cached = CachedSearch {
        query: query.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        response: response.clone(),
    };

    // REQ-CACHE-130: Human-readable JSON format (pretty-print)
    let json = serde_json::to_string_pretty(&cached)
        .map_err(|e| format!("Cache serialization failure: {}", e))?;

    // REQ-CACHE-090: Handle cache file write failures
    std::fs::write(&cache_file, json).map_err(|e| format!("Cache file write failure: {}", e))?;

    Ok(())
}

// =============================================================================
// Release Details Cache Operations
// =============================================================================

/// Load cached release details from disk
///
/// Attempts to load previously cached MusicBrainz release details using the
/// release MBID as the cache key.
///
/// # Arguments
/// * `cache_dir` - Root cache directory
/// * `mbid` - MusicBrainz Release ID (UUID format)
///
/// # Returns
/// - `Ok(Some(details))` - Cache hit, found cached release
/// - `Ok(None)` - Cache miss, file doesn't exist
/// - `Err(message)` - Cache file corrupted or read failure
///
/// # Requirements
/// REQ-CACHE-030: Release Details Caching
/// REQ-CACHE-090: Error Handling
pub(crate) fn load_cached_release(
    cache_dir: &Path,
    mbid: &str,
) -> Result<Option<MBReleaseDetails>, String> {
    let release_cache_dir = cache_dir.join("releases");
    let cache_file = release_cache_dir.join(format!("{}.json", mbid));

    if !cache_file.exists() {
        return Ok(None); // Cache miss
    }

    // REQ-CACHE-090: Handle cache file read failures
    let contents = std::fs::read_to_string(&cache_file)
        .map_err(|e| format!("Cache file read failure: {}", e))?;

    // REQ-CACHE-090: Handle corrupted cache (parse failure)
    let cached: CachedRelease = serde_json::from_str(&contents)
        .map_err(|e| format!("Cache file parse failure (corrupted): {}", e))?;

    Ok(Some(cached.details))
}

/// Store release details in cache
///
/// Writes MusicBrainz release details to disk cache with metadata (MBID, timestamp).
/// Creates cache directory structure if it doesn't exist.
///
/// # Arguments
/// * `cache_dir` - Root cache directory
/// * `mbid` - MusicBrainz Release ID
/// * `details` - Release details to cache (includes media, tracks, etc.)
///
/// # Returns
/// - `Ok(())` - Cache write successful
/// - `Err(message)` - Directory creation, serialization, or write failure
///
/// # File Format
/// Pretty-printed JSON with structure:
/// ```json
/// {
///   "mbid": "550e8400-e29b-41d4-a716-446655440000",
///   "timestamp": "2025-01-15T10:30:45Z",
///   "details": { ... }
/// }
/// ```
///
/// # Requirements
/// REQ-CACHE-030: Release Details Caching
/// REQ-CACHE-050: Cache directory creation
/// REQ-CACHE-130: Human-readable JSON format
pub(crate) fn store_release_cache(
    cache_dir: &Path,
    mbid: &str,
    details: &MBReleaseDetails,
) -> Result<(), String> {
    let release_cache_dir = cache_dir.join("releases");

    // REQ-CACHE-050: Ensure cache directory exists
    std::fs::create_dir_all(&release_cache_dir)
        .map_err(|e| format!("Cache directory creation failure: {}", e))?;

    let cache_file = release_cache_dir.join(format!("{}.json", mbid));

    let cached = CachedRelease {
        mbid: mbid.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        details: details.clone(),
    };

    // REQ-CACHE-130: Human-readable JSON format (pretty-print)
    let json = serde_json::to_string_pretty(&cached)
        .map_err(|e| format!("Cache serialization failure: {}", e))?;

    // REQ-CACHE-090: Handle cache file write failures
    std::fs::write(&cache_file, json).map_err(|e| format!("Cache file write failure: {}", e))?;

    Ok(())
}

// =============================================================================
// Cache Metadata Management
// =============================================================================

/// Update cache metadata file
///
/// Writes or updates metadata.json with cache statistics and version info.
/// Per HIGH-001 resolution, metadata updates occur on program exit only.
///
/// # Arguments
/// * `cache_dir` - Root cache directory
/// * `search_count` - Total number of search cache entries
/// * `release_count` - Total number of release cache entries
///
/// # Returns
/// - `Ok(())` - Metadata updated successfully
/// - `Err(message)` - Directory creation, serialization, or write failure
///
/// # File Format
/// ```json
/// {
///   "version": "album_matcher_26",
///   "created": "2025-01-15T10:30:45Z",
///   "search_count": 42,
///   "release_count": 18,
///   "last_updated": "2025-01-15T11:45:30Z"
/// }
/// ```
///
/// # Requirements
/// REQ-CACHE-050: Cache directory creation
/// REQ-CACHE-080: Cache Statistics Report
pub(crate) fn update_metadata(
    cache_dir: &Path,
    search_count: usize,
    release_count: usize,
) -> Result<(), String> {
    // REQ-CACHE-050: Ensure cache directory exists
    std::fs::create_dir_all(cache_dir)
        .map_err(|e| format!("Cache directory creation failure: {}", e))?;

    let metadata_file = cache_dir.join("metadata.json");
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // Load existing metadata or create new
    let metadata = if metadata_file.exists() {
        let contents = std::fs::read_to_string(&metadata_file)
            .map_err(|e| format!("Metadata file read failure: {}", e))?;
        let mut meta: CacheMetadata =
            serde_json::from_str(&contents).unwrap_or_else(|_| CacheMetadata {
                version: "album_matcher_26".to_string(),
                created: now.clone(),
                search_count: 0,
                release_count: 0,
                last_updated: now.clone(),
            });
        // Update counts and timestamp
        meta.search_count = search_count;
        meta.release_count = release_count;
        meta.last_updated = now;
        meta
    } else {
        CacheMetadata {
            version: "album_matcher_26".to_string(),
            created: now.clone(),
            search_count,
            release_count,
            last_updated: now,
        }
    };

    // Write metadata (pretty-print for human readability)
    let json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Metadata serialization failure: {}", e))?;

    std::fs::write(&metadata_file, json)
        .map_err(|e| format!("Metadata file write failure: {}", e))?;

    Ok(())
}

// =============================================================================
// Cache Statistics Reporting
// =============================================================================

/// Print cache statistics report at end of run
///
/// Displays cache mode, hit/miss counts, hit rate percentage, and cache location.
/// Also updates metadata.json with final counts if in ReadWrite mode.
///
/// # Arguments
/// * `config` - Cache configuration (mode, directory path)
/// * `stats` - Cache statistics (atomic counters for hits/misses)
///
/// # Output Format
/// ```text
/// === Cache Statistics ===
/// Cache Mode: ReadWrite (build cache, use cache on hits)
/// Search queries: 42 hits, 18 misses, 60 total
/// Release details: 15 hits, 3 misses, 18 total
/// Cache hit rate: 75.0% (57/78)
/// Cache location: ./cache
/// ========================
/// ```
///
/// # Requirements
/// REQ-CACHE-080: Cache Statistics Report
pub(crate) fn print_cache_statistics(config: &CacheConfig, stats: &CacheStats) {
    println!("\n=== Cache Statistics ===");

    let mode_str = match config.mode {
        CacheMode::Disabled => "Disabled (no caching)",
        CacheMode::ReadWrite => "ReadWrite (build cache, use cache on hits)",
        CacheMode::ReadOnly => "ReadOnly (cache only, error on miss)",
    };
    println!("Cache Mode: {}", mode_str);

    let search_hits = stats.search_hits.load(Ordering::Relaxed);
    let search_misses = stats.search_misses.load(Ordering::Relaxed);
    let release_hits = stats.release_hits.load(Ordering::Relaxed);
    let release_misses = stats.release_misses.load(Ordering::Relaxed);

    let search_total = search_hits + search_misses;
    let release_total = release_hits + release_misses;

    println!(
        "Search queries: {} hits, {} misses, {} total",
        search_hits, search_misses, search_total
    );
    println!(
        "Release details: {} hits, {} misses, {} total",
        release_hits, release_misses, release_total
    );

    let total_hits = search_hits + release_hits;
    let total_requests = search_total + release_total;

    if total_requests > 0 {
        let hit_rate = (total_hits as f64 / total_requests as f64) * 100.0;
        println!(
            "Cache hit rate: {:.1}% ({}/{})",
            hit_rate, total_hits, total_requests
        );
    } else {
        println!("Cache hit rate: N/A (no requests)");
    }

    println!("Cache location: {}", config.cache_dir.display());

    // Update metadata file with final counts
    if matches!(config.mode, CacheMode::ReadWrite) {
        if let Err(e) = update_metadata(
            &config.cache_dir,
            search_total as usize,
            release_total as usize,
        ) {
            eprintln!("WARNING: Failed to update cache metadata: {}", e);
        }
    }

    println!("========================\n");
}

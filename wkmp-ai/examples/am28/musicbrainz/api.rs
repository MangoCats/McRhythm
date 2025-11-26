//! # MusicBrainz API Client
//!
//! HTTP client with rate limiting (2-second interval between requests).
//!
//! ## Features
//! - **Rate limiting**: Enforces 1.55s delay between requests (safety margin for MB's 1 req/sec limit)
//! - **Exponential backoff**: Automatic retry with backoff (0s, 5s, 15s, 45s, 60s delays)
//! - **Transparent caching**: Integrates with cache module for 3-mode operation
//! - **Query statistics**: Tracks API calls, retries, rate limit waits
//!
//! ## Cache Integration
//! MBClient transparently integrates with the cache system (PLAN026):
//! - **Disabled mode**: Direct API calls, no caching
//! - **ReadWrite mode**: Check cache → API on miss → Store result
//! - **ReadOnly mode**: Cache-only, error on miss (for testing)
//!
//! ## Rate Limiting Strategy
//! Uses tokio::sync::Mutex to serialize API requests:
//! 1. Lock mutex
//! 2. Check elapsed time since last request
//! 3. Sleep if needed to enforce minimum interval
//! 4. Update timestamp
//! 5. Release lock
//!
//! This ensures only one task can be checking/waiting/updating at a time.
//!
//! ## Related Modules
//! - `types`: MBClient, RateLimiter, QueryStats, MBSearchResponse, MBReleaseDetails
//! - `cache`: hash_query, load_cached_search, store_search_cache, load_cached_release, store_release_cache
//! - `constants`: MB_RATE_LIMIT_MS, MB_REQUEST_TIMEOUT_SECS, MB_RETRY_DELAYS_SECS

use crate::constants::*;
use crate::musicbrainz::cache::*;
use crate::types::*;
use crate::utils::query_stats::QueryStats;
use crate::matching::validation::{best_levenshtein_ratio, calculate_name_distance};
use crate::matching::edition::cmp_f64;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

// =============================================================================
// Rate Limiter
// =============================================================================

/// Rate limiter for MusicBrainz API compliance.
/// Enforces minimum delay between requests to avoid being blocked.
///
/// Uses tokio::sync::Mutex to ensure atomic check-and-wait across concurrent tasks.
/// The lock is held across the await point to serialize all API requests.
#[derive(Debug, Clone)]
pub(crate) struct RateLimiter {
    /// Timestamp of last API request, protected by async mutex for serialization.
    last_request: Arc<tokio::sync::Mutex<std::time::Instant>>,
    /// Global query counter for debugging (atomic)
    query_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl RateLimiter {
    pub(crate) fn new() -> Self {
        Self {
            last_request: Arc::new(tokio::sync::Mutex::new(
                std::time::Instant::now() - Duration::from_millis(MB_RATE_LIMIT_MS)
            )),
            query_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Wait if needed to enforce rate limit, with optional statistics tracking
    ///
    /// Holds the mutex lock across the entire wait operation to serialize requests.
    /// This ensures only one task can be checking/waiting/updating at a time.
    ///
    /// # Arguments
    /// * `stats` - Optional QueryStats to track rate limit waits
    ///
    /// # Algorithm
    /// 1. Acquire mutex lock (serializes all API requests)
    /// 2. Calculate elapsed time since last request
    /// 3. If elapsed < MB_RATE_LIMIT_MS (1550ms), sleep for remainder
    /// 4. Update last_request timestamp
    /// 5. Release lock (allows next request to proceed)
    pub(crate) async fn wait_with_stats(&self, stats: Option<&QueryStats>) {
        // Hold the lock across the entire wait operation to serialize requests.
        // This ensures only one task can be checking/waiting/updating at a time.
        let mut last = self.last_request.lock().await;

        let query_num = self.query_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let elapsed = last.elapsed();
        let elapsed_ms = elapsed.as_millis();

        // MusicBrainz API limit: 1 req/sec
        // Use 1.55s delay for safety margin to prevent 503 errors
        if elapsed < Duration::from_millis(MB_RATE_LIMIT_MS) {
            let wait_time = Duration::from_millis(MB_RATE_LIMIT_MS) - elapsed;
            let wait_ms = wait_time.as_millis();
            debug!("[RATE_LIMITER] Query #{}: elapsed={}ms < limit={}ms, WAITING {}ms",
                   query_num, elapsed_ms, MB_RATE_LIMIT_MS, wait_ms);
            if let Some(s) = stats {
                s.record_rate_wait();
            }
            sleep(wait_time).await;
            debug!("[RATE_LIMITER] Query #{}: wait complete, proceeding", query_num);
        } else {
            debug!("[RATE_LIMITER] Query #{}: elapsed={}ms >= limit={}ms, NO WAIT",
                   query_num, elapsed_ms, MB_RATE_LIMIT_MS);
        }

        let before_update = std::time::Instant::now();
        *last = before_update;
        debug!("[RATE_LIMITER] Query #{}: timestamp updated, releasing lock", query_num);
    }
}

// =============================================================================
// Retry with Exponential Backoff
// =============================================================================

/// Retry a network operation with exponential backoff, tracking statistics.
/// Attempts: immediate, +5s, +15s, +45s, +60s (then gives up)
///
/// # Arguments
/// * `log_prefix` - Prefix for log messages (e.g., "[A42]" for album 42)
/// * `stats` - Optional QueryStats to track retries and outcomes
/// * `operation` - Async closure that performs the network operation
///
/// # Returns
/// - `Ok(result)` - Operation succeeded (possibly after retries)
/// - `Err(error)` - Operation failed after all retry attempts
///
/// # Retry Schedule
/// ```text
/// Attempt 1: immediate (delay 0s)
/// Attempt 2: after 5s  (delay 5s)
/// Attempt 3: after 15s (delay 15s)
/// Attempt 4: after 45s (delay 45s)
/// Attempt 5: after 60s (delay 60s)
/// Give up after attempt 5
/// ```
///
/// # Statistics Tracking
/// - `record_retry()` - Called before each retry sleep
/// - `record_query_start()` - Called before each attempt
/// - `record_success()` - Called on successful completion
/// - `record_failure()` - Called after final failure
pub(crate) async fn retry_with_backoff_stats<F, Fut, T, E>(
    log_prefix: &str,
    stats: Option<&QueryStats>,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let max_attempts = MB_RETRY_DELAYS_SECS.len();

    for (attempt, &delay) in MB_RETRY_DELAYS_SECS.iter().enumerate() {
        if delay > 0 {
            if let Some(s) = stats {
                s.record_retry();
            }
            warn!("{}    Retrying after {} seconds (attempt {}/{})...", log_prefix, delay, attempt + 1, max_attempts);
            sleep(Duration::from_secs(delay)).await;
        }

        if let Some(s) = stats {
            s.record_query_start();
        }

        match operation().await {
            Ok(result) => {
                if let Some(s) = stats {
                    s.record_success();
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt < max_attempts - 1 {
                    warn!("{}    Network error: {} - will retry", log_prefix, e);
                } else {
                    if let Some(s) = stats {
                        s.record_failure();
                    }
                    error!("{}    Network error: {} - giving up after {} attempts", log_prefix, e, max_attempts);
                    return Err(e);
                }
            }
        }
    }

    unreachable!()
}

// =============================================================================
// MBClient: MusicBrainz API Wrapper with Caching (PLAN026)
// =============================================================================

/// MusicBrainz API client with transparent caching
///
/// Integrates rate limiting, retry logic, and transparent caching for all
/// MusicBrainz API operations. Supports 3 cache modes (Disabled, ReadWrite, ReadOnly).
///
/// # Cache Modes
/// - **Disabled**: Direct API calls, no caching
/// - **ReadWrite**: Build cache on misses, use cache on hits
/// - **ReadOnly**: Cache-only operation, error on cache miss
///
/// # Requirements Coverage
/// REQ-CACHE-040: Transparent API wrapper
/// REQ-CACHE-110: MBClient constructor and methods
pub(crate) struct MBClient {
    http_client: reqwest::Client,
    rate_limiter: Arc<RateLimiter>,
    config: CacheConfig,
    stats: Arc<CacheStats>,
}

impl MBClient {
    /// Create new MBClient with caching support
    ///
    /// Creates HTTP client with timeout and initializes cache directory structure
    /// if operating in ReadWrite or ReadOnly mode.
    ///
    /// # Arguments
    /// * `config` - Cache configuration (mode, directory path)
    /// * `rate_limiter` - Shared rate limiter instance
    ///
    /// # Returns
    /// - `Ok(client)` - MBClient ready for use
    /// - `Err(message)` - HTTP client creation or cache directory creation failure
    ///
    /// # Requirements
    /// REQ-CACHE-110: MBClient constructor
    pub(crate) fn new(config: CacheConfig, rate_limiter: Arc<RateLimiter>) -> Result<Self, String> {
        // Create cache directory structure if ReadWrite or ReadOnly mode
        match config.mode {
            CacheMode::ReadWrite | CacheMode::ReadOnly => {
                std::fs::create_dir_all(config.cache_dir.join("searches"))
                    .map_err(|e| format!("Cache directory creation failure: {}", e))?;
                std::fs::create_dir_all(config.cache_dir.join("releases"))
                    .map_err(|e| format!("Cache directory creation failure: {}", e))?;
            }
            CacheMode::Disabled => {
                // No cache directory needed
            }
        }

        // Build HTTP client with timeout
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(MB_REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| format!("HTTP client creation failure: {}", e))?;

        Ok(Self {
            http_client,
            rate_limiter,
            config,
            stats: Arc::new(CacheStats::new()),
        })
    }

    /// Search for releases with transparent caching
    ///
    /// Searches MusicBrainz for releases matching the query. Transparently checks cache
    /// first (if enabled), and stores results in cache after successful API queries.
    ///
    /// # Arguments
    /// * `query` - Lucene query string (e.g., "artist:Pink Floyd AND release:The Wall")
    /// * `album_idx` - Optional album index for logging (0-based, displayed as 1-based)
    ///
    /// # Returns
    /// - `Ok(response)` - Search results (from cache or API)
    /// - `Err(error)` - Cache miss in ReadOnly mode, network error, or parse error
    ///
    /// # Cache Behavior
    /// - **Disabled**: Direct API query (no cache check)
    /// - **ReadWrite**: Check cache → API on miss → Store result
    /// - **ReadOnly**: Check cache → Error on miss (no API query)
    ///
    /// # Error Handling
    /// - REQ-CACHE-090: Cache corruption falls back to API (ReadWrite mode)
    /// - REQ-CACHE-090: Cache miss/corruption returns error (ReadOnly mode)
    ///
    /// # Requirements
    /// REQ-CACHE-020: Search query caching
    /// REQ-CACHE-040: Transparent caching
    /// REQ-CACHE-070: Cache hit/miss logging
    pub(crate) async fn search_releases(
        &self,
        query: &str,
        album_idx: Option<usize>,
    ) -> Result<MBSearchResponse, Box<dyn std::error::Error>> {
        let cache_key = hash_query(query);
        let prefix = album_idx.map(|idx| format!("[A{}] ", idx + 1)).unwrap_or_default();

        // Try cache first (unless Disabled mode)
        match self.config.mode {
            CacheMode::Disabled => {
                // Skip cache entirely
            }
            CacheMode::ReadWrite | CacheMode::ReadOnly => {
                match load_cached_search(&self.config.cache_dir, &cache_key) {
                    Ok(Some(response)) => {
                        // REQ-CACHE-070: Log cache hit
                        info!("{}Cache hit: search query [{}]", prefix, &cache_key);
                        self.stats.record_search_hit();
                        return Ok(response);
                    }
                    Ok(None) => {
                        // Cache miss
                        self.stats.record_search_miss();

                        // REQ-CACHE-090: ReadOnly mode errors on cache miss
                        if matches!(self.config.mode, CacheMode::ReadOnly) {
                            return Err(format!(
                                "Cache miss in read-only mode for query: {}",
                                query
                            )
                            .into());
                        }

                        // REQ-CACHE-070: Log cache miss
                        info!("{}Cache miss: search query [{}]", prefix, &cache_key);
                    }
                    Err(e) => {
                        // REQ-CACHE-090: Cache corruption detected
                        eprintln!("{}WARNING: Cache corruption detected: {}", prefix, e);
                        println!("{}Falling back to live API query", prefix);
                        self.stats.record_search_miss();

                        if matches!(self.config.mode, CacheMode::ReadOnly) {
                            return Err(format!(
                                "Cache corruption in read-only mode: {}",
                                e
                            )
                            .into());
                        }
                    }
                }
            }
        }

        // Query live API (Disabled mode or ReadWrite cache miss)
        // REQ-CACHE-040: Respect rate limiting
        self.rate_limiter.wait_with_stats(None).await;

        let url = format!(
            "https://musicbrainz.org/ws/2/release/?query={}&fmt=json&limit=100",
            urlencoding::encode(query)
        );

        let response = self
            .http_client
            .get(&url)
            .header("User-Agent", "WKMP-Album-Matcher/1.0")
            .send()
            .await?;

        let mb_response: MBSearchResponse = response.json().await?;

        // Store in cache if ReadWrite mode
        if matches!(self.config.mode, CacheMode::ReadWrite) {
            if let Err(e) = store_search_cache(
                &self.config.cache_dir,
                &cache_key,
                query,
                &mb_response,
            ) {
                // REQ-CACHE-090: Log cache write failure but continue
                eprintln!("{}WARNING: Cache write failure: {}", prefix, e);
                println!("{}Continuing without caching this query", prefix);
            }
        }

        Ok(mb_response)
    }

    /// Get release details with transparent caching
    ///
    /// Fetches detailed release information from MusicBrainz (recordings, track durations,
    /// artist credits). Transparently checks cache first (if enabled), and stores results
    /// after successful API queries.
    ///
    /// # Arguments
    /// * `mbid` - MusicBrainz Release ID (UUID format)
    /// * `album_idx` - Optional album index for logging
    ///
    /// # Returns
    /// - `Ok(details)` - Release details (from cache or API)
    /// - `Err(error)` - Cache miss in ReadOnly mode, network error, or parse error
    ///
    /// # Cache Behavior
    /// - **Disabled**: Direct API query (no cache check)
    /// - **ReadWrite**: Check cache → API on miss → Store result
    /// - **ReadOnly**: Check cache → Error on miss (no API query)
    ///
    /// # Requirements
    /// REQ-CACHE-030: Release details caching
    /// REQ-CACHE-040: Transparent caching
    /// REQ-CACHE-070: Cache hit/miss logging
    pub(crate) async fn get_release_details(
        &self,
        mbid: &str,
        album_idx: Option<usize>,
    ) -> Result<MBReleaseDetails, Box<dyn std::error::Error>> {
        let prefix = album_idx.map(|idx| format!("[A{}] ", idx + 1)).unwrap_or_default();

        // Try cache first (unless Disabled mode)
        match self.config.mode {
            CacheMode::Disabled => {
                // Skip cache entirely
            }
            CacheMode::ReadWrite | CacheMode::ReadOnly => {
                match load_cached_release(&self.config.cache_dir, mbid) {
                    Ok(Some(details)) => {
                        // REQ-CACHE-070: Log cache hit
                        info!("{}Cache hit: release {}", prefix, mbid);
                        self.stats.record_release_hit();
                        return Ok(details);
                    }
                    Ok(None) => {
                        // Cache miss
                        self.stats.record_release_miss();

                        // REQ-CACHE-090: ReadOnly mode errors on cache miss
                        if matches!(self.config.mode, CacheMode::ReadOnly) {
                            return Err(format!(
                                "Cache miss in read-only mode for release: {}",
                                mbid
                            )
                            .into());
                        }

                        // REQ-CACHE-070: Log cache miss
                        info!("{}Cache miss: release {}", prefix, mbid);
                    }
                    Err(e) => {
                        // REQ-CACHE-090: Cache corruption detected
                        eprintln!("{}WARNING: Cache corruption detected: {}", prefix, e);
                        println!("{}Falling back to live API query", prefix);
                        self.stats.record_release_miss();

                        if matches!(self.config.mode, CacheMode::ReadOnly) {
                            return Err(format!(
                                "Cache corruption in read-only mode: {}",
                                e
                            )
                            .into());
                        }
                    }
                }
            }
        }

        // Query live API (Disabled mode or ReadWrite cache miss)
        // REQ-CACHE-040: Respect rate limiting
        self.rate_limiter.wait_with_stats(None).await;

        let url = format!(
            "https://musicbrainz.org/ws/2/release/{}?inc=recordings+artist-credits&fmt=json",
            mbid
        );

        let response = self
            .http_client
            .get(&url)
            .header("User-Agent", "WKMP-Album-Matcher/1.0")
            .send()
            .await?;

        let details: MBReleaseDetails = response.json().await?;

        // Store in cache if ReadWrite mode
        if matches!(self.config.mode, CacheMode::ReadWrite) {
            if let Err(e) = store_release_cache(&self.config.cache_dir, mbid, &details)
            {
                // REQ-CACHE-090: Log cache write failure but continue
                eprintln!("{}WARNING: Cache write failure: {}", prefix, e);
                println!("{}Continuing without caching this release", prefix);
            }
        }

        Ok(details)
    }

    /// Get cache statistics
    ///
    /// Returns Arc-wrapped CacheStats for inspection (hit/miss counts).
    ///
    /// # Requirements
    /// REQ-CACHE-080: Cache statistics reporting
    pub(crate) fn get_stats(&self) -> Arc<CacheStats> {
        Arc::clone(&self.stats)
    }

    /// Get cache configuration
    ///
    /// Returns reference to cache configuration (mode, directory path).
    pub(crate) fn get_config(&self) -> &CacheConfig {
        &self.config
    }
}

// =============================================================================
// MusicBrainz Search Helper Functions
// =============================================================================

/// Split CamelCase strings into space-separated words
///
/// Inserts spaces before uppercase letters that follow lowercase letters.
/// Used to handle album names like "NativeAmericanFluteLullabies" → "Native American Flute Lullabies"
///
/// # Arguments
/// * `s` - Input string (potentially CamelCase)
///
/// # Returns
/// Space-separated string
///
/// # Example
/// ```ignore
/// assert_eq!(split_camel_case("NativeAmericanFlute"), "Native American Flute");
/// assert_eq!(split_camel_case("Lizzy"), "Lizzy"); // No change for non-CamelCase
/// ```
fn split_camel_case(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_uppercase() && chars[i-1].is_lowercase() {
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

/// Apply wildcard to handle common misspellings
///
/// Transforms specific known misspelling patterns into wildcard queries.
/// Currently handles "Lizzie" vs "Lizzy" (Thin Lizzie → Thin Lizz*)
///
/// # Arguments
/// * `text` - Input text to check for misspelling patterns
///
/// # Returns
/// - `Some(fixed_text)` - Text with wildcard applied if pattern detected
/// - `None` - No known misspelling pattern found
///
/// # Example
/// ```ignore
/// assert_eq!(apply_wildcard_fixes("Thin Lizzie"), Some("Thin Lizz*".to_string()));
/// assert_eq!(apply_wildcard_fixes("Pink Floyd"), None);
/// ```
fn apply_wildcard_fixes(text: &str) -> Option<String> {
    // Handle "Lizzie" vs "Lizzy" (Thin Lizzie -> Thin Lizz*)
    // Use * instead of ? to match any number of characters (Lizzy, Lizzie, Lizzies, etc.)
    if text.contains("Lizzie") {
        return Some(text.replace("Lizzie", "Lizz*"));
    }
    None
}

/// Generate search query variants to try in sequence
///
/// Creates 7 search strategies from most precise to most fuzzy:
/// 1. Original query with type:album filter
/// 2. CamelCase split (e.g., "NativeAmericanFlute" → "Native American Flute")
/// 3. Fuzzy matching (~1 edit distance)
/// 4. Targeted wildcards for known misspellings
/// 5. Aggressive fuzzy search (~2 edits)
/// 6. Per-token fuzzy matching
/// 7. Album-only fallback (ignores artist)
///
/// # Arguments
/// * `artist` - Artist name from ID3 tags
/// * `album` - Album name from ID3 tags
///
/// # Returns
/// Vec of MusicBrainz Lucene query strings to try in order
///
/// # Example
/// ```ignore
/// let queries = generate_search_queries("Pink Floyd", "The Wall");
/// // Returns ~7 variants from precise to fuzzy
/// ```
fn generate_search_queries(artist: &str, album: &str) -> Vec<String> {
    let mut queries = Vec::new();

    // Strategy 1: Original query with type:album filter
    queries.push(format!("type:album AND artist:{} AND release:{}", artist, album));

    // Strategy 2: CamelCase split (most effective per test results)
    let album_spaced = split_camel_case(album);
    if album_spaced != album {
        queries.push(format!("type:album AND artist:{} AND release:\"{}\"", artist, album_spaced));
    }

    // Strategy 3: Fuzzy matching (catches punctuation differences like "Funk49" -> "Funk #49")
    queries.push(format!("type:album AND artist:{}~ AND release:{}~", artist, album));

    // Strategy 4: Targeted wildcard for common misspellings (e.g., "Lizzie" -> "Lizz*")
    if let Some(artist_wildcard) = apply_wildcard_fixes(artist) {
        let album_variant = apply_wildcard_fixes(album).unwrap_or_else(|| album.to_string());
        queries.push(format!("type:album AND artist:{} AND release:{}", artist_wildcard, album_variant));
    } else if let Some(album_wildcard) = apply_wildcard_fixes(album) {
        queries.push(format!("type:album AND artist:{} AND release:{}", artist, album_wildcard));
    }

    // Strategy 5: Aggressive fuzzy search (~2 edits - more tolerant, catches more misspellings)
    queries.push(format!("type:album AND artist:{}~2 AND release:{}~2", artist, album));

    // Strategy 6: Per-token fuzzy matching (handles multi-word names better)
    let artist_tokens: Vec<&str> = artist.split_whitespace().collect();
    let album_tokens: Vec<&str> = album.split_whitespace().collect();
    if artist_tokens.len() > 1 || album_tokens.len() > 1 {
        let artist_fuzzy = artist_tokens.iter().map(|t| format!("{}~", t)).collect::<Vec<_>>().join(" ");
        let album_fuzzy = album_tokens.iter().map(|t| format!("{}~", t)).collect::<Vec<_>>().join(" ");
        queries.push(format!("type:album AND artist:({}) AND release:({})", artist_fuzzy, album_fuzzy));
    }

    // Strategy 7: Album-only fallback (last resort when artist name is problematic)
    queries.push(format!("type:album AND release:{}", album));

    queries
}

// =============================================================================
// Comprehensive MusicBrainz Search
// =============================================================================

/// Search using ALL artist/album/strategy combinations
///
/// Executes all search strategies for all artist/album name variants, deduplicates
/// results by MBID, and enforces maximum release limit.
///
/// # Arguments
/// * `mb_client` - MBClient with caching and rate limiting
/// * `artist_variants` - Artist name variants (e.g., ["Jessita Reyes", "Various"])
/// * `album_variants` - Album name variants (e.g., ["Native American Flute Lullabies", "NativeAmericanFluteLullabies"])
/// * `album_idx` - Album index for logging (0-based)
/// * `stats` - Optional QueryStats for heartbeat logging
///
/// # Returns
/// Vec of unique MBRelease objects (deduplicated by MBID, max MB_MAX_RELEASES)
///
/// # Algorithm
/// 1. For each artist variant:
///    - For each album variant:
///      - Generate search strategies (7 variants)
///      - Execute each strategy via MBClient
///      - Deduplicate by MBID
///      - Stop at MB_MAX_RELEASES limit
///
/// # Example
/// ```ignore
/// let releases = search_all_mb_strategies(
///     &mb_client,
///     &["Pink Floyd"],
///     &["The Wall", "TheWall"],
///     0,
///     Some(&stats)
/// ).await;
/// ```
async fn search_all_mb_strategies(
    mb_client: &MBClient,
    artist_variants: &[String],
    album_variants: &[String],
    album_idx: usize,
    stats: Option<&QueryStats>,
) -> Vec<MBRelease> {
    let mut all_releases: Vec<MBRelease> = Vec::new();
    let mut seen_mbids = std::collections::HashSet::new();
    let log_prefix = format!("[A{}]", album_idx + 1);

    for artist in artist_variants {
        for album in album_variants {
            let search_queries = generate_search_queries(artist, album);

            for (i, query) in search_queries.iter().enumerate() {
                if all_releases.len() >= MB_MAX_RELEASES {
                    info!("[A{}]   Reached {} release limit", album_idx + 1, MB_MAX_RELEASES);
                    break;
                }

                if let Some(s) = stats {
                    s.set_activity(&format!("searching: {} / {} (strategy {}/{})",
                        artist, album, i + 1, search_queries.len()));
                }

                debug!("{} MB query: {}", log_prefix, query);

                // REQ-CACHE-120: Use MBClient with transparent caching
                // Note: MBClient handles rate limiting and retries internally
                let response = retry_with_backoff_stats(&log_prefix, stats, || async {
                    mb_client
                        .search_releases(query, Some(album_idx))
                        .await
                        .map_err(|e| format!("error querying MusicBrainz: {}", e))
                }).await;

                let response = match response {
                    Ok(r) => r,
                    Err(e) => {
                        info!("  Strategy {}/{} for '{}' / '{}' FAILED: {}",
                                 i + 1, search_queries.len(), artist, album, e);
                        continue;
                    }
                };

                if response.releases.is_empty() {
                    continue;
                }

                // Add new releases (deduplicate by MBID)
                for release in response.releases {
                    if seen_mbids.insert(release.id.clone()) {
                        all_releases.push(release);

                        if all_releases.len() >= MB_MAX_RELEASES {
                            break;
                        }
                    }
                }
            }

            if all_releases.len() >= MB_MAX_RELEASES {
                break;
            }
        }

        if all_releases.len() >= MB_MAX_RELEASES {
            break;
        }
    }

    all_releases
}

/// Calculate Name Distance Rank (NDR) for releases and filter by ratio and rank thresholds.
///
/// Run 25c: Applies a two-stage filter with Combination A strategy:
/// 1. Pre-NDR Combination A filter:
///    - For "Various Artists": requires album ratio >= MIN_VARIOUS_ALBUM_RATIO (35%)
///    - For regular albums: requires weighted combined score >= MIN_COMBINED_RATIO (42%)
///      where combined = artist_ratio * ARTIST_WEIGHT + album_ratio * ALBUM_WEIGHT
/// 2. NDR rank filter: Ranks remaining releases by name distance score, then filters
///    out releases with rank > MAX_NAME_DISTANCE_RANK.
///
/// # Returns
/// Filtered releases with their rank (1-N) and NDR score.
fn calculate_ndr_and_filter<'a>(
    releases: &'a [MBRelease],
    artist_variants: &[String],
    album_variants: &[String],
    album_idx: usize,
) -> Vec<(&'a MBRelease, usize, f64)> {
    let initial_count = releases.len();

    // Helper function to check if artist is "Various Artists"
    let is_various_artist = |artist: &str| -> bool {
        artist.eq_ignore_ascii_case("Various") ||
        artist.eq_ignore_ascii_case("Various Artists") ||
        artist.starts_with("Various")
    };

    // === Run 25c: Stage 1 - Combination A Filter ===
    // Special handling for "Various Artists" + weighted combined score for regular albums
    let ratio_filtered: Vec<&MBRelease> = releases
        .iter()
        .filter(|release| {
            let artist = release.artist_credit
                .as_ref()
                .and_then(|credits| credits.first())
                .and_then(|credit| credit.artist.as_ref())
                .map(|artist| artist.name.as_str())
                .unwrap_or("Unknown Artist");

            let artist_ratio = best_levenshtein_ratio(artist, artist_variants);
            let album_ratio = best_levenshtein_ratio(&release.title, album_variants);

            // Combination A logic
            if is_various_artist(artist) {
                // For compilations: only check album similarity
                album_ratio >= MIN_VARIOUS_ALBUM_RATIO
            } else {
                // For regular albums: weighted combined score
                let combined_score = artist_ratio * ARTIST_WEIGHT + album_ratio * ALBUM_WEIGHT;
                combined_score >= MIN_COMBINED_RATIO
            }
        })
        .collect();

    let ratio_filtered_count = initial_count - ratio_filtered.len();
    if ratio_filtered_count > 0 {
        info!("[A{}]   Run 25c Combination A filter: skipping {} releases (weighted<{:.0}% OR various/album<{:.0}%), {} remain",
              album_idx + 1, ratio_filtered_count,
              MIN_COMBINED_RATIO * 100.0, MIN_VARIOUS_ALBUM_RATIO * 100.0,
              ratio_filtered.len());
    }

    // === Stage 2 - Calculate NDR scores on remaining releases ===
    let mut releases_with_ndr: Vec<(&MBRelease, f64)> = ratio_filtered
        .into_iter()
        .map(|release| {
            let artist = release.artist_credit
                .as_ref()
                .and_then(|credits| credits.first())
                .and_then(|credit| credit.artist.as_ref())
                .map(|artist| artist.name.as_str())
                .unwrap_or("Unknown Artist");
            let score = calculate_name_distance(artist, &release.title, artist_variants, album_variants);
            (release, score)
        })
        .collect();

    // Sort by NDR score (ascending - lower is better)
    releases_with_ndr.sort_by(|a, b| cmp_f64(a.1, b.1));

    // === Stage 3 - Filter by NDR rank ===
    let pre_rank_filter_count = releases_with_ndr.len();
    let filtered_releases: Vec<(&MBRelease, usize, f64)> = releases_with_ndr
        .into_iter()
        .enumerate()
        .map(|(idx, (release, score))| (release, idx + 1, score)) // Assign rank 1-N
        .filter(|(_, rank, _)| *rank <= MAX_NAME_DISTANCE_RANK)
        .collect();

    let ndr_filtered_count = pre_rank_filter_count - filtered_releases.len();
    if ndr_filtered_count > 0 {
        info!("[A{}]   Early NDR filter: skipping {} releases (NDR > {}), fetching details for {}",
                 album_idx + 1, ndr_filtered_count, MAX_NAME_DISTANCE_RANK, filtered_releases.len());
    }

    filtered_releases
}

/// Fetch track details for a single release from MusicBrainz.
///
/// Retrieves media/tracks for a release and extracts durations, artist name,
/// and media format (CD detection).
///
/// # Returns
/// `Some((durations, recording_mbids, mbid_info, artist, album, rank, score))` if successful, `None` if failed.
async fn fetch_release_track_details(
    mb_client: &MBClient,
    release: &MBRelease,
    rank: usize,
    score: f64,
    album_idx: usize,
    stats: Option<&QueryStats>,
) -> Option<(Vec<u32>, Vec<String>, EditionMBID, String, String, usize, f64)> {
    if let Some(s) = stats {
        s.set_activity(&format!("fetching details: {} (rank {})", release.title, rank));
    }

    let log_prefix = format!("[A{}]", album_idx + 1);
    debug!("{} MB details: {}", log_prefix, release.id);

    // REQ-CACHE-120: Use MBClient with transparent caching
    // Note: MBClient handles rate limiting internally
    let details = retry_with_backoff_stats(&log_prefix, stats, || async {
        mb_client
            .get_release_details(&release.id, Some(album_idx))
            .await
            .map_err(|e| format!("error fetching details: {}", e))
    }).await;

    let details = match details {
        Ok(d) => d,
        Err(_) => return None,
    };

    // Extract track durations, recording MBIDs, and media format
    let mut durations = Vec::new();
    let mut recording_mbids = Vec::new();
    let mut is_cd = false;
    for medium in &details.media {
        for track in &medium.tracks {
            if let Some(length_ms) = track.length {
                durations.push(length_ms / 1000); // Convert to seconds
                // Extract recording MBID (or empty string if not available)
                let recording_mbid = track.recording
                    .as_ref()
                    .map(|r| r.id.clone())
                    .unwrap_or_default();
                recording_mbids.push(recording_mbid);
            }
        }
        if let Some(ref format) = medium.format {
            if format == "CD" {
                is_cd = true;
            }
        }
    }

    if durations.is_empty() {
        return None;
    }

    // Extract artist name
    let artist = release.artist_credit
        .as_ref()
        .and_then(|credits| credits.first())
        .and_then(|credit| credit.artist.as_ref())
        .map(|artist| artist.name.clone())
        .unwrap_or_else(|| "Unknown Artist".to_string());

    Some((
        durations,
        recording_mbids,
        EditionMBID {
            mbid: release.id.clone(),
            country: release.country.clone(),
            status: release.status.clone(),
            is_cd,
        },
        artist,
        release.title.clone(),
        rank,
        score,
    ))
}

/// Comprehensive MusicBrainz search using ALL strategies and name variants.
///
/// Orchestrates the full search process:
/// 1. Search using all artist/album/strategy combinations
/// 2. Filter by Name Distance Rank (NDR)
/// 3. Fetch track details for filtered releases
///
/// # Returns
/// Vec of (durations, recording_mbids, mbid_info, artist, album, name_distance_rank, name_distance_score)
pub(crate) async fn comprehensive_musicbrainz_search(
    mb_client: &MBClient,
    artist_variants: &[String],  // e.g., ["Jessita Reyes", "Various"]
    album_variants: &[String],   // e.g., ["Native American Flute Lullabies", "NativeAmericanFluteLullabies"]
    album_idx: usize,            // Album index for log messages
    stats: Option<&QueryStats>,  // Optional stats for heartbeat logging
) -> Result<Vec<(Vec<u32>, Vec<String>, EditionMBID, String, String, usize, f64)>, Box<dyn std::error::Error>> {
    info!("[A{}]   Fetching MusicBrainz data (comprehensive search)...", album_idx + 1);

    // Step 1: Search using all artist/album/strategy combinations
    // REQ-CACHE-120: Use MBClient for transparent caching
    let all_releases = search_all_mb_strategies(mb_client, artist_variants, album_variants, album_idx, stats).await;
    info!("[A{}]   Found {} unique releases across all search strategies", album_idx + 1, all_releases.len());

    // Step 2: Calculate NDR and filter releases (Run 15 optimization)
    let filtered_releases = calculate_ndr_and_filter(&all_releases, artist_variants, album_variants, album_idx);

    if let Some(s) = stats {
        s.set_activity(&format!("fetching track details for {} releases", filtered_releases.len()));
    }

    // Step 3: Fetch track details for filtered releases
    // REQ-CACHE-120: Use MBClient for transparent caching
    let mut results: Vec<(Vec<u32>, Vec<String>, EditionMBID, String, String, usize, f64)> = Vec::new();
    for (release, rank, score) in filtered_releases {
        if let Some(result) = fetch_release_track_details(mb_client, release, rank, score, album_idx, stats).await {
            results.push(result);
        }
    }

    if let Some(s) = stats {
        s.set_activity("MusicBrainz search complete");
    }

    Ok(results)
}

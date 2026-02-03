//! MusicBrainz API client
//!
//! **[AIA-INT-010]** MusicBrainz API integration with rate limiting
//!
//! Per [IMPL011](../../docs/IMPL011-musicbrainz_client.md)

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;

const MUSICBRAINZ_BASE_URL: &str = "https://musicbrainz.org/ws/2";
const USER_AGENT: &str = "WKMP/0.1.0 (https://github.com/wkmp/wkmp)";
const RATE_LIMIT_MS: u64 = 1000; // 1 request per second

/// MusicBrainz client errors
#[derive(Debug, Error)]
pub enum MBError {
    /// Network communication error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Recording not found in MusicBrainz database
    #[error("Recording not found: {0}")]
    RecordingNotFound(String),

    /// Rate limit exceeded (1 request/second)
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// MusicBrainz API returned error response
    #[error("API error {0}: {1}")]
    ApiError(u16, String),

    /// Failed to parse API response JSON
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// MusicBrainz Recording response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MBRecording {
    /// Recording MBID (MusicBrainz ID)
    pub id: String,
    /// Recording title
    pub title: String,
    /// Recording length in milliseconds
    pub length: Option<u64>,
    /// Artist credits for this recording
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<MBArtistCredit>,
    /// Releases containing this recording
    pub releases: Option<Vec<MBRelease>>,
    /// Relations to other entities (e.g., works)
    pub relations: Option<Vec<MBRelation>>,
}

/// MusicBrainz artist credit
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MBArtistCredit {
    /// Display name (may differ from artist.name for collaborations)
    pub name: String,
    /// Artist information
    pub artist: MBArtist,
}

/// MusicBrainz artist
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MBArtist {
    /// Artist MBID (MusicBrainz ID)
    pub id: String,
    /// Artist name
    pub name: String,
    /// Artist sort name (for alphabetical sorting)
    #[serde(rename = "sort-name")]
    pub sort_name: String,
}

/// MusicBrainz release
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MBRelease {
    /// Release MBID (MusicBrainz ID)
    pub id: String,
    /// Release title
    pub title: String,
    /// Release date in YYYY-MM-DD format
    pub date: Option<String>,
}

/// MusicBrainz relation to another entity
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MBRelation {
    /// Relation type (e.g., "performance")
    #[serde(rename = "type")]
    pub relation_type: String,
    /// Relation type UUID
    #[serde(rename = "type-id")]
    pub type_id: String,
    /// Related work (if relation is to a work)
    pub work: Option<MBWork>,
}

/// MusicBrainz work (musical composition)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MBWork {
    /// Work MBID (MusicBrainz ID)
    pub id: String,
    /// Work title
    pub title: String,
}

/// MusicBrainz recording search response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MBRecordingSearchResponse {
    /// Number of results returned
    pub count: usize,
    /// List of recording search results
    pub recordings: Vec<MBRecordingSearchResult>,
}

/// MusicBrainz recording search result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MBRecordingSearchResult {
    /// Recording MBID (MusicBrainz ID)
    pub id: String,
    /// Recording title
    pub title: String,
    /// Match score (0-100) from MusicBrainz search
    pub score: u32,
    /// Recording length in milliseconds
    pub length: Option<u64>,
    /// Artist credits for this recording
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<MBArtistCredit>>,
    /// Releases containing this recording
    pub releases: Option<Vec<MBRelease>>,
}

/// MusicBrainz release search response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MBReleaseSearchResponse {
    /// Number of results returned
    pub count: usize,
    /// List of release search results
    pub releases: Vec<MBReleaseSearchResult>,
}

/// MusicBrainz release search result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MBReleaseSearchResult {
    /// Release MBID (MusicBrainz ID)
    pub id: String,
    /// Release title
    pub title: String,
    /// Match score (0-100) from MusicBrainz search
    pub score: u32,
    /// Release date in YYYY-MM-DD format
    pub date: Option<String>,
    /// Artist credits for this release
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<MBArtistCredit>>,
    /// Track count (if available)
    #[serde(rename = "track-count")]
    pub track_count: Option<u32>,
}

/// Rate limiter enforcing 1 request/second
struct RateLimiter {
    last_request: Mutex<Option<Instant>>,
    min_interval: Duration,
}

impl RateLimiter {
    fn new(min_interval_ms: u64) -> Self {
        Self {
            last_request: Mutex::new(None),
            min_interval: Duration::from_millis(min_interval_ms),
        }
    }

    /// Wait if necessary to comply with rate limit
    async fn wait(&self) {
        let mut last = self.last_request.lock().await;

        if let Some(last_time) = *last {
            let elapsed = last_time.elapsed();
            if elapsed < self.min_interval {
                let wait_time = self.min_interval - elapsed;
                tracing::debug!("Rate limiting: waiting {:?}", wait_time);
                tokio::time::sleep(wait_time).await;
            }
        }

        *last = Some(Instant::now());
    }
}

/// MusicBrainz API client
pub struct MusicBrainzClient {
    http_client: reqwest::Client,
    rate_limiter: Arc<RateLimiter>,
}

impl MusicBrainzClient {
    /// Create new MusicBrainz client
    pub fn new() -> Result<Self, MBError> {
        let http_client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| MBError::NetworkError(e.to_string()))?;

        Ok(Self {
            http_client,
            rate_limiter: Arc::new(RateLimiter::new(RATE_LIMIT_MS)),
        })
    }

    /// Execute HTTP request with retry logic for connection errors
    ///
    /// Retries up to 3 times with exponential backoff when connection is closed by server.
    /// - Attempt 1: base throttling period (1000ms)
    /// - Attempt 2: 2x throttling period (2000ms)
    /// - Attempt 3: 4x throttling period (4000ms)
    ///
    /// # Arguments
    /// * `operation` - Description of operation for logging (e.g., "lookup recording abc123")
    /// * `request_fn` - Closure that executes the HTTP request
    async fn execute_with_retry<F, Fut>(
        &self,
        operation: &str,
        mut request_fn: F,
    ) -> Result<reqwest::Response, MBError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    {
        const MAX_RETRIES: u32 = 3;
        let base_wait_ms = RATE_LIMIT_MS;

        for attempt in 1..=MAX_RETRIES {
            match request_fn().await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    let error_msg = e.to_string();
                    let is_connection_closed = error_msg.contains("connection closed before message completed")
                        || error_msg.contains("connection closed")
                        || error_msg.contains("forcibly closed")  // Windows error 10054
                        || error_msg.contains("connection reset")  // ECONNRESET
                        || error_msg.contains("broken pipe")       // EPIPE
                        || error_msg.contains("stream closed");

                    if is_connection_closed && attempt < MAX_RETRIES {
                        // Calculate exponential backoff: 1s, 2s, 4s
                        let wait_ms = base_wait_ms * (1 << (attempt - 1)); // 2^(attempt-1)
                        let wait_duration = Duration::from_millis(wait_ms);

                        tracing::warn!(
                            operation = %operation,
                            attempt = attempt,
                            max_retries = MAX_RETRIES,
                            "Connection closed by MusicBrainz server - waiting {}ms before retry",
                            wait_ms
                        );

                        tokio::time::sleep(wait_duration).await;

                        tracing::info!(
                            operation = %operation,
                            attempt = attempt + 1,
                            max_retries = MAX_RETRIES,
                            "Retrying connection to MusicBrainz (attempt {}/{})",
                            attempt + 1,
                            MAX_RETRIES
                        );
                    } else {
                        // Non-retryable error or max retries exceeded
                        return Err(MBError::NetworkError(error_msg));
                    }
                }
            }
        }

        // Should never reach here due to return in loop, but satisfy type checker
        Err(MBError::NetworkError(
            "Max retries exceeded".to_string(),
        ))
    }

    /// Lookup recording by MBID
    ///
    /// **[AIA-INT-010]** Query MusicBrainz for recording metadata
    pub async fn lookup_recording(&self, mbid: &str) -> Result<MBRecording, MBError> {
        // Rate limit
        self.rate_limiter.wait().await;

        // Query API
        let url = format!(
            "{}/recording/{}?inc=artist-credits+releases+work-rels&fmt=json",
            MUSICBRAINZ_BASE_URL, mbid
        );

        tracing::debug!(mbid = %mbid, url = %url, "Querying MusicBrainz API");

        let operation = format!("lookup recording {}", mbid);
        let response = self
            .execute_with_retry(&operation, || {
                self.http_client.get(&url).send()
            })
            .await?;

        let status = response.status();

        if status == 404 {
            return Err(MBError::RecordingNotFound(mbid.to_string()));
        }

        if status == 503 {
            return Err(MBError::RateLimitExceeded);
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(MBError::ApiError(status.as_u16(), error_text));
        }

        let recording: MBRecording = response
            .json()
            .await
            .map_err(|e| MBError::ParseError(e.to_string()))?;

        tracing::info!(
            mbid = %mbid,
            title = %recording.title,
            artist = %recording.artist_credit.first().map(|a| a.name.as_str()).unwrap_or("Unknown"),
            "Retrieved recording from MusicBrainz"
        );

        Ok(recording)
    }

    /// Lookup multiple recordings by MBIDs
    ///
    /// Automatically rate-limited to 1 req/sec
    pub async fn lookup_recordings(&self, mbids: &[String]) -> Vec<Result<MBRecording, MBError>> {
        let mut results = Vec::with_capacity(mbids.len());

        for mbid in mbids {
            results.push(self.lookup_recording(mbid).await);
        }

        results
    }

    /// Search recordings using Lucene query syntax
    ///
    /// **[REQ-CTXM-020]** Search by artist + title
    ///
    /// # Arguments
    /// * `query` - Lucene query string (e.g., `artist:"Beatles" AND recording:"Help!"`)
    /// * `limit` - Maximum number of results (default 25, max 100)
    ///
    /// # Returns
    /// List of recording search results sorted by relevance score
    pub async fn search_recordings(
        &self,
        query: &str,
        limit: Option<u32>,
    ) -> Result<MBRecordingSearchResponse, MBError> {
        // Rate limit
        self.rate_limiter.wait().await;

        let limit = limit.unwrap_or(25).min(100);

        // Query API
        let url = format!(
            "{}/recording/?query={}&limit={}&fmt=json",
            MUSICBRAINZ_BASE_URL,
            urlencoding::encode(query),
            limit
        );

        tracing::debug!(query = %query, limit, url = %url, "Searching MusicBrainz recordings");

        let operation = format!("search recordings: {}", query);
        let response = self
            .execute_with_retry(&operation, || {
                self.http_client.get(&url).send()
            })
            .await?;

        let status = response.status();

        if status == 503 {
            return Err(MBError::RateLimitExceeded);
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(MBError::ApiError(status.as_u16(), error_text));
        }

        let search_response: MBRecordingSearchResponse = response
            .json()
            .await
            .map_err(|e| MBError::ParseError(e.to_string()))?;

        tracing::info!(
            query = %query,
            count = search_response.count,
            results = search_response.recordings.len(),
            "Recording search completed"
        );

        Ok(search_response)
    }

    /// Search releases using Lucene query syntax
    ///
    /// **[REQ-CTXM-030]** Search by artist + album + track count
    ///
    /// # Arguments
    /// * `query` - Lucene query string (e.g., `artist:"Beatles" AND release:"Abbey Road" AND tracks:17`)
    /// * `limit` - Maximum number of results (default 25, max 100)
    ///
    /// # Returns
    /// List of release search results sorted by relevance score
    pub async fn search_releases(
        &self,
        query: &str,
        limit: Option<u32>,
    ) -> Result<MBReleaseSearchResponse, MBError> {
        // Rate limit
        self.rate_limiter.wait().await;

        let limit = limit.unwrap_or(25).min(100);

        // Query API
        let url = format!(
            "{}/release/?query={}&limit={}&fmt=json",
            MUSICBRAINZ_BASE_URL,
            urlencoding::encode(query),
            limit
        );

        // **[DEBUG]** Log URL at INFO level for debugging search issues
        tracing::info!(query = %query, limit, url = %url, "Searching MusicBrainz releases");

        let operation = format!("search releases: {}", query);
        let response = self
            .execute_with_retry(&operation, || {
                self.http_client.get(&url).send()
            })
            .await?;

        let status = response.status();
        // **[DEBUG]** Log response status
        tracing::info!(status = %status, query = %query, "MusicBrainz API response");

        if status == 503 {
            tracing::warn!(query = %query, "MusicBrainz rate limit exceeded (503)");
            return Err(MBError::RateLimitExceeded);
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!(status = %status, error = %error_text, query = %query, "MusicBrainz API error");
            return Err(MBError::ApiError(status.as_u16(), error_text));
        }

        let search_response: MBReleaseSearchResponse = response
            .json()
            .await
            .map_err(|e| MBError::ParseError(e.to_string()))?;

        tracing::info!(
            query = %query,
            count = search_response.count,
            results = search_response.releases.len(),
            "Release search completed"
        );

        Ok(search_response)
    }
}

impl Default for MusicBrainzClient {
    fn default() -> Self {
        Self::new().expect("Failed to create MusicBrainz client")
    }
}

// =============================================================================
// Album Matching Extensions (PLAN030 Increment 12)
// =============================================================================

use crate::matching::types::{MBReleaseDetails, MBSearchResponse};

impl MusicBrainzClient {
    /// Lookup release by MBID with full track details
    ///
    /// **[PLAN030]** Fetches complete release information including
    /// all media and track listings for album matching.
    ///
    /// # Arguments
    /// * `release_mbid` - MusicBrainz Release MBID
    ///
    /// # Returns
    /// Complete release details including track durations
    pub async fn lookup_release(&self, release_mbid: &str) -> Result<MBReleaseDetails, MBError> {
        // Rate limit
        self.rate_limiter.wait().await;

        // Query API with recordings, artist credits, and labels
        // Note: country/date come in base response, no special inc needed
        let url = format!(
            "{}/release/{}?inc=recordings+media+artist-credits+labels&fmt=json",
            MUSICBRAINZ_BASE_URL, release_mbid
        );

        tracing::debug!(mbid = %release_mbid, url = %url, "Fetching release details from MusicBrainz");

        let operation = format!("lookup release {}", release_mbid);
        let response = self
            .execute_with_retry(&operation, || {
                self.http_client.get(&url).send()
            })
            .await?;

        let status = response.status();

        if status == 404 {
            return Err(MBError::RecordingNotFound(format!(
                "Release not found: {}",
                release_mbid
            )));
        }

        if status == 503 {
            return Err(MBError::RateLimitExceeded);
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(MBError::ApiError(status.as_u16(), error_text));
        }

        let details: MBReleaseDetails = response
            .json()
            .await
            .map_err(|e| MBError::ParseError(e.to_string()))?;

        tracing::info!(
            mbid = %release_mbid,
            title = %details.title,
            tracks = details.media.iter().map(|m| m.tracks.len()).sum::<usize>(),
            "Retrieved release details from MusicBrainz"
        );

        Ok(details)
    }

    /// Comprehensive album search with full track details
    ///
    /// **[PLAN030]** Searches for releases by artist/album and fetches
    /// complete track listings for each result.
    ///
    /// # Arguments
    /// * `artist` - Artist name (from metadata)
    /// * `album` - Album/release title (from metadata)
    /// * `limit` - Maximum releases to return (default 10)
    ///
    /// # Returns
    /// Vector of releases with full track details for album matching
    pub async fn comprehensive_search(
        &self,
        artist: &str,
        album: &str,
        limit: Option<usize>,
    ) -> Result<Vec<MBReleaseDetails>, MBError> {
        let limit = limit.unwrap_or(10);

        // Generate multi-strategy search queries (am29 approach)
        let strategies = generate_search_strategies(artist, album);
        let mut all_releases = Vec::new();
        let mut seen_mbids = std::collections::HashSet::new();

        tracing::debug!(
            artist = %artist,
            album = %album,
            strategies = strategies.len(),
            "Starting multi-strategy MusicBrainz search"
        );

        // Try each strategy, accumulating unique releases across strategies
        // until we reach the requested limit
        for (i, query) in strategies.iter().enumerate() {
            // **[DEBUG]** Log each strategy attempt at INFO level for visibility
            tracing::info!(
                strategy = i + 1,
                total = strategies.len(),
                query = %query,
                artist = %artist,
                album = %album,
                "Trying MusicBrainz search strategy"
            );

            // Search for releases using this strategy
            let search_response = match self.search_releases(query, Some(limit as u32)).await {
                Ok(response) => {
                    // **[DEBUG]** Log result count for each strategy
                    tracing::info!(
                        strategy = i + 1,
                        results = response.releases.len(),
                        query = %query,
                        "Strategy returned results"
                    );
                    response
                }
                Err(e) => {
                    tracing::warn!(
                        strategy = i + 1,
                        error = %e,
                        query = %query,
                        "Strategy failed, trying next"
                    );
                    continue;
                }
            };

            // Deduplicate by MBID and fetch full details
            for release in search_response.releases {
                if seen_mbids.insert(release.id.clone()) {
                    match self.lookup_release(&release.id).await {
                        Ok(details) => all_releases.push(details),
                        Err(e) => {
                            tracing::warn!(
                                release_id = %release.id,
                                error = %e,
                                "Failed to fetch release details, skipping"
                            );
                        }
                    }

                    // Stop if we have enough releases
                    if all_releases.len() >= limit {
                        break;
                    }
                }
            }

            // Stop if this strategy gave us enough results
            if all_releases.len() >= limit {
                tracing::info!(
                    strategy = i + 1,
                    found = all_releases.len(),
                    "Found enough results, stopping search"
                );
                break;
            }
        }

        // **[DEBUG]** Enhanced completion logging
        if all_releases.is_empty() {
            tracing::warn!(
                artist = %artist,
                album = %album,
                strategies_tried = strategies.len(),
                "Comprehensive search found NO releases after trying all strategies"
            );
        } else {
            tracing::info!(
                artist = %artist,
                album = %album,
                found = all_releases.len(),
                strategies_tried = strategies.len(),
                "Comprehensive search completed"
            );
        }

        Ok(all_releases)
    }

    /// Comprehensive album search with full track details (with caching)
    ///
    /// Same as `comprehensive_search()` but uses database caching to avoid redundant API calls.
    /// This dramatically speeds up album matching for:
    /// - Duplicate albums (same album in multiple formats)
    /// - Common albums across music libraries
    /// - Test re-runs and development iterations
    ///
    /// **Cache Strategy:**
    /// - Phase 1: Cache release search (artist/album → release MBIDs)
    /// - Phase 2: Cache release details (MBID → full track listing)
    /// - 90-day TTL (shorter than recording cache due to edition churn)
    ///
    /// # Arguments
    /// * `artist` - Artist name (from metadata)
    /// * `album` - Album/release title (from metadata)
    /// * `limit` - Maximum releases to return (default 10)
    /// * `pool` - Database connection pool for cache access
    ///
    /// # Returns
    /// Vector of releases with full track details for album matching
    pub async fn comprehensive_search_cached(
        &self,
        artist: &str,
        album: &str,
        limit: Option<usize>,
        pool: &sqlx::SqlitePool,
    ) -> Result<Vec<MBReleaseDetails>, MBError> {
        let limit = limit.unwrap_or(10);

        // Phase 1: Check search cache for release MBIDs
        let release_ids = if let Ok(Some(cached)) =
            crate::db::release_cache::get_search_cached(pool, artist, album).await
        {
            tracing::debug!(
                artist = %artist,
                album = %album,
                count = cached.release_ids.len(),
                cached_at = %cached.cached_at,
                "Album search cache hit"
            );
            cached.release_ids
        } else {
            tracing::debug!(
                artist = %artist,
                album = %album,
                "Album search cache miss, running multi-strategy search"
            );

            // Cache miss - run multi-strategy search
            // USER DECISION: Cache aggregated result after trying all successful strategies
            let search_results = self.comprehensive_search(artist, album, Some(limit)).await?;

            let ids: Vec<String> = search_results.iter().map(|r| r.id.clone()).collect();

            // Cache the aggregated search results
            if let Err(e) = crate::db::release_cache::cache_search(pool, artist, album, &ids).await
            {
                tracing::warn!(
                    artist = %artist,
                    album = %album,
                    error = %e,
                    "Failed to cache release search results"
                );
            }

            // Also cache all release details we just fetched
            for result in search_results {
                if let Ok(details_json) = serde_json::to_string(&result) {
                    if let Err(e) =
                        crate::db::release_cache::cache_details(pool, &result.id, &details_json).await
                    {
                        tracing::warn!(
                            mbid = %result.id,
                            error = %e,
                            "Failed to cache release details"
                        );
                    }
                }
            }

            ids
        };

        // Phase 2: Fetch details for each release (with caching)
        let mut details = Vec::with_capacity(release_ids.len());

        for mbid in release_ids.iter().take(limit) {
            // Check details cache first
            if let Ok(Some(cached)) =
                crate::db::release_cache::get_details_cached(pool, mbid).await
            {
                tracing::debug!(
                    mbid = %mbid,
                    cached_at = %cached.cached_at,
                    "Release details cache hit"
                );

                // Deserialize from cached JSON
                match serde_json::from_str::<MBReleaseDetails>(&cached.details_json) {
                    Ok(release_details) => {
                        details.push(release_details);
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!(
                            mbid = %mbid,
                            error = %e,
                            "Failed to deserialize cached release details, will query API"
                        );
                    }
                }
            }

            // Cache miss or deserialization failed - query API
            tracing::debug!(mbid = %mbid, "Release details cache miss, querying API");

            match self.lookup_release(mbid).await {
                Ok(full_details) => {
                    // Cache the result
                    if let Ok(details_json) = serde_json::to_string(&full_details) {
                        if let Err(e) =
                            crate::db::release_cache::cache_details(pool, mbid, &details_json).await
                        {
                            tracing::warn!(
                                mbid = %mbid,
                                error = %e,
                                "Failed to cache release details"
                            );
                        }
                    }

                    details.push(full_details);
                }
                Err(e) => {
                    tracing::warn!(
                        release_id = %mbid,
                        error = %e,
                        "Failed to fetch release details, skipping"
                    );
                }
            }
        }

        tracing::info!(
            artist = %artist,
            album = %album,
            found = details.len(),
            "Comprehensive search completed (with caching)"
        );

        Ok(details)
    }

    /// Album-only search (artist-relaxed fallback)
    ///
    /// Used as fallback when primary search fails due to MusicBrainz artist credit
    /// differing from file metadata (e.g., "Disney" vs "Lin-Manuel Miranda" for Moana,
    /// "Jessita Reyes" vs "Various Artists" for Native American Flute Lullabies).
    ///
    /// Searches by album title only, ignoring artist constraint. Uses per-release
    /// detail caching when a database pool is provided.
    pub async fn album_only_search(
        &self,
        album: &str,
        limit: Option<usize>,
        pool: Option<&sqlx::SqlitePool>,
    ) -> Result<Vec<MBReleaseDetails>, MBError> {
        let limit = limit.unwrap_or(25);

        let query = format!("type:album AND release:\"{}\"", escape_lucene(album));

        tracing::info!(
            album = %album,
            query = %query,
            "Album-only fallback search (artist-relaxed)"
        );

        let search_response = self.search_releases(&query, Some(limit as u32)).await?;

        tracing::info!(
            album = %album,
            found = search_response.releases.len(),
            "Album-only search returned results"
        );

        let mut all_releases = Vec::new();
        let mut seen_mbids = std::collections::HashSet::new();

        for release in search_response.releases {
            if !seen_mbids.insert(release.id.clone()) {
                continue;
            }

            // Try detail cache first
            if let Some(p) = pool {
                if let Ok(Some(cached)) =
                    crate::db::release_cache::get_details_cached(p, &release.id).await
                {
                    if let Ok(details) = serde_json::from_str::<MBReleaseDetails>(&cached.details_json) {
                        all_releases.push(details);
                        if all_releases.len() >= limit {
                            break;
                        }
                        continue;
                    }
                }
            }

            // Cache miss — fetch from API
            match self.lookup_release(&release.id).await {
                Ok(details) => {
                    // Cache the result if pool available
                    if let Some(p) = pool {
                        if let Ok(json) = serde_json::to_string(&details) {
                            let _ = crate::db::release_cache::cache_details(p, &release.id, &json).await;
                        }
                    }
                    all_releases.push(details);
                }
                Err(e) => {
                    tracing::warn!(
                        release_id = %release.id,
                        error = %e,
                        "Fallback: failed to fetch release details, skipping"
                    );
                }
            }

            if all_releases.len() >= limit {
                break;
            }
        }

        tracing::info!(
            album = %album,
            found = all_releases.len(),
            "Album-only fallback search completed"
        );

        Ok(all_releases)
    }

    /// Search releases and return basic search response
    ///
    /// **[PLAN030]** Wrapper returning MBSearchResponse type used by matching module.
    pub async fn search_releases_for_matching(
        &self,
        artist: &str,
        album: &str,
        limit: Option<u32>,
    ) -> Result<MBSearchResponse, MBError> {
        let query = format!(
            "artist:\"{}\" AND release:\"{}\"",
            escape_lucene(artist),
            escape_lucene(album)
        );

        let response = self.search_releases(&query, limit).await?;

        // Convert to matching module types
        let releases = response
            .releases
            .into_iter()
            .map(|r| crate::matching::types::MBRelease {
                id: r.id,
                title: r.title,
                artist_credit: r.artist_credit.map(|credits| {
                    credits
                        .into_iter()
                        .map(|c| crate::matching::types::MBArtistCredit {
                            name: c.name,
                            artist: crate::matching::types::MBArtist {
                                name: c.artist.name,
                            },
                        })
                        .collect()
                }),
                country: None,
                status: None,
                packaging: None,
            })
            .collect();

        Ok(MBSearchResponse { releases })
    }
}

/// Escape special Lucene query characters
///
/// MusicBrainz uses Lucene for search queries. Special characters must
/// be escaped to be treated as literals.
pub fn escape_lucene(s: &str) -> String {
    let special_chars = [
        '+', '-', '&', '|', '!', '(', ')', '{', '}', '[', ']', '^', '"', '~', '*', '?', ':', '\\',
        '/',
    ];
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}

/// Split CamelCase strings into space-separated words
///
/// Handles common ID3 tag issues where album names are concatenated without spaces.
///
/// # Examples
/// ```
/// # use wkmp_ai::services::musicbrainz_client::split_camel_case;
/// assert_eq!(split_camel_case("HappyNation"), "Happy Nation");
/// assert_eq!(split_camel_case("TheWall"), "The Wall");
/// assert_eq!(split_camel_case("ABC"), "ABC"); // Preserves all-caps
/// ```
fn split_camel_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 10);
    let chars: Vec<char> = s.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        // Add space before uppercase letter if:
        // 1. Not first character
        // 2. Previous char was lowercase OR next char is lowercase (handles "XMLParser" → "XML Parser")
        if i > 0 && c.is_uppercase() {
            let prev_lower = chars[i - 1].is_lowercase();
            let next_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
            if prev_lower || next_lower {
                result.push(' ');
            }
        }
        result.push(c);
    }

    result
}

/// Apply wildcard fixes for common misspellings
///
/// Returns Some(fixed_string) if a fix was applied, None otherwise.
/// Used in Strategy 4 of multi-strategy search.
///
/// # Examples
/// - "Lizzie" → "Lizz*" (catches "Lizzy")
/// - "Theatre" → "Theat*" (catches "Theater")
fn apply_wildcard_fixes(s: &str) -> Option<String> {
    // Common misspelling patterns
    let fixes = [
        ("Lizzie", "Lizz*"),
        ("Theatre", "Theat*"),
        ("Theater", "Theat*"),
        ("Colour", "Colo*"),
        ("Color", "Colo*"),
    ];

    for (pattern, replacement) in &fixes {
        if s.contains(pattern) {
            return Some(s.replace(pattern, replacement));
        }
    }

    None
}

/// Generate progressive search strategies for MusicBrainz album search
///
/// Returns 7 search strategies in order from most precise to most fuzzy.
/// Implements am29's multi-strategy approach for robust album matching.
///
/// # Strategies
/// 1. Basic unquoted search with type filter
/// 2. CamelCase split (handles concatenated titles)
/// 3. Fuzzy matching ~1 edit
/// 4. Wildcard fixes for common misspellings
/// 5. Aggressive fuzzy ~2 edits
/// 6. Per-token fuzzy (multi-word names)
/// 7. Album-only fallback (last resort)
///
/// # Arguments
/// * `artist` - Artist name from metadata
/// * `album` - Album name from metadata
///
/// Strip punctuation from text for search
///
/// **[PHASE 1 EXTENSION 4]** Handles punctuation variations:
/// - Hyphens and slashes become spaces: "Go-Go" → "Go Go", "AC/DC" → "AC DC"
/// - Other punctuation removed: "Go's" → "Gos", "R.E.M." → "REM"
fn strip_punctuation(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else if c == '-' || c == '/' {
                ' '  // Replace separators with space
            } else {
                '\0'  // Mark for removal
            }
        })
        .filter(|&c| c != '\0')  // Remove marked characters
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Strip "The " prefix from artist name
///
/// MusicBrainz search often fails when artist names start with "The"
/// due to how the search index handles articles.
///
/// # Examples
/// - "The Go-Go's" → "Go-Go's"
/// - "The Score" → "Score"
/// - "Beatles" → "Beatles" (unchanged, no prefix)
fn strip_the_prefix(artist: &str) -> &str {
    let prefixes = ["The ", "the "];
    for prefix in prefixes {
        if let Some(stripped) = artist.strip_prefix(prefix) {
            return stripped;
        }
    }
    artist
}

/// Strip common ensemble suffixes from artist name
///
/// Handles cases where the file metadata includes ensemble type
/// but MusicBrainz has the artist without it.
///
/// # Examples
/// - "Dave Brubeck Quartet" → "Dave Brubeck"
/// - "Boston Pops Orchestra" → "Boston Pops"
/// - "Dave Brubeck" → "Dave Brubeck" (unchanged)
fn strip_ensemble_suffix(artist: &str) -> String {
    let suffixes = [
        " Quartet", " Trio", " Quintet", " Sextet",
        " Band", " Orchestra", " Ensemble", " Group",
        " Philharmonic", " Symphony",
    ];

    let mut result = artist.to_string();
    for suffix in suffixes {
        if let Some(stripped) = result.strip_suffix(suffix) {
            result = stripped.to_string();
            break; // Only strip one suffix
        }
    }
    result
}

/// # Returns
/// Vec of Lucene query strings, ordered by precision (most specific first)
fn generate_search_strategies(artist: &str, album: &str) -> Vec<String> {
    let mut strategies = Vec::with_capacity(12);  // **[PHASE 1]** Increased from 7 to 12

    // Strategy 1: Basic unquoted search with type:album filter
    // Avoids quoted exact-match which is too restrictive
    strategies.push(format!(
        "type:album AND artist:{} AND release:{}",
        artist, album
    ));

    // Strategy 2: CamelCase split
    // Handles "HappyNation" → "Happy Nation"
    let album_spaced = split_camel_case(album);
    if album_spaced != album {
        strategies.push(format!(
            "type:album AND artist:{} AND release:\"{}\"",
            artist, album_spaced
        ));
    }

    // Strategy 3: Fuzzy matching ~1 edit
    // Catches minor typos and punctuation differences
    strategies.push(format!(
        "type:album AND artist:{}~ AND release:{}~",
        artist, album
    ));

    // Strategy 4: Wildcard fixes for common misspellings
    if let Some(artist_fixed) = apply_wildcard_fixes(artist) {
        let album_fixed = apply_wildcard_fixes(album).unwrap_or_else(|| album.to_string());
        strategies.push(format!(
            "type:album AND artist:{} AND release:{}",
            artist_fixed, album_fixed
        ));
    } else if let Some(album_fixed) = apply_wildcard_fixes(album) {
        strategies.push(format!(
            "type:album AND artist:{} AND release:{}",
            artist, album_fixed
        ));
    }

    // **[PHASE 1 EXTENSION 4 - REORDERED]** Artist name variation strategies
    // These precise strategies come BEFORE aggressive fuzzy to avoid overly-broad matches

    // Strategy 5: "The" prefix stripping (MOVED UP - was Strategy 11)
    // Handles "The Go-Go's" → "Go-Go's", "The Score" → "Score"
    // MusicBrainz search often fails with "The" prefix due to indexing
    let artist_no_the = strip_the_prefix(artist);
    if artist_no_the != artist {
        strategies.push(format!(
            "type:album AND artist:{} AND release:{}",
            artist_no_the, album
        ));
    }

    // Strategy 6: Ensemble suffix stripping (MOVED UP - was Strategy 12)
    // Handles "Dave Brubeck Quartet" → "Dave Brubeck"
    // Common suffixes: Quartet, Trio, Band, Orchestra, Ensemble
    let artist_no_ensemble = strip_ensemble_suffix(artist);
    if artist_no_ensemble != artist {
        strategies.push(format!(
            "type:album AND artist:{} AND release:{}",
            artist_no_ensemble, album
        ));
    }

    // Strategy 7: Punctuation-stripped search (MOVED UP - was Strategy 9)
    // Handles "Go-Go's" → "GoGos" variations
    let artist_stripped = strip_punctuation(artist);
    let album_stripped = strip_punctuation(album);
    if artist_stripped != artist || album_stripped != album {
        strategies.push(format!(
            "type:album AND artist:{} AND release:{}",
            artist_stripped, album_stripped
        ));
    }

    // Strategy 8: Aggressive fuzzy ~2 edits (was Strategy 5)
    // More tolerant, catches more misspellings
    strategies.push(format!(
        "type:album AND artist:{}~2 AND release:{}~2",
        artist, album
    ));

    // Strategy 9: Per-token fuzzy matching (was Strategy 6)
    // Handles multi-word names better - but is VERY broad, so comes late
    let artist_tokens: Vec<&str> = artist.split_whitespace().collect();
    let album_tokens: Vec<&str> = album.split_whitespace().collect();
    if artist_tokens.len() > 1 || album_tokens.len() > 1 {
        let artist_fuzzy = artist_tokens
            .iter()
            .map(|t| format!("{}~", t))
            .collect::<Vec<_>>()
            .join(" ");
        let album_fuzzy = album_tokens
            .iter()
            .map(|t| format!("{}~", t))
            .collect::<Vec<_>>()
            .join(" ");
        strategies.push(format!(
            "type:album AND artist:({}) AND release:({})",
            artist_fuzzy, album_fuzzy
        ));
    }

    // Strategy 10: Album-only fallback (was Strategy 7)
    // Last resort when artist name is problematic
    strategies.push(format!("type:album AND release:{}", album));

    // Strategy 11: Artist prefix/suffix search (was Strategy 8)
    // Handles "Carlos Santana" → "Santana" or "John Mayall" → "Mayall"
    if artist_tokens.len() >= 2 {
        // Try last name only (e.g., "Santana" from "Carlos Santana")
        let last_name = artist_tokens.last().unwrap();
        strategies.push(format!(
            "type:album AND artist:{} AND release:{}",
            last_name, album
        ));

        // Try first name only (e.g., "Carlos" from "Carlos Santana")
        let first_name = artist_tokens.first().unwrap();
        strategies.push(format!(
            "type:album AND artist:{} AND release:{}",
            first_name, album
        ));
    }

    // Strategy 12: Album-focused with artist wildcard (was Strategy 10)
    // Last resort - prioritizes album match with loose artist constraint
    if !artist_tokens.is_empty() {
        strategies.push(format!(
            "type:album AND artist:{}* AND release:\"{}\"",
            artist_tokens.first().unwrap(), album
        ));
    }

    // Note: Strategies 11 and 12 from original ordering have been moved up
    // to positions 5 and 6 respectively, ensuring precise artist variations
    // are tried before overly-broad fuzzy strategies that return wrong artists

    strategies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(1000);
        assert_eq!(limiter.min_interval, Duration::from_millis(1000));
    }

    #[test]
    fn test_client_creation() {
        let client = MusicBrainzClient::new();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_timing() {
        let limiter = RateLimiter::new(500); // 500ms for faster test

        let start = Instant::now();

        // First request - no wait
        limiter.wait().await;
        let first_elapsed = start.elapsed();

        // Second request - should wait ~500ms
        limiter.wait().await;
        let second_elapsed = start.elapsed();

        // Third request - should wait another ~500ms
        limiter.wait().await;
        let third_elapsed = start.elapsed();

        assert!(first_elapsed < Duration::from_millis(100)); // Minimal delay
        assert!(second_elapsed >= Duration::from_millis(450)); // ~500ms wait
        assert!(third_elapsed >= Duration::from_millis(950)); // ~1000ms total
    }

    // =========================================================================
    // PLAN030 Increment 12: MusicBrainz Integration Tests
    // =========================================================================

    /// TC-U-012-01: Verify Lucene query construction for comprehensive search
    #[test]
    fn test_comprehensive_search_query_format() {
        // Test query construction with normal artist/album
        let artist = "The Beatles";
        let album = "Abbey Road";
        let expected_query = "artist:\"The Beatles\" AND release:\"Abbey Road\"";

        let query = format!(
            "artist:\"{}\" AND release:\"{}\"",
            escape_lucene(artist),
            escape_lucene(album)
        );

        assert_eq!(query, expected_query);
    }

    /// TC-U-012-02: Verify special Lucene characters are escaped
    #[test]
    fn test_escape_lucene_special_chars() {
        // Test each special character
        assert_eq!(escape_lucene("+"), "\\+");
        assert_eq!(escape_lucene("-"), "\\-");
        assert_eq!(escape_lucene("&"), "\\&");
        assert_eq!(escape_lucene("|"), "\\|");
        assert_eq!(escape_lucene("!"), "\\!");
        assert_eq!(escape_lucene("("), "\\(");
        assert_eq!(escape_lucene(")"), "\\)");
        assert_eq!(escape_lucene("{"), "\\{");
        assert_eq!(escape_lucene("}"), "\\}");
        assert_eq!(escape_lucene("["), "\\[");
        assert_eq!(escape_lucene("]"), "\\]");
        assert_eq!(escape_lucene("^"), "\\^");
        assert_eq!(escape_lucene("\""), "\\\"");
        assert_eq!(escape_lucene("~"), "\\~");
        assert_eq!(escape_lucene("*"), "\\*");
        assert_eq!(escape_lucene("?"), "\\?");
        assert_eq!(escape_lucene(":"), "\\:");
        assert_eq!(escape_lucene("\\"), "\\\\");
        assert_eq!(escape_lucene("/"), "\\/");
    }

    /// TC-U-012-02: Verify normal text passes through unchanged
    #[test]
    fn test_escape_lucene_normal_text() {
        assert_eq!(escape_lucene("The Beatles"), "The Beatles");
        assert_eq!(escape_lucene("Abbey Road"), "Abbey Road");
        assert_eq!(escape_lucene("Sgt. Pepper's"), "Sgt. Pepper's");
    }

    /// TC-U-012-02: Verify mixed text with special chars
    #[test]
    fn test_escape_lucene_mixed_text() {
        // Artist names with special characters
        assert_eq!(escape_lucene("AC/DC"), "AC\\/DC");
        assert_eq!(escape_lucene("Guns N' Roses"), "Guns N' Roses");
        assert_eq!(escape_lucene("R.E.M."), "R.E.M.");

        // Album names with special characters
        assert_eq!(escape_lucene("What's Going On?"), "What's Going On\\?");
        assert_eq!(
            escape_lucene("...And Justice for All"),
            "...And Justice for All"
        );
        assert_eq!(
            escape_lucene("(What's the Story) Morning Glory?"),
            "\\(What's the Story\\) Morning Glory\\?"
        );
    }

    /// TC-U-012-02: Verify query building with special characters escaped
    #[test]
    fn test_comprehensive_search_query_with_escaping() {
        // AC/DC - Back in Black
        let query = format!(
            "artist:\"{}\" AND release:\"{}\"",
            escape_lucene("AC/DC"),
            escape_lucene("Back in Black")
        );
        assert_eq!(query, "artist:\"AC\\/DC\" AND release:\"Back in Black\"");

        // Question mark in album
        let query = format!(
            "artist:\"{}\" AND release:\"{}\"",
            escape_lucene("Marvin Gaye"),
            escape_lucene("What's Going On?")
        );
        assert_eq!(
            query,
            "artist:\"Marvin Gaye\" AND release:\"What's Going On\\?\""
        );
    }

    /// Test empty string handling
    #[test]
    fn test_escape_lucene_empty() {
        assert_eq!(escape_lucene(""), "");
    }

    /// Test all special chars in one string
    #[test]
    fn test_escape_lucene_all_special() {
        let all_special = "+-&|!(){}[]^\"~*?:\\/";
        let escaped = escape_lucene(all_special);
        assert_eq!(
            escaped,
            "\\+\\-\\&\\|\\!\\(\\)\\{\\}\\[\\]\\^\\\"\\~\\*\\?\\:\\\\\\/",
        );
    }

    // =========================================================================
    // Phase 1 Extension 4: New Search Strategies Tests
    // =========================================================================

    #[test]
    fn test_strip_punctuation_basic() {
        assert_eq!(strip_punctuation("Go-Go's"), "Go Gos");
        assert_eq!(strip_punctuation("AC/DC"), "AC DC");
        assert_eq!(strip_punctuation("R.E.M."), "REM");
    }

    #[test]
    fn test_strip_punctuation_no_change() {
        assert_eq!(strip_punctuation("The Beatles"), "The Beatles");
        assert_eq!(strip_punctuation("Abbey Road"), "Abbey Road");
    }

    #[test]
    fn test_generate_search_strategies_count() {
        let artist = "Carlos Santana";
        let album = "Abraxas";
        let strategies = generate_search_strategies(artist, album);

        // Should have at least 7 base strategies + up to 3 new ones
        assert!(strategies.len() >= 7, "Should have at least 7 strategies, got {}", strategies.len());
    }

    #[test]
    fn test_generate_search_strategies_artist_prefix() {
        let artist = "Carlos Santana";
        let album = "Abraxas";
        let strategies = generate_search_strategies(artist, album);

        // Should include last-name-only search (Strategy 8)
        let has_last_name = strategies.iter().any(|s| s.contains("artist:Santana AND"));
        assert!(has_last_name, "Should include last-name search strategy");

        // Should include first-name-only search (Strategy 8)
        let has_first_name = strategies.iter().any(|s| s.contains("artist:Carlos AND"));
        assert!(has_first_name, "Should include first-name search strategy");
    }

    #[test]
    fn test_generate_search_strategies_punctuation() {
        let artist = "The Go-Go's";
        let album = "Beauty and the Beat";
        let strategies = generate_search_strategies(artist, album);

        // Should include punctuation-stripped search (Strategy 9)
        let has_stripped = strategies.iter().any(|s| s.contains("Go Gos"));
        assert!(has_stripped, "Should include punctuation-stripped strategy");
    }

    #[test]
    fn test_generate_search_strategies_wildcard() {
        let artist = "John Mayall";
        let album = "A Hard Road";
        let strategies = generate_search_strategies(artist, album);

        // Should include artist wildcard search (Strategy 10)
        let has_wildcard = strategies.iter().any(|s| s.contains("artist:John* AND"));
        assert!(has_wildcard, "Should include artist wildcard strategy");
    }

    #[test]
    fn test_generate_search_strategies_single_name() {
        // Single-word artist should not trigger first/last name strategies
        let artist = "Madonna";
        let album = "Like a Prayer";
        let strategies = generate_search_strategies(artist, album);

        // Should still have base strategies
        assert!(strategies.len() >= 5, "Should have base strategies for single-word artist");
    }

    #[test]
    fn test_strip_the_prefix() {
        assert_eq!(strip_the_prefix("The Go-Go's"), "Go-Go's");
        assert_eq!(strip_the_prefix("The Score"), "Score");
        assert_eq!(strip_the_prefix("the beatles"), "beatles");
        assert_eq!(strip_the_prefix("Beatles"), "Beatles"); // No change
        assert_eq!(strip_the_prefix("Therapy?"), "Therapy?"); // No change
    }

    #[test]
    fn test_strip_ensemble_suffix() {
        assert_eq!(strip_ensemble_suffix("Dave Brubeck Quartet"), "Dave Brubeck");
        assert_eq!(strip_ensemble_suffix("Modern Jazz Quartet"), "Modern Jazz");
        assert_eq!(strip_ensemble_suffix("Boston Pops Orchestra"), "Boston Pops");
        assert_eq!(strip_ensemble_suffix("Dave Brubeck"), "Dave Brubeck"); // No change
        assert_eq!(strip_ensemble_suffix("Kraftwerk"), "Kraftwerk"); // No change
    }

    #[test]
    fn test_generate_search_strategies_the_prefix() {
        let artist = "The Go-Go's";
        let album = "Beauty and the Beat";
        let strategies = generate_search_strategies(artist, album);

        // Should include "The" stripped search (Strategy 11)
        let has_stripped = strategies.iter().any(|s| s.contains("artist:Go-Go's"));
        assert!(has_stripped, "Should include 'The' stripped strategy");
    }

    #[test]
    fn test_generate_search_strategies_ensemble_suffix() {
        let artist = "Dave Brubeck Quartet";
        let album = "Time Out";
        let strategies = generate_search_strategies(artist, album);

        // Should include ensemble suffix stripped search (Strategy 12)
        let has_stripped = strategies.iter().any(|s| s.contains("artist:Dave Brubeck AND") && !s.contains("Quartet"));
        assert!(has_stripped, "Should include ensemble suffix stripped strategy");
    }
}

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
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Recording not found: {0}")]
    RecordingNotFound(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("API error {0}: {1}")]
    ApiError(u16, String),

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

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| MBError::NetworkError(e.to_string()))?;

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
}

impl Default for MusicBrainzClient {
    fn default() -> Self {
        Self::new().expect("Failed to create MusicBrainz client")
    }
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
}

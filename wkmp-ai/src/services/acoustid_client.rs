//! AcoustID API client
//!
//! **[AIA-INT-020]** AcoustID fingerprint lookup integration
//!
//! Per [IMPL012](../../docs/IMPL012-acoustid_client.md)

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;

const ACOUSTID_BASE_URL: &str = "https://api.acoustid.org/v2/lookup";
const ACOUSTID_API_KEY: &str = "YOUR_API_KEY"; // TODO: Load from config
const USER_AGENT: &str = "WKMP/0.1.0 (https://github.com/wkmp/wkmp)";
const RATE_LIMIT_MS: u64 = 334; // 3 requests per second (~333ms between requests)

/// AcoustID client errors
#[derive(Debug, Error)]
pub enum AcoustIDError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("No matches found for fingerprint")]
    NoMatches,

    #[error("API error {0}: {1}")]
    ApiError(u16, String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid API key")]
    InvalidApiKey,
}

/// AcoustID lookup response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcoustIDResponse {
    pub status: String,
    pub results: Vec<AcoustIDResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcoustIDResult {
    pub id: String, // AcoustID
    pub score: f64, // Match confidence (0.0 to 1.0)
    pub recordings: Option<Vec<AcoustIDRecording>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcoustIDRecording {
    pub id: String, // MusicBrainz Recording MBID
    pub title: Option<String>,
    pub artists: Option<Vec<AcoustIDArtist>>,
    pub duration: Option<u64>, // Seconds
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcoustIDArtist {
    pub id: String, // MusicBrainz Artist MBID
    pub name: String,
}

/// Rate limiter for AcoustID (3 req/sec)
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

    async fn wait(&self) {
        let mut last = self.last_request.lock().await;

        if let Some(last_time) = *last {
            let elapsed = last_time.elapsed();
            if elapsed < self.min_interval {
                let wait_time = self.min_interval - elapsed;
                tracing::debug!("AcoustID rate limiting: waiting {:?}", wait_time);
                tokio::time::sleep(wait_time).await;
            }
        }

        *last = Some(Instant::now());
    }
}

/// AcoustID API client
pub struct AcoustIDClient {
    http_client: reqwest::Client,
    rate_limiter: Arc<RateLimiter>,
    api_key: String,
}

impl AcoustIDClient {
    pub fn new(api_key: String) -> Result<Self, AcoustIDError> {
        let http_client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| AcoustIDError::NetworkError(e.to_string()))?;

        Ok(Self {
            http_client,
            rate_limiter: Arc::new(RateLimiter::new(RATE_LIMIT_MS)),
            api_key,
        })
    }

    /// Lookup recording by Chromaprint fingerprint
    ///
    /// **[AIA-INT-020]** Query AcoustID for MusicBrainz MBIDs
    pub async fn lookup(
        &self,
        fingerprint: &str,
        duration_seconds: u64,
    ) -> Result<AcoustIDResponse, AcoustIDError> {
        // Rate limit
        self.rate_limiter.wait().await;

        // Build query parameters
        let params = [
            ("client", self.api_key.as_str()),
            ("meta", "recordings recordingids"),
            ("duration", &duration_seconds.to_string()),
            ("fingerprint", fingerprint),
        ];

        tracing::debug!(
            duration_seconds = duration_seconds,
            "Querying AcoustID API"
        );

        let response = self
            .http_client
            .post(ACOUSTID_BASE_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| AcoustIDError::NetworkError(e.to_string()))?;

        let status = response.status();

        if status == 401 {
            return Err(AcoustIDError::InvalidApiKey);
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AcoustIDError::ApiError(status.as_u16(), error_text));
        }

        let acoustid_response: AcoustIDResponse = response
            .json()
            .await
            .map_err(|e| AcoustIDError::ParseError(e.to_string()))?;

        if acoustid_response.results.is_empty() {
            return Err(AcoustIDError::NoMatches);
        }

        // Log top match
        if let Some(top_result) = acoustid_response.results.first() {
            tracing::info!(
                acoustid = %top_result.id,
                score = top_result.score,
                recordings = top_result.recordings.as_ref().map(|r| r.len()).unwrap_or(0),
                "AcoustID lookup successful"
            );
        }

        Ok(acoustid_response)
    }

    /// Get best MusicBrainz MBID from lookup result
    ///
    /// Returns the recording with highest score
    pub fn get_best_mbid(response: &AcoustIDResponse) -> Option<String> {
        response
            .results
            .first()?
            .recordings
            .as_ref()?
            .first()
            .map(|r| r.id.clone())
    }
}

impl Default for AcoustIDClient {
    fn default() -> Self {
        Self::new(ACOUSTID_API_KEY.to_string()).expect("Failed to create AcoustID client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(334);
        assert_eq!(limiter.min_interval, Duration::from_millis(334));
    }

    #[test]
    fn test_client_creation() {
        let client = AcoustIDClient::new("test_key".to_string());
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_allows_3_per_second() {
        let limiter = RateLimiter::new(334); // ~3 req/sec

        let start = Instant::now();

        // Make 3 requests
        for _ in 0..3 {
            limiter.wait().await;
        }

        let elapsed = start.elapsed();

        // Should take at least ~668ms (2 waits * 334ms)
        assert!(elapsed >= Duration::from_millis(600));
        // But less than 1 second
        assert!(elapsed < Duration::from_secs(1));
    }

    #[test]
    fn test_get_best_mbid() {
        let response = AcoustIDResponse {
            status: "ok".to_string(),
            results: vec![AcoustIDResult {
                id: "acoustid-123".to_string(),
                score: 0.95,
                recordings: Some(vec![AcoustIDRecording {
                    id: "mbid-456".to_string(),
                    title: Some("Test Song".to_string()),
                    artists: None,
                    duration: Some(180),
                }]),
            }],
        };

        let mbid = AcoustIDClient::get_best_mbid(&response);
        assert_eq!(mbid, Some("mbid-456".to_string()));
    }

    #[test]
    fn test_get_best_mbid_no_results() {
        let response = AcoustIDResponse {
            status: "ok".to_string(),
            results: vec![],
        };

        let mbid = AcoustIDClient::get_best_mbid(&response);
        assert_eq!(mbid, None);
    }
}

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
const USER_AGENT: &str = "WKMP/0.1.0 (https://github.com/wkmp/wkmp)";
const RATE_LIMIT_MS: u64 = 334; // 3 requests per second (~333ms between requests)

// **[PLAN031 Fix 1]** Emergency kill switch and ultra-aggressive timeout
const ACOUSTID_ENABLED: bool = false; // EMERGENCY: Disable entirely (725 timeouts in testV.log)
const ACOUSTID_TIMEOUT_SECS: u64 = 1; // Down from 5s (PLAN030) -> 1s (PLAN031 emergency)
const CIRCUIT_BREAKER_THRESHOLD: u32 = 3; // Open after 3 consecutive failures
const CIRCUIT_BREAKER_COOLDOWN_SECS: u64 = 60; // Stay open for 60s before retry

/// AcoustID client errors
#[derive(Debug, Error)]
pub enum AcoustIDError {
    /// Network communication error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// No matching recordings found for fingerprint
    #[error("No matches found for fingerprint")]
    NoMatches,

    /// AcoustID API returned error response
    #[error("API error {0}: {1}")]
    ApiError(u16, String),

    /// Failed to parse API response JSON
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Invalid or missing API key
    #[error("Invalid API key")]
    InvalidApiKey,
}

/// AcoustID lookup response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcoustIDResponse {
    /// API response status (e.g., "ok", "error")
    pub status: String,
    /// List of matching results
    pub results: Vec<AcoustIDResult>,
}

/// AcoustID match result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcoustIDResult {
    /// AcoustID fingerprint identifier
    pub id: String,
    /// Match confidence score (0.0 to 1.0)
    pub score: f64,
    /// Matching MusicBrainz recordings
    pub recordings: Option<Vec<AcoustIDRecording>>,
}

/// AcoustID recording information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcoustIDRecording {
    /// MusicBrainz Recording MBID
    pub id: String,
    /// Recording title
    pub title: Option<String>,
    /// Artist credits
    pub artists: Option<Vec<AcoustIDArtist>>,
    /// Recording duration in seconds
    pub duration: Option<u64>,
}

/// AcoustID artist information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcoustIDArtist {
    /// MusicBrainz Artist MBID
    pub id: String,
    /// Artist name
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

/// **[PLAN030 Task 3.4]** Circuit breaker to prevent cascading failures
struct CircuitBreaker {
    state: Mutex<CircuitState>,
}

#[derive(Debug, Clone)]
enum CircuitState {
    Closed { consecutive_failures: u32 },
    Open { opened_at: Instant },
}

impl CircuitBreaker {
    fn new() -> Self {
        Self {
            state: Mutex::new(CircuitState::Closed {
                consecutive_failures: 0,
            }),
        }
    }

    async fn is_open(&self) -> bool {
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::Open { opened_at } => {
                // Check if cooldown has expired
                if opened_at.elapsed() >= Duration::from_secs(CIRCUIT_BREAKER_COOLDOWN_SECS) {
                    tracing::info!("Circuit breaker cooldown expired, attempting half-open state");
                    *state = CircuitState::Closed {
                        consecutive_failures: 0,
                    };
                    false
                } else {
                    true
                }
            }
            CircuitState::Closed { .. } => false,
        }
    }

    async fn on_success(&self) {
        let mut state = self.state.lock().await;
        *state = CircuitState::Closed {
            consecutive_failures: 0,
        };
    }

    async fn on_failure(&self) {
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::Closed {
                consecutive_failures,
            } => {
                let new_failures = consecutive_failures + 1;
                if new_failures >= CIRCUIT_BREAKER_THRESHOLD {
                    tracing::warn!(
                        "AcoustID circuit breaker opened after {} consecutive failures",
                        new_failures
                    );
                    *state = CircuitState::Open {
                        opened_at: Instant::now(),
                    };
                } else {
                    *state = CircuitState::Closed {
                        consecutive_failures: new_failures,
                    };
                }
            }
            CircuitState::Open { .. } => {
                // Already open, no change
            }
        }
    }
}

/// AcoustID API client with database caching
///
/// **[PLAN030 Task 3.4]** Enhanced with circuit breaker and 5s timeout
pub struct AcoustIDClient {
    http_client: reqwest::Client,
    rate_limiter: Arc<RateLimiter>,
    circuit_breaker: Arc<CircuitBreaker>,
    api_key: String,
    db: sqlx::SqlitePool,
}

impl AcoustIDClient {
    /// Create new AcoustID client with API key and database pool
    ///
    /// **[PLAN030 Task 3.4]** 5-second timeout + circuit breaker
    pub fn new(api_key: String, db: sqlx::SqlitePool) -> Result<Self, AcoustIDError> {
        let http_client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(ACOUSTID_TIMEOUT_SECS)) // **[PLAN030]** 5s (was 30s)
            .build()
            .map_err(|e| AcoustIDError::NetworkError(e.to_string()))?;

        Ok(Self {
            http_client,
            rate_limiter: Arc::new(RateLimiter::new(RATE_LIMIT_MS)),
            circuit_breaker: Arc::new(CircuitBreaker::new()), // **[PLAN030]** Circuit breaker
            api_key,
            db,
        })
    }

    /// Lookup recording by Chromaprint fingerprint
    ///
    /// **[AIA-INT-020]** Query AcoustID for MusicBrainz MBIDs
    /// **[REQ-CA-010]** Check cache before API call
    pub async fn lookup(
        &self,
        fingerprint: &str,
        duration_seconds: u64,
    ) -> Result<AcoustIDResponse, AcoustIDError> {
        // Check cache first
        if let Some(mbid) = self.get_cached_mbid(fingerprint).await? {
            tracing::debug!("Cache hit for fingerprint");
            // Construct response from cached MBID
            return Ok(AcoustIDResponse {
                status: "ok".to_string(),
                results: vec![AcoustIDResult {
                    id: String::new(), // Not cached
                    score: 1.0, // Cached result assumed perfect match
                    recordings: Some(vec![AcoustIDRecording {
                        id: mbid,
                        title: None,
                        artists: None,
                        duration: None,
                    }]),
                }],
            });
        }

        // **[PLAN031 Fix 1]** Emergency kill switch - AcoustID causing 725 timeouts in testV.log
        if !ACOUSTID_ENABLED {
            tracing::debug!("AcoustID disabled via ACOUSTID_ENABLED flag, skipping lookup");
            return Err(AcoustIDError::NoMatches);
        }

        // **[PLAN030 Task 3.4]** Check circuit breaker state
        if self.circuit_breaker.is_open().await {
            tracing::warn!("AcoustID circuit breaker is open, skipping lookup");
            return Err(AcoustIDError::NoMatches);
        }

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
            fingerprint_len = fingerprint.len(),
            fingerprint_preview = &fingerprint[..fingerprint.len().min(100)],
            "Querying AcoustID API"
        );

        // **[PLAN030 Task 3.4]** Wrap HTTP request with circuit breaker callbacks
        let response = match self
            .http_client
            .post(ACOUSTID_BASE_URL)
            .form(&params)
            .send()
            .await
        {
            Ok(resp) => {
                // **[PLAN030]** Record success (network level)
                self.circuit_breaker.on_success().await;
                resp
            }
            Err(e) => {
                // **[PLAN030]** Record failure (timeout, connection error)
                self.circuit_breaker.on_failure().await;
                tracing::warn!("AcoustID network error: {}", e);
                return Err(AcoustIDError::NetworkError(e.to_string()));
            }
        };

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

            // Cache successful result if we have an MBID
            if let Some(mbid) = Self::get_best_mbid(&acoustid_response) {
                if let Err(e) = self.cache_mbid(fingerprint, &mbid).await {
                    tracing::warn!("Failed to cache fingerprint → MBID mapping: {}", e);
                    // Non-fatal - continue with result
                }
            }
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

    /// Hash fingerprint using SHA-256
    ///
    /// **[REQ-CA-030]** Deterministic SHA-256 hashing for cache keys
    fn hash_fingerprint(&self, fingerprint: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(fingerprint.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get cached MBID from database
    ///
    /// **[REQ-CA-010]** Check cache before API call
    async fn get_cached_mbid(&self, fingerprint: &str) -> Result<Option<String>, AcoustIDError> {
        let fingerprint_hash = self.hash_fingerprint(fingerprint);

        let row: Option<(String,)> = sqlx::query_as(
            "SELECT mbid FROM acoustid_cache WHERE fingerprint_hash = ?"
        )
        .bind(&fingerprint_hash)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AcoustIDError::NetworkError(format!("Database error: {}", e)))?;

        Ok(row.map(|(mbid,)| mbid))
    }

    /// Cache fingerprint → MBID mapping
    ///
    /// **[REQ-CA-020]** Store successful lookups in database
    async fn cache_mbid(&self, fingerprint: &str, mbid: &str) -> Result<(), AcoustIDError> {
        let fingerprint_hash = self.hash_fingerprint(fingerprint);

        sqlx::query(
            "INSERT INTO acoustid_cache (fingerprint_hash, mbid, cached_at)
             VALUES (?, ?, datetime('now'))
             ON CONFLICT(fingerprint_hash) DO UPDATE SET
                mbid = excluded.mbid,
                cached_at = excluded.cached_at"
        )
        .bind(&fingerprint_hash)
        .bind(mbid)
        .execute(&self.db)
        .await
        .map_err(|e| AcoustIDError::NetworkError(format!("Database error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_db() -> sqlx::SqlitePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // Create acoustid_cache table
        sqlx::query(
            "CREATE TABLE acoustid_cache (
                fingerprint_hash TEXT PRIMARY KEY,
                mbid TEXT NOT NULL,
                cached_at TEXT NOT NULL DEFAULT (datetime('now')),
                CHECK (length(fingerprint_hash) = 64)
            )"
        )
        .execute(&pool)
        .await
        .expect("Failed to create test table");

        pool
    }

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(334);
        assert_eq!(limiter.min_interval, Duration::from_millis(334));
    }

    #[tokio::test]
    async fn test_client_creation() {
        let db = create_test_db().await;
        let client = AcoustIDClient::new("test_key".to_string(), db);
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

    #[tokio::test]
    async fn test_fingerprint_hash_determinism() {
        let db = create_test_db().await;
        let client = AcoustIDClient::new("test_key".to_string(), db).unwrap();

        let hash1 = client.hash_fingerprint("AQADtN...");
        let hash2 = client.hash_fingerprint("AQADtN...");

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex length
    }

    #[tokio::test]
    async fn test_fingerprint_hash_uniqueness() {
        let db = create_test_db().await;
        let client = AcoustIDClient::new("test_key".to_string(), db).unwrap();

        let hash1 = client.hash_fingerprint("AQADtN...");
        let hash3 = client.hash_fingerprint("different");

        assert_ne!(hash1, hash3);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let db = create_test_db().await;
        let client = AcoustIDClient::new("test_key".to_string(), db).unwrap();

        let mbid = client.get_cached_mbid("AQADtN...").await.unwrap();
        assert_eq!(mbid, None);
    }

    #[tokio::test]
    async fn test_cache_insert() {
        let db = create_test_db().await;
        let client = AcoustIDClient::new("test_key".to_string(), db).unwrap();

        // Cache a fingerprint → MBID mapping
        client.cache_mbid("AQADtN...", "mbid-123").await.unwrap();

        // Verify it was cached
        let mbid = client.get_cached_mbid("AQADtN...").await.unwrap();
        assert_eq!(mbid, Some("mbid-123".to_string()));
    }

    #[tokio::test]
    async fn test_cache_upsert() {
        let db = create_test_db().await;
        let client = AcoustIDClient::new("test_key".to_string(), db).unwrap();

        // Cache initial value
        client.cache_mbid("AQADtN...", "mbid-123").await.unwrap();

        // Update with new value (UPSERT)
        client.cache_mbid("AQADtN...", "mbid-456").await.unwrap();

        // Verify it was updated
        let mbid = client.get_cached_mbid("AQADtN...").await.unwrap();
        assert_eq!(mbid, Some("mbid-456".to_string()));
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let db = create_test_db().await;
        let client = AcoustIDClient::new("test_key".to_string(), db).unwrap();

        // Pre-populate cache
        client.cache_mbid("AQADtN...", "mbid-789").await.unwrap();

        // Cache hit should return cached value without API call
        let mbid = client.get_cached_mbid("AQADtN...").await.unwrap();
        assert_eq!(mbid, Some("mbid-789".to_string()));
    }
}

//! MusicBrainz Client (Tier 1)
//!
//! Queries MusicBrainz API to resolve Recording MBIDs to metadata.
//! Uses MBIDs from TASK-005 (ID3 tags) or TASK-007 (AcoustID) to fetch authoritative metadata.
//!
//! # Implementation
//! - TASK-008: MusicBrainz Client (PLAN024)
//! - Confidence: 0.9 (authoritative metadata source)
//!
//! # Architecture
//! Implements `SourceExtractor` trait for integration with parallel extraction pipeline.
//! Fetches metadata from MusicBrainz Web Service (WS/2).
//!
//! # API Reference
//! - Endpoint: https://musicbrainz.org/ws/2/recording/{mbid}
//! - Documentation: https://musicbrainz.org/doc/Development/XML_Web_Service/Version_2
//! - Rate Limit: 1 request/second (as per MusicBrainz Terms of Service)

use crate::types::{
    ConfidenceValue, ExtractionError, ExtractionResult, MetadataExtraction, PassageContext,
    SourceExtractor,
};
use async_trait::async_trait;
use reqwest::{header, Client};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use tracing::debug;

/// MusicBrainz API base URL
const MUSICBRAINZ_API_URL: &str = "https://musicbrainz.org/ws/2";

/// Default timeout for MusicBrainz API requests
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

/// Rate limit: 1 request per second (MusicBrainz TOS)
const RATE_LIMIT_INTERVAL: Duration = Duration::from_millis(1000);

/// User-Agent header (required by MusicBrainz)
const USER_AGENT: &str = "WKMP-AI/0.1.0 (https://github.com/yourusername/wkmp)";

/// MusicBrainz Client
///
/// Queries MusicBrainz API to fetch Recording metadata from Recording MBID.
/// Implements rate limiting (1 req/sec) per MusicBrainz Terms of Service.
///
/// # Confidence Scoring
/// Base confidence: 0.9 (authoritative metadata source)
/// - MusicBrainz is the canonical music metadata database
/// - Higher confidence than ID3 tags (user-editable)
/// - Same confidence level as ID3-embedded MBIDs
///
/// # Requirements
/// - Requires Recording MBID (from ID3 or AcoustID)
/// - Requires network connectivity
/// - Respects rate limiting (1 request/second)
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::extractors::musicbrainz_client::MusicBrainzClient;
/// use wkmp_ai::types::{SourceExtractor, PassageContext};
///
/// let client = MusicBrainzClient::new();
/// let result = client.extract(&passage_ctx).await?;
///
/// if let Some(metadata) = result.metadata {
///     if let Some(title) = metadata.title {
///         println!("Title: {} (confidence: {})", title.value, title.confidence);
///     }
/// }
/// ```
pub struct MusicBrainzClient {
    /// HTTP client for API requests
    http_client: Client,
    /// Base confidence for MusicBrainz metadata
    base_confidence: f32,
    /// Rate limiter (last request time)
    rate_limiter: Arc<Mutex<Option<Instant>>>,
}

impl MusicBrainzClient {
    /// Create new MusicBrainz client
    pub fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(USER_AGENT),
        );

        Self {
            http_client: Client::builder()
                .timeout(DEFAULT_TIMEOUT)
                .default_headers(headers)
                .build()
                .expect("Failed to create HTTP client"),
            base_confidence: 0.9,
            rate_limiter: Arc::new(Mutex::new(None)),
        }
    }

    /// Enforce rate limit (1 request/second)
    ///
    /// MusicBrainz Terms of Service requires rate limiting.
    /// This method sleeps if necessary to maintain 1 req/sec limit.
    async fn enforce_rate_limit(&self) {
        let mut last_request = self.rate_limiter.lock().await;

        if let Some(last_time) = *last_request {
            let elapsed = last_time.elapsed();
            if elapsed < RATE_LIMIT_INTERVAL {
                let sleep_duration = RATE_LIMIT_INTERVAL - elapsed;
                debug!(
                    sleep_ms = sleep_duration.as_millis(),
                    "Rate limiting: sleeping before MusicBrainz request"
                );
                sleep(sleep_duration).await;
            }
        }

        *last_request = Some(Instant::now());
    }

    /// Query MusicBrainz Recording by MBID
    ///
    /// # Arguments
    /// * `mbid` - MusicBrainz Recording ID (UUID format)
    ///
    /// # Returns
    /// Recording metadata (title, artist, album, etc.)
    ///
    /// # Errors
    /// Returns error if:
    /// - Network request fails
    /// - API returns error (404 = MBID not found)
    /// - Response parse fails
    async fn query_recording(
        &self,
        mbid: &str,
    ) -> Result<MetadataExtraction, ExtractionError> {
        debug!(mbid = %mbid, "Querying MusicBrainz Recording");

        // Enforce rate limit
        self.enforce_rate_limit().await;

        // Build request URL
        let url = format!(
            "{}/recording/{}?inc=artists+releases&fmt=json",
            MUSICBRAINZ_API_URL, mbid
        );

        // Execute request
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExtractionError::Network(format!("MusicBrainz API request failed: {}", e)))?;

        // Check status
        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 404 {
                return Err(ExtractionError::NotAvailable(format!(
                    "Recording MBID not found: {}",
                    mbid
                )));
            }
            let body = response.text().await.unwrap_or_default();
            return Err(ExtractionError::Api(format!(
                "MusicBrainz API returned error {}: {}",
                status, body
            )));
        }

        // Parse response
        let recording: MusicBrainzRecording = response.json().await.map_err(|e| {
            ExtractionError::Parse(format!("Failed to parse MusicBrainz response: {}", e))
        })?;

        // Extract metadata
        let metadata = self.extract_metadata_from_recording(&recording);

        debug!(
            mbid = %mbid,
            title = ?metadata.title.as_ref().map(|t| &t.value),
            artist = ?metadata.artist.as_ref().map(|a| &a.value),
            "MusicBrainz query complete"
        );

        Ok(metadata)
    }

    /// Extract metadata from MusicBrainz Recording response
    fn extract_metadata_from_recording(&self, recording: &MusicBrainzRecording) -> MetadataExtraction {
        let mut metadata = MetadataExtraction::default();

        // Title
        if !recording.title.is_empty() {
            metadata.title = Some(ConfidenceValue::new(
                recording.title.clone(),
                self.base_confidence,
                "MusicBrainz",
            ));
        }

        // Artist (from artist-credit)
        if let Some(ref artist_credit) = recording.artist_credit {
            if !artist_credit.is_empty() {
                // Combine multiple artists if present
                let artist_names: Vec<&str> = artist_credit
                    .iter()
                    .map(|ac| ac.name.as_str())
                    .collect();
                let artist_str = artist_names.join(", ");

                metadata.artist = Some(ConfidenceValue::new(
                    artist_str,
                    self.base_confidence,
                    "MusicBrainz",
                ));

                // Store artist MBIDs in additional metadata
                // For single artist: store directly
                // For multiple artists: store comma-separated list
                if artist_credit.len() == 1 {
                    metadata.additional.insert(
                        "artist_mbid".to_string(),
                        ConfidenceValue::new(
                            artist_credit[0].artist.id.clone(),
                            self.base_confidence,
                            "MusicBrainz",
                        ),
                    );
                } else {
                    // Multiple artists - store comma-separated MBIDs
                    let artist_mbids: Vec<&str> = artist_credit
                        .iter()
                        .map(|ac| ac.artist.id.as_str())
                        .collect();
                    let artist_mbids_str = artist_mbids.join(",");

                    metadata.additional.insert(
                        "artist_mbids".to_string(),
                        ConfidenceValue::new(
                            artist_mbids_str,
                            self.base_confidence,
                            "MusicBrainz",
                        ),
                    );
                }
            }
        }

        // Album (from first release if available)
        if let Some(ref releases) = recording.releases {
            if let Some(first_release) = releases.first() {
                if !first_release.title.is_empty() {
                    metadata.album = Some(ConfidenceValue::new(
                        first_release.title.clone(),
                        self.base_confidence,
                        "MusicBrainz",
                    ));

                    // Store release/album MBID in additional metadata
                    if !first_release.id.is_empty() {
                        metadata.additional.insert(
                            "release_mbid".to_string(),
                            ConfidenceValue::new(
                                first_release.id.clone(),
                                self.base_confidence,
                                "MusicBrainz",
                            ),
                        );
                    }
                }
            }
        }

        // Recording MBID
        if !recording.id.is_empty() {
            metadata.recording_mbid = Some(ConfidenceValue::new(
                recording.id.clone(),
                self.base_confidence,
                "MusicBrainz",
            ));
        }

        // Additional metadata
        let mut additional = HashMap::new();

        // Length (milliseconds)
        if let Some(length) = recording.length {
            additional.insert(
                "length_ms".to_string(),
                ConfidenceValue::new(length.to_string(), self.base_confidence, "MusicBrainz"),
            );
        }

        // Disambiguation
        if let Some(ref disambiguation) = recording.disambiguation {
            if !disambiguation.is_empty() {
                additional.insert(
                    "disambiguation".to_string(),
                    ConfidenceValue::new(
                        disambiguation.clone(),
                        self.base_confidence,
                        "MusicBrainz",
                    ),
                );
            }
        }

        metadata.additional = additional;
        metadata
    }

    /// Extract metadata using a known Recording MBID
    ///
    /// **[PLAN024 Option 3]** This method is used in the two-pass pipeline:
    /// - Pass 1: Parallel extraction (this extractor returns NotAvailable)
    /// - Fusion: Bayesian confidence selection of Recording MBID
    /// - Pass 2: This method queries MusicBrainz with fused MBID
    ///
    /// # Arguments
    /// * `mbid` - MusicBrainz Recording ID (from Pass 1 fusion)
    /// * `ctx` - Passage context (for logging only)
    ///
    /// # Returns
    /// * `ExtractionResult` with rich metadata from MusicBrainz API
    pub async fn extract_with_mbid(
        &self,
        mbid: &str,
        ctx: &PassageContext,
    ) -> Result<ExtractionResult, ExtractionError> {
        debug!(
            passage_id = %ctx.passage_id,
            mbid = %mbid,
            "Extracting metadata from MusicBrainz with known MBID (Pass 2)"
        );

        // Query MusicBrainz API with MBID
        let metadata = self.query_recording(mbid).await?;

        debug!(
            passage_id = %ctx.passage_id,
            mbid = %mbid,
            title = ?metadata.title.as_ref().map(|t| &t.value),
            artist = ?metadata.artist.as_ref().map(|a| &a.value),
            "MusicBrainz extraction complete"
        );

        // Wrap in ExtractionResult
        Ok(ExtractionResult {
            metadata: Some(metadata),
            identity: None,  // MBID already known from Pass 1
            musical_flavor: None,
        })
    }
}

impl Default for MusicBrainzClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceExtractor for MusicBrainzClient {
    fn name(&self) -> &'static str {
        "MusicBrainz"
    }

    fn base_confidence(&self) -> f32 {
        self.base_confidence
    }

    async fn extract(&self, ctx: &PassageContext) -> Result<ExtractionResult, ExtractionError> {
        debug!(
            passage_id = %ctx.passage_id,
            file_path = ?ctx.file_path,
            "MusicBrainz extractor in Pass 1 (no MBID available yet)"
        );

        // Pass 1: No MBID available yet (comes from other extractors)
        // Pipeline will call extract_with_mbid() in Pass 2 after fusion
        Err(ExtractionError::NotAvailable(
            "Recording MBID not yet available (Pass 2 will use extract_with_mbid)".to_string(),
        ))
    }
}

// ============================================================================
// MusicBrainz API Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct MusicBrainzRecording {
    id: String,
    title: String,
    length: Option<u64>, // milliseconds
    disambiguation: Option<String>,
    #[serde(rename = "artist-credit")]
    artist_credit: Option<Vec<ArtistCredit>>,
    releases: Option<Vec<Release>>,
}

#[derive(Debug, Deserialize)]
struct ArtistCredit {
    name: String,
    artist: Artist,
}

#[derive(Debug, Deserialize)]
struct Artist {
    id: String,
    #[allow(dead_code)] // Deserialized from API but not directly accessed
    name: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    id: String,
    title: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn test_client_name() {
        let client = MusicBrainzClient::new();
        assert_eq!(client.name(), "MusicBrainz");
    }

    #[test]
    fn test_default_confidence() {
        let client = MusicBrainzClient::new();
        assert_eq!(client.base_confidence(), 0.9);
    }

    #[test]
    fn test_default_trait() {
        let client = MusicBrainzClient::default();
        assert_eq!(client.base_confidence(), 0.9);
    }

    #[tokio::test]
    async fn test_extract_returns_not_available() {
        // Current implementation returns NotAvailable because it needs
        // MBID from upstream extractors (ID3 or AcoustID)
        let client = MusicBrainzClient::new();
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: None,
            sample_rate: Some(44100),
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        let result = client.extract(&ctx).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExtractionError::NotAvailable(_)));
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let client = MusicBrainzClient::new();

        // First request should not sleep
        let start = Instant::now();
        client.enforce_rate_limit().await;
        let first_elapsed = start.elapsed();
        assert!(first_elapsed.as_millis() < 100, "First request should be immediate");

        // Second request within 1 second should sleep
        let start = Instant::now();
        client.enforce_rate_limit().await;
        let second_elapsed = start.elapsed();
        assert!(
            second_elapsed.as_millis() >= 900,
            "Second request should sleep ~1s, got {}ms",
            second_elapsed.as_millis()
        );
    }

    // Note: Testing actual MusicBrainz API queries requires:
    // 1. Network connectivity
    // 2. Valid Recording MBIDs
    // 3. Respecting rate limits
    // These tests are covered in integration tests with network mocking
}

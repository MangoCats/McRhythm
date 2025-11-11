//! AcoustID Client (Tier 1)
//!
//! Queries AcoustID API to resolve audio fingerprints to MusicBrainz Recording IDs.
//! Uses fingerprints from TASK-006 (Chromaprint Analyzer) to identify recordings.
//!
//! # Implementation
//! - TASK-007: AcoustID Client (PLAN024)
//! - Confidence: 0.8-0.95 (based on AcoustID match score)
//!
//! # Architecture
//! Implements `SourceExtractor` trait for integration with parallel extraction pipeline.
//! Depends on Chromaprint fingerprint from TASK-006 for identity resolution.
//!
//! # API Reference
//! - Endpoint: https://api.acoustid.org/v2/lookup
//! - Documentation: https://acoustid.org/webservice

use crate::types::{
    ExtractionError, ExtractionResult, IdentityExtraction, PassageContext, SourceExtractor,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, warn};

/// AcoustID API endpoint
const ACOUSTID_API_URL: &str = "https://api.acoustid.org/v2/lookup";

/// Default timeout for AcoustID API requests
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Default AcoustID API client key (public demo key)
/// Production deployments should use their own API key via configuration
const DEFAULT_CLIENT_KEY: &str = "8XaBELgH";

/// AcoustID Client
///
/// Queries AcoustID API to resolve audio fingerprints to MusicBrainz Recording IDs.
/// Returns highest-scoring match with confidence based on AcoustID score.
///
/// # Confidence Scoring
/// AcoustID returns scores 0.0-1.0:
/// - 0.9+ → confidence 0.95 (excellent match)
/// - 0.8-0.9 → confidence 0.90 (very good match)
/// - 0.7-0.8 → confidence 0.85 (good match)
/// - 0.6-0.7 → confidence 0.80 (acceptable match)
/// - <0.6 → reject (insufficient confidence)
///
/// # Requirements
/// - Requires fingerprint from Chromaprint Analyzer (TASK-006)
/// - Requires audio duration in seconds
/// - Requires network connectivity to AcoustID API
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::extractors::acoustid_client::AcoustIDClient;
/// use wkmp_ai::types::{SourceExtractor, PassageContext};
///
/// let client = AcoustIDClient::new("your_api_key".to_string());
/// let result = client.extract(&passage_ctx).await?;
///
/// if let Some(identity) = result.identity {
///     println!("Recording MBID: {} (confidence: {})",
///         identity.recording_mbid, identity.confidence);
/// }
/// ```
pub struct AcoustIDClient {
    /// HTTP client for API requests
    http_client: Client,
    /// AcoustID API client key
    api_key: String,
    /// Base confidence (0.8-0.95 depending on match quality)
    base_confidence: f32,
    /// Minimum acceptable AcoustID score (default: 0.6)
    min_score: f32,
}

impl AcoustIDClient {
    /// Create new AcoustID client with custom API key
    pub fn new(api_key: String) -> Self {
        Self {
            http_client: Client::builder()
                .timeout(DEFAULT_TIMEOUT)
                .build()
                .expect("Failed to create HTTP client"),
            api_key,
            base_confidence: 0.8,
            min_score: 0.6,
        }
    }

    /// Create new AcoustID client with default demo API key
    ///
    /// # Note
    /// The demo API key has rate limits. Production deployments should obtain
    /// their own API key from https://acoustid.org/new-application
    pub fn with_default_key() -> Self {
        Self::new(DEFAULT_CLIENT_KEY.to_string())
    }

    /// Set minimum acceptable AcoustID score (default: 0.6)
    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = min_score.clamp(0.0, 1.0);
        self
    }

    /// **[AIA-SEC-030]** Validate AcoustID API key with a test request
    ///
    /// Makes a minimal API call to check if the key is valid.
    /// Returns Ok(()) if valid, Err if invalid or network error.
    ///
    /// **Strategy:** Send a request with intentionally invalid fingerprint.
    /// - If API key is invalid: AcoustID returns error code 5 or 6
    /// - If API key is valid: AcoustID returns error code 3 (invalid fingerprint)
    /// We consider error code 3 as SUCCESS because it proves the API key was accepted.
    pub async fn validate_api_key(&self) -> Result<(), ExtractionError> {
        debug!("Validating AcoustID API key");

        // Make a test request with intentionally invalid fingerprint
        // Valid API key → Error 3 (invalid fingerprint) = API key accepted
        // Invalid API key → Error 5 (invalid API key) or Error 6 (invalid format)
        let response = self
            .http_client
            .post(ACOUSTID_API_URL)
            .form(&[
                ("client", self.api_key.as_str()),
                ("duration", "1"),
                ("fingerprint", "INVALID"), // Intentionally invalid to test API key
            ])
            .send()
            .await
            .map_err(|e| ExtractionError::Network(format!("API key validation request failed: {}", e)))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Parse JSON response
        let json_response: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ExtractionError::Api(format!("Failed to parse AcoustID response: {}", e)))?;

        // Check error code
        if let Some(error_code) = json_response.get("error").and_then(|e| e.get("code")).and_then(|c| c.as_i64()) {
            match error_code {
                3 => {
                    // Error 3 = "invalid fingerprint"
                    // This means the API key was ACCEPTED (validated before fingerprint check)
                    debug!("AcoustID API key is valid (error code 3 = fingerprint rejected, key accepted)");
                    return Ok(());
                }
                5 => {
                    // Error 5 = "invalid API key"
                    return Err(ExtractionError::Api(
                        "AcoustID API key is invalid (error code 5)".to_string()
                    ));
                }
                6 => {
                    // Error 6 = "invalid format" (could be API key format issue)
                    return Err(ExtractionError::Api(
                        "AcoustID API key has invalid format (error code 6)".to_string()
                    ));
                }
                _ => {
                    // Other error codes - treat as validation failure
                    return Err(ExtractionError::Api(format!(
                        "AcoustID validation failed with error code {}: {}",
                        error_code,
                        json_response.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).unwrap_or("unknown error")
                    )));
                }
            }
        }

        // No error in response - should not happen with invalid fingerprint, but accept it
        if status.is_success() {
            debug!("AcoustID API key is valid (success response)");
            return Ok(());
        }

        // Unexpected response format
        Err(ExtractionError::Api(format!(
            "Unexpected AcoustID response (status {}): {}",
            status, body
        )))
    }

    /// Query AcoustID API with fingerprint
    ///
    /// # Arguments
    /// * `fingerprint` - Base64-encoded Chromaprint fingerprint
    /// * `duration` - Audio duration in seconds
    ///
    /// # Returns
    /// Best matching Recording MBID with confidence score
    ///
    /// # Errors
    /// Returns error if:
    /// - Network request fails
    /// - API returns error
    /// - No matches above minimum score threshold
    async fn query_acoustid(
        &self,
        fingerprint: &str,
        duration: f32,
    ) -> Result<IdentityExtraction, ExtractionError> {
        debug!(
            fingerprint_length = fingerprint.len(),
            duration = duration,
            "Querying AcoustID API"
        );

        // Build request
        let response = self
            .http_client
            .post(ACOUSTID_API_URL)
            .form(&[
                ("client", self.api_key.as_str()),
                ("duration", &duration.to_string()),
                ("fingerprint", fingerprint),
                ("meta", "recordings"), // Request MusicBrainz Recording IDs
            ])
            .send()
            .await
            .map_err(|e| ExtractionError::Network(format!("AcoustID API request failed: {}", e)))?;

        // Check status
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ExtractionError::Api(format!(
                "AcoustID API returned error {}: {}",
                status, body
            )));
        }

        // Parse response
        let acoustid_response: AcoustIDResponse = response.json().await.map_err(|e| {
            ExtractionError::Parse(format!("Failed to parse AcoustID response: {}", e))
        })?;

        // Check for API-level errors
        if acoustid_response.status != "ok" {
            return Err(ExtractionError::Api(format!(
                "AcoustID API error: {}",
                acoustid_response
                    .error
                    .map_or("Unknown error".to_string(), |e| e.message)
            )));
        }

        // Find best match
        let best_match = acoustid_response
            .results
            .into_iter()
            .filter(|r| r.score >= self.min_score)
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));

        let Some(result) = best_match else {
            return Err(ExtractionError::NotAvailable(
                "No AcoustID matches above minimum score threshold".to_string(),
            ));
        };

        // Extract Recording MBID from first recording
        let Some(recordings) = result.recordings else {
            return Err(ExtractionError::NotAvailable(
                "AcoustID match has no MusicBrainz recordings".to_string(),
            ));
        };

        let Some(recording) = recordings.first() else {
            return Err(ExtractionError::NotAvailable(
                "AcoustID recordings array empty".to_string(),
            ));
        };

        // Map AcoustID score to confidence
        let confidence = map_score_to_confidence(result.score);

        debug!(
            recording_mbid = %recording.id,
            acoustid_score = result.score,
            confidence = confidence,
            "AcoustID match found"
        );

        Ok(IdentityExtraction {
            recording_mbid: recording.id.clone(),
            confidence,
            source: "AcoustID".to_string(),
        })
    }
}

impl Default for AcoustIDClient {
    fn default() -> Self {
        Self::with_default_key()
    }
}

#[async_trait]
impl SourceExtractor for AcoustIDClient {
    fn name(&self) -> &'static str {
        "AcoustID"
    }

    fn base_confidence(&self) -> f32 {
        self.base_confidence
    }

    async fn extract(&self, ctx: &PassageContext) -> Result<ExtractionResult, ExtractionError> {
        debug!(
            passage_id = %ctx.passage_id,
            file_path = ?ctx.file_path,
            "Extracting identity via AcoustID"
        );

        // This extractor requires:
        // 1. Audio samples (to compute duration)
        // 2. Sample rate (to compute duration)
        // 3. Fingerprint (from Chromaprint Analyzer - TASK-006)
        //
        // Note: In the parallel extraction pipeline, Chromaprint runs concurrently
        // with AcoustID. For now, we return NotAvailable if fingerprint not yet ready.
        // The fusion layer (TASK-012) will handle combining results from multiple extractors.

        let Some(ref samples) = ctx.audio_samples else {
            warn!(
                passage_id = %ctx.passage_id,
                "No audio samples available for duration calculation"
            );
            return Err(ExtractionError::Internal(
                "No audio samples available".to_string(),
            ));
        };

        let Some(sample_rate) = ctx.sample_rate else {
            return Err(ExtractionError::Internal(
                "Sample rate not specified".to_string(),
            ));
        };

        let Some(num_channels) = ctx.num_channels else {
            return Err(ExtractionError::Internal(
                "Number of channels not specified".to_string(),
            ));
        };

        // Compute duration from passage timing (SPEC017 tick-based)
        // TICK_RATE = 28,224,000 Hz
        const TICK_RATE: i64 = 28_224_000;
        let duration_ticks = ctx.end_time_ticks - ctx.start_time_ticks;
        let duration_seconds = duration_ticks as f32 / TICK_RATE as f32;

        debug!(
            passage_id = %ctx.passage_id,
            duration_seconds = duration_seconds,
            sample_count = samples.len(),
            sample_rate = sample_rate,
            channels = num_channels,
            "Computing Chromaprint fingerprint for AcoustID lookup"
        );

        // Generate fingerprint inline
        // TODO: In production, this should coordinate with Chromaprint Analyzer
        // to avoid duplicate fingerprint generation
        let fingerprint = {
            use crate::ffi::chromaprint::ChromaprintContext;
            let mut ctx_chromaprint = ChromaprintContext::new().map_err(|e| {
                ExtractionError::NotAvailable(format!("Chromaprint library not available: {}", e))
            })?;

            ctx_chromaprint
                .generate_fingerprint(samples, sample_rate, num_channels)
                .map_err(|e| {
                    ExtractionError::Internal(format!("Fingerprint generation failed: {}", e))
                })?
            // ctx_chromaprint dropped here (before await)
        };

        // Query AcoustID
        let identity = self.query_acoustid(&fingerprint, duration_seconds).await?;

        debug!(
            passage_id = %ctx.passage_id,
            recording_mbid = %identity.recording_mbid,
            confidence = identity.confidence,
            "AcoustID extraction complete"
        );

        Ok(ExtractionResult {
            metadata: None,
            identity: Some(identity),
            musical_flavor: None,
        })
    }
}

// ============================================================================
// AcoustID API Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct AcoustIDResponse {
    status: String,
    #[serde(default)]
    results: Vec<AcoustIDResult>,
    error: Option<AcoustIDError>,
}

#[derive(Debug, Deserialize)]
struct AcoustIDResult {
    score: f32,
    #[allow(dead_code)] // Deserialized from API but not directly accessed
    id: String, // AcoustID fingerprint ID
    recordings: Option<Vec<AcoustIDRecording>>,
}

#[derive(Debug, Deserialize)]
struct AcoustIDRecording {
    id: String, // MusicBrainz Recording MBID
}

#[derive(Debug, Deserialize)]
struct AcoustIDError {
    message: String,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// **[AIA-SEC-030]** Check if error is "invalid API key" from AcoustID
///
/// Used to detect when user needs to be prompted for valid API key.
pub fn is_invalid_api_key_error(error: &ExtractionError) -> bool {
    match error {
        ExtractionError::Api(msg) => {
            msg.contains("400 Bad Request") && msg.contains("invalid API key")
        }
        _ => false,
    }
}

/// **[AIA-SEC-030]** Validate an AcoustID API key with a test request
///
/// Makes a minimal API call to check if the key is valid.
/// Returns Ok(()) if valid, Err with description if invalid.
pub async fn validate_acoustid_key(api_key: &str) -> Result<(), String> {
    let client = AcoustIDClient::new(api_key.to_string());
    match client.validate_api_key().await {
        Ok(()) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Map AcoustID score to confidence value
///
/// AcoustID scores range 0.0-1.0 (higher = better match)
/// Map to confidence:
/// - 0.9+ → 0.95 (excellent)
/// - 0.8-0.9 → 0.90 (very good)
/// - 0.7-0.8 → 0.85 (good)
/// - 0.6-0.7 → 0.80 (acceptable)
/// - <0.6 → reject (handled by min_score filter)
fn map_score_to_confidence(score: f32) -> f32 {
    if score >= 0.9 {
        0.95
    } else if score >= 0.8 {
        0.90
    } else if score >= 0.7 {
        0.85
    } else {
        0.80 // 0.6-0.7 range
    }
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
        let client = AcoustIDClient::with_default_key();
        assert_eq!(client.name(), "AcoustID");
    }

    #[test]
    fn test_default_confidence() {
        let client = AcoustIDClient::with_default_key();
        assert_eq!(client.base_confidence(), 0.8);
    }

    #[test]
    fn test_custom_api_key() {
        let client = AcoustIDClient::new("custom_key".to_string());
        assert_eq!(client.api_key, "custom_key");
    }

    #[test]
    fn test_min_score_clamping() {
        let client = AcoustIDClient::with_default_key().with_min_score(1.5);
        assert_eq!(client.min_score, 1.0, "Should clamp to 1.0");

        let client = AcoustIDClient::with_default_key().with_min_score(-0.5);
        assert_eq!(client.min_score, 0.0, "Should clamp to 0.0");
    }

    #[test]
    fn test_score_to_confidence_mapping() {
        assert_eq!(map_score_to_confidence(0.95), 0.95, "Excellent match");
        assert_eq!(map_score_to_confidence(0.85), 0.90, "Very good match");
        assert_eq!(map_score_to_confidence(0.75), 0.85, "Good match");
        assert_eq!(map_score_to_confidence(0.65), 0.80, "Acceptable match");
    }

    #[test]
    fn test_default_trait() {
        let client = AcoustIDClient::default();
        assert_eq!(client.base_confidence(), 0.8);
    }

    #[tokio::test]
    async fn test_extract_missing_samples() {
        let client = AcoustIDClient::with_default_key();
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: None, // No samples
            sample_rate: Some(44100),
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        let result = client.extract(&ctx).await;
        assert!(result.is_err(), "Should fail when no audio samples provided");
    }

    #[tokio::test]
    async fn test_extract_missing_sample_rate() {
        let client = AcoustIDClient::with_default_key();
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: Some(vec![0.0f32; 44100]),
            sample_rate: None, // Missing
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        let result = client.extract(&ctx).await;
        assert!(result.is_err(), "Should fail when sample rate not specified");
    }

    // Note: Testing actual AcoustID API queries requires:
    // 1. Network connectivity
    // 2. Valid API key
    // 3. Real audio fingerprints
    // These tests are covered in integration tests with network mocking
}

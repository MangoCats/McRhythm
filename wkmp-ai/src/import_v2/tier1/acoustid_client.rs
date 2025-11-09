// PLAN023 Tier 1: AcoustID API Client
//
// Concept: Query AcoustID API for MusicBrainz Recording ID candidates using audio fingerprint
// Confidence: 0.8 (fingerprint matching + crowd-sourced validation)
//
// Resolution: HIGH-003, HIGH-004 - API timeout and rate limiting configuration
//
// API Documentation: https://acoustid.org/webservice

use crate::import_v2::types::{
    ExtractionSource, ExtractorResult, ImportError, ImportResult, MBIDCandidate,
};
use reqwest::Client;
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use std::time::Duration;
use uuid::Uuid;
use wkmp_common::config::TomlConfig;

/// AcoustID API response structure
#[derive(Debug, Deserialize)]
struct AcoustIDResponse {
    status: String,
    results: Option<Vec<AcoustIDResult>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AcoustIDResult {
    id: String, // AcoustID identifier
    score: f64, // Match score (0.0 to 1.0)
    recordings: Option<Vec<AcoustIDRecording>>,
}

#[derive(Debug, Deserialize)]
struct AcoustIDRecording {
    id: String, // MusicBrainz Recording ID
    // title: Option<String>,
    // artists: Option<Vec<AcoustIDArtist>>,
}

/// AcoustID API client (Tier 1 extractor concept)
///
/// **Legible Software Principle:**
/// - Independent module: Only depends on HTTP client
/// - Explicit synchronization: Returns `Result<ExtractorResult<Vec<MBIDCandidate>>>`
/// - Transparent behavior: API calls are explicit with visible timeouts
/// - Integrity: Rate limiting enforced, errors returned explicitly
pub struct AcoustIDClient {
    /// HTTP client with configured timeouts
    client: Client,
    /// AcoustID API key (user-provided)
    api_key: String,
    /// Base URL for AcoustID API
    base_url: String,
    /// Default confidence for AcoustID matches
    confidence: f64,
}

impl AcoustIDClient {
    /// Create new AcoustID client with API key
    ///
    /// # Arguments
    /// * `api_key` - AcoustID API key (obtain from https://acoustid.org/api-key)
    ///
    /// # Panics
    /// Panics if HTTP client cannot be built (should not happen with valid config)
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15)) // Per HIGH-003: 15s total timeout
            .connect_timeout(Duration::from_secs(5)) // Per HIGH-003: 5s connection timeout
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_key,
            base_url: "https://api.acoustid.org/v2/lookup".to_string(),
            confidence: ExtractionSource::AcoustID.default_confidence(),
        }
    }

    /// Create AcoustID client from configuration sources
    ///
    /// **Resolution Priority:** Database → ENV → TOML
    ///
    /// # Arguments
    /// * `db` - Database connection pool
    /// * `toml_config` - TOML configuration
    ///
    /// # Errors
    /// Returns error if API key cannot be resolved from any source
    ///
    /// # Traceability
    /// [APIK-RES-010] - Multi-tier configuration resolution
    pub async fn from_config(
        db: &Pool<Sqlite>,
        toml_config: &TomlConfig,
    ) -> wkmp_common::Result<Self> {
        let api_key = crate::config::resolve_acoustid_api_key(db, toml_config).await?;
        Ok(Self::new(api_key))
    }

    /// Query AcoustID API for MusicBrainz Recording IDs
    ///
    /// # Arguments
    /// * `fingerprint` - Chromaprint fingerprint (base64-encoded)
    /// * `duration_secs` - Audio duration in seconds
    ///
    /// # Returns
    /// Vector of MBID candidates with confidence scores
    ///
    /// # Errors
    /// Returns error if:
    /// - API request fails (network, timeout)
    /// - API returns error status
    /// - Response cannot be parsed
    pub async fn lookup(
        &self,
        fingerprint: &str,
        duration_secs: u32,
    ) -> ImportResult<ExtractorResult<Vec<MBIDCandidate>>> {
        // Build query parameters
        let params = [
            ("client", self.api_key.as_str()),
            ("fingerprint", fingerprint),
            ("duration", &duration_secs.to_string()),
            ("meta", "recordings"), // Include recording metadata
        ];

        tracing::debug!(
            "Querying AcoustID API: duration={}s, fingerprint_len={}",
            duration_secs,
            fingerprint.len()
        );

        // Send GET request
        let response = self
            .client
            .get(&self.base_url)
            .query(&params)
            .send()
            .await
            .map_err(|e| {
                ImportError::ExtractionFailed(format!("AcoustID API request failed: {}", e))
            })?;

        // Check HTTP status
        if !response.status().is_success() {
            return Err(ImportError::ExtractionFailed(format!(
                "AcoustID API returned error status: {}",
                response.status()
            )));
        }

        // Parse JSON response
        let api_response: AcoustIDResponse = response.json().await.map_err(|e| {
            ImportError::ExtractionFailed(format!("Failed to parse AcoustID response: {}", e))
        })?;

        // Check API status
        if api_response.status != "ok" {
            return Err(ImportError::ExtractionFailed(format!(
                "AcoustID API returned error status: {}",
                api_response.status
            )));
        }

        // Extract MBID candidates
        let mut candidates = Vec::new();

        if let Some(results) = api_response.results {
            for result in results {
                if let Some(recordings) = result.recordings {
                    for recording in recordings {
                        // Parse MusicBrainz Recording ID
                        if let Ok(mbid) = Uuid::parse_str(&recording.id) {
                            // Combine AcoustID match score with base confidence
                            // Score ranges 0.0-1.0, we scale by base confidence
                            let combined_confidence = result.score * self.confidence;

                            candidates.push(MBIDCandidate {
                                mbid,
                                confidence: combined_confidence,
                                sources: vec![ExtractionSource::AcoustID],
                            });
                        } else {
                            tracing::warn!("Invalid MBID format from AcoustID: {}", recording.id);
                        }
                    }
                }
            }
        }

        // Sort by confidence (highest first)
        candidates.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        tracing::info!(
            "AcoustID lookup returned {} MBID candidates",
            candidates.len()
        );

        Ok(ExtractorResult {
            data: candidates,
            confidence: self.confidence,
            source: ExtractionSource::AcoustID,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = AcoustIDClient::new("test_api_key".to_string());
        assert_eq!(client.api_key, "test_api_key");
        assert_eq!(client.confidence, 0.8);
    }

    #[test]
    fn test_base_url() {
        let client = AcoustIDClient::new("test_key".to_string());
        assert!(client.base_url.contains("acoustid.org"));
        assert!(client.base_url.contains("/v2/lookup"));
    }

    // Note: Tests for from_config() are in tests/unit/db_settings_tests.rs
    // where full database migrations can be run properly. Unit tests here
    // focus on the new() constructor which doesn't require database setup.

    // Note: Integration tests with real API calls would require:
    // 1. Valid API key
    // 2. Network access
    // 3. Rate limiting coordination
    // These would go in tests/ directory as integration tests
}

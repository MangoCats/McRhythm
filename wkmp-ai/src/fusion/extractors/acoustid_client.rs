// AcoustID API Client
//
// PLAN023: REQ-AI-021, REQ-AI-031 - Lookup Recording MBID via AcoustID fingerprint
// Confidence: 0.85-0.99 (based on AcoustID score)

use super::chromaprint_analyzer;
use crate::fusion::extractors::Extractor;
use crate::fusion::{ExtractionResult, IdentityExtraction, Confidence};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, warn};

const ACOUSTID_API_URL: &str = "https://api.acoustid.org/v2/lookup";

#[derive(Debug, Deserialize)]
struct AcoustIdResponse {
    status: String,
    results: Option<Vec<AcoustIdResult>>,
}

#[derive(Debug, Deserialize)]
struct AcoustIdResult {
    score: f64,
    recordings: Option<Vec<AcoustIdRecording>>,
}

#[derive(Debug, Deserialize)]
struct AcoustIdRecording {
    id: String, // MusicBrainz Recording MBID
}

pub struct AcoustIdClient {
    api_key: String,
    client: reqwest::Client,
    rate_limiter: governor::RateLimiter<
        governor::state::NotKeyed,
        governor::state::InMemoryState,
        governor::clock::DefaultClock,
    >,
}

impl AcoustIdClient {
    pub fn new(api_key: String) -> Self {
        // AcoustID rate limit: 3 requests/second
        // Safe: 3 is always non-zero
        let quota = governor::Quota::per_second(std::num::NonZeroU32::new(3).unwrap());
        let rate_limiter = governor::RateLimiter::direct(quota);

        Self {
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client (system error)"),
            rate_limiter,
        }
    }

    /// Check if API key is configured
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

#[async_trait]
impl Extractor for AcoustIdClient {
    fn source_id(&self) -> &'static str {
        "AcoustID"
    }

    async fn extract(
        &self,
        file_path: &Path,
        start_seconds: f64,
        end_seconds: f64,
    ) -> Result<ExtractionResult> {
        debug!(
            "Looking up AcoustID: {} ({:.2}s - {:.2}s)",
            file_path.display(),
            start_seconds,
            end_seconds
        );

        // Generate Chromaprint fingerprint
        let fingerprint = chromaprint_analyzer::generate_fingerprint(file_path, start_seconds, end_seconds)
            .await
            .context("Failed to generate fingerprint")?;

        let duration = end_seconds - start_seconds;

        // Rate limit API calls
        self.rate_limiter.until_ready().await;

        // POST to AcoustID API
        let params = [
            ("client", self.api_key.as_str()),
            ("duration", &duration.to_string()),
            ("fingerprint", &fingerprint),
            ("meta", "recordings"),
        ];

        debug!("Querying AcoustID API (duration: {:.1}s, fingerprint length: {})", duration, fingerprint.len());

        let response = self
            .client
            .post(ACOUSTID_API_URL)
            .form(&params)
            .send()
            .await
            .context("AcoustID API request failed")?;

        if !response.status().is_success() {
            anyhow::bail!("AcoustID API returned error: {}", response.status());
        }

        let api_response: AcoustIdResponse = response
            .json()
            .await
            .context("Failed to parse AcoustID response")?;

        if api_response.status != "ok" {
            anyhow::bail!("AcoustID API returned status: {}", api_response.status);
        }

        // Extract best match
        let results = api_response.results.unwrap_or_default();
        if results.is_empty() {
            debug!("No AcoustID matches found");
            return Ok(ExtractionResult {
                source: self.source_id().to_string(),
                confidence: 0.0,
                timestamp: chrono::Utc::now().timestamp(),
                metadata: None,
                flavor: None,
                identity: None,
            });
        }

        // Get first result (highest score)
        let best_result = &results[0];
        let score = best_result.score;

        // Extract MBID
        let mbid = best_result
            .recordings
            .as_ref()
            .and_then(|recs| recs.first())
            .map(|rec| rec.id.clone());

        if let Some(mbid) = mbid {
            debug!("AcoustID match found: {} (score: {:.3})", mbid, score);

            // Create context for identity extraction
            let mut context = HashMap::new();
            context.insert("acoustid_score".to_string(), serde_json::json!(score));
            context.insert("duration".to_string(), serde_json::json!(duration));

            Ok(ExtractionResult {
                source: self.source_id().to_string(),
                confidence: score, // AcoustID score is 0.0-1.0
                timestamp: chrono::Utc::now().timestamp(),
                metadata: None,
                flavor: None,
                identity: Some(IdentityExtraction {
                    recording_mbid: mbid,
                    confidence: score,
                    context: Some(context),
                }),
            })
        } else {
            warn!("AcoustID result has no recordings");
            Ok(ExtractionResult {
                source: self.source_id().to_string(),
                confidence: 0.0,
                timestamp: chrono::Utc::now().timestamp(),
                metadata: None,
                flavor: None,
                identity: None,
            })
        }
    }

    fn is_available(&self) -> bool {
        self.is_configured()
    }

    fn confidence_range(&self) -> (Confidence, Confidence) {
        (0.85, 0.99) // AcoustID scores are typically high when matches found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_configured() {
        let client = AcoustIdClient::new("test-key".to_string());
        assert!(client.is_configured());

        let client_empty = AcoustIdClient::new(String::new());
        assert!(!client_empty.is_configured());
    }

    #[test]
    fn test_source_id() {
        let client = AcoustIdClient::new("key".to_string());
        assert_eq!(client.source_id(), "AcoustID");
    }
}


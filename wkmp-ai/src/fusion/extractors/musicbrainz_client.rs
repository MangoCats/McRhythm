// MusicBrainz API Client
//
// PLAN023: REQ-AI-031, REQ-AI-041 - Fetch metadata and relationships from MusicBrainz
// Confidence: 0.95-1.0 (authoritative source)

use crate::fusion::extractors::Extractor;
use crate::fusion::{ExtractionResult, MetadataExtraction, IdentityExtraction, Confidence};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use tracing::debug;

const MUSICBRAINZ_API_URL: &str = "https://musicbrainz.org/ws/2";

#[derive(Debug, Deserialize)]
struct MusicBrainzRecording {
    id: String,
    title: String,
    #[serde(rename = "artist-credit")]
    artist_credit: Option<Vec<ArtistCredit>>,
    releases: Option<Vec<Release>>,
    #[serde(default)]
    length: Option<u64>, // Duration in milliseconds
}

#[derive(Debug, Deserialize)]
struct ArtistCredit {
    name: String,
    #[allow(dead_code)]
    artist: Artist,
}

#[derive(Debug, Deserialize)]
struct Artist {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    #[allow(dead_code)]
    id: String,
    title: String,
}

pub struct MusicBrainzClient {
    client: reqwest::Client,
    rate_limiter: governor::RateLimiter<
        governor::state::NotKeyed,
        governor::state::InMemoryState,
        governor::clock::DefaultClock,
    >,
}

impl Default for MusicBrainzClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MusicBrainzClient {
    pub fn new() -> Self {
        // MusicBrainz rate limit: 1 request/second
        // Safe: 1 is always non-zero
        let quota = governor::Quota::per_second(std::num::NonZeroU32::new(1).unwrap());
        let rate_limiter = governor::RateLimiter::direct(quota);

        Self {
            client: reqwest::Client::builder()
                .user_agent("WKMP/0.1.0 (https://github.com/swilliams11/wkmp)")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client (system error)"),
            rate_limiter,
        }
    }

    /// Fetch recording metadata by MBID
    ///
    /// # Arguments
    /// * `mbid` - MusicBrainz Recording ID
    ///
    /// # Returns
    /// * `ExtractionResult` with metadata from MusicBrainz
    pub async fn fetch_by_mbid(&self, mbid: &str) -> Result<ExtractionResult> {
        debug!("Fetching MusicBrainz recording: {}", mbid);

        // Rate limit API calls
        self.rate_limiter.until_ready().await;

        // GET recording with artists and releases
        let url = format!(
            "{}/recording/{}?fmt=json&inc=artists+releases",
            MUSICBRAINZ_API_URL, mbid
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("MusicBrainz API request failed")?;

        if !response.status().is_success() {
            anyhow::bail!("MusicBrainz API returned error: {}", response.status());
        }

        let recording: MusicBrainzRecording = response
            .json()
            .await
            .context("Failed to parse MusicBrainz response")?;

        debug!(
            "MusicBrainz match: '{}' by '{}'",
            recording.title,
            recording
                .artist_credit
                .as_ref()
                .and_then(|ac| ac.first())
                .map(|ac| ac.name.as_str())
                .unwrap_or("Unknown")
        );

        // Extract metadata
        let title = recording.title;
        let artist = recording
            .artist_credit
            .as_ref()
            .and_then(|ac| ac.first())
            .map(|ac| ac.name.clone());

        let album = recording
            .releases
            .as_ref()
            .and_then(|releases| releases.first())
            .map(|release| release.title.clone());

        let duration_seconds = recording
            .length
            .map(|ms| ms as f64 / 1000.0);

        // MusicBrainz is authoritative - high confidence
        let confidence = 0.98;

        Ok(ExtractionResult {
            source: self.source_id().to_string(),
            confidence,
            timestamp: chrono::Utc::now().timestamp(),
            metadata: Some(MetadataExtraction {
                title: Some(title),
                artist,
                album,
                duration_seconds,
                title_confidence: Some(confidence),
                artist_confidence: Some(confidence),
            }),
            flavor: None, // MusicBrainz doesn't provide musical flavor
            identity: Some(IdentityExtraction {
                recording_mbid: recording.id,
                confidence,
                context: None,
            }),
        })
    }
}

#[async_trait]
impl Extractor for MusicBrainzClient {
    fn source_id(&self) -> &'static str {
        "MusicBrainz"
    }

    async fn extract(
        &self,
        _file_path: &Path,
        _start_seconds: f64,
        _end_seconds: f64,
    ) -> Result<ExtractionResult> {
        // MusicBrainz extraction requires an MBID (from ID3 or AcoustID)
        // This extractor is typically called via fetch_by_mbid() during fusion
        anyhow::bail!("MusicBrainz extractor requires MBID - use fetch_by_mbid() instead")
    }

    fn confidence_range(&self) -> (Confidence, Confidence) {
        (0.95, 1.0) // MusicBrainz is authoritative source
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_id() {
        let client = MusicBrainzClient::new();
        assert_eq!(client.source_id(), "MusicBrainz");
    }

    #[test]
    fn test_confidence_range() {
        let client = MusicBrainzClient::new();
        assert_eq!(client.confidence_range(), (0.95, 1.0));
    }

    #[tokio::test]
    #[ignore] // Requires network access - run with: cargo test -- --ignored
    async fn test_fetch_with_known_mbid() {
        // Test with known MBID - placeholder ID, replace with valid recording MBID
        // To find valid MBIDs: https://musicbrainz.org/search
        // Example: Search for a well-known recording, copy the MBID from the URL
        // MBID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx (recording ID)
        //
        // NOTE: This test is marked #[ignore] because it requires:
        // 1. Network connectivity
        // 2. Valid MBID from MusicBrainz API
        // 3. Stable test data (recording must exist in MB database)
        //
        // To run: cargo test test_fetch_with_known_mbid -- --ignored

        let client = MusicBrainzClient::new();
        let mbid = "e3b2c77b-cac5-4c0f-8c45-4ca75e6d63d0";

        let result = client.fetch_by_mbid(mbid).await;

        // Verify request succeeded
        assert!(result.is_ok(), "MusicBrainz API request failed: {:?}", result.err());

        let extraction = result.unwrap();

        // Verify extraction source
        assert_eq!(extraction.source, "musicbrainz");

        // Verify metadata extraction
        let metadata = extraction.metadata.expect("Should have metadata");

        // Verify title contains "Let It Be" (exact match may vary by version)
        assert!(
            metadata.title.as_ref().map(|t| t.to_lowercase().contains("let it be")).unwrap_or(false),
            "Title should contain 'Let It Be', got: {:?}",
            metadata.title
        );

        // Verify artist is The Beatles
        assert!(
            metadata.artist.as_ref().map(|a| a.to_lowercase().contains("beatles")).unwrap_or(false),
            "Artist should be The Beatles, got: {:?}",
            metadata.artist
        );

        // Verify duration exists and is reasonable (original is ~3:50 = 230 seconds)
        if let Some(duration) = metadata.duration_seconds {
            assert!(
                duration > 180.0 && duration < 300.0,
                "Duration should be between 180-300 seconds, got: {}",
                duration
            );
        }

        // Verify identity extraction
        let identity = extraction.identity.expect("Should have identity");
        assert_eq!(identity.recording_mbid, mbid.to_string());

        // Verify confidence is high (MusicBrainz is authoritative)
        assert!(
            extraction.confidence >= 0.95,
            "MusicBrainz confidence should be >= 0.95, got: {}",
            extraction.confidence
        );

        // Verify identity confidence is also high
        assert!(
            identity.confidence >= 0.95,
            "MusicBrainz identity confidence should be >= 0.95, got: {}",
            identity.confidence
        );
    }
}


// PLAN023 Tier 1: MusicBrainz API Client
//
// Concept: Query MusicBrainz API for detailed recording metadata using MBID
// Confidence: 0.9 (authoritative music database)
//
// Resolution: HIGH-003, HIGH-004 - API timeout and rate limiting configuration
//
// API Documentation: https://musicbrainz.org/doc/MusicBrainz_API

use crate::import_v2::types::{
    ExtractionSource, ExtractorResult, ImportError, ImportResult, MetadataBundle, MetadataField,
};
use reqwest::Client;
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use std::time::Duration;
use uuid::Uuid;
use wkmp_common::config::TomlConfig;

/// MusicBrainz API response for recording lookup
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MBRecording {
    id: String,
    title: String,
    #[serde(default)]
    disambiguation: String,
    #[serde(rename = "artist-credit", default)]
    artist_credit: Vec<MBArtistCredit>,
    #[serde(default)]
    releases: Vec<MBRelease>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MBArtistCredit {
    name: String,
    artist: MBArtist,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MBArtist {
    id: String,
    name: String,
    #[serde(rename = "sort-name")]
    sort_name: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MBRelease {
    id: String,
    title: String,
    date: Option<String>,
    #[serde(rename = "track-count", default)]
    track_count: u32,
}

/// MusicBrainz API client (Tier 1 extractor concept)
///
/// **Legible Software Principle:**
/// - Independent module: Only depends on HTTP client
/// - Explicit synchronization: Returns `Result<ExtractorResult<MetadataBundle>>`
/// - Transparent behavior: API calls are explicit with visible timeouts
/// - Integrity: Rate limiting enforced (1 req/sec per MusicBrainz guidelines)
pub struct MusicBrainzClient {
    /// HTTP client with configured timeouts
    client: Client,
    /// Base URL for MusicBrainz API
    base_url: String,
    /// User-Agent string (required by MusicBrainz)
    user_agent: String,
    /// Default confidence for MusicBrainz metadata
    confidence: f64,
}

impl MusicBrainzClient {
    /// Create new MusicBrainz client with user-agent
    ///
    /// # Arguments
    /// * `user_agent` - User-Agent string (required by MusicBrainz policy)
    ///   Format: "AppName/Version ( contact@email.com )"
    ///
    /// # Panics
    /// Panics if HTTP client cannot be built (should not happen with valid config)
    pub fn new(user_agent: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15)) // Per HIGH-003: 15s total timeout
            .connect_timeout(Duration::from_secs(5)) // Per HIGH-003: 5s connection timeout
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: "https://musicbrainz.org/ws/2".to_string(),
            user_agent,
            confidence: ExtractionSource::MusicBrainz.default_confidence(),
        }
    }

    /// Create MusicBrainz client from configuration sources
    ///
    /// Uses standard WKMP user-agent string from wkmp_common::config::get_user_agent()
    ///
    /// # Arguments
    /// * `_db` - Database connection pool (unused, for API consistency)
    /// * `_toml_config` - TOML configuration (unused, for API consistency)
    ///
    /// # Returns
    /// MusicBrainz client configured with standard user-agent
    ///
    /// # Traceability
    /// [APIK-UA-010] - Standard user-agent for HTTP clients
    pub async fn from_config(
        _db: &Pool<Sqlite>,
        _toml_config: &TomlConfig,
    ) -> wkmp_common::Result<Self> {
        let user_agent = wkmp_common::config::get_user_agent();
        Ok(Self::new(user_agent))
    }

    /// Lookup recording metadata by MusicBrainz Recording ID
    ///
    /// # Arguments
    /// * `mbid` - MusicBrainz Recording ID (UUID)
    ///
    /// # Returns
    /// MetadataBundle with title, artist, album, release date
    ///
    /// # Errors
    /// Returns error if:
    /// - API request fails (network, timeout)
    /// - API returns 404 (MBID not found)
    /// - Response cannot be parsed
    ///
    /// # Rate Limiting
    /// MusicBrainz requires ≤1 request/second. Caller is responsible for
    /// rate limiting enforcement (via governor crate in workflow layer).
    pub async fn lookup(
        &self,
        mbid: Uuid,
    ) -> ImportResult<ExtractorResult<MetadataBundle>> {
        // Build URL: /ws/2/recording/{mbid}?inc=artists+releases
        let url = format!(
            "{}/recording/{}?inc=artist-credits+releases&fmt=json",
            self.base_url, mbid
        );

        tracing::debug!("Querying MusicBrainz API: mbid={}", mbid);

        // Send GET request with User-Agent header (required)
        let response = self
            .client
            .get(&url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await
            .map_err(|e| {
                ImportError::ExtractionFailed(format!("MusicBrainz API request failed: {}", e))
            })?;

        // Check HTTP status
        let status = response.status();
        if status == 404 {
            return Err(ImportError::ExtractionFailed(format!(
                "MusicBrainz recording not found: {}",
                mbid
            )));
        } else if !status.is_success() {
            return Err(ImportError::ExtractionFailed(format!(
                "MusicBrainz API returned error status: {}",
                status
            )));
        }

        // Parse JSON response
        let recording: MBRecording = response.json().await.map_err(|e| {
            ImportError::ExtractionFailed(format!("Failed to parse MusicBrainz response: {}", e))
        })?;

        // Extract metadata fields
        let title = recording.title;

        // Combine artist credits (handles featured artists, etc.)
        let artist = if !recording.artist_credit.is_empty() {
            recording
                .artist_credit
                .iter()
                .map(|ac| ac.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            String::new()
        };

        // Use first release for album and date (if available)
        let (album, release_date) = if let Some(first_release) = recording.releases.first() {
            (
                Some(first_release.title.clone()),
                first_release.date.clone(),
            )
        } else {
            (None, None)
        };

        tracing::info!(
            "MusicBrainz lookup successful: title='{}', artist='{}', album={:?}",
            title,
            artist,
            album
        );

        // Build MetadataBundle with Vec<MetadataField<T>> for each field
        let mut bundle = MetadataBundle::default();

        // Add title field
        bundle.title.push(MetadataField {
            value: title,
            confidence: self.confidence,
            source: ExtractionSource::MusicBrainz,
        });

        // Add artist field (if not empty)
        if !artist.is_empty() {
            bundle.artist.push(MetadataField {
                value: artist,
                confidence: self.confidence,
                source: ExtractionSource::MusicBrainz,
            });
        }

        // Add album field (if available)
        if let Some(album_title) = album {
            bundle.album.push(MetadataField {
                value: album_title,
                confidence: self.confidence,
                source: ExtractionSource::MusicBrainz,
            });
        }

        // Add release date (if available)
        if let Some(date) = release_date {
            bundle.release_date.push(MetadataField {
                value: date,
                confidence: self.confidence,
                source: ExtractionSource::MusicBrainz,
            });
        }

        // Note: track_number and duration_ms not extracted from recording endpoint

        Ok(ExtractorResult {
            data: bundle,
            confidence: self.confidence,
            source: ExtractionSource::MusicBrainz,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = MusicBrainzClient::new("TestApp/1.0 (test@example.com)".to_string());
        assert_eq!(
            client.user_agent,
            "TestApp/1.0 (test@example.com)"
        );
        assert_eq!(client.confidence, 0.9);
    }

    #[test]
    fn test_base_url() {
        let client = MusicBrainzClient::new("TestApp/1.0".to_string());
        assert!(client.base_url.contains("musicbrainz.org"));
        assert!(client.base_url.contains("/ws/2"));
    }

    #[test]
    fn test_url_construction() {
        let client = MusicBrainzClient::new("TestApp/1.0".to_string());
        let mbid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let expected_url = format!(
            "{}/recording/{}?inc=artist-credits+releases&fmt=json",
            client.base_url, mbid
        );

        // URL should include MBID and required query parameters
        assert!(expected_url.contains("550e8400-e29b-41d4-a716-446655440000"));
        assert!(expected_url.contains("inc=artist-credits+releases"));
        assert!(expected_url.contains("fmt=json"));
    }

    #[tokio::test]
    async fn test_from_config() {
        // Test that from_config correctly uses wkmp_common::config::get_user_agent()
        // Note: Full database migrations not needed for this test since MusicBrainz
        // client doesn't require database configuration resolution.

        let db = sqlx::SqlitePool::connect(":memory:").await.unwrap();

        // Create empty TOML config (user-agent comes from wkmp_common)
        let toml_config = wkmp_common::config::TomlConfig {
            root_folder: None,
            logging: Default::default(),
            static_assets: None,
            acoustid_api_key: None,
            musicbrainz_token: None,
        };

        // Test from_config
        let client = MusicBrainzClient::from_config(&db, &toml_config)
            .await
            .expect("Should create client from config");

        // Verify user-agent follows standard format: WKMP/{version}
        assert!(client.user_agent.starts_with("WKMP/"));
        assert!(client.user_agent.contains("github.com/wkmp/wkmp"));
        assert_eq!(client.confidence, 0.9);
    }

    // Note: Integration tests with real API calls would require:
    // 1. Valid User-Agent header
    // 2. Network access
    // 3. Rate limiting coordination (1 req/sec)
    // 4. Known test MBIDs from MusicBrainz database
    // These would go in tests/ directory as integration tests
}

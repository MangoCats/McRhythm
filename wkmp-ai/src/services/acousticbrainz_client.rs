//! AcousticBrainz API client
//!
//! **[AIA-INT-030]** AcousticBrainz HIGH-LEVEL musical flavor integration
//!
//! **DATA CAPTURED:** HIGH-LEVEL musical characteristics for passage selection:
//! - Musical descriptors: key, scale, tempo (BPM)
//! - Perceptual qualities: danceability
//! - Spectral summaries: brightness (spectral centroid mean), energy
//! - Harmonic features: dissonance (harmonic complexity mean)
//! - Dynamic properties: amplitude variation
//!
//! **DATA NOT CAPTURED:** Raw audio, frame-level data, full spectral details
//!
//! These are AGGREGATED features computed by Essentia, suitable for
//! Program Director's automatic passage selection based on musical similarity.
//!
//! Note: AcousticBrainz ceased accepting new submissions in 2022, so data
//! is only available for recordings analyzed before that date.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;

const ACOUSTICBRAINZ_BASE_URL: &str = "https://acousticbrainz.org/api/v1";
const USER_AGENT: &str = "WKMP/0.1.0 (https://github.com/wkmp/wkmp)";
const RATE_LIMIT_MS: u64 = 1000; // 1 request per second (conservative)

/// AcousticBrainz client errors
#[derive(Debug, Error)]
pub enum ABError {
    /// Network communication error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Recording not found in AcousticBrainz database
    #[error("Recording not found in AcousticBrainz: {0}")]
    RecordingNotFound(String),

    /// AcousticBrainz API returned error response
    #[error("API error {0}: {1}")]
    ApiError(u16, String),

    /// Failed to parse API response JSON
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// AcousticBrainz low-level response (simplified)
///
/// The full API response contains hundreds of features. We extract
/// the most relevant ones for musical flavor characterization.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ABLowLevel {
    /// Analysis metadata (version, audio properties)
    pub metadata: ABMetadata,
    /// Tonal features (key, scale, chords)
    pub tonal: Option<ABTonal>,
    /// Rhythm features (BPM, danceability)
    pub rhythm: Option<ABRhythm>,
    /// Low-level audio features (spectral, dynamic)
    pub lowlevel: Option<ABLowLevelFeatures>,
}

/// AcousticBrainz metadata section
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ABMetadata {
    /// Essentia version information
    pub version: Option<ABVersion>,
    /// Audio file properties
    pub audio_properties: Option<ABAudioProperties>,
}

/// Essentia version information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ABVersion {
    /// Essentia library version string
    pub essentia: Option<String>,
}

/// Audio file properties
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ABAudioProperties {
    /// Track duration in seconds
    pub length: Option<f64>,
    /// Sample rate in Hz
    pub sample_rate: Option<i32>,
    /// Bit rate in kbps
    pub bit_rate: Option<i32>,
}

/// Tonal features (key, scale, chords)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ABTonal {
    /// Musical key (e.g., "C", "A")
    pub key_key: Option<String>,
    /// Musical scale (e.g., "major", "minor")
    pub key_scale: Option<String>,
    /// Key detection confidence (0.0-1.0)
    pub key_strength: Option<f64>,
    /// Predominant chord key
    pub chords_key: Option<String>,
    /// Predominant chord scale
    pub chords_scale: Option<String>,
}

/// Rhythm features (BPM, beats)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ABRhythm {
    /// Beats per minute
    pub bpm: Option<f64>,
    /// Note onset rate (onsets per second)
    pub onset_rate: Option<f64>,
    /// Danceability score (0.0-1.0)
    pub danceability: Option<f64>,
}

/// Low-level audio features
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ABLowLevelFeatures {
    /// Spectral centroid statistics (brightness)
    pub spectral_centroid: Option<ABStatistics>,
    /// Spectral energy statistics
    pub spectral_energy: Option<ABStatistics>,
    /// Spectral rolloff statistics
    pub spectral_rolloff: Option<ABStatistics>,
    /// Dissonance statistics (harmonic complexity)
    pub dissonance: Option<ABStatistics>,
    /// Dynamic complexity (amplitude variation)
    pub dynamic_complexity: Option<f64>,
}

/// Statistical summary of audio feature
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ABStatistics {
    /// Mean value
    pub mean: Option<f64>,
    /// Median value
    pub median: Option<f64>,
    /// Variance
    pub var: Option<f64>,
    /// Minimum value
    pub min: Option<f64>,
    /// Maximum value
    pub max: Option<f64>,
}

/// Musical flavor vector - HIGH-LEVEL musical characteristics
///
/// **IMPORTANT:** This captures HIGH-LEVEL musical features, NOT raw audio data.
///
/// These are aggregated/computed features from Essentia analysis:
/// - Musical descriptors (key, scale, tempo)
/// - Perceptual qualities (danceability)
/// - Spectral summaries (brightness, energy)
/// - Harmonic characteristics (dissonance)
/// - Dynamic properties (amplitude variation)
///
/// Used by Program Director for automatic passage selection based on musical similarity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicalFlavorVector {
    /// Musical key (e.g., "C", "A")
    pub key: Option<String>,
    /// Musical scale (e.g., "major", "minor")
    pub scale: Option<String>,
    /// Key detection confidence (0.0-1.0)
    pub key_strength: Option<f64>,

    /// Beats per minute
    pub bpm: Option<f64>,
    /// Danceability score (0.0-1.0)
    pub danceability: Option<f64>,

    /// Spectral centroid (brightness)
    pub spectral_centroid: Option<f64>,
    /// Spectral energy (overall energy level)
    pub spectral_energy: Option<f64>,
    /// Dissonance (harmonic complexity)
    pub dissonance: Option<f64>,

    /// Dynamic complexity (amplitude variation)
    pub dynamic_complexity: Option<f64>,

    /// Data source ("acousticbrainz" or "essentia")
    pub source: String,
}

impl MusicalFlavorVector {
    /// Convert to JSON string for database storage
    pub fn to_json(&self) -> Result<String, ABError> {
        serde_json::to_string(self)
            .map_err(|e| ABError::ParseError(format!("Failed to serialize flavor vector: {}", e)))
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, ABError> {
        serde_json::from_str(json)
            .map_err(|e| ABError::ParseError(format!("Failed to parse flavor vector: {}", e)))
    }

    /// Extract HIGH-LEVEL musical flavor vector from AcousticBrainz data
    ///
    /// **Data Source:** AcousticBrainz "low-level" endpoint (misleading name)
    /// **What We Extract:** HIGH-LEVEL aggregated musical features:
    /// - Tonal: Musical key, scale, key strength (confidence)
    /// - Rhythm: BPM, danceability score
    /// - Spectral: Brightness (centroid mean), energy (mean), rolloff
    /// - Harmonic: Dissonance (mean harmonic complexity)
    /// - Dynamic: Amplitude variation complexity
    ///
    /// **What We DON'T Extract:** Raw audio samples, frame-level data, full spectrum
    pub fn from_acousticbrainz(data: &ABLowLevel) -> Self {
        Self {
            key: data.tonal.as_ref().and_then(|t| t.key_key.clone()),
            scale: data.tonal.as_ref().and_then(|t| t.key_scale.clone()),
            key_strength: data.tonal.as_ref().and_then(|t| t.key_strength),
            bpm: data.rhythm.as_ref().and_then(|r| r.bpm),
            danceability: data.rhythm.as_ref().and_then(|r| r.danceability),
            spectral_centroid: data
                .lowlevel
                .as_ref()
                .and_then(|l| l.spectral_centroid.as_ref())
                .and_then(|s| s.mean),
            spectral_energy: data
                .lowlevel
                .as_ref()
                .and_then(|l| l.spectral_energy.as_ref())
                .and_then(|s| s.mean),
            dissonance: data
                .lowlevel
                .as_ref()
                .and_then(|l| l.dissonance.as_ref())
                .and_then(|s| s.mean),
            dynamic_complexity: data.lowlevel.as_ref().and_then(|l| l.dynamic_complexity),
            source: "acousticbrainz".to_string(),
        }
    }
}

/// Rate limiter for AcousticBrainz (1 req/sec)
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
                tracing::debug!("AcousticBrainz rate limiting: waiting {:?}", wait_time);
                tokio::time::sleep(wait_time).await;
            }
        }

        *last = Some(Instant::now());
    }
}

/// AcousticBrainz API client
pub struct AcousticBrainzClient {
    http_client: reqwest::Client,
    rate_limiter: Arc<RateLimiter>,
}

impl AcousticBrainzClient {
    /// Create new AcousticBrainz client
    pub fn new() -> Result<Self, ABError> {
        let http_client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ABError::NetworkError(e.to_string()))?;

        Ok(Self {
            http_client,
            rate_limiter: Arc::new(RateLimiter::new(RATE_LIMIT_MS)),
        })
    }

    /// Lookup musical features by recording MBID
    ///
    /// **[AIA-INT-030]** Query AcousticBrainz for HIGH-LEVEL musical flavor
    ///
    /// **Note on Naming:** This queries the AcousticBrainz "low-level" endpoint,
    /// but extracts HIGH-LEVEL aggregated musical features (key, BPM, danceability, etc.),
    /// NOT raw audio data. The endpoint name is historical/misleading.
    ///
    /// **What We Get:** Essentia-computed summaries suitable for music selection
    /// **What We DON'T Get:** Raw waveform data, frame-level spectrograms, detailed MFCC arrays
    pub async fn lookup_lowlevel(&self, recording_mbid: &str) -> Result<ABLowLevel, ABError> {
        // Rate limit
        self.rate_limiter.wait().await;

        // Query API
        let url = format!("{}/{}/low-level", ACOUSTICBRAINZ_BASE_URL, recording_mbid);

        tracing::debug!(mbid = %recording_mbid, url = %url, "Querying AcousticBrainz API");

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ABError::NetworkError(e.to_string()))?;

        let status = response.status();

        if status == 404 {
            return Err(ABError::RecordingNotFound(recording_mbid.to_string()));
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ABError::ApiError(status.as_u16(), error_text));
        }

        let lowlevel: ABLowLevel = response
            .json()
            .await
            .map_err(|e| ABError::ParseError(e.to_string()))?;

        tracing::info!(
            mbid = %recording_mbid,
            has_tonal = lowlevel.tonal.is_some(),
            has_rhythm = lowlevel.rhythm.is_some(),
            "AcousticBrainz lookup successful"
        );

        Ok(lowlevel)
    }

    /// Get musical flavor vector for recording
    ///
    /// Convenience method that queries and extracts flavor vector
    pub async fn get_flavor_vector(
        &self,
        recording_mbid: &str,
    ) -> Result<MusicalFlavorVector, ABError> {
        let lowlevel = self.lookup_lowlevel(recording_mbid).await?;
        Ok(MusicalFlavorVector::from_acousticbrainz(&lowlevel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = AcousticBrainzClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_flavor_vector_serialization() {
        let vector = MusicalFlavorVector {
            key: Some("C".to_string()),
            scale: Some("major".to_string()),
            key_strength: Some(0.85),
            bpm: Some(120.0),
            danceability: Some(0.7),
            spectral_centroid: Some(1500.0),
            spectral_energy: Some(0.6),
            dissonance: Some(0.3),
            dynamic_complexity: Some(0.5),
            source: "acousticbrainz".to_string(),
        };

        let json = vector.to_json().unwrap();
        let parsed = MusicalFlavorVector::from_json(&json).unwrap();

        assert_eq!(parsed.key, Some("C".to_string()));
        assert_eq!(parsed.bpm, Some(120.0));
        assert_eq!(parsed.source, "acousticbrainz");
    }

    #[tokio::test]
    async fn test_rate_limiter_timing() {
        let limiter = RateLimiter::new(100); // 100ms between requests

        let start = Instant::now();
        limiter.wait().await; // First request - immediate
        let first_elapsed = start.elapsed();

        limiter.wait().await; // Second request - should wait ~100ms
        let second_elapsed = start.elapsed();

        assert!(first_elapsed.as_millis() < 50);
        assert!(second_elapsed.as_millis() >= 100);
    }
}

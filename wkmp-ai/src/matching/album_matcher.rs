//! Album Matcher Service
//!
//! **[PLAN026]** Increment 5: Album matching via am28 algorithm
//!
//! Provides album identification by matching detected track boundaries
//! against MusicBrainz release editions.
//!
//! # Algorithm Overview
//! 1. Extract metadata from audio file (ID3 tags + path)
//! 2. Search MusicBrainz for candidate releases
//! 3. Group releases into editions (by track count/duration pattern)
//! 4. Test each edition through Stages 2-5:
//!    - Stage 2: Silence detection parameter optimization
//!    - Stage 3: Over-segmentation assembly
//!    - Stage 4: Quiet spot detection
//!    - Stage 5: Extra track merging
//! 5. Select best matching edition with artist verification

use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::types::{AlbumMatchResult, MatchingStage};

/// Album matching error types
#[derive(Debug)]
pub enum AlbumMatchError {
    /// Audio decoding failed
    DecodeError(String),
    /// Metadata extraction failed
    MetadataError(String),
    /// MusicBrainz API error
    MusicBrainzError(String),
    /// No candidates found
    NoCandidates,
    /// Internal processing error
    InternalError(String),
}

impl std::fmt::Display for AlbumMatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlbumMatchError::DecodeError(s) => write!(f, "Decode error: {}", s),
            AlbumMatchError::MetadataError(s) => write!(f, "Metadata error: {}", s),
            AlbumMatchError::MusicBrainzError(s) => write!(f, "MusicBrainz error: {}", s),
            AlbumMatchError::NoCandidates => write!(f, "No MusicBrainz candidates found"),
            AlbumMatchError::InternalError(s) => write!(f, "Internal error: {}", s),
        }
    }
}

impl std::error::Error for AlbumMatchError {}

/// Configuration for album matching
#[derive(Debug, Clone)]
pub struct AlbumMatcherConfig {
    /// Default silence threshold for track detection (dB)
    pub default_threshold_db: f64,
    /// Default minimum silence duration (seconds)
    pub default_min_duration_secs: f64,
    /// Track duration tolerance for matching (seconds)
    pub match_tolerance_secs: f64,
    /// Minimum artist similarity (Jaro-Winkler) to accept match
    pub min_artist_similarity: f64,
    /// Enable MusicBrainz API caching
    pub enable_cache: bool,
    /// Cache directory path
    pub cache_dir: Option<std::path::PathBuf>,
}

impl Default for AlbumMatcherConfig {
    fn default() -> Self {
        Self {
            default_threshold_db: -48.0,
            default_min_duration_secs: 0.5,
            match_tolerance_secs: 3.0,
            min_artist_similarity: 0.50,
            enable_cache: true,
            cache_dir: None,
        }
    }
}

/// Album matcher service
///
/// **[PLAN026]** Orchestrates album identification through am28 stages.
pub struct AlbumMatcher {
    /// Configuration
    config: AlbumMatcherConfig,
    /// HTTP client for MusicBrainz API
    http_client: reqwest::Client,
}

impl AlbumMatcher {
    /// Create new album matcher with default configuration
    pub fn new() -> Self {
        Self::with_config(AlbumMatcherConfig::default())
    }

    /// Create new album matcher with custom configuration
    pub fn with_config(config: AlbumMatcherConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent("WKMP/0.1.0 (https://github.com/wkmp/wkmp)")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
        }
    }

    /// Match an album file against MusicBrainz
    ///
    /// **[REQ-ALG-004..014]** Full album matching workflow:
    /// 1. Decode audio and detect track boundaries
    /// 2. Extract metadata (ID3 tags + path)
    /// 3. Search MusicBrainz for candidate releases
    /// 4. Test editions through Stages 2-5
    /// 5. Return best match with verification
    ///
    /// # Arguments
    /// * `audio_path` - Path to the audio file
    /// * `artist_hint` - Artist name from metadata (for MusicBrainz search)
    /// * `album_hint` - Album name from metadata (for MusicBrainz search)
    ///
    /// # Returns
    /// AlbumMatchResult with match details or error status
    pub async fn match_album(
        &self,
        audio_path: &Path,
        artist_hint: Option<&str>,
        album_hint: Option<&str>,
    ) -> Result<AlbumMatchResult, AlbumMatchError> {
        info!(
            "Starting album match for: {}",
            audio_path.display()
        );

        // TODO: Full am28 integration
        // For now, return placeholder result indicating album path is not yet implemented

        debug!(
            "Album matching requested for artist={:?}, album={:?}",
            artist_hint, album_hint
        );

        // Placeholder: In full implementation, this would:
        // 1. Call decode_audio() to get PCM samples
        // 2. Call precompute_silence_cache() for all parameter combinations
        // 3. Call comprehensive_musicbrainz_search() to get candidate releases
        // 4. Call group_into_editions() to organize by track pattern
        // 5. Call test_single_edition() for each edition through Stages 2-5
        // 6. Select best result with artist verification

        warn!(
            "Album matching not yet fully implemented - returning placeholder result"
        );

        Ok(AlbumMatchResult::no_match(
            "Album matching not yet fully implemented (Increment 5 placeholder)".to_string(),
        ))
    }

    /// Get configuration
    pub fn config(&self) -> &AlbumMatcherConfig {
        &self.config
    }
}

impl Default for AlbumMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_album_matcher_config_defaults() {
        let config = AlbumMatcherConfig::default();
        assert_eq!(config.default_threshold_db, -48.0);
        assert_eq!(config.default_min_duration_secs, 0.5);
        assert_eq!(config.match_tolerance_secs, 3.0);
        assert_eq!(config.min_artist_similarity, 0.50);
    }

    #[test]
    fn test_album_matcher_creation() {
        let matcher = AlbumMatcher::new();
        assert_eq!(matcher.config().match_tolerance_secs, 3.0);
    }
}

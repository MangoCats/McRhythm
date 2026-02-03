//! Recording Matcher Service
//!
//! Searches MusicBrainz for recordings matching metadata.
//! Used as fallback when AcoustID fails or returns low confidence.
//!
//! **[SSI-MB-010]** MusicBrainz recording search fallback
//!
//! ## Features
//! - Lucene query building with artist, title, and duration filters
//! - Jaro-Winkler similarity scoring for candidates
//! - Configurable thresholds for title and artist matching
//! - Sorted results with best matches first

use crate::db::recording_cache;
use crate::services::musicbrainz_client::{
    escape_lucene, MBError, MBRecordingSearchResult, MusicBrainzClient,
};
use crate::utils::string_similarity::{
    jaro_winkler_similarity, normalize_for_comparison, ARTIST_SIMILARITY_THRESHOLD,
    TITLE_SIMILARITY_THRESHOLD,
};
use sqlx::SqlitePool;
use std::sync::Arc;

/// Matched recording candidate from MusicBrainz search
#[derive(Debug, Clone)]
pub struct RecordingCandidate {
    /// MusicBrainz Recording MBID
    pub mbid: String,
    /// Recording title from MusicBrainz
    pub title: String,
    /// Artist name from MusicBrainz (joined from artist credits)
    pub artist: String,
    /// Duration in seconds (if available)
    pub duration: Option<f64>,
    /// Combined similarity score (0.0-1.0)
    pub similarity: f64,
    /// Title similarity component
    pub title_similarity: f64,
    /// Artist similarity component
    pub artist_similarity: f64,
    /// MusicBrainz search score (0-100)
    pub mb_score: u32,
}

/// Recording matcher configuration
#[derive(Debug, Clone)]
pub struct RecordingMatcherConfig {
    /// Minimum title similarity (default: 0.85)
    pub title_threshold: f64,
    /// Minimum artist similarity (default: 0.80)
    pub artist_threshold: f64,
    /// Duration tolerance in seconds (default: 10.0)
    pub duration_tolerance_secs: f64,
    /// Maximum results to request from MusicBrainz (default: 25)
    pub max_results: u32,
}

impl Default for RecordingMatcherConfig {
    fn default() -> Self {
        Self {
            title_threshold: TITLE_SIMILARITY_THRESHOLD,
            artist_threshold: ARTIST_SIMILARITY_THRESHOLD,
            duration_tolerance_secs: 10.0,
            max_results: 25,
        }
    }
}

/// Recording matcher service
///
/// Searches MusicBrainz for recordings matching artist/title/duration metadata.
pub struct RecordingMatcher {
    mb_client: Arc<MusicBrainzClient>,
    config: RecordingMatcherConfig,
}

impl RecordingMatcher {
    /// Create a new recording matcher with default configuration
    pub fn new(mb_client: Arc<MusicBrainzClient>) -> Self {
        Self {
            mb_client,
            config: RecordingMatcherConfig::default(),
        }
    }

    /// Create a new recording matcher with custom configuration
    pub fn with_config(mb_client: Arc<MusicBrainzClient>, config: RecordingMatcherConfig) -> Self {
        Self { mb_client, config }
    }

    /// Build MusicBrainz Lucene query from metadata
    ///
    /// # Arguments
    /// * `artist` - Artist name to search
    /// * `title` - Recording title to search
    /// * `duration_secs` - Optional duration filter (±tolerance)
    ///
    /// # Returns
    /// Lucene query string for MusicBrainz API
    pub fn build_query(&self, artist: &str, title: &str, duration_secs: Option<f64>) -> String {
        let mut query = format!(
            "artist:\"{}\" AND recording:\"{}\"",
            escape_lucene(artist),
            escape_lucene(title)
        );

        if let Some(dur) = duration_secs {
            let min_ms = ((dur - self.config.duration_tolerance_secs) * 1000.0).max(0.0) as i64;
            let max_ms = ((dur + self.config.duration_tolerance_secs) * 1000.0) as i64;
            query.push_str(&format!(" AND dur:[{} TO {}]", min_ms, max_ms));
        }

        query
    }

    /// Search for matching recordings
    ///
    /// # Arguments
    /// * `artist` - Artist name from ID3 tags
    /// * `title` - Recording title from ID3 tags
    /// * `duration_secs` - Optional duration for filtering
    ///
    /// # Returns
    /// List of candidates sorted by similarity (best first)
    pub async fn search(
        &self,
        artist: &str,
        title: &str,
        duration_secs: Option<f64>,
    ) -> Result<Vec<RecordingCandidate>, MBError> {
        let query = self.build_query(artist, title, duration_secs);

        tracing::debug!(
            artist = %artist,
            title = %title,
            duration = ?duration_secs,
            query = %query,
            "Searching MusicBrainz for recording"
        );

        let response = self
            .mb_client
            .search_recordings(&query, Some(self.config.max_results))
            .await?;

        let mut candidates: Vec<RecordingCandidate> = response
            .recordings
            .into_iter()
            .filter_map(|rec| self.score_candidate(&rec, artist, title))
            .collect();

        // Sort by similarity descending
        candidates.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        tracing::info!(
            artist = %artist,
            title = %title,
            candidates = candidates.len(),
            best_similarity = candidates.first().map(|c| c.similarity),
            "Recording search completed"
        );

        Ok(candidates)
    }

    /// Search for matching recordings with cache
    ///
    /// Checks cache first to avoid redundant API calls. If no cached result,
    /// queries MusicBrainz and caches the result.
    ///
    /// # Arguments
    /// * `artist` - Artist name from ID3 tags
    /// * `title` - Recording title from ID3 tags
    /// * `duration_secs` - Optional duration for filtering
    /// * `pool` - Database pool for cache access
    ///
    /// # Returns
    /// List of candidates sorted by similarity (best first)
    pub async fn search_with_cache(
        &self,
        artist: &str,
        title: &str,
        duration_secs: Option<f64>,
        pool: &SqlitePool,
    ) -> Result<Vec<RecordingCandidate>, MBError> {
        let artist_norm = normalize_for_comparison(artist);
        let title_norm = normalize_for_comparison(title);

        // Check cache first
        if let Ok(Some(cached)) = recording_cache::get_cached(pool, &artist_norm, &title_norm).await
        {
            tracing::debug!(
                artist = %artist,
                title = %title,
                cached_mbid = ?cached.mbid,
                "Cache hit for recording search"
            );

            if let Some(mbid) = cached.mbid {
                return Ok(vec![RecordingCandidate {
                    mbid,
                    title: title.to_string(),
                    artist: artist.to_string(),
                    duration: duration_secs,
                    similarity: cached.confidence.unwrap_or(1.0),
                    title_similarity: cached.confidence.unwrap_or(1.0),
                    artist_similarity: cached.confidence.unwrap_or(1.0),
                    mb_score: 100,
                }]);
            } else {
                // Cached "not found" result
                return Ok(vec![]);
            }
        }

        // Cache miss - query MusicBrainz
        let candidates = self.search(artist, title, duration_secs).await?;

        // Cache the result (best candidate or "not found")
        let (mbid, confidence) = candidates
            .first()
            .map(|c| (Some(c.mbid.as_str()), Some(c.similarity)))
            .unwrap_or((None, None));

        if let Err(e) = recording_cache::cache_result(pool, &artist_norm, &title_norm, mbid, confidence).await {
            tracing::warn!(error = %e, "Failed to cache recording search result");
        }

        Ok(candidates)
    }

    /// Score a recording candidate against query metadata
    ///
    /// Returns None if candidate doesn't meet minimum thresholds.
    fn score_candidate(
        &self,
        rec: &MBRecordingSearchResult,
        query_artist: &str,
        query_title: &str,
    ) -> Option<RecordingCandidate> {
        // Extract artist name from credits
        let artist = self.extract_artist_name(rec);

        // Calculate similarities
        let artist_sim = jaro_winkler_similarity(&artist, query_artist);
        let title_sim = jaro_winkler_similarity(&rec.title, query_title);

        // Both must meet thresholds
        if artist_sim < self.config.artist_threshold || title_sim < self.config.title_threshold {
            tracing::trace!(
                mbid = %rec.id,
                title = %rec.title,
                artist = %artist,
                artist_sim = %artist_sim,
                title_sim = %title_sim,
                "Candidate rejected: below threshold"
            );
            return None;
        }

        // Combined score: title weighted 60%, artist weighted 40%
        // Title is more discriminating for recordings
        let similarity = artist_sim * 0.4 + title_sim * 0.6;

        Some(RecordingCandidate {
            mbid: rec.id.clone(),
            title: rec.title.clone(),
            artist,
            duration: rec.length.map(|ms| ms as f64 / 1000.0),
            similarity,
            title_similarity: title_sim,
            artist_similarity: artist_sim,
            mb_score: rec.score,
        })
    }

    /// Extract artist name from MusicBrainz artist credits
    ///
    /// Joins multiple artist credits with their joinphrases (typically " feat. " or " & ")
    fn extract_artist_name(&self, rec: &MBRecordingSearchResult) -> String {
        match &rec.artist_credit {
            Some(credits) if !credits.is_empty() => {
                credits.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(" ")
            }
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_query_basic() {
        let mb_client = Arc::new(MusicBrainzClient::new().expect("MusicBrainz client"));
        let matcher = RecordingMatcher::new(mb_client);

        let query = matcher.build_query("Queen", "Bohemian Rhapsody", None);
        assert_eq!(query, r#"artist:"Queen" AND recording:"Bohemian Rhapsody""#);
    }

    #[test]
    fn test_build_query_with_duration() {
        let mb_client = Arc::new(MusicBrainzClient::new().expect("MusicBrainz client"));
        let matcher = RecordingMatcher::new(mb_client);

        let query = matcher.build_query("Queen", "Bohemian Rhapsody", Some(180.0));

        // Duration tolerance is ±10s, so 170000-190000 ms
        assert!(query.contains("dur:[170000 TO 190000]"));
        assert!(query.contains(r#"artist:"Queen""#));
        assert!(query.contains(r#"recording:"Bohemian Rhapsody""#));
    }

    #[test]
    fn test_build_query_escapes_special_chars() {
        let mb_client = Arc::new(MusicBrainzClient::new().expect("MusicBrainz client"));
        let matcher = RecordingMatcher::new(mb_client);

        // Test that special Lucene characters are escaped
        let query = matcher.build_query("AC/DC", "Back in Black (Live)", None);
        assert!(query.contains(r#"artist:"AC\/DC""#));
        assert!(query.contains(r#"recording:"Back in Black \(Live\)""#));
    }

    #[test]
    fn test_build_query_duration_tolerance() {
        let mb_client = Arc::new(MusicBrainzClient::new().expect("MusicBrainz client"));
        let config = RecordingMatcherConfig {
            duration_tolerance_secs: 5.0,
            ..Default::default()
        };
        let matcher = RecordingMatcher::with_config(mb_client, config);

        let query = matcher.build_query("Artist", "Title", Some(120.0));

        // ±5s tolerance: 115000-125000 ms
        assert!(query.contains("dur:[115000 TO 125000]"));
    }

    #[test]
    fn test_build_query_short_duration() {
        let mb_client = Arc::new(MusicBrainzClient::new().expect("MusicBrainz client"));
        let matcher = RecordingMatcher::new(mb_client);

        // 5 second track with 10 second tolerance - min should be 0, not negative
        let query = matcher.build_query("Artist", "Title", Some(5.0));
        assert!(query.contains("dur:[0 TO 15000]"));
    }

    #[test]
    fn test_default_config() {
        let config = RecordingMatcherConfig::default();
        assert!((config.title_threshold - 0.85).abs() < 0.001);
        assert!((config.artist_threshold - 0.80).abs() < 0.001);
        assert!((config.duration_tolerance_secs - 10.0).abs() < 0.001);
        assert_eq!(config.max_results, 25);
    }

    #[test]
    fn test_recording_candidate_fields() {
        let candidate = RecordingCandidate {
            mbid: "test-mbid".to_string(),
            title: "Test Title".to_string(),
            artist: "Test Artist".to_string(),
            duration: Some(180.0),
            similarity: 0.95,
            title_similarity: 0.98,
            artist_similarity: 0.90,
            mb_score: 100,
        };

        assert_eq!(candidate.mbid, "test-mbid");
        assert_eq!(candidate.title, "Test Title");
        assert_eq!(candidate.artist, "Test Artist");
        assert_eq!(candidate.duration, Some(180.0));
        assert!((candidate.similarity - 0.95).abs() < 0.001);
    }
}

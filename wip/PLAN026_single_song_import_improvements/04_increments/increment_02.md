# Increment 2: Recording Matcher Service

**Estimated Effort:** 3-4 hours
**Dependencies:** Increment 1 (string_similarity)
**Tests:** TC-U-MB-010-01, TC-U-MB-010-02, TC-U-MB-020-01, TC-U-MB-020-02, TC-U-MB-030-01

---

## Objective

Create RecordingMatcher service that queries MusicBrainz for recordings by artist/title/duration.

---

## Deliverables

### 1. New File: `src/services/recording_matcher.rs`

```rust
//! Recording Matcher Service
//!
//! Searches MusicBrainz for recordings matching metadata.
//! Used as fallback when AcoustID fails or returns low confidence.

use crate::services::musicbrainz_client::{MusicBrainzClient, MBError};
use crate::utils::string_similarity::{normalize_for_comparison, jaro_winkler_similarity};
use std::sync::Arc;

/// Matched recording candidate
#[derive(Debug, Clone)]
pub struct RecordingCandidate {
    /// MusicBrainz Recording MBID
    pub mbid: String,
    /// Recording title from MusicBrainz
    pub title: String,
    /// Artist name from MusicBrainz
    pub artist: String,
    /// Duration in seconds (if available)
    pub duration: Option<f64>,
    /// Combined similarity score (0.0-1.0)
    pub similarity: f64,
}

/// Recording matcher configuration
pub struct RecordingMatcherConfig {
    /// Minimum title similarity (default: 0.85)
    pub title_threshold: f64,
    /// Minimum artist similarity (default: 0.80)
    pub artist_threshold: f64,
    /// Duration tolerance in seconds (default: 10.0)
    pub duration_tolerance_secs: f64,
}

impl Default for RecordingMatcherConfig {
    fn default() -> Self {
        Self {
            title_threshold: 0.85,
            artist_threshold: 0.80,
            duration_tolerance_secs: 10.0,
        }
    }
}

/// Recording matcher service
pub struct RecordingMatcher {
    mb_client: Arc<MusicBrainzClient>,
    config: RecordingMatcherConfig,
}

impl RecordingMatcher {
    pub fn new(mb_client: Arc<MusicBrainzClient>) -> Self {
        Self {
            mb_client,
            config: RecordingMatcherConfig::default(),
        }
    }

    /// Build MusicBrainz Lucene query
    fn build_query(&self, artist: &str, title: &str, duration_secs: Option<f64>) -> String {
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
    pub async fn search(
        &self,
        artist: &str,
        title: &str,
        duration_secs: Option<f64>,
    ) -> Result<Vec<RecordingCandidate>, MBError> {
        let query = self.build_query(artist, title, duration_secs);
        let response = self.mb_client.search_recordings(&query, Some(25)).await?;

        let mut candidates: Vec<RecordingCandidate> = response.recordings
            .into_iter()
            .filter_map(|rec| self.score_candidate(&rec, artist, title))
            .collect();

        // Sort by similarity descending
        candidates.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

        Ok(candidates)
    }

    /// Score a recording candidate
    fn score_candidate(
        &self,
        rec: &MBRecording,
        query_artist: &str,
        query_title: &str,
    ) -> Option<RecordingCandidate> {
        let artist_sim = jaro_winkler_similarity(&rec.artist_credit, query_artist);
        let title_sim = jaro_winkler_similarity(&rec.title, query_title);

        // Both must meet thresholds
        if artist_sim < self.config.artist_threshold || title_sim < self.config.title_threshold {
            return None;
        }

        // Combined score (weighted average)
        let similarity = (artist_sim * 0.4 + title_sim * 0.6);

        Some(RecordingCandidate {
            mbid: rec.id.clone(),
            title: rec.title.clone(),
            artist: rec.artist_credit.clone(),
            duration: rec.length.map(|ms| ms as f64 / 1000.0),
            similarity,
        })
    }
}

fn escape_lucene(s: &str) -> String {
    // Use existing function from musicbrainz_client
    crate::services::musicbrainz_client::escape_lucene(s)
}
```

### 2. Update: `src/services/mod.rs`

Add module declaration:
```rust
pub mod recording_matcher;
```

---

## Files Modified

| File | Action | Lines |
|------|--------|-------|
| `src/services/recording_matcher.rs` | Create | ~200 |
| `src/services/mod.rs` | Modify | +1 |

---

## Verification

- [ ] TC-U-MB-020-01 passes (query includes artist, title, duration)
- [ ] TC-U-MB-020-02 passes (duration tolerance ±10s)
- [ ] TC-U-MB-030-01 passes (candidates sorted by similarity)
- [ ] `cargo test recording_matcher` passes
- [ ] No clippy warnings

---

## Success Criteria

- RecordingMatcher builds valid Lucene queries
- Candidates scored and filtered by similarity thresholds
- Sorted results with best matches first

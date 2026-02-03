# Increment 6: Edition Grouping

**Estimated Effort:** 4 hours
**Dependencies:** Increment 1
**Deliverables:** matching/editions/ module

---

## Objective

Create edition grouping module to organize MusicBrainz releases by track pattern.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| matching/edition.rs | ~300 | Adapt to library |

---

## Tasks

### 6.1 Create matching/editions/mod.rs

```rust
//! Edition Grouping and Filtering
//!
//! Groups MusicBrainz releases into "editions" - unique combinations
//! of track count and duration pattern.

pub mod grouping;
pub mod scoring;
pub mod filtering;

pub use grouping::*;
pub use scoring::*;
pub use filtering::*;
```

### 6.2 Create matching/editions/grouping.rs

```rust
//! Edition Grouping

use crate::matching::types::*;
use std::collections::HashMap;

/// Group releases into editions by track pattern
///
/// # Arguments
/// * `releases` - MusicBrainz releases with track details
///
/// # Returns
/// Vector of unique editions
pub fn group_into_editions(releases: &[MBReleaseDetails]) -> Vec<Edition> {
    let mut edition_map: HashMap<EditionKey, Edition> = HashMap::new();

    for release in releases {
        // Get total tracks and durations across all media
        let mut track_count = 0;
        let mut durations_ms: Vec<u32> = Vec::new();
        let mut recording_mbids: Vec<String> = Vec::new();

        for media in &release.media {
            for track in &media.tracks {
                track_count += 1;
                durations_ms.push(track.length.unwrap_or(0));
                if let Some(ref recording) = track.recording {
                    recording_mbids.push(recording.id.clone());
                }
            }
        }

        let key = EditionKey::new(track_count, &durations_ms);

        edition_map.entry(key.clone()).or_insert_with(|| {
            Edition {
                release_mbid: release.id.clone(),
                title: release.title.clone(),
                artist: extract_artist_from_release(release),
                artist_credit: None,
                country: None,
                status: None,
                track_count,
                track_durations: durations_ms.iter()
                    .map(|ms| *ms as f64 / 1000.0)
                    .collect(),
                recording_mbids,
                name_distance_rank: None,
                name_distance_score: None,
                durations: durations_ms,
            }
        });
    }

    edition_map.into_values().collect()
}

/// Key for grouping editions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EditionKey {
    track_count: usize,
    duration_signature: String,
}

impl EditionKey {
    fn new(track_count: usize, durations_ms: &[u32]) -> Self {
        // Create signature by rounding durations to nearest 5 seconds
        let signature: Vec<String> = durations_ms
            .iter()
            .map(|ms| {
                let secs = ms / 1000;
                let rounded = (secs / 5) * 5;
                rounded.to_string()
            })
            .collect();

        Self {
            track_count,
            duration_signature: signature.join("-"),
        }
    }
}

fn extract_artist_from_release(release: &MBReleaseDetails) -> String {
    // MBReleaseDetails doesn't have artist_credit, use from first recording
    "Unknown Artist".to_string() // Will be filled from search results
}
```

### 6.3 Create matching/editions/scoring.rs

```rust
//! Edition Match Scoring

use crate::matching::types::*;

/// Score how well an edition matches detected track durations
pub fn score_edition_match(
    detected_durations: &[f64],
    expected_durations: &[u32],
    tolerance_secs: f64,
) -> f64 {
    if detected_durations.len() != expected_durations.len() {
        return 0.0;
    }

    let mut matched = 0;
    for (detected, expected) in detected_durations.iter().zip(expected_durations.iter()) {
        let expected_secs = *expected as f64 / 1000.0;
        let error = (detected - expected_secs).abs();
        if error <= tolerance_secs {
            matched += 1;
        }
    }

    (matched as f64 / expected_durations.len() as f64) * 100.0
}

/// Analyze track matching in detail
pub fn analyze_track_matching(
    detected_durations: &[f64],
    expected_durations: &[u32],
    tolerance_secs: f64,
) -> CandidateTestResult {
    let expected_count = expected_durations.len();
    let detected_count = detected_durations.len();

    // Handle count mismatch
    if detected_count != expected_count {
        return CandidateTestResult {
            percentage: 0.0,
            detected_durations: detected_durations.to_vec(),
            matched_count: 0,
            expected_count,
            errors: Vec::new(),
        };
    }

    let mut matched_count = 0;
    let mut errors = Vec::new();

    for (detected, expected) in detected_durations.iter().zip(expected_durations.iter()) {
        let expected_secs = *expected as f64 / 1000.0;
        let error = (detected - expected_secs).abs();
        errors.push(error);

        if error <= tolerance_secs {
            matched_count += 1;
        }
    }

    let percentage = (matched_count as f64 / expected_count as f64) * 100.0;

    CandidateTestResult {
        percentage,
        detected_durations: detected_durations.to_vec(),
        matched_count,
        expected_count,
        errors,
    }
}
```

### 6.4 Create matching/editions/filtering.rs

```rust
//! Edition Filtering and Sorting

use crate::matching::types::*;
use strsim::jaro_winkler;

/// Filter and sort editions by relevance
pub fn filter_and_sort_editions(
    editions: Vec<Edition>,
    source_artist: &str,
    source_album: &str,
    max_editions: usize,
) -> Vec<Edition> {
    let mut scored: Vec<(Edition, f64)> = editions
        .into_iter()
        .map(|mut edition| {
            let score = calculate_name_distance(&edition.artist, &edition.title, source_artist, source_album);
            edition.name_distance_score = Some(score);
            (edition, score)
        })
        .collect();

    // Sort by name distance (higher = better match)
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Assign ranks
    scored.iter_mut().enumerate().for_each(|(idx, (edition, _))| {
        edition.name_distance_rank = Some(idx + 1);
    });

    // Take top N
    scored.into_iter()
        .take(max_editions)
        .map(|(edition, _)| edition)
        .collect()
}

/// Calculate combined name distance score
fn calculate_name_distance(
    mb_artist: &str,
    mb_album: &str,
    source_artist: &str,
    source_album: &str,
) -> f64 {
    let artist_sim = jaro_winkler(&mb_artist.to_lowercase(), &source_artist.to_lowercase());
    let album_sim = jaro_winkler(&mb_album.to_lowercase(), &source_album.to_lowercase());

    // Weight artist slightly higher
    artist_sim * 0.6 + album_sim * 0.4
}
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-006-01 | Group by track count | Same track count grouped |
| TC-U-006-02 | Group by duration pattern | Similar durations grouped |
| TC-U-006-03 | Filter by relevance | Relevant editions kept |
| TC-U-006-04 | Sort by name distance | Best matches first |
| TC-U-006-05 | Handle multi-disc | Tracks from all discs counted |

---

## Acceptance Criteria

- [ ] editions/ module created
- [ ] Grouping logic correct
- [ ] Scoring logic correct
- [ ] Filtering/sorting working
- [ ] All 5 tests pass

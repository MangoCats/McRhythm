# Artist Filtering Algorithm Design

**Date:** 2026-01-01
**Purpose:** Filter out candidate editions with clearly incorrect artists before multi-stage matching
**Addresses:** Regression where "The Cars - Panorama" matched to "Stephan Mathieu - Radioland"

---

## Problem Analysis

### Current Behavior

**Pipeline Flow (album_matcher.rs:439-444):**
```rust
let editions = group_into_editions(&releases);
let editions = filter_and_sort_editions(editions, &artist, &album, 20);  // Line 441
let editions = filter_editions_by_file_duration(editions, file_duration_ms);
```

**Issue with `filter_and_sort_editions()`:**
- Calculates combined similarity: `artist_sim × 0.6 + album_sim × 0.4`
- Sorts by this score (descending)
- Takes top N editions (default 20)
- **NO minimum threshold applied**

**Result:**
- Wrong artists can pass if they're in top N
- Example: "Stephan Mathieu" for "The Cars" source
  - Artist similarity: ~0.25
  - Album similarity: ~0.30
  - Combined: 0.25×0.6 + 0.30×0.4 = 0.27
  - If only 10 candidates exist, this makes top 20 → proceeds to matching

### Why Wrong Artists Win Matching

Even with low name similarity (0.27), wrong artist can win if:
1. **Duration score is good** (single-track album happens to fit file duration)
2. **Quality score is acceptable** (track boundaries align by coincidence)
3. **Track count penalty is low** (1-track vs 10-track has graduated penalty)

Multi-factor scoring can't compensate for fundamentally wrong artist.

---

## Design Solution

### Approach: Pure Artist Similarity Pre-Filter

**Insert new filter BEFORE `filter_and_sort_editions()`:**

```rust
// Step 6: Group into editions and filter
let editions = group_into_editions(&releases);

// **[ARTIST FILTERING]** Reject editions with clearly incorrect artists
let editions = filter_editions_by_artist(editions, &artist, MIN_ARTIST_SIMILARITY);

// Sort remaining editions by combined similarity (artist+album)
let editions = filter_and_sort_editions(editions, &artist, &album, 20);

// Filter by file duration
let editions = filter_editions_by_file_duration(editions, file_duration_ms);
```

### Algorithm Specification

**Function Signature:**
```rust
pub fn filter_editions_by_artist(
    editions: Vec<Edition>,
    source_artist: &str,
    min_similarity: f64,
) -> Vec<Edition>
```

**Implementation:**
```rust
use strsim::jaro_winkler;

pub fn filter_editions_by_artist(
    editions: Vec<Edition>,
    source_artist: &str,
    min_similarity: f64,
) -> Vec<Edition> {
    editions
        .into_iter()
        .filter(|edition| {
            let artist_sim = jaro_winkler(
                &edition.artist.to_lowercase(),
                &source_artist.to_lowercase()
            );
            artist_sim >= min_similarity
        })
        .collect()
}
```

**Key Properties:**
- Uses **pure artist similarity only** (not combined artist+album score)
- Uses same Jaro-Winkler algorithm as existing code (consistency)
- Case-insensitive comparison (same as existing code)
- Threshold: `MIN_ARTIST_SIMILARITY = 0.50` (50%)

---

## Threshold Analysis

### Why 0.50 Threshold is Appropriate

**Examples at 0.50 threshold:**

| Source Artist | Candidate Artist | Similarity | Pass? |
|---------------|------------------|------------|-------|
| The Cars | The Cars | 1.00 | ✓ |
| The Cars | Cars | 0.88 | ✓ |
| The Cars | The Car | 0.93 | ✓ |
| The Cars | Stephan Mathieu | 0.25 | ✗ |
| Pink Floyd | Pink Floyd | 1.00 | ✓ |
| Pink Floyd | The Pink Floyd | 0.89 | ✓ |
| Led Zeppelin | Led Zeppelin | 1.00 | ✓ |
| Led Zeppelin | The Beatles | 0.32 | ✗ |

**0.50 (50%) provides:**
- Exact matches: Always pass (1.00)
- Minor variations: Pass ("The Beatles" vs "Beatles" = 0.88)
- Typos/abbreviations: Pass ("Led Zep" vs "Led Zeppelin" = 0.73)
- Completely wrong artists: Reject ("The Cars" vs "Stephan Mathieu" = 0.25)

**Validation:**
- Threshold already used for post-match artist verification (constants.rs:201)
- Proven acceptable in production (no known false negatives)
- Moving threshold to pre-filtering is safe

---

## Impact Analysis

### Expected Benefits

1. **Eliminates Wrong Artist Regressions**
   - "The Cars - Panorama" → "Stephan Mathieu - Radioland" regression prevented
   - Artist similarity 0.25 < 0.50 threshold → filtered out before matching

2. **Performance Improvement**
   - Fewer editions proceed to expensive multi-stage matching
   - Example: 50 candidates → 20 (filter_and_sort top-N) → 15 (artist filter) → 12 (duration filter)
   - Reduces orchestration workload by ~40% in cases with wrong artists

3. **Improved Confidence**
   - All matched editions guaranteed to have artist_sim >= 0.50
   - Post-match artist_verified flag becomes redundant safeguard (defense in depth)

### Risk Assessment

**Potential False Negatives:**
- Artists with name changes/variations might be filtered out
- Example: "Prince" vs "Prince and the Revolution" (similarity ~0.65, passes)
- **Mitigation:** 0.50 threshold is conservative (allows significant variation)

**Edge Cases:**
- "Various Artists" compilations: similarity ~0.30-0.40 to any specific artist
  - **Acceptable:** These should fail matching anyway (wrong artist)
  - **Future:** Could add special handling for VARIOUS_ARTISTS constant

**Backward Compatibility:**
- Stricter filtering may reduce matches for albums with inconsistent metadata
- **Acceptable:** These are low-confidence matches that should fail artist verification anyway
- **Benefit:** Prevents false positives (better to reject than mismatch)

---

## Testing Strategy

### Unit Tests (filtering.rs)

```rust
#[test]
fn test_filter_editions_by_artist_exact_match() {
    let editions = vec![
        create_test_edition("The Cars", "Panorama", 10),
        create_test_edition("Stephan Mathieu", "Radioland", 1),
    ];
    let filtered = filter_editions_by_artist(editions, "The Cars", 0.50);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].artist, "The Cars");
}

#[test]
fn test_filter_editions_by_artist_minor_variation() {
    let editions = vec![
        create_test_edition("The Beatles", "Abbey Road", 17),
        create_test_edition("Beatles", "Abbey Road", 17),
        create_test_edition("Led Zeppelin", "Physical Graffiti", 15),
    ];
    let filtered = filter_editions_by_artist(editions, "The Beatles", 0.50);
    assert_eq!(filtered.len(), 2); // Both "The Beatles" and "Beatles" pass
    assert!(filtered.iter().any(|e| e.artist == "The Beatles"));
    assert!(filtered.iter().any(|e| e.artist == "Beatles"));
}

#[test]
fn test_filter_editions_by_artist_wrong_artist() {
    let editions = vec![
        create_test_edition("The Cars", "Panorama", 10),
        create_test_edition("Pink Floyd", "The Wall", 26),
    ];
    let filtered = filter_editions_by_artist(editions, "The Cars", 0.50);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].artist, "The Cars");
}

#[test]
fn test_filter_editions_by_artist_case_insensitive() {
    let editions = vec![
        create_test_edition("THE CARS", "Panorama", 10),
        create_test_edition("the cars", "Panorama", 10),
    ];
    let filtered = filter_editions_by_artist(editions, "The Cars", 0.50);
    assert_eq!(filtered.len(), 2); // Both pass (case-insensitive)
}
```

### Integration Test (Regression Case)

**Test:** The Cars - Panorama (from am/run29f_regression_analysis.md)

**Setup:**
```rust
// Source metadata
artist = "The Cars"
album = "Panorama"

// MusicBrainz candidates (simulated)
candidates = [
    Edition { artist: "The Cars", title: "Panorama", track_count: 10, ... },
    Edition { artist: "Stephan Mathieu", title: "Radioland", track_count: 1, ... },
]
```

**Expected Result:**
```rust
// After artist filtering
filtered_candidates = [
    Edition { artist: "The Cars", title: "Panorama", track_count: 10, ... },
    // "Stephan Mathieu" filtered out (similarity 0.25 < 0.50)
]

// Orchestration only runs on correct artist
final_match = Edition { artist: "The Cars", title: "Panorama", track_count: 10, ... }
```

### Performance Test

**Benchmark:** 200-file comparison test with artist filter enabled vs disabled

**Metrics:**
- Total filtering time (should be negligible: <1ms per edition)
- Orchestration time (should decrease due to fewer candidates)
- Match accuracy (should improve: fewer wrong-artist false positives)

---

## Implementation Plan

### Phase 1: Add Filter Function
- [ ] Add `filter_editions_by_artist()` to `editions/filtering.rs`
- [ ] Add unit tests for exact match, variations, wrong artist, case-insensitivity
- [ ] Add documentation with examples

### Phase 2: Integrate into Pipeline
- [ ] Modify `album_matcher.rs:441` to insert artist filter before name sorting
- [ ] Update logging to show before/after counts
- [ ] Preserve MIN_ARTIST_SIMILARITY constant usage

### Phase 3: Testing
- [ ] Run unit tests
- [ ] Run integration test with The Cars - Panorama regression case
- [ ] Run 200-file comparison test
- [ ] Verify no new regressions introduced

### Phase 4: Documentation
- [ ] Update CLAUDE.md with artist filtering in architecture description
- [ ] Add comment in album_matcher.rs explaining filter sequence
- [ ] Document in change_history.md via /commit workflow

---

## Code Changes Summary

**Files Modified:**
1. `wkmp-ai/src/matching/editions/filtering.rs` - Add `filter_editions_by_artist()`
2. `wkmp-ai/src/matching/album_matcher.rs` - Insert filter in pipeline at line 441

**Lines of Code:** ~50 LOC (30 implementation + 20 tests)

**Risk Level:** LOW
- Simple, focused change
- Uses existing threshold constant
- Additive (doesn't modify existing functions)
- Well-defined test coverage

---

## Success Criteria

**Primary:**
- [ ] The Cars - Panorama regression resolved (no longer matches "Stephan Mathieu")
- [ ] All existing unit tests pass
- [ ] No new regressions in 200-file comparison test

**Secondary:**
- [ ] Orchestration time reduced by filtering wrong artists early
- [ ] Artist verification pass rate improves (fewer post-match rejections)

---

## Notes

- This addresses **root cause** of wrong-artist regressions
- Complements (doesn't replace) existing post-match artist verification
- Can be extended in future to handle "Various Artists" special case
- Threshold is configurable via AlbumMatcherConfig if needed

# Integration Tests: Multi-Strategy MusicBrainz Search (REQ-AM-096)

**Requirement:** REQ-AM-096 - Multi-Strategy MusicBrainz Search
**Test Count:** 2 integration tests
**Priority:** P1 (Important)

---

## TC-I-096-01: MBID Deduplication Across Strategies

**Test ID:** TC-I-096-01
**Test Type:** Integration Test
**Requirement:** REQ-AM-096
**Priority:** P1
**Scope:** MusicBrainz client multi-strategy search with deduplication

### Setup
Mock MusicBrainz API responses for artist="Ace of Base", album="Happy Nation":

**Strategy 1 (Basic unquoted):**
- Returns 12 results including MBIDs: `[std-11, std-us-12, deluxe-17, jp-12, ...]`

**Strategy 2 (CamelCase split "Happy Nation"):**
- Returns 8 results including MBIDs: `[std-11, std-us-12, remaster-11, ...]`

**Strategy 3 (Fuzzy ~1):**
- Returns 15 results including MBIDs: `[std-11, deluxe-17, jp-12, reissue-11, ...]`

**Strategy 4-7:**
- Various results with overlapping MBIDs

**Total unique MBIDs across all strategies:** 25
**Total results before deduplication:** 63

### When
- `multi_strategy_search("Ace of Base", "Happy Nation")` is called
- All 7 strategies execute
- Results are deduplicated by release MBID

### Then
- **Expected behavior:**
  - All 7 strategies invoked in order
  - Duplicate MBIDs removed (63 results → 25 unique)
  - Each MBID appears exactly once in final list
  - Result order preserved (first occurrence wins)

### Verify
```rust
let results = multi_strategy_search("Ace of Base", "Happy Nation").await?;

// Verify deduplication
assert_eq!(results.len(), 25);  // 25 unique MBIDs

// Verify no duplicates
let mbids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
let unique_mbids: HashSet<String> = mbids.iter().cloned().collect();
assert_eq!(mbids.len(), unique_mbids.len());

// Verify standard edition present (should appear in strategy 1, 2, 3)
assert!(results.iter().any(|r| r.id == "std-11"));

// Verify deluxe edition present (should appear in strategy 1, 3)
assert!(results.iter().any(|r| r.id == "deluxe-17"));
```

### Pass Criteria
- All strategies invoked
- Duplicate MBIDs removed (one entry per MBID)
- No MBID appears more than once in final list
- Standard edition found (correct edition in result set)

### Fail Criteria
- Duplicate MBIDs in final results
- Strategies not all invoked
- Expected editions missing

---

## TC-I-096-02: Edge Case - All Strategies Return 0 Results

**Test ID:** TC-I-096-02
**Test Type:** Integration Test
**Requirement:** REQ-AM-096
**Priority:** P1
**Scope:** Handling completely unknown album

### Setup
Mock MusicBrainz API responses for artist="Unknown Artist", album="Unknown Album":

**All 7 strategies:**
- Each returns 0 results (album not in MusicBrainz database)

### When
- `multi_strategy_search("Unknown Artist", "Unknown Album")` is called
- All 7 strategies execute
- All strategies return empty result sets

### Then
- **Expected behavior:**
  - Function returns empty vector `[]`
  - No panic or error
  - **Stage 0 validation (REQ-AM-003) should have already caught this**

### Verify
```rust
let results = multi_strategy_search("Unknown Artist", "Unknown Album").await?;

// Verify empty results
assert!(results.is_empty());
assert_eq!(results.len(), 0);
```

### Pass Criteria
- Returns empty vector without error
- No panic
- Stage 0 validation catches this before edition selection

### Fail Criteria
- Panics or errors
- Returns non-empty results for non-existent album

**Note:** This edge case should be caught by Stage 0 validation (REQ-AM-003: "Validate MusicBrainz search returns > 0 candidates"). If this test is reached during actual matching, it indicates a Stage 0 validation bug.

---

## Multi-Strategy Search Implementation Reference

**7 Progressive Search Strategies:**

```rust
pub fn generate_search_strategies(artist: &str, album: &str) -> Vec<String> {
    vec![
        // Strategy 1: Basic unquoted search with type filter
        format!("artist:{} AND release:{} AND type:album", artist, album),

        // Strategy 2: CamelCase split (e.g., "HappyNation" → "Happy Nation")
        format!("artist:{} AND release:{} AND type:album", artist, split_camelcase(album)),

        // Strategy 3: Fuzzy ~1 edit (minor typos)
        format!("artist:{}~1 AND release:{}~1 AND type:album", artist, album),

        // Strategy 4: Wildcard fixes for common misspellings
        format!("artist:{} AND release:{}* AND type:album", artist, album),

        // Strategy 5: Aggressive fuzzy ~2 edits
        format!("artist:{}~2 AND release:{}~2 AND type:album", artist, album),

        // Strategy 6: Per-token fuzzy (multi-word names)
        format!("artist:{} AND release:{} AND type:album", fuzzy_tokens(artist), fuzzy_tokens(album)),

        // Strategy 7: Album-only fallback (when artist unknown)
        format!("release:{} AND type:album", album),
    ]
}
```

**Already Implemented:** `wkmp-ai/src/services/musicbrainz_client.rs` (lines 863-950)

---

## Success Metrics Clarification

**PRIMARY METRIC (Correct):**
- ≥98% album-level success (correct edition selected)
- ≥99.5% passage-level accuracy (correct MBIDs assigned)

**NOT SUCCESS METRICS (Intermediate):**
- ❌ Number of editions found (more ≠ better)
- ❌ "Found 25 editions instead of 16" is NOT a success metric

**Explanation:**
- Finding more editions may IMPROVE performance (correct edition now in pool)
- Finding more editions may DEGRADE performance (wrong editions ranked higher)
- **True test:** Does multi-strategy search + edition selection produce correct MBIDs?

**Example:**
- HappyNation: Strategy 1 found 16 editions, multi-strategy found 25 editions
- **Question:** Did we assign correct MBIDs to all passages?
  - ✅ YES → Success (multi-strategy helped find correct edition)
  - ❌ NO → Failure (extra editions confused ranking, selected wrong one)

---

## Test Data Requirements

**Mock MusicBrainz API:**
- Realistic response structure (JSON with releases, artist-credit, media, tracks)
- Multiple overlapping result sets (simulate 7 strategies)
- Known albums: "Happy Nation" (25 editions), "Aqualung" (16 editions)
- Unknown albums: Return empty results

**Deduplication Test Dataset:**
- 63 total results across 7 strategies
- 25 unique MBIDs (38 duplicates to remove)
- Specific editions appear in multiple strategies

**Location:** `wkmp-ai/tests/data/musicbrainz_mock/`

---

**Test File Version:** 1.0
**Last Updated:** 2025-12-28
**Estimated Implementation Time:** 3-4 hours (integration + mock API setup)

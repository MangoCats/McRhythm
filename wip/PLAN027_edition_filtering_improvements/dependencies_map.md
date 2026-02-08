# Dependencies Map - Edition Filtering Improvements

**Plan:** PLAN027
**Date:** 2026-01-16

---

## Dependency Overview

**Status:** All required dependencies exist. No new external libraries needed.

**Impact:** Low-risk implementation. Changes are isolated to edition filtering logic.

---

## Existing Code Dependencies (Read-Only)

### 1. MusicBrainz Client Module
**Location:** `wkmp-ai/src/services/musicbrainz_client.rs`
**Status:** ✅ Exists (no changes required)
**Interface Used:**
- Edition metadata already fetched (title, track count, track list)
- Artist metadata already available per track
- No additional API queries needed

**What We Get:**
```rust
struct Edition {
    mbid: String,
    title: String,           // Used for keyword matching
    track_count: usize,      // Used for pre-filtering
    tracks: Vec<Track>,      // Used for artist consistency
}

struct Track {
    title: String,           // Used for remix detection
    artist: String,          // Used for artist consistency
    duration_ms: u64,
}
```

### 2. Existing Matching Pipeline
**Location:** `wkmp-ai/src/matching/mod.rs`
**Status:** ✅ Exists (integration point)
**Interface Used:**
- 7-stage matching pipeline orchestration
- Edition candidate list preparation
- Final MBID selection logic

**Integration Point:**
Edition filtering will be applied BEFORE Stage 2 (album-first matching):
```rust
// Existing flow:
let editions = fetch_editions_from_musicbrainz(album_artist, album_title);
let best_match = run_7_stage_pipeline(editions, audio_data);

// New flow:
let editions = fetch_editions_from_musicbrainz(album_artist, album_title);
let filtered = apply_edition_filters(editions, detected_track_count);  // NEW
let best_match = run_7_stage_pipeline(filtered, audio_data);
```

### 3. Stage 2 - Album-First Matching
**Location:** `wkmp-ai/src/matching/stages/stage2_album_first.rs`
**Status:** ✅ Exists (will be modified)
**Current Behavior:**
- Receives list of candidate editions
- Runs album-first matching algorithm
- Returns best match with confidence score

**Required Change:**
Apply track count pre-filtering before running matching algorithm:
```rust
pub fn stage2_album_first(
    editions: &[Edition],           // Input: All editions from MusicBrainz
    detected_boundaries: usize,      // Input: Number of tracks detected
    // ... other params
) -> Option<MatchResult> {
    // NEW: Filter by track count
    let filtered = filter_by_track_count(editions, detected_boundaries);

    // Existing matching logic operates on filtered list
    run_album_first_matching(filtered, ...)
}
```

### 4. Progressive RMS Optimization
**Location:** `wkmp-ai/src/matching/stages/boundary_refinement.rs`
**Status:** ✅ Exists (recently completed, no changes)
**Dependency Type:** Indirect (used by Stage 7, not by filtering)

**Why It Matters:**
Boundary refinement produces accurate track boundaries, which edition filtering uses to validate track count. Accurate boundary detection → accurate track count → better filtering decisions.

### 5. Database Models
**Location:** `wkmp-common/src/db_models.rs` (or similar)
**Status:** ✅ Exists (no changes required)
**Interface Used:**
- No database writes during edition filtering
- Edition filtering is in-memory only
- Matching results stored after selection (existing code unchanged)

---

## Code to Create

### 1. Edition Filter Module
**Location:** `wkmp-ai/src/matching/edition_filter.rs` (NEW)
**Purpose:** Centralized edition filtering and scoring logic
**Exports:**
```rust
/// Apply all edition filters (track count, artist consistency)
pub fn filter_editions(
    editions: &[Edition],
    detected_track_count: usize,
    source_artist: &str,
) -> Vec<Edition>;

/// Score edition preference (deluxe penalty, compilation penalty)
pub fn score_edition_preference(
    edition: &Edition,
    detected_track_count: usize,
) -> f64;

/// Validate artist consistency (reject multi-artist compilations)
pub fn validate_artist_consistency(
    edition: &Edition,
    source_artist: &str,
) -> bool;

/// Check if track is a remix (for error tolerance)
pub fn is_remix_track(title: &str) -> bool;
```

**Alternative Approach:**
Add functions to `wkmp-ai/src/matching/stages/mod.rs` instead of new module. Tradeoff:
- ✅ Fewer files
- ❌ Less modular
- **Decision:** New module preferred for testability

### 2. Unit Tests
**Location:** `wkmp-ai/src/matching/edition_filter.rs` (inline tests) or `wkmp-ai/tests/edition_filter_test.rs`
**Purpose:** Verify filtering logic correctness
**Test Cases:**
- Track count filtering (±3 tolerance)
- Artist consistency validation (multi-artist rejection)
- Edition scoring (deluxe/compilation penalties)
- Remix detection (keyword matching)
- Edge cases (empty lists, exact matches, boundary conditions)

### 3. Integration Points (Modifications)
**Location:** `wkmp-ai/src/matching/stages/stage2_album_first.rs`
**Change Type:** Add pre-filtering call before matching
**Lines Changed:** ~10-15 lines

**Location:** `wkmp-ai/src/matching/stages/mod.rs`
**Change Type:** Export new filtering module
**Lines Changed:** ~2 lines

---

## External Dependencies (No Changes)

### 1. MusicBrainz API
**Status:** ✅ Stable (read-only access)
**What We Use:**
- Release/edition metadata (title, track count, track list)
- Artist metadata (per-track artist names)
- No new API calls required

**Constraint:**
- Rate limiting: 1 request/second (already handled by client)
- No changes to query patterns

### 2. Rust Standard Library
**Status:** ✅ Available (no new crates)
**What We Use:**
- `String::to_lowercase()` for case-insensitive matching
- `str::contains()` for keyword detection
- `HashSet<String>` for unique artist extraction
- No external crates needed (pure stdlib)

### 3. Logging Framework
**Status:** ✅ Available (`log` crate, already in use)
**What We Use:**
- `debug!()` macro for filtering decisions
- `trace!()` macro for detailed scoring breakdown
- Existing log configuration unchanged

---

## Dependency Graph

```
MusicBrainz API (external, read-only)
         ↓
musicbrainz_client.rs (existing, no changes)
         ↓
     editions: Vec<Edition>
         ↓
edition_filter.rs (NEW) ← detected_track_count, source_artist
         ↓
  filtered_editions: Vec<Edition>
         ↓
stage2_album_first.rs (modified: add filtering call)
         ↓
7-stage matching pipeline (existing, no changes)
         ↓
     best_match: MatchResult
```

**Key Insight:** Edition filtering is a pure function with no side effects. Inputs: edition metadata + detected track count. Output: filtered edition list. No database access, no API calls, no state mutation.

---

## Integration Complexity Assessment

### Low Complexity
- ✅ Edition filtering is a pure function (no side effects)
- ✅ Integration point is well-defined (before Stage 2)
- ✅ All required data already available (no new data fetching)
- ✅ No database schema changes
- ✅ No API contract changes

### Medium Complexity
- ⚠️ Need to handle edge cases (all editions filtered, empty edition list)
- ⚠️ Logging must not degrade performance (DEBUG level only)
- ⚠️ Edition scoring formula may need tuning based on test results

### Mitigations
- Fallback to unfiltered list if all editions rejected
- Lazy logging (construct log messages only if DEBUG enabled)
- Conservative penalty multipliers (0.7, 0.6) with room to adjust

---

## Backward Compatibility

**Question:** Do existing imports break with new filtering logic?

**Answer:** No breaking changes.
- Edition filtering is transparent to user
- Existing albums remain unchanged (no re-matching)
- New imports use new algorithm automatically
- If new algorithm selects different MBID, user sees improved results (not a regression)

**Migration:** None required. Changes take effect on next import.

---

## Testing Dependencies

### 1. Test Suite
**Location:** `wkmp-ai/tests/` (integration tests)
**Status:** ✅ Exists
**What We Need:**
- Full 200-album test suite (baseline: run29f_baseline_1-12-26)
- 4 problem albums as regression tests
- Comparison scripts (already exist in Python)

### 2. Test Data
**Location:** `C:\Users\Mango Cat\Music` (local test library)
**Status:** ✅ Available (200 albums)
**Critical Albums:**
- Imagine Dragons - Night Visions (track count mismatch)
- Michael Jackson - Thriller (compilation mismatch)
- James Gang - Funk #49 (multi-artist mismatch)
- Chemical Brothers - Surrender (remix tolerance)

### 3. Baseline Comparison
**Location:** `wkmp-ai/baseline_1-12-26_stats.json`
**Status:** ✅ Exists
**What It Provides:**
- 186/200 albums matched (93% baseline)
- Track-level error data
- MBID selections for comparison

---

## Environment Dependencies

### Build Environment
- **Rust:** 1.70+ (stable channel)
- **Cargo:** Standard Rust build system
- **OS:** Windows (development), Linux (deployment target)

**No Changes Required:** All dependencies already satisfied.

### Runtime Environment
- **Database:** SQLite with JSON1 extension (already required)
- **Network:** MusicBrainz API access (already required)
- **Audio Files:** User's music library (already present)

**No Changes Required:** Edition filtering adds no new runtime dependencies.

---

## Dependency Risk Assessment

| Dependency | Type | Status | Risk | Mitigation |
|------------|------|--------|------|------------|
| MusicBrainz API | External | Stable | Low | Read-only, rate-limited, cached |
| musicbrainz_client.rs | Internal | Stable | Low | No changes required |
| Matching pipeline | Internal | Stable | Low | Integration point well-defined |
| Progressive RMS | Internal | New | Medium | Recently completed, monitor for issues |
| Test suite | Internal | Stable | Low | Baseline comparison available |

**Overall Risk:** Low. All dependencies are stable or recently validated.

---

## Future Dependencies (Out of Scope)

**NOT Required for This Plan:**
- Machine learning libraries (no ML-based selection)
- Genre classification (no genre-based filtering)
- User preference storage (no personalization)
- Advanced text analysis (simple keyword matching sufficient)

**May Be Needed Later:**
- Fuzzy string matching library (better artist name matching)
- Statistical modeling (confidence intervals for scoring)
- User feedback mechanism (learn from manual corrections)

**Decision:** Keep implementation simple for now. Add complexity only if data shows it's needed.

---

## Summary

**Existing Dependencies:** All satisfied. No new external libraries required.

**Code to Create:** ~300-400 lines (edition_filter.rs + tests + integration)

**Integration Points:** 1 primary (stage2_album_first.rs), 1 export (stages/mod.rs)

**Risk Level:** Low. Pure functions, well-defined interfaces, comprehensive testing available.

**Ready to Proceed:** ✅ All dependencies mapped. No blockers identified.

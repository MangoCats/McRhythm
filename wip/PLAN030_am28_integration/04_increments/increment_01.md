# Increment 1: Types Migration

**Estimated Effort:** 3 hours
**Dependencies:** None (foundation increment)
**Deliverables:** Enhanced matching/types.rs with all am28 types

---

## Objective

Merge am28 type definitions into the existing `wkmp-ai/src/matching/types.rs`, creating a unified type system for album matching.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| types.rs | 783 | Merge/adapt |

---

## Tasks

### 1.1 Analyze Type Conflicts

Compare existing types with am28 types:

**Existing (keep as primary):**
- `MatchingStage` - Already defined, matches am28
- `MatchedTrack` - Keep, enhance if needed
- `Edition` - Merge with am28 version (add missing fields)
- `AlbumMatchResult` - Keep, map am28 `ValidationResult` to it

**New from am28 (add):**
- `CacheMode`, `CacheConfig`, `CacheStats` - Caching support
- `CachedSearch`, `CachedRelease`, `CacheMetadata` - Cache storage
- `MBSearchResponse`, `MBRelease`, `MBArtistCredit`, `MBArtist` - MB API
- `MBReleaseDetails`, `MBMedia`, `MBTrack`, `MBRecording` - MB details
- `SilenceCache` - Pre-computed silence detection
- `CandidateTestResult`, `EditionTestResult` - Stage results

### 1.2 Merge Edition Type

```rust
// Existing Edition (matching/types.rs:76-98)
pub struct Edition {
    pub release_mbid: String,
    pub title: String,
    pub artist: String,
    pub artist_credit: Option<String>,  // existing
    pub country: Option<String>,        // existing
    pub status: Option<String>,         // existing
    pub track_count: usize,
    pub track_durations: Vec<f64>,
    pub recording_mbids: Vec<String>,
}

// Add from am28:
pub struct Edition {
    // ... existing fields ...
    pub name_distance_rank: Option<usize>,   // NEW from am28
    pub name_distance_score: Option<f64>,    // NEW from am28
    pub durations: Vec<u32>,                 // NEW: milliseconds version
}
```

### 1.3 Add Cache Types

```rust
/// Cache mode configuration
#[derive(Debug, Clone, Copy)]
pub enum CacheMode {
    Disabled,
    ReadWrite,
    ReadOnly,
}

/// MusicBrainz response caching configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub mode: CacheMode,
    pub cache_dir: PathBuf,
}
```

### 1.4 Add MusicBrainz API Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MBSearchResponse {
    pub releases: Vec<MBRelease>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MBRelease {
    pub id: String,
    pub title: String,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<MBArtistCredit>>,
    pub country: Option<String>,
    pub status: Option<String>,
}
// ... etc
```

### 1.5 Add Stage Result Types

```rust
/// Result from testing a single parameter combination
#[derive(Debug, Clone)]
pub struct CandidateTestResult {
    pub percentage: f64,
    pub detected_durations: Vec<f64>,
    pub matched_count: usize,
    pub expected_count: usize,
    pub errors: Vec<f64>,
}

/// Result from testing one edition through all stages
#[derive(Debug, Clone)]
pub struct EditionTestResult {
    pub edition_idx: usize,
    pub best_percentage: f64,
    pub best_result: Option<CandidateTestResult>,
    pub best_stage: String,
    pub best_threshold: Option<f64>,
    pub best_min_duration: Option<f64>,
    pub expected_durations: Vec<u32>,
    pub log_messages: Vec<String>,
}
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-001-01 | Edition serialization roundtrip | Serialize → deserialize equals original |
| TC-U-001-02 | CacheConfig validation | Invalid paths rejected |
| TC-U-001-03 | MBSearchResponse parsing | Parse sample JSON correctly |
| TC-U-001-04 | CandidateTestResult construction | All fields populated |

---

## Acceptance Criteria

- [ ] All existing types preserved (no breaking changes)
- [ ] All am28 types added
- [ ] Serde derive for serializable types
- [ ] `cargo check` passes
- [ ] All 4 tests pass
- [ ] No compiler warnings in types.rs

---

## Implementation Notes

1. **Import organization:** Group by domain (cache, MB API, stages)
2. **Visibility:** Use `pub` for all types (library API)
3. **Documentation:** Add doc comments for all new types
4. **Naming:** Keep am28 names where sensible, align with wkmp conventions otherwise

# SPEC: Content Type Determination

**Purpose:** Define algorithms for classifying audio files by content type (single song, full album, multiple songs, or unidentified) during wkmp-ai import processing.

**Parent document:** [refactor1122.md](refactor1122.md) Step 6

**Related:** album_matcher_20.rs (reference implementation for album path)

---

## Status Outcomes

| Status | Description |
|--------|-------------|
| SINGLE_SONG | One song identified with MusicBrainz Recording MBID |
| FULL_ALBUM | All/most tracks matched to a MusicBrainz Release MBID (≥95% match) |
| PARTIAL_ALBUM | Most tracks matched but some missing/extra (75-94% match) |
| MULTIPLE_SONGS | Multiple recordings identified, no single release association |
| NOT_IN_MUSICBRAINZ | Audio content present but no MusicBrainz identification possible |
| IDENTIFICATION_FAILED | Processing completed but low confidence in any classification |

---

## Algorithm Overview

```
┌─────────────────────────────────────┐
│  Input: Decoded audio + metadata    │
│  (from Step 5 AUDIO_DECODED)        │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│  1. Duration-based triage           │
│     Route to appropriate path       │
└──────────────┬──────────────────────┘
               │
       ┌───────┴───────┬───────────────┐
       │               │               │
   < 12 min       12-25 min        > 25 min
       │               │               │
       ▼               ▼               ▼
┌────────────┐  ┌────────────┐  ┌────────────┐
│Single-song │  │ Dual-path  │  │Album path  │
│   path     │  │            │  │            │
└─────┬──────┘  └─────┬──────┘  └─────┬──────┘
      │               │               │
      ▼               ▼               ▼
┌─────────────────────────────────────┐
│  2. Classification result           │
│     Update files table status       │
└─────────────────────────────────────┘
```

---

## 1. Duration-Based Triage

| Runtime | Classification | Primary Path | Rationale |
|---------|----------------|--------------|-----------|
| < 12 min | Likely single song | Single-song path | Typical song 2-8 min; allows for extended mixes |
| 12-25 min | Ambiguous | Dual-path | EP territory; could be long composition or short album |
| 25-80 min | Likely full album | Album path | Standard album length |
| > 80 min | Multi-disc/compilation | Album path | May need disc-aware processing |

**Constants:**
```rust
const DURATION_SINGLE_SONG_MAX_SECS: f64 = 720.0;   // 12 minutes
const DURATION_AMBIGUOUS_MAX_SECS: f64 = 1500.0;    // 25 minutes
const DURATION_LIKELY_MULTIDC_SECS: f64 = 4800.0;   // 80 minutes
```

---

## 2. Quick Silence Scan

Before MusicBrainz lookup, estimate segment count using relaxed parameters.

**Purpose:** Inform path selection and provide early track count estimate.

**Parameters:**
```rust
const QUICK_SCAN_THRESHOLD_DB: f64 = -45.0;      // Permissive (catch obvious gaps only)
const QUICK_SCAN_MIN_GAP_SECS: f64 = 2.0;        // Only clear inter-track gaps
const QUICK_SCAN_RMS_WINDOW_SECS: f64 = 0.1;     // 100ms window
```

**Interpretation:**

| Segments Detected | Interpretation | Action |
|-------------------|----------------|--------|
| 1 | Strong single-song candidate | Prioritize single-song path |
| 2-4 | Ambiguous | Consider medley, EP, or movements |
| 5+ | Strong album candidate | Prioritize album path |

**Note:** This scan reuses the single-pass `WindowDbProfile` technique from album_matcher_20.rs for efficiency.

---

## 3. Single-Song Path

For files classified as likely single songs (< 12 min OR 1 segment detected).

### 3.1 Chromaprint Generation

```rust
const CHROMAPRINT_DURATION_SECS: u32 = 120;  // First 2 minutes sufficient
```

1. Extract first 120 seconds of audio (or full file if shorter)
2. Compute chromaprint fingerprint
3. Store fingerprint in files table for future use

### 3.2 AcoustID Lookup

**API:** `https://api.acoustid.org/v2/lookup`

**Parameters:**
- `client`: Application API key
- `duration`: Audio duration in seconds
- `fingerprint`: Chromaprint result
- `meta`: `recordings` (request MusicBrainz Recording data)

**Rate limit:** 3 requests/second (more permissive than MusicBrainz)

### 3.3 AcoustID Response Processing

```rust
const ACOUSTID_HIGH_CONFIDENCE: f64 = 0.80;    // Strong match
const ACOUSTID_MEDIUM_CONFIDENCE: f64 = 0.40;  // Possible match, needs validation
const ACOUSTID_LOW_CONFIDENCE: f64 = 0.20;     // Weak match
```

| AcoustID Score | Action |
|----------------|--------|
| ≥ 0.80 | Accept as SINGLE_SONG with Recording MBID |
| 0.40 - 0.79 | Cross-validate with ID3 metadata; accept if artist/title match |
| 0.20 - 0.39 | Low confidence; try album path as fallback |
| < 0.20 or no match | Mark NOT_IN_MUSICBRAINZ or try album path |

### 3.4 Metadata Cross-Validation

When AcoustID confidence is medium (0.40-0.79):

1. Compare AcoustID recording title to ID3 `TIT2`/`TITLE`
2. Compare AcoustID artist to ID3 `TPE1`/`ARTIST`
3. Use Levenshtein distance with threshold

```rust
const TITLE_MATCH_THRESHOLD: usize = 5;   // Max edit distance for title match
const ARTIST_MATCH_THRESHOLD: usize = 8;  // Max edit distance for artist match
```

If both title AND artist match within thresholds → accept as SINGLE_SONG.

---

## 4. Album Path

For files classified as likely albums (> 25 min OR 5+ segments detected).

Adapted from album_matcher_20.rs stages 2-5.

### 4.1 Metadata Reconciliation

**Sources:**
1. Internal metadata (ID3, Vorbis, MP4, etc.)
2. Path/filename parsing

**Reconciliation strategies:**
- `DirectMatch`: ID3 and path agree
- `PartialMatch`: Partial overlap, chose one source
- `Conflict`: Sources disagree, applied heuristics
- `GapFill`: One source missing, used other
- `PathOnly`: No internal tags, path only

**Output:** Artist/album variants for MusicBrainz search

### 4.2 MusicBrainz Release Search

**Search strategies** (try multiple, deduplicate results):
1. Exact artist + album
2. Album only (for compilations)
3. Artist only + filter by track count
4. Fuzzy variants (handle "The" prefix, camelCase splits)

**API rate limit:** 1 request/second (use 1.5s delay for safety)

```rust
const MB_RATE_LIMIT_MS: u64 = 1550;
const MB_MAX_RELEASES: usize = 150;
```

### 4.3 Edition Grouping and Filtering

Group MusicBrainz releases into "editions" by track count + duration signature.

**Runtime filter:**
```rust
const RUNTIME_FILTER_MIN_RATIO: f64 = 0.75;  // Edition must be ≥75% of file duration
const RUNTIME_FILTER_MAX_RATIO: f64 = 1.25;  // Edition must be ≤125% of file duration
```

**Name Distance Ranking (NDR):**
- Calculate Levenshtein distance from each edition to source artist/album
- Filter editions with NDR > 100 (poor name similarity)
- Sort by combined runtime match + name similarity

### 4.4 Edition Testing (Stages 2-5)

For each candidate edition (in order of likelihood):

**Stage 2 - Parameter Optimization:**
- Test silence detection across parameter grid (threshold × min_duration)
- Compare detected track durations to edition's expected durations
- Collect over-segmented candidates for Stage 3

```rust
const STAGE2_THRESHOLD_VALUES: [f64; 12] = [
    -50.0, -58.0, -47.0, -54.0, -38.0, -56.0,
    -42.0, -52.0, -34.0, -60.0, -36.0, -30.0
];
const STAGE2_MIN_DURATION_VALUES: [f64; 15] = [
    0.05, 4.0, 0.10, 3.0, 0.15, 2.5, 0.2,
    2.0, 0.25, 1.5, 0.3, 1.0, 0.4, 0.8, 0.5
];
```

**Stage 3 - Segment Assembly:**
- Dynamic programming to assemble over-segmented candidates
- Merge adjacent segments to match expected track count

**Stage 4 - Quiet Spot Detection:**
- Use edition track durations as guide
- Find quietest point within search radius of expected boundary
- Apply penalty (results never exceed 75% confidence)

**Stage 5 - Extra Track Merging:**
- When detected > expected AND match ≥100%
- Merge shortest adjacent pair iteratively

### 4.5 Match Quality Classification

```rust
const MATCH_TOLERANCE_SECS: f64 = 10.0;         // Track duration tolerance
const CONFIDENCE_EXCELLENT: f64 = 95.0;         // → FULL_ALBUM
const CONFIDENCE_GOOD: f64 = 75.0;              // → PARTIAL_ALBUM
const CONFIDENCE_FAIR: f64 = 50.0;              // → investigate further
const CONFIDENCE_POOR: f64 = 50.0;              // → fallback to segment analysis
```

| Match % | Mean Error | Classification |
|---------|------------|----------------|
| ≥ 95% | < 5s | FULL_ALBUM |
| 75-94% | < 10s | PARTIAL_ALBUM |
| 50-74% | varies | Try fallback (section 5) |
| < 50% | varies | Try fallback (section 5) |

### 4.6 Early Exit

If 100% match found with mean_error < 5s:
- Signal early exit
- Allow grace period for parallel editions to complete
- Select best result (lowest mean_error among 100% matches)

```rust
const EARLY_EXIT_GRACE_PERIOD_SECS: u64 = 20;
```

---

## 5. Fallback: Per-Segment Analysis

When album matching fails (< 50%) but multiple segments detected.

### 5.1 Segment Extraction

For each detected segment:
1. Extract segment audio with ±1 second buffer
2. Compute chromaprint for segment
3. Query AcoustID

### 5.2 Result Analysis

| Scenario | Classification |
|----------|----------------|
| Most segments match recordings from DIFFERENT releases | MULTIPLE_SONGS |
| Most segments match recordings from SAME release | Retry album matching with that specific release |
| Most segments have no AcoustID match | NOT_IN_MUSICBRAINZ |
| Mixed results, low confidence | IDENTIFICATION_FAILED |

**Thresholds:**
```rust
const SEGMENT_MATCH_MAJORITY: f64 = 0.60;      // 60% of segments must match for conclusion
const SEGMENT_SAME_RELEASE_THRESHOLD: f64 = 0.80;  // 80% from same release → album
```

---

## 6. Dual-Path Processing (12-25 min files)

For ambiguous duration files, run both paths and compare results.

### 6.1 Parallel Execution

1. Start single-song path (AcoustID lookup)
2. Start album path (MusicBrainz Release search)
3. Compare results

### 6.2 Result Selection

| Single-Song Result | Album Result | Decision |
|--------------------|--------------|----------|
| High confidence (≥0.80) | Any | SINGLE_SONG |
| Medium confidence | ≥75% match | Compare; prefer higher confidence |
| Medium confidence | <50% match | SINGLE_SONG (if ≥0.40) |
| Low/no match | ≥75% match | FULL_ALBUM or PARTIAL_ALBUM |
| Low/no match | <50% match | Fallback analysis or NOT_IN_MUSICBRAINZ |

---

## 7. Confidence Signals

Weight these signals for edge cases:

| Signal | Weight | Notes |
|--------|--------|-------|
| AcoustID score | High | Direct acoustic fingerprint match |
| Album match % | High | Track duration alignment |
| Track count match | High | Detected vs expected tracks |
| Runtime ratio | High | File vs edition duration |
| Name distance (NDR) | Medium | Metadata similarity |
| ID3 TRCK tag format | Medium | "1/12" suggests single from album |
| ID3 TALB present/absent | Low | May indicate album vs single |
| Segment count | Medium | Quick scan estimate |

---

## 8. Database Updates

Upon classification completion, update files table:

| Field | Value |
|-------|-------|
| status | One of the status outcomes above |
| content_type | "single_song", "full_album", "partial_album", "multiple_songs", "unknown" |
| classification_confidence | 0.0-1.0 normalized confidence score |
| classification_method | "acoustid", "album_match", "fallback_segment", "dual_path" |
| primary_mbid | Recording MBID (single) or Release MBID (album) |
| match_percentage | For album matches, the track match % |
| mean_error_secs | For album matches, mean duration error |

---

## 9. Constants Summary

```rust
// Duration triage
const DURATION_SINGLE_SONG_MAX_SECS: f64 = 720.0;
const DURATION_AMBIGUOUS_MAX_SECS: f64 = 1500.0;

// Quick silence scan
const QUICK_SCAN_THRESHOLD_DB: f64 = -45.0;
const QUICK_SCAN_MIN_GAP_SECS: f64 = 2.0;

// AcoustID confidence
const ACOUSTID_HIGH_CONFIDENCE: f64 = 0.80;
const ACOUSTID_MEDIUM_CONFIDENCE: f64 = 0.40;

// Album matching
const MATCH_TOLERANCE_SECS: f64 = 10.0;
const RUNTIME_FILTER_MIN_RATIO: f64 = 0.75;
const RUNTIME_FILTER_MAX_RATIO: f64 = 1.25;

// Classification thresholds
const CONFIDENCE_EXCELLENT: f64 = 95.0;
const CONFIDENCE_GOOD: f64 = 75.0;
const CONFIDENCE_FAIR: f64 = 50.0;

// API rate limits
const MB_RATE_LIMIT_MS: u64 = 1550;
const ACOUSTID_RATE_LIMIT_MS: u64 = 334;  // ~3/sec

// Early exit
const EARLY_EXIT_GRACE_PERIOD_SECS: u64 = 20;
```

---

## 10. Error Handling

| Error Condition | Action |
|-----------------|--------|
| AcoustID API unavailable | Skip single-song path; rely on album path |
| MusicBrainz API unavailable | Retry with exponential backoff; mark IDENTIFICATION_FAILED after max retries |
| Decode failure | Should not reach Step 6 (caught in Step 5) |
| No segments detected | Treat as single segment; try single-song path |
| All editions filtered out | Mark NOT_IN_MUSICBRAINZ or IDENTIFICATION_FAILED |

---

## References

- album_matcher_20.rs: Reference implementation for album matching stages
- AcoustID API: https://acoustid.org/webservice
- MusicBrainz API: https://musicbrainz.org/doc/MusicBrainz_API
- Chromaprint: https://acoustid.org/chromaprint

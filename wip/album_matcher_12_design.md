# Album Matcher 12 - Algorithmic Design

## Overview

Album Matcher identifies MusicBrainz releases for concatenated album MP3 files by:
1. Detecting track boundaries via silence/RMS analysis
2. Matching detected track durations against MusicBrainz edition data
3. Using progressive refinement through 5 stages to maximize match quality

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         INPUT: Album MP3 File                        │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│              PHASE 0: Metadata Extraction & Reconciliation           │
│  - Extract ID3 tags (ffprobe)                                        │
│  - Parse path for artist/album                                       │
│  - Reconcile conflicts, generate search variants                     │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                MUSICBRAINZ SEARCH (Comprehensive)                    │
│  - Query with ALL artist/album variants                              │
│  - Up to 150 releases fetched                                        │
│  - Group into unique editions by duration signature                  │
│  - Filter by runtime (±25% of file duration)                         │
│  - Sort by match likelihood (duration + track count)                 │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    5-STAGE PROGRESSIVE MATCHING                      │
│                                                                      │
│  Stage 1: Initial Detection (default params)                         │
│      │                                                               │
│      ▼  if < 100%                                                    │
│  Stage 2: Parameter Optimization (180 combinations)                  │
│      │   + Collect over-segmented candidates                         │
│      ▼  if < 100%                                                    │
│  Stage 3: Comprehensive Assembly (DP on all candidates)              │
│      │                                                               │
│      ▼  if < 100%                                                    │
│  Stage 4: Quiet Spot Detection (RMS-based)                           │
│      │                                                               │
│      ▼  if detected > expected && quality ≥ 100%                     │
│  Stage 5: Extra Track Merging                                        │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          OUTPUT: Best Match                          │
│  - MusicBrainz MBID + URL                                            │
│  - Track-by-track match details                                      │
│  - Match percentage + confidence level                               │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Phase 0: Metadata Extraction & Reconciliation

### Purpose
Extract artist/album names from multiple sources and reconcile conflicts.

### Sources
1. **ID3 Tags** - via ffprobe JSON output
2. **File Path** - pattern: `.../Artist/Album.mp3`

### Reconciliation Strategies
| Scenario | Strategy | Confidence |
|----------|----------|------------|
| Both match | DirectMatch | High |
| One matches | PartialMatch | Medium |
| Both conflict | Conflict (heuristic choice) | Low |
| One missing | GapFill | Low |

### Heuristics
- Avoid "Various Artists" if alternative available
- Prefer ID3 for compilations (greatest hits, anthology)
- Prefer ID3 if contains edition details (Deluxe, Remaster)

### Output
- Primary artist/album for search
- Alternate variants for expanded search
- Estimated track count (from ID3 comment field)

---

## MusicBrainz Search

### Search Strategy Sequence
1. `artist:"X" AND release:"Y"` (strict)
2. `artist:"X" release:"Y"` (relaxed)
3. `"X" AND "Y"` (any field)
4. `X Y` (full text)

### Edition Grouping
Multiple MBIDs may represent the same edition (US vs UK release). Group by:
- Track count
- Duration signature (all track durations concatenated)

### Edition Scoring
```
base_score = |edition_duration - file_duration|
           + |edition_tracks - estimated_tracks| × 60s

priority_bonus = -50 (CD) + -40 (Official) + -30 (US)

final_score = base_score + priority_bonus
```
Lower score = better match.

### Name Distance Ranking (NDR)
Levenshtein distance from each candidate to source artist/album names.
- Weighted: album × 2.0, artist × 1.0
- Ranks 1-N displayed in logging for debugging

---

## Stage 1: Initial Detection

### Algorithm
1. Decode MP3 to PCM samples (symphonia)
2. Detect silence regions using default parameters:
   - Threshold: -57 dB
   - Min duration: 0.9 seconds
3. Convert silence boundaries to track durations
4. Test against ALL edition candidates
5. Return best match

### Silence Detection
```
for each RMS window:
    db = 20 × log10(rms)
    if db < threshold_db:
        mark as silent

merge adjacent silent windows
filter by min_duration_secs
```

### Adaptive Window Sizing
| min_duration | RMS window |
|--------------|------------|
| ≤ 0.3s | 25ms |
| 0.3-0.6s | 50ms |
| > 0.6s | 100ms |

---

## Stage 2: Parameter Optimization (Run 12 Enhanced)

### Purpose
Find optimal silence detection parameters through grid search.

### Parameter Grid
- **Thresholds**: -36, -38, -40, -42, -44, -47, -50, -52, -54, -56, -58, -60 dB
- **Min durations**: 0.05, 0.10, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.8, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0 s
- **Total**: 180 combinations

### Run 12 Enhancement
**Problem (Run 10/11):** Only kept single best result, discarding over-segmented candidates.

**Solution (Run 12):** Collect ALL over-segmented candidates for Stage 3.

```rust
struct Stage2Results {
    best_result: Option<CandidateTestResult>,      // Best immediate match
    over_segmented_candidates: Vec<OverSegmentedCandidate>,  // For Stage 3
}
```

### Over-Segmentation Detection
```
if detected_track_count > max_expected_tracks:
    collect candidate for Stage 3 assembly
```

---

## Stage 3: Comprehensive Segment Assembly (Run 12 Fixed)

### Purpose
Assemble over-segmented results into correct track counts via dynamic programming.

### Run 12 Fix
**Problem (Run 10/11):** Only 14-16 assemblies tested (from single best result).

**Solution (Run 12):** Test ALL collected over-segmented candidates (~100+ assemblies).

### Dynamic Programming Algorithm
```
Goal: Group N detected segments into K expected tracks
      minimizing total duration error

dp[i][j] = minimum error when grouping first i segments into j tracks

Recurrence:
dp[i][j] = min over all start positions s:
    dp[s][j-1] + |sum(segments[s..i]) - expected[j]|

Backtrack to reconstruct optimal grouping.
```

### Assembly Loop (Run 12)
```
for each over_segmented_candidate:
    for each target_edition:
        if candidate.segments > edition.tracks:
            assembled = assemble_segments_dp(candidate, edition)
            test assembled against ALL editions
            if improved: update best
            if 100%: early exit
```

### Funk #49 Case Study
| Run | Stage 2 Best | Over-Seg Candidates | Stage 3 Assemblies | Final |
|-----|--------------|---------------------|-------------------|-------|
| 7 | 50% (10 trk) | 109 | 85 tested | **100%** |
| 10 | 50% (10 trk) | 0 (discarded) | 14 tested | 50% |
| 11 | 50% (10 trk) | 0 (discarded) | 16 tested | 50% |
| 12 | 50% (10 trk) | ~100 (collected) | ~200 tested | **100%** |

---

## Stage 4: Quiet Spot Detection

### Purpose
Alternative segmentation when silence-based detection fails.

### Algorithm
1. Calculate RMS for 500ms windows (25% overlap)
2. Compute mean and standard deviation of RMS values
3. Find local minima below `mean - std_dev`
4. Select N-1 quietest spots (for N tracks)
5. Convert positions to track durations

### Use Case
Albums with continuous audio (live recordings, concept albums) where true silence doesn't exist.

---

## Stage 5: Extra Track Merging

### Purpose
Correct over-detection when match quality is already good.

### Trigger Conditions
- `detected_tracks > expected_tracks`
- `match_percentage ≥ 100%` (all expected tracks matched)

### Algorithm
```
for each adjacent pair (i, i+1):
    merged = tracks[..i] + [tracks[i] + tracks[i+1]] + tracks[i+2..]
    error = sum of |merged[j] - expected[j]| for all j
    track best merge by minimum error

apply best merge
```

---

## Configuration Constants

```rust
// Silence Detection
DEFAULT_THRESHOLD_DB: -57.0
DEFAULT_MIN_DURATION_SECS: 0.9
MATCH_TOLERANCE_SECS: 10.0

// MusicBrainz API
MB_RATE_LIMIT_SECS: 2
MB_MAX_RELEASES: 150
MB_SEARCH_LIMIT: 100

// Scoring
SCORE_CD_BONUS: -50.0
SCORE_OFFICIAL_BONUS: -40.0
SCORE_US_BONUS: -30.0
SCORE_TRACK_COUNT_PENALTY: 60.0

// Runtime Filter
RUNTIME_FILTER_MIN_RATIO: 0.75  // 75%
RUNTIME_FILTER_MAX_RATIO: 1.25  // 125%

// Confidence Thresholds
CONFIDENCE_EXCELLENT: ≥80%
CONFIDENCE_GOOD: 60-79%
CONFIDENCE_FAIR: 40-59%
CONFIDENCE_POOR: <40%
```

---

## Output Format

### ValidationResult Structure
```rust
{
    album_path: String,
    artist: String,
    album: String,
    mbid: String,
    musicbrainz_url: String,
    expected_track_count: usize,
    detected_track_count: usize,
    perfect_count_match: bool,
    track_matches: Vec<TrackMatch>,
    extra_tracks: Vec<ExtraTrack>,
    matched_tracks_count: usize,
    match_percentage: f64,
    mean_error: f64,
    matching_stage: String,
    confidence: String,
}
```

### Track Match Detail
```rust
{
    track_number: usize,
    detected_duration: f64,
    expected_duration: u32,
    error: f64,
    matches: bool,  // error ≤ MATCH_TOLERANCE_SECS
}
```

---

## Performance Characteristics

| Metric | Typical Value |
|--------|---------------|
| Albums per hour | ~30-40 (rate-limited by MusicBrainz API) |
| Stage 2 combinations | 180 |
| Stage 3 assemblies | 50-200 (Run 12) |
| Memory per album | ~50-100 MB (decoded audio) |
| MusicBrainz queries per album | 10-50 |

---

## Version History

| Run | Key Changes |
|-----|-------------|
| 7 | Baseline with comprehensive assembly |
| 8 | Added Stage 5 extra track merging |
| 10 | Refactored architecture, broke assembly |
| 11 | Added NDR logging, same architecture as 10 |
| **12** | **Fixed assembly regression, code cleanup** |

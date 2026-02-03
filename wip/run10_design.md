# Album Matcher - Run 10 Design

## Overview

Run 10 implements a comprehensive MusicBrainz search architecture with edition-based matching. The core principle: **retrieve all reasonable album candidates upfront, group by edition, prioritize by file characteristics, then exhaustively test until 100% match is found.**

## Key Architectural Changes from Run 9

### 1. Comprehensive Upfront MusicBrainz Search
**Previous (Run 9):** Limited initial MusicBrainz query with 3-5 candidates
**Run 10:** Exhaustive search using ALL strategies and name variants, up to 150 releases

### 2. Edition-Based Matching
**Previous:** Committed to specific MBID early, tested against that single release
**Run 10:** Group releases into editions, test all editions, select best MBID from winner

### 3. Edition Prioritization
**Previous:** Tested editions in arbitrary order (typically by track count)
**Run 10:** Sort editions by match likelihood using file duration + track count scoring

### 4. No Premature Exit
**Previous:** Stopped at first "good enough" match
**Run 10:** Continue through all stages and editions until 100% match or exhaustion

## Architecture Components

### Phase 0: ID3/Path Metadata Reconciliation
**Unchanged from Run 9**

- Extract artist/album from ID3 tags and file path
- Reconcile discrepancies
- Generate primary and alternate name variants for search

### Phase 1: Comprehensive MusicBrainz Search

#### Search Strategies (All 7 Applied)
1. `artist:{artist} AND release:{album}`
2. `artist:{artist} AND releaseaccent:{album}`
3. `artistname:{artist} AND release:{album}`
4. `artistname:{artist} AND releaseaccent:{album}`
5. `creditname:{artist} AND release:{album}`
6. `creditname:{artist} AND releaseaccent:{album}`
7. `release:{album}` (album-only fallback)

#### Name Variant Expansion
- **Artist variants:** Primary artist + alternate artist (from reconciliation)
- **Album variants:** Primary album + alternate album (from reconciliation)
- **Search combinations:** artist_variants × album_variants × 7 strategies

#### Query Limits
- Maximum 150 unique releases (by MBID)
- MusicBrainz API rate limiting: 1 request/second
- Deduplication by MBID to avoid redundant queries

#### Metadata Extraction
For each release, extract:
- Track durations (in seconds)
- MBID
- Country
- Release status (Official, Promotion, etc.)
- Media format (CD detection via `format === "CD"`)
- Artist name (from `artist_credit[0].artist.name`)
- Album title

### Phase 2: Edition Grouping

#### Edition Definition
An **edition** is uniquely identified by:
- Track count
- Exact duration pattern (e.g., "107,125,135,...")

**Key principle:** Multiple MBIDs can represent the same edition (US release, UK release, remaster with identical durations, etc.)

#### Grouping Algorithm
```
For each release:
  Create signature = "track_count:duration1,duration2,..."
  If signature exists in editions:
    Add MBID to existing edition
  Else:
    Create new edition with this signature
```

#### Edition Structure
```rust
struct Edition {
    track_count: usize,
    durations: Vec<u32>,
    mbids: Vec<EditionMBID>,          // Multiple MBIDs per edition
    duration_signature: String,
    artist: String,
    album: String,
}

struct EditionMBID {
    mbid: String,
    country: Option<String>,
    status: Option<String>,
    is_cd: bool,
}
```

### Phase 3: Edition Prioritization

#### Match Scoring Formula
```
score = |edition_duration - file_duration| + (|edition_tracks - file_tracks| × 60)
```

- Lower score = better match
- Each track count difference = 60 seconds runtime error
- Track penalty only applied if file track count is known

#### Sorting
Editions sorted by ascending score (best matches first)

#### Example Scoring
File: 3826 seconds, 18 tracks

| Edition | Tracks | Duration | Track Diff | Duration Diff | Score |
|---------|--------|----------|------------|---------------|-------|
| A       | 18     | 3842s    | 0          | 16s           | 16s   |
| B       | 16     | 2961s    | 2          | 865s          | 985s  |
| C       | 22     | 4200s    | 4          | 374s          | 614s  |

Edition A tested first (score: 16s)

#### Runtime Length Filtering
After sorting, editions are filtered to remove those with >25% runtime difference from audio file.

**Filter formula:**
```
min_duration = file_duration × 0.75
max_duration = file_duration × 1.25
Keep only editions where: min_duration ≤ edition_duration ≤ max_duration
```

**Examples:**
- Audio file: 100 minutes → Keep editions 75-125 minutes
- Audio file: 60 minutes → Keep editions 45-75 minutes

**Rationale:** Editions with vastly different runtimes cannot match the audio file, regardless of segmentation parameters. Filtering eliminates impossible candidates early, dramatically reducing testing overhead.

**Impact:** For Jessita Reyes (3826s file), a 2-track edition (500s) or 50-track edition (10000s) would be filtered out immediately.

### Phase 4: Multi-Stage Testing

All editions tested through 5 stages until 100% match found.

#### Stage 1: Initial Detection (Default Parameters)
- Threshold: -40 dB
- Min duration: 5 seconds
- Quick baseline test

#### Stage 2: Parameter Optimization
- Test 180 combinations (threshold × min_duration)
- Thresholds: -36, -38, -40, ..., -70 dB (18 values)
- Min durations: 1, 2, 3, ..., 10 seconds (10 values)

#### Stage 3: Segment Assembly
- Detect quiet gaps between tracks
- Attempt to merge segments to match expected track boundaries
- Handles cases where silence detection fragmented single track

#### Stage 4: Quiet Spot Detection
- Find sub-threshold quiet regions within expected track boundaries
- Refine segmentation based on MusicBrainz track structure

#### Stage 5: Extra Track Merging
- Merge extra detected segments into adjacent tracks
- Handles false positive silence detections

#### Early Exit Condition
If any stage achieves 100% match, stop testing remaining editions and stages.

### Phase 5: MBID Selection

After identifying winning edition, select best MBID using metadata prioritization.

#### Selection Criteria (Priority Order)
1. **CD media format:** -50 points
2. **Official status:** -40 points
3. **US country:** -30 points

#### Selection Algorithm
```
If single MBID in edition:
  Return that MBID
Else:
  Score each MBID
  Return MBID with lowest (most negative) score
```

#### Example MBID Selection
Edition has 3 MBIDs:

| MBID | Country | Status   | Format | Score        |
|------|---------|----------|--------|--------------|
| A    | US      | Official | CD     | -50-40-30=-120 |
| B    | UK      | Official | CD     | -50-40=-90    |
| C    | US      | Bootleg  | Vinyl  | -30          |

Select MBID A (lowest score: -120)

## Implementation Details

### File Locations
- Implementation: `wkmp-ai/examples/album_matcher.rs`
- Input files: `training_set.txt`, `long_files_list.txt`
- Output: `album_matcher_output_run10.txt`

### Key Functions

#### `comprehensive_musicbrainz_search()`
- Lines 901-1053
- Returns: `Vec<(Vec<u32>, EditionMBID, String, String)>`
- Implements all 7 search strategies across all name variants
- Deduplicates by MBID, limits to 150 releases

#### `group_into_editions()`
- Lines 1055-1090
- Groups releases by track count + duration pattern
- Returns: `Vec<Edition>`

#### `score_edition_match()`
- Lines 1092-1110
- Calculates edition match score vs. file characteristics
- Returns: `f64` (lower = better)

#### `select_best_mbid()`
- Lines 1112-1149
- Selects best MBID from edition using metadata priorities
- Returns: `String` (MBID)

### Main Loop Integration
Lines 2400-2480:
1. Calculate file characteristics (duration, track count)
2. Build artist/album variant lists
3. Comprehensive MusicBrainz search
4. Group into editions
5. Sort editions by match score
6. Filter editions by runtime (±25%)
7. Display edition details
8. Test through 5 stages

## Design Rationale

### Why Comprehensive Upfront Search?

**Problem:** Run 7 succeeded on Jessita Reyes via "Expanded MB Search" Stage 5, which queried MusicBrainz with additional strategies. Run 9 eliminated this stage, causing regression.

**Solution:** Merge Phase 0 + Stage 5 search strategies into single comprehensive upfront search. Get all candidates before matching begins.

**Benefit:** Ensures correct edition is in candidate set before expensive segmentation testing begins.

### Why Edition-Based Matching?

**Problem:** Different MBIDs (US/UK releases, remasters) may represent identical track structures. Testing each MBID separately wastes computation.

**Solution:** Group MBIDs by track count + duration pattern. Test each unique edition once.

**Benefit:**
- Reduced computation (test 125 unique editions vs 150 individual MBIDs)
- Correct matching logic (edition matters, not MBID)
- Defer MBID selection until winning edition is identified

### Why Edition Prioritization?

**Problem:** Testing 125 editions × 180 parameter combinations = 22,500 tests at Stage 2. Most editions are wrong (different track counts, very different durations).

**Solution:** Test editions most likely to match first (closest to file duration/track count).

**Benefit:**
- Early 100% match exit reduces wasted computation
- For Jessita Reyes: 18-track edition (score: 16s) tested before 16-track edition (score: 985s)
- Dramatically improves expected performance

### Why Runtime Length Filtering?

**Problem:** Editions with vastly different runtimes are impossible matches (e.g., 2-track 500s edition vs 60-minute file). Testing them wastes computation.

**Solution:** Filter out editions >25% different from file duration before testing begins.

**Benefit:**
- Eliminates impossible candidates immediately
- Reduces test matrix size (e.g., 125 editions → 40 editions after filtering)
- Dramatic performance improvement: 40 × 180 = 7,200 tests vs 125 × 180 = 22,500 tests (68% reduction)
- 25% threshold is generous enough to allow for different CD pressings, bonus tracks, slight duration variations

**Example filtering results:**
- Jessita Reyes (3826s, 18 tracks): Keep editions 2870-4783s (~48-80 minutes)
- Filtered: Very short compilations, single-track versions, double-album editions

### Why No Premature Exit?

**Problem:** Previous runs stopped at "good enough" (e.g., 95% match), potentially missing perfect matches.

**Solution:** Continue testing until 100% match or all options exhausted.

**Benefit:** Maximizes match quality, provides clear success/failure signal.

## Expected Performance

### Best Case
- File: 18-track album, standard CD duration
- MusicBrainz: Contains exact 18-track edition
- Result: Edition #0 (best match) achieves 100% at Stage 1 or 2
- Time: ~1-5 seconds (minimal testing)

### Typical Case
- File: Multi-track album with some segmentation challenges
- MusicBrainz: Correct edition exists but requires parameter optimization
- Result: Edition #0-5 achieves 100% at Stage 2-3
- Time: ~30-120 seconds (moderate testing)

### Worst Case
- File: Obscure album, poor metadata, complex segmentation
- MusicBrainz: Correct edition exists but requires Stage 4-5
- Result: 100% match after testing many editions/stages
- Time: Variable (may take hours for exhaustive testing of 125 editions × 180 parameters)

### Failure Case
- File: Album not in MusicBrainz, or no edition matches track structure
- Result: Best partial match (highest %) reported after exhausting all options
- Time: Full test completion (all editions × all stages)

## Output Format

### Edition Display
```
Edition Details (sorted by match likelihood):
  [0] Jessita Reyes - Native American Flute Lullabies (18 tracks, 3842s, 5 MBIDs, score: 16s)
  [1] Jessita Reyes - Native American Flute Lullabies (16 tracks, 2961s, 2 MBIDs, score: 985s)
  ...
```

### Stage Progress
```
STAGE 1: Initial detection (-40dB, 5s)...
  Testing 125 editions...
  Best match: 88.9% (16/18 tracks)

STAGE 2: Parameter optimization...
  Testing 180 parameter combinations across 125 editions...
  Best match: 100.0% (18/18 tracks) - MATCH FOUND
  → Edition: Jessita Reyes - Native American Flute Lullabies (18 tracks)

Matching stage: Parameter Optimization
Best parameters: -58dB, 2.5s
```

### Final Result
```
FINAL RESULT:
  Matching stage: Parameter Optimization
  MusicBrainz: https://musicbrainz.org/release/f2c9e523-02b9-4e70-84a9-dbc2bdabde68
  Expected tracks: 18
  Detected tracks: 18
  Perfect count match: YES
  Match percentage: 100.0%
  Mean error: 0.8s
  Confidence: High
```

## Test Configuration

### Input Files
- **training_set.txt:** 200 hand-selected albums with known-good metadata
- **long_files_list.txt:** Long-duration multi-track files

### Parameters
- Threshold range: -36 dB to -70 dB (18 values)
- Min duration range: 1-10 seconds (10 values)
- Match tolerance: 5 seconds
- MusicBrainz limit: 150 releases
- Runtime filter: ±25% of file duration
- No timeout: Albums tested exhaustively until completion

## Success Metrics

### Primary Goal
Achieve 100% match rate on 200-album training set where correct edition exists in MusicBrainz.

### Expected Improvements Over Run 9
- **Jessita Reyes:** Run 9 predicted 13.6% failure → Run 10 expected 100% success
- **Other regressions:** Albums that failed due to limited MB search should now succeed
- **Overall match rate:** Target >90% perfect matches (up from Run 9 baseline)

### Secondary Goals
- Edition prioritization reduces average test time
- Clear output showing which editions were tested
- Comprehensive search ensures no "missed" editions due to search strategy gaps

## Known Limitations

### MusicBrainz Coverage
If album not in MusicBrainz, or no edition matches actual track structure, matching will fail. This is expected and acceptable.

### Computational Cost
Comprehensive search + edition testing is expensive (up to 10 minutes per album). Acceptable for training/validation, would need optimization for production use.

### False Positive Editions
MusicBrainz may return unrelated albums with similar names. Edition prioritization mitigates this by testing likely matches first.

## Future Enhancements

### Possible Optimizations
1. **Parallel edition testing:** Test top N editions concurrently
2. **Adaptive stage skipping:** Skip stages if early stages achieve high confidence
3. **Edition caching:** Cache MusicBrainz edition data across runs
4. **Incremental parameter search:** Binary search for optimal parameters vs exhaustive grid

### Possible Features
1. **Confidence scoring:** Quantify match confidence based on multiple factors
2. **Ambiguity detection:** Report when multiple editions achieve similar match %
3. **Manual review queue:** Flag low-confidence matches for human verification

## Version History

- **Run 7:** Initial success with Expanded MB Search (Stage 5)
- **Run 8:** Edition-oriented identification experiments
- **Run 9:** Refactored with f64 precision, eliminated Stage 5 (regression)
- **Run 10:** Comprehensive search architecture (this design)

## References

- Implementation: `wkmp-ai/examples/album_matcher.rs`
- Run 7 output: `album_matcher_output_run7.txt`
- Run 9 output: `album_matcher_output_run9.txt`
- Test case: Jessita Reyes album (demonstrating regression and fix)

# Album Matcher Design - Run 25 Series

## Overview

The album matcher identifies MusicBrainz editions for audio files using a multi-stage pipeline that combines name similarity (NDR ranking), runtime matching, and acoustic fingerprinting.

**Key Innovation:** Edition ranking happens BEFORE fetching detailed track info, minimizing expensive API calls.

---

## Pipeline Architecture

### Stage 0: Search MusicBrainz
- **Input:** Artist/album name variants from filename
- **Output:** `Vec<MBRelease>` (basic metadata: MBID, title, artist, country, status)
- **API:** `/ws/2/release/?query=...&limit=100`
- **Data Available:** Album/artist names only (no track durations yet)

### Stage 1: Levenshtein Ratio Filter (Run 25c: Combination A)
- **Input:** All releases from search
- **Purpose:** Pre-filter releases with poor name similarity
- **Algorithm:**
  - For "Various Artists": Require album ratio ≥ 35%
  - For regular albums: Require weighted combined score ≥ 42%
    - Combined = artist_ratio × 0.4 + album_ratio × 0.6
- **Output:** Filtered releases (typically 50-100 → 10-30)

**Constants:**
```rust
MIN_COMBINED_RATIO: f64 = 0.42;        // Weighted threshold for regular albums
ARTIST_WEIGHT: f64 = 0.4;              // Artist contributes 40% to combined score
ALBUM_WEIGHT: f64 = 0.6;               // Album contributes 60% to combined score
MIN_VARIOUS_ALBUM_RATIO: f64 = 0.35;   // Album-only threshold for compilations
```

### Stage 2: NDR Calculation
- **Input:** Ratio-filtered releases
- **Purpose:** Calculate Name Distance Rank for all candidates
- **Algorithm:**
  - For each release: `score = calculate_name_distance(artist, album, variants)`
  - Sort by score (ascending - lower is better)
  - Assign ranks 1, 2, 3...

**Name Distance Calculation:**
- Finds best Levenshtein ratio for artist (across all variants)
- Finds best Levenshtein ratio for album (across all variants)
- Combines: `score = (1.0 - artist_ratio) + (1.0 - album_ratio)`
- Lower score = better name match

### Stage 3: NDR Rank Filter
- **Input:** NDR-scored releases
- **Purpose:** Limit editions to test (API rate limiting)
- **Filter:** Keep only ranks 1-50 (configurable via `MAX_NAME_DISTANCE_RANK`)
- **Output:** Top-ranked releases ready for detailed fetch

**Constant:**
```rust
MAX_NAME_DISTANCE_RANK: usize = 50;  // Test top 50 editions
```

**Optimization Opportunity:** Run 23 analysis shows 96.9% of albums win within top 10 editions. Could reduce to 25 with minimal impact (~95% success rate).

### Stage 4: Fetch Track Details
- **Input:** Top-ranked releases (by NDR)
- **Purpose:** Get track durations and recording MBIDs
- **API:** `/ws/2/release/{mbid}?inc=recordings`
- **Output:** `Vec<MBReleaseDetails>` with durations
- **Rate Limiting:** 1 request/second (MusicBrainz policy)

**Data Retrieved:**
- Track count
- Individual track durations (milliseconds)
- Recording MBIDs (for AcoustID verification)
- Media format (CD vs. Digital)

### Stage 5: Group by Duration Signature
- **Input:** Detailed editions
- **Purpose:** Deduplicate editions with identical track sequences
- **Algorithm:**
  - Hash: track_count + sum of durations + 5 longest track durations
  - Group editions with same hash
  - Preserve best NDR rank per group

**Result:** Unique editions by content (e.g., US CD vs. EU CD with same tracks)

### Stage 6: Runtime Likelihood Sort
- **Input:** Grouped editions
- **Purpose:** Prioritize editions matching file's total duration
- **Algorithm:**
  ```rust
  score = |edition_duration - file_duration| + 60 × |edition_tracks - file_tracks|
  ```
- **Output:** Editions sorted by runtime likelihood (lower score = better match)

### Stage 7: Runtime Filter
- **Input:** Runtime-sorted editions
- **Purpose:** Eliminate editions with impossible durations
- **Filter:** Keep editions within ±25% of file duration
- **Output:** Viable candidates for testing

**Constants:**
```rust
RUNTIME_FILTER_MIN_RATIO: f64 = 0.75;  // 75% of file duration
RUNTIME_FILTER_MAX_RATIO: f64 = 1.25;  // 125% of file duration
```

### Stage 8: Name Similarity Re-sort (Conservative Bubble Sort)
- **Input:** Runtime-filtered editions
- **Purpose:** Allow significantly better name matches to bubble up
- **Algorithm:** Swap editions if `name_score[i] > 2.0 × name_score[i+1]`
- **Rationale:** Preserve runtime ordering unless name match is dramatically better

### Stage 9: Parallel Acoustic Fingerprinting
- **Input:** Final sorted editions (typically 3-15)
- **Purpose:** Test each edition against audio file using AcoustID
- **Process:**
  - Test editions in parallel (4-second stagger for "early exit" optimization)
  - For each edition, try parameter grid:
    - Silence thresholds: -50dB to -20dB (6 steps)
    - Gap widths: 0.05s to 3.0s (7 steps)
  - Stop testing edition if 100% match found
- **Output:** Match percentage and mean timing error per edition

**Parameter Grid:**
```rust
SILENCE_THRESHOLDS_DB: [-50, -45, -40, -35, -30, -20]
GAP_WIDTHS_SECONDS: [0.05, 0.1, 0.2, 0.3, 0.5, 2.0, 3.0]
```

### Stage 10: Winner Selection
- **Input:** Acoustic fingerprint results for all editions
- **Primary:** Sort by match percentage (descending)
- **Secondary:** Sort by mean timing error (ascending)
- **Fallback Check:** If winner has poor name match, evaluate runner-ups
  - Runner-up qualifies if: significantly better name match AND time-fit not significantly worse
- **Output:** Best edition (MBID, match%, mean error)

---

## Analysis Results: Run 23 Edition Winners

**Dataset:** 195 successful matches from 200 albums (95% success rate)

### Edition Rank Distribution

| Edition Rank | Count | Percentage | Notes |
|--------------|-------|------------|-------|
| 1 | 157 | 80.5% | First-ranked edition wins 4/5 times |
| 2 | 22 | 11.3% | Common fallback (wrong region, bonus tracks) |
| 3 | 1 | 0.5% | |
| 4 | 6 | 3.1% | |
| 5 | 2 | 1.0% | |
| 6 | 3 | 1.5% | |
| 7 | 1 | 0.5% | |
| 9 | 1 | 0.5% | |
| 10 | 2 | 1.0% | Deepest rank needed |

**Statistics:**
- Mean edition rank: 1.50
- Median edition rank: 1
- 91.8% of albums matched within top 2 editions
- 96.9% of albums matched within top 10 editions

### Interpretation

**NDR ranking is highly effective:**
- First edition wins 80.5% of the time (validates name-based ranking)
- Only 8.2% of albums required edition 3+ (edge cases)
- Even edition 10 winners achieved 81.8%-100% acoustic match (correct edition found despite low rank)

**Why higher editions win:**
- Track count variations (deluxe vs. standard)
- Regional differences (US vs. UK, Japan imports)
- Remaster vs. original release ordering
- Compilation artist name variations ("Various" vs. actual artist)

**Optimization potential:**
- Reducing `MAX_NAME_DISTANCE_RANK` from 50 to 25 would:
  - Cut API fetches in half
  - Still achieve ~95%+ success rate
  - Only risk 3.1% of albums (those needing editions 26-50)

---

## Key Design Decisions

### 1. Why NDR Ranking Before Detailed Fetch?
**Problem:** MusicBrainz returns 100-150 releases per search. Fetching details for all = 100-150 API calls @ 1/second = 2-3 minutes per album.

**Solution:** Filter to top 50 by name similarity first. Reduces to ~30 seconds per album.

**Trade-off:** If correct edition ranks >50, it won't be tested. Run 23 data suggests this is rare (<5%).

### 2. Why Combination A Filter (Run 25c)?
**Problem:** Run 24d's strict 50%/50% AND filter rejected all editions for 6 albums, causing 5.5% drop in success rate.

**Root Cause:** "Various Artists" compilations failed because artist name similarity was low even when album name matched well.

**Solution:**
- Special case: "Various Artists" → check album similarity only (≥35%)
- Regular albums: weighted combined score (artist 40%, album 60%, ≥42%)

**Expected Impact:** Recover 4-5 failed albums, achieve 94-95% success rate.

### 3. Why Runtime Sorting After NDR Ranking?
**Rationale:** Runtime is a stronger signal for the correct edition once you have track durations.

**Example:** Edition ranked 3 by name might be ranked 1 by runtime (exact duration match). Runtime sorting gives it priority for acoustic testing.

**Conservative Bubble Sort:** Preserves runtime ordering unless name match is 2× better, preventing bad name matches from overriding good runtime matches.

### 4. Why Parallel Edition Testing?
**Efficiency:** Testing editions sequentially would take hours. Parallel testing with 4-second stagger allows:
- Early exit: Stop testing other editions if one achieves 100% match
- Resource utilization: CPU can handle multiple fingerprinting threads
- Progress visibility: User sees incremental results

---

## Run 25c Changes (Current)

**From Run 24d:**
1. Replaced 50%/50% AND filter with Combination A:
   - "Various Artists" albums: album ratio ≥ 35%
   - Regular albums: weighted score ≥ 42% (artist 40%, album 60%)
2. Expected improvement: 92% → 94-95% success rate

**From Run 23 (baseline):**
- Run 23 had no ratio filter (tested all editions up to MAX_NAME_DISTANCE_RANK)
- Run 25c adds filter to reduce API calls while maintaining success rate

---

## Future Optimization Opportunities

### 1. Reduce MAX_NAME_DISTANCE_RANK
- **Current:** 50
- **Proposed:** 25
- **Impact:** 50% fewer API calls, ~95% success rate (vs. 96.9% with rank 50)
- **Risk:** Lose 3.1% of albums that need editions 26-50

### 2. Dynamic Rank Limit
- **Idea:** Use fewer editions for high-confidence matches
- **Algorithm:**
  - If edition 1 NDR score < 0.5: test only top 10 editions
  - If edition 1 NDR score < 1.0: test only top 25 editions
  - Otherwise: test top 50 editions
- **Benefit:** Reduce average API calls without fixed cutoff

### 3. Early Edition Fetch Termination
- **Current:** Fetch all top-50 editions before testing
- **Proposed:** Start acoustic testing as soon as first 3-5 editions fetched
- **Benefit:** Reduce time-to-first-result for albums that match early editions

### 4. Edition Caching
- **Observation:** Popular albums (greatest hits compilations) tested multiple times
- **Proposal:** Cache edition details (durations, MBIDs) keyed by release MBID
- **Benefit:** Eliminate redundant API calls for duplicate albums in collection

---

## Glossary

**NDR (Name Distance Rank):** Numeric rank (1, 2, 3...) based on Levenshtein similarity between edition's artist/album names and source file's parsed names. Lower rank = better name match.

**Levenshtein Ratio:** String similarity metric (0.0-1.0). 1.0 = identical strings, 0.0 = completely different.

**Edition:** Unique combination of track count and durations representing a specific album release (e.g., "US CD 1991 Original", "UK CD 2009 Remaster").

**Runtime Likelihood Score:** How well edition's total duration matches audio file's duration, plus track count penalty. Lower = better match.

**Acoustic Fingerprinting:** Chromaprint-based audio analysis that generates fingerprints for comparison with AcoustID database.

**AcoustID:** Online database of audio fingerprints linked to MusicBrainz Recording MBIDs.

**MBID (MusicBrainz Identifier):** UUID identifying an entity in MusicBrainz (release, recording, artist, etc.).

---

## References

- **Run 23 Log:** `album_matcher_output_run23.txt` - Baseline (95% success, no ratio filter)
- **Run 24d Log:** `album_matcher_output_run24d.txt` - Strict 50%/50% AND filter (92% success)
- **Run 25c Log:** `album_matcher_output_run25c.txt` - Combination A filter (in progress)
- **Edition Analysis:** `analyze_editions.py` - Extracts winning edition ranks from logs
- **Source Code:** `wkmp-ai/examples/album_matcher_25.rs`

# SPEC: Optimal Album Matching Stage Architecture

**Document Type:** Specification
**Status:** Analysis & Recommendation (REVISED)
**Date:** 2025-11-25
**Revision:** 1.1 - Full automatic matching, retain 180-parameter sweep
**Analysis Basis:** album_matcher_27.rs (7324 lines), Run 27 empirical results

---

## Executive Summary

The current album matching implementation (Run 27) achieves high success rates (~93-99% automatic) through a complex 5-stage pipeline. Analysis reveals that while the system works well, the stage ordering is sub-optimal: it starts with maximum complexity (180-parameter grid search) before attempting simpler approaches.

**Key Findings:**
- **Stage 3 dominance:** DP assembly produces >90% of successful matches
- **Stage 2 feeds Stage 3:** 180-parameter sweep generates over-segmented candidates that Stage 3 assembles
- **Simplicity-first opportunity:** Most albums likely matchable with default parameters, avoiding 180-parameter sweep
- **Parameter testing is fast:** Testing many combinations is relatively quick due to pre-computed silence cache
- **Different parameter sets:** Adaptive sweep uses different combinations than initial file segmentation

**Core Principle:** Automatic matching for 98-100% of deterministically matchable albums. Manual review is ONLY acceptable for truly unmatchable cases (corrupt files, fundamentally wrong MusicBrainz data).

**Recommendation:** Redesign with simplicity-first progression while retaining full 180-parameter sweep as Stage 2 fallback. Target: 98-100% automatic success for valid albums, with **1.74x average performance improvement** (115s vs 200s per album).

---

## Part 1: Current Implementation Analysis

### 1.1 Current Architecture (Run 27)

**Pre-Stages (Edition Discovery):**
- Stage 0: MusicBrainz search (100-150 candidates)
- Stage 1: Combination A filter (weighted<42% OR various/album<35%)
- NDR ranking: Sort by name distance ratio
- Fetch track details via API (top 50 by NDR)
- Runtime filtering: ±25% duration match
- Final: 3-15 editions to test acoustically

**Acoustic Matching Stages:**
- **Stage 2:** Parameter grid search (180 combinations: silence thresholds × min durations)
  - Pre-computes silence cache (WindowDbProfile) - FAST due to single-pass dB profiling
  - Tests all 180 combinations against cache (cheap filtering, not re-scanning audio)
  - Generates over-segmented candidates for Stage 3
  - Lines: ~1200 (including caching infrastructure)
  - Complexity: O(1 × audio_scan + 180 × cheap_filters)

- **Stage 3:** Dynamic programming assembly
  - Assembles over-segmented candidates from Stage 2
  - Lines: ~400
  - Success: >90% of matches show "via assembly" in logs

- **Stage 4:** Edition-guided quiet spot detection (RMS profiling)
  - Lines: ~300
  - Success: ~5-10% (wins when Stages 2-3 fail)
  - Penalty: 25% applied to results

- **Stage 5:** Adjacent track merging
  - Lines: ~200
  - Success: <1% (requires detected > expected AND 100% match)

### 1.2 Key Observations

**Observation 1: Stage 2 Complexity is Front-Loaded**
- **Issue:** EVERY album starts with 180-parameter sweep, even those that would match with defaults
- **Evidence:** Run 27 logs show immediate "100.0% via assembly" for many albums
- **Implication:** Simpler approaches should be tried first (simplicity-first principle)

**Observation 2: Parameter Testing is Moderately Expensive**
- **Current implementation:** Pre-computes WindowDbProfile (one audio scan), then tests 180 parameter combinations via cheap filtering
- **Empirical cost:** ~60-90 seconds total for 180-parameter sweep (including WindowDbProfile computation, filtering, and DP assembly)
- **Note:** Earlier estimate of ~5-6s was incorrect; actual Run 27 data shows 60-90s overhead
- **Implication:** 180-parameter sweep adds significant time (~30% of total album processing); avoiding it for simple cases yields performance gains

**Observation 3: Different Parameter Sets for Different Purposes**
- **Initial segmentation:** Uses one set of parameters (unknown from code review)
- **Adaptive sweep:** Uses 180 combinations (STAGE2_THRESHOLD_VALUES × STAGE2_MIN_DURATION_VALUES)
- **Implication:** Specification must distinguish between initial segmentation and adaptive sweep parameters

**Observation 4: Stage 3 is the Workhorse**
- **Evidence:** "via assembly" dominates log output (>90%)
- **Implication:** Stage 3 DP algorithm should be preserved unchanged
- **Architecture:** Stage 2 feeds Stage 3 by generating over-segmented candidates

**Observation 5: Stages 4-5 Rarely Win**
- **Stage 4:** ~5-10% of logs mention "via guided quiet spots"
- **Stage 5:** 0% observed in sampled logs
- **Implication:** Consider whether Stages 4-5 justify ~500 lines complexity, BUT user rejects manual review so they must stay or be replaced with better automatic strategies

### 1.3 Estimated Success Rates (Inferred)

Based on log analysis:

| Stage | Success Rate (First-Time) | Cumulative Coverage | Notes |
|-------|---------------------------|---------------------|-------|
| Stage 2 | ~20-30% | 20-30% | Direct match with one parameter combo |
| Stage 3 | ~60-70% | 90-95% | Dominant stage, handles over-segmentation |
| Stage 4 | ~3-5% | 93-98% | Quiet spot fallback |
| Stage 5 | ~0-1% | 93-99% | Edge case (perfect over-segmentation) |
| Unmatchable | ~1-5% | 100% | Corrupt files, wrong MB data |

**Key Insight:** Stages 2+3 combined achieve ~90-95% success. Stages 4-5 add ~3-6%. Goal: optimize for 98-100% automatic success.

---

## Part 2: Optimal Stage Progression Design

### Design Principles

1. **Simplicity-first:** Try simple approaches before complex ones
2. **100% automatic target:** Manual review ONLY for truly unmatchable (corrupt/wrong data)
3. **Retain all strategies:** Keep 180-parameter sweep and Stages 4-5, but reorder by simplicity
4. **Fast failure:** Explicit rejection criteria prevent false positives
5. **Preserve what works:** Stage 3 (DP assembly) unchanged

### 2.1 Proposed Architecture

**Stage 0: Pre-Flight Validation**

**Purpose:** Reject invalid inputs before expensive processing

**Algorithm:**
- Validate file is decodable (try decode first 10 seconds)
- Validate file duration > 60 seconds (reject singles/fragments)
- Validate MusicBrainz search returns > 0 candidates
- Validate at least one edition passes runtime filter (±25%)
- Single-track detection (score threshold ≥1.5 → likely single track)

**Rejection Criteria:**
- File corrupt/unreadable → REJECT (error)
- File < 60s → REJECT (not an album)
- No MusicBrainz matches → REJECT (unknown album)
- All editions outside ±25% runtime → REJECT (duration mismatch)
- Single-track score ≥1.5 → REJECT (not a multi-track album)

**Transition Logic:**
- If ALL rejection criteria pass → Stage 1
- Else → UNMATCHABLE (with specific reason)

**Complexity:** ~150 lines (includes single-track detection)
**Success:** N/A (gates invalid inputs, ~5-10% rejection)
**Cumulative:** 0% (rejects invalid)

**Rationale:** Current implementation has single-track detection and basic validation scattered throughout. Consolidating into explicit Stage 0 clarifies rejection logic and prevents false positives (matching single tracks as albums).

---

**Stage 1: Default Parameter Silence Detection**

**Purpose:** Match albums with clear silence gaps using PROVEN default parameters (simplest approach)

**Algorithm:**
- Decode entire file to PCM
- Apply SINGLE silence detection pass with proven defaults:
  - Threshold: -54dB (from Run 27 constants: DEFAULT_THRESHOLD_DB)
  - Min duration: 0.6s (from Run 27 constants: DEFAULT_MIN_DURATION_SECS)
- Extract track durations from silence gaps
- Test against top 3 editions (NDR ranks 1-3)
- Compare detected durations vs expected durations (±1.5s tolerance)
- If match ≥95%: ACCEPT and early exit

**Rejection Criteria:**
- Detected tracks < 50% of expected → Stage 2 (under-segmentation, need different params)
- Detected tracks > 150% of expected → Stage 2 (over-segmentation, need assembly)
- Match <95% → Stage 2 (wrong parameters or need assembly)

**Transition Logic:**
- IF match ≥95% on any of top 3 editions → ACCEPT (success)
- ELSE → Stage 2 (try full parameter sweep)

**Complexity:** ~200 lines
**Expected Success:** 70-80% (albums with clear gaps + correct edition in top 3)
**Cumulative:** 70-80%

**Rationale:** Most commercial albums have distinct track gaps. Default parameters (-54dB, 0.6s) are proven optimal from previous runs. Testing top 3 editions captures 92% of wins (per Run 23 data). This simple approach handles the majority case without parameter sweep complexity.

---

**Stage 2: Full Adaptive Parameter Sweep (RETAINED FROM RUN 27)**

**Purpose:** Find optimal silence parameters for albums needing parameter tuning

**Algorithm:**
- Pre-compute WindowDbProfile (single-pass dB profiling across entire file)
  - Scan audio ONCE, store dB level per window
  - Cost: ~500ms, O(audio_length)
- Test ALL 180 parameter combinations via cheap filtering:
  - Thresholds: STAGE2_THRESHOLD_VALUES (9 values: -42 to -66 dB in 3dB steps)
  - Min durations: STAGE2_MIN_DURATION_VALUES (20 values: 0.1 to 2.0s)
  - Total: 180 combinations
  - Cost per combo: ~10-50ms (filter pre-computed profile, not re-scan audio)
  - Total cost: ~5-6 seconds for all 180 combinations
- Generate track durations for each combination
- Test against top 5 editions (expanded from Stage 1)
- Track over-segmented candidates (detected > expected) for Stage 3
- If match ≥95%: ACCEPT and early exit

**Key Implementation Detail (from Run 27):**
- **Different parameters than Stage 1:** Stage 1 uses DEFAULT_THRESHOLD_DB/DEFAULT_MIN_DURATION_SECS from configuration constants
- **Adaptive sweep uses grid:** STAGE2_THRESHOLD_VALUES and STAGE2_MIN_DURATION_VALUES arrays define search space
- **Pre-computed cache:** WindowDbProfile enables O(windows) filtering vs O(samples) re-scanning

**Rejection Criteria:**
- All 180 combinations produce detected = 0 → Stage 3 (silence detection completely failed, try guided quiet spots)
- All combinations produce match <50% → Stage 3 (need assembly)
- No over-segmented candidates AND match <80% → Stage 3 (try different approach)

**Transition Logic:**
- IF match ≥95% → ACCEPT (success)
- ELSE IF over-segmented candidates exist → Stage 3 (assembly needed)
- ELSE IF match <80% → Stage 4 (try guided quiet spots)
- ELSE → ACCEPT with LOW confidence flag

**Complexity:** ~1200 lines (UNCHANGED from Run 27)
**Expected Success:** 15-20% of remaining pool (albums needing parameter tuning)
**Cumulative:** 85-95%

**Rationale:** Keep full 180-parameter sweep from Run 27. Testing is fast (~5-6s) due to pre-computed cache. This provides comprehensive coverage of parameter space without expensive re-scanning. Adaptive sweep uses different parameter combinations than Stage 1 default.

---

**Stage 3: Dynamic Programming Assembly (PRESERVED FROM RUN 27)**

**Purpose:** Assemble over-segmented candidates into correct track count

**Algorithm:**
- Input: Over-segmented candidates from Stage 2 (detected > expected)
- For each candidate:
  - Run DP algorithm: dp[i][j] = minimum error when grouping first i segments into j tracks
  - Generate all valid assemblies (merge adjacent segments)
  - Test each assembly against expected track durations
  - Track best match percentage
- If match ≥95%: ACCEPT and early exit

**Rejection Criteria:**
- No over-segmented candidates from Stage 2 → Skip to Stage 4
- Best assembly <80% → Stage 4 (DP failed to find good assembly)

**Transition Logic:**
- IF match ≥95% → ACCEPT (success)
- ELSE IF match ≥80% → ACCEPT with MEDIUM confidence flag
- ELSE → Stage 4 (try guided quiet spots)

**Complexity:** ~400 lines (UNCHANGED from Run 27)
**Expected Success:** 5-10% of remaining pool
**Cumulative:** 90-98%

**Rationale:** Current Stage 3 is the workhorse (>90% of matches show "via assembly"). Preserve unchanged. Only runs if Stage 2 produces over-segmented candidates, so complexity is conditional.

---

**Stage 4: Edition-Guided Quiet Spot Detection (PRESERVED FROM RUN 27)**

**Purpose:** Handle albums where silence detection fails (continuous audio, crossfades, live recordings)

**Algorithm:**
- Calculate RMS profile across entire audio file
  - Window size: QUIET_SPOT_WINDOW_SECS
  - Step size: QUIET_SPOT_WINDOW_STEP_SECS
- For each expected track boundary (from edition metadata):
  - Calculate dynamic search radius (15% of preceding track duration)
  - Find quietest spot within search window
  - Score candidates: RMS in dB + distance penalty
- Convert detected boundaries to track durations
- Test against expected durations
- Apply STAGE4_PENALTY (25% de-rating)
- If adjusted match ≥80%: ACCEPT

**Rejection Criteria:**
- Match <80% after penalty → Stage 5 (try merging)

**Transition Logic:**
- IF match ≥95% → ACCEPT (success)
- ELSE IF match ≥80% → ACCEPT with LOW confidence flag
- ELSE → Stage 5 (try merging)

**Complexity:** ~300 lines (UNCHANGED from Run 27)
**Expected Success:** 3-5% of remaining pool
**Cumulative:** 93-98%

**Rationale:** Handles albums with no clear silence (live recordings, DJ mixes, crossfaded tracks). Success rate is low (~5-10% of attempts) but catches edge cases that all previous stages miss. 25% penalty reflects lower reliability vs silence-based detection.

---

**Stage 5: Adjacent Track Merging (PRESERVED FROM RUN 27)**

**Purpose:** Correct perfect over-segmentation (detected > expected AND near-perfect match)

**Algorithm:**
- Input: Over-segmented result (detected > expected)
- Precondition: Match ≥95% (near-perfect alignment despite extra tracks)
- Strategy: Merge adjacent detected tracks to reduce count to expected
- Test all merge combinations
- If match = 100%: ACCEPT

**Rejection Criteria:**
- Detected ≤ expected → Skip (not over-segmented)
- Match <95% → Skip (not near-perfect)
- No merge produces 100% → Stage 6 (unmatchable)

**Transition Logic:**
- IF match = 100% → ACCEPT (success)
- ELSE → Stage 6 (unmatchable)

**Complexity:** ~200 lines (UNCHANGED from Run 27)
**Expected Success:** <1% of remaining pool
**Cumulative:** 93-99%

**Rationale:** Handles rare case where silence detection is hyper-sensitive and splits tracks that shouldn't be split, but otherwise produces perfect alignment. Very low success rate but adds minimal complexity.

---

**Stage 6: Unmatchable (REPLACES Manual Review)**

**Purpose:** Terminal stage for albums that exhaust all automatic strategies

**Classification:**
- **Corrupt file:** File decodes but produces nonsense audio (silence, noise, truncated)
- **Wrong MusicBrainz data:** File metadata points to wrong album, or MB data is incorrect
- **Fundamentally unmatchable:** Medleys, DJ mixes, non-standard formats that defeat all algorithms
- **Edge case failure:** Algorithm should have matched but didn't (potential bug)

**Algorithm:**
- Log comprehensive diagnostic report:
  - File path, duration, metadata (artist, album)
  - All stage results (what was tried, why it failed)
  - Best match achieved (edition, percentage, mean error)
  - Detected vs expected track counts per stage
  - Suggested action: "Wrong edition in MB?" / "File corrupt?" / "Non-standard format?"
- Emit UNMATCHABLE status with reason code

**Rejection Criteria:**
- N/A (terminal stage)

**Transition Logic:**
- User reviews diagnostic report
- Options:
  1. Manually specify correct edition (override algorithm)
  2. Flag MB data as incorrect (report to MusicBrainz)
  3. Accept file as unmatchable (skip)
  4. Report as bug if file should have matched

**Complexity:** ~200 lines (diagnostic reporting)
**Expected Success:** N/A (human intervention or acceptance)
**Cumulative:** 100%

**Rationale:** Remaining 1-5% of albums are NOT deterministically matchable by current algorithms. These are:
- Corrupt files (0.5-1%)
- Wrong MB metadata (0.5-2%)
- Fundamentally non-standard (medleys, DJ mixes) (0.5-2%)
- Algorithm bugs (unknown, ideally 0%)

Manual review is acceptable ONLY for these truly unmatchable cases, NOT for albums that should work but don't due to insufficient algorithmic coverage.

---

### 2.2 Edition Selection and Ranking

**Purpose:** After acoustic matching stages produce track candidates, select the CORRECT edition from multiple MusicBrainz matches. This is critical for MBID assignment accuracy.

**Problem Statement (Full Library Test Results):**
- **Baseline success:** 85.3% (145/170 albums matched correctly)
- **Target success:** 98%+ (reduce 14.7% failure rate to <2%)
- **Key failure mode:** Wrong edition selection
  - Example: Aqualung.mp3 matched 147-track box set (99.3% confidence) instead of 11-track standard edition
  - Root cause: Edition selection weighted only name similarity (40%) + match percentage (60%), missing total duration validation

**Success Metric:** Percentage of files where ALL passages are assigned correct MBIDs (ground truth validation). NOT intermediate metrics like number of editions found or match percentage (high match percentage with WRONG edition = failure).

---

#### 2.2.1 Current Edition Selection (Run 27 Baseline)

**Current Weighted Scoring:**
```
score = (name_similarity × 0.4) + (match_percentage × 0.6)

Where:
- name_similarity: Jaro-Winkler distance between detected album name and edition title (0.0-1.0)
- match_percentage: (tracks_within_tolerance / total_tracks) × 100
```

**Limitations:**
1. **No total duration validation:** 40-minute file can match 4-hour box set if first N tracks align
2. **Binary track matching:** 0.5s error = 9.5s error (both "within tolerance"), no quality distinction
3. **No track count flexibility:** Rejects editions with ±1 track difference even if all other factors excellent
4. **Name overweighted:** 40% weight on name similarity can favor wrong edition with similar name

**Impact:** Wrong edition selection → All MBIDs incorrect → 0% passage accuracy despite high "match percentage"

---

#### 2.2.2 Multi-Factor Weighted Scoring (Required)

**Requirement [REQ-AM-092]:** Edition selection MUST use multi-factor weighted scoring combining total duration alignment, track match quality, name similarity, and track count tolerance.

**Proposed Weighted Scoring:**
```
edition_score = (total_duration_score × 0.30)
              + (track_quality_score × 0.45)
              + (name_similarity_score × 0.25)
              + (track_count_penalty)

Rank all candidate editions by edition_score (descending)
Select top-ranked edition as final match
```

**Rationale:**
- **Total duration (30%):** Prevents box set mismatches (Aqualung 147-track issue)
- **Track quality (45%):** Primary matching signal - how well do individual tracks align?
- **Name similarity (25%):** Reduced from 40% - useful tiebreaker but not primary signal
- **Track count penalty:** Multiplicative penalty for ±1, ±2, ±3+ track differences

**Edge Cases:**
- If `candidate_editions` is empty: Return `None` (no selection possible)
- If all edition scores ≤ 0.0: Return `None` (no acceptable matches)
- If multiple editions have identical scores: Prefer edition with higher `name_similarity`, then lexicographically lower MBID (deterministic tie-breaking)

---

#### 2.2.3 Total Duration Alignment Scoring

**Requirement [REQ-AM-093]:** Edition selection MUST validate total duration to prevent matching box sets, deluxe editions, or compilations when standard edition is correct.

**Algorithm:**
```rust
fn calculate_total_duration_score(
    detected_total_ms: u64,
    edition_total_ms: u64,
) -> f64 {
    let diff_ms = detected_total_ms.abs_diff(edition_total_ms);
    let diff_pct = (diff_ms as f64 / detected_total_ms as f64) * 100.0;

    // Graduated penalty based on percentage difference
    if diff_pct < 5.0 {
        0.95  // Excellent alignment (< 5% difference)
    } else if diff_pct < 10.0 {
        0.80  // Good alignment (5-10% difference)
    } else if diff_pct < 15.0 {
        0.60  // Acceptable (10-15% difference)
    } else if diff_pct < 25.0 {
        0.30  // Poor alignment (15-25% difference)
    } else {
        0.05  // Very poor alignment (> 25% difference) - likely wrong edition
    }
}
```

**Example (Aqualung):**
- Detected total: ~40 minutes (11 tracks)
- Standard edition: ~42 minutes (11 tracks) → diff_pct = 4.8% → score = 0.95
- Box set edition: ~240 minutes (147 tracks) → diff_pct = 500% → score = 0.05

**Impact:** With 30% weighting, total duration score drops box set from 99.3% to ~60%, while standard edition maintains ~95%+

**Edge Cases:**
- If `detected_total_ms = 0` or `edition_total_ms = 0`: Return score = 0.05 (very poor alignment, treat as likely wrong edition)

---

#### 2.2.4 Track Match Quality Scoring (Graduated)

**Requirement [REQ-AM-094]:** Track matching MUST use graduated quality scoring based on error magnitude, NOT binary "within tolerance" checks.

**Current Problem:**
```rust
// Current: Binary matching
if track_error <= tolerance {
    track_match = 1.0;  // Same score for 0.5s and 9.5s error
} else {
    track_match = 0.0;
}
match_percentage = matched_tracks / total_tracks;
```

**Proposed Algorithm:**
```rust
fn calculate_track_quality_score(
    detected_durations: &[f64],
    edition_durations: &[f64],
    tolerance_secs: f64,
) -> f64 {
    let mut total_quality = 0.0;
    let track_count = detected_durations.len().min(edition_durations.len());

    for i in 0..track_count {
        let error = (detected_durations[i] - edition_durations[i]).abs();

        // Graduated quality: better alignment = higher quality
        let track_quality = if error <= tolerance_secs {
            1.0 - (error / tolerance_secs)  // Linear decay from 1.0 to 0.0
        } else {
            0.0  // Outside tolerance = 0 quality
        };

        total_quality += track_quality;
    }

    total_quality / track_count as f64  // Average quality across all tracks
}
```

**Example:**
- Track with 0.5s error (tolerance 10s): quality = 1.0 - (0.5/10) = 0.95
- Track with 5.0s error (tolerance 10s): quality = 1.0 - (5.0/10) = 0.50
- Track with 9.5s error (tolerance 10s): quality = 1.0 - (9.5/10) = 0.05
- Track with 12s error (tolerance 10s): quality = 0.0 (outside tolerance)

**Impact:** Distinguishes excellent matches (0.95 average quality) from marginal matches (0.50 average quality), enabling better edition ranking when multiple editions have same track count.

**Track Count Mismatch Handling:**
- Quality calculated only on first `min(detected_count, edition_count)` tracks
- Extra tracks beyond minimum length are ignored for quality calculation
- Track count difference penalty applied separately via REQ-AM-095 (graduated track count tolerance)
- This provides clean separation: quality measures alignment of matched portion, penalty addresses count difference

**Edge Cases:**
- If both arrays are empty (zero tracks): Return score = 0.0 (no tracks to match)
- If `tolerance_secs ≤ 0`: Assert violation or return score = 0.0 (invalid tolerance)

---

#### 2.2.5 Graduated Track Count Tolerance

**Requirement [REQ-AM-095]:** Edition selection MUST accept editions with ±1-2 track differences using graduated penalties, NOT binary threshold rejection.

**User Specification:**
- "Do not implement a simple case of +/- N tracks"
- "Grade the match based on difference"
- Exact match = best but not required
- ±1 track = acceptable when other parameters match well
- ±2 tracks = acceptable only with significantly better other parameters
- "Progressively scored parameter in final edition selection criteria"

**Algorithm:**
```rust
fn calculate_track_count_penalty(
    detected_count: usize,
    edition_count: usize,
) -> f64 {
    let diff = detected_count.abs_diff(edition_count);

    match diff {
        0 => 1.00,  // Exact match - no penalty
        1 => 0.95,  // ±1 track - minimal penalty (acceptable with good other params)
        2 => 0.85,  // ±2 tracks - moderate penalty (needs excellent other params)
        3 => 0.70,  // ±3 tracks - significant penalty
        4..=5 => 0.50,  // ±4-5 tracks - large penalty
        _ => 0.20,  // ±6+ tracks - severe penalty (likely wrong edition)
    }
}
```

**Integration with Multi-Factor Scoring:**
```rust
// Multiplicative penalty (NOT additive weight)
edition_score = base_score × track_count_penalty;

// Where base_score = (duration × 0.30) + (quality × 0.45) + (name × 0.25)
```

**Example:**
- Edition A: Exact track count (11 = 11), base_score = 0.90 → final = 0.90 × 1.00 = 0.90
- Edition B: ±1 track (11 vs 12), base_score = 0.95 → final = 0.95 × 0.95 = 0.90 (competitive!)
- Edition C: ±2 tracks (11 vs 13), base_score = 0.98 → final = 0.98 × 0.85 = 0.83 (still viable if excellent match)
- Edition D: ±6 tracks (11 vs 17), base_score = 0.85 → final = 0.85 × 0.20 = 0.17 (rejected)

**Rationale:** Allows flexibility for editions with bonus tracks, hidden tracks, or different segmentation (Japanese edition with extra track, vinyl vs CD splits), while still penalizing large differences that indicate wrong edition type (singles collection vs standard album).

---

#### 2.2.6 Multi-Strategy MusicBrainz Search

**Requirement [REQ-AM-096]:** MusicBrainz edition search MUST use multi-strategy progressive search to maximize coverage of valid editions.

**Current Implementation:** 7-strategy progressive search already implemented in `wkmp-ai/src/services/musicbrainz_client.rs`:
1. Basic unquoted search with type:album filter (avoids overly restrictive quoted exact-match)
2. CamelCase split (handles "HappyNation" → "Happy Nation")
3. Fuzzy ~1 edit (handles minor typos)
4. Wildcard fixes for common misspellings
5. Aggressive fuzzy ~2 edits
6. Per-token fuzzy (multi-word names)
7. Album-only fallback (when artist unknown)

**Success Metric:** NOT "more editions found" but "correct edition found AND ranked correctly"

**Performance Baseline:**
- Full library test (170 albums): 85.3% success (145 correct, 25 failures)
- Key failures: Wrong edition selected (Aqualung box set, Goodbye Yellow Brick Road deluxe)

**Target Performance:**
- 98%+ success rate (reduce failures from 25 to <4)
- Correct edition selection via multi-factor scoring
- Measured by: Ground truth MBID validation (passage-level accuracy)

**Deduplication:**
- Editions returned from multiple strategies are deduplicated by release MBID
- If all strategies return 0 results: Stage 0 validation (REQ-AM-003) should have caught this (no MusicBrainz candidates found)

---

#### 2.2.7 Edition Ranking Algorithm (Complete)

**Complete edition selection flow:**

```rust
fn select_best_edition(
    detected_total_duration_ms: u64,
    detected_track_durations: &[f64],
    detected_track_count: usize,
    candidate_editions: Vec<Edition>,
    tolerance_secs: f64,
) -> Option<Edition> {
    let mut scored_editions = Vec::new();

    for edition in candidate_editions {
        // Component scores
        let duration_score = calculate_total_duration_score(
            detected_total_duration_ms,
            edition.total_duration_ms(),
        );

        let quality_score = calculate_track_quality_score(
            detected_track_durations,
            &edition.track_durations,
            tolerance_secs,
        );

        let name_score = calculate_name_similarity(
            detected_album_name,
            &edition.title,
        );

        let track_count_penalty = calculate_track_count_penalty(
            detected_track_count,
            edition.track_count,
        );

        // Multi-factor weighted score
        let base_score = (duration_score * 0.30)
                       + (quality_score * 0.45)
                       + (name_score * 0.25);

        let final_score = base_score * track_count_penalty;

        scored_editions.push((edition, final_score));
    }

    // Rank by score (descending)
    scored_editions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Select top-ranked edition
    scored_editions.first().map(|(edition, _score)| edition.clone())
}
```

---

#### 2.2.8 Validation and Success Criteria

**Validation Approach:**
1. **Ground Truth Dataset:** Create test set with known-correct MBIDs
   - Include: Standard albums, box sets, deluxe editions, Japanese editions with bonus tracks
   - Mix: 80% standard, 10% deluxe/special, 10% edge cases
2. **Passage-Level Accuracy:** Measure percentage of passages with correct MBID assigned
   - 100% passage accuracy = success (all tracks matched correctly)
   - 0% passage accuracy = failure (wrong edition selected → all MBIDs wrong)
3. **Edition Type Distribution:** Verify correct edition types selected
   - Standard edition preferred when available
   - Deluxe/box set selected ONLY when file actually is deluxe/box set

**Success Criteria:**
- **Primary:** ≥98% album-level success (correct edition selected)
- **Secondary:** ≥99.5% passage-level accuracy (correct MBIDs assigned)
- **Regression Prevention:** No increase in unmatchable rate (stay ≤2%)
- **Performance:** Edition selection adds ≤5s per album (scoring is cheap)

**Failure Case Analysis (Aqualung):**
- **Before:** Matched 147-track box set (99.3% match percentage)
- **After:** Should match 11-track standard edition
- **Validation:**
  - Total duration score: Standard (0.95) vs Box set (0.05)
  - Final score: Standard (~0.90) vs Box set (~0.35)
  - Selection: Standard edition (correct)

**Risk Mitigation:**
- Monitor edition selection distribution (80/10/10 expected vs actual)
- Flag cases where deluxe edition ranked higher than standard (investigate)
- Track "close calls" where top 2 editions scored within 0.05 (manual review sample)

---

## Part 3: Rejection Criteria Framework

### 3.1 Rejection Types

**Input Rejection (Stage 0):** "This file is invalid or not an album"
- File corrupt/undecodable
- File < 60s duration (single track)
- No MusicBrainz candidates found
- All editions outside ±25% runtime
- Single-track detection score ≥1.5

**Match Rejection (Stages 1-5):** "This result is too poor quality to accept, try next stage"
- Stage 1: Match <95% OR detected/expected ratio <50% or >150%
- Stage 2: All 180 combos produce match <50% OR no over-segmented candidates
- Stage 3: Best assembly <80%
- Stage 4: Match <80% after 25% penalty
- Stage 5: Not over-segmented OR match <95% OR no merge = 100%

**Terminal Rejection (Stage 6):** "Exhausted all automatic strategies"
- Stages 1-5 all failed to produce match ≥80%
- Emit UNMATCHABLE with diagnostic report

### 3.2 Threshold Justification

**95% threshold (Stages 1-2):**
- Commercial albums: 10-15 tracks typical
- 95% = 1 track error acceptable
- Example: 12 tracks, 11/12 correct = 91.7% (borderline, try next stage)
- Example: 12 tracks, 12/12 correct = 100% (accept)

**80% threshold (Stages 3-4):**
- Later stages are less reliable (DP assembly, quiet spots)
- 80% = ~2 track errors on 10-track album
- Below 80% suggests fundamental mismatch (wrong edition, corrupted file)
- Accept with confidence flag allows tracking for quality review

**100% threshold (Stage 5):**
- Merging only makes sense if near-perfect alignment exists
- Lower threshold would merge incorrectly segmented albums

### 3.3 Confidence Flags

**HIGH confidence:** Stages 1-2 match ≥95%
- Clear algorithmic success
- Very low false positive risk

**MEDIUM confidence:** Stage 3 match 80-94%
- DP assembly succeeded but not perfect
- Moderate false positive risk (assemblies can be ambiguous)

**LOW confidence:** Stages 4-5 match 80-94%
- Quiet spots or merging succeeded
- Higher false positive risk (RMS profiling is heuristic)

**UNMATCHABLE:** All stages <80%
- Not automatically matchable with current algorithms
- Requires human review or is truly unprocessable

---

## Part 4: Comparison Analysis

### 4.1 Stage Ordering Comparison

| Priority | Current (Run 27) | Proposed | Change |
|----------|------------------|----------|--------|
| 1st attempt | Stage 2 (180 params) | Stage 1 (1 default) | **Simplicity-first** |
| 2nd attempt | Stage 3 (DP assembly) | Stage 2 (180 params) | **Deferred complexity** |
| 3rd attempt | Stage 4 (quiet spots) | Stage 3 (DP assembly) | **Preserved** |
| 4th attempt | Stage 5 (merging) | Stage 4 (quiet spots) | **Preserved** |
| 5th attempt | N/A | Stage 5 (merging) | **Preserved** |
| Terminal | Implicit failure | Stage 6 (unmatchable) | **Explicit status** |

**Key Change:** Simplicity-first (default → 180 params → DP → quiet spots → merge) vs. complexity-first (180 params → DP → quiet spots → merge)

### 4.2 Code Complexity Comparison

| Component | Current | Proposed | Change |
|-----------|---------|----------|--------|
| Stage 0 (pre-flight) | Implicit | ~150 | +150 (explicit validation) |
| Stage 1 (default params) | N/A | ~200 | +200 (new stage) |
| Stage 2 (180 params) | ~1200 | ~1200 | 0 (preserved) |
| Stage 3 (DP assembly) | ~400 | ~400 | 0 (preserved) |
| Stage 4 (quiet spots) | ~300 | ~300 | 0 (preserved) |
| Stage 5 (merging) | ~200 | ~200 | 0 (preserved) |
| Stage 6 (unmatchable) | N/A | ~200 | +200 (diagnostic) |
| **Total** | **~2100** | **~2650** | **+550 (+26%)** |

**Note:** Total INCREASES by 550 lines (+26%) due to adding Stage 0 (pre-flight) and Stage 1 (default params). However:
- Stage 0 consolidates scattered validation logic (net neutral)
- Stage 1 is simple and handles 70-80% of albums (high value per line)
- Total remains well under original Run 27 complexity (~7324 lines total including infrastructure)

### 4.3 Expected Success Rate Comparison

| Outcome | Current | Proposed | Change |
|---------|---------|----------|--------|
| Stage 1 success | N/A (part of Stage 2) | 70-80% | Early exit for majority |
| Stage 2 success | ~20-30% | ~15-20% | Reduced (filtered by Stage 1) |
| Stage 3 success | ~60-70% | ~5-10% | Reduced (filtered by Stages 1-2) |
| Stage 4 success | ~3-5% | ~3-5% | Unchanged |
| Stage 5 success | ~0-1% | ~0-1% | Unchanged |
| **Total automatic** | **93-99%** | **98-100%** | **+0 to +5%** (target improvement) |
| Unmatchable | ~1-5% | ~0-2% | Reduced (better coverage) |

**Trade-off:** Slight code increase (+26%, 550 lines) for:
- Simplicity-first ordering (faster for majority case)
- Explicit validation stage (clearer rejection logic)
- Potentially higher success rate (98-100% target vs 93-99% current)

### 4.4 Performance Comparison

**ACTUAL Run 27 Performance (Empirical Data from 179 albums):**
- **Average:** 200.5 seconds (3.3 minutes) per album
- **Median:** 120.0 seconds (2.0 minutes)
- **P90:** 360.0 seconds (6.0 minutes)
- **Distribution:**
  - 54.7% take 2-3 minutes (120-180s)
  - 31.3% take 3-5 minutes (180-300s)
  - 12.3% take 5-10 minutes (300-600s)
  - 1.7% take > 10 minutes (600s+)

**Current (Run 27) - Complexity-First Breakdown:**
- ALL albums start with full processing:
  - Audio decode: ~30-60s (depends on file size)
  - MusicBrainz API calls + filtering: ~20-40s (cached or live)
  - 180-parameter sweep (WindowDbProfile + filtering): ~60-90s
  - DP assembly (if needed): ~20-40s
  - Edition testing overhead: ~10-30s
- **Total:** 140-260s range, **200s average**

**Proposed - Simplicity-First Breakdown:**

**Stage 1 (70-80% of albums):** Default parameters only
- Audio decode: ~30-60s (same as current)
- MusicBrainz API calls + filtering: ~20-40s (same as current)
- Single silence detection (-54dB, 0.6s): ~5-10s (vs 60-90s for 180-param)
- Test vs top 3 editions: ~5-10s
- **Total:** ~60-120s (vs 200s current)
- **Speedup:** 1.7-3.3x faster

**Stage 2 (15-20% of albums):** Proceed to 180-parameter sweep
- Stage 1 attempt (failed): ~60-120s
- 180-parameter sweep: ~60-90s (same as current Stage 2)
- Test vs top 5 editions: ~10-20s
- **Total:** ~130-230s (similar to current 200s)
- **No regression**

**Stage 3+ (5-10% of albums):** DP assembly and beyond
- Stages 1+2: ~130-230s
- DP assembly + Stages 4-5: ~20-50s
- **Total:** ~150-280s (similar to current 200s or slightly slower)
- **Acceptable:** Edge cases, small percentage

**Weighted Average Performance:**
```
Proposed average = (75% × 90s) + (17.5% × 180s) + (7.5% × 215s)
                 = 67.5s + 31.5s + 16.1s
                 = 115s (1.9 minutes)

Current average  = 200s (3.3 minutes)

Performance improvement: 1.74x faster (115s vs 200s)
```

**Performance Summary:**
- **Majority case (75%):** 1.7-3.3x faster (60-120s vs 200s)
- **Medium complexity (17.5%):** Similar speed (~180s)
- **High complexity (7.5%):** Slight slowdown acceptable (215s vs 200s)
- **Overall average:** 1.74x faster (115s vs 200s)

**Key Insight:** The current system wastes ~60-90s on 180-parameter sweep for 70-80% of albums that would match with default parameters. Simplicity-first eliminates this waste for the majority case.

---

## Part 5: Implementation Guidance

### 5.1 Stage Transition Logic (Pseudo-code)

```rust
fn match_album(file: &Path, editions: Vec<Edition>) -> MatchResult {
    // Stage 0: Pre-flight validation
    let validation = validate_input(file, &editions)?;
    if !validation.is_valid {
        return MatchResult::Unmatchable {
            reason: validation.reason,
            stage: 0,
        };
    }

    // Stage 1: Default parameter silence detection (simplest)
    let stage1 = silence_detection_default_params(file, &editions[0..3]);
    if stage1.match_pct >= 95.0 {
        return MatchResult::Accepted {
            result: stage1,
            confidence: Confidence::High,
            stage: 1,
        };
    }
    if !stage1.should_try_next_stage() {
        // Detected/expected ratio too far off, skip to Stage 3
        return try_stage_3_or_later(file, editions);
    }

    // Stage 2: Full 180-parameter adaptive sweep
    let stage2 = silence_detection_adaptive_sweep(file, &editions[0..5]);
    if stage2.match_pct >= 95.0 {
        return MatchResult::Accepted {
            result: stage2,
            confidence: Confidence::High,
            stage: 2,
        };
    }
    if stage2.over_segmented_candidates.is_empty() && stage2.match_pct < 80.0 {
        // No candidates for Stage 3, skip to Stage 4
        return try_stage_4_or_later(file, editions);
    }

    // Stage 3: DP assembly (conditional - only if over-segmented candidates exist)
    if !stage2.over_segmented_candidates.is_empty() {
        let stage3 = dp_assemble_segments(&stage2.over_segmented_candidates, &editions[0..5]);
        if stage3.match_pct >= 95.0 {
            return MatchResult::Accepted {
                result: stage3,
                confidence: Confidence::High,
                stage: 3,
            };
        }
        if stage3.match_pct >= 80.0 {
            return MatchResult::Accepted {
                result: stage3,
                confidence: Confidence::Medium,
                stage: 3,
            };
        }
    }

    // Stage 4: Edition-guided quiet spot detection
    let stage4 = guided_quiet_spots(file, &editions[0..5]);
    if stage4.match_pct >= 95.0 {
        return MatchResult::Accepted {
            result: stage4,
            confidence: Confidence::High,
            stage: 4,
        };
    }
    if stage4.match_pct >= 80.0 {
        return MatchResult::Accepted {
            result: stage4,
            confidence: Confidence::Low,
            stage: 4,
        };
    }

    // Stage 5: Adjacent track merging (conditional - only if over-segmented)
    let best_so_far = get_best_result(&[stage1, stage2, stage3, stage4]);
    if best_so_far.detected > best_so_far.expected && best_so_far.match_pct >= 95.0 {
        let stage5 = merge_adjacent_tracks(&best_so_far, &editions[0..5]);
        if stage5.match_pct == 100.0 {
            return MatchResult::Accepted {
                result: stage5,
                confidence: Confidence::High,
                stage: 5,
            };
        }
    }

    // Stage 6: Unmatchable (all strategies exhausted)
    let diagnostic = generate_diagnostic_report(
        file,
        &editions,
        &[stage1, stage2, stage3, stage4],
    );

    return MatchResult::Unmatchable {
        reason: classify_unmatchable_reason(&diagnostic),
        best_attempt: best_so_far,
        diagnostic,
        stage: 6,
    };
}
```

### 5.2 Metrics to Track Stage Effectiveness

**Per-Stage Metrics:**
- **Attempts:** How many albums reach this stage?
- **Success:** How many albums match at threshold (95% for Stages 1-2, 80% for Stages 3-5)?
- **Rejection:** How many trigger rejection criteria?
- **Time:** Average execution time per album
- **Early exit rate:** % of albums that exit at this stage vs proceed to next

**Overall Metrics:**
- **Automatic success rate:** % of albums matched automatically (Stages 1-5)
- **Unmatchable rate:** % of albums reaching Stage 6
- **Average time per album:** Weighted by stage success rates
- **False positive rate:** % of accepted matches later rejected by user/verification
- **False negative rate:** % of unmatchable albums that should have matched

**Example Dashboard:**
```
Stage 0 (Pre-flight):  8.2% rejected (corrupt/invalid), 91.8% → Stage 1 (avg 0.5s)
Stage 1 (Default):     76.3% success, 15.2% → Stage 2 (avg 2.8s)
Stage 2 (Adaptive):    12.1% success, 7.6% → Stage 3 (avg 5.4s)
Stage 3 (DP):          6.2% success, 1.4% → Stage 4 (avg 2.1s)
Stage 4 (Quiet):       1.1% success, 0.3% → Stage 5 (avg 3.2s)
Stage 5 (Merge):       0.2% success, 0.1% → Stage 6 (avg 1.5s)
Stage 6 (Unmatch):     0.1% terminal

Overall: 96.0% automatic, 0.1% unmatchable, 8.2% invalid
Average time: 3.2s per valid album (weighted)
```

### 5.3 False Positive Detection

**Problem:** Algorithm may produce high match % for wrong edition (especially compilations with similar track counts)

**Detection Methods (from Run 27):**
1. **Artist verification:** Check matched edition artist vs file metadata (Jaro-Winkler + Levenshtein)
   - Threshold: ARTIST_MISMATCH_THRESHOLD (0.60 = 60% similarity)
   - If below threshold AND match% < ARTIST_MISMATCH_MIN_MATCH_PCT: Flag as suspect
2. **Album verification:** Check matched edition album vs file metadata
   - Same similarity metrics as artist
3. **Duration variance:** High match % but large mean timing error
   - Threshold: Mean error >5s per track → Flag as suspect
4. **Track count mismatch:** Detected significantly different from expected
   - Threshold: |detected - expected| > 3 → Flag as suspect

**Confidence Adjustment:**
- If any suspect criteria triggered: Downgrade confidence (High → Medium → Low)
- If multiple criteria triggered: Emit warning, suggest manual verification

### 5.4 Validation Approach

**Unit Testing:**
- Stage 0: Invalid files (corrupt, too short, no MB matches) → Expect rejection
- Stage 1: Known-good albums with clear gaps → Expect 100% match
- Stage 2: Albums requiring parameter tuning → Expect Stage 2 success
- Stage 3: Over-segmented albums (sensitive silence detection) → Expect Stage 3 success
- Stage 4: Live recordings, crossfaded tracks → Expect Stage 4 success
- Stage 5: Hyper-sensitive segmentation with perfect alignment → Expect Stage 5 success
- Stage 6: Corrupt files, wrong MB data → Expect unmatchable

**Integration Testing:**
- Full dataset run (e.g., Run 27's 200 albums)
- Compare automatic success rate: target ≥98%
- Compare unmatchable rate: target ≤2%
- Compare execution time: target <5s avg per album (50% faster than current)

**Regression Testing:**
- Track albums that regress from automatic → unmatchable after changes
- Track false positives (accepted matches later rejected by user)
- Track false negatives (unmatchable albums that manual review shows should have matched)

**A/B Testing:**
- Run current (Run 27) vs proposed side-by-side on same dataset
- Compare success rates, execution time, false positive/negative rates
- Decision criteria: Proposed must achieve ≥98% automatic success with ≤5s avg time

---

## Part 6: Risk Assessment

### 6.1 Risk: Adding Stage 1 May Not Achieve 70-80% Target

**Description:** Stage 1 (default parameters) may only match 50-60% of albums, not 70-80%

**Impact:** Less performance improvement than expected (early exit rate lower)

**Mitigation:**
- Empirical validation: Run Stage 1 on existing album dataset
- If <70%, adjust thresholds or add 2-3 default parameter sets
- Fallback: Stage 2 always available if Stage 1 under-performs

**Residual Risk:** LOW (Stage 2 provides comprehensive coverage regardless)

### 6.2 Risk: Increased Code Complexity

**Current:** ~2100 lines (core matching logic)
**Proposed:** ~2650 lines (+550, +26%)
**Impact:** More code to maintain, test, debug

**Mitigation:**
- Stage 0 consolidates existing scattered validation (net neutral complexity)
- Stage 1 is simple (~200 lines, single algorithm)
- Stages 2-5 preserved unchanged (no new bugs)
- Net increase is controlled (+26%, not +100%)

**Residual Risk:** LOW (complexity increase is minimal and well-structured)

### 6.3 Risk: Unmatchable Rate Higher Than Expected

**Target:** 0-2% unmatchable
**Current:** 1-5% (estimated from log analysis)
**Risk:** Unmatchable rate remains at 3-5%, not 0-2%

**Impact:** More albums require human intervention than desired

**Mitigation:**
- Preserve all Stages 2-5 from Run 27 (maintains current coverage)
- Add Stage 1 for early wins (net positive)
- Explicit diagnostic reports make human review faster
- Track unmatchable reasons to identify algorithm gaps

**Residual Risk:** MEDIUM (dependent on actual album dataset characteristics)

### 6.4 Risk: Performance Regression for Complex Albums

**Current:** 180-param sweep runs for ALL albums (~5-6s overhead)
**Proposed:** 180-param sweep only for Stage 2 (~15-20% of albums)
**Risk:** Stage 1 albums get faster, but Stage 2+ albums get SLOWER (more total time: Stage 1 + Stage 2)

**Impact:** Complex albums may take longer (2-3s Stage 1 + 5-6s Stage 2 = 7-9s vs 5-6s current)

**Mitigation:**
- Stage 1 should fail-fast (reject criteria prevent wasted time)
- If detected/expected ratio is 0% or >200%, skip directly to Stage 4
- Overall average should still improve due to 70-80% early exit

**Residual Risk:** LOW (fail-fast criteria prevent significant regression)

---

## Part 7: Recommendations

### 7.1 Implementation Priority

**Phase 1: Add Stage 1 (Default Parameters) - 2-3 weeks**
1. Implement Stage 1 alongside current Run 27 (parallel execution)
2. Log Stage 1 results: success rate, match%, time
3. Validate 70-80% target on existing album dataset
4. If <70%, adjust parameters or add 2-3 default sets
5. If ≥70%, proceed to Phase 2

**Phase 2: Add Stage 0 (Pre-Flight Validation) - 1-2 weeks**
1. Extract existing validation logic into explicit Stage 0
2. Add single-track detection gate
3. Test on invalid files (corrupt, too short, singles)
4. Validate 5-10% rejection rate (filters out invalids)

**Phase 3: Reorder Stages - 2-3 weeks**
1. Restructure control flow: Stage 0 → Stage 1 → Stage 2 (current) → Stage 3-5 (current)
2. Ensure Stage 2-5 logic unchanged (preserve algorithms)
3. Test stage transition logic (rejection criteria, early exits)
4. Validate overall success rate ≥98%

**Phase 4: Add Stage 6 (Unmatchable Diagnostics) - 1-2 weeks**
1. Implement diagnostic report generation
2. Classify unmatchable reasons (corrupt, wrong MB, non-standard)
3. Test on albums that failed Stages 1-5
4. Validate diagnostic quality (actionable information)

**Phase 5: Validation and Deployment - 2-3 weeks**
1. Full dataset regression testing (Run 27 vs proposed)
2. Measure automatic success rate (target ≥98%)
3. Measure unmatchable rate (target ≤2%)
4. Measure average time (target ≤5s)
5. Decision: Deploy if all targets met

**Total Timeline:** 8-13 weeks (2-3 months)

### 7.2 Success Criteria

**Must Have:**
- Automatic success rate ≥98% (deterministically matchable albums)
- Unmatchable rate ≤2% (truly unprocessable files)
- Stage 1 success rate ≥70% (validates simplicity-first)
- Average time ≤150s per album (1.5x faster than current 200s)

**Should Have:**
- Automatic success rate ≥99% (stretch goal)
- Unmatchable rate ≤1% (ideal)
- Stage 1 success rate ≥80% (strong simplicity-first)
- Average time ≤120s per album (1.67x faster than current)

**Could Have:**
- False positive rate ≤1% (very high quality)
- False negative rate = 0% (no missed deterministic matches)

### 7.3 Rollback Plan

**If automatic success rate <98% after Phase 5:**
- Investigate root cause: Which stage is under-performing?
- If Stage 1: Adjust parameters or add more default sets
- If Stage 2-5: Bug in preservation/migration, revert to Run 27
- If Stage 0: Rejection criteria too aggressive, loosen thresholds

**If unmatchable rate >2%:**
- Analyze unmatchable reasons (corrupt? wrong MB? algorithm gap?)
- If algorithm gap: Add new stage or enhance existing stage
- If corrupt/wrong MB: Acceptable, document limitation

**If performance regression (avg time >200s):**
- Investigate Stage 1 bottlenecks (should be <120s)
- Optimize silence detection (current implementation may have overhead)
- Add fail-fast shortcuts (skip stages if impossible to match)
- Ensure WindowDbProfile computation is deferred to Stage 2 only

---

## Part 8: Appendix

### 8.1 Parameter Configuration Constants (from Run 27)

**Default Parameters (Stage 1):**
```rust
const DEFAULT_THRESHOLD_DB: f64 = -54.0;  // Proven optimal from analysis
const DEFAULT_MIN_DURATION_SECS: f64 = 0.6;  // Proven optimal
const MATCH_TOLERANCE_SECS: f64 = 1.5;  // ±1.5s per track
```

**Adaptive Sweep Parameters (Stage 2):**
```rust
// Stage 2 Parameter Grid: Threshold values (dB)
const STAGE2_THRESHOLD_VALUES: [f64; 9] = [
    -42.0, -45.0, -48.0, -51.0, -54.0, -57.0, -60.0, -63.0, -66.0
];

// Stage 2 Parameter Grid: Min duration values (seconds)
const STAGE2_MIN_DURATION_VALUES: [f64; 20] = [
    0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
    1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0
];

// Total combinations: 9 × 20 = 180
```

**Stage 4 Parameters:**
```rust
const QUIET_SPOT_WINDOW_SECS: f64 = 0.1;  // RMS window size
const QUIET_SPOT_WINDOW_STEP_SECS: f64 = 0.05;  // RMS step size
const QUIET_SPOT_SEARCH_RADIUS_RATIO: f64 = 0.15;  // 15% of track duration
const QUIET_SPOT_SEARCH_RADIUS_MIN: f64 = 2.0;  // Minimum 2s search radius
const QUIET_SPOT_SEARCH_RADIUS_MAX: f64 = 10.0;  // Maximum 10s search radius
const STAGE4_PENALTY: f64 = 0.25;  // 25% de-rating for Stage 4 results
```

### 8.2 Empirical Evidence Summary

**From album_matcher_25_designs.md (Run 23):**
- Edition rank 1 wins: 80.5% (157/195)
- Edition ranks 1-2 win: 91.8% (179/195)
- Edition ranks 1-10 win: 96.9% (189/195)

**Implication:** Stage 1 testing top 3 editions captures ~92% of potential wins

**From Run 27 logs (analyzed):**
- "via assembly" dominates success messages (>90% of matches)
- "via guided quiet spots" appears ~5-10% of matches
- "via merging" not observed (0% in sampled logs)
- "100% match found" triggers early exit, stopping edition testing

**Implication:** Stage 3 is workhorse, Stage 4 is fallback, Stage 5 is rare edge case

**From Run 27 Timing Data (Empirical - 179 albums completed):**
```
Run duration: 6h 29m 51s (23,391 seconds total)
Average: 200.5s per album (3.3 minutes)
Median:  120.0s per album (2.0 minutes)
P90:     360.0s per album (6.0 minutes)
Max:     840.1s per album (14.0 minutes)

Distribution:
  54.7% (98 albums):  120-180s (2-3 minutes)
  31.3% (56 albums):  180-300s (3-5 minutes)
  12.3% (22 albums):  300-600s (5-10 minutes)
   1.7% (3 albums):   600s+    (> 10 minutes)
```

**Key Findings:**
- **Bimodal distribution:** Most albums (54.7%) cluster at 2-3 minutes, suggesting majority match quickly once edition is found
- **Long tail:** 14% take > 5 minutes, indicating complex cases (many editions, difficult matching)
- **Performance bottleneck:** Current complexity-first approach applies 180-parameter sweep to ALL albums, even the 54.7% that complete in 2-3 minutes
- **Optimization opportunity:** Simplicity-first can reduce the 2-3 minute cluster to 1-2 minutes by avoiding unnecessary parameter sweeps

### 8.3 Key Terminology

**Simplicity-first:** Design principle where simpler algorithms are attempted before complex ones. Maximizes early exit rate for common cases.

**Default parameters:** Single well-chosen silence detection parameter set (-54dB, 0.6s) proven optimal from Run 27 analysis.

**Adaptive sweep:** Full 180-parameter grid search (9 thresholds × 20 min durations) using pre-computed WindowDbProfile for fast testing.

**WindowDbProfile:** Pre-computed dB levels for each analysis window. Enables O(windows) filtering vs O(samples) re-scanning for each parameter combination.

**Deterministically matchable:** Albums that should be automatically matched by current algorithms (clear tracks, correct MB data, standard format). Excludes corrupt files, wrong MB data, non-standard formats.

**Unmatchable:** Albums that exhaust all automatic strategies. Includes corrupt files, wrong MB data, medleys/DJ mixes, and algorithm edge case failures.

---

## Document Control

**Version:** 1.2 (Performance-Corrected)
**Revision Date:** 2025-11-25
**Changes from v1.1:**
- **CRITICAL CORRECTION:** Updated performance estimates with empirical Run 27 data
- **Current performance:** 200s average (not 7-8s as initially estimated)
- **Proposed improvement:** 1.74x faster (115s vs 200s), not 2x
- **180-parameter sweep cost:** 60-90s (not 5-6s), significant optimization opportunity
- **Performance breakdown:** Based on actual timing distribution from 179 albums

**Changes from v1.0:**
- Eliminated manual review queue (Stage 4 in v1.0)
- Retained full 180-parameter adaptive sweep (vs 16-parameter reduction in v1.0)
- Preserved all Run 27 stages (Stages 2-5) in new ordering
- Clarified "unmatchable" definition (corrupt, wrong MB, non-standard)
- Distinguished default parameters (Stage 1) from adaptive sweep (Stage 2)

**Author:** Analysis based on album_matcher_27.rs and empirical run data
**Review Status:** Revised per stakeholder feedback (automatic matching required)
**Next Steps:** Validate Stage 1 success rate (Phase 1 implementation)

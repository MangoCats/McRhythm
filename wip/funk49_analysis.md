# Funk #49 Album Matching Analysis: Run 7 vs Run 10

## Executive Summary

**Run 7:** 100% match (10/10 tracks, Excellent) via segment assembly
**Run 10:** 50% match (5/10 tracks, Fair) - DEGRADED

**Root Cause:** Run 10's Stage 2 parameter optimization discards over-segmented candidates that could assemble into perfect matches.

---

## Detailed Results Comparison

### Run 7 (James Gang - Funk #49)
```
STAGE 1: Testing default parameters (-57dB, 0.9s)
  - Found 9 tracks
  - Match: 0.0% (Poor confidence)

STAGE 2: Testing 180 parameter combinations
  - New best: 10.0% with -44dB, 0.5s
  - New best: 20.0% with -44dB, 2s
  - New best: 30.0% with -50dB, 2s
  - New best: 50.0% with -60dB, 0.3s
  - Best: 50.0% (Fair confidence)

STAGE 3: Comprehensive segment assembly across 109 over-segmented candidates
  - Tested 85 assemblies
  - New best: 80.0% via assembly of (-44dB, 0.05s) → 102 segments → 10 tracks
  - New best: 90.0% via assembly of (-44dB, 0.3s) → 26 segments → 10 tracks
  - New best: 100.0% via assembly of (-58dB, 0.3s) → 18 segments → 10 tracks ✓

FINAL RESULT: 10/10 tracks (100.0%), Mean error: 3.49s
```

### Run 10 (James Gang - Funk #49)
```
STAGE 1: Testing default parameters (-57dB, 0.9s)
  - Found 9 tracks
  - Best match: 12.5% with 8 tracks from Instant Funk (Poor)

STAGE 2: Testing 180 parameter combinations against all editions
  - New best: 16.7% with -36dB, 0.25s → 12 tracks
  - New best: 18.8% with -36dB, 0.5s → 16 tracks
  - New best: 22.2% with -36dB, 0.8s → 9 tracks
  - New best: 30.0% with -36dB, 4s → 10 tracks
  - New best: 33.3% with -38dB, 1.5s → 9 tracks
  - New best: 37.5% with -47dB, 1.5s → 8 tracks
  - New best: 50.0% with -60dB, 0.3s → 10 tracks ✓
  - Best: 50.0% (Fair confidence)

STAGE 3: Attempting segment assembly against all editions
  - Tested 14 assemblies, 0 improved over best
  - NO ASSEMBLY ATTEMPTED: Current best = 10 tracks, Target = 10 tracks

STAGE 4: Attempting quiet spot detection
  - Tested 40 configurations, 0 improved

FINAL RESULT: 5/10 tracks (50.0%), Mean error: 2.93s ✗
```

---

## Algorithmic Differences

### Run 7 Architecture (Successful)

**Stage 2: Parameter Optimization**
- Tests 180 parameter combinations
- **Collects ALL over-segmented results** (stored 109 candidates)
- Keeps any segmentation with MORE tracks than any MB candidate edition
- These become inputs to Stage 3

**Stage 3: Comprehensive Assembly**
```
for each over-segmented_candidate in collected_candidates:
    for each target_edition in mb_editions:
        if candidate.tracks > target_edition.tracks:
            try_assemble(candidate → target_edition)
            test_assembled_result()
```
- Tried 85 different assemblies
- Found perfect match: 18 segments → 10 tracks

### Run 10 Architecture (Failed)

**Stage 2: Parameter Optimization**
**album_matcher.rs:2010-2064**
```rust
fn run_stage2_parameter_optimization(...) -> Option<CandidateTestResult> {
    let mut best_result: Option<CandidateTestResult> = None;

    for &thresh in threshold_values {
        for &min_dur in min_duration_values {
            let test_durations = get_track_durations(samples, sample_rate, thresh, min_dur);

            if let Some(result) = test_segmentation_against_all_candidates(...) {
                if improved && result.percentage > current_best_percentage {
                    best_result = Some(result);  // ONLY KEEPS BEST ❌
                }
            }
        }
    }

    best_result  // Returns SINGLE best result
}
```
- Tests 180 parameter combinations
- **Only keeps the SINGLE BEST result** (50% match with 10 tracks)
- **Discards all over-segmented candidates** (like 18-segment result)

**Stage 3: Limited Assembly**
**album_matcher.rs:2068-2122**
```rust
fn run_stage3_segment_assembly(
    best_durations: &[f64],  // ONLY receives single best from Stage 2 ❌
    mb_candidates: &[(Vec<u32>, String)],
    ...
) -> Option<CandidateTestResult> {
    for (expected_u32, _mbid) in mb_candidates {
        // Only try assembly if current segmentation is over-segmented
        if best_durations.len() > expected_u32.len() {  // FAILS for Funk #49 ❌
            // Try assembly...
        }
    }
}
```
- Only receives SINGLE best segmentation (10 tracks)
- Condition: `if best_durations.len() > expected_u32.len()`
- For Funk #49: 10 tracks detected == 10 tracks expected
- **No assembly attempted** because not over-segmented

**Result:** Stuck at 50% match because the perfect-assembling 18-segment candidate was discarded in Stage 2.

---

## Root Cause Analysis

### Critical Flaw in Run 10

**Stage 2 Optimization Paradox:**
- A **low-scoring over-segmented result** (e.g., 18 segments with 20% initial match) may be discarded
- That same result could **assemble into a perfect match** (100%) when merged
- But it never reaches Stage 3 because Stage 2 only keeps the best immediate score

**Evidence from Funk #49:**
1. Stage 2 probably generated an 18-segment result at -58dB, 0.3s
2. This 18-segment result had LOW immediate match score (<50%)
3. It was discarded in favor of 10-segment 50% match
4. The 10-segment result cannot be assembled (not over-segmented)
5. The 18-segment result COULD have assembled to 100% but was lost

### Why Run 7 Succeeded

**Key Insight:** Run 7 separated **segmentation exploration** from **match scoring**

1. **Exploration Phase:** Generate many diverse segmentations (109 candidates)
2. **Assembly Phase:** Try assembling each candidate to each target
3. **Selection Phase:** Pick best assembled result

This allows low-scoring over-segmented candidates to prove their value through assembly.

---

## Suggested Improvements for Future Runs

### Strategy 1: Collect Over-Segmented Candidates (Recommended)

**Modify Stage 2 to collect ALL over-segmented parameter combinations:**

```rust
// CURRENT (Run 10 - WRONG):
fn run_stage2_parameter_optimization(...) -> Option<CandidateTestResult> {
    let mut best_result: Option<CandidateTestResult> = None;
    // Only keeps best_result ❌
}

// PROPOSED (Run 7 style - CORRECT):
fn run_stage2_parameter_optimization(...) -> (
    Option<CandidateTestResult>,           // Best immediate match
    Vec<(Vec<f64>, f64, f64)>              // All over-segmented candidates: (durations, thresh, min_dur)
) {
    let mut best_result: Option<CandidateTestResult> = None;
    let mut over_segmented_candidates = Vec::new();

    // Get max track count across all MB editions
    let max_expected_tracks = mb_candidates.iter()
        .map(|(durs, _)| durs.len())
        .max()
        .unwrap_or(0);

    for &thresh in threshold_values {
        for &min_dur in min_duration_values {
            let test_durations = get_track_durations(samples, sample_rate, thresh, min_dur);

            // Track best immediate match
            if let Some(result) = test_segmentation_against_all_candidates(...) {
                if improved {
                    best_result = Some(result);
                }
            }

            // ALSO collect over-segmented candidates for assembly ✓
            if test_durations.len() > max_expected_tracks {
                over_segmented_candidates.push((test_durations.clone(), thresh, min_dur));
            }
        }
    }

    (best_result, over_segmented_candidates)
}
```

**Modify Stage 3 to use collected candidates:**

```rust
fn run_stage3_comprehensive_assembly(
    over_segmented_candidates: &[(Vec<f64>, f64, f64)],  // All candidates to try
    mb_candidates: &[(Vec<u32>, String)],
    tolerance: f64,
    current_best_percentage: f64,
) -> Option<CandidateTestResult> {
    println!("  STAGE 3: Comprehensive segment assembly across {} over-segmented candidates...",
        over_segmented_candidates.len());

    let mut assemblies_tested = 0;
    let mut best_result: Option<CandidateTestResult> = None;

    for (candidate_durations, thresh, min_dur) in over_segmented_candidates {
        for (expected_u32, _mbid) in mb_candidates {
            if candidate_durations.len() > expected_u32.len() {
                if let Some(assembled) = assemble_segments_dp(candidate_durations, expected_u32) {
                    assemblies_tested += 1;

                    if let Some(result) = test_segmentation_against_all_candidates(...) {
                        if result.percentage > current_best_percentage {
                            println!("    New best: {:.1}% via assembly of ({}, {}) ({} segments → {} tracks)",
                                result.percentage, thresh, min_dur,
                                candidate_durations.len(), expected_u32.len());
                            best_result = Some(result);

                            if result.percentage >= 100.0 {
                                return best_result;  // Early exit on perfect
                            }
                        }
                    }
                }
            }
        }
    }

    best_result
}
```

**Expected Impact:**
- **Funk #49:** Would collect 18-segment candidate from -58dB, 0.3s
- Stage 3 would try assembling 18→10 tracks
- Would achieve 100% match like Run 7
- **Risk to other albums:** LOW - only ADDS candidate attempts, doesn't remove existing logic

---

### Strategy 2: Try Assembly Even When Not Over-Segmented (Complementary)

Sometimes perfect segmentation has imperfect boundaries. Try small merges/adjustments:

```rust
fn run_stage3_segment_assembly(...) {
    // CURRENT: Only assembles if over-segmented
    if best_durations.len() > expected_u32.len() {
        // Try assembly...
    }

    // PROPOSED: ALSO try boundary adjustments when counts match
    else if best_durations.len() == expected_u32.len() {
        // Try small boundary adjustments (±10% track duration)
        // This handles cases where track boundaries are slightly off
        if let Some(adjusted) = adjust_track_boundaries(best_durations, expected_u32, tolerance) {
            // Test adjusted result...
        }
    }
}
```

**Expected Impact:**
- May help albums with correct track count but poor boundaries
- **Risk:** MEDIUM - could degrade some results if boundaries get worse

---

### Strategy 3: Prioritize Over-Segmented Parameters (Preventive)

Test finer-grained parameters FIRST in Stage 2 to find over-segmented candidates early:

```rust
// Sort parameters to test over-segmentation-prone values first
let mut param_pairs: Vec<(f64, f64)> = threshold_values.iter()
    .flat_map(|&t| min_duration_values.iter().map(move |&d| (t, d)))
    .collect();

// Sort by likelihood of over-segmentation:
// - Stricter thresholds (more negative) come first
// - Shorter min durations come first
param_pairs.sort_by(|a, b| {
    let score_a = a.0 + (a.1 * 100.0);  // Favor strict+short
    let score_b = b.0 + (b.1 * 100.0);
    score_a.partial_cmp(&score_b).unwrap()
});
```

**Expected Impact:**
- Finds assembly candidates earlier → faster early exit on perfect matches
- **Risk:** NONE - just reorders search, doesn't change logic

---

## Recommendation

**Implement Strategy 1 (Collect Over-Segmented Candidates) in Run 12+**

**Rationale:**
1. **Direct fix** for Funk #49 failure mode
2. **Low risk** - only adds attempts, doesn't change existing logic
3. **Matches Run 7** architecture which achieved 100% on training set
4. **Preserves existing wins** - all other Run 10 improvements remain

**Expected Results:**
- Funk #49: 50% → 100% (restored to Run 7 level)
- Other albums: No degradation (only adds assembly candidates)
- Overall success rate: Should match or exceed Run 7's 100% training set rate

**Implementation Priority:**
- HIGH: This is a regression fix (Run 7 worked, Run 10 broke it)
- EFFORT: Moderate (~100 lines of code changes)
- TESTING: Validate on Funk #49 first, then full 200-album set

---

## Appendix: Track-by-Track Error Details

### Run 7 (100% match)
```
1.  229.9s vs  233s  error= 3.1s  ✓
2.  196.4s vs  200s  error= 3.6s  ✓
3.  167.6s vs  170s  error= 2.4s  ✓
4.  323.2s vs  332s  error= 8.8s  ✓
5.  164.1s vs  167s  error= 2.9s  ✓
6-10: All matched (not shown in excerpt)
```

### Run 10 (50% match)
```
1.  230.0s vs  233s  error= 3.0s  ✓
2.  196.4s vs  200s  error= 3.6s  ✓
3.  167.7s vs  170s  error= 2.3s  ✓
4.   10.5s vs  332s  error=321.5s ✗  <- Wrong track!
5.  480.0s vs  167s  error=313.0s ✗  <- Wrong track!
6-10: Not matched (detected 9 tracks total)
```

**Analysis:** Tracks 1-3 match perfectly in both runs. Track 4+ diverge because Run 10's 10-track segmentation has incorrect boundaries, creating very short (10.5s) and very long (480.0s) false segments.

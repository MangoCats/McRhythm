# Run 12 Recommendations: Comprehensive Album Matcher Improvements

## Executive Summary

**Problem:** Run 10/11 regressed Funk #49 from 100% (Run 7) to 50% match.
**Root Cause:** Stage 3 segment assembly receives only SINGLE BEST segmentation from Stage 2, discarding over-segmented candidates that could assemble into perfect matches.
**Solution:** Collect ALL over-segmented candidates during Stage 2, pass to Stage 3 for comprehensive assembly.

---

## Funk #49 Comparison: Run 7 vs Run 10 vs Run 11

| Metric | Run 7 | Run 10 | Run 11 |
|--------|-------|--------|--------|
| Stage 1 (Default) | 9 tracks, 0.0% | 9 tracks, 12.5% | 9 tracks, 16.7% |
| Stage 2 (Optimization) | 50.0% (-60dB, 0.3s, 10 trk) | 50.0% (-60dB, 0.3s, 10 trk) | 50.0% (-60dB, 0.3s, 10 trk) |
| Stage 3 (Assembly) | **100%** via 18→10 assembly | 14 assemblies, 0 improved | 16 assemblies, 0 improved |
| **Final Result** | **100.0% (10/10)** | 50.0% (5/10) ✗ | 50.0% (5/10) ✗ |

**Key Insight:** All three runs achieve identical Stage 2 results (50% with 10 tracks). The divergence occurs in Stage 3:
- **Run 7:** Had access to 109 over-segmented candidates → Found 100% match by assembling 18 segments
- **Run 10/11:** Only had 10-track best result → No over-segmentation → No assembly possible

---

## Root Cause Analysis

### The Problem: Single Best Selection

**Run 10/11 Stage 2 Code (album_matcher.rs:2010-2064):**
```rust
fn run_stage2_parameter_optimization(...) -> Option<CandidateTestResult> {
    let mut best_result: Option<CandidateTestResult> = None;  // ← SINGLE result

    for &thresh in threshold_values {
        for &min_dur in min_duration_values {
            let test_durations = get_track_durations(...);

            if let Some(result) = test_segmentation_against_all_candidates(...) {
                if improved && result.percentage > current_best_percentage {
                    best_result = Some(result);  // ← Overwrites, discards previous
                }
            }
        }
    }

    best_result  // ← Returns ONLY the single best
}
```

**Problem:** During the 180 parameter sweeps, Funk #49 likely generated:
- -58dB, 0.3s → 18 segments (low immediate match ~20%)
- -60dB, 0.3s → 10 segments (best immediate match 50%)

The 18-segment result was **discarded** because it had lower immediate score, but it could have **assembled to 100%**.

### The Solution: Collect Over-Segmented Candidates

**Run 7 Approach (Successful):**
1. Stage 2: Test all parameter combinations
2. **Collect ALL over-segmented results** (109 candidates with more segments than targets)
3. Stage 3: Try assembling EACH candidate against EACH target edition
4. Found: 18 segments (-58dB, 0.3s) → assembles to 10 tracks → 100% match

---

## Run 12 Implementation Plan

### Phase 1: Core Algorithm Fix (HIGH PRIORITY)

**1.1 Modify Stage 2 Return Type**

Change from:
```rust
fn run_stage2_parameter_optimization(...) -> Option<CandidateTestResult>
```

To:
```rust
struct Stage2Results {
    best_result: Option<CandidateTestResult>,
    over_segmented_candidates: Vec<OverSegmentedCandidate>,
}

struct OverSegmentedCandidate {
    durations: Vec<f64>,
    threshold_db: f64,
    min_duration_secs: f64,
    track_count: usize,
}

fn run_stage2_parameter_optimization(...) -> Stage2Results
```

**1.2 Collect Over-Segmented Candidates During Stage 2**

```rust
fn run_stage2_parameter_optimization(
    samples: &[f32],
    sample_rate: u32,
    threshold_values: &[f64],
    min_duration_values: &[f64],
    mb_candidates: &[(Vec<u32>, String)],
    tolerance: f64,
    current_best_percentage: f64,
) -> Stage2Results {
    let mut best_result: Option<CandidateTestResult> = None;
    let mut over_segmented_candidates: Vec<OverSegmentedCandidate> = Vec::new();

    // Get max expected track count across all editions
    let max_expected_tracks = mb_candidates.iter()
        .map(|(durs, _)| durs.len())
        .max()
        .unwrap_or(0);

    for &thresh in threshold_values {
        for &min_dur in min_duration_values {
            let test_durations = get_track_durations(samples, sample_rate, thresh, min_dur);

            // Track best immediate match (existing behavior)
            if let Some(result) = test_segmentation_against_all_candidates(&test_durations, mb_candidates, tolerance) {
                let improved = best_result.as_ref().map_or(true, |br| result.percentage > br.percentage);
                if improved && result.percentage > current_best_percentage {
                    best_result = Some(result);

                    if best_result.as_ref().unwrap().percentage >= 100.0 {
                        return Stage2Results { best_result, over_segmented_candidates };
                    }
                }
            }

            // NEW: Collect over-segmented candidates for Stage 3 assembly
            if test_durations.len() > max_expected_tracks {
                over_segmented_candidates.push(OverSegmentedCandidate {
                    durations: test_durations,
                    threshold_db: thresh,
                    min_duration_secs: min_dur,
                    track_count: test_durations.len(),
                });
            }
        }
    }

    Stage2Results { best_result, over_segmented_candidates }
}
```

**1.3 Modify Stage 3 to Use Collected Candidates**

```rust
fn run_stage3_comprehensive_assembly(
    over_segmented_candidates: &[OverSegmentedCandidate],
    mb_candidates: &[(Vec<u32>, String)],
    tolerance: f64,
    current_best_percentage: f64,
) -> Option<CandidateTestResult> {
    if current_best_percentage >= 100.0 {
        return None;
    }

    println!("  STAGE 3: Comprehensive segment assembly across {} over-segmented candidates...",
        over_segmented_candidates.len());

    let mut assemblies_tested = 0;
    let mut assemblies_improved = 0;
    let mut best_result: Option<CandidateTestResult> = None;

    for candidate in over_segmented_candidates {
        for (expected_u32, mbid) in mb_candidates {
            if candidate.durations.len() > expected_u32.len() {
                if let Some(assembled_durations) = assemble_segments_dp(&candidate.durations, expected_u32) {
                    assemblies_tested += 1;

                    if let Some(result) = test_segmentation_against_all_candidates(
                        &assembled_durations, mb_candidates, tolerance
                    ) {
                        let improved = best_result.as_ref()
                            .map_or(true, |br| result.percentage > br.percentage);

                        if improved && result.percentage > current_best_percentage {
                            assemblies_improved += 1;
                            println!("    New best: {:.1}% via assembly of ({}dB, {}s) ({} segments → {} tracks)",
                                result.percentage,
                                candidate.threshold_db, candidate.min_duration_secs,
                                candidate.track_count, expected_u32.len());

                            best_result = Some(result);

                            if best_result.as_ref().unwrap().percentage >= 100.0 {
                                println!("    Tested {} assemblies, {} improved over best",
                                    assemblies_tested, assemblies_improved);
                                return best_result;
                            }
                        }
                    }
                }
            }
        }
    }

    println!("    Tested {} assemblies, {} improved over best", assemblies_tested, assemblies_improved);
    best_result
}
```

**1.4 Update Main Processing Loop**

```rust
// Stage 2
let stage2_results = run_stage2_parameter_optimization(
    &samples, sample_rate,
    &threshold_values, &min_duration_values,
    &edition_candidates, match_tolerance_secs,
    current_best_percentage
);

if let Some(ref r) = stage2_results.best_result {
    // Update tracking...
    current_best_percentage = r.percentage;
    best_durations = r.detected_durations.clone();
}

// Stage 3 - Use collected over-segmented candidates
if let Some(result) = run_stage3_comprehensive_assembly(
    &stage2_results.over_segmented_candidates,
    &edition_candidates,
    match_tolerance_secs,
    current_best_percentage
) {
    // Update tracking...
}
```

### Phase 2: Expected Outcomes

**For Funk #49:**
- Stage 2: Will still find 50% match (10 tracks)
- Stage 2: Will ALSO collect ~50-100 over-segmented candidates (including 18-segment result)
- Stage 3: Will test ~200-400 assemblies (vs current 14-16)
- Stage 3: Will find 100% match by assembling 18→10 tracks
- **Expected Result: 100.0% (restored to Run 7 performance)**

**For Other Albums:**
- No degradation expected - this is purely additive
- May improve some albums that were stuck at sub-optimal matches
- Will increase Stage 3 runtime slightly (more assemblies to test)

---

## Additional Recommendations for Run 12

### 2.1 Code Cleanup (During Refactor)

Since Run 12 should be a "refactored/cleaned up version of album_matcher_11.rs":

1. **Extract Constants to Top-Level:**
   ```rust
   const DEFAULT_THRESHOLD_DB: f64 = -57.0;
   const DEFAULT_MIN_DURATION_SECS: f64 = 0.9;
   const MATCH_TOLERANCE_SECS: f64 = 10.0;
   const RUNTIME_FILTER_TOLERANCE: f64 = 0.25;  // 25%
   ```

2. **Consolidate Edition Structs:**
   - `EditionCandidate`, `CandidateTestResult`, etc. should have clear relationships
   - Consider using a builder pattern for complex structs

3. **Remove Dead Code:**
   - `MatchContext` struct (never constructed)
   - `test_all_combinations` function (never used)
   - `match_single_file` function (never used)
   - `output_match_result` function (never used)

4. **Fix Compiler Warnings:**
   - Unused variables: `file_duration_secs`, `id3_track_count`, `matched_count`
   - Unused mutable: `best_threshold`, `best_min_duration`

### 2.2 Logging Improvements (Already in Run 11)

Run 11 added enhanced result reporting with NDR scores. Additional improvements:

1. **Log Over-Segmented Candidate Count:**
   ```rust
   println!("  Collected {} over-segmented candidates for Stage 3 assembly",
       over_segmented_candidates.len());
   ```

2. **Log Assembly Source on Success:**
   ```rust
   println!("    Best assembly found from {}dB, {}s ({} segments → {} tracks)",
       best_candidate.threshold_db, best_candidate.min_duration_secs,
       best_candidate.track_count, target_track_count);
   ```

### 2.3 Performance Optimization (LOW PRIORITY)

If Stage 3 becomes slow due to more assemblies:

1. **Deduplicate Over-Segmented Candidates:**
   - Some parameter combinations may produce identical segmentations
   - Hash by segment count + total duration to avoid redundant assembly attempts

2. **Prioritize Likely Candidates:**
   - Sort over-segmented candidates by segment count closest to target
   - Stop early if perfect match found

3. **Parallel Assembly Testing:**
   - Assembly attempts are independent → can parallelize with rayon

---

## Risk Assessment

### Low Risk
- **Algorithm change:** Purely additive - keeps existing Stage 2 best selection, adds candidate collection
- **Backward compatibility:** All existing matches preserved
- **Runtime impact:** Slight increase in Stage 3 time (proportional to candidate count)

### Medium Risk (Mitigated)
- **Memory usage:** Collecting many candidates could use memory
  - **Mitigation:** Limit to ~200 candidates, deduplicate by segment count
- **False positives:** Assembly might create spurious matches
  - **Mitigation:** Assembly result still tested against ALL editions; best match wins

---

## Validation Plan

1. **Unit Test:** Verify Funk #49 achieves 100% match with new algorithm
2. **Regression Test:** Run full 200-album set, compare to Run 7 and Run 11
3. **Expected Outcomes:**
   - Funk #49: 50% → 100% ✓
   - All Run 7 100% albums: Should remain 100%
   - Run 11 non-100% albums: May improve (some may benefit from assembly)

---

## Summary

**Run 12 Core Change:**
1. Stage 2: Return `Stage2Results { best_result, over_segmented_candidates }` instead of `Option<CandidateTestResult>`
2. Stage 3: Accept and iterate over `over_segmented_candidates` instead of single `best_durations`
3. Assembly: Test each over-segmented candidate against each target edition

**Expected Impact:**
- Funk #49: 50% → 100% (restored)
- Other albums: No degradation, potential improvement
- Code quality: Cleaner, more maintainable structure

**Implementation Effort:** ~100-150 lines of code changes, medium complexity refactor.

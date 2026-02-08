# Algorithm Improvement Roadmap
**Created:** 2025-12-31
**Goal:** Systematically improve album matching to eliminate regressions vs. run29f baseline while maximizing improvements

---

## Test Protocol (ALL Experiments)

### Standard Test Procedure
1. **Build:** `cargo build --release`
2. **Run Test:** `cargo test --release run29f_full_baseline_comparison -- --nocapture`
3. **Save Results:** Copy `wkmp-ai/run29f_comparison_results.json` to `am/experimentNN_results.json`
4. **Analyze:** Categorize all 122 differing albums as Better/Equivocal/Worse
5. **Document:** Record metrics in this file
6. **Decide:** GO/NO-GO for next experiment based on success criteria

### Success Criteria
- **CRITICAL:** Zero "clearly worse" regressions (current: 7)
- **TARGET:** Maintain ≥81 "clearly better" improvements
- **ACCEPTABLE:** Equivocal changes may increase/decrease

### Baseline Metrics (Current State)
- **Configuration:** Negative floor -1.0 + boundary refinement in Stage 2+4
- **Exact matches:** 66/193 (34.2%)
- **MBID changes:** 122/193 (63.2%)
- **Clearly better:** 81/122 (66.4%)
- **Equivocal:** 34/122 (27.9%)
- **Clearly worse:** 7/122 (5.7%) ⚠️ **TARGET FOR ELIMINATION**

---

## Phase 1: Quality Score Floor Experiments

### Hypothesis
The -1.0 negative floor causes quality score collapse when boundary detection fails, enabling degenerate 1-track matches to beat correct multi-track matches.

### Rationale
run29f has no quality score component—uses binary match percentage (track matches or doesn't). Current implementation's graduated quality penalties with negative floor create catastrophic failures.

---

### Experiment 1A: No Negative Floor (0.0)

**Change:**
```rust
// wkmp-ai/src/matching/editions/scoring.rs:339
const MIN_QUALITY_FLOOR: f64 = 0.0;  // Match run29f: bad tracks contribute 0, not negative
```

**Expected Outcome:**
- Fix The Cars - Panorama (quality score no longer collapses)
- Fix other 6 regressions with low quality scores
- Maintain 81 improvements (different mechanism than floor)

**Success Criteria:**
- ≤2 "clearly worse" (from 7)
- ≥75 "clearly better"

**Decision Point:**
- **IF SUCCESS:** Proceed to Experiment 1B (reduced floor comparison)
- **IF FAILURE:** Analyze why, consider Experiment 2 (revert to run29f scoring)

---

### Experiment 1B: Reduced Negative Floor (-0.3)

**Prerequisites:** Run ONLY if Experiment 1A succeeds

**Change:**
```rust
// wkmp-ai/src/matching/editions/scoring.rs:339
const MIN_QUALITY_FLOOR: f64 = -0.3;  // Softer penalty than -1.0
```

**Expected Outcome:**
- May fix regressions while preserving some discrimination
- Better than -1.0, possibly worse than 0.0

**Success Criteria:**
- ≤2 "clearly worse"
- ≥75 "clearly better"

**Decision Point:**
- **COMPARE:** 0.0 vs. -0.3 results
- **SELECT:** Configuration with fewer regressions OR more improvements if regressions equal
- **OUTCOME:** Lock quality floor value for Phase 2

---

### Experiment 1C: Negative Floor (-0.5)

**Prerequisites:** Run ONLY if both 1A and 1B succeed but have tradeoffs

**Change:**
```rust
const MIN_QUALITY_FLOOR: f64 = -0.5;  // Middle ground
```

**Goal:** Find optimal balance if 0.0 and -0.3 show different strengths

**Decision Point:**
- **FINAL:** Select best of {0.0, -0.3, -0.5} based on metrics
- **LOCK:** Quality floor value for all subsequent experiments

---

## Phase 2: Boundary Refinement Timing

### Hypothesis
Boundary refinement runs AFTER edition scoring/selection. Wrong editions get selected before refinement can improve boundary detection. Moving refinement BEFORE selection should improve match accuracy.

### Prerequisites
- Phase 1 complete with quality floor locked

---

### Experiment 2A: Refinement Before Selection (Stage 2)

**Current Flow:**
```
Stage 2:
1. Test edition with all 180 parameter combinations
2. Select best parameters by match %
3. Apply boundary refinement to winner's detected durations
4. Return Stage2Result with refined durations

Orchestrator:
5. Score all editions using refined durations
6. Select best edition
```

**Problem:** Refinement happens per-edition AFTER parameter selection, but BEFORE inter-edition comparison. This is actually already optimal for Stage 2.

**New Flow:**
```
Stage 2:
1. Test edition with all 180 parameter combinations
2. Select best parameters by match %
3. Apply refinement BEFORE calculating final match %
4. Recalculate match % with refined durations
5. Return Stage2Result with refined durations + updated match %
```

**Change:**
Move refinement in [stage2.rs:190-206](wkmp-ai/src/matching/stages/stage2.rs#L190-L206) to BEFORE `best_percentage` calculation, then recalculate.

**Expected Outcome:**
- Refined boundaries improve match % calculation
- Better editions selected due to improved scoring
- The Cars may still fail if wrong edition selected before refinement

**Success Criteria:**
- ≤1 "clearly worse" (from Phase 1 result)
- ≥80 "clearly better"

**Decision Point:**
- **IF SUCCESS:** Proceed to Experiment 2B (Stage 4 refinement)
- **IF FAILURE:** Revert, document that current timing is optimal

---

### Experiment 2B: Iterative Refinement in Stage 2

**Prerequisites:** Experiment 2A shows improvement

**Change:**
Apply refinement in loop during parameter search:
```rust
for threshold_idx in 0..num_thresholds {
    for duration_idx in 0..num_min_durations {
        let detected = silence_cache.get(cache_idx);

        // NOVEL: Apply refinement before scoring
        let refined = apply_refinement_to_durations(
            detected,
            &edition.durations,
            audio_samples,
            sample_rate,
            tolerance_secs
        );

        let result = analyze_track_matching(&refined, &edition.durations, tolerance_secs);

        if result.percentage > best_percentage {
            best_percentage = result.percentage;
            // ... update best
        }
    }
}
```

**Expected Outcome:**
- Each parameter combination gets refined boundaries before scoring
- Better parameter selection (considers refined results)
- Higher computational cost (refinement × 180 per edition)

**Success Criteria:**
- ≥2 more improvements than Experiment 2A
- Computational cost acceptable (≤2× runtime)

**Decision Point:**
- **COMPARE:** Cost vs. benefit
- **IF BENEFIT > COST:** Keep iterative refinement
- **ELSE:** Revert to 2A timing

---

## Phase 3: Scoring Algorithm Experiments

### Hypothesis
Multi-factor scoring (duration 30%, quality 45%, name 25%) is more complex than run29f's binary match percentage but causes regressions. Simpler may be better.

### Prerequisites
- Phase 1 complete (quality floor locked)
- Phase 2 complete (refinement timing locked)

---

### Experiment 3A: Pure Match Percentage (run29f Style)

**Change:**
Modify orchestrator to sort editions by:
```rust
// Primary: match percentage (binary: track matches or doesn't)
// Secondary: mean error (tie-breaker)
// NO quality score, NO duration score, NO name score

sorted_editions.sort_by(|a, b| {
    // 1. Match percentage (descending)
    let pct_cmp = b.match_pct.partial_cmp(&a.match_pct);
    if pct_cmp != Ordering::Equal {
        return pct_cmp;
    }

    // 2. Mean error (ascending, tie-breaker)
    a.mean_error.partial_cmp(&b.mean_error)
});
```

**Expected Outcome:**
- Simpler algorithm, fewer edge cases
- May lose some of 81 improvements (unknown)
- Should match run29f behavior closely

**Success Criteria:**
- 0 "clearly worse" (absolute requirement)
- ≥70 "clearly better" (acceptable loss if regressions eliminated)

**Decision Point:**
- **IF 0 REGRESSIONS:** Seriously consider adoption
- **COMPARE:** Improvement count vs. current multi-factor
- **FINAL:** Select algorithm for Phase 4

---

### Experiment 3B: Hybrid Scoring (Match % + Quality)

**Prerequisites:** Run if 3A loses too many improvements

**Change:**
```rust
// Primary: match percentage (descending)
// Secondary: quality score (descending, tie-breaker within same match %)
// Tertiary: mean error (ascending, final tie-breaker)

sorted_editions.sort_by(|a, b| {
    // 1. Match percentage
    let pct_cmp = b.match_pct.partial_cmp(&a.match_pct);
    if pct_cmp != Ordering::Equal {
        return pct_cmp;
    }

    // 2. Quality score (only if match % equal)
    let qual_cmp = b.quality.partial_cmp(&a.quality);
    if qual_cmp != Ordering::Equal {
        return qual_cmp;
    }

    // 3. Mean error
    a.mean_error.partial_cmp(&b.mean_error)
});
```

**Expected Outcome:**
- Match percentage dominates (like run29f)
- Quality score refines ties (preserves improvements)
- Best of both worlds

**Success Criteria:**
- 0 "clearly worse"
- ≥80 "clearly better"

---

### Experiment 3C: Weighted Scoring with Quality Capped

**Prerequisites:** Run if 3B shows promise but needs refinement

**Change:**
```rust
// Reduce quality weight from 45% to 20%
base_score = (duration_score × 0.30) + (quality_score × 0.20) + (name_score × 0.50)

// Or cap quality impact
let quality_contribution = (quality_score * 0.45).max(0.0);  // Never negative
base_score = (duration_score × 0.30) + quality_contribution + (name_score × 0.25);
```

**Expected Outcome:**
- Reduced quality score impact prevents catastrophic failures
- Maintains graduated scoring benefits

---

## Phase 4: Parameter Tuning

### Prerequisites
- Phase 1-3 complete with algorithm locked

### Experiment 4A: Tolerance Adjustment

**Current:** 10s tolerance

**Test Values:**
- 8s (stricter)
- 12s (more forgiving)
- 15s (very forgiving)

**Hypothesis:** Larger tolerance may help albums with consistent timing drift.

**Method:** Test each value, measure impact on regressions vs. improvements

---

### Experiment 4B: Track Count Penalty Tuning

**Current:** Graduated penalties (0.95 for ±1 track, 0.85 for ±2, etc.)

**Test Values:**
- run29f style: 4% per extra track (simpler)
- Current graduated (baseline)
- Aggressive: 6% per extra track

**Method:** Test each, optimize for box set discrimination

---

### Experiment 4C: Silence Detection Parameters

**Current:** 180 combinations (12 thresholds × 15 min durations)

**Optimization:**
- Analyze which parameter ranges never win
- Reduce grid to 90 combinations (6 × 15) for performance
- Test if accuracy maintained

---

## Phase 5: Novel Algorithms

### Prerequisites
- Phase 1-4 complete with baseline locked

---

### Experiment 5A: Confidence-Weighted Selection

**Hypothesis:** High match % with high mean error indicates lucky coincidence, not true match.

**Algorithm:**
```rust
confidence_score = match_pct × (1.0 - normalized_mean_error)

where normalized_mean_error = mean_error / tolerance
```

**Expected Outcome:** Penalize matches with high errors even if many tracks match

---

### Experiment 5B: Median Quality Score

**Hypothesis:** Mean quality is vulnerable to outliers. Median resists catastrophic track failures.

**Change:**
```rust
// Current
let avg_quality = qualities.iter().sum() / qualities.len();

// Proposed
let mut sorted_qualities = qualities.clone();
sorted_qualities.sort_by(|a, b| a.partial_cmp(b).unwrap());
let median_quality = sorted_qualities[qualities.len() / 2];
```

**Expected Outcome:** Outlier resistance, fewer catastrophic failures

---

### Experiment 5C: Adaptive Tolerance per Track

**Hypothesis:** Short tracks should have tighter tolerance, long tracks looser.

**Algorithm:**
```rust
// Current: Fixed 10s tolerance for all tracks
// Proposed: 5% of expected duration or 10s, whichever is larger
let adaptive_tolerance = max(10.0, expected_duration * 0.05);
```

**Expected Outcome:** Better discrimination for long tracks, stricter for short

---

### Experiment 5D: Two-Pass Boundary Detection

**Hypothesis:** First pass detects obvious boundaries, second pass refines in problem regions.

**Algorithm:**
```
Pass 1: Standard silence detection (180 grid search)
Pass 2: For tracks with error > tolerance:
  - Focus search in ±15s window around expected boundary
  - Use RMS quiet spot detection (Stage 4 technique)
  - Update boundaries if improvement found
```

**Expected Outcome:** Better boundary detection in difficult regions

---

## Tracking Template

### Experiment N: [Name]

**Date:** YYYY-MM-DD
**Configuration:**
- Quality floor: [value]
- Scoring algorithm: [description]
- Refinement timing: [description]
- Other parameters: [list]

**Code Changes:**
- File: [path]
- Lines: [range]
- Description: [what changed]

**Results:**
```
Total albums tested: 193
Exact matches: X/193 (X.X%)
MBID changes: X/193 (X.X%)

Of X MBID changes:
- Clearly better: X (X.X%)
- Equivocal: X (X.X%)
- Clearly worse: X (X.X%)
```

**Key Findings:**
- [Bullet points of observations]

**Critical Cases:**
- The Cars - Panorama: [MBID, tracks, status]
- Eagles - The Long Run: [MBID, tracks, status]
- [Other regressions]

**Decision:**
- [ ] GO: Proceed to next experiment
- [ ] NO-GO: Revert changes
- [ ] PIVOT: Try alternative approach

**Next Steps:**
- [What to do next]

---

## Progress Log

### 2025-12-31: Initial Roadmap Created
- Baseline established: 7 regressions, 81 improvements
- Root cause identified: -1.0 negative floor
- Roadmap defined: 5 phases, 15+ experiments

### [Date]: Experiment 1A - No Negative Floor
- Status: PENDING
- [Results to be filled]

---

## Decision Tree Summary

```
START
  ↓
Phase 1: Quality Floor
  ├─ 1A (0.0 floor) → SUCCESS? → 1B (-0.3 floor) → SELECT BEST → PHASE 2
  └─ 1A (0.0 floor) → FAILURE? → Experiment 2 (revert to run29f)
  ↓
Phase 2: Refinement Timing
  ├─ 2A (before selection) → SUCCESS? → 2B (iterative)
  └─ 2A (before selection) → FAILURE? → KEEP CURRENT
  ↓
Phase 3: Scoring Algorithm
  ├─ 3A (pure match %) → 0 REGRESSIONS? → CONSIDER ADOPTION
  ├─ 3B (hybrid) → BETTER THAN 3A? → ADOPT
  └─ 3C (weighted) → BEST OF ALL? → ADOPT
  ↓
Phase 4: Parameter Tuning (optimize locked algorithm)
  ↓
Phase 5: Novel Algorithms (explore innovations)
  ↓
END: Optimal Configuration Documented
```

---

## Notes

### Why Incremental Testing?
- Complex interactions between components
- Each change affects downstream results
- Need clear cause-effect understanding
- Avoid "shotgun debugging"

### Why 200-File Testing?
- Statistical significance
- Edge case coverage
- Confidence in changes
- Reproducible results

### Cost-Benefit Analysis
- Each test: ~30 min (build + run + analyze)
- 15 experiments = ~7.5 hours
- Value: Eliminate regressions, improve accuracy
- Trade-off: Time investment for quality

---

## Final Configuration (TBD)

**To be filled after experiments complete:**

```rust
// Quality Score
const MIN_QUALITY_FLOOR: f64 = [TBD];

// Scoring Weights (if kept)
const DURATION_WEIGHT: f64 = [TBD];
const QUALITY_WEIGHT: f64 = [TBD];
const NAME_WEIGHT: f64 = [TBD];

// Tolerance
const MATCH_TOLERANCE_SECS: f64 = [TBD];

// Algorithm
[Description of final scoring approach]
```

**Metrics:**
- Exact matches: [TBD]
- Clearly better: [TBD]
- Equivocal: [TBD]
- Clearly worse: [TBD] (TARGET: 0)

**Improvement vs. run29f:**
- Net improvement: [TBD]% of 200 albums

---

## References

- Baseline comparison: `am/run29f_comparison_results.json`
- Regression analysis: `am/run29f_regression_analysis.md`
- run29f source: `wkmp-ai/examples/am29/`
- Current source: `wkmp-ai/src/matching/`

# TC-S-060-03: Parameter Tuning - Track Count Tolerance

**Test ID:** TC-S-060-03
**Test Type:** System Test (Parameter Optimization)
**Requirement:** REQ-EF-060 (Zero Regression Validation)
**Priority:** Critical
**Estimated Effort:** 3-4 hours (5 test runs + analysis)

---

## Test Objective

Determine optimal track count tolerance value (±N tracks) that achieves:
1. **Primary Goal:** Zero regressions on 186 currently-passing albums
2. **Secondary Goal:** Maximum problem album fixes (specifically Imagine Dragons, Michael Jackson)

---

## Test Specification

**Scope:** Systematic testing of track count tolerance values to find zero-regression configuration

**Approach:** Sequential testing from conservative to aggressive

**Parameter Space:**
- Track count tolerance: [±2, ±3, ±4, ±5, ±6]
- Total tests: 5

**Optimization Strategy:**
- Start with ±3 (proposed value from analysis)
- If ±3 has regressions, test ±4, ±5 (more conservative)
- If ±3 has zero regressions, optionally test ±2 (more aggressive)
- Stop when zero-regression configuration found

---

## Test Execution Procedure

### Step 1: Implement Configurable Tolerance (15 minutes)

Modify `edition_filter.rs` to use configurable constant:

```rust
// At top of edition_filter.rs
pub const TRACK_COUNT_TOLERANCE: i32 = 3;  // Tunable

pub fn filter_by_track_count(editions: &[Edition], detected: usize) -> Vec<Edition> {
    editions.iter()
        .filter(|e| {
            let diff = (e.track_count as i32 - detected as i32).abs();
            diff <= TRACK_COUNT_TOLERANCE
        })
        .cloned()
        .collect()
}
```

### Step 2: Create Test Script (30 minutes)

```bash
#!/bin/bash
# tune_track_count_tolerance.sh

BASELINE="baseline_1-12-26_results.json"
OUTPUT_DIR="track_count_tuning_results"
mkdir -p "$OUTPUT_DIR"

# Test values in order: proposed, then more conservative, then more aggressive
TOLERANCE_VALUES=(3 4 5 2 6)

for tolerance in "${TOLERANCE_VALUES[@]}"; do
    echo "Testing: Track count tolerance = ±$tolerance"

    # Update constant in edition_filter.rs
    sed -i "s/TRACK_COUNT_TOLERANCE: i32 = .*/TRACK_COUNT_TOLERANCE: i32 = $tolerance;/" \
        wkmp-ai/src/matching/edition_filter.rs

    # Rebuild
    cargo build --release -p wkmp-ai

    # Run test suite (use subset for speed: 50 albums)
    RUST_LOG=wkmp_ai=debug cargo test --release test_tolerance_tuning_subset \
        -- --nocapture > "$OUTPUT_DIR/test_tolerance_$tolerance.log" 2>&1

    # Extract results
    python scripts/extract_results.py \
        "$OUTPUT_DIR/test_tolerance_$tolerance.log" \
        "$OUTPUT_DIR/results_tolerance_$tolerance.json"

    # Quick comparison
    python scripts/quick_comparison.py \
        "$BASELINE" \
        "$OUTPUT_DIR/results_tolerance_$tolerance.json" \
        "$OUTPUT_DIR/summary_tolerance_$tolerance.txt"

    # Check results
    regressions=$(grep 'Regressions:' "$OUTPUT_DIR/summary_tolerance_$tolerance.txt" | awk '{print $2}')
    fixes=$(grep 'Problem albums fixed:' "$OUTPUT_DIR/summary_tolerance_$tolerance.txt" | awk '{print $4}' | cut -d'/' -f1)

    echo "  Regressions: $regressions"
    echo "  Problem albums fixed: $fixes/4"

    if [ "$regressions" == "0" ]; then
        echo "✓ Zero regressions found with ±$tolerance tolerance"
        echo "$tolerance $fixes" >> "$OUTPUT_DIR/zero_regression_candidates.txt"

        # If this is first zero-regression configuration AND it fixes ≥2 problem albums, stop
        if [ "$fixes" -ge 2 ]; then
            echo "Stopping early: Zero regressions + ≥2 fixes achieved"
            break
        fi
    else
        echo "✗ Regressions detected: $regressions albums"
    fi
done

# Generate final report
python scripts/analyze_tolerance_tuning.py "$OUTPUT_DIR" > tolerance_tuning_report.md
```

### Step 3: Execute Sequential Search (2-3 hours)

**Test Sequence:**
1. ±3 (proposed value)
2. If ±3 fails → ±4 (more conservative)
3. If ±4 fails → ±5 (more conservative)
4. If ±3 succeeds → optionally ±2 (more aggressive, for comparison)

**Early Stopping:**
- If any tolerance achieves 0 regressions + ≥2 problem fixes → stop, that's the winner

**Estimated Runtime:**
- Subset (50 albums): ~10 minutes per test
- Likely outcome: Stop after testing ±3 (expected to work)
- Total: 1-3 tests × 10 minutes = 10-30 minutes (subset)

### Step 4: Full Validation (80 minutes)

**Validate Winner on Full Suite:**
```bash
tolerance=$(cat zero_regression_candidates.txt | head -1 | awk '{print $1}')

echo "Final validation: ±$tolerance on full 200-album suite"

# Set tolerance
sed -i "s/TRACK_COUNT_TOLERANCE: i32 = .*/TRACK_COUNT_TOLERANCE: i32 = $tolerance;/" \
    wkmp-ai/src/matching/edition_filter.rs

# Rebuild
cargo build --release -p wkmp-ai

# Run full test
RUST_LOG=wkmp_ai=debug cargo test --release test_run29f_full \
    -- --nocapture > full_validation_tolerance_$tolerance.log 2>&1

# Verify zero regressions on full 200 albums
python scripts/compare_results.py \
    baseline_1-12-26_results.json \
    full_validation_tolerance_${tolerance}_results.json \
    > final_tolerance_report.md
```

---

## Pass Criteria

### Success Conditions

✅ **Criterion 1: Zero-regression tolerance found**
- At least one tolerance value achieves 0 regressions (subset)
- Full validation confirms 0 regressions (200 albums)

✅ **Criterion 2: Problem albums benefit from filtering**
- Tolerance fixes ≥2 problem albums (Imagine Dragons + Michael Jackson minimum)
- Tolerance = ∞ (no filtering) would fix 0 albums, so any improvement is good

✅ **Criterion 3: Optimal tolerance selected**
- Among zero-regression candidates, choose:
  - Maximum problem album fixes (primary)
  - Most restrictive tolerance (secondary, tie-breaker for performance)

### Failure Conditions

❌ **All tolerance values cause regressions**
- Even ±6 causes regressions
- **Mitigation:** Test larger values (±7, ±8) or disable track count filtering

❌ **Zero-regression tolerance fixes zero problem albums**
- E.g., ±6 achieves zero regressions but Imagine Dragons still selects 16-track deluxe
- **Decision:** Track count filtering ineffective, investigate alternative approaches

---

## Expected Output

### tolerance_tuning_report.md Structure

```markdown
# Track Count Tolerance Tuning Report

**Date:** [timestamp]
**Test Sequence:** ±3 → [±4 → ±5] or [±2]
**Subset:** 50 albums (4 problem + 46 baseline)
**Full Validation:** Winner on 200 albums

## Sequential Search Results

| Tolerance | Regressions (Subset) | Problem Fixes | Albums Filtered (Avg) | Status |
|-----------|----------------------|---------------|----------------------|--------|
| ±2 | 3 | 3/4 | 40% | ✗ Too strict |
| ±3 | 0 | 2/4 | 25% | ✅ CANDIDATE |
| ±4 | 0 | 2/4 | 15% | Candidate (less restrictive) |
| ±5 | 0 | 1/4 | 10% | Too loose |

**Winner (Subset):** ±3 (zero regressions, 2/4 fixes, good filtering rate)

## Full Validation Results

| Tolerance | Regressions (200) | Problem Fixes | Performance Impact |
|-----------|-------------------|---------------|--------------------|
| ±3 | 0 | 2/4 | -20% candidate editions |

**Winner (Full):** ±3 confirmed

## Recommended Configuration

**Selected Tolerance:** `TRACK_COUNT_TOLERANCE = 3`

**Justification:**
- Zero regressions on full 200-album test
- Fixes 2 problem albums:
  - Imagine Dragons - Night Visions: Filters 16-track deluxe, selects 11-track standard ✓
  - Michael Jackson - Thriller: Filters compilation editions, selects 9-track standard ✓
- Performance benefit: ~20% fewer editions passed to Stage 2 (faster matching)

**Problem Albums Status:**
- Imagine Dragons: FIXED (track count filter)
- Michael Jackson: FIXED (track count filter + compilation penalty)
- James Gang: Not fixed by track count (requires artist consistency validation)
- Chemical Brothers: N/A (track count not the issue)

**Trade-offs Evaluated:**
- ±2 (stricter): 3 regressions → rejected
- ±4 (looser): Same fixes, less filtering → ±3 preferred for performance
- ±5 (too loose): Fewer fixes → rejected

## Implementation

Update `wkmp-ai/src/matching/edition_filter.rs`:
```rust
pub const TRACK_COUNT_TOLERANCE: i32 = 3;
```

## Next Steps

1. ✅ Track count tolerance tuned and validated
2. ✅ Penalty multipliers tuned (TC-S-060-02)
3. ➡️ Run TC-S-060-01 (full regression suite with all parameters tuned)
```

---

## Test Data

**Input:**
- Baseline: baseline_1-12-26_results.json
- Subset: 50 albums (must include Imagine Dragons, Michael Jackson)
- Full: 200 albums

**Parameter Range:** [±2, ±3, ±4, ±5, ±6]

**Expected Output:**
- 3-5 test runs
- 1 optimal tolerance value (likely ±3)

---

## Problem Album Context

### Imagine Dragons - Night Visions

**Issue:** 11 tracks in file, matched to 16-track deluxe edition

**MusicBrainz Editions:**
- Standard: 11 tracks (difference = 0, within any tolerance) ✓
- Deluxe: 16 tracks (difference = 5)
  - Tolerance ±2: FILTERED
  - Tolerance ±3: FILTERED
  - Tolerance ±4: FILTERED
  - Tolerance ±5: PASS
  - Tolerance ±6: PASS

**Expected:** ±3 filters deluxe, allows standard → FIXED

### Michael Jackson - Thriller

**Issue:** Standard 9-track album matched to 19-25 track compilation

**MusicBrainz Editions:**
- Standard: 9 tracks (difference = 0) ✓
- Compilation: ~20 tracks (difference = 11)
  - Tolerance ±2: FILTERED
  - Tolerance ±3: FILTERED
  - Tolerance ±4: FILTERED
  - Tolerance ±5: FILTERED
  - Tolerance ±10: PASS

**Expected:** All tested tolerances filter compilation → FIXED

---

## Decision Logic

```
tolerance_candidates = [3, 4, 5, 2, 6]  # Test order: proposed first, then expand

FOR tolerance IN tolerance_candidates:
    Run 50-album subset test
    Extract: regressions, problem_fixes

    IF regressions == 0:
        Record as candidate
        IF problem_fixes >= 2:
            WINNER = tolerance
            BREAK  # Early stop
    ELSE:
        Continue to next tolerance

IF no winner found:
    WINNER = most conservative candidate with zero regressions

Run full 200-album validation on WINNER
Confirm zero regressions

RETURN WINNER
```

---

## Dependencies

**Code Dependencies:**
- Track count filter with configurable tolerance
- Penalty multipliers already tuned (TC-S-060-02) or set to baseline (1.0, 1.0)

**Execution Order:**
- Can run independently of TC-S-060-02 (orthogonal parameters)
- Should complete before TC-S-060-01 (full regression suite needs tuned parameters)

**Time Dependencies:**
- Subset tests: 10 minutes each × 3-5 runs = 30-50 minutes
- Full validation: 80 minutes
- Total: 2-3 hours

---

## Notes

**Expected Outcome:**
±3 tolerance likely optimal based on problem album analysis:
- Filters Imagine Dragons deluxe (diff=5)
- Filters Michael Jackson compilation (diff=11)
- Unlikely to filter valid editions (most editions within ±3 tracks of actual)

**Regression Risk:**
Low. Track count filtering is conservative:
- Only filters editions with >3 track difference
- Falls back to unfiltered list if all filtered
- Unlikely to reject correct editions for well-matched albums

**Performance Benefit:**
Track count filtering reduces candidate editions by ~20-30%, improving Stage 2 performance.

**Related Tests:**
- TC-S-060-01: Uses tolerance from this test for full regression suite
- TC-S-060-02: Tunes penalty multipliers (independent parameter)
- TC-S-030-01: Imagine Dragons problem album test (validates fix)

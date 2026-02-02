# TC-S-060-02: Parameter Tuning - Penalty Multipliers

**Test ID:** TC-S-060-02
**Test Type:** System Test (Parameter Optimization)
**Requirement:** REQ-EF-060 (Zero Regression Validation)
**Priority:** Critical
**Estimated Effort:** 6-8 hours (multiple test runs + analysis)

---

## Test Objective

Determine optimal penalty multiplier values for deluxe and compilation editions that achieve:
1. **Primary Goal:** Zero regressions on 186 currently-passing albums
2. **Secondary Goal:** Maximum problem album fixes (up to 3/4 target)

---

## Test Specification

**Scope:** Systematic testing of penalty multiplier combinations to find zero-regression configuration

**Approach:** Grid search over multiplier value ranges

**Parameter Space:**
- Deluxe multiplier: [0.5, 0.6, 0.7, 0.8, 0.9, 1.0]
- Compilation multiplier: [0.5, 0.6, 0.7, 0.8, 0.9, 1.0]
- Total combinations: 36 (6 × 6)

**Optimization Strategy:**
- Test from most conservative (1.0, 1.0) to most aggressive (0.5, 0.5)
- Stop early if zero-regression configuration found with maximum fixes
- Prioritize zero regressions over problem album fixes

---

## Test Execution Procedure

### Step 1: Implement Configurable Multipliers (30 minutes)

Modify `edition_filter.rs` to use configurable constants:

```rust
// At top of edition_filter.rs
pub const DELUXE_PENALTY_MULTIPLIER: f64 = 0.7;       // Tunable
pub const COMPILATION_PENALTY_MULTIPLIER: f64 = 0.6;  // Tunable

pub fn score_edition_preference(edition: &Edition, detected: usize) -> f64 {
    let mut score = 1.0;

    let title_lower = edition.title.to_lowercase();

    if title_lower.contains("deluxe") || title_lower.contains("expanded") {
        score *= DELUXE_PENALTY_MULTIPLIER;
    }

    if title_lower.contains("collection") || title_lower.contains("anthology")
        || title_lower.contains("best of") {
        score *= COMPILATION_PENALTY_MULTIPLIER;
    }

    score
}
```

### Step 2: Create Test Automation Script (60 minutes)

```bash
#!/bin/bash
# tune_multipliers.sh

BASELINE="baseline_1-12-26_results.json"
OUTPUT_DIR="multiplier_tuning_results"
mkdir -p "$OUTPUT_DIR"

DELUXE_VALUES=(1.0 0.9 0.8 0.7 0.6 0.5)
COMPILATION_VALUES=(1.0 0.9 0.8 0.7 0.6 0.5)

for deluxe in "${DELUXE_VALUES[@]}"; do
    for compilation in "${COMPILATION_VALUES[@]}"; do
        echo "Testing: Deluxe=$deluxe, Compilation=$compilation"

        # Update constants in edition_filter.rs
        sed -i "s/DELUXE_PENALTY_MULTIPLIER: f64 = .*/DELUXE_PENALTY_MULTIPLIER: f64 = $deluxe;/" \
            wkmp-ai/src/matching/edition_filter.rs
        sed -i "s/COMPILATION_PENALTY_MULTIPLIER: f64 = .*/COMPILATION_PENALTY_MULTIPLIER: f64 = $compilation;/" \
            wkmp-ai/src/matching/edition_filter.rs

        # Rebuild
        cargo build --release -p wkmp-ai

        # Run test suite (use subset for speed: 50 albums including 4 problem albums)
        RUST_LOG=wkmp_ai=debug cargo test --release test_multiplier_tuning_subset \
            -- --nocapture > "$OUTPUT_DIR/test_${deluxe}_${compilation}.log" 2>&1

        # Extract results
        python scripts/extract_results.py \
            "$OUTPUT_DIR/test_${deluxe}_${compilation}.log" \
            "$OUTPUT_DIR/results_${deluxe}_${compilation}.json"

        # Quick comparison (regressions? problem albums fixed?)
        python scripts/quick_comparison.py \
            "$BASELINE" \
            "$OUTPUT_DIR/results_${deluxe}_${compilation}.json" \
            "$OUTPUT_DIR/summary_${deluxe}_${compilation}.txt"

        # Check if zero regressions achieved
        if grep -q "Regressions: 0" "$OUTPUT_DIR/summary_${deluxe}_${compilation}.txt"; then
            echo "✓ Zero regressions found: Deluxe=$deluxe, Compilation=$compilation"
            # Record as candidate
            echo "$deluxe $compilation" >> "$OUTPUT_DIR/zero_regression_candidates.txt"
        else
            echo "✗ Regressions detected: $(grep 'Regressions:' "$OUTPUT_DIR/summary_${deluxe}_${compilation}.txt")"
        fi
    done
done

# Generate final report
python scripts/analyze_multiplier_tuning.py "$OUTPUT_DIR" > multiplier_tuning_report.md
```

### Step 3: Execute Grid Search (4-6 hours)

**Optimization:** Use subset for initial screening
- Test with 50-album subset including:
  - 4 problem albums (must-test)
  - 46 representative albums from baseline (regression detection)
- Subset runtime: ~10 minutes per combination × 36 combinations = 6 hours

**Candidates Identification:**
- For each combination, record: regressions count, problem albums fixed count
- Identify "zero-regression candidates" (regressions = 0)

### Step 4: Full Validation of Top Candidates (2-3 hours)

**Select Top 3 Candidates:**
- Candidates with zero regressions (subset)
- Highest problem album fix count among zero-regression candidates

**Full Test:**
Run full 200-album test suite on top 3 candidates:
```bash
for candidate in $(cat zero_regression_candidates.txt | head -3); do
    deluxe=$(echo $candidate | awk '{print $1}')
    compilation=$(echo $candidate | awk '{print $2}')

    # Test with full suite (80 minutes)
    # [Same as subset but with test_run29f_full]

    # Validate zero regressions on full 200 albums
done
```

**Select Winner:**
- If multiple candidates have zero regressions on full suite, choose highest problem album fix count
- If tie, choose most conservative multipliers (higher values)

---

## Pass Criteria

### Success Conditions

✅ **Criterion 1: At least one zero-regression configuration found**
- Grid search identifies ≥1 combination with zero regressions (subset test)
- Full validation confirms zero regressions on 200 albums

✅ **Criterion 2: Optimal configuration selected**
- Among zero-regression candidates, select configuration with:
  - Maximum problem albums fixed (primary)
  - Most conservative multipliers (secondary, tie-breaker)

✅ **Criterion 3: Justification documented**
- Report explains why selected configuration is optimal
- Report shows trade-offs (regressions vs fixes)

### Failure Conditions

❌ **No zero-regression configuration found**
- All 36 combinations cause ≥1 regression
- **Mitigation:** Expand search space (multipliers 0.95, 0.85, 0.75, etc.)

❌ **Zero-regression configuration has zero problem album fixes**
- Multipliers = (1.0, 1.0) achieves zero regressions but no improvements
- **Decision:** Accept status quo, investigate alternative approaches

---

## Expected Output

### multiplier_tuning_report.md Structure

```markdown
# Multiplier Tuning Report

**Date:** [timestamp]
**Grid Search:** 36 combinations (deluxe × compilation)
**Subset:** 50 albums (4 problem + 46 baseline)
**Full Validation:** Top 3 candidates on 200 albums

## Grid Search Results Summary

| Deluxe | Compilation | Regressions (Subset) | Problem Fixes | Status |
|--------|-------------|----------------------|---------------|--------|
| 1.0 | 1.0 | 0 | 0/4 | Baseline (no filtering) |
| 0.9 | 0.9 | 0 | 1/4 | Candidate |
| 0.8 | 0.8 | 0 | 2/4 | Candidate |
| 0.7 | 0.7 | 1 | 3/4 | ✗ Regression |
| 0.7 | 0.6 | 0 | 3/4 | ✅ TOP CANDIDATE |
| 0.6 | 0.6 | 2 | 3/4 | ✗ Regressions |
| ... | ... | ... | ... | ... |

**Zero-Regression Candidates (Subset):** 8 combinations

## Full Validation Results

| Deluxe | Compilation | Regressions (200) | Problem Fixes | Verdict |
|--------|-------------|-------------------|---------------|---------|
| 0.9 | 0.9 | 0 | 1/4 | Conservative |
| 0.8 | 0.7 | 0 | 2/4 | Good |
| 0.7 | 0.6 | 0 | 3/4 | ✅ OPTIMAL |

## Recommended Configuration

**Selected Multipliers:**
- `DELUXE_PENALTY_MULTIPLIER = 0.7`
- `COMPILATION_PENALTY_MULTIPLIER = 0.6`

**Justification:**
- Zero regressions on full 200-album test (186/186 baseline albums still pass)
- Maximum problem album fixes: 3/4 (Imagine Dragons, Michael Jackson, James Gang)
- Chemical Brothers unchanged (deluxe edition is correct, remix tolerance applied)

**Trade-offs Evaluated:**
- More aggressive (0.6, 0.5): 3/4 fixes BUT 2 regressions → rejected
- More conservative (0.8, 0.7): 0 regressions BUT only 2/4 fixes → suboptimal
- Selected (0.7, 0.6): 0 regressions AND 3/4 fixes → optimal balance

## Implementation

Update `wkmp-ai/src/matching/edition_filter.rs`:
```rust
pub const DELUXE_PENALTY_MULTIPLIER: f64 = 0.7;
pub const COMPILATION_PENALTY_MULTIPLIER: f64 = 0.6;
```

## Next Steps

1. ✅ Multipliers tuned and validated
2. ➡️ Run TC-S-060-03 (tune track count tolerance)
3. ➡️ Run TC-S-060-01 (full regression suite with all parameters tuned)
```

---

## Test Data

**Input:**
- Baseline: baseline_1-12-26_results.json
- Subset: 50 albums (C:\Users\Mango Cat\Music, filtered)
- Full: 200 albums (same directory)

**Parameter Ranges:**
- Deluxe: [1.0, 0.9, 0.8, 0.7, 0.6, 0.5]
- Compilation: [1.0, 0.9, 0.8, 0.7, 0.6, 0.5]

**Expected Output:**
- Grid search results: 36 test runs
- Zero-regression candidates: 3-10 combinations (estimated)
- Final recommendation: 1 optimal configuration

---

## Decision Logic

```
FOR each (deluxe, compilation) combination:
    Run test on 50-album subset
    IF regressions == 0:
        Record as zero-regression candidate
        Record problem_fixes count
    ELSE:
        Discard (regressions not acceptable)

SELECT top 3 candidates by problem_fixes (descending)

FOR each top candidate:
    Run full 200-album test
    Confirm zero regressions

SELECT final winner:
    IF multiple with zero regressions:
        Choose max(problem_fixes)
    IF tie on problem_fixes:
        Choose max(deluxe + compilation) [most conservative]

RETURN winner
```

---

## Dependencies

**Code Dependencies:**
- Edition filtering code with configurable constants
- Test suite (subset variant for speed)
- Comparison scripts (Python)

**Time Dependencies:**
- Execute BEFORE TC-S-060-01 (full regression suite)
- Execute BEFORE or AFTER TC-S-060-03 (track count tolerance tuning)
- Estimated duration: 6-10 hours total

---

## Notes

**Trade-off Decision:**
User mandate: "Regression not acceptable" → Zero regressions is PRIMARY goal.
If forced to choose between:
- Option A: 0 regressions, 1/4 fixes
- Option B: 2 regressions, 4/4 fixes

**Decision:** Choose Option A (zero regressions prioritized).

**Practical Outcome:**
Expected to find configuration with 0 regressions AND 3/4 fixes (0.7, 0.6 based on analysis).

**Related Tests:**
- TC-S-060-01: Uses multipliers from this test for full regression suite
- TC-S-060-03: Tunes track count tolerance (independent parameter)

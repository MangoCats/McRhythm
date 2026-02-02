# TC-S-060-01: Full Regression Suite (200 Albums)

**Test ID:** TC-S-060-01
**Test Type:** System Test (End-to-End Regression)
**Requirement:** REQ-EF-060 (Zero Regression Validation)
**Priority:** Critical
**Estimated Effort:** 4-6 hours (execute + analyze + report)

---

## Test Objective

Verify that edition filtering improvements cause ZERO regressions on currently-passing albums. Baseline: 186/200 albums pass (93% match rate). After changes: ≥186/200 must pass.

---

## Test Specification

**Scope:** Complete end-to-end system test of entire matching pipeline with edition filtering enabled

**Environment:**
- Full test library: C:\Users\Mango Cat\Music (200 albums)
- Test configuration: Same as baseline (run29f_full)
- Database: Clean .cache/run29f_regression_test.db
- Logging: RUST_LOG=wkmp_ai=debug

**Given:**
- Baseline test results: baseline_1-12-26_stats.json
  - 186/200 albums matched (93.0%)
  - 0 failed, 14 skipped
  - Mean error: varies by album
  - MBID selections: 186 known-good MBIDs
- Edition filtering code implemented with tuned parameters:
  - Deluxe multiplier: [value from TC-S-060-02]
  - Compilation multiplier: [value from TC-S-060-02]
  - Track count tolerance: [value from TC-S-060-03]

**When:**
- Execute full test suite with edition filtering enabled
- Command: `RUST_LOG=wkmp_ai=debug cargo test --release test_run29f_full -- --nocapture > regression_test_output.log 2>&1`
- Duration: ~80-90 minutes (similar to baseline)

**Then:**
- Test completes successfully
- Results captured in regression_test_output.log
- Comparison script generates regression report

**Verify:**
- **Albums matched:** ≥186/200 (no decrease from baseline)
- **Match rate:** ≥93.0% (no decrease from baseline)
- **Track duration errors:** No significant increase in mean/median/p95 error
- **MBID stability:** ≥90% of albums select same MBID as baseline
- **Problem albums fixed:** 3/4 expected fixes (Imagine Dragons, Michael Jackson, James Gang)
- **Zero regressions:** No currently-passing album fails after changes

---

## Pass Criteria

### Primary Criteria (MUST Meet ALL)

✅ **Criterion 1: No Decrease in Pass Rate**
- Baseline: 186/200 albums matched
- Regression test: ≥186/200 albums matched
- **CRITICAL:** If < 186, test FAILS (regression detected)

✅ **Criterion 2: No Increase in Track Errors**
- Compare error distribution (mean, median, p95, p99) per album
- Allowed: Individual album errors may shift slightly (±2s)
- **CRITICAL:** If overall error distribution worsens (p95 increases >5s), test FAILS

✅ **Criterion 3: MBID Stability**
- Count albums where MBID changed from baseline
- **Allowed:** Up to 10% MBID churn (≤20 albums select different MBID)
- **CRITICAL:** If >20 albums change MBID, investigate and justify

✅ **Criterion 4: Problem Albums Fixed**
- Imagine Dragons - Night Visions: Match 11-track standard (not 16-track deluxe)
- Michael Jackson - Thriller: Match 9-track standard (not compilation)
- James Gang - Funk #49: Reject multi-artist compilation
- **Target:** 3/4 fixed (75% improvement)
- **Acceptable:** 2/4 fixed (50% improvement) if zero regressions maintained

### Secondary Criteria (Desirable)

- Performance: Test completes in ≤90 minutes (baseline: 80.9 minutes)
- Logging: DEBUG logs provide clear filtering decisions
- Code quality: No compiler warnings, no clippy warnings

---

## Test Execution Procedure

### Step 1: Prepare Environment (15 minutes)

```bash
# Clean database
rm -f .cache/run29f_regression_test.db

# Verify baseline exists
ls -l wkmp-ai/baseline_1-12-26_stats.json

# Verify test library accessible
ls "C:\Users\Mango Cat\Music" | wc -l  # Should show ~200 entries

# Build release binary
cargo build --release -p wkmp-ai
```

### Step 2: Execute Test Suite (80-90 minutes)

```bash
# Run full test suite with debug logging
RUST_LOG=wkmp_ai=debug cargo test --release test_run29f_full -- --nocapture > regression_test_output.log 2>&1

# Monitor progress
tail -f regression_test_output.log
```

### Step 3: Extract Results (5 minutes)

```bash
# Extract structured results
python scripts/extract_test_results.py regression_test_output.log > regression_results.json

# Generate comparison report
python scripts/compare_results.py baseline_1-12-26_results.json regression_results.json > regression_report.md
```

### Step 4: Analyze Results (30-60 minutes)

**Review regression_report.md for:**
1. Pass rate comparison (186 baseline vs current)
2. Albums that regressed (passed before, fail now)
3. Albums that improved (failed before, pass now)
4. MBID changes (same/different from baseline)
5. Error distribution changes (mean, median, p95, p99)
6. Problem album status (fixed yes/no)

**Acceptance Decision:**
- If all Primary Criteria met → ✅ PASS
- If any Primary Criterion failed → ❌ FAIL (tune parameters, re-test)

### Step 5: Document Results (15 minutes)

Update regression_report.md with:
- Final pass/fail verdict
- Detailed regression analysis
- Problem album status
- Recommendations (if failed: which parameters to adjust)

---

## Expected Output Format

### regression_report.md Structure

```markdown
# Regression Test Report - Edition Filtering Improvements

**Date:** [timestamp]
**Baseline:** baseline_1-12-26_stats.json
**Test:** regression_results.json

## Summary

| Metric | Baseline | Current | Change | Status |
|--------|----------|---------|--------|--------|
| Albums Matched | 186/200 (93.0%) | [X]/200 ([Y]%) | [+/-Z] | [PASS/FAIL] |
| Mean Error | [X]s | [Y]s | [+/-Z]s | [PASS/FAIL] |
| P95 Error | [X]s | [Y]s | [+/-Z]s | [PASS/FAIL] |
| MBID Changes | N/A | [X]/186 ([Y]%) | N/A | [PASS/FAIL] |
| Problem Albums Fixed | 0/4 | [X]/4 ([Y]%) | +[X] | [PASS/FAIL] |

## Primary Criteria

✅/❌ Criterion 1: No decrease in pass rate ([X]/200 vs 186/200)
✅/❌ Criterion 2: No increase in track errors (p95: [X]s vs baseline [Y]s)
✅/❌ Criterion 3: MBID stability ([X]/186 changed, [Y]% churn)
✅/❌ Criterion 4: Problem albums fixed ([X]/4 fixed)

## Regressions Detected

[If any:]

### Album: [Name]
- Baseline: PASS (MBID: [xxx], Match: [Y]%)
- Current: FAIL ([reason])
- Root cause: [analysis]
- Recommendation: [parameter adjustment]

[If none:]
✅ Zero regressions detected. All 186 baseline albums still pass.

## Problem Albums Status

### Imagine Dragons - Night Visions
- Baseline: 16-track deluxe (wrong)
- Current: [11-track standard / still deluxe]
- Status: [FIXED / NOT FIXED]

[Repeat for other 3 problem albums]

## MBID Changes

| Album | Baseline MBID | Current MBID | Reason | Acceptable? |
|-------|---------------|--------------|--------|-------------|
| [Name] | [xxx] | [yyy] | [Better match / Edition filtering] | [Yes/No] |

## Recommendations

[If PASS:]
✅ All criteria met. Ready for production deployment.

[If FAIL:]
❌ Criteria failed: [list]
Recommendations:
1. Adjust [parameter]: [suggested value]
2. Re-run test with adjusted parameters
3. Review [specific albums] for root cause
```

---

## Test Data

**Input:**
- Baseline: baseline_1-12-26_stats.json (known-good results)
- Test library: 200 albums (C:\Users\Mango Cat\Music)
- Edition filtering parameters: Values from TC-S-060-02 and TC-S-060-03

**Expected Output:**
- Regression report showing ≥186/200 albums pass
- 0 regressions detected
- 3/4 problem albums fixed
- ≥90% MBID stability

---

## Failure Modes and Mitigations

### Failure Mode 1: Regressions Detected

**Symptoms:**
- < 186 albums pass (e.g., 183/200)
- 3 currently-passing albums now fail

**Root Causes:**
- Penalty multipliers too aggressive (rejected valid editions)
- Track count tolerance too strict (filtered correct editions)

**Mitigation:**
1. Identify which albums regressed
2. Analyze filtering logs for those albums
3. Determine if deluxe/compilation penalty or track count filter caused regression
4. Adjust parameters (reduce penalties, increase tolerance)
5. Re-run TC-S-060-02 or TC-S-060-03 with new parameter range
6. Re-run full regression test

### Failure Mode 2: Problem Albums Not Fixed

**Symptoms:**
- 186/200 still pass (no regressions)
- BUT only 1/4 or 2/4 problem albums fixed (not 3/4 target)

**Decision:**
- If 0 regressions: Accept current parameters, investigate unfixed albums separately
- If fixes < 50% (< 2/4): Consider more aggressive parameters, re-test

**Mitigation:**
1. Review logs for unfixed problem albums
2. Determine why filtering didn't work (unexpected edition titles, track counts, etc.)
3. Update requirements if needed (add keywords, adjust thresholds)
4. Re-test

### Failure Mode 3: Excessive MBID Churn

**Symptoms:**
- >20 albums select different MBID than baseline
- Many "better match" justifications

**Analysis:**
- Is churn due to filtering? (Good - selected better editions)
- Or due to side effects? (Bad - unstable matching)

**Decision:**
- Review changed MBIDs: Are new matches actually better?
- If better: Accept churn as improvement
- If worse: Investigate and adjust parameters

---

## Dependencies

**Code Dependencies:**
- All edition filtering code implemented (REQ-EF-010 through REQ-EF-050)
- Parameters tuned via TC-S-060-02 and TC-S-060-03
- Logging instrumentation in place

**Data Dependencies:**
- Baseline results: baseline_1-12-26_stats.json
- Test library accessible: C:\Users\Mango Cat\Music
- Comparison scripts: extract_test_results.py, compare_results.py

**Time Dependencies:**
- Execute AFTER TC-S-060-02 and TC-S-060-03 complete (parameters tuned)
- Allow 80-90 minutes for test execution
- Allow 30-60 minutes for analysis

---

## Notes

**Critical Constraint:** User explicitly stated "regression in other albums is not acceptable". This test MUST pass for implementation to be accepted.

**Decision Authority:** If test fails, implementation team must:
1. Adjust parameters to achieve zero regressions
2. Accept fewer problem album fixes if necessary
3. Do NOT deploy if regressions exist

**Success Metric Priority:**
1. Zero regressions (primary, non-negotiable)
2. Problem albums fixed (secondary, best-effort)

**Related Tests:**
- TC-S-060-02: Parameter tuning for penalty multipliers
- TC-S-060-03: Parameter tuning for track count tolerance
- TC-S-030-01: Imagine Dragons problem album test

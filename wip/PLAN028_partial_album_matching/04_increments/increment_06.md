# Increment 6: Full 200-Album Regression Test

**Objective:** Verify no regressions on existing test suite

---

## Deliverables

1. Run complete `run29f_full_comparison_test`

2. Compare results against baseline:
   - All 187 previously matched albums still match
   - No match percentage drops >5%
   - Fluke/Puppy.mp3 now matches (improvement)

3. Generate regression report

---

## Files to Verify

- `wkmp-ai/tests/run29f_full_comparison_test.rs`
- `wkmp-ai/run29f_comparison_results.json` (baseline)

---

## Acceptance Tests

- TC-S-PAM-002: No regressions on 200-album test suite

---

## Test Command

```bash
cargo test -p wkmp-ai run29f_full_comparison_test -- --test-threads=1 --nocapture 2>&1 | tee test_plan028_regression.log
```

---

## Expected Output

```
Regression Test Results:
========================
Albums Tested: 200
Matched (baseline): 187
Matched (current): 188+ (Puppy.mp3 now matches)

REGRESSIONS (Category A): 0
WARNINGS (Category B): 0

NEW MATCHES (improvement): 1
  - Fluke/Puppy.mp3: 0% -> 100% (partial 8/11)

SUMMARY: PASS
```

---

## Success Criteria

- Zero Category A regressions
- Zero Category B warnings
- All 187 baseline albums maintain score
- Fluke/Puppy.mp3 now matches
- Test execution completes in <60 minutes

---

## Rollback Trigger

If ANY Category A regression detected:
1. Stop testing
2. Investigate root cause
3. Fix before merge
4. Re-run full regression

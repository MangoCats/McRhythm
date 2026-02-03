# TC-S-PAM-002: 200-Album Regression Test

**Test Type:** System Test
**Requirement:** REQ-PAM-007
**Priority:** P0

---

## Environment

- Full wkmp-ai test environment
- MusicBrainz API access (rate-limited)
- Complete 200-album test corpus
- Baseline results: `run29f_comparison_results.json`

---

## Scenario

### Pre-conditions

**Given:**
- All 200 test albums available in test corpus
- Baseline results recorded for 187+ matched albums
- Partial album matching feature enabled

### Test Execution

**When:** Full `run29f_full_comparison_test` is executed

**Then:**
1. All 200 albums processed
2. Each album compared against baseline
3. Regression detected if:
   - Previously matched album now fails to match
   - Match percentage decreases by >5%
   - Wrong edition selected (different MBID)

---

## Verification

| Metric | Baseline | Minimum Acceptable |
|--------|----------|-------------------|
| Matched albums | 187 | ≥187 (no decrease) |
| Average match % | ~95% | ≥95% |
| Edition accuracy | Current | Same or better |

### Per-Album Checks

For each album with `baseline_tracks > 0`:

```rust
assert!(current_percentage >= baseline_percentage - 5.0,
    "Regression: {} dropped from {}% to {}%",
    album_name, baseline_percentage, current_percentage);
```

---

## Pass Criteria

- Zero regressions on previously matched albums
- All 187 matched albums maintain ≥95% of baseline score
- No new false negatives (albums that should match but don't)
- Fluke/Puppy.mp3 now matches (NEW: was 0%, should be 100% of 8 tracks)

---

## Fail Criteria

- Any album drops >5% match percentage
- Any previously matched album fails to match
- Total matched albums decreases

---

## Regression Categories

**Category A - Critical (Test Fails):**
- Album matched before, fails now
- Match % drops >10%

**Category B - Warning (Investigate):**
- Match % drops 5-10%
- Different edition selected with lower score

**Category C - Acceptable:**
- Minor % variance (<5%)
- Different edition with equal or better score

---

## Test Output Format

```
Regression Test Results:
========================
Albums Tested: 200
Matched (baseline): 187
Matched (current): XXX

REGRESSIONS (Category A): N
  - Album 1: 95% -> 0% (FAILED)

WARNINGS (Category B): N
  - Album 2: 98% -> 91% (-7%)

NEW MATCHES (improvement): N
  - Fluke/Puppy.mp3: 0% -> 100% (partial 8/11)

SUMMARY: [PASS/FAIL]
```

---

## Estimated Effort

Test execution: 30-45 minutes (200 albums with rate limiting)
Analysis: 15 minutes

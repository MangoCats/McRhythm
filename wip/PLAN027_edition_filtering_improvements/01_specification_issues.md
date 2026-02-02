# Specification Issues - Edition Filtering Improvements

**Plan:** PLAN027
**Date:** 2026-01-16
**Analysis Method:** 7-step completeness verification per /plan Phase 2

---

## Executive Summary

**Total Issues Found:** 8 (1 Critical, 3 High, 3 Medium, 1 Low)

**Critical Issues (MUST resolve):** 1
- No explicit "zero regressions" requirement despite user mandate

**High Issues (Should resolve before implementation):** 3
- Penalty multipliers not validated (may cause regressions)
- Track count tolerance not validated (may cause regressions)
- Fallback behavior for "all editions filtered" not fully specified

**Medium Issues (Address during implementation):** 3
- Fuzzy artist matching algorithm not specified
- Remix keyword list incomplete
- Performance constraint (100ms) not validated

**Low Issues (Document/note):** 1
- Edition title keyword variations not exhaustively listed

**Recommendation:** ✅ PROCEED with requirement clarifications (no blockers, but must add regression testing requirement)

---

## Issue Analysis by Requirement

### REQ-EF-010: Edition Preference Scoring - Deluxe Penalty

**Completeness Check:**
- ✅ Inputs: Edition title, detected track count
- ✅ Outputs: Score multiplier (0.7×)
- ✅ Behavior: Case-insensitive keyword matching, multiplicative penalty
- ✅ Constraints: None specified (issue: no performance constraint)
- ❌ **Error Cases:** What if edition title is NULL or empty? (MEDIUM)
- ✅ Dependencies: Edition metadata from MusicBrainz

**Ambiguity Check:**
- ⚠️ **HIGH: "0.7× score multiplier" not validated against regression risk**
  - Question: Has 0.7 been tested? Could cause currently-passing albums to fail?
  - Recommendation: Make multiplier tunable, test with [0.5, 0.6, 0.7, 0.8, 0.9]
- ⚠️ **LOW: "Deluxe" and "expanded" keywords may not catch all variants**
  - Example: "Remastered", "Anniversary Edition", "Special Edition"
  - Recommendation: Document known variants, add logging to discover new ones

**Consistency Check:**
- ✅ No conflicts with other requirements

**Testability:**
- ✅ Can objectively verify: Given edition with "Deluxe" in title, score *= 0.7
- ⚠️ **Cannot verify "no regressions" without explicit requirement** (CRITICAL)

---

### REQ-EF-020: Edition Preference Scoring - Compilation Penalty

**Completeness Check:**
- ✅ Inputs: Edition title
- ✅ Outputs: Score multiplier (0.6×)
- ✅ Behavior: Case-insensitive keyword matching for "collection", "anthology", "best of"
- ✅ Constraints: Penalty applies before other adjustments
- ❌ **Error Cases:** NULL/empty title handling not specified (MEDIUM)
- ✅ Dependencies: Edition metadata from MusicBrainz

**Ambiguity Check:**
- ⚠️ **HIGH: "0.6× score multiplier" not validated against regression risk**
  - Same concern as REQ-EF-010
  - Recommendation: Make multiplier tunable, test with range
- ✅ Keywords are explicit and testable

**Consistency Check:**
- ✅ Works with REQ-EF-010 (cumulative penalties: 0.7 × 0.6 = 0.42 for deluxe compilation)

**Testability:**
- ✅ Objective verification possible
- ⚠️ **Cannot verify "no regressions" without explicit requirement** (CRITICAL)

---

### REQ-EF-030: Track Count Pre-Filtering

**Completeness Check:**
- ✅ Inputs: Edition track count, detected boundaries
- ✅ Outputs: Filtered edition list
- ✅ Behavior: Filter where |edition.track_count - detected| > 3
- ✅ Constraints: Apply before Stage 2, fallback if all filtered
- ⚠️ **Error Cases: Fallback behavior partially specified** (HIGH)
  - Specified: "Fall back to unfiltered list with warning log"
  - Missing: What if unfiltered list is also empty? Return error? Skip album?
- ✅ Dependencies: Detected boundary count from boundary detection stage

**Ambiguity Check:**
- ⚠️ **HIGH: "±3 tracks tolerance" not validated against regression risk**
  - User explicitly stated: "Evaluate with testing, regression not acceptable"
  - Question: Could ±3 be too strict? Too loose?
  - Recommendation: Test with [±2, ±3, ±4, ±5] and measure regression rate
- ✅ Filter logic is unambiguous (absolute difference > 3)

**Consistency Check:**
- ✅ Compatible with scoring requirements (filtering happens first)

**Testability:**
- ✅ Objective verification possible: Given 11 detected tracks, editions with 7 or 15+ tracks are filtered
- ⚠️ **Cannot verify "no regressions" without explicit requirement** (CRITICAL)

---

### REQ-EF-040: Artist Consistency Validation

**Completeness Check:**
- ✅ Inputs: Edition track list, source artist name
- ✅ Outputs: Boolean (accept/reject edition)
- ✅ Behavior: Extract unique artists, reject if >3 unique, fuzzy matching
- ✅ Constraints: Log rejection reason
- ❌ **Error Cases:** What if track.artist is NULL for some tracks? (MEDIUM)
- ✅ Dependencies: Track metadata from MusicBrainz

**Ambiguity Check:**
- ⚠️ **MEDIUM: "Fuzzy matching" algorithm not specified**
  - Current spec: "source artist name contained in track artist or vice versa"
  - Ambiguous: How to handle "The Beatles" vs "Beatles"? Case sensitivity?
  - Recommendation: Define exact matching algorithm:
    ```rust
    fn fuzzy_artist_match(source: &str, track_artist: &str) -> bool {
        let source_lower = source.to_lowercase();
        let track_lower = track_artist.to_lowercase();
        source_lower.contains(&track_lower) || track_lower.contains(&source_lower)
    }
    ```
- ⚠️ **MEDIUM: ">3 unique artists" threshold not validated**
  - Question: Could legitimate albums have 3-4 featured artists?
  - Example: Hip-hop albums with many featured artists
  - Recommendation: Test with actual data, consider >5 instead of >3

**Consistency Check:**
- ✅ No conflicts with other requirements

**Testability:**
- ⚠️ **Fuzzy matching not testable until algorithm specified** (MEDIUM)
- ✅ ">3 artists" threshold is testable

---

### REQ-EF-050: Remix Track Error Tolerance

**Completeness Check:**
- ✅ Inputs: Track title, duration error
- ✅ Outputs: Boolean (accept/reject error)
- ✅ Behavior: Detect keywords, accept if error < 120s
- ✅ Constraints: Standard tolerance still applies to non-remix tracks
- ✅ Error Cases: Implicitly handled (non-remix tracks fall through to standard validation)
- ✅ Dependencies: Track title from MusicBrainz, error calculation from matching stage

**Ambiguity Check:**
- ⚠️ **MEDIUM: Remix keyword list may be incomplete**
  - Specified: "remix", "extended", "mix)", "version"
  - Missing: "edit", "dub", "instrumental", "live", "acoustic"?
  - Question: Should "live" versions get tolerance? "Acoustic" versions?
  - Recommendation: Start with specified keywords, add logging to discover needed additions
- ✅ 120s threshold is explicit and testable

**Consistency Check:**
- ✅ Compatible with other requirements (applies after edition selected)

**Testability:**
- ✅ Objective verification possible: Track with "remix" in title and 85s error → accepted

---

## Cross-Requirement Issues

### CRITICAL-001: No Regression Testing Requirement

**Severity:** CRITICAL
**Category:** Missing Requirement

**Issue:**
User explicitly stated "regression in other albums is not acceptable" for both penalty multipliers and track count tolerance. However, requirements_index.md contains NO requirement for regression testing or validation.

**Impact:**
Without explicit regression testing requirement:
- Cannot verify that changes don't break 186 currently-passing albums
- Cannot tune penalty multipliers safely
- Cannot validate ±3 track tolerance
- No acceptance criteria for "zero regressions"

**Recommendation:**
Add **REQ-EF-060: Zero Regression Validation**
```markdown
**Category:** Non-Functional (Validation)
**Priority:** Critical

**Description:**
ALL edition filtering improvements MUST NOT cause regressions in currently-passing albums.

**Acceptance Criteria:**
- Run full 200-album test suite before and after implementation
- Baseline: 186/200 albums pass (93% match rate)
- After changes: ≥186/200 albums pass (no regressions)
- Track-level error comparison: No increase in mean/median/p95 duration error
- MBID stability: ≥90% of albums select same MBID (unless better match)

**Validation Method:**
1. Run baseline test: `cargo test --release test_run29f_full`
2. Capture baseline metrics (186 passes, error distribution, MBIDs)
3. Implement changes
4. Run regression test with identical test suite
5. Compare: passes (>=186), errors (no increase), MBIDs (>90% stable)
6. If regressions detected: Tune parameters (multipliers, tolerance) and re-test

**Parameter Tuning Protocol:**
- If 0.7 deluxe multiplier causes regressions, test [0.75, 0.8, 0.85, 0.9]
- If 0.6 compilation multiplier causes regressions, test [0.65, 0.7, 0.75]
- If ±3 track tolerance causes regressions, test [±4, ±5]
- Accept smallest penalty/largest tolerance that produces zero regressions

**Pass Criteria:**
Zero regressions on currently-passing albums (186/200 baseline).
```

**Status:** MUST ADD before implementation

---

### HIGH-001: Penalty Multipliers Not Empirically Validated

**Severity:** HIGH
**Category:** Unvalidated Assumption

**Issue:**
REQ-EF-010 and REQ-EF-020 specify 0.7 and 0.6 multipliers based on intuition, not empirical testing. User explicitly required validation: "Evaluate with testing, regression not acceptable."

**Impact:**
- Multipliers may be too aggressive (reject valid editions)
- Multipliers may be too conservative (still select wrong editions)
- Cannot know correct values without testing

**Recommendation:**
1. Make multipliers configurable (not hardcoded)
2. Test suite runs with multiple multiplier combinations:
   - Deluxe: [0.5, 0.6, 0.7, 0.8, 0.9]
   - Compilation: [0.5, 0.6, 0.7, 0.8, 0.9]
3. For each combination: measure regressions, measure problem album fixes
4. Select multipliers that achieve:
   - Zero regressions (primary)
   - Maximum problem album fixes (secondary)

**Example Results Matrix:**
```
Deluxe  Compilation  Regressions  Problems Fixed
0.7     0.6          0            3/4  ← Target if validated
0.8     0.7          0            3/4  ← Accept if 0.7/0.6 has regressions
0.9     0.8          0            2/4  ← Fallback if stricter values have regressions
```

**Status:** Resolve during implementation (add tuning code)

---

### HIGH-002: Track Count Tolerance Not Empirically Validated

**Severity:** HIGH
**Category:** Unvalidated Assumption

**Issue:**
REQ-EF-030 specifies ±3 tracks tolerance based on intuition. User explicitly required validation.

**Impact:**
- ±3 may be too strict (filter valid editions, cause regressions)
- ±3 may be too loose (allow wrong editions through)

**Recommendation:**
1. Test with multiple tolerance values: [±2, ±3, ±4, ±5]
2. For each value: measure regressions, measure problem album fixes
3. Select tolerance that achieves zero regressions + maximum fixes

**Expected Outcome:**
±3 is likely correct (based on problem album analysis), but must validate empirically.

**Status:** Resolve during implementation (add tuning code)

---

### HIGH-003: Fallback Behavior Incomplete

**Severity:** HIGH
**Category:** Incomplete Error Handling

**Issue:**
REQ-EF-030 specifies "fall back to unfiltered list if all editions filtered", but doesn't specify what happens if unfiltered list is also empty.

**Scenarios:**
1. Track count filter rejects all editions → fallback to unfiltered → proceed with unfiltered
2. Track count filter passes some editions → proceed with filtered
3. **Unspecified:** MusicBrainz returns zero editions (album not in database)

**Recommendation:**
Clarify REQ-EF-030 to specify:
```markdown
**Fallback Behavior:**
- If track count filter rejects all editions: Use unfiltered list, log warning
- If unfiltered list is empty (0 editions from MusicBrainz): Return error to user ("Album not found in MusicBrainz")
- If artist consistency filter rejects all editions: Use unfiltered list, log warning
```

**Status:** Clarify requirement

---

## Medium Priority Issues

### MEDIUM-001: Fuzzy Artist Matching Algorithm Not Specified

**Severity:** MEDIUM
**Category:** Ambiguous Behavior

**Issue:**
REQ-EF-040 says "fuzzy matching: source artist name contained in track artist or vice versa" but doesn't specify:
- Case sensitivity (should be insensitive)
- Whitespace handling ("The Beatles" vs "Beatles")
- Punctuation ("AC/DC" vs "AC DC")

**Recommendation:**
Define exact algorithm in requirement:
```rust
fn fuzzy_artist_match(source: &str, track_artist: &str) -> bool {
    let normalize = |s: &str| s.to_lowercase().trim().to_string();
    let source_norm = normalize(source);
    let track_norm = normalize(track_artist);
    source_norm.contains(&track_norm) || track_norm.contains(&source_norm)
}
```

**Status:** Clarify requirement or document as implementation detail

---

### MEDIUM-002: Remix Keyword List Incomplete

**Severity:** MEDIUM
**Category:** Incomplete Specification

**Issue:**
REQ-EF-050 specifies 4 keywords but may miss legitimate remix variants:
- Specified: "remix", "extended", "mix)", "version"
- Potentially missing: "edit", "dub", "instrumental", "radio", "single"

**Question:** Should non-remix variants like "live" or "acoustic" also get tolerance?
- Live recordings often have variable duration (audience interaction)
- Acoustic versions may differ significantly from studio versions

**Recommendation:**
1. Start with specified 4 keywords
2. Add DEBUG logging when error tolerance applied
3. Review logs after test run to discover needed additions
4. Document keyword list as "initial set, expandable based on empirical data"

**Status:** Accept as-is, plan for iteration

---

### MEDIUM-003: Performance Constraint Not Validated

**Severity:** MEDIUM
**Category:** Unvalidated Constraint

**Issue:**
scope_statement.md specifies "Edition filtering must complete in <100ms per album" but this is not validated.

**Question:** How was 100ms chosen? Is it achievable?

**Analysis:**
- Track count filtering: O(n) where n = edition count (typically 5-20) → <1ms
- Artist consistency: O(n×m) where m = tracks per edition (typically 10-15) → <5ms
- Edition scoring: O(n) → <1ms
- Total: <10ms expected, 100ms is very conservative

**Recommendation:**
100ms constraint is achievable. Add performance test to verify, but not a blocker.

**Status:** Accept constraint, verify during testing

---

## Low Priority Issues

### LOW-001: Edition Title Keyword Variations Not Exhaustive

**Severity:** LOW
**Category:** Incomplete Specification

**Issue:**
Deluxe edition keywords ("deluxe", "expanded") may not catch all variants:
- "Remastered" (often includes bonus tracks)
- "Anniversary Edition" (often deluxe)
- "Special Edition" (often bonus content)
- "Collector's Edition"

**Impact:** Minor. Most deluxe editions include "deluxe" or "expanded" in title. Uncaught variants can be added later based on empirical data.

**Recommendation:**
Document as known limitation, add to future enhancements. Add logging to discover missed cases.

**Status:** Document, defer to future iteration

---

## Issues Summary Table

| ID | Severity | Category | Issue | Resolution |
|----|----------|----------|-------|------------|
| CRITICAL-001 | Critical | Missing Requirement | No regression testing requirement | ADD REQ-EF-060 |
| HIGH-001 | High | Unvalidated | Penalty multipliers not tested | Make tunable, validate empirically |
| HIGH-002 | High | Unvalidated | Track count tolerance not tested | Test ±2/±3/±4/±5, select best |
| HIGH-003 | High | Incomplete | Fallback behavior for empty list | Clarify REQ-EF-030 |
| MEDIUM-001 | Medium | Ambiguous | Fuzzy artist matching algorithm | Define exact algorithm |
| MEDIUM-002 | Medium | Incomplete | Remix keyword list | Document as initial set, iterate |
| MEDIUM-003 | Medium | Unvalidated | 100ms performance constraint | Verify during testing |
| LOW-001 | Low | Incomplete | Edition title keywords | Document limitation, iterate |

---

## Dependency Validation

**All dependencies from dependencies_map.md verified:**
- ✅ MusicBrainz client: Stable, provides edition metadata
- ✅ Matching pipeline: Stable, integration point well-defined
- ✅ Progressive RMS: Recently completed, stable
- ✅ Test suite: Available (200 albums, baseline comparison)

**No dependency issues identified.**

---

## Recommendations

### MUST DO (Critical)

1. **Add REQ-EF-060: Zero Regression Validation**
   - Explicit acceptance criteria: ≥186/200 albums pass
   - Parameter tuning protocol defined
   - Test comparison methodology specified

### SHOULD DO (High Priority)

2. **Make Penalty Multipliers Tunable**
   - Add configuration constants at top of edition_filter.rs
   - Test suite can iterate over multiplier combinations
   - Select values that achieve zero regressions + max fixes

3. **Make Track Count Tolerance Tunable**
   - Similar to penalty multipliers
   - Test ±2, ±3, ±4, ±5
   - Select value with zero regressions

4. **Clarify Fallback Behavior**
   - Update REQ-EF-030 to specify empty list handling
   - Define error return for "album not in MusicBrainz"

### NICE TO HAVE (Medium/Low)

5. **Define Fuzzy Artist Matching Algorithm**
   - Can be implementation detail (not in requirement)
   - Document in code comments

6. **Document Remix Keyword List as Expandable**
   - Initial set specified
   - Add logging to discover new variants
   - Plan for iteration based on data

---

## Decision Point

**Can we proceed to Phase 3 (Acceptance Test Definition)?**

**Answer:** ✅ YES, with clarifications

**Blockers Resolved:**
- CRITICAL-001: Will add REQ-EF-060 during Phase 3
- HIGH-001, HIGH-002: Will add tuning protocol to acceptance tests
- HIGH-003: Will clarify in acceptance test specification

**No hard blockers.** Issues identified are clarifications needed for test definition, which is exactly what Phase 3 addresses.

---

## Next Steps

**Phase 3 Preview:**
Will define acceptance tests including:
- Regression test suite (REQ-EF-060)
- Parameter tuning tests (penalty multipliers, track count tolerance)
- Edge case tests (empty lists, NULL values)
- Integration tests (full 200-album suite)
- Unit tests (individual filtering functions)

**Estimated Test Count:** 25-35 tests (5 requirements × 5-7 tests each)

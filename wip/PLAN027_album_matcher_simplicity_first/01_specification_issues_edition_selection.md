# Specification Issues - Edition Selection Requirements

**Plan:** PLAN027 Album Matcher Simplicity-First Redesign
**Focus:** Part 2.2 Edition Selection and Ranking (REQ-AM-092 through REQ-AM-096)
**Date:** 2025-12-28

---

## Phase 2: Specification Completeness Verification

### Edition Selection Requirements Analysis

**Requirements Verified:**
- REQ-AM-092: Multi-Factor Weighted Scoring
- REQ-AM-093: Total Duration Alignment Scoring
- REQ-AM-094: Track Match Quality - Graduated Scoring
- REQ-AM-095: Graduated Track Count Tolerance
- REQ-AM-096: Multi-Strategy MusicBrainz Search

---

## Completeness Check Results

### REQ-AM-092: Multi-Factor Weighted Scoring

**Inputs Specified:** ✅
- Total duration score (0.0-1.0)
- Track quality score (0.0-1.0)
- Name similarity score (0.0-1.0)
- Track count penalty (0.0-1.0)

**Outputs Specified:** ✅
- Edition score (0.0-1.0 range)
- Ranked list of editions (descending by score)
- Top-ranked edition selected

**Behavior Specified:** ✅
- Weighted combination: (duration × 0.30) + (quality × 0.45) + (name × 0.25)
- Multiplicative track count penalty applied to base score
- Sorting and selection logic clear

**Constraints Specified:** ✅
- Weights must sum to 1.0 (0.30 + 0.45 + 0.25 = 1.00)
- All component scores in range [0.0, 1.0]
- Track count penalty multiplicative (not additive)

**Error Cases Specified:** ⚠️ **MEDIUM Issue**
- MISSING: What if candidate_editions list is empty?
- MISSING: What if all editions score 0.0?
- MISSING: What if multiple editions have identical scores (tie-breaking)?

**Dependencies Specified:** ✅
- Depends on REQ-AM-093 (duration scoring function)
- Depends on REQ-AM-094 (quality scoring function)
- Depends on REQ-AM-095 (track count penalty function)
- Depends on name similarity calculation (existing)

**Status:** ✅ MOSTLY COMPLETE - Minor edge cases need clarification

---

### REQ-AM-093: Total Duration Alignment Scoring

**Inputs Specified:** ✅
- Detected total duration (milliseconds)
- Edition total duration (milliseconds)

**Outputs Specified:** ✅
- Duration score (0.05, 0.30, 0.60, 0.80, or 0.95)

**Behavior Specified:** ✅
- Calculate percentage difference
- Graduated penalties clearly defined (<5%, 5-10%, 10-15%, 15-25%, >25%)

**Constraints Specified:** ✅
- Percentage diff_pct based on detected_total_ms (denominator)
- Five discrete penalty bands

**Error Cases Specified:** ⚠️ **LOW Issue**
- MISSING: What if detected_total_ms = 0? (division by zero)
- MISSING: What if edition_total_ms = 0?

**Dependencies Specified:** ✅
- Requires total duration calculation from all tracks
- Standalone function, no external dependencies

**Status:** ✅ MOSTLY COMPLETE - Edge case (zero duration) unlikely but should handle

---

### REQ-AM-094: Track Match Quality - Graduated Scoring

**Inputs Specified:** ✅
- Detected track durations (array of f64, seconds)
- Edition track durations (array of f64, seconds)
- Tolerance (f64, seconds)

**Outputs Specified:** ✅
- Average quality score (0.0-1.0)

**Behavior Specified:** ✅
- Per-track quality: 1.0 - (error / tolerance) for error ≤ tolerance
- Per-track quality: 0.0 for error > tolerance
- Average across all tracks

**Constraints Specified:** ✅
- Uses minimum of two array lengths (handles mismatched counts)
- Linear decay from 1.0 to 0.0 based on error magnitude

**Error Cases Specified:** ⚠️ **MEDIUM Issue**
- MISSING: What if both arrays are empty? (zero tracks)
- MISSING: What if tolerance = 0? (division by zero)
- CLARIFY: Should mismatched track counts beyond min(len1, len2) contribute to quality?

**Dependencies Specified:** ✅
- Standalone calculation
- No external dependencies

**Status:** ⚠️ NEEDS CLARIFICATION - Edge cases and track count mismatch handling

---

### REQ-AM-095: Graduated Track Count Tolerance

**Inputs Specified:** ✅
- Detected track count (usize)
- Edition track count (usize)

**Outputs Specified:** ✅
- Penalty multiplier (0.20, 0.50, 0.70, 0.85, 0.95, or 1.00)

**Behavior Specified:** ✅
- Calculate absolute difference
- Match statement with 6 penalty bands (0, 1, 2, 3, 4-5, 6+)

**Constraints Specified:** ✅
- Penalties decrease as difference increases
- Exact match = no penalty (1.00)

**Error Cases Specified:** ✅
- All cases covered (0 through unlimited difference)
- No edge cases identified

**Dependencies Specified:** ✅
- Standalone function
- No external dependencies

**Status:** ✅ COMPLETE - No issues identified

---

### REQ-AM-096: Multi-Strategy MusicBrainz Search

**Inputs Specified:** ✅
- Artist name (string)
- Album name (string)

**Outputs Specified:** ✅
- List of unique editions (combined from all strategies)

**Behavior Specified:** ✅
- 7 progressive search strategies documented
- Already implemented in wkmp-ai/src/services/musicbrainz_client.rs

**Constraints Specified:** ⚠️ **LOW Issue**
- CLARIFY: How are duplicate editions (same MBID from different strategies) handled?
- CLARIFY: What's the maximum combined result limit?

**Error Cases Specified:** ⚠️ **MEDIUM Issue**
- MISSING: What if all 7 strategies return 0 results?
- MISSING: What if API rate limit exceeded mid-search?

**Dependencies Specified:** ✅
- Existing MusicBrainz client infrastructure
- Caching layer (PLAN026)

**Status:** ⚠️ NEEDS CLARIFICATION - Already implemented but edge cases unclear in spec

---

## Ambiguity Check Results

### REQ-AM-092: Multi-Factor Weighted Scoring

**Ambiguous Language:** None identified
**Unquantified Requirements:** None identified
**Undefined Terms:** None identified
**Multiple Interpretations:** ⚠️ **MEDIUM**

**Ambiguity:** Tie-breaking when multiple editions have identical scores
- Could sort by: name similarity, edition MBID, random
- **Resolution:** Add tie-breaking rule to specification

**Test:** Two engineers could implement tie-breaking differently
**Verdict:** ⚠️ MINOR AMBIGUITY - needs tie-breaking specification

---

### REQ-AM-094: Track Match Quality

**Ambiguous Language:** None identified
**Unquantified Requirements:** None identified
**Undefined Terms:** None identified
**Multiple Interpretations:** ⚠️ **MEDIUM**

**Ambiguity:** Handling tracks beyond min(detected_count, edition_count)
- Option A: Ignore extra tracks (current spec)
- Option B: Penalize for missing/extra tracks (quality = 0 for unmatched)
- **Impact:** Affects quality score significantly for mismatched counts

**Test:** Two engineers could implement track count mismatch handling differently
**Verdict:** ⚠️ AMBIGUITY - needs clarification on unmatched tracks

---

### Other Requirements

**REQ-AM-093, REQ-AM-095, REQ-AM-096:** ✅ No ambiguities identified

---

## Consistency Check Results

### Cross-Requirement Analysis

**Consistent:**
- ✅ REQ-AM-094 and REQ-AM-095 both handle track count differences (quality vs penalty)
- ✅ Component score ranges [0.0, 1.0] consistent across REQ-AM-093, REQ-AM-094
- ✅ Weights in REQ-AM-092 sum to 1.0 (30% + 45% + 25% = 100%)

**Potential Conflicts:** ⚠️ **MEDIUM Issue**

**Conflict:** Track count handling appears in TWO places:
1. **REQ-AM-094 (Track Quality):** Uses min(detected, edition) - ignores mismatched tracks
2. **REQ-AM-095 (Track Count Penalty):** Applies penalty based on absolute difference

**Question:** Are these complementary or redundant?
- If detected=11, edition=12:
  - Quality calculated on first 11 tracks (12th edition track ignored)
  - Penalty applied for ±1 difference (0.95 multiplier)
- This seems correct (quality + penalty work together)

**Verdict:** ✅ CONSISTENT - Both factors work together (quality on matched portion, penalty for count difference)

---

## Testability Check Results

### REQ-AM-092: Multi-Factor Weighted Scoring

**Objectively Verifiable:** ✅ YES
**Test to Prove Compliance:**
```rust
#[test]
fn test_multi_factor_scoring() {
    let duration_score = 0.95;
    let quality_score = 0.85;
    let name_score = 0.70;
    let track_count_penalty = 0.95;

    let base = (0.95 * 0.30) + (0.85 * 0.45) + (0.70 * 0.25);
    let expected = base * 0.95;
    let actual = calculate_edition_score(...);

    assert_approx_eq!(actual, expected, epsilon=0.001);
}
```

**Test to Prove Violation:**
```rust
#[test]
fn test_incorrect_weighting() {
    // If weights don't sum to 1.0 or formula wrong, test fails
    // Check component scores individually
}
```

**Test Conditions Achievable:** ✅ YES - all inputs can be controlled

**Status:** ✅ TESTABLE

---

### REQ-AM-093: Total Duration Alignment

**Objectively Verifiable:** ✅ YES
**Test to Prove Compliance:**
```rust
#[test]
fn test_duration_scoring_thresholds() {
    assert_eq!(score(40_000, 41_000), 0.95);  // 2.5% diff → <5%
    assert_eq!(score(40_000, 43_000), 0.80);  // 7.5% diff → 5-10%
    assert_eq!(score(40_000, 45_000), 0.60);  // 12.5% diff → 10-15%
    assert_eq!(score(40_000, 48_000), 0.30);  // 20% diff → 15-25%
    assert_eq!(score(40_000, 60_000), 0.05);  // 50% diff → >25%
}
```

**Test Conditions Achievable:** ✅ YES

**Status:** ✅ TESTABLE

---

### REQ-AM-094, REQ-AM-095, REQ-AM-096: All Testable

✅ All requirements have clear pass/fail criteria
✅ All inputs and outputs quantified
✅ Test data easily generated

---

## Issues Summary

### CRITICAL Issues: 0
No critical issues that block implementation

### HIGH Issues: 0
No high-risk issues

### MEDIUM Issues: 4

**MEDIUM-01: REQ-AM-092 - Missing Error Handling**
- **Issue:** Undefined behavior for empty edition list, all-zero scores, score ties
- **Impact:** Edge cases may cause crashes or non-deterministic results
- **Recommendation:**
  - Return None if editions.is_empty()
  - Return None if all scores ≤ 0.0
  - Tie-breaking: prefer edition with higher name similarity, then lexicographic MBID

**MEDIUM-02: REQ-AM-094 - Track Count Mismatch Ambiguity**
- **Issue:** Unclear how unmatched tracks beyond min(len1, len2) affect quality
- **Impact:** Different implementations may produce different quality scores
- **Recommendation:**
  - Option A (Current): Quality based only on matched tracks (first N)
  - Option B (Penalize): Unmatched tracks contribute quality=0
  - **Suggest Option A** - clearer separation of concerns (quality vs count penalty)

**MEDIUM-03: REQ-AM-094 - Zero Tolerance Edge Case**
- **Issue:** Division by zero if tolerance = 0
- **Impact:** Panic at runtime
- **Recommendation:** Assert tolerance > 0 or return quality=0 if tolerance=0

**MEDIUM-04: REQ-AM-096 - Missing Error Handling**
- **Issue:** Undefined behavior if all strategies return 0 results
- **Impact:** Stage 0 should catch this, but specification unclear
- **Recommendation:** Clarify that empty result = validation failure in Stage 0

### LOW Issues: 2

**LOW-01: REQ-AM-093 - Zero Duration Edge Case**
- **Issue:** Division by zero if detected_total_ms = 0
- **Impact:** Unlikely (Stage 0 validates duration > 60s) but possible
- **Recommendation:** Return score=0.05 if either duration is 0

**LOW-02: REQ-AM-096 - Duplicate Handling Unclear**
- **Issue:** Specification doesn't state how duplicate MBIDs from different strategies are handled
- **Impact:** Minor - likely deduplicated by implementation
- **Recommendation:** Clarify that editions are deduplicated by MBID

---

## Dependency Validation

### Edition Selection Dependencies

**Edition selection depends on:**
1. ✅ Acoustic matching stages (Stages 2-5) produce track candidates
2. ✅ MusicBrainz search (REQ-AM-096) provides candidate editions
3. ✅ Name similarity calculation (existing function)
4. ✅ All component scoring functions exist (REQ-AM-093, 094, 095)

**Dependencies Verified:**
- ✅ Multi-strategy search already implemented (wkmp-ai/src/services/musicbrainz_client.rs)
- ✅ Name similarity (Jaro-Winkler) exists in utils/string_similarity.rs
- ⚠️ Duration calculation needs to aggregate all track durations (NEW utility function)
- ⚠️ Edition selection logic needs to be integrated into orchestrator.rs

---

## Resolution Recommendations

### Immediate (Before Implementation)

1. **Add to SPEC Part 2.2.2 (Multi-Factor Scoring):**
   ```
   **Edge Cases:**
   - If candidate_editions is empty: Return None (no selection possible)
   - If all edition scores ≤ 0.0: Return None (no acceptable matches)
   - If multiple editions have identical scores: Prefer higher name_similarity, then lexicographic MBID
   ```

2. **Add to SPEC Part 2.2.4 (Track Quality):**
   ```
   **Track Count Mismatch Handling:**
   - Quality calculated only on first min(detected_count, edition_count) tracks
   - Extra tracks beyond min length are ignored for quality calculation
   - Track count difference penalty applied separately via REQ-AM-095
   - Tolerance must be > 0 (assert in function or return 0.0 if tolerance ≤ 0)
   ```

3. **Add to SPEC Part 2.2.3 (Duration Alignment):**
   ```
   **Edge Cases:**
   - If detected_total_ms = 0 or edition_total_ms = 0: Return score = 0.05 (very poor alignment)
   ```

4. **Add to SPEC Part 2.2.6 (Multi-Strategy Search):**
   ```
   **Deduplication:**
   - Editions returned from multiple strategies are deduplicated by release MBID
   - If all strategies return 0 results: Stage 0 validation should have caught this (REQ-AM-003)
   ```

### Nice-to-Have (Can Address During Implementation)

- Add comprehensive edge case unit tests
- Add integration test for Aqualung case (box set vs standard)
- Monitor edition type distribution in production

---

## Overall Assessment

**Specification Quality:** ✅ GOOD with minor improvements needed

**Summary:**
- **0 CRITICAL issues** - No blockers
- **4 MEDIUM issues** - Edge cases and ambiguities (addressable with spec clarifications)
- **2 LOW issues** - Unlikely edge cases (good practice to handle)

**Recommendation:** ✅ **PROCEED to Phase 3** (Test Definition)
- Specification is sufficiently complete for implementation
- Address MEDIUM issues by adding clarifications to SPEC Part 2.2 (before writing tests)
- LOW issues can be handled with defensive programming during implementation

**User Approval Checkpoint:** Present these findings to user before proceeding to test definition.

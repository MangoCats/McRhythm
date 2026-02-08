# PLAN028: Approach Selection

## Decision: Implementation Approach for Partial Album Matching

---

## Approach A: Modify Duration Filter Only

**Description:** Extend the existing duration filter in `filtering.rs` to allow 50-85% matches, then let existing matching logic handle the rest.

**Pros:**
- Minimal code changes (~20 lines)
- No new modules

**Cons:**
- Existing matching logic expects full album
- Would need significant changes to boundary detection
- Track count mismatch would cause poor quality scores
- **HIGH RISK:** Likely to cause regressions on existing matches

**Risk Assessment:**
- Probability: High (70%)
- Impact: High (regressions on 187 working albums)
- Residual Risk: **HIGH**

---

## Approach B: New Partial Matching Module (RECOMMENDED)

**Description:** Create a new `partial_matching.rs` module that:
1. Detects partial album candidates (50-85% ratio)
2. Calculates cumulative track durations
3. Finds best N tracks matching file duration
4. Calls existing boundary detection with N tracks
5. Returns partial match result

**Pros:**
- Isolates new logic from existing matching
- Existing full-match path unchanged
- Easy to enable/disable
- Clear separation of concerns
- Low regression risk

**Cons:**
- More code (~150 lines)
- New module to maintain

**Risk Assessment:**
- Probability: Low (15%)
- Impact: Low (isolated to partial matches only)
- Mitigation: Comprehensive unit tests, regression suite
- Residual Risk: **LOW**

---

## Approach C: Post-Filter Partial Detection

**Description:** After full match fails duration filter, check if file could be partial match as fallback.

**Pros:**
- Full match path completely unchanged
- Partial matching is pure addition

**Cons:**
- Same as Approach B but less elegant
- Slightly more complex control flow

**Risk Assessment:**
- Probability: Low (20%)
- Impact: Low
- Residual Risk: **LOW**

---

## Decision

**RECOMMENDATION: Approach B (New Partial Matching Module)**

**Risk-Based Justification:**
- Approach B has lowest residual risk (Low) after mitigation
- Approach A has unacceptable risk of regressions (High)
- Approach C is equivalent risk to B but less elegant

Per CLAUDE.md Decision-Making Framework: Risk (primary) → Quality (secondary) → Effort (tertiary).

**Quality Characteristics (B and C equivalent):**
- Maintainability: High (isolated module)
- Test coverage: High (comprehensive unit tests possible)
- Architectural alignment: High (follows existing module pattern)

**Effort:**
- Approach B: ~4 hours implementation
- Approach A: ~2 hours but HIGH regression risk
- Risk reduction justifies effort differential

---

## Architecture Decision Record

**Status:** Proposed
**Date:** 2026-01-26
**Context:** Need to support partial album matching for files containing tracks 1-N of an album (e.g., Fluke/Puppy.mp3 contains tracks 1-8 of 11)

**Decision:** Create new `partial_matching.rs` module with isolated partial match logic, invoked after full match fails duration filter.

**Consequences:**
- New module added to wkmp-ai/src/matching/
- AlbumMatchResult gains `partial` flag and `matched_tracks`/`total_tracks` fields
- Logging updated to show partial match details
- 200-album regression suite must pass before merge

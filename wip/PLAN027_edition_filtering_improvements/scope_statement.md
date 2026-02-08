# Scope Statement - Edition Filtering Improvements

**Plan:** PLAN027
**Date:** 2026-01-16
**Source:** wkmp-ai/problem_albums_analysis.md

---

## Executive Summary

Implement 5 targeted improvements to MusicBrainz edition selection algorithm to fix 3 out of 4 remaining problem albums. Changes are internal to matching logic with no database or API modifications required.

---

## ✅ In Scope

### 1. Edition Preference Scoring (Critical)
- Implement scoring function that penalizes deluxe editions (0.7× multiplier)
- Implement scoring function that penalizes compilation editions (0.6× multiplier)
- Apply scoring to all candidate editions before final selection
- Case-insensitive keyword matching in edition titles

### 2. Track Count Pre-Filtering (High Priority)
- Filter editions with track count >±3 from detected boundaries
- Apply filter BEFORE running matching stages (performance optimization)
- Fallback to unfiltered list if all editions rejected
- Log filtered editions for debugging

### 3. Artist Consistency Validation (High Priority)
- Extract unique artists from edition track metadata
- Reject editions with >3 unique artists (multi-artist compilations)
- Fuzzy artist name matching (substring containment)
- Log rejection reason for debugging

### 4. Remix Track Error Tolerance (Medium Priority)
- Detect remix tracks by keywords ("remix", "extended", "mix)", "version")
- Accept up to 120s duration error for identified remix tracks
- Standard tolerance (<30s) still applies to non-remix tracks
- Log when remix tolerance applied

### 5. Logging Enhancements (Low Priority)
- Log edition filtering decisions (track count, artist consistency)
- Log edition scoring adjustments (deluxe penalty, compilation penalty)
- Log remix tolerance application
- All logs at DEBUG level (no changes to existing INFO/WARN output)

---

## ❌ Out of Scope

### 1. Database Schema Changes
- NO changes to `albums`, `recordings`, `artists` tables
- NO new database fields or indexes
- Filtering is in-memory during matching only

### 2. API Changes
- NO changes to wkmp-ai HTTP API endpoints
- NO changes to request/response formats
- NO changes to SSE event schema

### 3. UI Changes
- NO changes to import wizard UI
- NO new configuration options exposed to user
- NO changes to progress reporting

### 4. MusicBrainz API Integration
- NO changes to MusicBrainz API client
- NO additional API queries
- Use existing edition metadata only

### 5. Boundary Detection Algorithm
- NO changes to progressive RMS optimization (just completed)
- NO changes to Stage 7 refinement logic
- NO changes to silence detection

### 6. Advanced Edition Selection
- NO machine learning or statistical modeling
- NO genre-based edition selection
- NO user preference learning
- Simple keyword-based heuristics only

### 7. Backward Compatibility
- NO support for re-matching existing albums (user must re-import)
- NO migration of existing matches to new algorithm

---

## Assumptions

### Technical Assumptions

1. **MusicBrainz Data Quality:**
   - Edition titles contain recognizable keywords ("deluxe", "compilation", etc.)
   - Track artist metadata is present and reasonably accurate
   - Track count metadata matches actual track list length

2. **User Audio Files:**
   - Files are studio album rips (not compilations)
   - Files match a single artist (not multi-artist albums)
   - Files are complete albums (not partial rips)

3. **Existing Algorithm:**
   - Current 7-stage matching pipeline remains unchanged
   - Progressive RMS optimization (just completed) is stable
   - Edition metadata is already fetched from MusicBrainz

### Operational Assumptions

1. **Testing:**
   - Test suite includes 4 problem albums as regression tests
   - Baseline comparison available (run29f_baseline_1-12-26)
   - Full test run completes in ~80 minutes

2. **Deployment:**
   - Changes deployed via standard wkmp-ai rebuild
   - No configuration updates required
   - No user action required (automatic on next import)

---

## Constraints

### Technical Constraints

1. **Performance:**
   - Edition filtering must complete in <100ms per album
   - No additional MusicBrainz API queries allowed (use cached data)
   - Track count pre-filtering must reduce computation, not increase it

2. **Compatibility:**
   - Must work with existing MusicBrainz schema (no custom fields)
   - Must integrate with existing 7-stage pipeline without refactoring
   - Must not break currently-passing albums (186/200 baseline)

3. **Code Quality:**
   - Follow IMPL002-coding_conventions.md
   - Add unit tests for new filtering functions
   - Document scoring formula in comments

### Process Constraints

1. **Testing:**
   - Must run full 200-album test suite before committing
   - Must verify 0 regressions on currently-passing albums
   - Must verify 3/4 problem albums fixed (75% target)

2. **Documentation:**
   - Update problem_albums_analysis.md with actual results
   - Document scoring formula in code comments
   - Add logging examples to implementation notes

3. **Timeline:**
   - Implementation: 1-2 days (14-20 hours estimated)
   - Testing: 4-6 hours (3× full test runs + analysis)
   - Total: 3-4 days elapsed time

---

## Dependencies

### Existing Code (No Changes)

- `wkmp-ai/src/services/musicbrainz_client.rs` - Edition metadata fetching
- `wkmp-ai/src/matching/stages/stage2_album_first.rs` - Album-first matching
- `wkmp-ai/src/matching/stages/boundary_refinement.rs` - Progressive RMS (just completed)
- `wkmp-ai/src/matching/mod.rs` - Matching pipeline orchestration

### Code to Modify

- `wkmp-ai/src/matching/stages/mod.rs` - Add edition filtering functions
  OR
- `wkmp-ai/src/matching/edition_filter.rs` - New module for filtering logic (preferred)
- `wkmp-ai/src/matching/stages/stage2_album_first.rs` - Apply pre-filtering before matching

### External Dependencies (No Changes)

- MusicBrainz API (read-only, no new queries)
- SQLite database (read-only during matching)
- Progressive RMS implementation (dependency, not modified)

---

## Success Criteria

### Quantitative Metrics

1. **Problem Albums Fixed:**
   - Imagine Dragons - Night Visions: Match 11-track standard edition (currently 16-track deluxe)
   - Michael Jackson - Thriller: Match 9-track standard edition (currently compilation)
   - James Gang - Funk #49: Reject multi-artist compilation (currently matches)
   - Chemical Brothers - Surrender: Accept with remix tolerance (currently acceptable anyway)

2. **No Regressions:**
   - All 186 currently-passing albums still pass
   - No increase in average track duration error
   - No increase in MBID churn (different MBID selected)

3. **Performance:**
   - Edition filtering adds <100ms per album (amortized)
   - Full test run completes in <90 minutes (baseline: 80.9 minutes)
   - Track count pre-filtering reduces candidate editions by 30-50%

### Qualitative Criteria

1. **Code Quality:**
   - Edition filtering logic is modular and testable
   - Scoring formula is documented and explainable
   - Logging provides clear debugging information

2. **Maintainability:**
   - New filtering logic integrates cleanly with existing pipeline
   - No duplication between stages (DRY principle)
   - Easy to add new edition scoring factors in future

3. **User Experience:**
   - No visible changes (improvements are transparent)
   - Import results improve for affected albums
   - No new configuration required

---

## Risk Assessment

### Low Risk

- **Implementation Complexity:** Simple keyword matching and arithmetic scoring
- **Integration:** Minimal changes to existing pipeline
- **Testing:** Full regression suite available

### Medium Risk

- **Edge Cases:** Unknown edition title variations (e.g., "Remastered", "Anniversary")
- **Mitigation:** Comprehensive logging, easy to add new keywords

### Acceptable Trade-offs

- **May reject valid editions:** Overly aggressive filtering might reject correct editions
  - Mitigation: Conservative thresholds (±3 tracks, >3 artists), fallback to unfiltered
- **Keyword-based heuristics:** Not perfect (e.g., "Deluxe" in original title)
  - Mitigation: Acceptable for 75% improvement, can refine later if needed

---

## Out of Scope Justification

### Why No Database Changes?

Edition filtering is algorithmic selection logic, not persistent data. All information needed (edition metadata, track counts, artist names) is already available from MusicBrainz API responses. No need to store filtering decisions.

### Why No UI Changes?

Edition selection is fully automatic. User has no control over which edition is selected (by design - algorithmic selection based on musical flavor distance). Exposing edition selection preferences would add complexity without clear benefit.

### Why No Re-Matching of Existing Albums?

Re-matching would require:
- Identifying albums matched by old algorithm
- Re-running full matching pipeline (expensive)
- Handling cases where new algorithm produces worse results
- User confusion ("why did my album metadata change?")

User can re-import affected albums manually if desired. Most users have correctly-matched albums and don't need re-matching.

---

## Next Steps

**After Scope Approval:**
1. Proceed to Phase 2: Specification Completeness Verification
2. Proceed to Phase 3: Acceptance Test Definition
3. If no critical issues: Proceed to implementation

**Phase 2 Preview:**
Will verify that all requirements have:
- Clear inputs/outputs specified
- Testable acceptance criteria
- Error cases defined
- No ambiguous language

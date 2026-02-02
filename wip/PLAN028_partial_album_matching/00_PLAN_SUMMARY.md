# PLAN028: Partial Album Matching - PLAN SUMMARY

**Status:** Ready for Implementation
**Created:** 2026-01-26
**Specification Source:** Analysis of Fluke/Puppy.mp3 duration mismatch
**Plan Location:** `wip/PLAN028_partial_album_matching/`

---

## READ THIS FIRST

This document provides the implementation plan for partial album matching - supporting files that contain a contiguous subset of tracks from the beginning of an album.

**For Implementation:**
- Read this summary (~200 lines)
- Review detailed requirements: `requirements_index.md`
- Review test specifications: `02_test_specifications/test_index.md`
- Follow traceability matrix: `02_test_specifications/traceability_matrix.md`

**Context Window Budget:** ~650 lines per increment (summary + increment + relevant tests)

---

## Executive Summary

### Problem Being Solved

Files containing a contiguous subset of tracks from the beginning of an album are currently rejected by the duration filter (85% minimum threshold). Example:

| File | Duration | Album Duration | Ratio | Current Result |
|------|----------|----------------|-------|----------------|
| Fluke/Puppy.mp3 | 2995s (49:55) | 4039s (67:19) | 74.2% | REJECTED |

However, this file contains **tracks 1-8** of the 11-track album (cumulative duration: 2989s), which could yield 8 valid Recording MBIDs for AcousticBrainz lookup.

### Solution Approach

Create a new partial matching module that:
1. Detects partial album candidates (50-85% duration ratio)
2. Calculates cumulative track durations
3. Finds best N tracks matching file duration
4. Returns valid MBIDs for matched tracks

### Implementation Status

**Phases 1-3 Complete:**
- Phase 1: Scope Definition - 8 requirements extracted
- Phase 2: Specification Verification - 0 Critical, 0 High, 2 Medium, 1 Low issues
- Phase 3: Test Definition - 12 tests defined, 100% coverage

**Phases 4-8 Complete:**
- Phase 4: Approach Selection - New partial_matching.rs module (lowest risk)
- Phase 5: Implementation Breakdown - 6 increments defined
- Phase 8: Plan Documentation - This document

---

## Requirements Summary

**Total Requirements:** 8 (4 P0, 3 P1, 1 P2)

| Req ID | Priority | Description |
|--------|----------|-------------|
| REQ-PAM-001 | P0 | Detect partial album candidates (50-85% ratio) |
| REQ-PAM-002 | P0 | Calculate cumulative track durations, find best N |
| REQ-PAM-003 | P0 | Return valid Recording MBIDs for matched tracks |
| REQ-PAM-004 | P1 | Require minimum 60% track coverage |
| REQ-PAM-005 | P1 | Apply 80% match quality threshold |
| REQ-PAM-006 | P2 | Log partial matches with track count |
| REQ-PAM-007 | P1 | No regressions on 200-album test suite |
| REQ-PAM-008 | P2 | Performance impact <5% |

**Full Requirements:** See `requirements_index.md`

---

## Scope

### In Scope

- Detecting files with 50-85% duration ratio
- Matching tracks 1-N from beginning of album
- Returning MBIDs for matched tracks
- Minimum 60% track coverage threshold
- Quality threshold for partial matches
- Logging partial match details
- Full regression testing

### Out of Scope

- Matching tracks from middle or end of album
- Shuffled track order matching
- Multiple partial segments
- Automatic file repair suggestions

**Full Scope:** See `requirements_index.md` Out of Scope section

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 0
- **HIGH Issues:** 0
- **MEDIUM Issues:** 2 (60% threshold edge cases, multiple edition selection)
- **LOW Issues:** 1 (logging format)

**Decision:** PROCEED - No blocking issues

**Full Analysis:** See `01_specification_issues.md`

---

## Implementation Roadmap

### Increment 1: Partial Album Detection Functions
**Objective:** Implement core detection and calculation functions
**Deliverables:**
- Create `partial_matching.rs` module
- Functions: `is_partial_album_candidate`, `calculate_cumulative_durations`, `find_partial_track_count`, `is_partial_match_acceptable`
- Unit tests (6)
**Tests:** TC-U-PAM-001 through TC-U-PAM-006
**Success Criteria:** All 6 unit tests pass

### Increment 2: AlbumMatchResult Extensions
**Objective:** Extend result types for partial match info
**Deliverables:**
- Add `partial`, `matched_tracks`, `total_tracks` fields
- Backward compatible defaults
**Tests:** Existing tests pass
**Success Criteria:** No breaking changes

### Increment 3: Integrate Partial Matching into Pipeline
**Objective:** Wire partial matching into orchestrator
**Deliverables:**
- Detect partial candidates after duration filter fails
- Call partial matching functions
- Create partial match results
**Tests:** TC-I-PAM-001, TC-I-PAM-002
**Success Criteria:** Partial matching triggers correctly

### Increment 4: Logging and Result Formatting
**Objective:** Update logging for partial matches
**Deliverables:**
- Log format: `partial_match=8/11`
- Backward compatible for full matches
**Tests:** TC-I-PAM-003
**Success Criteria:** Logs distinguishable

### Increment 5: Fluke/Puppy.mp3 System Test
**Objective:** Verify primary use case works
**Deliverables:**
- Update test baseline
- Run single-album test
- Verify 8/11 tracks matched
**Tests:** TC-S-PAM-001
**Success Criteria:** Puppy.mp3 matches with 8 MBIDs

### Increment 6: Full 200-Album Regression Test
**Objective:** Verify no regressions
**Deliverables:**
- Run full test suite
- Compare against baseline
- Generate regression report
**Tests:** TC-S-PAM-002
**Success Criteria:** Zero regressions, Puppy.mp3 now matches

---

## Test Coverage Summary

**Total Tests:** 12 (6 unit, 3 integration, 2 system, 1 manual)
**Coverage:** 100% - All 8 requirements have acceptance tests

**Traceability:** Complete matrix in `02_test_specifications/traceability_matrix.md`

---

## Risk Assessment

**Residual Risk:** LOW

**Top Risks:**
1. **Regression on existing matches** - Mitigated by isolated module design and comprehensive testing
2. **Quality threshold edge cases** - Mitigated by using existing quality scoring

**Full Risk Analysis:** See `03_approach_selection.md`

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document technical debt.

---

## Success Metrics

**Quantitative:**
- Fluke/Puppy.mp3 matches with 8/11 tracks
- 8 valid Recording MBIDs returned
- Zero regressions on 187 matched albums
- Performance impact <5%

**Qualitative:**
- Clear distinction between partial and full matches in logs
- Maintainable, isolated code module

---

## Dependencies

**Existing Code (Read-Only Reference):**
- `wkmp-ai/src/matching/editions/filtering.rs` - Duration filter
- `wkmp-ai/src/matching/orchestrator.rs` - Match orchestration
- `wkmp-ai/src/matching/album_matcher.rs` - AlbumMatchResult

**Files to Modify:**
- `wkmp-ai/src/matching/mod.rs` - Add module
- `wkmp-ai/src/matching/orchestrator.rs` - Integration
- `wkmp-ai/src/matching/album_matcher.rs` - Result struct

**New Files:**
- `wkmp-ai/src/matching/partial_matching.rs` - New module

**No External Dependencies**

---

## Constraints

**Technical:**
- Must not break existing full-match logic
- Must use existing boundary detection algorithms
- No additional MusicBrainz API calls

**Process:**
- Full regression test required before merge
- Performance measurement required

---

## Next Steps

### Immediate (Ready Now)
1. Begin Increment 1: Create partial_matching.rs module
2. Implement core detection functions
3. Write unit tests

### Implementation Sequence
1. Increment 1: Detection functions
2. Increment 2: Result type extensions
3. Increment 3: Pipeline integration
4. Increment 4: Logging
5. Increment 5: Puppy.mp3 test
6. Increment 6: Full regression

### After Implementation
1. Execute Phase 9: Post-Implementation Review
2. Generate technical debt report
3. Run all 12 tests
4. Verify traceability matrix 100% complete
5. Create final implementation report
6. Archive plan using `/archive-plan PLAN028`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- `requirements_index.md` - All requirements with priorities
- `01_specification_issues.md` - Phase 2 analysis

**Test Specifications:**
- `02_test_specifications/test_index.md` - All tests quick reference
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests mapping

**Implementation:**
- `03_approach_selection.md` - Approach decision
- `04_increments/increment_01.md` through `increment_06.md`

**For Implementation:**
- Read this summary (~200 lines)
- Read current increment specification (~80 lines)
- Read relevant test specs (~50-100 lines)
- **Total context:** ~350-400 lines per increment

---

## Plan Status

**Phase 1-3 Status:** Complete
**Phases 4-8 Status:** Complete
**Current Status:** Ready for Implementation

---

## Approval and Sign-Off

**Plan Created:** 2026-01-26
**Plan Status:** Ready for Implementation Review

**Next Action:** Review plan, approve, begin Increment 1 implementation

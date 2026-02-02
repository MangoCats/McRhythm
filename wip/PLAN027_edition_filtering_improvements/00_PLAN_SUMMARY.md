# PLAN027: Edition Filtering Improvements - PLAN SUMMARY

**Status:** Ready for Implementation
**Created:** 2026-01-16
**Specification Source:** wkmp-ai/problem_albums_analysis.md
**Plan Location:** `wip/PLAN027_edition_filtering_improvements/`

---

## READ THIS FIRST

This document provides executive summary of edition filtering improvements to fix 3 out of 4 remaining problem albums in MusicBrainz matching.

**For Implementation:**
- Read this summary
- Review detailed requirements: `requirements_index.md`
- Review test specifications: `02_test_specifications/test_index.md`
- Follow traceability matrix: `02_test_specifications/traceability_matrix.md`

**Context Window Budget:**
- This summary: ~450 lines
- Requirements index: ~180 lines
- Critical test specs: ~200 lines per test
- **Total for implementation:** ~630-830 lines (not 2000+)

---

## Executive Summary

### Problem Being Solved

After progressive RMS optimization improved boundary detection, 4 albums remain with ≥2 tracks having ≥30s duration errors:
1. **Chemical Brothers - Surrender:** Deluxe edition with extended remixes (-83s on one track)
2. **Imagine Dragons - Night Visions:** 16-track deluxe matched to 11-track file
3. **Michael Jackson - Thriller:** Compilation matched to 9-track standard
4. **James Gang - Funk #49:** Multi-artist compilation wrongly matched

**Root Cause:** Wrong MBID/edition selection, NOT boundary detection failure.

### Solution Approach

Implement 5 algorithmic improvements to edition selection:
1. **Edition preference scoring:** Penalize deluxe (0.7×) and compilation (0.6×) editions
2. **Track count pre-filtering:** Filter editions with >±3 track difference BEFORE matching
3. **Artist consistency validation:** Reject multi-artist compilations (>3 unique artists)
4. **Remix track tolerance:** Accept up to 120s error for tracks with "remix" keywords
5. **Zero regression validation:** Tune parameters to ensure 0 regressions on 186 currently-passing albums

**Expected Impact:** Fix 3/4 problem albums (75% improvement), zero regressions

### Implementation Status

**Phases 1-3 Complete:**
- ✅ Phase 1: Scope Definition - 5 requirements + 1 regression req extracted
- ✅ Phase 2: Specification Verification - 8 issues found (1 Critical, 3 High, 3 Medium, 1 Low)
- ✅ Phase 3: Test Definition - 36 tests defined, 100% coverage

**Phases 4-8 Status:** N/A (Phases 1-3 sufficient for implementation)

---

## Requirements Summary

**Total Requirements:** 6 (2 Critical, 2 High, 1 Medium, 1 Critical validation)

| Req ID | Priority | Brief Description |
|--------|----------|-------------------|
| REQ-EF-010 | Critical | Edition preference scoring - deluxe penalty (0.7×) |
| REQ-EF-020 | Critical | Edition preference scoring - compilation penalty (0.6×) |
| REQ-EF-030 | High | Track count pre-filtering (±3 tracks tolerance) |
| REQ-EF-040 | High | Artist consistency validation (reject >3 unique artists) |
| REQ-EF-050 | Medium | Remix track error tolerance (up to 120s) |
| REQ-EF-060 | Critical | Zero regression validation (≥186/200 albums pass) |

**Full Requirements:** See `requirements_index.md`

---

## Scope

### ✅ In Scope

**1. Edition Preference Scoring** (Critical)
- Penalize deluxe editions: 0.7× multiplier
- Penalize compilation editions: 0.6× multiplier
- Case-insensitive keyword matching
- **Parameters:** Tunable via TC-S-060-02 (may adjust based on regression testing)

**2. Track Count Pre-Filtering** (High)
- Filter editions where |track_count - detected| > 3
- Apply BEFORE Stage 2 (performance optimization)
- Fallback to unfiltered list if all editions rejected
- **Parameters:** Tunable via TC-S-060-03

**3. Artist Consistency Validation** (High)
- Extract unique artists from edition tracks
- Reject if >3 unique artists (multi-artist compilations)
- Fuzzy artist name matching (substring containment)

**4. Remix Track Tolerance** (Medium)
- Detect remix keywords: "remix", "extended", "mix)", "version"
- Accept up to 120s error for remix tracks
- Standard tolerance still applies to non-remix

**5. Zero Regression Validation** (Critical)
- Full 200-album regression test suite
- Parameter tuning (multipliers, tolerance)
- Baseline comparison (≥186/200 must pass)

### ❌ Out of Scope

- Database schema changes (filtering is in-memory)
- API changes (internal matching logic only)
- UI changes (no user configuration)
- Re-matching existing albums (user must re-import)
- Advanced edition selection (ML, genre-based, etc.)

**Full Scope:** See `scope_statement.md`

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 1 (No regression testing requirement → added REQ-EF-060)
- **HIGH Issues:** 3 (Penalty multipliers not validated, track count tolerance not validated, fallback behavior incomplete)
- **MEDIUM Issues:** 3 (Fuzzy artist matching algorithm, remix keywords, performance constraint)
- **LOW Issues:** 1 (Edition title keywords not exhaustive)

**Decision:** ✅ PROCEED - All critical issues resolved (REQ-EF-060 added), high issues addressed in test design

**Full Analysis:** See `01_specification_issues.md`

---

## Test Coverage Summary

**Total Tests:** 36 (20 unit, 8 integration, 8 system)
**Coverage:** 100% - All 6 requirements have acceptance tests
**Traceability:** Complete matrix in `02_test_specifications/traceability_matrix.md`

### Test Breakdown by Requirement

| Requirement | Unit | Integration | System | Total |
|-------------|------|-------------|--------|-------|
| REQ-EF-010 (Deluxe penalty) | 4 | 1 | 1 | 6 |
| REQ-EF-020 (Compilation penalty) | 4 | 1 | 1 | 6 |
| REQ-EF-030 (Track count filter) | 5 | 3 | 1 | 9 |
| REQ-EF-040 (Artist consistency) | 4 | 2 | 1 | 7 |
| REQ-EF-050 (Remix tolerance) | 3 | 1 | 1 | 5 |
| REQ-EF-060 (Zero regressions) | 0 | 0 | 3 | 3 |

### Critical System Tests

**TC-S-060-01:** Full regression suite (200 albums, ≥186 must pass)
**TC-S-060-02:** Parameter tuning - penalty multipliers (find optimal 0.7/0.6 or adjust)
**TC-S-060-03:** Parameter tuning - track count tolerance (validate ±3 or adjust)

**Estimated Test Execution Time:** 9-10 hours (unit + integration + system)

---

## Implementation Roadmap

### Module 1: Edition Filter Functions
**Objective:** Create edition_filter.rs module with core filtering logic
**Effort:** 4-6 hours
**Deliverables:**
- `filter_by_track_count()` - Track count pre-filtering
- `validate_artist_consistency()` - Multi-artist rejection
- `score_edition_preference()` - Deluxe/compilation penalties
- `is_remix_track()` - Remix keyword detection
**Tests:** TC-U-010-01 through TC-U-050-03 (20 unit tests)
**Success Criteria:** All 20 unit tests pass

### Module 2: Integration with Stage 2
**Objective:** Apply filtering before Stage 2 matching
**Effort:** 2-3 hours
**Deliverables:**
- Modify `stage2_album_first.rs` to call filtering functions
- Add logging for filter decisions
- Handle edge cases (empty lists, fallback)
**Tests:** TC-I-010-01 through TC-I-050-01 (8 integration tests)
**Success Criteria:** All 8 integration tests pass, filtering occurs before matching

### Module 3: Parameter Tuning
**Objective:** Find optimal multiplier and tolerance values with zero regressions
**Effort:** 6-10 hours
**Deliverables:**
- Penalty multiplier tuning (grid search, 36 combinations)
- Track count tolerance tuning (sequential search, 3-5 tests)
- Tuning reports with justifications
**Tests:** TC-S-060-02, TC-S-060-03
**Success Criteria:** Zero-regression configuration found for both parameters

### Module 4: Full Regression Validation
**Objective:** Verify zero regressions on full 200-album suite
**Effort:** 4-6 hours
**Deliverables:**
- Full test run with tuned parameters
- Regression report comparing to baseline
- Problem album status (3/4 fixed)
**Tests:** TC-S-060-01, TC-S-010-01 through TC-S-050-01
**Success Criteria:** ≥186/200 albums pass, 3/4 problem albums fixed

**Total Estimated Effort:** 16-25 hours (2-3 days elapsed)

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Security issues
- Deferred requirements

See Phase 9 section of /plan.md for 7-step technical debt discovery process.

---

## Success Metrics

### Quantitative Metrics

✅ **Albums Fixed:** 3/4 problem albums
- Imagine Dragons - Night Visions: 11-track standard selected (was 16-track deluxe)
- Michael Jackson - Thriller: 9-track standard selected (was compilation)
- James Gang - Funk #49: Multi-artist compilation rejected
- Chemical Brothers - Surrender: Remix tolerance applied (already acceptable)

✅ **Zero Regressions:** ≥186/200 albums pass (baseline: 186/200)

✅ **MBID Stability:** ≥90% albums select same MBID as baseline (≤20 changes)

✅ **Performance:** Edition filtering <100ms per album, Stage 2 processes 20-30% fewer editions

### Qualitative Metrics

✅ **Code Quality:** Edition filtering logic is modular, testable, DRY
✅ **Logging:** Clear filtering decisions for debugging
✅ **Maintainability:** Easy to add new keywords or adjust penalties
✅ **User Experience:** Transparent improvements (no config changes required)

---

## Dependencies

### Existing Code (No Changes Required)

- `wkmp-ai/src/services/musicbrainz_client.rs` - Edition metadata fetching
- `wkmp-ai/src/matching/mod.rs` - Matching pipeline orchestration
- `wkmp-ai/src/matching/stages/boundary_refinement.rs` - Progressive RMS (just completed)

### Code to Create

- `wkmp-ai/src/matching/edition_filter.rs` - New module (~300-400 lines)
- Unit tests in same file or separate test file

### Code to Modify

- `wkmp-ai/src/matching/stages/stage2_album_first.rs` - Apply pre-filtering (~10-15 lines added)

### Test Data

- Full 200-album library: C:\Users\Mango Cat\Music
- Baseline comparison: baseline_1-12-26_stats.json
- 4 problem albums for validation

**Dependencies Status:** ✅ All satisfied, no external libraries needed

**Full Dependencies:** See `dependencies_map.md`

---

## Constraints

### Technical Constraints

- **Performance:** Edition filtering <100ms per album
- **Zero Regressions:** Mandatory (user-specified)
- **No Database Changes:** Filtering is in-memory only
- **No API Changes:** Internal matching logic only

### Process Constraints

- **Testing:** Full 200-album test suite required before committing
- **Parameter Tuning:** Empirical validation required for multipliers and tolerance
- **Regression Detection:** Any regression blocks deployment until resolved

### Timeline Constraints

- **Implementation:** 1-2 days (14-20 hours)
- **Testing:** 4-6 hours (3× full test runs)
- **Total:** 3-4 days elapsed time

---

## Risk Assessment

**Residual Risk:** Low

**Risks Identified:**
1. **Penalty multipliers too aggressive** → Mitigated by parameter tuning (TC-S-060-02)
2. **Track count tolerance too strict** → Mitigated by parameter tuning (TC-S-060-03)
3. **Unforeseen regressions** → Mitigated by full regression suite (TC-S-060-01)

**Mitigation Strategy:**
- Tune parameters empirically (don't trust initial estimates)
- Test with baseline comparison (catch regressions early)
- Accept fewer fixes if needed to achieve zero regressions (user mandate)

---

## Next Steps

### Immediate (Ready Now)

1. **Create edition_filter.rs module**
   - Implement 4 core functions
   - Add unit tests inline
   - Verify all 20 unit tests pass

2. **Integrate with Stage 2**
   - Modify stage2_album_first.rs
   - Add logging
   - Verify 8 integration tests pass

3. **Parameter Tuning**
   - Run TC-S-060-02 (multipliers)
   - Run TC-S-060-03 (tolerance)
   - Document optimal values

4. **Full Regression Validation**
   - Run TC-S-060-01 with tuned parameters
   - Verify ≥186/200 pass, 3/4 problems fixed
   - Generate regression report

### Implementation Sequence

1. Module 1: Core filtering functions (4-6 hours)
2. Module 2: Stage 2 integration (2-3 hours)
3. Module 3: Parameter tuning (6-10 hours)
4. Module 4: Full regression validation (4-6 hours)

**Total:** 16-25 hours over 2-3 days

### After Implementation

1. Execute Phase 9: Post-Implementation Review (MANDATORY)
2. Generate technical debt report
3. Run all 36 tests (verify 100% pass)
4. Create implementation completion report
5. Archive plan using `/archive-plan PLAN027`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- `requirements_index.md` - All 6 requirements with acceptance criteria (~180 lines)
- `scope_statement.md` - In/out scope, assumptions, constraints (~200 lines)
- `dependencies_map.md` - Code dependencies, integration points (~150 lines)
- `01_specification_issues.md` - Phase 2 analysis (8 issues) (~280 lines)

**Test Specifications:**
- `02_test_specifications/test_index.md` - All 36 tests quick reference (~140 lines)
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests mapping (~150 lines)
- Critical tests:
  - `system_tests/tc_s_060_01_full_regression.md` - Full regression suite (~220 lines)
  - `system_tests/tc_s_060_02_parameter_tuning_multipliers.md` - Multiplier tuning (~200 lines)
  - `system_tests/tc_s_060_03_parameter_tuning_tolerance.md` - Tolerance tuning (~190 lines)

**For Implementation:**
- Read this summary (~450 lines)
- Read requirements_index.md (~180 lines)
- Read relevant test specs (~200 lines per module)
- **Total context:** ~630-830 lines per module (not 2000+)

---

## Plan Status

**Phase 1-3 Status:** ✅ Complete
**Phases 4-8 Status:** N/A (not required for this plan)
**Current Status:** Ready for Implementation
**Estimated Timeline:** 16-25 hours over 2-3 days

---

## Approval and Sign-Off

**Plan Created:** 2026-01-16
**Plan Status:** Ready for Implementation

**Critical Constraints:**
- ✅ Zero regressions mandatory (user-specified)
- ✅ Parameters must be tuned empirically (user-specified)
- ✅ Full regression suite required before deployment

**Next Action:** Begin implementation with Module 1 (edition_filter.rs core functions)

---

## Key Constraints Summary

**USER MANDATE:** "Regression in other albums is not acceptable"
- Parameters (multipliers, tolerance) MUST be validated through testing
- If initial values (0.7, 0.6, ±3) cause regressions, MUST tune until zero regressions achieved
- Accept fewer problem album fixes if necessary to maintain zero regressions

**Implementation Note:** Do NOT hardcode parameters. Make tunable constants so testing can iterate values.

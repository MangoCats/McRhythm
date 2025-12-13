# PLAN026: Single Song Import Improvements - PLAN SUMMARY

**Status:** Ready for Implementation
**Created:** 2025-12-13
**Specification Source:** Analysis of wkmp-ai single-song import gaps
**Plan Location:** `wip/PLAN026_single_song_import_improvements/`

---

## READ THIS FIRST

This plan addresses the gap between wkmp-ai's well-optimized album import path and its under-developed single-song import path. The core improvement is adding MusicBrainz recording search as a fallback when AcoustID fails.

**For Implementation:**
- Read this summary (~400 lines)
- Review detailed requirements: `requirements_index.md`
- Review test specifications: `02_test_specifications/test_index.md`
- Follow traceability matrix: `02_test_specifications/traceability_matrix.md`

**Context Window Budget:** ~650-800 lines per increment

---

## Executive Summary

### Problem Being Solved

The single-song import path relies entirely on AcoustID for song identification. When AcoustID fails (timeout, no match, low confidence), files are marked `NOT_IN_MUSICBRAINZ` even when ID3 metadata could identify them via direct MusicBrainz recording search.

**Impact:** Many legitimate songs fail import unnecessarily.

### Solution Approach

1. Add MusicBrainz recording search as fallback when AcoustID fails
2. Upgrade metadata validation from Levenshtein to Jaro-Winkler (already proven in album matcher)
3. Integrate existing IdentityResolver for multi-source Bayesian fusion
4. Add caching to prevent redundant API calls

### Implementation Status

**Phases 1-8 Complete:**
- ✅ Phase 1: Scope Definition - 20 requirements extracted
- ✅ Phase 2: Specification Verification - 0 Critical, 2 High, 3 Medium issues
- ✅ Phase 3: Test Definition - 20 tests defined, 100% coverage
- ✅ Phase 4: Approach Selection - Extend ContentTypeClassifier
- ✅ Phase 5: Implementation Breakdown - 6 increments defined
- ✅ Phase 8: Plan Summary (this document)

---

## Requirements Summary

**Total Requirements:** 20 (8 P0, 8 P1, 4 P2)

| Priority | Count | In Scope |
|----------|-------|----------|
| P0 (Critical) | 8 | ✅ All |
| P1 (High) | 8 | ✅ All |
| P2 (Medium) | 4 | ❌ Deferred |

**Key Requirements:**
- SSI-MB-010: MusicBrainz recording search fallback (P0)
- SSI-VAL-010: Jaro-Winkler validation (P0)
- SSI-FUS-010: IdentityResolver integration (P1)
- SSI-INT-030: Recording cache (P1)

**Full Requirements:** See `requirements_index.md`

---

## Scope

### ✅ In Scope

- MusicBrainz recording search fallback
- Jaro-Winkler string similarity (port from am29)
- Multi-source identity fusion
- Recording lookup caching
- Unit and integration tests

### ❌ Out of Scope

- Folder context awareness (P2 - deferred)
- Per-segment fingerprinting (P2 - separate PLAN025)
- ML-based classification (research-level)

**Full Scope:** See `scope_statement.md`

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 0
- **HIGH Issues:** 2 (resolvable)
- **MEDIUM Issues:** 3 (documented)
- **LOW Issues:** 2 (tracked)

**Decision:** PROCEED - No blockers

**Full Analysis:** See `01_specification_issues.md`

---

## Implementation Roadmap

### Increment 1: String Similarity Utilities
**Objective:** Port Jaro-Winkler and normalization from am29
**Effort:** 2-3 hours
**Deliverables:**
- `src/utils/string_similarity.rs` (~150 lines)
**Tests:** TC-U-VAL-010-01, TC-U-VAL-010-02, TC-U-VAL-020-01, TC-U-VAL-020-02

### Increment 2: Recording Matcher Service
**Objective:** Create service to query MusicBrainz recordings
**Effort:** 3-4 hours
**Deliverables:**
- `src/services/recording_matcher.rs` (~200 lines)
**Tests:** TC-U-MB-010-01, TC-U-MB-010-02, TC-U-MB-020-01, TC-U-MB-020-02, TC-U-MB-030-01

### Increment 3: Recording Cache
**Objective:** Add database caching for recording lookups
**Effort:** 2 hours
**Deliverables:**
- `src/db/recording_cache.rs` (~80 lines)
**Tests:** TC-I-INT-030-01

### Increment 4: ContentTypeClassifier Integration
**Objective:** Integrate recording matcher as fallback
**Effort:** 3-4 hours
**Deliverables:**
- Modified `src/services/content_type_classifier.rs` (~100 lines)
**Tests:** TC-U-MB-010-01, TC-U-MB-010-02, TC-I-INT-010-01, TC-I-FUS-010-01

### Increment 5: Multi-Source Fusion Integration
**Objective:** Full Bayesian fusion with IdentityResolver
**Effort:** 2-3 hours
**Deliverables:**
- Modified `src/services/content_type_classifier.rs` (~80 lines)
**Tests:** TC-I-FUS-020-01, TC-I-FUS-020-02, TC-U-FUS-030-01

### Increment 6: Integration Tests and E2E Verification
**Objective:** Comprehensive test coverage
**Effort:** 2-3 hours
**Deliverables:**
- `tests/single_song_integration_tests.rs` (~200 lines)
**Tests:** TC-S-E2E-01, TC-S-E2E-02, TC-I-INT-020-01

**Total Estimated Effort:** 14-20 hours

---

## Test Coverage Summary

**Total Tests:** 20 (12 unit, 6 integration, 2 system)
**Coverage:** 100% - All 15 in-scope requirements have acceptance tests

| Category | Tests |
|----------|-------|
| Unit (TC-U-*) | 12 |
| Integration (TC-I-*) | 6 |
| System (TC-S-*) | 2 |

**Traceability:** Complete matrix in `02_test_specifications/traceability_matrix.md`

---

## Risk Assessment

**Residual Risk:** Low

**Top Risks:**
1. MusicBrainz rate limiting delays import - Mitigation: Cache aggressively
2. Jaro-Winkler false positives - Mitigation: Require both artist+title match
3. Regression in album path - Mitigation: Comprehensive test suite

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Deferred requirements

---

## Dependencies

**Existing Code (Reuse):**
- `MusicBrainzClient.search_recordings()` - Already implemented
- `IdentityResolver` - Ready for integration
- `strsim` crate - Jaro-Winkler implementation

**New Files:**
- `src/utils/string_similarity.rs`
- `src/services/recording_matcher.rs`
- `src/db/recording_cache.rs`
- `tests/single_song_integration_tests.rs`

---

## Constraints

**Technical:**
- MusicBrainz: 1 request/second rate limit
- AcoustID: 1 second timeout (circuit breaker)
- No schema changes (use existing tables + new cache table)

**Process:**
- No breaking changes to album import
- ≥80% test coverage for new code

---

## Next Steps

### Immediate (Ready Now)
1. Create `src/utils/string_similarity.rs` (Increment 1)
2. Run `cargo test string_similarity` to verify

### Implementation Sequence
1. Increment 1: String utilities → Checkpoint 1
2. Increment 2: Recording matcher → Checkpoint 1
3. Increment 3: Cache → Checkpoint 2
4. Increment 4: ContentTypeClassifier → Checkpoint 2
5. Increment 5: Multi-source fusion → Checkpoint 3
6. Increment 6: Integration tests → Checkpoint 3

### After Implementation
1. Execute Phase 9: Post-Implementation Review
2. Generate technical debt report
3. Run all 20 tests
4. Verify traceability matrix 100% complete
5. Archive plan using `/archive-plan PLAN026`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- `requirements_index.md` - All requirements with priorities
- `scope_statement.md` - In/out scope, assumptions, constraints
- `01_specification_issues.md` - Phase 2 analysis

**Test Specifications:**
- `02_test_specifications/test_index.md` - All tests quick reference
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests mapping

**Implementation Details:**
- `03_approach_selection.md` - ADR for selected approach
- `04_increments/increment_01.md` through `increment_06.md`
- `04_increments/checkpoints.md` - Verification points

**Context Budget:**
- Summary + Increment: ~650-800 lines
- DO NOT read all increments at once

---

## Plan Status

**Phase 1-8 Status:** Complete
**Current Status:** Ready for Implementation
**Estimated Timeline:** 14-20 hours

---

## Approval and Sign-Off

**Plan Created:** 2025-12-13
**Plan Status:** Ready for Implementation Review

**Next Action:** Begin Increment 1 (String Similarity Utilities)

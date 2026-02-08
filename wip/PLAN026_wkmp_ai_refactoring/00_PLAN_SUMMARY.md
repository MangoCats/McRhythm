# PLAN026: wkmp-ai Major Refactoring - PLAN SUMMARY

**Status:** Ready for Implementation
**Created:** 2025-11-26
**Specification Source:** [wkmp-ai/refactor1126.md](../../wkmp-ai/refactor1126.md)
**Plan Location:** `wip/PLAN026_wkmp_ai_refactoring/`

---

## READ THIS FIRST

This plan implements a major refactoring of wkmp-ai to integrate the am28 album matching algorithm. The specification was analyzed using /think before /plan - all gaps, ambiguities, and conflicts have been resolved.

**For Implementation:**
- Read this summary first
- Review: `requirements_index.md` (39 requirements)
- Review: `02_test_specifications/test_index.md` (52 tests)
- Follow: `02_test_specifications/traceability_matrix.md`

**Context Window Budget:**
- This summary: ~400 lines
- Requirements index: ~200 lines
- Test index: ~150 lines
- Per-increment: ~200 lines

---

## Executive Summary

### Problem Being Solved

The current wkmp-ai implementation has:
- Overly complex passage identification that doesn't work reliably
- Tightly coupled code that's difficult to maintain
- Poor MusicBrainz integration for album identification

The am28 example (`wkmp-ai/examples/am28/`) has proven highly effective through 28 test runs, achieving reliable album matching with:
- Silence-based track boundary detection
- MusicBrainz edition testing (Stages 2-5)
- Artist verification and mismatch handling
- Configurable confidence thresholds

### Solution Approach

Integrate am28 into wkmp-ai while preserving valuable existing components:
1. Keep: File scanner, magic byte reader, SSE UI updates, db_access pattern, zero-conf settings
2. Replace: Passage identification, content type determination
3. Add: New database tables (Works, Albums), relationship tables, import metadata

### Implementation Status

**Phases 1-3 Complete:**
- ✅ Phase 1: Scope Definition - 39 requirements extracted
- ✅ Phase 2: Specification Verification - 0 blocking issues (22 previously resolved)
- ✅ Phase 3: Test Definition - 52 tests defined, 100% coverage

**Phases 4-8:** Pending (per /plan Week 1 deliverable)

---

## Requirements Summary

**Total Requirements:** 39

| Category | Count | Priority |
|----------|-------|----------|
| Pipeline Steps (REQ-STEP-*) | 7 | P0 |
| Database Schema (REQ-DB-*) | 9 | P0 |
| Algorithm (REQ-ALG-*) | 14 | P0/P1 |
| Classification (REQ-CLASS-*) | 6 | P0 |
| Integration (REQ-INT-*) | 5 | P1 |
| Architecture (REQ-ARCH-*) | 4 | P0 |

**Key Requirements:**
- REQ-STEP-001..007: 7-step pipeline (scan → classify)
- REQ-DB-001..009: Database schema extensions
- REQ-ALG-001..014: Algorithm thresholds and behavior
- REQ-CLASS-001..006: 6 content type classifications

**Full Requirements:** See `requirements_index.md`

---

## Scope

### ✅ In Scope

- 7-step pipeline: Folder scan → Content type determination
- Database schema: Files, Passages, Songs, Works, Artists, Albums, relationships
- am28 algorithm: Stages 2-5 edition testing, silence detection
- Classification: SINGLE_SONG, FULL_ALBUM, PARTIAL_ALBUM, MULTIPLE_SONGS, NOT_IN_MUSICBRAINZ, IDENTIFICATION_FAILED
- Architecture: Preserve db_access, SSE, zero-conf patterns

### ❌ Out of Scope

- Folder-level album detection (GAP-01 documented, defer to future)
- Essentia integration (placeholder only)
- Lyric/cover art extraction
- wkmp-ap, wkmp-pd, wkmp-ui changes

**Full Scope:** See `scope_statement.md`

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 0 (GAP-01 resolved, deferred)
- **HIGH Issues:** 0 (all resolved)
- **MEDIUM Issues:** 0 (all resolved)
- **LOW Issues:** 0

**Decision:** PROCEED - No blocking issues

**Prior Analysis:** /think analysis completed before /plan, documented in [refactor1126_analysis.md](../../wkmp-ai/refactor1126_analysis.md)

**Full Analysis:** See `01_specification_issues.md`

---

## Implementation Roadmap (Proposed)

### Increment 1: Database Schema Foundation
**Objective:** Create/extend all required database tables
**Deliverables:**
- Files table extensions (content_type, match_*, etc.)
- Passages table with tick-based timing
- Songs, Works, Artists, Albums tables
- Relationship tables (passage_songs, song_artists, passage_albums)
**Tests:** TC-U-DB-001..007, TC-I-DB-001..003
**Estimated Effort:** 4-6 hours

### Increment 2: Pipeline Steps 1-3
**Objective:** Implement folder scan, known check, hash computation
**Deliverables:**
- Magic byte scanner (reuse existing)
- Known files check against database
- SHA-256 hash computation
- Duplicate detection logic
**Tests:** TC-U-STEP-001..003, TC-I-STEP-001..003
**Estimated Effort:** 4-6 hours

### Increment 3: Pipeline Steps 4-5
**Objective:** Metadata extraction and audio decoding
**Deliverables:**
- Metadata extraction (ID3, Vorbis, MP4, APE, BWF, WMA)
- Audio decoding via symphonia
- NO AUDIO detection
**Tests:** TC-U-STEP-004..005, TC-I-STEP-004..005
**Estimated Effort:** 4-6 hours

### Increment 4: Content Type Determination - Single Song Path
**Objective:** Implement single-song classification via AcoustID
**Deliverables:**
- Duration-based triage
- Chromaprint fingerprinting
- AcoustID lookup
- SINGLE_SONG classification
**Tests:** TC-U-ALG-001..003, TC-U-CLASS-001
**Estimated Effort:** 6-8 hours

### Increment 5: Content Type Determination - Album Path
**Objective:** Integrate am28 album matching
**Deliverables:**
- Metadata reconciliation
- MusicBrainz release search
- Edition testing (Stages 2-5)
- FULL_ALBUM, PARTIAL_ALBUM classification
**Tests:** TC-I-ALG-001..002, TC-U-CLASS-002..003, TC-S-ALG-001
**Estimated Effort:** 8-12 hours

### Increment 6: Result Integration & Entity Creation
**Objective:** Create database records from matching results
**Deliverables:**
- ValidationResult → Files + Passages mapping
- Song/Artist/Album creation from Edition
- Relationship table population
- Import metadata JSON
**Tests:** TC-I-DB-002..003, TC-I-CLASS-001..002
**Estimated Effort:** 4-6 hours

### Increment 7: Fallback & Edge Cases
**Objective:** Handle remaining classifications
**Deliverables:**
- MULTIPLE_SONGS handling
- NOT_IN_MUSICBRAINZ handling
- IDENTIFICATION_FAILED handling
- Completed duplicate handling
**Tests:** TC-U-STEP-007, TC-S-CLASS-001..002
**Estimated Effort:** 4-6 hours

### Increment 8: Integration & System Testing
**Objective:** End-to-end verification
**Deliverables:**
- Full pipeline integration
- System test execution
- SSE progress reporting verification
- Documentation updates
**Tests:** TC-S-STEP-001..002, TC-S-ALG-002
**Estimated Effort:** 4-6 hours

**Total Estimated Effort:** 38-56 hours

---

## Test Coverage Summary

**Total Tests:** 52
- Unit Tests: 34
- Integration Tests: 12
- System Tests: 6

**Coverage:** 100% - All 39 requirements have acceptance tests

**Traceability:** Complete matrix in `02_test_specifications/traceability_matrix.md`

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Security issues
- Deferred requirements

See Phase 9 section of /plan workflow for 7-step technical debt discovery process.

---

## Dependencies

### Existing Code (Keep)
- wkmp-ai/src/scanner.rs (file scanner)
- wkmp-ai/src/magic.rs (magic byte reader)
- wkmp-ai/src/sse.rs (SSE progress)
- wkmp-ai/src/db_access.rs (db thread pattern)

### New Code (Integrate)
- wkmp-ai/examples/am28/ → wkmp-ai/src/matching/

### External Dependencies
- symphonia 0.5.x (audio decoding)
- lofty 0.18.x (metadata)
- sha2 0.10.x (hashing)
- reqwest 0.11.x (HTTP)

---

## Constraints

**Technical:**
- SQLite single-writer (db_access pattern)
- SPEC017 tick-based timing
- SPEC002 six timing points

**API Rate Limits:**
- MusicBrainz: 1 req/sec
- AcoustID: 3 req/sec

**Quality:**
- High confidence: ≥80%
- Match tolerance: 3.0 seconds
- Artist similarity: ≥50% Jaro-Winkler

---

## Next Steps

### Immediate (Ready Now)
1. Review this plan summary
2. Verify scope alignment with expectations
3. Approve plan for implementation

### Implementation Sequence
1. Database schema (Increment 1) - Foundation
2. Pipeline steps 1-3 (Increment 2) - File processing
3. Pipeline steps 4-5 (Increment 3) - Metadata/decode
4. Single-song path (Increment 4) - AcoustID
5. Album path (Increment 5) - am28 integration
6. Result integration (Increment 6) - Entity creation
7. Edge cases (Increment 7) - Fallback handling
8. System testing (Increment 8) - E2E verification

### After Implementation
1. Execute Phase 9: Post-Implementation Review (MANDATORY)
2. Generate technical debt report
3. Run all 52 tests
4. Verify traceability matrix 100% complete
5. Archive plan using `/archive-plan PLAN026`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- `requirements_index.md` - All 39 requirements with priorities
- `scope_statement.md` - In/out scope, assumptions, constraints
- `01_specification_issues.md` - Phase 2 analysis (all resolved)

**Test Specifications:**
- `02_test_specifications/test_index.md` - All 52 tests quick reference
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests mapping
- `02_test_specifications/tc_*.md` - Individual test specs

**For Implementation:**
- Read this summary (~400 lines)
- Read current increment details (when Phase 5 complete)
- Read relevant test specs (~100 lines)
- **Total context:** ~600-700 lines per increment

---

## Plan Status

**Phase 1-3 Status:** Complete
**Phases 4-8 Status:** Pending (Week 2-3 deliverables per /plan workflow)
**Current Status:** Ready for Implementation Review
**Estimated Timeline:** 38-56 hours over ~2 weeks

---

## Approval and Sign-Off

**Plan Created:** 2025-11-26
**Plan Status:** Ready for Implementation Review

**Next Action:** User review and approval to proceed with implementation

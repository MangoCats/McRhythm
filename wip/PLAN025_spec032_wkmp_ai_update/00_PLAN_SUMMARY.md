# PLAN025: SPEC032 wkmp-ai Update - PLAN SUMMARY

**Status:** Ready for Implementation (Phases 1-3 Complete)
**Created:** 2025-11-10
**Specification Source:** wip/SPEC032_IMPLEMENTATION_UPDATE.md
**Plan Location:** `wip/PLAN025_spec032_wkmp_ai_update/`

---

## READ THIS FIRST

This document provides a comprehensive overview of the SPEC032 wkmp-ai implementation update plan. The plan addresses a fundamental architectural shift from fingerprint-first to segmentation-first import pipeline with evidence-based MBID identification.

**For Implementation:**
- Read this summary first (~450 lines)
- Review requirements: `requirements_index.md`
- Review test specifications: `02_test_specifications/test_index.md`
- Follow traceability matrix: `02_test_specifications/traceability_matrix.md`
- Implement increment-by-increment (Phase 5 TBD - Week 2)

**Context Window Budget:**
- This summary: ~450 lines
- Requirements index: ~250 lines
- Test index: ~180 lines
- **Total planning context:** ~880 lines (optimal for implementation)

---

## Executive Summary

### Problem Being Solved

The current wkmp-ai implementation uses a **fingerprint-first, batch-phase architecture** that:
1. **Fingerprints entire files BEFORE segmentation** - Missing structural clues (track count, gap patterns)
2. **Processes all files through each phase sequentially** - Poor resource utilization, coarse progress granularity
3. **Lacks contextual matching** - No metadata+pattern combination to narrow MusicBrainz candidates
4. **No confidence assessment** - Binary success/fail, no evidence-based decisions
5. **Whole-file fingerprinting only** - Less accurate for multi-track album files

**Result:** Lower identification accuracy, slower imports, no graceful degradation.

### Solution Approach

Implement **segmentation-first, evidence-based architecture** with:

1. **Reordered Pipeline** - Segment → Match → Fingerprint → Identify (structural clues first)
2. **Per-File Processing** - Each file through full pipeline with 4 parallel workers
3. **Pattern Analyzer** (NEW) - Detect track count, gap patterns, source media type
4. **Contextual Matcher** (NEW) - Combine metadata + pattern → narrow MusicBrainz candidates
5. **Confidence Assessor** (NEW) - Weight evidence (30% metadata + 60% fingerprint + 10% duration)
6. **Per-Segment Fingerprinting** - Individual segment fingerprints (more accurate for albums)
7. **Tick-Based Timing** - SPEC017 compliance (sample-accurate precision)

**Result:** Higher accuracy (>90%), faster imports (better resource use), graceful degradation (zero-song passages).

### Implementation Status

**Phases 1-3 Complete (Week 1 Deliverable):**
- ✅ Phase 1: Scope Definition - 12 requirements extracted, scope boundaries defined
- ✅ Phase 2: Specification Verification - 8 issues identified (0 CRITICAL, 2 HIGH, 4 MEDIUM, 2 LOW)
- ✅ Phase 3: Test Definition - 32 tests defined, 100% requirement coverage

**Phases 4-8 Status:** Pending (Week 2-3)
- Phase 4: Approach Selection (Week 2)
- Phase 5: Implementation Breakdown (Week 2)
- Phase 6: Effort Estimation (Week 3)
- Phase 7: Risk Assessment (Week 3)
- Phase 8: Final Documentation (Week 3)

---

## Requirements Summary

**Total Requirements:** 12 functional + architectural requirements
- **P0 (Critical):** 2 requirements - Pipeline reordering (PIPE-010, PIPE-020)
- **P1 (High):** 6 requirements - New components (PATT-010, CTXM-010/020/030, CONF-010, FING-010)
- **P2 (Medium):** 4 requirements - Pattern details (PATT-020/030/040), tick timing (TICK-010)

### Critical Requirements (P0)

**REQ-PIPE-010: Segmentation-First Pipeline**
- Move segmentation BEFORE fingerprinting
- Required sequence: Verify → Extract → Hash → **SEGMENT** → Match → Fingerprint → Identify → Amplitude → Flavor → DB
- SPEC032 Lines: 344-375

**REQ-PIPE-020: Per-File Pipeline**
- Replace batch phases with per-file processing
- 4 concurrent workers via `futures::stream::buffer_unordered(4)`
- Each worker: one file through all steps → next file
- SPEC032 Lines: 232-250, 543-578

### High-Priority Requirements (P1)

**REQ-PATT-010: Pattern Analyzer Component**
- Detect track count, gap patterns (mean/std dev), segment durations, source media type
- Output: `PatternMetadata` with confidence
- Target: >80% accuracy on test dataset

**REQ-CTXM-010/020/030: Contextual Matcher Component**
- Single-segment: Artist + title → MusicBrainz (±10% duration)
- Multi-segment: Album structure → MusicBrainz releases (track count + duration filters)
- Fuzzy string matching (Jaro-Winkler, threshold 0.85)
- Target: Narrow to <10 candidates in >80% of cases

**REQ-CONF-010: Confidence Assessor Component**
- Combine evidence: 30% metadata + 60% fingerprint + 10% duration (single-segment)
- Thresholds: Accept ≥0.85, Review 0.60-0.85, Reject <0.60
- Target: >90% acceptance rate, <5% false positive rate

**REQ-FING-010: Per-Segment Fingerprinting**
- Generate Chromaprint fingerprints for EACH segment individually
- Per-segment AcoustID queries (rate-limited)
- Target: More accurate than whole-file for albums

### Medium-Priority Requirements (P2)

**REQ-PATT-020/030/040:** Pattern analyzer details (track count, gap analysis, source media)
**REQ-TICK-010:** Tick-based timing conversion per SPEC017 (all 7 timing fields)

**Full Requirements:** See `requirements_index.md`

---

## Scope

### ✅ In Scope

**Pipeline Changes:**
- Reorder sequence (segment before fingerprint)
- Convert to per-file pipeline (4 workers)

**New Components:**
- Pattern Analyzer (services/pattern_analyzer.rs)
- Contextual Matcher (services/contextual_matcher.rs)
- Confidence Assessor (services/confidence_assessor.rs)

**Architectural Changes:**
- Per-segment fingerprinting (modify services/fingerprinter.rs)
- Tick-based timing (apply to all components)

**Testing:**
- 32 tests (18 unit, 10 integration, 4 system)
- 100% requirement coverage
- Test dataset: 70 files minimum (50 single-track, 10 albums, 10 edge cases)

### ❌ Out of Scope

**Not Changing:**
- File scanner, metadata extractor, amplitude analyzer, silence detector (unchanged)
- Database schema (using existing SPEC031-compliant schema)
- UI/API interfaces (no HTTP/SSE changes)

**Deferred to Future:**
- Manual MBID review queue UI (logged only, no UI)
- Advanced fuzzy matching (basic Jaro-Winkler, not NLP)
- Machine learning (heuristics, not ML models)
- Confidence threshold configuration (hardcoded, not user-configurable)

**Explicitly Not Implementing:**
- Whole-library re-import (no migration of existing data)
- Backwards compatibility (old architecture replaced)
- A/B testing (no comparison of approaches)

**Full Scope:** See `scope_statement.md`

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 0
- **HIGH Issues:** 2
- **MEDIUM Issues:** 4
- **LOW Issues:** 2

**Decision:** ✅ **PROCEED** - No critical blockers

**Key Issues Identified:**

**HIGH-001:** Per-segment PCM extraction not fully specified
- Resolution: Document buffer strategy in implementation (Increment 3)

**HIGH-002:** Per-file vs. batch terminology ambiguous
- Resolution: Clarify architecture in WorkflowOrchestrator comments

**MEDIUM-001:** Fuzzy matching algorithm not specified
- Resolution: Use Jaro-Winkler (strsim crate), threshold 0.85

**MEDIUM-002:** Source media heuristics not detailed
- Resolution: Implement heuristics with confidence scoring

**Full Analysis:** See `01_specification_issues.md`

---

## Test Coverage Summary

**Total Tests:** 32 tests
- **Unit Tests:** 18 (component logic)
- **Integration Tests:** 10 (component interactions)
- **System Tests:** 4 (end-to-end accuracy)

**Coverage:** 100% - All 12 requirements have acceptance tests

**Test Data Required:**
- 50 single-track files (known MBIDs)
- 10 full album files (12+ tracks, known track lists)
- 10 edge cases (no tags, ambiguous metadata)
- **Total:** 70 test files minimum
- **Location:** `wkmp-ai/tests/fixtures/` (NOT in git, documented in README)

**Traceability:** Complete matrix in `02_test_specifications/traceability_matrix.md`

---

## Implementation Roadmap (Phase 5 - TBD Week 2)

**Phase 1 (Critical): Pipeline Reordering**
- **Objective:** Reorder pipeline sequence (segment before fingerprint)
- **Effort:** 2-3 days
- **Deliverables:**
  - Refactored `workflow_orchestrator/mod.rs`
  - Per-file pipeline function (`pipeline.rs`)
  - 4 parallel workers via `futures::stream::buffer_unordered(4)`
- **Tests:** TC-U-PIPE-010-01, TC-U-PIPE-020-01, TC-I-PIPE-020-01
- **Success Criteria:** Segmentation executes before fingerprinting, all existing tests pass

**Phase 2 (High): Intelligence-Gathering Components**
- **Objective:** Implement pattern analyzer, contextual matcher, confidence assessor
- **Effort:** 4-5 days
- **Deliverables:**
  - `services/pattern_analyzer.rs` (NEW)
  - `services/contextual_matcher.rs` (NEW)
  - `services/confidence_assessor.rs` (NEW)
- **Tests:** TC-U-PATT-*, TC-U-CTXM-*, TC-U-CONF-*, TC-I-CTXM-*, TC-I-CONF-*, TC-S-*
- **Success Criteria:** >80% pattern accuracy, <10 candidates, >90% acceptance rate, <5% false positive

**Phase 3 (High): Per-Segment Fingerprinting**
- **Objective:** Refactor fingerprinter for per-segment operation
- **Effort:** 2-3 days
- **Deliverables:**
  - Modified `services/fingerprinter.rs`
  - Per-segment AcoustID queries
- **Tests:** TC-U-FING-*, TC-I-FING-*, TC-S-FING-*
- **Success Criteria:** Per-segment more accurate than whole-file for albums

**Phase 4 (Medium): Tick-Based Timing**
- **Objective:** Apply tick conversion to all timing points
- **Effort:** 1 day
- **Deliverables:**
  - `seconds_to_ticks()` function
  - Applied to all 7 timing fields
- **Tests:** TC-U-TICK-*, TC-I-TICK-*
- **Success Criteria:** All timing in database as INTEGER ticks, <1 sample error

**Total Estimated Effort:** 9-12 days (2-3 weeks)

**Detailed increments will be defined in Phase 5 (Week 2)**

---

## Risk Assessment (Phase 7 - TBD Week 3)

**Residual Risk:** Low-Medium (after mitigation)

**Top Risks:**

1. **Per-Segment Fingerprinting Performance**
   - Risk: Significantly slower than whole-file
   - Mitigation: Optimize PCM extraction, cache decoded audio, parallel workers

2. **Contextual Matching Accuracy**
   - Risk: Fails to narrow candidates effectively
   - Mitigation: Fuzzy matching, tune tolerances, extensive testing

3. **Evidence-Based Identification False Positives**
   - Risk: Accepts incorrect MBID matches
   - Mitigation: Conservative thresholds (0.85), validation, review queue

4. **Pipeline Reordering Regression**
   - Risk: Breaks existing functionality
   - Mitigation: Comprehensive testing, gradual rollout, feature flag

**Full risk analysis will be completed in Phase 7 (Week 3)**

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Security issues
- Deferred requirements

See Phase 9 section of `/plan` workflow for 7-step technical debt discovery process.

---

## Success Metrics

**Quantitative:**
- ✅ MBID identification accuracy: >90% for known-good files
- ✅ False positive rate: <5% (incorrect MBID accepted)
- ✅ Pattern detection accuracy: >80% (source media type)
- ✅ Contextual matching effectiveness: <10 candidates in >80% of cases
- ✅ Import throughput: ≥20 files/second (4 workers, 3-min songs)
- ✅ Per-segment fingerprinting overhead: <20% vs. whole-file
- ✅ Contextual matching latency: <2 seconds per file
- ✅ Test coverage: >80% for new components

**Qualitative:**
- ✅ Pipeline executes in correct sequence (segment → match → fingerprint)
- ✅ Zero-configuration preserved (database auto-initialization)
- ✅ No regressions in existing functionality
- ✅ All 32 acceptance tests pass
- ✅ Traceability matrix 100% complete

---

## Dependencies

**Existing Documents (Read-Only):**
- SPEC032 - Audio Ingest Architecture (~900 lines)
- SPEC017 - Sample Rate Conversion (~150 lines)
- SPEC031 - Data-Driven Schema Maintenance (~200 lines)
- IMPL001 - Database Schema (~300 lines)

**Integration Points:**
- `wkmp-ai/src/services/workflow_orchestrator/mod.rs` (refactor state machine)
- `wkmp-ai/src/services/fingerprinter.rs` (add per-segment support)

**External Dependencies:**
- MusicBrainz API (1 req/s rate limit)
- AcoustID API (3 req/s, requires API key)
- AcousticBrainz API (free, no rate limit)

**Library Dependencies:**
- `strsim` (fuzzy string matching) - May need to add to Cargo.toml
- `governor` (rate limiting) - Verify if already present

---

## Constraints

**Technical:**
- Rust stable channel required
- SQLite database (single file)
- API rate limits: MusicBrainz 1 req/s, AcoustID 3 req/s
- Memory: 4 concurrent files decoded to PCM
- Performance: ≥20 files/second throughput

**Process:**
- No breaking API changes (HTTP/SSE unchanged)
- Zero-configuration preserved
- Test coverage target: >80%

**Timeline:**
- Estimated: 9-12 days (2-3 weeks)
- Phased implementation (4 phases)
- Single developer focus (no parallel work)

---

## Next Steps

### Immediate (Ready Now)
1. Review this plan summary
2. Confirm understanding of architectural changes
3. Approve proceeding to implementation
4. Curate test dataset (70 files minimum)

### Implementation Sequence (After Approval)
1. **Phase 1 (Critical):** Pipeline reordering (2-3 days)
   - Refactor workflow orchestrator
   - Implement per-file pipeline with 4 workers
   - Verify segmentation before fingerprinting

2. **Phase 2 (High):** New components (4-5 days)
   - Implement pattern analyzer
   - Implement contextual matcher
   - Implement confidence assessor
   - Integrate into pipeline

3. **Phase 3 (High):** Per-segment fingerprinting (2-3 days)
   - Refactor fingerprinter for per-segment
   - Per-segment AcoustID queries
   - Validate accuracy improvement

4. **Phase 4 (Medium):** Tick-based timing (1 day)
   - Implement conversion function
   - Apply to all timing fields
   - Verify database writes

### After Implementation
1. Execute Phase 9: Post-Implementation Review (MANDATORY)
2. Generate technical debt report
3. Run all 32 tests (verify 100% pass)
4. Verify traceability matrix 100% complete
5. Create final implementation report
6. Archive plan using `/archive-plan PLAN025`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- `requirements_index.md` - All 12 requirements with priorities (~250 lines)
- `scope_statement.md` - In/out scope, assumptions, constraints (~350 lines)
- `01_specification_issues.md` - Phase 2 analysis, 8 issues identified (~200 lines)

**Test Specifications:**
- `02_test_specifications/test_index.md` - All 32 tests quick reference (~180 lines)
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests mapping (~150 lines)

**For Implementation:**
- Read this summary (~450 lines)
- Read requirements index (~250 lines)
- Read test index (~180 lines)
- **Total context:** ~880 lines per increment

**Do NOT Read (Context Overload):**
- wip/SPEC032_IMPLEMENTATION_UPDATE.md (~800 lines) - Source spec, reference only
- docs/SPEC032-audio_ingest_architecture.md (~900 lines) - Detailed spec, reference specific sections only

---

## Plan Status

**Phase 1-3 Status:** ✅ Complete (Week 1 Deliverable)
- Phase 1: Scope Definition - 12 requirements extracted
- Phase 2: Specification Verification - 8 issues (0 CRITICAL), proceeding
- Phase 3: Test Definition - 32 tests, 100% coverage

**Phases 4-8 Status:** Pending (Week 2-3)
- Phase 4: Approach Selection (Week 2)
- Phase 5: Implementation Breakdown (Week 2)
- Phase 6: Effort Estimation (Week 3)
- Phase 7: Risk Assessment (Week 3)
- Phase 8: Final Documentation (Week 3)

**Current Status:** Ready for Implementation Review
**Estimated Timeline:** 9-12 days over 2-3 weeks

---

## Approval and Sign-Off

**Plan Created:** 2025-11-10
**Plan Status:** Ready for Implementation Review (Phases 1-3 Complete)

**Next Action:** User review and approval to proceed with implementation

---

**END OF PLAN SUMMARY**

**Context Window Optimization:** This summary provides complete planning context in ~450 lines. Combined with requirements (~250) and tests (~180), total implementation context is ~880 lines - optimal for AI/human comprehension during incremental development.

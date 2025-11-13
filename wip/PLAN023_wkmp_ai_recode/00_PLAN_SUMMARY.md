# PLAN023: WKMP-AI Ground-Up Recode - PLAN SUMMARY

**Status:** Ready for Implementation Review
**Created:** 2025-01-08
**Specification Source:** wip/SPEC_wkmp_ai_recode.md
**Plan Location:** `wip/PLAN023_wkmp_ai_recode/`

---

## READ THIS FIRST

This plan provides complete specifications for a ground-up recode of wkmp-ai with 3-tier hybrid fusion architecture, per-song sequential processing, and real-time SSE UI.

**For Implementation:**
- Read this summary (400 lines)
- Review requirements: `requirements_index.md`
- Review test specifications: `02_test_specifications/test_index.md`
- Follow traceability matrix: `02_test_specifications/traceability_matrix.md`

**Context Window Budget:**
- This summary: ~400 lines
- Requirements index: ~350 lines
- Test index: ~250 lines
- **Total planning context:** ~1000 lines (optimized for implementation)

---

## Executive Summary

### Problem Being Solved

Current wkmp-ai import has critical limitations:
1. **File-level atomic processing** - No per-song granularity in multi-song files
2. **Linear override strategy** - Later sources blindly replace earlier ones (information loss)
3. **AcousticBrainz obsolescence** - Service ended 2022, no multi-source flavor fusion
4. **No confidence framework** - All sources treated equally, no quality-based decisions
5. **Limited user feedback** - File-level progress only, no per-song status visibility

### Solution Approach

**3-Tier Hybrid Fusion + Per-Song Sequential Processing + Real-Time SSE UI**

**Tier 1:** Parallel source extractors (ID3, Chromaprint, AcoustID, MusicBrainz, Essentia, Audio-derived, Genre mapping)

**Tier 2:** Confidence-weighted fusion (Identity Resolution via Bayesian update, Metadata Fusion via weighted selection, Musical Flavor Synthesis via characteristic-wise averaging)

**Tier 3:** Quality validation (Consistency checks, completeness scoring, conflict detection)

**Workflow:** Per-song sequential processing with real-time SSE events at each stage

### Implementation Status

**Phases 1-3 Complete:**
- ✅ Phase 1: Scope Definition - 46 requirement IDs, 98 atomic requirements extracted
- ✅ Phase 2: Specification Verification - 4 CRITICAL, 8 HIGH, 6 MEDIUM, 3 LOW issues identified
- ✅ Phase 3: Test Definition - 76 tests defined, 100% P0/P1 coverage

**Phases 4-8 Status:** Pending (Week 2-3 implementation)

---

## Requirements Summary

**Total Requirements:** 46 requirement IDs (98 individual SHALL/MUST statements)

**By Priority:**
- **P0 (Critical):** 30 requirements - Core functionality
- **P1 (High):** 10 requirements - Important features
- **P2 (Medium):** 2 requirements - Future enhancements (multi-strategy boundary, parallel processing)

**By Category:**
- Workflow: 4 requirements
- Identity Resolution: 5 requirements (Bayesian fusion)
- Metadata Fusion: 5 requirements (weighted selection)
- Musical Flavor: 6 requirements (characteristic-wise averaging)
- Boundary Detection: 4 requirements (silence baseline)
- Validation: 5 requirements (consistency checks)
- Events (SSE): 4 requirements (real-time updates)
- Database: 8 requirements (provenance tracking)
- Non-Functional: 13 requirements (performance, reliability, maintainability, extensibility)

**Full Requirements:** See `requirements_index.md`

---

## Scope

### ✅ In Scope

**Core Functionality:**
- Per-song import workflow (Phase 0: boundary detection, Phases 1-6: per-song processing)
- 3-tier hybrid fusion (7 extractors, 4 fusers, 3 validators)
- Database schema extensions (13 new columns + import_provenance table)
- Real-time SSE event system (10 event types)
- Complete ground-up recode (clean implementation, no legacy copying)

**Key Features:**
- Bayesian identity resolution with conflict detection
- Characteristic-wise musical flavor synthesis (handles AcousticBrainz obsolescence)
- Per-song error isolation (one failure doesn't abort import)
- Source provenance tracking (every field tracks origin + confidence)
- Validation with quality scoring

### ❌ Out of Scope

**Explicitly Excluded:**
- Multi-strategy passage boundary fusion (beat tracking, structural analysis) - P2
- Parallel song processing - Future optimization
- User feedback learning - Future enhancement
- Legacy wkmp-ai UI modifications
- Import history/replay functionality
- Migration from legacy import data

**Full Scope:** See `scope_statement.md`

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 4 (genre mapping undefined, expected characteristics count, Levenshtein implementation, SSE buffering)
- **HIGH Issues:** 8 (Chromaprint bindings, Essentia availability, API timeouts, rate limiting, etc.)
- **MEDIUM Issues:** 6 (API key configuration, threshold justification, etc.)
- **LOW Issues:** 3 (minor documentation issues)

**Decision:** **⚠️ CONDITIONAL PROCEED**
- Resolve CRITICAL issues before implementation (see Immediate Actions below)
- Resolve HIGH issues during Increment 0 (dependencies research)
- MEDIUM/LOW issues: Address during implementation with reasonable defaults

**Full Analysis:** See `01_specification_issues.md`

---

## Test Coverage Summary

**Total Tests:** 76 (100% P0/P1 requirement coverage)
- **Unit Tests:** 51 (fusion algorithms, extractors, validators)
- **Integration Tests:** 17 (per-song workflow, database schema, SSE events)
- **System Tests:** 4 (end-to-end import, performance)
- **Manual Tests:** 4 (architecture review, test coverage verification)

**Critical Path Tests:**
1. TC-S-010-01: End-to-end multi-song import (system test)
2. TC-U-023-01/02/03: Bayesian update algorithm (unit tests)
3. TC-U-044-01/02: Musical flavor normalization (unit tests)
4. TC-U-043-01/02: Characteristic-wise weighted averaging (unit tests)

**Traceability:** Complete matrix in `02_test_specifications/traceability_matrix.md`

---

## Implementation Roadmap

**Note:** Detailed implementation breakdown (Phases 4-8) will be added in Week 2-3. Current plan provides foundation (requirements, tests, scope).

**Estimated Increments:**
- **Increment 0:** Resolve CRITICAL issues + Dependencies research (2-3 days)
- **Increment 1-N:** TBD in Phase 5 (Implementation Breakdown)

**Estimated Effort:** TBD in Phase 6 (Effort Estimation)

---

## Risk Assessment

**Residual Risk:** Low-Medium (after CRITICAL/HIGH issue resolution)

**Top Risks:**
1. **Chromaprint/Essentia Bindings Unavailable** - Mitigation: Use FFI or defer Essentia
2. **AcousticBrainz Full Offline** - Mitigation: Multi-source flavor synthesis handles absence
3. **API Rate Limiting Violations** - Mitigation: Implement governor crate rate limiting
4. **Database Migration Failure** - Mitigation: Backup database before migration

**Full Risk Analysis:** TBD in Phase 7 (Risk Assessment)

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

## Success Metrics

**Quantitative:**
- ✅ 100% P0/P1 requirements have passing tests (76/76 tests)
- ✅ Test coverage >90% (per REQ-AI-NF-032)
- ✅ Per-song processing ≤ 2 min/song average (per REQ-AI-NF-011)
- ✅ Musical flavor normalized (sum to 1.0 ± 0.0001 per category)
- ✅ Database migration successful (13 columns + 1 table)

**Qualitative:**
- ✅ Per-song import workflow processes multi-song files correctly
- ✅ Hybrid fusion produces higher-quality metadata than single-source
- ✅ Real-time SSE shows per-song progress (not just file-level)
- ✅ Error isolation prevents single-song failure from aborting import
- ✅ Ground-up recode is clean of legacy dependencies

---

## Dependencies

**Existing Documents (Read-Only):**
- SPEC003-musical_flavor.md (~200 lines) - Musical flavor definitions
- REQ002-entity_definitions.md (~140 lines) - Entity definitions
- IMPL001-database_schema.md (current passages table schema)

**Integration Points:**
- Database: Extend `passages` table via migration (non-destructive)
- wkmp-common: Use database pool, event bus utilities (read-only)
- HTTP Server: Port 5723 SSE endpoints

**External Dependencies:**
- Rust crates: Need to add `strsim`, research Chromaprint/Essentia bindings
- APIs: AcoustID (need API key), MusicBrainz, AcousticBrainz (archive)

**Full Dependency Map:** See `dependencies_map.md`

---

## Constraints

**Technical:**
- Rust (stable), Tokio, Axum, SQLite, symphonia - Non-negotiable
- Sequential processing (baseline) - Parallel is future optimization
- Extend `passages` table (not recreate) - Preserve existing data

**Process:**
- No legacy code copying - Reference only
- Test-first approach - Acceptance tests before implementation
- wip/SPEC_wkmp_ai_recode.md is authoritative

**Quality:**
- Normalization precision: 1.0 ± 0.0001
- Test coverage: >90%
- Error handling: No `.unwrap()` on user input/I/O

**Resource:**
- API rate limits: MusicBrainz 1 req/sec, AcoustID 3 req/sec
- Memory: Audio passages loaded individually (not entire file)

---

## Immediate Next Actions

### Before Implementation Begins

**Resolve CRITICAL Issues (Increment 0):**
1. **CRITICAL-001 (Genre Mapping):** Create basic genre → characteristics mapping (20-30 genres)
   - **Action:** Create `genre_mapping.json` or document inline table
   - **Estimated Effort:** 2-3 hours

2. **CRITICAL-002 (Expected Characteristics):** Read SPEC003, extract expected count
   - **Action:** Document total expected characteristics (likely ~30-40)
   - **Estimated Effort:** 30 minutes

3. **CRITICAL-003 (Levenshtein):** Confirm using `strsim::normalized_levenshtein()`
   - **Action:** Update spec, add `strsim` to Cargo.toml
   - **Estimated Effort:** 15 minutes

4. **CRITICAL-004 (SSE Buffering):** Define buffer strategy (bounded queue, backpressure)
   - **Action:** Document using `tokio::sync::mpsc` with capacity 1000
   - **Estimated Effort:** 30 minutes

**Resolve HIGH Issues (Dependencies Research):**
5. **HIGH-001 (Chromaprint):** Research Rust crates, select or plan FFI
   - **Estimated Effort:** 2-4 hours

6. **HIGH-002 (Essentia):** Research bindings, defer if unavailable
   - **Estimated Effort:** 2-4 hours

7. **HIGH-003 through HIGH-008:** Define timeouts, rate limiting, rollback plan
   - **Estimated Effort:** 4-6 hours

**Total Increment 0 Effort:** 2-3 days

### Implementation Sequence

**After Increment 0:**
1. Database migration (13 columns + import_provenance table)
2. Tier 1 extractors (7 modules)
3. Tier 2 fusers (4 modules)
4. Tier 3 validators (3 modules)
5. Per-song workflow engine
6. SSE event system
7. Integration and system testing

**Detailed breakdown TBD in Phase 5 (Week 2)**

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- `requirements_index.md` - All 46 requirements with descriptions
- `scope_statement.md` - In/out scope, assumptions, constraints
- `dependencies_map.md` - External dependencies, existing code, new structure
- `01_specification_issues.md` - Phase 2 completeness analysis

**Test Specifications:**
- `02_test_specifications/test_index.md` - All 76 tests quick reference
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests mapping
- `02_test_specifications/tc_*.md` - Individual test specifications

**For Implementation:**
- Read this summary (~400 lines)
- Read requirements_index.md (~350 lines)
- Read test_index.md (~250 lines)
- **Total context:** ~1000 lines per increment (optimized)

---

## Plan Status

**Phase 1-3 Status:** Complete ✅
- Phase 1: Requirements extraction and scope definition
- Phase 2: Specification completeness verification
- Phase 3: Acceptance test definition

**Phases 4-8 Status:** Pending (Week 2-3)
- Phase 4: Approach Selection (risk-based analysis)
- Phase 5: Implementation Breakdown (increments)
- Phase 6: Effort Estimation
- Phase 7: Risk Assessment
- Phase 8: Final Documentation

**Current Status:** Ready for Implementation Review

**Estimated Timeline:** TBD in Phase 6 (depends on Increment 0 resolution)

---

## Approval and Sign-Off

**Plan Created:** 2025-01-08
**Plan Status:** Ready for Implementation Review
**Phase 1-3 Complete:** Yes ✅

**Next Action:**
1. User reviews plan summary and specification issues
2. User approves CRITICAL issue resolutions
3. User approves proceeding to Increment 0 (dependencies research)
4. After Increment 0: Complete Phases 4-8 (detailed implementation plan)

---

**End of Plan Summary**

**For detailed information, see individual documents in this folder.**

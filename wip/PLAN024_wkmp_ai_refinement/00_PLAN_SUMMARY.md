# PLAN024: wkmp-ai Refinement - PLAN SUMMARY

**Status:** Ready for Implementation (Phases 1-3 Complete)
**Created:** 2025-11-12
**Specification Source:** [wip/SPEC032_wkmp-ai_refinement_specification.md](../SPEC032_wkmp-ai_refinement_specification.md)
**Plan Location:** `wip/PLAN024_wkmp_ai_refinement/`

---

## READ THIS FIRST

This document provides a comprehensive plan for implementing wkmp-ai refinements to align with the refined import workflow. The plan focuses wkmp-ai exclusively on automatic audio file ingest, removing quality control and manual editing features.

**For Implementation:**
1. Read this summary (~450 lines)
2. Review detailed requirements: [requirements_index.md](requirements_index.md)
3. Review test specifications: [02_test_specifications/test_index.md](02_test_specifications/test_index.md)
4. Follow traceability matrix: [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md)
5. Implement increments sequentially (dependency order in traceability matrix)

**Context Window Budget:**
- Plan summary: ~450 lines (this file)
- Requirements index: ~250 lines
- Test index: ~200 lines
- Individual test spec: ~100 lines
- **Total per increment:** ~700-900 lines (optimal for AI/human context)

---

## Executive Summary

### Problem Being Solved

**Current State:**
- SPEC032 includes features that should belong to future microservices (quality control → wkmp-qa, manual editing → wkmp-pe)
- Workflow steps not clearly enumerated (API key validation, folder selection missing)
- Per-file processing pipeline lacks detailed phase enumeration
- Duplicate detection strategy incomplete (filename matching + hash matching needed)
- Settings management unclear (database vs. TOML)
- UI progress display underspecified

**Problems This Plan Addresses:**
1. **Scope Creep:** Clarify wkmp-ai boundaries (automatic ingest only)
2. **Two-Stage Roadmap:** Define Stage One (root folder) vs. Stage Two (external folders)
3. **Workflow Clarity:** Enumerate 5-step top-level workflow
4. **Pipeline Detail:** Document 10-phase per-file processing pipeline
5. **Duplicate Detection:** Implement two-tier detection (filename + hash with bidirectional linking)
6. **Settings Management:** Database-first configuration for all 7 import parameters
7. **UI Progress:** 13 real-time SSE-driven status sections
8. **Status Fields:** Enumerate status values and transitions for files/passages/songs

### Solution Approach

**High-Level Strategy:**
- **Documentation First:** Update SPEC032 with 8 new sections (4-6 hours)
- **Code Refactoring:** Implement 10-phase pipeline and supporting components (8-12 hours)
- **UI Enhancement:** Add 13 progress sections with SSE updates (3-5 hours)
- **Testing & Validation:** Execute 78 acceptance tests (2-4 hours)
- **Total Effort:** 15-25 hours (Medium-Large project)

**Implementation Approach:**
- Test-first development (define tests before implementation)
- Incremental delivery (21 increments, dependency-ordered)
- 100% test coverage (all 26 requirements tested)
- Modular architecture (7 new components, ~15 components modified)

### Implementation Status

**Phases 1-3 Complete (Week 1 Deliverable):**
- ✅ Phase 1: Scope Definition - 26 requirements extracted, scope boundaries clear
- ✅ Phase 2: Specification Verification - 8 issues found (0 CRITICAL, 2 HIGH, 4 MEDIUM, 2 LOW), PROCEED approved
- ✅ Phase 3: Test Definition - 78 tests defined (42 unit, 24 integration, 12 system), 100% coverage

**Phases 4-8 Status:** NOT YET IMPLEMENTED (Week 2-3 scope)
- Phase 4: Approach Selection
- Phase 5: Implementation Breakdown
- Phase 6: Effort Estimation
- Phase 7: Risk Assessment
- Phase 8: Final Documentation

**Note:** This plan uses Week 1 deliverable (Phases 1-3 only). Implementation can begin immediately with:
- Clear scope boundaries
- Complete test specifications
- Traceability matrix for verification

---

## Requirements Summary

**Total Requirements:** 26 (21 functional, 5 non-functional)

| Category | Requirements | Priority Distribution |
|----------|--------------|----------------------|
| **Documentation (SPEC032 updates)** | 4 | P0: 4 |
| **Workflow (5-step top-level)** | 5 | P0: 4, P1: 1 |
| **Per-File Pipeline (10 phases)** | 10 | P0: 9, P1: 1 |
| **Settings & Configuration** | 2 | P0: 1, P1: 1 |
| **Non-Functional (performance, quality, security)** | 5 | P0: 2, P1: 3 |
| **TOTAL** | **26** | **P0: 19 (73%), P1: 7 (27%)** |

**Key Requirements:**
- REQ-SPEC032-001: Scope Definition (SPEC032 doc)
- REQ-SPEC032-004: AcoustID API Key Validation
- REQ-SPEC032-007: Filename Matching Logic (3 outcomes)
- REQ-SPEC032-008: Hash-Based Duplicate Detection (bidirectional linking)
- REQ-SPEC032-010: Silence-Based Segmentation (NO AUDIO detection)
- REQ-SPEC032-012: Song Matching with Confidence (0 or 1 songs per passage)
- REQ-SPEC032-018: Database Settings Table (7 parameters)
- REQ-SPEC032-020: Thirteen UI Progress Sections

**Full Requirements:** See [requirements_index.md](requirements_index.md)

---

## Scope

### ✅ In Scope

**SPEC032 Documentation Updates:**
- 8 new sections: Scope Definition, Two-Stage Roadmap, Five-Step Workflow, Ten-Phase Pipeline, Duplicate Detection, Settings Management, UI Progress, Status Enumerations
- 4 sections updated: Overview, Component Architecture, Workflow State Machine, Database Integration
- Remove quality control and manual editing features (out of scope)

**Code Implementation (Stage One Features):**
1. AcoustID API Key Validation (prompt if invalid, remember choice for session)
2. Folder Selection UI (root folder default, Stage One: subfolders only)
3. Ten-Phase Per-File Pipeline:
   - Filename Matching (skip/reuse/new)
   - Hashing (BLAKE3, bidirectional matching_hashes)
   - Metadata Extraction Merging (new overwrites, old preserved)
   - Silence-Based Segmentation (NO AUDIO detection)
   - Fingerprinting (per potential passage)
   - Song Matching (High/Medium/Low/No confidence, 0 or 1 songs)
   - Passage Recording (database insert, song relationships)
   - Amplitude Analysis (lead-in/lead-out, fade-in/fade-out NULL)
   - Musical Flavor Retrieval (AcousticBrainz → Essentia fallback)
   - Passages Complete (mark file 'INGEST COMPLETE')
4. File & Session Completion tracking
5. Database Settings Table (7 parameters, defaults, auto-initialization)
6. Thread Count Auto-Initialization (CPU_core_count + 1)
7. Status Field Enumerations (files/passages/songs status transitions)
8. Thirteen UI Progress Sections (SSE-driven real-time updates)
9. Parallel Processing (using thread count setting)
10. Sample-Accurate Timing (SPEC017 ticks)
11. Symlink/Junction Handling (do not follow)
12. Metadata Preservation (all extracted data stored)

### ❌ Out of Scope

**Explicitly Excluded (Future Microservices):**
- Quality Control → wkmp-qa (future): Skip/gap/quality detection
- Manual Passage Editing → wkmp-pe (future): User-directed fade points, manual MBID revision

**Stage Two Features (Future Enhancement):**
- External folder scanning (outside root folder)
- File movement/copying to root folder after identification

**Not Changed (Existing Features Preserved):**
- Port 5723, HTTP/SSE server infrastructure
- Zero-configuration database initialization (SPEC031)
- Integration with wkmp-ui launch points
- Existing database schema (tables preserved, fields added)

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 0 ✅
- **HIGH Issues:** 2 (metadata merge NULL handling, adjacent passage merging algorithm)
- **MEDIUM Issues:** 4 (confidence thresholds, symlink detection, Essentia integration, 25% lead-in condition)
- **LOW Issues:** 2 (file hash algorithm, UI update frequency)

**Decision:** PROCEED ✅ - No blockers, HIGH issues clarifiable during SPEC032 update

**Specification Quality Score:** 91% (GOOD - Ready for implementation)

**Full Analysis:** See [01_specification_issues.md](01_specification_issues.md)

**Actions Before Implementation:**
1. Resolve HIGH-001 (metadata merge): NULL preserves existing (never delete)
2. Resolve HIGH-002 (adjacent merging): Defer automatic merging to future OR implement simple heuristic (document in SPEC032)
3. Document MEDIUM issues in SPEC032 update (thresholds, algorithms, parameters)

---

## Implementation Roadmap

**Note:** Phases 4-5 (Implementation Breakdown) not yet executed. Below is preliminary roadmap based on dependency analysis from traceability matrix.

### Preliminary Increment Sequence (21 Increments)

**Phase 1: Documentation (Increment 1)**
- **Increment 1: SPEC032 Updates** (4-6 hours)
  - Add 8 new sections, update 4 existing sections
  - Remove out-of-scope features
  - Document HIGH/MEDIUM issue resolutions
  - Deliverables: Updated SPEC032 document
  - Tests: TC-S-001-01, TC-S-002-01, TC-S-003-01, TC-S-006-01 (manual verification)
  - Success: All sections present, markdown valid, references correct

**Phase 2: Foundational Components (Increments 2-3)**
- **Increment 2: Database Settings Table** (2-3 hours)
  - Component: `settings_manager.rs` (new)
  - Deliverables: Settings read/write with defaults, 7 parameters supported
  - Tests: TC-U-018-01 through TC-U-018-04, TC-I-018-01
  - Success: All settings readable, defaults applied, persistence verified

- **Increment 3: Status Field Enumerations** (1-2 hours)
  - Component: `status_manager.rs` (new)
  - Deliverables: Status value enforcement, transition validation
  - Tests: TC-U-021-01 through TC-U-021-04, TC-I-021-01
  - Success: Status transitions enforced, invalid values rejected

**Phase 3: Workflow Entry Points (Increments 4-5)**
- **Increment 4: AcoustID API Key Validation** (2-3 hours)
  - Component: `api_key_validator.rs` (new)
  - Deliverables: Key validation, user prompt, session memory
  - Tests: TC-U-004-01 through TC-U-004-04, TC-I-004-01, TC-S-004-01
  - Success: Invalid key prompts user, valid key continues silently, choice remembered

- **Increment 5: Folder Selection** (1-2 hours)
  - Component: `folder_selector.rs` (new)
  - Deliverables: Folder selection UI, Stage One constraint enforcement
  - Tests: TC-U-005-01 through TC-U-005-03, TC-S-005-01
  - Success: Root folder allowed, subfolders allowed, external folders rejected

**Phase 4: Per-File Pipeline (Increments 6-13)**
- **Increment 6: Filename Matching** (2-3 hours)
- **Increment 7: Hash-Based Deduplication** (3-4 hours)
- **Increment 8: Metadata Extraction Merging** (2-3 hours)
- **Increment 9: Silence-Based Segmentation** (3-4 hours)
- **Increment 10: Fingerprinting** (2-3 hours)
- **Increment 11: Song Matching with Confidence** (4-5 hours)
- **Increment 12: Passage Recording** (2-3 hours)
- **Increment 13: Amplitude Analysis** (3-4 hours)
- **Increment 14: Musical Flavor Retrieval** (3-4 hours)

**Phase 5: Workflow Completion (Increment 15)**
- **Increment 15: File & Session Completion** (1-2 hours)

**Phase 6: Performance & UI (Increments 16-18)**
- **Increment 16: Thread Count Auto-Initialization** (1-2 hours)
- **Increment 17: Parallel Processing** (2-3 hours)
- **Increment 18: UI Progress Sections** (3-5 hours)

**Phase 7: Quality & Edge Cases (Increments 19-21)**
- **Increment 19: Symlink/Junction Handling** (1-2 hours)
- **Increment 20: Real-Time Progress Updates** (1-2 hours)
- **Increment 21: Metadata Preservation & Sample-Accurate Timing** (1-2 hours)

**Total Estimated Effort:** 15-25 hours (per specification)

**Note:** Detailed increment breakdown with tasks, deliverables, and checkpoints will be provided in Phase 5 (Implementation Breakdown) - Week 2 deliverable.

---

## Test Coverage Summary

**Total Tests:** 78 (42 unit, 24 integration, 12 system)
**Coverage:** 100% - All 26 requirements have acceptance tests

**Test Distribution:**
| Test Type | Count | Percentage | Purpose |
|-----------|-------|------------|---------|
| **Unit Tests** | 42 | 54% | Component isolation, fast feedback |
| **Integration Tests** | 24 | 31% | Component interaction, database integration |
| **System Tests** | 12 | 15% | End-to-end workflows, user scenarios |

**Highest Test Coverage Requirements:**
- REQ-SPEC032-008 (Hash Duplicate Detection): 8 tests
- REQ-SPEC032-012 (Song Matching): 8 tests
- REQ-SPEC032-015 (Musical Flavor Retrieval): 8 tests

**Test Execution Strategy:**
1. **Pre-commit:** Unit tests (fast, <30 seconds)
2. **Pre-push:** Unit + Integration tests (<2 minutes)
3. **CI Pipeline:** All tests (<8 minutes total)

**Performance Targets:**
- Unit tests: <30 seconds total
- Integration tests: <2 minutes total
- System tests: <5 minutes total
- **Total test suite: <8 minutes**

**Traceability:** Complete matrix in [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md)

---

## Risk Assessment

**Overall Residual Risk:** Low-Medium (after mitigations)

**Top Risks:**

**Risk 1: SPEC032 Update Scope Creep**
- **Failure Mode:** Adding features beyond refinement scope
- **Residual Risk:** Low
- **Mitigation:** Strict adherence to requirements index, no additions without user approval

**Risk 2: Code Refactoring Breaks Existing Functionality**
- **Failure Mode:** Updating workflow orchestrator introduces regressions
- **Residual Risk:** Low-Medium
- **Mitigation:** Comprehensive tests before/after refactoring, staged rollout per phase

**Risk 3: UI Progress Display Performance Degradation**
- **Failure Mode:** 13 SSE sections cause UI lag or server load
- **Residual Risk:** Low
- **Mitigation:** Throttle SSE updates (max 100 updates/second), test with large imports (1000+ files)

**Risk 4: Settings Migration for Existing Databases**
- **Failure Mode:** Existing wkmp.db lacks settings table or entries
- **Residual Risk:** Low
- **Mitigation:** Settings manager provides defaults, SPEC031 schema sync creates missing table

**Risk 5: Duplicate Detection Edge Cases**
- **Failure Mode:** Complex rename/reorganization creates incorrect links
- **Residual Risk:** Low
- **Mitigation:** Comprehensive duplicate scenario testing, bidirectional link validation

**Full Risk Analysis:** To be provided in Phase 7 (Week 3)

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

**Placeholder for Post-Implementation:**
- Total Items: [N] ([N] critical, [N] high, [N] medium, [N] low)
- Estimated Remediation: [X-Y] hours
- Immediate Action Required: [Yes/No]

---

## Success Metrics

**Quantitative:**
- ✅ All 26 requirements implemented
- ✅ All 78 tests passing
- ✅ Code coverage ≥80% (measured via cargo-tarpaulin)
- ✅ Test suite executes in <8 minutes
- ✅ Import completes for 100-file library in <5 minutes
- ✅ Traceability matrix 100% complete (requirement ↔ test ↔ code)

**Qualitative:**
- ✅ SPEC032 updated with all 8 new sections
- ✅ Out-of-scope features removed (quality control, manual editing)
- ✅ User can import music library without configuration
- ✅ Duplicate files detected and skipped
- ✅ UI provides real-time progress visibility
- ✅ Zero regressions in existing wkmp-ai functionality

---

## Dependencies

### Existing Documents (Read-Only)

**Architecture & Requirements:**
- [SPEC032-audio_ingest_architecture.md](../../docs/SPEC032-audio_ingest_architecture.md) (~800 lines) - **Target document to UPDATE**
- [REQ001-requirements.md](../../docs/REQ001-requirements.md) (~2000+ lines) - System requirements reference
- [SPEC002-crossfade.md](../../docs/SPEC002-crossfade.md) (~600 lines) - Passage timing definitions
- [SPEC017-sample_rate_conversion.md](../../docs/SPEC017-sample_rate_conversion.md) (~400 lines) - Tick time units
- [IMPL001-database_schema.md](../../docs/IMPL001-database_schema.md) (~1200 lines) - Database schema
- [SPEC031-data_driven_schema_maintenance.md](../../docs/SPEC031-data_driven_schema_maintenance.md) (~500 lines) - Zero-config DB init

**Source:**
- [wip/wkmp-ai_refinement.md](../wkmp-ai_refinement.md) (~104 lines) - Original refinement notes (source of truth)

### Integration Points

**wkmp-ui Integration (Existing, Not Modified):**
- Launch point: "Import Music" button opens http://localhost:5723
- Health check: wkmp-ui checks `/health` endpoint
- No embedded import UI in wkmp-ui (wkmp-ai owns all import UX)

**Database (Shared):**
- wkmp.db in root folder
- Shared tables: files, passages, songs, artists, works, albums, settings
- No concurrent writes expected (wkmp-ai owns import, wkmp-ui reads)

### External Dependencies

**Rust Crates (Verify in Cargo.toml):**
- ✅ tokio, axum, symphonia, lofty, serde_json (confirmed available)
- ⚠️ num_cpus, sha2/blake3 (add if missing)

**External APIs (Runtime):**
- AcoustID API (fingerprint matching) - Requires valid API key
- AcousticBrainz API (musical flavor) - Optional, Essentia fallback available
- Essentia tool (command-line) - Must be in PATH or configured location

**Full Dependencies:** See [dependencies_map.md](dependencies_map.md)

---

## Constraints

**Technical:**
- Rust stable channel, Tokio async, Axum HTTP+SSE
- SQLite with JSON1 extension
- 48kHz internal processing, SPEC017 ticks for time values

**Performance:**
- Import speed limited by AcoustID API rate limits
- Parallel processing limited by CPU core count
- Memory usage must handle large libraries (100k+ files)

**Process:**
- Documentation first (SPEC032 before code)
- Test-first development (tests before implementation)
- No shortcuts (complete implementation required)
- Traceability maintained (requirement ↔ test ↔ code)

**Architectural:**
- Microservices isolation (wkmp-ai does NOT implement quality control or manual editing)
- Database schema backward-compatible
- Zero-configuration startup preserved (SPEC031)

**Timeline:**
- Estimated: 15-25 hours total
- No hard deadline (quality over speed)

---

## Next Steps

### Immediate (Ready Now)

1. **Review Plan:** User reviews this summary, approves proceeding
2. **Resolve HIGH Issues:** Document metadata merge NULL handling and adjacent passage merging algorithm in SPEC032
3. **Begin Increment 1:** Update SPEC032 with 8 new sections

### Implementation Sequence

**Week 1: Documentation & Foundation** (6-10 hours)
1. Increment 1: SPEC032 Updates (4-6 hours)
2. Increment 2: Database Settings Table (2-3 hours)
3. Increment 3: Status Field Enumerations (1-2 hours)

**Week 2: Workflow & Pipeline** (8-12 hours)
4. Increment 4-5: Workflow Entry Points (3-5 hours)
5. Increment 6-10: Pipeline Phases 1-5 (12-17 hours)
6. Checkpoint: Verify tests passing, no regressions

**Week 3: Pipeline Completion & UI** (8-12 hours)
7. Increment 11-15: Pipeline Phases 6-10, Completion (10-14 hours)
8. Increment 16-18: Performance & UI (6-10 hours)
9. Checkpoint: End-to-end import test

**Week 4: Quality & Finalization** (4-6 hours)
10. Increment 19-21: Edge Cases (3-6 hours)
11. Execute Phase 9: Post-Implementation Review (MANDATORY)
12. Generate technical debt report
13. Run all 78 tests, verify 100% passing
14. Verify traceability matrix complete
15. Create final implementation report
16. Archive plan using `/archive-plan PLAN024`

### After Implementation

1. **Execute Phase 9: Post-Implementation Review** (MANDATORY)
   - 7-step technical debt discovery process
   - Generate comprehensive technical debt report
   - Document known issues, test coverage gaps, performance concerns
2. Run all 78 tests (verify 100% passing)
3. Verify traceability matrix 100% complete (requirement ↔ test ↔ code)
4. Create final implementation report
5. Archive plan using `/archive-plan PLAN024`

**Critical:** Do NOT mark plan complete or archive until Phase 9 technical debt report is generated and attached.

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- [requirements_index.md](requirements_index.md) - All 26 requirements with priorities (~250 lines)
- [scope_statement.md](scope_statement.md) - In/out scope, assumptions, constraints (~450 lines)
- [dependencies_map.md](dependencies_map.md) - Complete dependency inventory (~350 lines)
- [01_specification_issues.md](01_specification_issues.md) - Phase 2 analysis, 8 issues found (~500 lines)

**Test Specifications:**
- [02_test_specifications/test_index.md](02_test_specifications/test_index.md) - All 78 tests quick reference (~200 lines)
- [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md) - Requirements ↔ Tests mapping (~400 lines)
- [02_test_specifications/tc_u_007_01.md](02_test_specifications/tc_u_007_01.md) - Example unit test (filename matching)
- [02_test_specifications/tc_i_008_01.md](02_test_specifications/tc_i_008_01.md) - Example integration test (hash deduplication)
- [02_test_specifications/tc_s_010_01.md](02_test_specifications/tc_s_010_01.md) - Example system test (silence segmentation)

**For Implementation:**
- Read plan summary (~450 lines) - THIS FILE
- Read current increment specification (TBD in Phase 5, ~250 lines)
- Read relevant test specs (~100-150 lines)
- **Total context:** ~700-900 lines per increment (optimal for AI/human)

---

## Plan Status

**Phase 1-3 Status:** ✅ Complete (Week 1 Deliverable)
- Phase 1: Scope Definition - 26 requirements, clear boundaries
- Phase 2: Specification Verification - 91% quality score, PROCEED approved
- Phase 3: Test Definition - 78 tests, 100% coverage

**Phases 4-8 Status:** ⏳ Pending (Week 2-3 Scope)
- Phase 4: Approach Selection (not yet executed)
- Phase 5: Implementation Breakdown (not yet executed)
- Phase 6: Effort Estimation (not yet executed)
- Phase 7: Risk Assessment (not yet executed)
- Phase 8: Final Documentation (not yet executed)

**Current Status:** ✅ Ready for Implementation (Phases 1-3 sufficient to begin coding)

**Estimated Timeline:**
- SPEC032 Update: 1-2 days (4-6 hours)
- Code Implementation: 3-5 days (12-18 hours)
- Testing & Validation: 1-2 days (3-6 hours)
- **Total: 5-9 days** (15-25 hours effort)

---

## Approval and Sign-Off

**Plan Created:** 2025-11-12
**Plan Status:** Ready for Implementation Review

**Phases 1-3 Deliverables:**
- ✅ Requirements extracted and categorized (26 total)
- ✅ Specification issues identified and prioritized (8 total, 0 CRITICAL)
- ✅ Acceptance tests defined and traced (78 total, 100% coverage)
- ✅ Modular plan structure created (optimal context window usage)

**Next Action:** User reviews plan, approves proceeding, begins Increment 1 (SPEC032 updates)

**Note:** This is a Week 1 deliverable (Phases 1-3 only). Phases 4-8 (Approach Selection, Implementation Breakdown, Effort Estimation, Risk Assessment, Final Documentation) will be added in Weeks 2-3 as /plan workflow matures.

**Implementation can begin immediately** with current deliverables providing:
- Clear scope boundaries
- Complete test specifications
- Traceability matrix for verification
- Dependency analysis
- Preliminary increment sequence

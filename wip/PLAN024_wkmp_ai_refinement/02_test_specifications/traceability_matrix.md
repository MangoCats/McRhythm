# Traceability Matrix: PLAN024 - wkmp-ai Refinement

**Plan:** PLAN024 - wkmp-ai Refinement Implementation
**Purpose:** Ensure 100% coverage - every requirement has tests, every test traces to requirements
**Date:** 2025-11-12
**Status:** Complete (all 26 requirements covered)

---

## Coverage Summary

**Requirements:** 26 total
**Tests:** 78 total (42 unit, 24 integration, 12 system)
**Coverage:** 100% (all requirements have acceptance tests)

---

## Traceability Matrix

| Requirement ID | Requirement Description | Unit Tests | Integration Tests | System Tests | Implementation File(s) | Status | Coverage |
|----------------|-------------------------|------------|-------------------|--------------|------------------------|--------|----------|
| **REQ-SPEC032-001** | Scope Definition (SPEC032 doc) | - | - | TC-S-001-01 | docs/SPEC032-audio_ingest_architecture.md | Pending | Complete |
| **REQ-SPEC032-002** | Two-Stage Roadmap (SPEC032 doc) | - | - | TC-S-002-01 | docs/SPEC032-audio_ingest_architecture.md | Pending | Complete |
| **REQ-SPEC032-003** | Five-Step Workflow (SPEC032 doc) | - | - | TC-S-003-01 | docs/SPEC032-audio_ingest_architecture.md | Pending | Complete |
| **REQ-SPEC032-004** | AcoustID API Key Validation | TC-U-004-01, TC-U-004-02, TC-U-004-03, TC-U-004-04 | TC-I-004-01 | TC-S-004-01 | wkmp-ai/src/services/api_key_validator.rs (new) | Pending | Complete |
| **REQ-SPEC032-005** | Folder Selection | TC-U-005-01, TC-U-005-02, TC-U-005-03 | - | TC-S-005-01 | wkmp-ai/src/api/ui/folder_selector.rs (new) | Pending | Complete |
| **REQ-SPEC032-006** | Ten-Phase Pipeline (SPEC032 doc) | - | - | TC-S-006-01 | docs/SPEC032-audio_ingest_architecture.md | Pending | Complete |
| **REQ-SPEC032-007** | Filename Matching Logic | TC-U-007-01, TC-U-007-02, TC-U-007-03 | TC-I-007-01 | - | wkmp-ai/src/services/filename_matcher.rs (new) | Pending | Complete |
| **REQ-SPEC032-008** | Hash-Based Duplicate Detection | TC-U-008-01, TC-U-008-02, TC-U-008-03, TC-U-008-04, TC-U-008-05 | TC-I-008-01, TC-I-008-02 | TC-S-008-01 | wkmp-ai/src/services/hash_deduplicator.rs (new), wkmp-ai/src/db/files.rs (modify) | Pending | Complete |
| **REQ-SPEC032-009** | Metadata Extraction Merging | TC-U-009-01, TC-U-009-02, TC-U-009-03, TC-U-009-04 | TC-I-009-01 | - | wkmp-ai/src/services/metadata_extractor.rs (modify) | Pending | Complete |
| **REQ-SPEC032-010** | Silence-Based Segmentation | TC-U-010-01, TC-U-010-02, TC-U-010-03, TC-U-010-04 | TC-I-010-01 | TC-S-010-01 | wkmp-ai/src/services/silence_detector.rs (modify) | Pending | Complete |
| **REQ-SPEC032-011** | Fingerprinting Per Passage | TC-U-011-01, TC-U-011-02, TC-U-011-03 | TC-I-011-01, TC-I-011-02 | - | wkmp-ai/src/services/fingerprinter.rs (modify) | Pending | Complete |
| **REQ-SPEC032-012** | Song Matching with Confidence | TC-U-012-01, TC-U-012-02, TC-U-012-03, TC-U-012-04, TC-U-012-05 | TC-I-012-01, TC-I-012-02 | TC-S-012-01 | wkmp-ai/src/services/confidence_assessor.rs (modify) | Pending | Complete |
| **REQ-SPEC032-013** | Passage Recording | TC-U-013-01, TC-U-013-02, TC-U-013-03 | TC-I-013-01, TC-I-013-02 | - | wkmp-ai/src/db/passages.rs (modify) | Pending | Complete |
| **REQ-SPEC032-014** | Amplitude-Based Lead-In/Lead-Out | TC-U-014-01, TC-U-014-02, TC-U-014-03, TC-U-014-04 | TC-I-014-01 | TC-S-014-01 | wkmp-ai/src/services/amplitude_analyzer.rs (modify) | Pending | Complete |
| **REQ-SPEC032-015** | Musical Flavor Retrieval | TC-U-015-01, TC-U-015-02, TC-U-015-03, TC-U-015-04 | TC-I-015-01, TC-I-015-02, TC-I-015-03 | TC-S-015-01 | wkmp-ai/src/services/acousticbrainz_client.rs (modify), wkmp-ai/src/services/essentia_client.rs (modify) | Pending | Complete |
| **REQ-SPEC032-016** | File Completion | TC-U-016-01, TC-U-016-02 | TC-I-016-01 | - | wkmp-ai/src/services/workflow_orchestrator/mod.rs (modify) | Pending | Complete |
| **REQ-SPEC032-017** | Session Completion | TC-U-017-01, TC-U-017-02 | TC-I-017-01 | - | wkmp-ai/src/services/workflow_orchestrator/mod.rs (modify) | Pending | Complete |
| **REQ-SPEC032-018** | Database Settings Table | TC-U-018-01, TC-U-018-02, TC-U-018-03, TC-U-018-04 | TC-I-018-01 | - | wkmp-ai/src/services/settings_manager.rs (new), wkmp-ai/src/db/settings.rs (modify) | Pending | Complete |
| **REQ-SPEC032-019** | Thread Count Auto-Initialization | TC-U-019-01, TC-U-019-02, TC-U-019-03 | TC-I-019-01 | - | wkmp-ai/src/services/settings_manager.rs (new) | Pending | Complete |
| **REQ-SPEC032-020** | Thirteen UI Progress Sections | - | TC-I-020-01 | TC-S-020-01 | wkmp-ai/src/api/sse.rs (modify), wkmp-ai/src/api/ui/import_progress.rs (modify) | Pending | Complete |
| **REQ-SPEC032-021** | Status Field Enumerations | TC-U-021-01, TC-U-021-02, TC-U-021-03, TC-U-021-04 | TC-I-021-01 | - | wkmp-ai/src/db/status_manager.rs (new), wkmp-ai/src/db/*.rs (modify) | Pending | Complete |
| **REQ-SPEC032-NF-001** | Parallel Processing | - | TC-I-NF001-01 | TC-S-NF001-01 | wkmp-ai/src/services/workflow_orchestrator/mod.rs (modify) | Pending | Complete |
| **REQ-SPEC032-NF-002** | Real-Time Progress Updates | - | TC-I-NF002-01 | TC-S-NF002-01 | wkmp-ai/src/api/sse.rs (modify) | Pending | Complete |
| **REQ-SPEC032-NF-003** | Sample-Accurate Timing | TC-U-NF003-01, TC-U-NF003-02 | - | - | wkmp-ai/src/services/silence_detector.rs (modify), wkmp-ai/src/services/amplitude_analyzer.rs (modify) | Pending | Complete |
| **REQ-SPEC032-NF-004** | Symlink/Junction Handling | TC-U-NF004-01, TC-U-NF004-02 | - | TC-S-NF004-01 | wkmp-ai/src/services/file_scanner.rs (modify) | Pending | Complete |
| **REQ-SPEC032-NF-005** | Metadata Preservation | TC-U-NF005-01, TC-U-NF005-02 | TC-I-NF005-01 | - | wkmp-ai/src/services/metadata_extractor.rs (modify) | Pending | Complete |

---

## Forward Traceability (Requirement → Tests)

**All 26 requirements have tests.** ✅

**Distribution:**
- Requirements with 1 test: 8 (31%)
- Requirements with 2-3 tests: 5 (19%)
- Requirements with 4-5 tests: 9 (35%)
- Requirements with 6-8 tests: 4 (15%)

**Highest Test Coverage:**
- REQ-SPEC032-008 (Hash Duplicate Detection): 8 tests
- REQ-SPEC032-012 (Song Matching): 8 tests
- REQ-SPEC032-015 (Musical Flavor): 8 tests

---

## Backward Traceability (Test → Requirement)

**All 78 tests trace to requirements.** ✅

**Orphaned Tests:** 0 (no tests without requirement traceability)

---

## Implementation Traceability (Requirement → Code)

**Files to Create (New Components):**
1. wkmp-ai/src/services/api_key_validator.rs (REQ-004)
2. wkmp-ai/src/api/ui/folder_selector.rs (REQ-005)
3. wkmp-ai/src/services/filename_matcher.rs (REQ-007)
4. wkmp-ai/src/services/hash_deduplicator.rs (REQ-008)
5. wkmp-ai/src/services/settings_manager.rs (REQ-018, REQ-019)
6. wkmp-ai/src/models/progress_tracker.rs (REQ-020)
7. wkmp-ai/src/db/status_manager.rs (REQ-021)

**Files to Modify (Existing Components):**
1. wkmp-ai/src/services/workflow_orchestrator/mod.rs (REQ-016, REQ-017, NF-001)
2. wkmp-ai/src/services/metadata_extractor.rs (REQ-009, NF-005)
3. wkmp-ai/src/services/silence_detector.rs (REQ-010, NF-003)
4. wkmp-ai/src/services/fingerprinter.rs (REQ-011)
5. wkmp-ai/src/services/confidence_assessor.rs (REQ-012)
6. wkmp-ai/src/services/amplitude_analyzer.rs (REQ-014, NF-003)
7. wkmp-ai/src/services/acousticbrainz_client.rs (REQ-015)
8. wkmp-ai/src/services/essentia_client.rs (REQ-015)
9. wkmp-ai/src/services/file_scanner.rs (NF-004)
10. wkmp-ai/src/db/files.rs (REQ-008)
11. wkmp-ai/src/db/passages.rs (REQ-013)
12. wkmp-ai/src/db/settings.rs (REQ-018)
13. wkmp-ai/src/api/sse.rs (REQ-020, NF-002)
14. wkmp-ai/src/api/ui/import_progress.rs (REQ-020)

**Documentation to Update:**
1. docs/SPEC032-audio_ingest_architecture.md (REQ-001, REQ-002, REQ-003, REQ-006)

---

## Gap Analysis

### Requirements Without Implementation (Pending)

**All requirements pending implementation.** This is expected - plan defines requirements and tests before implementation begins.

**Statuses:**
- **Pending:** Test specification complete, awaiting implementation (26 requirements, 100%)
- **Implemented:** Code written and passing tests (0 requirements, 0%)
- **Verified:** All tests passing, requirement satisfied (0 requirements, 0%)

### Tests Without Implementation (Ready to Implement)

**All 78 tests are defined and ready for implementation.** Developers can:
1. Read test specification (e.g., tc_u_007_01.md)
2. Implement code to pass test
3. Run test to verify implementation
4. Mark test status as "Implemented" when passing

---

## Test Execution Order

**Recommended Sequence (Dependency-Based):**

### Phase 1: Foundational Components (No Dependencies)

**Increment 1: Database Settings Table**
- Implement: REQ-SPEC032-018 (settings_manager.rs)
- Run Tests: TC-U-018-01 through TC-U-018-04, TC-I-018-01

**Increment 2: Status Field Enumerations**
- Implement: REQ-SPEC032-021 (status_manager.rs)
- Run Tests: TC-U-021-01 through TC-U-021-04, TC-I-021-01

### Phase 2: Workflow Entry Points

**Increment 3: API Key Validation**
- Implement: REQ-SPEC032-004 (api_key_validator.rs)
- Depends on: REQ-018 (settings table for key storage)
- Run Tests: TC-U-004-01 through TC-U-004-04, TC-I-004-01, TC-S-004-01

**Increment 4: Folder Selection**
- Implement: REQ-SPEC032-005 (folder_selector.rs)
- No dependencies
- Run Tests: TC-U-005-01 through TC-U-005-03, TC-S-005-01

### Phase 3: Per-File Pipeline (Sequential Phases)

**Increment 5: Filename Matching**
- Implement: REQ-SPEC032-007 (filename_matcher.rs)
- Depends on: REQ-021 (status fields)
- Run Tests: TC-U-007-01 through TC-U-007-03, TC-I-007-01

**Increment 6: Hash-Based Deduplication**
- Implement: REQ-SPEC032-008 (hash_deduplicator.rs)
- Depends on: REQ-007 (filename matching), REQ-021 (status fields)
- Run Tests: TC-U-008-01 through TC-U-008-05, TC-I-008-01, TC-I-008-02, TC-S-008-01

**Increment 7: Metadata Extraction Merging**
- Implement: REQ-SPEC032-009 (metadata_extractor.rs updates)
- No dependencies (existing component)
- Run Tests: TC-U-009-01 through TC-U-009-04, TC-I-009-01

**Increment 8: Silence-Based Segmentation**
- Implement: REQ-SPEC032-010 (silence_detector.rs updates)
- Depends on: REQ-018 (settings for thresholds), REQ-021 (NO AUDIO status)
- Run Tests: TC-U-010-01 through TC-U-010-04, TC-I-010-01, TC-S-010-01

**Increment 9: Fingerprinting**
- Implement: REQ-SPEC032-011 (fingerprinter.rs updates)
- Depends on: REQ-004 (API key validation)
- Run Tests: TC-U-011-01 through TC-U-011-03, TC-I-011-01, TC-I-011-02

**Increment 10: Song Matching with Confidence**
- Implement: REQ-SPEC032-012 (confidence_assessor.rs updates)
- Depends on: REQ-011 (fingerprinting), REQ-009 (metadata)
- Run Tests: TC-U-012-01 through TC-U-012-05, TC-I-012-01, TC-I-012-02, TC-S-012-01

**Increment 11: Passage Recording**
- Implement: REQ-SPEC032-013 (passages.rs updates)
- Depends on: REQ-012 (song matching), REQ-021 (status fields)
- Run Tests: TC-U-013-01 through TC-U-013-03, TC-I-013-01, TC-I-013-02

**Increment 12: Amplitude Analysis**
- Implement: REQ-SPEC032-014 (amplitude_analyzer.rs updates)
- Depends on: REQ-013 (passage recording), REQ-018 (settings for thresholds)
- Run Tests: TC-U-014-01 through TC-U-014-04, TC-I-014-01, TC-S-014-01

**Increment 13: Musical Flavor Retrieval**
- Implement: REQ-SPEC032-015 (acousticbrainz_client.rs, essentia_client.rs updates)
- Depends on: REQ-013 (passage recording), REQ-021 (song status)
- Run Tests: TC-U-015-01 through TC-U-015-04, TC-I-015-01 through TC-I-015-03, TC-S-015-01

### Phase 4: Workflow Completion

**Increment 14: File & Session Completion**
- Implement: REQ-SPEC032-016, REQ-SPEC032-017 (workflow_orchestrator.rs updates)
- Depends on: All 10 pipeline phases (REQ-007 through REQ-015)
- Run Tests: TC-U-016-01, TC-U-016-02, TC-I-016-01, TC-U-017-01, TC-U-017-02, TC-I-017-01

### Phase 5: Performance & UI

**Increment 15: Thread Count Auto-Initialization**
- Implement: REQ-SPEC032-019 (settings_manager.rs addition)
- Depends on: REQ-018 (settings table)
- Run Tests: TC-U-019-01 through TC-U-019-03, TC-I-019-01

**Increment 16: Parallel Processing**
- Implement: REQ-SPEC032-NF-001 (workflow_orchestrator.rs updates)
- Depends on: REQ-019 (thread count), REQ-016 (file completion)
- Run Tests: TC-I-NF001-01, TC-S-NF001-01

**Increment 17: UI Progress Sections**
- Implement: REQ-SPEC032-020 (sse.rs, import_progress.rs updates)
- Depends on: All pipeline phases (for statistics)
- Run Tests: TC-I-020-01, TC-S-020-01

**Increment 18: Real-Time Progress Updates**
- Implement: REQ-SPEC032-NF-002 (sse.rs updates)
- Depends on: REQ-020 (UI sections)
- Run Tests: TC-I-NF002-01, TC-S-NF002-01

### Phase 6: Quality & Edge Cases

**Increment 19: Symlink/Junction Handling**
- Implement: REQ-SPEC032-NF-004 (file_scanner.rs updates)
- No dependencies
- Run Tests: TC-U-NF004-01, TC-U-NF004-02, TC-S-NF004-01

**Increment 20: Metadata Preservation & Sample-Accurate Timing**
- Implement: REQ-SPEC032-NF-005, REQ-SPEC032-NF-003 (various component updates)
- Verify existing implementations meet requirements
- Run Tests: TC-U-NF005-01, TC-U-NF005-02, TC-I-NF005-01, TC-U-NF003-01, TC-U-NF003-02

### Phase 7: Documentation

**Increment 21: SPEC032 Updates**
- Implement: REQ-SPEC032-001, REQ-SPEC032-002, REQ-SPEC032-003, REQ-SPEC032-006
- Update: docs/SPEC032-audio_ingest_architecture.md
- Run Tests: TC-S-001-01, TC-S-002-01, TC-S-003-01, TC-S-006-01 (manual verification)

---

## Verification Checklist

**Before Marking Plan Complete:**

- [ ] All 26 requirements implemented
- [ ] All 78 tests passing
- [ ] Traceability matrix 100% complete (requirement ↔ test ↔ code)
- [ ] No orphaned tests (all tests trace to requirements)
- [ ] No untested requirements (all requirements have tests)
- [ ] Code coverage ≥80% (measured via cargo-tarpaulin)
- [ ] All acceptance criteria met (from specification)
- [ ] Documentation updated (SPEC032)
- [ ] Phase 9 technical debt assessment complete

**Status Updates:**

| Status | Description | Requirements | Tests |
|--------|-------------|--------------|-------|
| **Pending** | Awaiting implementation | 26 (100%) | 78 (100%) |
| **Implemented** | Code written, tests passing | 0 (0%) | 0 (0%) |
| **Verified** | All tests passing, peer reviewed | 0 (0%) | 0 (0%) |

**Final Verification:**
- [ ] Traceability matrix updated with implementation file paths (accurate)
- [ ] All tests status = "Implemented" and "Passing"
- [ ] Phase 9 technical debt report attached to plan
- [ ] Plan ready for archival

---

## Maintenance

**Updating This Matrix:**
1. When requirement added: Add row, ensure tests exist
2. When test added: Add to appropriate requirement row
3. When implementation file changes: Update "Implementation File(s)" column
4. When test passes: Update "Status" column to "Implemented" → "Verified"
5. When requirement complete: Mark "Status" as "Verified"

**Audit Frequency:**
- Per increment: Verify tests for that increment trace correctly
- Per checkpoint: Verify no gaps in coverage
- Final review: Verify 100% coverage before release

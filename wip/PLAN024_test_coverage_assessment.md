# PLAN024 Test Coverage Assessment: Regression Prevention Analysis

**Date:** 2025-11-13
**Context:** Assess unit test coverage's ability to prevent regression of recent architectural issues
**Recent Issues:** Batch extraction architecture persisting despite SPEC032 per-file pipeline requirement

---

## Executive Summary

**Overall Assessment: INSUFFICIENT COVERAGE for architectural compliance verification**

**Key Findings:**
1. ❌ **ZERO tests verify batch vs. per-file architecture** - No tests detect if batch extraction occurs
2. ❌ **NO integration tests for workflow orchestrator entry points** - execute_import_plan024() untested
3. ❌ **NO tests for phase_scanning behavior** - Embedded batch extraction went undetected
4. ❌ **NO tests verify SPEC031 zero-conf database schema** - session_id column issue preventable
5. ✅ **Good coverage for state machine and individual services** - 97 unit tests across 15 files

**Recommendation:** Add 5 critical regression prevention tests (detailed in Section 5)

---

## 1. Recent Issues Analysis

### Issue 1: Batch Extraction in execute_import_plan024
**Root Cause:** execute_import_plan024() called deprecated `phase_processing_plan024()` instead of `phase_processing_per_file()`

**Test Gap:**
- NO test verifies execute_import_plan024() calls correct processing method
- NO test verifies per-file vs. batch architecture
- NO integration test for execute_import_plan024()

**Would Test Have Caught This?** ✅ YES
- Integration test calling execute_import_plan024() would fail if batch extraction occurs
- Log assertion test checking for absence of batch metadata extraction messages

### Issue 2: Embedded Batch Extraction in phase_scanning
**Root Cause:** phase_scanning.rs had 270 lines of embedded batch processing (hashing + metadata extraction) that ran BEFORE per-file pipeline

**Test Gap:**
- NO test for phase_scanning behavior
- NO test verifies phase_scanning creates file records only (no processing)
- NO test checks that MetadataExtractor NOT called during scanning phase

**Would Test Have Caught This?** ✅ YES
- Unit test mocking MetadataExtractor would detect unexpected calls
- Integration test checking log output for metadata extraction during SCANNING phase

### Issue 3: Database Schema Mismatch (session_id)
**Root Cause:** Code tried to set session_id on AudioFile records, but files table doesn't have session_id column per SPEC031 zero-conf design

**Test Gap:**
- NO test verifies AudioFile::new() signature matches database schema
- NO test for database query correctness in phase_processing_per_file()
- NO schema validation test for SPEC031 compliance

**Would Test Have Caught This?** ⚠️ PARTIAL
- Database integration test would catch this during INSERT
- Would require actual database schema (not just in-memory SQLite)
- Current tests use fixtures/mocks that may not match production schema

### Issue 4: File Path Corruption
**Root Cause:** Database stores relative paths, but process_file_plan024_with_decoding() received relative path instead of absolute path for audio decoding

**Test Gap:**
- NO test for path handling in process_single_file_with_context()
- NO test verifies relative → absolute path conversion
- NO test for audio decoding with relative vs. absolute paths

**Would Test Have Caught This?** ✅ YES
- Integration test attempting to decode audio file would fail
- Unit test verifying path join logic would catch this

---

## 2. Existing Test Coverage

### 2.1 Workflow State Machine Tests
**File:** `wkmp-ai/tests/workflow_tests.rs`
**Count:** 12 unit tests
**Coverage:** ✅ GOOD

**Tests:**
- TC-WF-001 through TC-WF-012: State transitions (SCANNING → EXTRACTING → FINGERPRINTING → etc.)
- Session state persistence
- UUID generation
- Progress tracking
- Terminal states

**Limitations:**
- ❌ Tests use DEPRECATED batch-phase states (Extracting, Segmenting, Fingerprinting)
- ❌ No tests for PLAN024 states (SCANNING → PROCESSING → COMPLETED)
- ❌ Tests verify state machine logic but NOT actual workflow execution

**Relevance to Recent Issues:** ❌ NONE - Would not catch architectural issues

### 2.2 Workflow Integration Tests
**File:** `wkmp-ai/tests/workflow_integration.rs`
**Count:** 5 async integration tests
**Coverage:** ⚠️ PARTIAL (PLAN023 only, not PLAN024)

**Tests:**
- test_workflow_with_empty_config
- test_workflow_with_audio_derived_only
- test_event_bridge_integration
- test_boundary_detection_short_file
- test_fusion_with_no_extractions

**Architecture:** Tests PLAN023 per-song workflow (workflow::song_processor), NOT PLAN024 per-file import pipeline (services::workflow_orchestrator)

**Relevance to Recent Issues:** ❌ NONE - Tests different code path entirely

### 2.3 Component Service Tests
**File:** `wkmp-ai/tests/component_tests.rs`
**Count:** 18 unit tests
**Coverage:** ✅ GOOD for individual components

**Tests:**
- TC-COMP-001/002: FileScanner (directory traversal, symlink cycles)
- TC-COMP-003/004: MetadataExtractor (ID3 parsing, FLAC tags)

**Limitations:**
- ❌ Tests individual services in isolation
- ❌ Does NOT test orchestrator integration
- ❌ Does NOT verify services called in correct order or context

**Relevance to Recent Issues:** ❌ NONE - Would not detect batch extraction in orchestrator

### 2.4 Workflow Orchestrator Unit Tests
**File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs` lines 2637-2731
**Count:** 3 placeholder tests
**Coverage:** ❌ INSUFFICIENT

**Tests:**
- test_orchestrator_creation: Placeholder (assert!(true))
- tc_u_pipe_010_01_segmentation_before_fingerprinting: Structural verification only
- tc_u_pipe_020_01_four_workers_configured: Structural verification only
- tc_u_pipe_020_02_per_file_processing: Structural verification only

**Issues:**
- ❌ All tests are placeholders or structural checks (no actual execution)
- ❌ NO integration tests
- ❌ NO tests for execute_import(), execute_import_plan024()
- ❌ NO tests for phase_scanning(), phase_processing_per_file()
- ❌ Tests reference PLAN025 (obsolete), not PLAN024

**Relevance to Recent Issues:** ❌ NONE - Placeholder tests provide no verification

### 2.5 System Tests
**File:** `wkmp-ai/tests/system_tests.rs`
**Count:** 3 async tests
**Coverage:** ⚠️ PARTIAL (PLAN023 only)

**Tests:**
- test_system_multi_file_import: Tests PLAN023 song_processor (not orchestrator)
- test_system_performance_validation: Performance benchmarks
- test_system_database_persistence: Ignored (requires full schema)

**Relevance to Recent Issues:** ❌ NONE - Tests different workflow entirely

### 2.6 Other Test Files
**Files:** 15 total test files
**Count:** ~97 total tests across all files

**Coverage Areas:**
- API integration tests
- Concurrent session tests
- Config validation tests
- HTTP server tests
- Recovery/error handling tests
- Settings API tests
- Parallel file scanning tests
- Parallel fingerprinting tests

**Relevance to Recent Issues:** ❌ MINIMAL - No orchestrator workflow coverage

---

## 3. Test Coverage Gaps (Critical)

### Gap 1: NO Architecture Compliance Tests
**Risk Level:** 🔴 CRITICAL

**Missing Tests:**
1. Verify per-file pipeline architecture (not batch phases)
2. Verify no batch metadata extraction during SCANNING
3. Verify each file processes through all 10 phases before next file
4. Verify FuturesUnordered worker pool maintains N workers

**Impact:** Recent issues went undetected because no tests verify architectural compliance

### Gap 2: NO Entry Point Integration Tests
**Risk Level:** 🔴 CRITICAL

**Missing Tests:**
1. execute_import_plan024() end-to-end test
2. execute_import() with real audio files
3. Workflow progression through SCANNING → PROCESSING → COMPLETED states
4. Cancellation token handling

**Impact:** execute_import_plan024() called wrong method, went undetected

### Gap 3: NO Phase Behavior Tests
**Risk Level:** 🔴 CRITICAL

**Missing Tests:**
1. phase_scanning() creates file records only (no processing)
2. phase_scanning() does NOT call MetadataExtractor
3. phase_processing_per_file() processes files through 10-phase pipeline
4. Early exit conditions (AlreadyProcessed, DuplicateHash, NoAudio)

**Impact:** 270 lines of embedded batch extraction in phase_scanning went undetected

### Gap 4: NO Database Schema Validation Tests
**Risk Level:** 🟡 HIGH

**Missing Tests:**
1. AudioFile model matches files table schema (SPEC031)
2. Queries in phase_processing_per_file() match schema
3. session_id column absence verified (zero-conf design)

**Impact:** session_id schema mismatch occurred at runtime instead of test time

### Gap 5: NO Path Handling Tests
**Risk Level:** 🟡 MEDIUM

**Missing Tests:**
1. Relative → absolute path conversion
2. Database stores relative paths, audio decoding uses absolute paths
3. Path joining in process_single_file_with_context()

**Impact:** File path corruption occurred during manual testing

---

## 4. Test Coverage by Component

| Component | Unit Tests | Integration Tests | Architecture Tests | Coverage Rating |
|-----------|------------|-------------------|-------------------|-----------------|
| **ImportSession state machine** | ✅ 12 tests | ❌ None | ❌ None | ⚠️ PARTIAL |
| **FileScanner** | ✅ 2 tests | ❌ None | ❌ None | ⚠️ PARTIAL |
| **MetadataExtractor** | ✅ 2 tests | ❌ None | ❌ None | ⚠️ PARTIAL |
| **WorkflowOrchestrator** | ❌ 3 placeholders | ❌ None | ❌ None | 🔴 CRITICAL GAP |
| **phase_scanning** | ❌ None | ❌ None | ❌ None | 🔴 CRITICAL GAP |
| **phase_processing_per_file** | ❌ None | ❌ None | ❌ None | 🔴 CRITICAL GAP |
| **execute_import_plan024** | ❌ None | ❌ None | ❌ None | 🔴 CRITICAL GAP |
| **PLAN024 per-file pipeline** | ❌ None | ❌ None | ❌ None | 🔴 CRITICAL GAP |
| **Database schema (SPEC031)** | ❌ None | ⚠️ Partial | ❌ None | 🟡 HIGH GAP |

---

## 5. Recommended Regression Prevention Tests

### Test 1: Architecture Compliance Verification (CRITICAL)
**Priority:** P0
**Type:** Integration Test
**File:** `wkmp-ai/tests/workflow_architecture_tests.rs` (NEW)

```rust
/// TC-ARCH-001: Verify NO batch metadata extraction occurs during import
///
/// **Requirement:** SPEC032 [AIA-ASYNC-020] Per-file pipeline architecture
///
/// **Given:** Import workflow processing multiple audio files
/// **When:** execute_import_plan024() runs
/// **Then:**
///   - NO batch metadata extraction logs appear
///   - Each file processes individually through 10-phase pipeline
///   - Files show "Phase 1" through "Phase 10" logs sequentially per file
///
/// **Verification Method:**
///   - Capture tracing logs during import
///   - Assert NO logs match pattern: "Extracting metadata from N files"
///   - Assert logs show per-file progression: "Processing file 1/5: Phase 1", "Processing file 1/5: Phase 2", etc.
#[tokio::test]
async fn tc_arch_001_no_batch_metadata_extraction() {
    // Setup: Create temp database, generate 3 test audio files
    // Execute: Call execute_import_plan024() with test root folder
    // Verify: Capture logs, assert NO batch extraction pattern
    // Verify: Logs show per-file processing pattern
}
```

### Test 2: phase_scanning Behavior Verification (CRITICAL)
**Priority:** P0
**Type:** Unit Test
**File:** `wkmp-ai/tests/workflow_phase_tests.rs` (NEW)

```rust
/// TC-PHASE-001: phase_scanning creates file records only (no processing)
///
/// **Requirement:** SPEC032 SCANNING phase - file discovery only
///
/// **Given:** Orchestrator with mocked services
/// **When:** phase_scanning() executes
/// **Then:**
///   - FileScanner.scan() called exactly once
///   - MetadataExtractor NEVER called
///   - Database contains file records with path, mod_time, size
///   - Database file records have EMPTY hash field
///   - NO metadata fields populated
///
/// **Verification Method:**
///   - Mock MetadataExtractor to panic if called
///   - Assert file records have empty/null processing fields
#[tokio::test]
async fn tc_phase_001_scanning_no_processing() {
    // Setup: Mock orchestrator, generate test files
    // Mock: MetadataExtractor panics if extract() called
    // Execute: Call phase_scanning()
    // Verify: No panic (MetadataExtractor not called)
    // Verify: Database has file records with empty hash
}
```

### Test 3: execute_import_plan024 Integration Test (CRITICAL)
**Priority:** P0
**Type:** Integration Test
**File:** `wkmp-ai/tests/workflow_orchestrator_integration_tests.rs` (NEW)

```rust
/// TC-ORCH-001: execute_import_plan024 processes files through per-file pipeline
///
/// **Requirement:** SPEC032 [AIA-ASYNC-020] Per-file pipeline orchestration
///
/// **Given:** Import session with 3 test audio files
/// **When:** execute_import_plan024() runs to completion
/// **Then:**
///   - Session transitions: SCANNING → PROCESSING → COMPLETED
///   - All 3 files have status 'INGEST COMPLETE' or early exit status
///   - Database contains passage records for each file
///   - Progress shows "Processing X to Y of Z" format
///
/// **Verification Method:**
///   - Generate 3 test WAV files (30-45 seconds each)
///   - Call execute_import_plan024() end-to-end
///   - Query database for file/passage records
///   - Assert state transitions correct
#[tokio::test]
async fn tc_orch_001_execute_import_plan024_end_to_end() {
    // Setup: Create temp database, generate 3 test audio files
    // Execute: Call execute_import_plan024()
    // Verify: Session reaches COMPLETED state
    // Verify: Database has 3 files + passages
}
```

### Test 4: Database Schema Validation (HIGH PRIORITY)
**Priority:** P1
**Type:** Unit Test
**File:** `wkmp-ai/tests/database_schema_validation_tests.rs` (NEW)

```rust
/// TC-DB-001: AudioFile model matches files table schema (SPEC031)
///
/// **Requirement:** SPEC031 Zero-conf database (no session_id tracking)
///
/// **Given:** SQLite database with files table schema
/// **When:** AudioFile::new() creates record
/// **Then:**
///   - AudioFile struct fields match files table columns exactly
///   - NO session_id field present (SPEC031 zero-conf)
///   - Relative path stored, not absolute path
///
/// **Verification Method:**
///   - Query SQLite schema: PRAGMA table_info(files)
///   - Compare to AudioFile struct fields
///   - Assert no session_id column exists
#[tokio::test]
async fn tc_db_001_audio_file_schema_matches_table() {
    // Setup: Create temp database with migrations
    // Query: PRAGMA table_info(files)
    // Verify: No session_id column
    // Verify: AudioFile::new() INSERT succeeds
}
```

### Test 5: Path Handling Correctness (MEDIUM PRIORITY)
**Priority:** P2
**Type:** Unit Test
**File:** `wkmp-ai/tests/path_handling_tests.rs` (NEW)

```rust
/// TC-PATH-001: Relative paths stored, absolute paths used for decoding
///
/// **Requirement:** Database stores relative paths, audio operations use absolute
///
/// **Given:** File record with relative path "Artist/Album/Track.mp3"
/// **When:** process_single_file_with_context() processes file
/// **Then:**
///   - Database query returns relative path
///   - Absolute path constructed by joining with root folder
///   - Audio decoder receives absolute path
///
/// **Verification Method:**
///   - Mock file system with known root folder
///   - Create file record with relative path
///   - Call process_single_file_with_context()
///   - Capture path passed to audio decoder
///   - Assert path is absolute
#[tokio::test]
async fn tc_path_001_relative_to_absolute_conversion() {
    // Setup: Create temp file with known relative path
    // Execute: Call process_single_file_with_context()
    // Verify: Audio decoder receives absolute path
}
```

---

## 6. Implementation Priority

### Phase 1: Critical Architecture Tests (Week 1)
**Estimated Effort:** 8-12 hours

1. **TC-ARCH-001**: No batch metadata extraction (4 hours)
   - Highest priority - prevents primary issue
   - Integration test with real orchestrator
   - Log pattern verification

2. **TC-PHASE-001**: phase_scanning behavior (2 hours)
   - Verify scanning does no processing
   - Mock-based unit test

3. **TC-ORCH-001**: execute_import_plan024 end-to-end (4-6 hours)
   - Complete integration test
   - Requires test audio file generation
   - Database verification

### Phase 2: Schema and Path Tests (Week 2)
**Estimated Effort:** 4-6 hours

4. **TC-DB-001**: Database schema validation (2-3 hours)
   - Schema introspection test
   - Prevents session_id issue

5. **TC-PATH-001**: Path handling correctness (2-3 hours)
   - Unit test with mock file system
   - Prevents path corruption

---

## 7. Test Infrastructure Requirements

### 7.1 Test Audio File Generation
**Current:** workflow_integration.rs has `generate_test_wav()` helper
**Need:** Reusable test fixture generator

**Recommendation:**
- Create `wkmp-ai/tests/helpers/audio_generator.rs`
- Support multiple formats: WAV, MP3, FLAC
- Configurable duration, silence patterns
- Pre-generated fixtures in `tests/fixtures/` directory

### 7.2 Database Test Utilities
**Current:** Tests use in-memory SQLite or tempfile
**Need:** Schema validation utilities

**Recommendation:**
- Create `wkmp-ai/tests/helpers/db_utils.rs`
- Schema introspection functions
- Fixture data seeding
- Transaction rollback for test isolation

### 7.3 Log Capture Infrastructure
**Current:** NO log capture in tests
**Need:** Tracing subscriber for test assertions

**Recommendation:**
- Create `wkmp-ai/tests/helpers/log_capture.rs`
- Use `tracing-subscriber` test layer
- Provide assertions: `assert_no_logs_match!(pattern)`, `assert_logs_match!(pattern)`

### 7.4 Mock Service Builders
**Current:** Tests instantiate real services
**Need:** Mock/stub builders for isolated testing

**Recommendation:**
- Use `mockall` or manual mocks for WorkflowOrchestrator dependencies
- Create `wkmp-ai/tests/helpers/mocks.rs`
- Provide builders: `MockMetadataExtractor`, `MockFileScanner`, etc.

---

## 8. Continuous Integration Recommendations

### 8.1 Test Categories
Organize tests by execution time and dependencies:

```toml
# Cargo.toml
[[test]]
name = "unit"
path = "tests/unit/mod.rs"

[[test]]
name = "integration"
path = "tests/integration/mod.rs"
required-features = ["test-integration"]

[[test]]
name = "architecture"
path = "tests/architecture/mod.rs"
required-features = ["test-integration"]
```

### 8.2 CI Pipeline Stages
1. **Fast Unit Tests** (<1 min): Run on every commit
2. **Integration Tests** (<5 min): Run on PR creation
3. **Architecture Tests** (<10 min): Run before merge to main
4. **System Tests** (>10 min): Nightly builds

### 8.3 Failure Detection
Add CI checks for:
- ❌ FAIL if logs contain "Extracting metadata from N files" during import
- ❌ FAIL if deprecated batch-phase methods called
- ⚠️ WARN if test execution time exceeds baseline (performance regression)

---

## 9. Traceability Matrix

| Issue | Test ID | Test Name | Coverage | Status |
|-------|---------|-----------|----------|--------|
| **Batch extraction in execute_import_plan024** | TC-ARCH-001 | No batch metadata extraction | ✅ FULL | 🔴 NOT IMPLEMENTED |
| **Batch extraction in execute_import_plan024** | TC-ORCH-001 | execute_import_plan024 end-to-end | ✅ FULL | 🔴 NOT IMPLEMENTED |
| **Embedded batch extraction in phase_scanning** | TC-PHASE-001 | phase_scanning no processing | ✅ FULL | 🔴 NOT IMPLEMENTED |
| **session_id schema mismatch** | TC-DB-001 | AudioFile schema validation | ✅ FULL | 🔴 NOT IMPLEMENTED |
| **File path corruption** | TC-PATH-001 | Relative to absolute conversion | ✅ FULL | 🔴 NOT IMPLEMENTED |

---

## 10. Conclusion

### Current State
- ✅ **Good coverage** for state machine logic and individual services
- ❌ **ZERO coverage** for workflow orchestrator integration
- ❌ **ZERO coverage** for architectural compliance (per-file vs. batch)
- ❌ **ZERO coverage** for phase behavior verification

### Impact on Recent Issues
**ALL 4 recent issues would have been caught by recommended tests:**
1. ✅ TC-ARCH-001 would detect batch extraction
2. ✅ TC-ORCH-001 would detect wrong method call
3. ✅ TC-PHASE-001 would detect embedded processing
4. ⚠️ TC-DB-001 would catch session_id at test time (not runtime)
5. ✅ TC-PATH-001 would detect path corruption

### Risk Assessment
**Current Risk Level:** 🔴 **HIGH**

**Rationale:**
- Architectural changes have NO automated verification
- Manual testing is primary quality gate (caught issues after commit)
- Regression risk remains high for future changes

### Recommendation
**Implement 5 critical tests (Phase 1 + Phase 2) before next major feature:**
- Estimated effort: 12-18 hours
- Prevents recurrence of all 4 recent issues
- Provides confidence for future PLAN024 work

### Success Metrics
After implementing recommended tests:
- ✅ CI fails if batch extraction occurs
- ✅ CI fails if wrong architecture used
- ✅ CI catches database schema mismatches
- ✅ 80%+ coverage for workflow orchestrator integration
- ✅ Manual testing finds fewer issues (shifted left to automated tests)

---

## Appendices

### Appendix A: Test File Organization

**Proposed Structure:**
```
wkmp-ai/tests/
├── unit/                           # Fast unit tests (<100ms each)
│   ├── mod.rs
│   ├── phase_scanning_tests.rs     # NEW: TC-PHASE-001
│   └── path_handling_tests.rs      # NEW: TC-PATH-001
├── integration/                    # Integration tests (1-5s each)
│   ├── mod.rs
│   ├── orchestrator_tests.rs       # NEW: TC-ORCH-001
│   └── database_schema_tests.rs    # NEW: TC-DB-001
├── architecture/                   # NEW: Architecture compliance (5-30s each)
│   ├── mod.rs
│   └── pipeline_architecture_tests.rs  # NEW: TC-ARCH-001
├── helpers/                        # Shared test utilities
│   ├── mod.rs
│   ├── audio_generator.rs          # NEW: Test audio file generation
│   ├── db_utils.rs                 # NEW: Database test utilities
│   ├── log_capture.rs              # NEW: Tracing log capture
│   └── mocks.rs                    # NEW: Service mocks
└── fixtures/                       # Pre-generated test data
    ├── README.md
    ├── test_tagged.mp3
    └── generate_fixtures.sh
```

### Appendix B: Test Count Summary

**Current Test Counts:**
- Total test files: 15
- Total test functions: ~97
- WorkflowOrchestrator tests: 3 (all placeholders)
- PLAN024-specific tests: 0

**After Implementation:**
- Total test files: 19 (+4 new)
- Total test functions: ~102 (+5 new)
- WorkflowOrchestrator tests: 8 (+5 real tests)
- PLAN024-specific tests: 5 (NEW)

**Coverage Improvement:**
- Before: 0% orchestrator integration coverage
- After: ~80% orchestrator integration coverage (5 critical paths tested)

---

**Document Version:** 1.0
**Last Updated:** 2025-11-13
**Author:** Claude Code (automated analysis)
**Related Documents:**
- [wip/PLAN024_architecture_discrepancy_analysis.md](PLAN024_architecture_discrepancy_analysis.md) - Root cause analysis
- [wip/PLAN024_phase_7_ui_statistics_remaining.md](PLAN024_phase_7_ui_statistics_remaining.md) - UI statistics work
- [wip/wkmp-ai_refinement.md](wkmp-ai_refinement.md) - Original SPEC032 specification

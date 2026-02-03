# PLAN026: Requirements Traceability Matrix

**Plan:** PLAN026 - Batch Writes Optimization
**Purpose:** Map requirements → tests → implementation files
**Created:** 2025-01-15

---

## Traceability Overview

**Requirements:** 11
**Tests:** 21
**Coverage:** 100% - Every requirement has ≥1 acceptance test

---

## Complete Traceability Matrix

| Requirement | Priority | Unit Tests | Integration Tests | System Tests | Manual Tests | Implementation File(s) | Status | Coverage |
|-------------|----------|------------|-------------------|--------------|--------------|------------------------|--------|----------|
| **REQ-BW-010** | P0 | TC-U-BW-010-01 | TC-I-BW-010-02 | - | - | wkmp-ai/src/services/workflow_orchestrator/mod.rs | Pending | Complete |
| **REQ-BW-020** | P0 | TC-U-BW-020-01 | TC-I-BW-020-02 | - | - | wkmp-ai/src/db/*.rs (batch functions) | Pending | Complete |
| **REQ-BW-030** | P1 | TC-U-BW-030-01 | TC-I-BW-030-02 | - | - | wkmp-ai/src/services/passage_recorder.rs (pattern) | Pending | Complete |
| **REQ-BW-040** | P1 | TC-U-BW-040-01 | TC-I-BW-040-02 | - | - | All batch write functions (transaction handling) | Pending | Complete |
| **REQ-BW-050** | P2 | TC-U-BW-050-01 | TC-I-BW-050-02 | - | - | wkmp-ai/src/utils/db_retry.rs (preserve existing) | Pending | Complete |
| **REQ-DC-010** | P0 | - | - | - | TC-M-DC-010-01, TC-M-DC-010-02 | wkmp-ai/src/**/*.rs (dead code removal) | Pending | Complete |
| **REQ-DC-020** | P0 | - | - | - | TC-M-DC-020-01, TC-M-DC-020-02 | wkmp-ai/src/**/*.rs (dead code removal) | Pending | Complete |
| **REQ-DC-030** | P1 | - | - | - | TC-M-DC-030-01 | wkmp-ai/src/**/*.rs (import cleanup) | Pending | Complete |
| **REQ-DC-040** | P1 | - | - | - | TC-M-DC-040-01 | wkmp-ai/src/**/*.rs (#[allow] annotations) | Pending | Complete |
| **REQ-NF-010** | P0 | - | TC-I-NF-010-01, TC-I-NF-010-02 | - | - | All modified files (coverage verification) | Pending | Complete |
| **REQ-NF-020** | P2 | - | - | TC-S-NF-020-01, TC-S-NF-020-02 | - | Full import pipeline (performance measurement) | Pending | Complete |

---

## Forward Traceability (Requirements → Tests)

### REQ-BW-010: Reduce Lock Acquisitions by 10-20×

**Tests Verifying This Requirement:**
1. **TC-U-BW-010-01** (Unit) - Measure baseline lock acquisitions
   - Verifies: Current lock count is measurable
   - Method: Count BEGIN TRANSACTION in logs

2. **TC-I-BW-010-02** (Integration) - Verify 10-20× reduction
   - Verifies: Post-optimization locks reduced by target factor
   - Method: Compare after/before ratio

**Implementation:** wkmp-ai/src/services/workflow_orchestrator/mod.rs
- Batch writes in phase_fingerprinting.rs
- Batch writes in phase_segmenting.rs
- Batch writes in phase_analyzing.rs
- Batch writes in phase_flavoring.rs

**Coverage:** ✅ Complete - Baseline + verification tests

---

### REQ-BW-020: Batch Writes in Transactions

**Tests Verifying This Requirement:**
1. **TC-U-BW-020-01** (Unit) - Single transaction per batch
   - Verifies: Only one BEGIN/COMMIT per batch operation
   - Method: Mock database, count transaction calls

2. **TC-I-BW-020-02** (Integration) - Transaction duration <100ms
   - Verifies: Fast transactions (minimal connection hold time)
   - Method: Measure actual transaction execution time

**Implementation:** wkmp-ai/src/db/*.rs
- Batch insert functions for songs, artists, albums, passages
- Transaction wrappers around batch operations

**Coverage:** ✅ Complete - Behavior + performance tests

---

### REQ-BW-030: Pre-fetch Reads Outside Transactions

**Tests Verifying This Requirement:**
1. **TC-U-BW-030-01** (Unit) - Reads execute before transaction
   - Verifies: SELECT queries occur before BEGIN TRANSACTION
   - Method: Mock database, verify query order

2. **TC-I-BW-030-02** (Integration) - Cached reads used in transaction
   - Verifies: Transaction uses pre-fetched data, no SELECT within
   - Method: Monitor SQL execution, verify no reads inside transaction

**Implementation:** wkmp-ai/src/services/passage_recorder.rs (pattern reference)
- Apply pattern to other write-heavy services

**Coverage:** ✅ Complete - Ordering + usage tests

---

### REQ-BW-040: Maintain Transaction Atomicity

**Tests Verifying This Requirement:**
1. **TC-U-BW-040-01** (Unit) - Rollback on transaction failure
   - Verifies: Transaction failure triggers rollback
   - Method: Inject error mid-transaction, verify rollback

2. **TC-I-BW-040-02** (Integration) - No partial commits
   - Verifies: Database remains consistent after errors
   - Method: Verify no orphaned records after transaction failure

**Implementation:** All batch write functions
- Proper transaction error handling
- Rollback on any failure in batch

**Coverage:** ✅ Complete - Error handling + consistency tests

---

### REQ-BW-050: Preserve Retry Logic

**Tests Verifying This Requirement:**
1. **TC-U-BW-050-01** (Unit) - retry_on_lock wraps batch operations
   - Verifies: Batch write calls use retry_on_lock()
   - Method: Code review + unit test verification

2. **TC-I-BW-050-02** (Integration) - Retry behavior on lock contention
   - Verifies: Lock errors trigger retry with backoff
   - Method: Simulate lock contention, verify retry attempts

**Implementation:** wkmp-ai/src/utils/db_retry.rs (preserve existing)
- No changes needed, just verify usage continues

**Coverage:** ✅ Complete - Usage + behavior tests

---

### REQ-DC-010: Pre-Implementation Dead Code Removal

**Tests Verifying This Requirement:**
1. **TC-M-DC-010-01** (Manual) - cargo clippy --all-targets
   - Verifies: Zero unused warnings before implementation
   - Method: Run clippy, check output

2. **TC-M-DC-010-02** (Manual) - Tests pass after removal
   - Verifies: Dead code removal doesn't break functionality
   - Method: Run cargo test, verify all pass

**Implementation:** wkmp-ai/src/**/*.rs
- Remove unused functions identified by clippy
- Remove unused modules
- Clean up dead code paths

**Coverage:** ✅ Complete - Detection + verification tests

---

### REQ-DC-020: Post-Implementation Dead Code Removal

**Tests Verifying This Requirement:**
1. **TC-M-DC-020-01** (Manual) - cargo clippy --all-targets
   - Verifies: Zero unused warnings after optimization
   - Method: Run clippy, check output

2. **TC-M-DC-020-02** (Manual) - Tests pass after removal
   - Verifies: Cleanup doesn't break functionality
   - Method: Run cargo test, verify all pass

**Implementation:** wkmp-ai/src/**/*.rs
- Remove obsolete write paths replaced by batch writes
- Remove unused helper functions

**Coverage:** ✅ Complete - Detection + verification tests

---

### REQ-DC-030: Remove Unused Imports

**Tests Verifying This Requirement:**
1. **TC-M-DC-030-01** (Manual) - Zero unused import warnings
   - Verifies: No "unused import" warnings in build output
   - Method: cargo build, check for warnings

**Implementation:** wkmp-ai/src/**/*.rs
- Remove unused use statements
- Clean up import lists

**Coverage:** ✅ Complete - Single comprehensive test

---

### REQ-DC-040: Document Retained Dead Code

**Tests Verifying This Requirement:**
1. **TC-M-DC-040-01** (Manual) - Documentation review
   - Verifies: Retained dead code has #[allow(dead_code)] + comment
   - Method: Code review, check annotations

**Implementation:** wkmp-ai/src/**/*.rs
- Add #[allow(dead_code)] where needed
- Document rationale in comments

**Coverage:** ✅ Complete - Documentation verification

---

### REQ-NF-010: No Test Coverage Regression

**Tests Verifying This Requirement:**
1. **TC-I-NF-010-01** (Integration) - Measure baseline coverage
   - Verifies: Baseline coverage % is known
   - Method: Run cargo tarpaulin, record %

2. **TC-I-NF-010-02** (Integration) - Coverage ≥ baseline
   - Verifies: Post-implementation coverage not decreased
   - Method: Run cargo tarpaulin, compare to baseline

**Implementation:** All modified files
- Test coverage verification (not implementation per se)

**Coverage:** ✅ Complete - Baseline + verification tests

---

### REQ-NF-020: Measurable Throughput Improvement

**Tests Verifying This Requirement:**
1. **TC-S-NF-020-01** (System) - Baseline throughput benchmark
   - Verifies: Current import speed is measurable
   - Method: Import 100 files, measure wall clock time

2. **TC-S-NF-020-02** (System) - Post-optimization throughput
   - Verifies: Throughput improvement is measurable
   - Method: Import same 100 files, compare time

**Implementation:** Full import pipeline
- Performance measurement (informational)

**Coverage:** ✅ Complete - Baseline + verification tests

---

## Backward Traceability (Tests → Requirements)

### Unit Tests → Requirements

| Test ID | Requirement(s) Verified | Purpose |
|---------|------------------------|---------|
| TC-U-BW-010-01 | REQ-BW-010 | Baseline measurement |
| TC-U-BW-020-01 | REQ-BW-020 | Single transaction verification |
| TC-U-BW-030-01 | REQ-BW-030 | Read ordering verification |
| TC-U-BW-040-01 | REQ-BW-040 | Rollback verification |
| TC-U-BW-050-01 | REQ-BW-050 | retry_on_lock usage |

### Integration Tests → Requirements

| Test ID | Requirement(s) Verified | Purpose |
|---------|------------------------|---------|
| TC-I-BW-010-02 | REQ-BW-010 | Lock reduction measurement |
| TC-I-BW-020-02 | REQ-BW-020 | Transaction performance |
| TC-I-BW-030-02 | REQ-BW-030 | Cached reads usage |
| TC-I-BW-040-02 | REQ-BW-040 | Consistency verification |
| TC-I-BW-050-02 | REQ-BW-050 | Retry behavior |
| TC-I-NF-010-01 | REQ-NF-010 | Baseline coverage |
| TC-I-NF-010-02 | REQ-NF-010 | Post-coverage verification |
| TC-I-REGR-01 | All Requirements | Regression prevention |

### System Tests → Requirements

| Test ID | Requirement(s) Verified | Purpose |
|---------|------------------------|---------|
| TC-S-NF-020-01 | REQ-NF-020 | Baseline throughput |
| TC-S-NF-020-02 | REQ-NF-020 | Post-optimization throughput |

### Manual Tests → Requirements

| Test ID | Requirement(s) Verified | Purpose |
|---------|------------------------|---------|
| TC-M-DC-010-01 | REQ-DC-010 | Pre-implementation clippy |
| TC-M-DC-010-02 | REQ-DC-010 | Pre-implementation test pass |
| TC-M-DC-020-01 | REQ-DC-020 | Post-implementation clippy |
| TC-M-DC-020-02 | REQ-DC-020 | Post-implementation test pass |
| TC-M-DC-030-01 | REQ-DC-030 | Unused imports check |
| TC-M-DC-040-01 | REQ-DC-040 | Documentation review |

---

## Coverage Analysis

### Requirements Coverage

**100% Coverage Achieved:**
- All 11 requirements have ≥1 acceptance test
- Critical requirements (P0) have 2+ tests each
- High requirements (P1) have 2 tests each
- Medium requirements (P2) have 2 tests each (informational)

### Test Type Distribution

**Unit Tests:** 5 (24%)
- Focus: Individual function behavior
- Fast execution, good for CI/CD

**Integration Tests:** 8 (38%)
- Focus: Component interactions, database behavior
- Moderate execution time, core verification

**System Tests:** 2 (10%)
- Focus: End-to-end pipeline performance
- Slow execution, informational

**Manual Tests:** 6 (29%)
- Focus: Code quality verification
- One-time execution per phase

### Priority Coverage

**P0 (Critical) Requirements:** 5 requirements
- Total tests: 10 (50% of all tests)
- Average: 2 tests per requirement
- Coverage: ✅ Complete

**P1 (High) Requirements:** 4 requirements
- Total tests: 7 (33% of all tests)
- Average: 1.75 tests per requirement
- Coverage: ✅ Complete

**P2 (Medium) Requirements:** 2 requirements
- Total tests: 4 (19% of all tests)
- Average: 2 tests per requirement
- Coverage: ✅ Complete (informational)

---

## Implementation Tracking

### Pre-Implementation Phase

**Files to Modify:**
- wkmp-ai/src/**/*.rs (dead code removal)

**Tests to Run:**
- TC-M-DC-010-01, TC-M-DC-010-02
- TC-U-BW-010-01 (baseline measurement)
- TC-I-NF-010-01 (baseline coverage)
- TC-S-NF-020-01 (baseline throughput)

**Status:** Pending

---

### Implementation Phase

**Files to Create/Modify:**
- wkmp-ai/src/db/songs.rs (add batch_insert_songs)
- wkmp-ai/src/db/artists.rs (add batch_insert_artists)
- wkmp-ai/src/db/albums.rs (add batch_insert_albums)
- wkmp-ai/src/db/passages.rs (add batch_insert_passages)
- wkmp-ai/src/services/workflow_orchestrator/mod.rs (use batch writes)
- wkmp-ai/src/services/workflow_orchestrator/phase_*.rs (refactor writes)

**Tests to Run:**
- All TC-U-BW-* (unit tests)
- All TC-I-BW-* (integration tests)
- TC-I-REGR-01 (regression check)

**Status:** Pending

---

### Post-Implementation Phase

**Files to Modify:**
- wkmp-ai/src/**/*.rs (dead code removal)

**Tests to Run:**
- TC-M-DC-020-01, TC-M-DC-020-02
- TC-M-DC-030-01
- TC-M-DC-040-01

**Status:** Pending

---

### Final Verification Phase

**Tests to Run:**
- TC-I-NF-010-02 (coverage verification)
- TC-S-NF-020-02 (throughput measurement)

**Status:** Pending

---

## Traceability Verification

### Gaps Check

**Requirements without tests:** None ✅
**Tests without requirements:** None ✅
**Implementation without tests:** N/A (pending implementation)

### Orphan Detection

**Orphaned Requirements:** None
**Orphaned Tests:** None
**Orphaned Implementation:** Will verify during implementation

---

## Sign-Off

**Traceability Matrix Complete:** 2025-01-15
**Coverage:** 100% (11/11 requirements)
**Total Tests:** 21
**Gaps:** None

**Status:** ✅ Ready for implementation

**Next:** Create detailed test specification files

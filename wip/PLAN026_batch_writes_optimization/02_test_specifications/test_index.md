# PLAN026: Test Specifications Index

**Plan:** PLAN026 - Batch Writes Optimization
**Phase:** 3 - Acceptance Test Definition
**Created:** 2025-01-15

---

## Test Summary

**Total Tests:** 21
- Unit Tests: 9
- Integration Tests: 8
- System Tests: 2
- Manual Tests: 2

**Coverage:** 100% - All 11 requirements have acceptance tests

---

## Test Index (Quick Reference)

| Test ID | Type | Requirement | One-Line Description | Priority |
|---------|------|-------------|----------------------|----------|
| TC-U-BW-010-01 | Unit | REQ-BW-010 | Measure baseline lock acquisitions per file | P0 |
| TC-I-BW-010-02 | Integration | REQ-BW-010 | Verify 10-20× lock reduction after optimization | P0 |
| TC-U-BW-020-01 | Unit | REQ-BW-020 | Verify single transaction per batch write | P0 |
| TC-I-BW-020-02 | Integration | REQ-BW-020 | Verify transaction duration <100ms | P0 |
| TC-U-BW-030-01 | Unit | REQ-BW-030 | Verify reads execute before transaction | P1 |
| TC-I-BW-030-02 | Integration | REQ-BW-030 | Verify cached reads used in transaction | P1 |
| TC-U-BW-040-01 | Unit | REQ-BW-040 | Verify rollback on transaction failure | P1 |
| TC-I-BW-040-02 | Integration | REQ-BW-040 | Verify no partial commits after error | P1 |
| TC-U-BW-050-01 | Unit | REQ-BW-050 | Verify retry_on_lock wraps batch operations | P2 |
| TC-I-BW-050-02 | Integration | REQ-BW-050 | Verify retry behavior on lock contention | P2 |
| TC-M-DC-010-01 | Manual | REQ-DC-010 | Run cargo clippy --all-targets (pre-implementation) | P0 |
| TC-M-DC-010-02 | Manual | REQ-DC-010 | Verify tests pass after dead code removal | P0 |
| TC-M-DC-020-01 | Manual | REQ-DC-020 | Run cargo clippy --all-targets (post-implementation) | P0 |
| TC-M-DC-020-02 | Manual | REQ-DC-020 | Verify tests pass after dead code removal | P0 |
| TC-M-DC-030-01 | Manual | REQ-DC-030 | Verify zero unused import warnings | P1 |
| TC-M-DC-040-01 | Manual | REQ-DC-040 | Verify retained dead code has documentation | P1 |
| TC-I-NF-010-01 | Integration | REQ-NF-010 | Measure baseline test coverage | P0 |
| TC-I-NF-010-02 | Integration | REQ-NF-010 | Verify coverage ≥ baseline after implementation | P0 |
| TC-S-NF-020-01 | System | REQ-NF-020 | Benchmark baseline import throughput | P2 |
| TC-S-NF-020-02 | System | REQ-NF-020 | Measure throughput after optimization | P2 |
| TC-I-REGR-01 | Integration | All | Verify all existing tests pass (regression check) | P0 |

---

## Tests by Requirement

### REQ-BW-010: Reduce Lock Acquisitions
- TC-U-BW-010-01: Measure baseline
- TC-I-BW-010-02: Verify reduction

### REQ-BW-020: Batch Writes in Transactions
- TC-U-BW-020-01: Single transaction per batch
- TC-I-BW-020-02: Transaction duration

### REQ-BW-030: Pre-fetch Reads
- TC-U-BW-030-01: Reads before transaction
- TC-I-BW-030-02: Cached reads used

### REQ-BW-040: Transaction Atomicity
- TC-U-BW-040-01: Rollback on failure
- TC-I-BW-040-02: No partial commits

### REQ-BW-050: Preserve Retry Logic
- TC-U-BW-050-01: retry_on_lock wraps operations
- TC-I-BW-050-02: Retry behavior verified

### REQ-DC-010: Pre-Implementation Dead Code
- TC-M-DC-010-01: cargo clippy check
- TC-M-DC-010-02: Tests pass after removal

### REQ-DC-020: Post-Implementation Dead Code
- TC-M-DC-020-01: cargo clippy check
- TC-M-DC-020-02: Tests pass after removal

### REQ-DC-030: Remove Unused Imports
- TC-M-DC-030-01: Zero unused import warnings

### REQ-DC-040: Document Retained Dead Code
- TC-M-DC-040-01: Documentation verification

### REQ-NF-010: No Test Coverage Regression
- TC-I-NF-010-01: Baseline measurement
- TC-I-NF-010-02: Post-implementation verification

### REQ-NF-020: Throughput Improvement
- TC-S-NF-020-01: Baseline benchmark
- TC-S-NF-020-02: Post-optimization measurement

---

## Tests by Type

### Unit Tests (9 tests)
Focus: Individual functions and components

| Test ID | Requirement | Description |
|---------|-------------|-------------|
| TC-U-BW-010-01 | REQ-BW-010 | Baseline lock measurement |
| TC-U-BW-020-01 | REQ-BW-020 | Single transaction verification |
| TC-U-BW-030-01 | REQ-BW-030 | Read timing verification |
| TC-U-BW-040-01 | REQ-BW-040 | Rollback verification |
| TC-U-BW-050-01 | REQ-BW-050 | retry_on_lock usage |

### Integration Tests (8 tests)
Focus: Component interactions, database behavior

| Test ID | Requirement | Description |
|---------|-------------|-------------|
| TC-I-BW-010-02 | REQ-BW-010 | Lock reduction measurement |
| TC-I-BW-020-02 | REQ-BW-020 | Transaction duration |
| TC-I-BW-030-02 | REQ-BW-030 | Cached reads usage |
| TC-I-BW-040-02 | REQ-BW-040 | No partial commits |
| TC-I-BW-050-02 | REQ-BW-050 | Retry behavior |
| TC-I-NF-010-01 | REQ-NF-010 | Baseline coverage |
| TC-I-NF-010-02 | REQ-NF-010 | Post-coverage verification |
| TC-I-REGR-01 | All | Regression check |

### System Tests (2 tests)
Focus: End-to-end import pipeline

| Test ID | Requirement | Description |
|---------|-------------|-------------|
| TC-S-NF-020-01 | REQ-NF-020 | Baseline throughput |
| TC-S-NF-020-02 | REQ-NF-020 | Post-optimization throughput |

### Manual Tests (6 tests)
Focus: Code quality verification

| Test ID | Requirement | Description |
|---------|-------------|-------------|
| TC-M-DC-010-01 | REQ-DC-010 | Pre-implementation clippy |
| TC-M-DC-010-02 | REQ-DC-010 | Pre-implementation test pass |
| TC-M-DC-020-01 | REQ-DC-020 | Post-implementation clippy |
| TC-M-DC-020-02 | REQ-DC-020 | Post-implementation test pass |
| TC-M-DC-030-01 | REQ-DC-030 | Unused imports check |
| TC-M-DC-040-01 | REQ-DC-040 | Documentation review |

---

## Test Execution Order

### Phase 1: Pre-Implementation (Before Batch Writes)
1. TC-M-DC-010-01: Run clippy to identify dead code
2. TC-M-DC-010-02: Remove dead code, verify tests pass
3. TC-U-BW-010-01: Measure baseline lock acquisitions
4. TC-I-NF-010-01: Measure baseline test coverage
5. TC-S-NF-020-01: Benchmark baseline import throughput

### Phase 2: Implementation (Batch Writes Optimization)
6. TC-U-BW-020-01: Verify batching implementation
7. TC-U-BW-030-01: Verify read pre-fetching
8. TC-U-BW-040-01: Verify transaction atomicity
9. TC-U-BW-050-01: Verify retry logic preserved
10. TC-I-BW-020-02: Measure transaction duration
11. TC-I-BW-030-02: Verify cached reads used
12. TC-I-BW-040-02: Verify no partial commits
13. TC-I-BW-050-02: Test retry behavior
14. TC-I-BW-010-02: Measure lock reduction
15. TC-I-REGR-01: Run all existing tests (regression)

### Phase 3: Post-Implementation (Dead Code Cleanup)
16. TC-M-DC-020-01: Run clippy for new dead code
17. TC-M-DC-020-02: Remove dead code, verify tests
18. TC-M-DC-030-01: Check for unused imports
19. TC-M-DC-040-01: Verify documentation of retained code

### Phase 4: Final Verification
20. TC-I-NF-010-02: Verify test coverage ≥ baseline
21. TC-S-NF-020-02: Measure final throughput

---

## Prerequisites

### Tools Required

**Rust Toolchain:**
- rustc 1.70+ (stable channel)
- cargo (latest)
- cargo-clippy (latest)

**Test Coverage Tool:**
- cargo-tarpaulin 0.25+ (resolves ISSUE-003)
- Installation: `cargo install cargo-tarpaulin`
- Alternative: cargo-llvm-cov (if tarpaulin unavailable on Windows)

**Benchmarking:**
- Standard Rust `std::time::Instant` (built-in)
- Test dataset: 100 audio files (FLAC/MP3 mix)

### Environment Setup

**Database:**
- SQLite 3.35+ with WAL mode enabled
- wkmp.db with schema from migrations/
- Empty database for clean benchmarks

**Configuration:**
- ai_database_max_lock_wait_ms = 5000
- ingest_max_concurrent_jobs = 12
- Root folder pointing to test dataset

**Test Dataset:**
- 100 audio files
- Mix of formats (FLAC, MP3, AAC)
- Total size: ~500MB recommended
- Location: test_data/100_file_dataset/

---

## Test Data

### Minimal Test Data (Unit Tests)
- Single audio file (test.flac)
- Pre-populated database with known entities
- Mock MusicBrainz responses

### Integration Test Data
- 10-file subset of full dataset
- Faster execution for CI/CD

### System Test Data
- Full 100-file dataset
- Representative of production workload
- Reproducible (same files for before/after benchmarks)

---

## Success Criteria Summary

**Must Pass (P0):**
- All unit tests (TC-U-*)
- All integration tests (TC-I-*)
- All manual tests showing zero warnings (TC-M-DC-*)
- Coverage ≥ baseline (TC-I-NF-010-02)
- Lock reduction 10-20× (TC-I-BW-010-02)

**Should Pass (P1):**
- Transaction duration <100ms (TC-I-BW-020-02)
- No unused imports (TC-M-DC-030-01)

**Informational (P2):**
- Throughput improvement (TC-S-NF-020-02)
- Any measurable improvement is success

---

## Detailed Test Specifications

See individual test specification files:
- tc_u_*.md - Unit test specifications
- tc_i_*.md - Integration test specifications
- tc_s_*.md - System test specifications
- tc_m_*.md - Manual test specifications

---

## Traceability Matrix

See: [traceability_matrix.md](traceability_matrix.md)

Complete mapping of requirements → tests → implementation files.

---

## Test Execution Log Template

```markdown
# PLAN026 Test Execution Log

**Date:** YYYY-MM-DD
**Tester:** [Name]
**Environment:** [dev/staging/prod]

## Phase 1: Pre-Implementation
- [ ] TC-M-DC-010-01: PASS / FAIL - [Notes]
- [ ] TC-M-DC-010-02: PASS / FAIL - [Notes]
- [ ] TC-U-BW-010-01: PASS / FAIL - Baseline: [N] locks/file
- [ ] TC-I-NF-010-01: PASS / FAIL - Baseline: [X]% coverage
- [ ] TC-S-NF-020-01: PASS / FAIL - Baseline: [Y] files/min

## Phase 2: Implementation
[... all implementation tests ...]

## Phase 3: Post-Implementation
[... all cleanup tests ...]

## Phase 4: Final Verification
- [ ] TC-I-NF-010-02: PASS / FAIL - Final: [X]% coverage
- [ ] TC-S-NF-020-02: PASS / FAIL - Final: [Y] files/min

## Summary
**Total Tests:** [N]
**Passed:** [N]
**Failed:** [N]
**Blocked:** [N]

**Recommendation:** APPROVE / REJECT / CONDITIONAL
```

---

## Sign-Off

**Test Index Created:** 2025-01-15
**Total Tests Defined:** 21
**Coverage:** 100% of requirements
**Status:** Ready for detailed test specification creation

**Next:** Create individual test specification files

# Test Index - Edition Filtering Improvements

**Plan:** PLAN027
**Date:** 2026-01-16
**Total Tests:** 32 (18 unit, 8 integration, 6 system)

---

## Test Summary by Requirement

| Requirement | Unit Tests | Integration Tests | System Tests | Total |
|-------------|-----------|-------------------|--------------|-------|
| REQ-EF-010 (Deluxe penalty) | 4 | 1 | 1 | 6 |
| REQ-EF-020 (Compilation penalty) | 4 | 1 | 1 | 6 |
| REQ-EF-030 (Track count filter) | 5 | 3 | 1 | 9 |
| REQ-EF-040 (Artist consistency) | 4 | 2 | 1 | 7 |
| REQ-EF-050 (Remix tolerance) | 3 | 1 | 1 | 5 |
| REQ-EF-060 (Zero regressions) | 0 | 0 | 3 | 3 |
| **Total** | **20** | **8** | **8** | **36** |

---

## Test Catalog

### Unit Tests (20 total)

| Test ID | Requirement | Test Name | Priority | Type |
|---------|-------------|-----------|----------|------|
| TC-U-010-01 | REQ-EF-010 | Deluxe keyword detection (lowercase) | High | Unit |
| TC-U-010-02 | REQ-EF-010 | Deluxe keyword detection (mixed case) | High | Unit |
| TC-U-010-03 | REQ-EF-010 | Expanded keyword detection | Medium | Unit |
| TC-U-010-04 | REQ-EF-010 | Score multiplier calculation (0.7×) | High | Unit |
| TC-U-020-01 | REQ-EF-020 | Compilation keyword detection (collection) | High | Unit |
| TC-U-020-02 | REQ-EF-020 | Anthology keyword detection | Medium | Unit |
| TC-U-020-03 | REQ-EF-020 | "Best of" keyword detection | Medium | Unit |
| TC-U-020-04 | REQ-EF-020 | Score multiplier calculation (0.6×) | High | Unit |
| TC-U-030-01 | REQ-EF-030 | Track count exact match (no filter) | High | Unit |
| TC-U-030-02 | REQ-EF-030 | Track count within ±3 (pass filter) | High | Unit |
| TC-U-030-03 | REQ-EF-030 | Track count >3 difference (filtered) | High | Unit |
| TC-U-030-04 | REQ-EF-030 | Empty edition list handling | Medium | Unit |
| TC-U-030-05 | REQ-EF-030 | All editions filtered (fallback) | High | Unit |
| TC-U-040-01 | REQ-EF-040 | Single artist (all tracks same) | High | Unit |
| TC-U-040-02 | REQ-EF-040 | Multiple artists ≤3 (accepted) | Medium | Unit |
| TC-U-040-03 | REQ-EF-040 | Multiple artists >3 (rejected) | High | Unit |
| TC-U-040-04 | REQ-EF-040 | Fuzzy artist matching (substring) | High | Unit |
| TC-U-050-01 | REQ-EF-050 | Remix keyword detection | High | Unit |
| TC-U-050-02 | REQ-EF-050 | Extended keyword detection | Medium | Unit |
| TC-U-050-03 | REQ-EF-050 | Error tolerance application (120s) | High | Unit |

### Integration Tests (8 total)

| Test ID | Requirement | Test Name | Priority | Type |
|---------|-------------|-----------|----------|------|
| TC-I-010-01 | REQ-EF-010 | Deluxe penalty integration with matching | High | Integration |
| TC-I-020-01 | REQ-EF-020 | Compilation penalty integration | High | Integration |
| TC-I-030-01 | REQ-EF-030 | Track count filter before Stage 2 | Critical | Integration |
| TC-I-030-02 | REQ-EF-030 | Track count filter with scoring | High | Integration |
| TC-I-030-03 | REQ-EF-030 | Fallback behavior (all filtered) | High | Integration |
| TC-I-040-01 | REQ-EF-040 | Artist consistency with Stage 2 | High | Integration |
| TC-I-040-02 | REQ-EF-040 | Multi-artist rejection flow | High | Integration |
| TC-I-050-01 | REQ-EF-050 | Remix tolerance in validation | Medium | Integration |

### System Tests (8 total)

| Test ID | Requirement | Test Name | Priority | Type |
|---------|-------------|-----------|----------|------|
| TC-S-010-01 | REQ-EF-010 | Problem album: Chemical Brothers (deluxe) | High | System |
| TC-S-020-01 | REQ-EF-020 | Problem album: James Gang (compilation) | High | System |
| TC-S-030-01 | REQ-EF-030 | Problem album: Imagine Dragons (track count) | Critical | System |
| TC-S-040-01 | REQ-EF-040 | Problem album: James Gang (multi-artist) | Critical | System |
| TC-S-050-01 | REQ-EF-050 | Problem album: Chemical Brothers (remix) | Medium | System |
| TC-S-060-01 | REQ-EF-060 | Full regression suite (200 albums) | Critical | System |
| TC-S-060-02 | REQ-EF-060 | Parameter tuning (multipliers) | Critical | System |
| TC-S-060-03 | REQ-EF-060 | Parameter tuning (track count tolerance) | Critical | System |

---

## Test Execution Priority

### Phase 1: Unit Tests (Parallel, ~2 hours)
All unit tests can run in parallel. Must pass 100% before integration tests.

### Phase 2: Integration Tests (Sequential, ~1 hour)
Run after unit tests pass. Verify filtering integrates with matching pipeline.

### Phase 3: System Tests (Sequential, ~6 hours)
- TC-S-060-02, TC-S-060-03: Parameter tuning (find optimal values)
- TC-S-010-01 through TC-S-050-01: Problem album tests (with tuned parameters)
- TC-S-060-01: Full regression suite (final validation)

**Total Estimated Test Time:** 9-10 hours (including 3× full test runs for parameter tuning)

---

## Test Data Requirements

### Unit Tests
- Mock edition objects with various titles ("Deluxe", "Compilation", etc.)
- Mock track lists with 1-20 tracks
- Artist name variations ("The Beatles", "Beatles", etc.)

### Integration Tests
- Subset of real albums (5-10 albums per test)
- Known MusicBrainz edition metadata

### System Tests
- Full 200-album test library (C:\Users\Mango Cat\Music)
- 4 problem albums (Chemical Brothers, Imagine Dragons, Michael Jackson, James Gang)
- Baseline comparison data (baseline_1-12-26_stats.json)

---

## Pass Criteria Summary

**Unit Tests:** 100% pass (20/20)
**Integration Tests:** 100% pass (8/8)
**System Tests:**
- Problem albums: 3/4 fixed (75% target)
- Regression suite: ≥186/200 pass (zero regressions)
- Parameter tuning: Optimal values identified

**Overall:** 36/36 tests pass

---

## Test File Organization

```
02_test_specifications/
├── test_index.md (this file)
├── traceability_matrix.md
├── unit_tests/
│   ├── tc_u_010_01_deluxe_lowercase.md
│   ├── tc_u_010_02_deluxe_mixed_case.md
│   ├── ... (20 unit test files)
├── integration_tests/
│   ├── tc_i_010_01_deluxe_integration.md
│   ├── ... (8 integration test files)
└── system_tests/
    ├── tc_s_010_01_chemical_brothers.md
    ├── tc_s_060_01_full_regression.md
    ├── tc_s_060_02_parameter_tuning_multipliers.md
    └── tc_s_060_03_parameter_tuning_tolerance.md
```

---

## Test Automation

**Unit Tests:** Automated via Rust `#[test]` functions
**Integration Tests:** Automated via Rust integration tests in `tests/` folder
**System Tests:** Semi-automated (manual test execution, automated result comparison)

**Test Runner:**
```bash
# Unit tests
cargo test --lib edition_filter

# Integration tests
cargo test --test edition_filter_integration

# System tests (manual)
RUST_LOG=debug cargo test --release test_run29f_full > test_output.log
python compare_results.py baseline.json current.json
```

---

## Detailed Test Specifications

See individual test specification files in subdirectories for complete Given/When/Then/Verify details.

# Test Index - PLAN018: Centralized Global Parameters

## Quick Reference

**Total Tests:** 29 (21 unit, 6 integration, 2 manual)
**Requirements Coverage:** 100% (9/9 requirements)
**Parameter Coverage:** 100% (15/15 parameters)

---

## Tests by Requirement

| Requirement | Test IDs | Count | Type Mix |
|-------------|----------|-------|----------|
| FR-001 | TC-U-001-01, TC-U-001-02 | 2 | 2 unit |
| FR-002 | TC-U-002-01, TC-U-002-02, TC-U-002-03 | 3 | 3 unit |
| FR-003 | TC-U-003-01, TC-M-003-01 | 2 | 1 unit, 1 manual |
| FR-004 | TC-I-004-01, TC-I-004-02, TC-I-004-03, TC-I-004-04 | 4 | 4 integration |
| FR-005 | TC-U-005-01 | 1 | 1 unit |
| NFR-001 | TC-U-101-01, TC-U-101-02 | 2 | 2 unit |
| NFR-002 | TC-U-102-01 through TC-U-102-15 | 15 | 15 unit |
| NFR-003 | TC-I-103-01, TC-I-103-02 | 2 | 2 integration |
| NFR-004 | TC-M-104-01 | 1 | 1 manual |

---

## Tests by Type

### Unit Tests (21 total)

| Test ID | One-Line Description | Requirement |
|---------|---------------------|-------------|
| TC-U-001-01 | GlobalParams has all 15 parameter fields | FR-001 |
| TC-U-001-02 | Default values match SPEC016 | FR-001 |
| TC-U-002-01 | RwLock read access succeeds | FR-002 |
| TC-U-002-02 | RwLock write access succeeds | FR-002 |
| TC-U-002-03 | Concurrent RwLock reads succeed | FR-002 |
| TC-U-003-01 | Grep finds zero hardcoded values | FR-003 |
| TC-U-005-01 | API compilation unchanged | FR-005 |
| TC-U-101-01 | Parameter read performance < 10ns | NFR-001 |
| TC-U-101-02 | No allocation on parameter read | NFR-001 |
| TC-U-102-01 | volume_level validation [0.0, 1.0] | NFR-002 |
| TC-U-102-02 | working_sample_rate validation [8000, 192000] | NFR-002 |
| TC-U-102-03 | output_ringbuffer_size validation | NFR-002 |
| TC-U-102-04 | output_refill_period validation | NFR-002 |
| TC-U-102-05 | maximum_decode_streams validation | NFR-002 |
| TC-U-102-06 | decode_work_period validation | NFR-002 |
| TC-U-102-07 | decode_chunk_size validation | NFR-002 |
| TC-U-102-08 | playout_ringbuffer_size validation | NFR-002 |
| TC-U-102-09 | playout_ringbuffer_headroom validation | NFR-002 |
| TC-U-102-10 | decoder_resume_hysteresis_samples validation | NFR-002 |
| TC-U-102-11 | mixer_min_start_level validation | NFR-002 |
| TC-U-102-12 | pause_decay_factor validation | NFR-002 |
| TC-U-102-13 | pause_decay_floor validation | NFR-002 |
| TC-U-102-14 | audio_buffer_size validation | NFR-002 |
| TC-U-102-15 | mixer_check_interval_ms validation | NFR-002 |

### Integration Tests (6 total)

| Test ID | One-Line Description | Requirement |
|---------|---------------------|-------------|
| TC-I-004-01 | Load all parameters from database | FR-004 |
| TC-I-004-02 | Handle missing database entries (use defaults) | FR-004 |
| TC-I-004-03 | Handle invalid type in database (use defaults) | FR-004 |
| TC-I-004-04 | Handle out-of-range values (use defaults) | FR-004 |
| TC-I-103-01 | Full test suite passes after each parameter | NFR-003 |
| TC-I-103-02 | Integration tests pass with GlobalParams | NFR-003 |

### Manual Tests (2 total)

| Test ID | One-Line Description | Requirement |
|---------|---------------------|-------------|
| TC-M-003-01 | Manual code review for hardcoded values | FR-003 |
| TC-M-104-01 | Documentation review (doc comments present) | NFR-004 |

---

## Tests by Parameter (Migration Verification)

Each parameter requires these tests during migration:

| Parameter | Migration Tests | Post-Migration Tests |
|-----------|----------------|---------------------|
| volume_level | TC-U-003-01, TC-I-103-01 | TC-U-102-01 |
| working_sample_rate | TC-U-003-01, TC-I-103-01, TC-M-003-01 | TC-U-102-02 |
| output_ringbuffer_size | TC-U-003-01, TC-I-103-01 | TC-U-102-03 |
| output_refill_period | TC-U-003-01, TC-I-103-01 | TC-U-102-04 |
| maximum_decode_streams | TC-U-003-01, TC-I-103-01 | TC-U-102-05 |
| decode_work_period | TC-U-003-01, TC-I-103-01 | TC-U-102-06 |
| decode_chunk_size | TC-U-003-01, TC-I-103-01 | TC-U-102-07 |
| playout_ringbuffer_size | TC-U-003-01, TC-I-103-01 | TC-U-102-08 |
| playout_ringbuffer_headroom | TC-U-003-01, TC-I-103-01 | TC-U-102-09 |
| decoder_resume_hysteresis_samples | TC-U-003-01, TC-I-103-01 | TC-U-102-10 |
| mixer_min_start_level | TC-U-003-01, TC-I-103-01 | TC-U-102-11 |
| pause_decay_factor | TC-U-003-01, TC-I-103-01 | TC-U-102-12 |
| pause_decay_floor | TC-U-003-01, TC-I-103-01 | TC-U-102-13 |
| audio_buffer_size | TC-U-003-01, TC-I-103-01 | TC-U-102-14 |
| mixer_check_interval_ms | TC-U-003-01, TC-I-103-01 | TC-U-102-15 |

---

## Critical Tests (High-Risk Parameters)

**Timing-Critical Parameters (Tier 3):**

| Parameter | Critical Tests | Acceptance Criteria |
|-----------|----------------|---------------------|
| working_sample_rate | TC-M-003-01, Manual stopwatch test | Position accurate within ±100ms over 60s playback |
| audio_buffer_size | TC-I-103-01, Manual audio test | No glitches, smooth playback |
| mixer_check_interval_ms | TC-I-103-01, CPU monitor | No underruns, CPU usage unchanged |

---

## Test Execution Order

**Phase 1: Pre-Migration Tests**
1. TC-U-001-01, TC-U-001-02 - Verify GlobalParams structure
2. TC-U-002-01, TC-U-002-02, TC-U-002-03 - Verify RwLock access
3. TC-I-004-01, TC-I-004-02, TC-I-004-03, TC-I-004-04 - Verify database init

**Phase 2: Per-Parameter Migration Tests**
For each of 15 parameters:
1. TC-U-003-01 - Verify no hardcoded values remain
2. TC-I-103-01 - Full test suite passes
3. TC-U-102-XX - Parameter-specific validation test

**Phase 3: Post-Migration Tests**
1. TC-U-005-01 - API compilation unchanged
2. TC-U-101-01, TC-U-101-02 - Performance verification
3. TC-M-003-01 - Manual code review
4. TC-M-104-01 - Documentation review

---

## Test File Locations

**Unit Tests:** `wkmp-common/src/params.rs` (inline #[cfg(test)] module)
**Integration Tests:** `wkmp-common/tests/params_integration.rs` (separate file)
**Manual Tests:** Procedures documented in TC-M-003-01.md and TC-M-104-01.md

---

## Coverage Summary

| Aspect | Coverage | Details |
|--------|----------|---------|
| Requirements | 100% (9/9) | All requirements have tests |
| Parameters | 100% (15/15) | All parameters have validation tests |
| Error Paths | 100% | Missing, invalid type, out-of-range |
| Performance | Yes | Microbenchmark + manual verification |
| Safety | Yes | Validation + test suite |
| Documentation | Yes | Manual review |

---

## Document Status

**Phase 3 Status:** Test Index Complete
**Next Steps:** Generate detailed test specifications (individual TC-*.md files)
**Traceability Matrix:** See traceability_matrix.md

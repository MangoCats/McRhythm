# Test Index: PLAN022 Queue Handling Resilience

**Plan:** PLAN022 Queue Handling Resilience Improvements
**Total Tests:** 21 (9 unit, 3 integration, 2 system, 7 inline unit tests)
**Coverage Target:** 100% of new code paths

---

## Quick Reference Table

| Test ID | Type | Requirement | Brief Description | File | Status |
|---------|------|-------------|-------------------|------|--------|
| TC-U-IDEMP-001 | Unit | REQ-IDEMP-010 | First removal succeeds | tc_u_idemp_001.md | Pending |
| TC-U-IDEMP-002 | Unit | REQ-IDEMP-010 | Second removal idempotent | tc_u_idemp_002.md | Pending |
| TC-U-IDEMP-003 | Unit | REQ-IDEMP-010 | Remove non-existent entry | tc_u_idemp_003.md | Pending |
| TC-U-DEDUP-001 | Unit | REQ-DEDUP-010 | First event processed | tc_u_dedup_001.md | Pending |
| TC-U-DEDUP-002 | Unit | REQ-DEDUP-010 | Duplicate event ignored | tc_u_dedup_002.md | Pending |
| TC-U-DEDUP-003 | Unit | REQ-DEDUP-010 | Multiple distinct events | tc_u_dedup_003.md | Pending |
| TC-U-DEDUP-004 | Unit | REQ-DEDUP-020 | Stale entry cleanup | tc_u_dedup_004.md | Pending |
| TC-U-DRY-001 | Unit | REQ-DRY-020 | Cleanup helper order | tc_u_dry_001.md | Pending |
| TC-U-DRY-002 | Unit | REQ-DRY-010 | Cleanup helper idempotent | tc_u_dry_002.md | Pending |
| TC-U-DRY-003 | Unit | REQ-DRY-010 | Skip uses cleanup helper | tc_u_dry_003.md | Pending |
| TC-I-REMOVAL-001 | Integration | REQ-DRY-010, REQ-IDEMP-010 | Normal passage completion | tc_i_removal_001.md | Pending |
| TC-I-REMOVAL-002 | Integration | REQ-DEDUP-010 | Duplicate event during completion | tc_i_removal_002.md | Pending |
| TC-I-ADV-001 | Integration | REQ-DRY-010 | Promotion triggers decode | tc_i_adv_001.md | Pending |
| TC-S-RESIL-001 | System | ALL | Rapid skip operations | tc_s_resil_001.md | Pending |
| TC-S-RESIL-002 | System | REQ-DEDUP-010 | EOF before crossfade | tc_s_resil_002.md | Pending |

---

## Test Organization by Requirement

### REQ-QUEUE-IDEMP-010: Idempotent Queue Removal

**Unit Tests:**
- TC-U-IDEMP-001: First removal succeeds (Ok(true))
- TC-U-IDEMP-002: Second removal returns Ok(false)
- TC-U-IDEMP-003: Remove non-existent entry returns Ok(false)

**Coverage:** All idempotency cases (first, second, never-existed)

---

### REQ-QUEUE-IDEMP-020: Return Value Semantics

**Unit Tests:**
- TC-U-IDEMP-002: Verifies Ok(false) semantics
- TC-U-DRY-002: Verifies caller handling

**Coverage:** Return value interpretation by callers

---

### REQ-QUEUE-DEDUP-010: PassageComplete Deduplication

**Unit Tests:**
- TC-U-DEDUP-001: First event processes normally
- TC-U-DEDUP-002: Duplicate event ignored (within 5s)
- TC-U-DEDUP-003: Multiple distinct events process independently

**Integration Tests:**
- TC-I-REMOVAL-002: Duplicate event during real passage completion

**System Tests:**
- TC-S-RESIL-002: EOF before crossfade (multiple event sources)

**Coverage:** All deduplication scenarios

---

### REQ-QUEUE-DEDUP-020: Deduplication Scope

**Unit Tests:**
- TC-U-DEDUP-004: Verifies 5-second window + cleanup

**Coverage:** Time window and cleanup behavior

---

### REQ-QUEUE-DEDUP-030: Thread Safety

**Unit Tests:**
- TC-U-DEDUP-003: Concurrent distinct events (stress test component)

**System Tests:**
- TC-S-RESIL-001: Rapid skip operations (concurrency test)

**Coverage:** Concurrent access scenarios

---

### REQ-QUEUE-DRY-010: Single Cleanup Implementation

**Unit Tests:**
- TC-U-DRY-002: Helper is idempotent
- TC-U-DRY-003: Skip uses helper

**Integration Tests:**
- TC-I-REMOVAL-001: Normal completion uses helper
- TC-I-ADV-001: Removal + promotion sequence

**Coverage:** All cleanup paths use helper

---

### REQ-QUEUE-DRY-020: Cleanup Operation Ordering

**Unit Tests:**
- TC-U-DRY-001: Mock components verify 1-6 sequence

**Integration Tests:**
- TC-I-REMOVAL-001: Real cleanup sequence correct

**Coverage:** Cleanup step ordering verified

---

## Test Organization by Type

### Unit Tests (9 tests, 7 inline)

**File: wkmp-ap/src/db/queue.rs (inline tests)**
- TC-U-IDEMP-001: test_remove_queue_entry_first_call_succeeds
- TC-U-IDEMP-002: test_remove_queue_entry_second_call_idempotent
- TC-U-IDEMP-003: test_remove_queue_entry_never_existed

**File: wkmp-ap/tests/queue_deduplication_tests.rs**
- TC-U-DEDUP-001: test_passage_complete_first_event_processed
- TC-U-DEDUP-002: test_passage_complete_duplicate_event_ignored
- TC-U-DEDUP-003: test_passage_complete_multiple_distinct_events
- TC-U-DEDUP-004: test_deduplication_entry_cleanup_after_5_seconds

**File: wkmp-ap/tests/cleanup_helper_tests.rs**
- TC-U-DRY-001: test_cleanup_helper_step_order
- TC-U-DRY-002: test_cleanup_helper_idempotent
- TC-U-DRY-003: test_skip_next_uses_cleanup_helper

---

### Integration Tests (3 tests)

**File: wkmp-ap/tests/queue_removal_integration_tests.rs**
- TC-I-REMOVAL-001: test_passage_completion_cleanup_chain
- TC-I-REMOVAL-002: test_passage_completion_with_duplicate_event
- TC-I-ADV-001: test_removal_triggers_decode_for_promoted_passages

---

### System Tests (2 tests)

**File: wkmp-ap/tests/system_queue_resilience_tests.rs**
- TC-S-RESIL-001: test_rapid_skip_operations_no_errors
- TC-S-RESIL-002: test_eof_before_crossfade_no_duplicate_errors

---

## Test Execution Order

**Recommended order (fastest to slowest):**

1. **Unit Tests** (run first, ~1-5ms each)
   - Idempotency tests (inline in db/queue.rs)
   - Deduplication tests
   - DRY helper tests

2. **Integration Tests** (run second, ~100-500ms each)
   - Removal flow tests
   - Promotion tests

3. **System Tests** (run last, ~2-5 seconds each)
   - Rapid skip test
   - EOF handling test

**Total Execution Time Estimate:** ~10-15 seconds for full test suite

---

## Test Coverage Matrix

| Code Component | Unit Tests | Integration Tests | System Tests | Total Coverage |
|----------------|------------|-------------------|--------------|----------------|
| db/queue.rs::remove_from_queue() | 3 | 2 | 1 | 100% |
| engine/core.rs::completed_passages | 4 | 2 | 2 | 100% |
| engine/diagnostics.rs::dedup logic | 4 | 2 | 2 | 100% |
| engine/queue.rs::cleanup_queue_entry() | 3 | 3 | 2 | 100% |
| engine/queue.rs::skip_next() (refactored) | 1 | 1 | 1 | 100% |
| engine/queue.rs::remove_queue_entry() (refactored) | 1 | 1 | 0 | 100% |

**Overall Coverage:** 100% of new/modified code paths

---

## Test Data Requirements

### Audio Files Needed

| File Type | Purpose | Generation Method | Est. Size |
|-----------|---------|-------------------|-----------|
| Short audio (1s) | System tests | audio_generator.rs | ~50KB |
| Truncated audio | EOF test | audio_generator.rs with early EOF | ~30KB |

**Storage:** Generated in-memory per test (no persistent files)

### Database State

| State | Purpose | Setup Method |
|-------|---------|--------------|
| Empty queue | Idempotency tests | In-memory SQLite |
| Queue with 1 entry | Basic removal tests | Enqueue test passage |
| Queue with 3 entries | Promotion tests | Enqueue 3 passages |
| Queue with 10 entries | Rapid skip test | Enqueue loop |

**Database:** In-memory SQLite created per test (no shared state)

---

## Test Infrastructure Needs

### Existing (Reuse)

- ✅ TestEngine (test_engine.rs)
- ✅ create_test_audio_file() (helpers/audio_generator.rs)
- ✅ DecoderWorkerSpy (event_driven_playback_tests.rs)

### New (To Create)

- ❌ assert_no_error_logs() utility
- ❌ CleanupSpy (track cleanup step ordering)
- ❌ create_audio_file_with_truncated_end() (EOF test)

**Estimated Effort:** ~1 hour to create new test utilities

---

## Traceability to Requirements

See [traceability_matrix.md](traceability_matrix.md) for complete requirements ↔ tests mapping.

**Quick Stats:**
- Requirements with tests: 7 of 7 (100%)
- Tests covering multiple requirements: 5 tests
- Average tests per requirement: 3.0

---

## Test Execution Instructions

### Run All Tests

```bash
# Full test suite (including new tests)
cargo test --package wkmp-ap

# Just new tests (by file pattern)
cargo test --package wkmp-ap queue_deduplication_tests
cargo test --package wkmp-ap cleanup_helper_tests
cargo test --package wkmp-ap queue_removal_integration_tests
cargo test --package wkmp-ap system_queue_resilience_tests
```

### Run Specific Test

```bash
# By test name
cargo test --package wkmp-ap test_remove_queue_entry_first_call_succeeds

# With output (for debugging)
cargo test --package wkmp-ap test_passage_complete_duplicate_event_ignored -- --nocapture
```

### Coverage Report

```bash
# Generate coverage report (requires cargo-tarpaulin)
cargo tarpaulin --package wkmp-ap --out Html --output-dir coverage/
```

---

## Test Status Tracking

**As of 2025-11-06:**
- Pending: 21 tests (all)
- In Progress: 0 tests
- Complete: 0 tests
- Passing: 0 tests (not yet implemented)

**Update this section during implementation.**

---

## Sign-Off

**Test Index Created:** 2025-11-06
**Created By:** Claude Code (Plan Workflow Phase 3)
**Status:** Ready for test specification details
**Next:** Individual test specification files (tc_*.md)

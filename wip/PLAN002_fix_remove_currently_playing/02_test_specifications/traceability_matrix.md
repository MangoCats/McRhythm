# Traceability Matrix - Fix Remove Currently Playing Passage Bug

**Plan ID:** PLAN002
**Date:** 2025-10-27
**Coverage Status:** 100% (all requirements have tests)

---

## Complete Traceability Table

| Requirement | Unit Tests | Integration Tests | System Tests | Implementation File(s) | Status | Coverage |
|-------------|------------|-------------------|--------------|------------------------|--------|----------|
| REQ-FIX-010 (Stop immediately) | - | TC-I-002 | TC-S-001, TC-S-002 | engine.rs, mixer.rs | Pending | Complete |
| REQ-FIX-020 (Release decoder) | - | TC-I-001 | TC-S-001, TC-S-002 | engine.rs, decoder_worker.rs | Pending | Complete |
| REQ-FIX-030 (Clear mixer) | - | TC-I-002 | TC-S-001, TC-S-002 | engine.rs, mixer.rs | Pending | Complete |
| REQ-FIX-040 (Update queue) | TC-U-001 | - | TC-S-001 | queue_manager.rs | Pending | Complete |
| REQ-FIX-050 (Start next) | TC-U-002 | - | TC-S-002 | engine.rs | Pending | Complete |
| REQ-FIX-060 (No disruption) | - | - | TC-S-004 | engine.rs, queue_manager.rs | Pending | Complete |
| REQ-FIX-070 (Prevent resume) | - | - | TC-S-003 | engine.rs, decoder_worker.rs | Pending | Complete |
| REQ-FIX-080 (New starts) | - | - | TC-S-003 | engine.rs | Pending | Complete |

---

## Requirements → Tests (Forward Traceability)

### REQ-FIX-010: Stop playback immediately
- **Tests:** TC-I-002, TC-S-001, TC-S-002
- **Test Types:** Integration (mechanism), System (end-to-end)
- **Coverage:** Complete (immediate stop verified in multiple scenarios)

### REQ-FIX-020: Release decoder chain resources
- **Tests:** TC-I-001, TC-S-001, TC-S-002
- **Test Types:** Integration (mechanism), System (end-to-end)
- **Coverage:** Complete (resource cleanup verified)

### REQ-FIX-030: Clear mixer state
- **Tests:** TC-I-002, TC-S-001, TC-S-002
- **Test Types:** Integration (mechanism), System (end-to-end)
- **Coverage:** Complete (mixer clearing verified)

### REQ-FIX-040: Update queue structure
- **Tests:** TC-U-001, TC-S-001
- **Test Types:** Unit (mechanism), System (integration)
- **Coverage:** Complete (queue state verified)

### REQ-FIX-050: Start next passage if queue non-empty
- **Tests:** TC-U-002, TC-S-002
- **Test Types:** Unit (mechanism), System (end-to-end)
- **Coverage:** Complete (automatic start verified)

### REQ-FIX-060: No disruption when removing non-current
- **Tests:** TC-S-004
- **Test Types:** System (end-to-end)
- **Coverage:** Complete (continued playback verified)

### REQ-FIX-070: Prevent removed passage resume
- **Tests:** TC-S-003
- **Test Types:** System (bug scenario)
- **Coverage:** Complete (bug directly tested)

### REQ-FIX-080: New passage starts after removal
- **Tests:** TC-S-003
- **Test Types:** System (bug scenario)
- **Coverage:** Complete (new passage verified)

---

## Tests → Requirements (Backward Traceability)

### System Tests
- **TC-S-001:** REQ-FIX-010, REQ-FIX-020, REQ-FIX-030, REQ-FIX-040
- **TC-S-002:** REQ-FIX-010, REQ-FIX-020, REQ-FIX-030, REQ-FIX-050
- **TC-S-003:** REQ-FIX-070, REQ-FIX-080
- **TC-S-004:** REQ-FIX-060

### Integration Tests
- **TC-I-001:** REQ-FIX-020
- **TC-I-002:** REQ-FIX-010, REQ-FIX-030

### Unit Tests
- **TC-U-001:** REQ-FIX-040
- **TC-U-002:** REQ-FIX-050

---

## Implementation Files → Requirements

### engine.rs (Primary implementation)
- REQ-FIX-010 (coordinate stop)
- REQ-FIX-020 (clear decoder chain)
- REQ-FIX-030 (clear mixer)
- REQ-FIX-050 (start next)
- REQ-FIX-060 (handle non-current)
- REQ-FIX-070 (prevent resume)
- REQ-FIX-080 (enable new start)

### queue_manager.rs
- REQ-FIX-040 (update queue structure)
- REQ-FIX-060 (identify current vs. non-current)

### decoder_worker.rs
- REQ-FIX-020 (release decoder resources)
- REQ-FIX-070 (discard stale buffer)

### mixer.rs
- REQ-FIX-010 (stop audio output)
- REQ-FIX-030 (clear playback state)

---

## Coverage Analysis

### By Test Type
- **Unit Tests:** 2 requirements (25%)
- **Integration Tests:** 2 requirements (25%)
- **System Tests:** 8 requirements (100%)

**Rationale:** Most requirements are end-to-end behavior, tested at system level.

### By Priority
- **Critical Requirements (6):** All have multiple tests
- **High Requirements (2):** All have tests

**Coverage:** 100% of requirements have tests

### Gaps Analysis
**No gaps identified** - All requirements traced to tests

---

## Test Execution Verification

### During Implementation
For each requirement, implementer must:
1. **Before coding:** Read requirement + associated tests
2. **During coding:** Implement to satisfy test pass criteria
3. **After coding:** Run tests, verify pass
4. **Update matrix:** Mark implementation file(s), update status

### Before Release
Reviewer must verify:
1. **All requirements → tests:** No requirement without test
2. **All tests → requirements:** No orphaned tests
3. **All requirements → code:** Implementation files filled in
4. **All tests pass:** No failures
5. **Coverage complete:** 100% traceability

---

## Matrix Maintenance

### When to Update

**During Implementation:**
- Update "Implementation File(s)" column as files modified
- Update "Status" column (Pending → In Progress → Complete)

**After Testing:**
- Update "Status" to "Verified" when tests pass
- Add notes if partial implementation or known issues

**On Requirements Change:**
- Add new rows for new requirements
- Update test coverage
- Ensure no gaps

### Current Status
- **Requirements:** 8 (complete set)
- **Tests:** 8 (one-to-many mapping)
- **Implementation:** Pending (files identified)
- **Verification:** Not started

---

## Notes

- Matrix demonstrates 100% test coverage of requirements
- Multiple tests per critical requirement (redundancy)
- Bug scenario (TC-S-003) directly tests REQ-FIX-070 and REQ-FIX-080
- All subsystems (engine, queue, decoder, mixer) covered

---

## Next Steps

1. ✓ Matrix created with 100% coverage
2. Proceed to implementation using test-driven approach
3. Update matrix as implementation progresses
4. Verify all tests pass before considering bug fixed
5. Archive plan after successful implementation

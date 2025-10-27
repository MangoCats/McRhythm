# Test Index - Fix Remove Currently Playing Passage Bug

**Plan ID:** PLAN002
**Date:** 2025-10-27
**Total Tests:** 8 tests (4 system, 2 integration, 2 unit)

---

## Quick Reference Table

| Test ID | Type | Requirement(s) | One-Line Description | Priority |
|---------|------|----------------|----------------------|----------|
| TC-S-001 | System | REQ-FIX-010, 020, 030, 040 | Remove currently playing passage (empty queue after) | Critical |
| TC-S-002 | System | REQ-FIX-010, 020, 030, 050 | Remove currently playing passage (queue non-empty after) | Critical |
| TC-S-003 | System | REQ-FIX-070, 080 | Remove playing, enqueue new (bug scenario) | Critical |
| TC-S-004 | System | REQ-FIX-060 | Remove non-current passage (no disruption) | High |
| TC-I-001 | Integration | REQ-FIX-020 | Decoder chain resources released on removal | Critical |
| TC-I-002 | Integration | REQ-FIX-030 | Mixer state cleared on removal | Critical |
| TC-U-001 | Unit | REQ-FIX-040 | QueueManager::remove() detects current passage | High |
| TC-U-002 | Unit | REQ-FIX-050 | Next passage determination after current removed | Critical |

---

## Tests by Type

### System Tests (4 tests)
End-to-end scenarios via HTTP API:
- TC-S-001: Remove currently playing → empty queue
- TC-S-002: Remove currently playing → next starts
- TC-S-003: Remove + enqueue (the reported bug)
- TC-S-004: Remove non-current → no disruption

### Integration Tests (2 tests)
Component interaction verification:
- TC-I-001: Decoder worker resource cleanup
- TC-I-002: Mixer state management

### Unit Tests (2 tests)
Individual component behavior:
- TC-U-001: Queue structure detection of current
- TC-U-002: Next passage selection logic

---

## Tests by Requirement

| Requirement | Tests | Coverage |
|-------------|-------|----------|
| REQ-FIX-010 (Stop immediately) | TC-S-001, TC-S-002, TC-I-002 | Complete |
| REQ-FIX-020 (Release decoder) | TC-S-001, TC-S-002, TC-I-001 | Complete |
| REQ-FIX-030 (Clear mixer) | TC-S-001, TC-S-002, TC-I-002 | Complete |
| REQ-FIX-040 (Update queue) | TC-S-001, TC-U-001 | Complete |
| REQ-FIX-050 (Start next) | TC-S-002, TC-U-002 | Complete |
| REQ-FIX-060 (No disruption) | TC-S-004 | Complete |
| REQ-FIX-070 (Prevent resume) | TC-S-003 | Complete |
| REQ-FIX-080 (New starts) | TC-S-003 | Complete |

**Coverage:** 100% (all requirements have tests)

---

## Test Execution Order

**Recommended sequence:**
1. **Unit tests first** (fast, focused)
   - TC-U-001, TC-U-002
2. **Integration tests** (verify mechanisms)
   - TC-I-001, TC-I-002
3. **System tests** (end-to-end verification)
   - TC-S-001, TC-S-002, TC-S-003, TC-S-004

**Total estimated execution time:** 5-10 minutes (depends on audio file length)

---

## Test Data Requirements

### Audio Files Needed
- **test_passage_a.mp3** - Short audio file (2-5 seconds)
- **test_passage_b.mp3** - Different audio file (2-5 seconds)
- **test_passage_c.mp3** - Third audio file for multi-entry tests

### Test Database
- Clean slate database before each test
- Or use transaction rollback for isolation

---

## Success Criteria

**All tests must pass before considering fix complete:**
- ✓ All 2 unit tests pass
- ✓ All 2 integration tests pass
- ✓ All 4 system tests pass
- ✓ No regressions in existing playback scenarios

---

## Files in This Folder

- `test_index.md` - This file (quick reference)
- `tc_s_001.md` - System test: Remove current (empty after)
- `tc_s_002.md` - System test: Remove current (non-empty after)
- `tc_s_003.md` - System test: Bug scenario (remove + enqueue)
- `tc_s_004.md` - System test: Remove non-current
- `tc_i_001.md` - Integration test: Decoder cleanup
- `tc_i_002.md` - Integration test: Mixer clearing
- `tc_u_001.md` - Unit test: Current detection
- `tc_u_002.md` - Unit test: Next passage selection
- `traceability_matrix.md` - Requirements ↔ Tests ↔ Implementation mapping

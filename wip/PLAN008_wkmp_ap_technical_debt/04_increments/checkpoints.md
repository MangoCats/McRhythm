# Implementation Increments & Checkpoints

**Plan:** PLAN008
**Phase:** 5 - Implementation Breakdown
**Total Increments:** 22
**Total Estimated Effort:** 62 hours (3 weeks)

---

## Increment Overview

### Sprint 1: Security & Critical (Increments 1-7)

| Increment | Task | Requirements | Estimated Hours | Cumulative |
|-----------|------|--------------|-----------------|------------|
| 01 | Implement POST authentication | REQ-DEBT-SEC-001-* | 3h | 3h |
| 02 | Implement PUT authentication | REQ-DEBT-SEC-001-* | 2h | 5h |
| 03 | Add authentication edge case tests | REQ-DEBT-SEC-001-* | 2h | 7h |
| 04 | Add file_path field to ChunkedDecoder | REQ-DEBT-FUNC-001-* | 2h | 9h |
| 05 | Update decoder error sites with file_path | REQ-DEBT-FUNC-001-* | 1h | 10h |
| 06 | Implement BufferManager database config | REQ-DEBT-FUNC-002-* | 3h | 13h |
| 07 | Add buffer config validation & tests | REQ-DEBT-FUNC-002-* | 2h | 15h |

**Checkpoint 1 (End of Sprint 1):**
- All POST/PUT endpoints authenticated
- Security tests passing (6 tests)
- Decoder errors include file paths (3 tests)
- Buffer config from database (4 tests)
- **Total Tests Passing:** 13

---

### Sprint 2: Functionality & Diagnostics (Increments 8-15)

| Increment | Task | Requirements | Estimated Hours | Cumulative |
|-----------|------|--------------|-----------------|------------|
| 08 | Create DecoderTelemetry infrastructure | REQ-DEBT-FUNC-003-* | 3h | 18h |
| 09 | Populate BufferChainInfo with telemetry | REQ-DEBT-FUNC-003-* | 2h | 20h |
| 10 | Create get_passage_album_uuids() function | REQ-DEBT-FUNC-004-030 | 2h | 22h |
| 11 | Populate PassageStarted with albums | REQ-DEBT-FUNC-004-010 | 1h | 23h |
| 12 | Populate PassageComplete with albums | REQ-DEBT-FUNC-004-020 | 1h | 24h |
| 13 | Add passage start time tracking to mixer | REQ-DEBT-FUNC-005-* | 2h | 26h |
| 14 | Calculate duration_played on completion | REQ-DEBT-FUNC-005-* | 2h | 28h |
| 15 | Cleanup: Config deduplication + backups | REQ-DEBT-QUALITY-004/005-* | 1h | 29h |

**Checkpoint 2 (End of Sprint 2):**
- Developer UI telemetry complete (4 tests)
- Passage events include albums (3 tests)
- Duration played accurate (3 tests)
- Config cleanup done
- **Total Tests Passing:** 23 (+10 from Sprint 1)

---

### Sprint 3: Code Health (Increments 16-22)

| Increment | Task | Requirements | Estimated Hours | Cumulative |
|-----------|------|--------------|-----------------|------------|
| 16 | Replace .unwrap() in audio/buffer.rs | REQ-DEBT-QUALITY-001-* | 3h | 32h |
| 17 | Replace .unwrap() in events.rs | REQ-DEBT-QUALITY-001-* | 2h | 34h |
| 18 | Extract engine/diagnostics.rs module | REQ-DEBT-QUALITY-002-* | 3h | 37h |
| 19 | Extract engine/queue.rs module | REQ-DEBT-QUALITY-002-* | 4h | 41h |
| 20 | Extract engine/core.rs module | REQ-DEBT-QUALITY-002-* | 4h | 45h |
| 21 | Fix compiler warnings (cargo fix + manual) | REQ-DEBT-QUALITY-003-* | 3h | 48h |
| 22 | Add clipping warning log + verification | REQ-DEBT-FUTURE-003-010 | 2h | 50h |

**Checkpoint 3 (End of Sprint 3):**
- Zero compiler warnings
- No .unwrap() in audio hot paths (2 tests)
- engine.rs refactored (1 test)
- All code quality tests passing (5 tests)
- **Total Tests Passing:** 28 (all tests)

---

## Checkpoint Criteria

### Checkpoint 1 (Sprint 1 Complete)

**Entry Criteria:**
- Sprint 1 increments 1-7 complete
- All code committed to feature branch

**Verification:**
```bash
# Run Sprint 1 tests
cargo test --lib -p wkmp-ap -- tc_sec tc_func_001 tc_func_002

# Expected: 13 tests pass
```

**Exit Criteria:**
- [ ] All 13 Sprint 1 tests passing
- [ ] No compiler errors
- [ ] Security tests verified: POST/PUT require auth
- [ ] Decoder tests verified: Errors include file paths
- [ ] Buffer tests verified: Settings read from database
- [ ] Code review approved for Sprint 1

**Proceed to Sprint 2:** Yes/No

---

### Checkpoint 2 (Sprint 2 Complete)

**Entry Criteria:**
- Checkpoint 1 passed
- Sprint 2 increments 8-15 complete
- All code committed to feature branch

**Verification:**
```bash
# Run all tests through Sprint 2
cargo test --lib -p wkmp-ap -- tc_sec tc_func

# Expected: 23 tests pass
```

**Exit Criteria:**
- [ ] All 23 Sprint 1-2 tests passing
- [ ] No compiler errors
- [ ] Telemetry tests verified: Developer UI shows complete data
- [ ] Album tests verified: Events include UUID lists
- [ ] Duration tests verified: Accurate to ±100ms
- [ ] Config cleanup verified: Single config file, no backups
- [ ] Code review approved for Sprint 2

**Proceed to Sprint 3:** Yes/No

---

### Checkpoint 3 (Sprint 3 Complete - Final)

**Entry Criteria:**
- Checkpoint 2 passed
- Sprint 3 increments 16-22 complete
- All code committed to feature branch

**Verification:**
```bash
# Run ALL tests
cargo test --lib -p wkmp-ap

# Expected: All wkmp-ap tests pass (including new 28)

# Verify zero warnings
cargo build -p wkmp-ap 2>&1 | grep "warning:" | wc -l
# Expected: 0

# Verify refactoring
ls wkmp-ap/src/playback/engine/*.rs | wc -l
# Expected: 4 (mod.rs + core.rs + queue.rs + diagnostics.rs)
```

**Exit Criteria:**
- [ ] All 28 new tests passing
- [ ] All existing wkmp-ap tests still passing (regression check)
- [ ] Zero compiler warnings
- [ ] engine.rs refactored: 4 files, each <1500 lines
- [ ] No .unwrap() in audio/buffer.rs or events.rs
- [ ] Performance verified: <1% overhead
- [ ] Final code review approved
- [ ] Integration tests pass
- [ ] Ready for merge to main

**Technical Debt Remediation Complete:** Yes/No

---

## Increment Dependencies

**No cross-sprint dependencies:**
- Sprints can proceed linearly
- Within sprint, some increments depend on previous ones

**Sprint 1 Dependencies:**
- Inc 03 depends on Inc 01-02 (tests need auth implementation)
- Inc 05 depends on Inc 04 (need file_path field before using it)
- Inc 07 depends on Inc 06 (tests need config implementation)

**Sprint 2 Dependencies:**
- Inc 09 depends on Inc 08 (need telemetry infrastructure)
- Inc 11-12 depend on Inc 10 (need DB function before using it)
- Inc 14 depends on Inc 13 (need tracking before calculating)

**Sprint 3 Dependencies:**
- Inc 19-20 depend on Inc 18 (extract diagnostics first, then queue, then core)

---

## Implementation Guidance

**For Each Increment:**
1. Read increment file (`increment_XX.md`)
2. Review requirements and tests referenced
3. Implement to pass tests
4. Run tests: `cargo test --lib -p wkmp-ap -- <test_pattern>`
5. Commit with message: `[PLAN008-XX] <brief description> - Refs REQ-DEBT-XXX-YYY`
6. Update traceability matrix (mark "Complete")
7. Proceed to next increment

**At Each Checkpoint:**
1. Run all tests through current sprint
2. Verify all exit criteria met
3. Request code review
4. Obtain approval before proceeding

**If Increment Blocked:**
1. Document blocker in traceability matrix
2. Skip to next independent increment if available
3. Resolve blocker before checkpoint

---

## Effort Distribution

**By Sprint:**
- Sprint 1: 15 hours (7 increments, avg 2.1h each)
- Sprint 2: 14 hours (8 increments, avg 1.8h each)
- Sprint 3: 21 hours (7 increments, avg 3.0h each)

**Total:** 50 hours base + 12 hours contingency = 62 hours (3 weeks)

**By Task Type:**
- Implementation: 35 hours (70%)
- Testing: 10 hours (20%)
- Refactoring: 11 hours (22%)
- Cleanup: 1 hour (2%)
- Documentation: 5 hours (10%)

---

## Risk Mitigation in Increments

**High-Risk Increments (extra care):**
- Inc 01-02: Authentication (security-critical)
- Inc 18-20: Refactoring (regression risk)

**Mitigation:**
- Extra test coverage for Inc 01-02
- Run full test suite after Inc 18-20
- Code review required for all high-risk increments

---

## Phase 5 Status

**Deliverables:**
- ✅ 22 increments defined (2-4 hours each)
- ✅ 3 checkpoints with clear criteria
- ✅ Dependency analysis complete
- ✅ Effort distribution analyzed
- ✅ Risk-based sequencing applied

**Next Phase:** Phase 6 - Effort & Schedule Estimation

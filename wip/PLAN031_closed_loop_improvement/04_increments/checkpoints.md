# PLAN031 Implementation Checkpoints

## Checkpoint 1: Foundation Complete (After I3)

**Timing:** After Increments 1-3 (~7-10 hours)

**Verify:**
- [ ] Testing module compiles and exports types
- [ ] Ground truth table created via migration
- [ ] Can load ground truth from JSON file
- [ ] EvaluationEngine classifies TP/FP/TN/FN correctly
- [ ] Basic metrics calculation works

**Tests Passing:**
- TC-U-GT-001, TC-U-GT-002, TC-U-GT-003, TC-U-GT-004
- TC-U-EV-001, TC-U-EV-002, TC-U-EV-003, TC-U-EV-004
- TC-U-EV-005, TC-U-EV-006

**Decision Gate:**
- GREEN: All tests pass → Proceed to I4-I7
- YELLOW: Minor issues → Fix and retest
- RED: Major issues → Stop, reassess approach

## Checkpoint 2: Core System Complete (After I7)

**Timing:** After Increments 4-7 (~12-17 hours total)

**Verify:**
- [ ] Batch orchestrator runs batches against files
- [ ] Pipeline hooks record results (test in isolation)
- [ ] Failure analyzer categorizes failures
- [ ] Database reset modes work correctly

**Tests Passing:**
- TC-U-BM-001, TC-U-BM-002, TC-U-BM-003
- TC-U-FA-001, TC-U-FA-002, TC-U-FA-003
- TC-U-DB-001, TC-U-DB-002

**Decision Gate:**
- GREEN: All tests pass → Proceed to I8-I9
- YELLOW: Integration issues → Debug and fix
- RED: Architecture problems → May need to refactor

## Checkpoint 3: System Ready (After I8)

**Timing:** After Increment 8 (~18-24 hours total)

**Verify:**
- [ ] CLI commands work (`test-batch`, `show-report`)
- [ ] Reports display correctly in console
- [ ] Phase progression logic works

**Tests Passing:**
- TC-U-RP-001, TC-U-RP-002
- TC-S-003 (CLI produces readable report)

**Decision Gate:**
- GREEN: CLI works → Proceed to integration tests
- YELLOW: Output formatting issues → Polish and fix
- RED: Major CLI issues → Fix before integration

## Checkpoint 4: Integration Verified (After I9)

**Timing:** After Increment 9 (~22-30 hours total)

**Verify:**
- [ ] End-to-end test with real files passes
- [ ] Pipeline integration doesn't break normal imports
- [ ] All integration tests pass

**Tests Passing:**
- TC-I-BM-001, TC-I-BM-002
- TC-I-API-001, TC-I-API-002
- TC-I-FA-001
- TC-S-001, TC-S-002

**Decision Gate:**
- GREEN: All integration tests pass → PLAN031 Complete
- YELLOW: Flaky tests → Investigate and stabilize
- RED: Integration failures → Debug pipeline hooks

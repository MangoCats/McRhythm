# PLAN022: Queue Handling Resilience - PLAN SUMMARY

**Status:** Ready for Implementation
**Created:** 2025-11-06
**Specification Source:** wip/SPEC029-queue_handling_resilience.md
**Plan Location:** `wip/PLAN022_queue_handling_resilience/`

---

## READ THIS FIRST

This plan addresses reliability gaps in wkmp-ap queue handling that cause ERROR logs from duplicate PassageComplete events. Implementation uses test-driven development (TDD) with three independent increments.

**For Implementation:**
- Read this summary (~450 lines)
- Review requirements: `requirements_index.md` (7 requirements)
- Review test specifications: `02_test_specifications/test_index.md` (21 tests)
- Follow traceability matrix: `02_test_specifications/traceability_matrix.md`
- Implement using test-first approach (write tests before code)

**Context Window Budget:**
- This summary: ~450 lines
- Requirements index: ~300 lines
- Test index: ~250 lines
- **Total for planning:** ~1000 lines
- **Per-increment implementation:** ~600-800 lines (summary + tests + spec section)

---

## Executive Summary

### Problem Being Solved

**Current State:** wkmp-ap queue handling has three reliability gaps:

1. **Non-Idempotent Database Removal:** Second removal attempt fails with ERROR log instead of graceful no-op
2. **Multiple PassageComplete Event Sources:** Three event sources (PassageComplete marker, EOF, EOF before leadout) trigger duplicate removal attempts
3. **Code Duplication:** 5-7 cleanup steps repeated verbatim across skip_next(), remove_queue_entry(), and clear_queue()

**Observed Symptom** (from logs):
```
ERROR Failed to remove entry from database: Queue error: Queue entry not found: 66364e56...
WARN Failed to remove queue entry from memory: 66364e56...
[7ms later - duplicate event]
ERROR Failed to remove entry from database: Queue error: Queue entry not found: 66364e56...
```

**Impact:**
- Log noise obscures real errors
- Telemetry pollution (false positives)
- Maintenance burden (duplicate logic can diverge)
- Cosmetic (system recovers, no functional impact)

### Solution Approach

**Three-Pronged Approach** (lowest risk per CLAUDE.md Risk-First Framework):

**1. Idempotent Database Operations** (REQ-QUEUE-IDEMP-010, REQ-QUEUE-IDEMP-020)
- Change `remove_from_queue()` to return `Result<bool>` instead of `Result<()>`
- `Ok(true)` = removed, `Ok(false)` = already removed (not error)
- Eliminate ERROR logs for duplicate removal

**2. PassageComplete Event Deduplication** (REQ-QUEUE-DEDUP-010, 020, 030)
- Track processed `queue_entry_id`s for 5 seconds
- First event processes, duplicates ignored with DEBUG log
- Thread-safe via Arc<RwLock<HashMap>>

**3. DRY Cleanup Refactoring** (REQ-QUEUE-DRY-010, 020)
- Single `cleanup_queue_entry()` helper method
- All cleanup callers use helper (40-60% line reduction)
- Correct ordering: release chain → stop mixer → remove queue → release buffer → emit events → assign chains

**Risk Assessment:**
- Failure Risk: Low (well-understood patterns, comprehensive testing)
- Residual Risk: Low (after mitigations)
- Quality: High (maintainable, testable, architecturally aligned)

### Implementation Status

**Phases 1-3 Complete:**
- ✅ Phase 1: Scope Definition - 7 requirements extracted
- ✅ Phase 2: Specification Verification - 0 Critical issues, 2 Medium (documentation clarifications), 1 Low
- ✅ Phase 3: Test Definition - 21 tests defined, 100% coverage

**Phases 4-8 Status:** Not Applicable (Week 1 implementation - Phases 1-3 only)

**Ready to Begin:** ✅ YES - All requirements clear, tests defined, specification verified

---

## Requirements Summary

**Total Requirements:** 7 (all P0 - High Priority)

| Req ID | Brief Description | Tests |
|--------|-------------------|-------|
| REQ-QUEUE-IDEMP-010 | Idempotent queue removal operations | 6 tests |
| REQ-QUEUE-IDEMP-020 | Return value semantics for removal | 3 tests |
| REQ-QUEUE-DEDUP-010 | PassageComplete event deduplication | 6 tests |
| REQ-QUEUE-DEDUP-020 | Deduplication scope (5-second window) | 2 tests |
| REQ-QUEUE-DEDUP-030 | Thread-safe deduplication state | 3 tests |
| REQ-QUEUE-DRY-010 | Single cleanup implementation (DRY) | 5 tests |
| REQ-QUEUE-DRY-020 | Cleanup operation ordering | 2 tests |

**Full Requirements:** See `requirements_index.md` (detailed description, acceptance criteria, rationale)

---

## Scope

### ✅ In Scope

**Reliability Improvements:**
- Idempotent database removal operations
- PassageComplete event deduplication (5-second window)
- DRY refactoring of cleanup logic
- 21 comprehensive tests (unit + integration + system)
- Zero ERROR logs from duplicate events

**Files to Modify (4 existing files):**
- wkmp-ap/src/db/queue.rs (~15 lines)
- wkmp-ap/src/playback/engine/core.rs (~5 lines - add field)
- wkmp-ap/src/playback/engine/diagnostics.rs (~30 lines)
- wkmp-ap/src/playback/engine/queue.rs (~200 lines - helper + refactoring)

**New Test Files (4 new):**
- wkmp-ap/tests/queue_deduplication_tests.rs (~200 lines)
- wkmp-ap/tests/cleanup_helper_tests.rs (~150 lines)
- wkmp-ap/tests/queue_removal_integration_tests.rs (~100 lines)
- wkmp-ap/tests/system_queue_resilience_tests.rs (~100 lines)

### ❌ Out of Scope

**Explicitly Deferred (Future Enhancements):**
- Telemetry SSE events for duplicate detection (SPEC029:1170)
- Configurable deduplication window (SPEC029:1173)
- Event source consolidation (mixer/marker changes, SPEC029:1176)
- Background cleanup task (alternative to per-event spawn, SPEC029:342)
- Performance optimization (profiling deferred to future)

**Architecture Constraints:**
- No breaking changes to HTTP REST API
- No SSE event format changes
- No new external dependencies (Cargo crates)
- No database schema changes (no migrations)

**Full Scope:** See `scope_statement.md` (assumptions, constraints, success criteria)

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 0 ✓
- **HIGH Issues:** 0 ✓
- **MEDIUM Issues:** 2 (documentation clarifications, not blocking)
- **LOW Issues:** 1 (rollback plan note)

**Decision:** ✅ **PROCEED** - No blocking issues

**Medium Issues (Acceptable):**
1. Cleanup mechanism not in requirement text (but detailed in implementation section 4.2.2)
2. Cleanup steps not listed in REQ-DRY-010 (listed in REQ-DRY-020 - cross-reference needed)

**Resolution:** Both addressed via code comments during implementation, no specification updates required.

**Full Analysis:** See `01_specification_issues.md` (requirement-by-requirement completeness check)

---

## Implementation Roadmap

### Increment 1: Idempotent Database Operations

**Objective:** Make queue removal idempotent (safe to retry)

**Effort:** 1.5 hours

**Deliverables:**
- Modify `db::queue::remove_from_queue()` to return `Result<bool>`
- Update callers in queue.rs and diagnostics.rs to handle new return type
- 3 inline unit tests (db/queue.rs)

**Tests:**
- TC-U-IDEMP-001: First removal returns Ok(true)
- TC-U-IDEMP-002: Second removal returns Ok(false)
- TC-U-IDEMP-003: Never-existed returns Ok(false)

**Success Criteria:**
- All 3 unit tests pass
- No ERROR logs for duplicate removal
- Existing test suite passes (no regression)

**Implementation Guide:**
- Read SPEC029 Section 4.1 (lines 147-240)
- Follow implementation pattern (code examples provided)
- Test-first: Write tests before changing remove_from_queue()

---

### Increment 2: PassageComplete Event Deduplication

**Objective:** Deduplicate PassageComplete events (5-second window)

**Effort:** 1.5 hours

**Deliverables:**
- Add `completed_passages` field to PlaybackEngine
- Implement dedup logic in diagnostics.rs PassageComplete handler
- Per-event cleanup spawn (5-second delay)
- 4 unit tests + 1 integration test

**Tests:**
- TC-U-DEDUP-001: First event processes
- TC-U-DEDUP-002: Duplicate event ignored
- TC-U-DEDUP-003: Multiple distinct events
- TC-U-DEDUP-004: Stale entry cleanup
- TC-I-REMOVAL-002: Duplicate event in real scenario

**Success Criteria:**
- All 5 tests pass
- Debug log shows "Ignoring duplicate PassageComplete"
- No memory leak (cleanup verified)
- Existing test suite passes

**Implementation Guide:**
- Read SPEC029 Section 4.2 (lines 246-373)
- Use per-event spawn pattern (simpler than background task)
- Deduplication check BEFORE processing event

---

### Increment 3: DRY Cleanup Refactoring

**Objective:** Single cleanup implementation (reduce duplication 40-60%)

**Effort:** 1.5 hours

**Deliverables:**
- Create `cleanup_queue_entry()` helper method
- Refactor skip_next() to use helper
- Refactor remove_queue_entry() to use helper
- Refactor complete_passage_removal() to use helper
- 3 unit tests + 2 integration tests + 2 system tests

**Tests:**
- TC-U-DRY-001: Cleanup helper order verification
- TC-U-DRY-002: Cleanup helper idempotent
- TC-U-DRY-003: Skip uses cleanup helper
- TC-I-REMOVAL-001: Normal passage completion
- TC-I-ADV-001: Promotion triggers decode
- TC-S-RESIL-001: Rapid skip operations
- TC-S-RESIL-002: EOF before crossfade

**Success Criteria:**
- All 7 tests pass
- Line count reduced 40-60% (measured)
- Cleanup order verified (mock component tracking)
- All existing tests pass (100%)

**Implementation Guide:**
- Read SPEC029 Section 4.3 (lines 375-669)
- Extract cleanup logic into helper first
- Refactor one caller at a time (test after each)
- Maintain exact behavior (no functional changes)

---

**Total Estimated Effort:** 4.5 hours (plus 0.5h for integration/final testing = 5 hours total)

---

## Test Coverage Summary

**Total Tests:** 21
- **Unit Tests:** 9 (43%) - Fast, isolated component tests
- **Integration Tests:** 3 (14%) - Real component interactions
- **System Tests:** 2 (10%) - End-to-end scenarios
- **Inline Unit Tests:** 7 (33%) - In db/queue.rs #[cfg(test)]

**Coverage:** 100% - All 7 requirements have acceptance tests

**Test Organization:**
- Idempotency: 3 inline unit tests (db/queue.rs)
- Deduplication: 4 unit tests (queue_deduplication_tests.rs)
- DRY Helper: 3 unit tests (cleanup_helper_tests.rs)
- Integration: 3 tests (queue_removal_integration_tests.rs)
- System: 2 tests (system_queue_resilience_tests.rs)

**Traceability:** Complete matrix in `02_test_specifications/traceability_matrix.md`
- Forward: Every requirement has tests ✓
- Backward: Every test traces to requirement ✓
- Implementation: Every requirement has code location ✓

---

## Risk Assessment

**Residual Risk:** Low (after mitigations)

**Top Risks:**

1. **Idempotency Logic Bug**
   - Risk: Low
   - Mitigation: Comprehensive unit tests for all edge cases
   - Tests: TC-U-IDEMP-001/002/003

2. **Deduplication State Memory Leak**
   - Risk: Low
   - Mitigation: Automatic cleanup after 5 seconds, unit test verifies
   - Tests: TC-U-DEDUP-004

3. **Refactoring Introduces Regression**
   - Risk: Low
   - Mitigation: Test-first approach, existing test suite must pass
   - Tests: All existing tests + new DRY tests

**Risk-Based Decision:** Approach 1 (Idempotent + Dedup + DRY) chosen as lowest-risk solution per CLAUDE.md Risk-First Framework.

**Full Risk Analysis:** See SPEC029 Section 7 (Failure Mode Analysis, Mitigation Planning)

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Security issues
- Deferred requirements

See Phase 9 section of CLAUDE.md /plan workflow for 7-step technical debt discovery process.

**Post-Implementation:** This section will be updated with technical debt report before plan is marked complete.

---

## Success Metrics

### Quantitative Metrics

1. **ERROR Log Elimination**
   - Current: >0 ERROR logs per duplicate event
   - Target: 0 ERROR logs
   - Measurement: Integration test TC-I-REMOVAL-002 log analysis

2. **Code Duplication Reduction**
   - Current: ~120 lines duplicated across 3 methods
   - Target: <70 lines (40-60% reduction)
   - Measurement: Line count before/after refactoring

3. **Test Coverage**
   - Current: Edge cases not covered
   - Target: 100% coverage for new code
   - Measurement: cargo-tarpaulin coverage report

4. **Regression Prevention**
   - Current: Baseline (all tests passing)
   - Target: 100% passing (no regressions)
   - Measurement: Full test suite execution

### Qualitative Metrics

1. **Log Clarity:** Logs distinguish idempotent no-op (DEBUG) from errors (ERROR)
2. **Code Maintainability:** Single cleanup implementation easier to understand
3. **Deduplication Transparency:** Future developers understand mechanism easily

---

## Dependencies

### Existing Documents (Read-Only)

**Architecture References:**
- docs/SPEC028-playback_orchestration.md (~900 lines) - Event-driven architecture
- docs/SPEC016-decoder_buffer_design.md (~600 lines) - Chain lifecycle
- docs/REQ001-requirements.md (~1500 lines) - Upstream requirements (search "queue")

**Reading Strategy:** Read summaries only, use grep for relevant sections

**Analysis References:**
- wip/queue_handling_mechanism_analysis.md (~850 lines) - Root cause analysis
- wip/SPEC029-queue_handling_resilience.md (1201 lines) - Source specification

### Integration Points

**No Changes to External Interfaces:**
- HTTP REST API: Unchanged (backward compatible)
- SSE Events: Unchanged (no format changes)
- Database Schema: Unchanged (no migrations)
- Microservices: wkmp-ap internal changes only

**Test Infrastructure:**
- ✅ Existing: TestEngine, audio_generator.rs, DecoderWorkerSpy
- ❌ New: assert_no_error_logs(), CleanupSpy, truncated audio file generator

**No External Dependencies:**
- No new Cargo crates required
- All using existing Rust/Tokio/SQLite stack

**Full Dependencies:** See `dependencies_map.md` (files, crates, integration points)

---

## Constraints

**Technical Constraints:**
- Backward compatibility (no breaking API changes)
- Event format stability (no SSE schema changes)
- Technology stack (no new dependencies)
- Coding conventions (follow IMPL002)

**Process Constraints:**
- Test-driven development (write tests first)
- Code review required (peer approval)
- No regression policy (all existing tests must pass)

**Timeline Constraints:**
- One-day implementation target (5 hours estimated)
- Incremental delivery (3 independent increments)

**Quality Constraints:**
- 100% test coverage for new code
- Zero ERROR logs from duplicate events
- 40-60% code duplication reduction

**Full Constraints:** See `scope_statement.md` (detailed constraint analysis)

---

## Next Steps

### Immediate (Ready Now)

1. **Review Plan Outputs:**
   - [ ] Read this summary
   - [ ] Review requirements_index.md (7 requirements)
   - [ ] Review test_index.md (21 tests)
   - [ ] Review traceability_matrix.md (100% coverage)

2. **Verify Understanding:**
   - [ ] Understand problem (duplicate event ERROR logs)
   - [ ] Understand solution (idempotent + dedup + DRY)
   - [ ] Understand test-first approach

3. **Prepare Environment:**
   - [ ] Ensure wkmp-ap compiles (`cargo build -p wkmp-ap`)
   - [ ] Run existing tests (`cargo test -p wkmp-ap`)
   - [ ] Verify baseline (all tests passing)

### Implementation Sequence

**Increment 1: Idempotent Operations** (1.5 hours)
1. Read SPEC029 Section 4.1 (Idempotent Database Operations)
2. Write 3 unit tests (TC-U-IDEMP-001/002/003) in db/queue.rs
3. Run tests (expect failures - not yet implemented)
4. Modify remove_from_queue() to return Result<bool>
5. Update callers (queue.rs, diagnostics.rs)
6. Run tests (expect pass)
7. Run full test suite (verify no regression)
8. Commit with message: "Implement idempotent queue removal (REQ-QUEUE-IDEMP-010/020)"

**Increment 2: Event Deduplication** (1.5 hours)
1. Read SPEC029 Section 4.2 (Event Deduplication)
2. Write 4 unit tests + 1 integration test
3. Run tests (expect failures)
4. Add completed_passages field to PlaybackEngine
5. Implement dedup logic in diagnostics.rs
6. Run tests (expect pass)
7. Run full test suite (verify no regression)
8. Commit with message: "Implement PassageComplete deduplication (REQ-QUEUE-DEDUP-010/020/030)"

**Increment 3: DRY Refactoring** (1.5 hours)
1. Read SPEC029 Section 4.3 (DRY Cleanup Refactoring)
2. Write 3 unit + 2 integration + 2 system tests
3. Run tests (expect failures for new tests, existing pass)
4. Create cleanup_queue_entry() helper
5. Refactor skip_next() to use helper
6. Refactor remove_queue_entry() to use helper
7. Refactor complete_passage_removal() to use helper
8. Run tests (expect all pass)
9. Measure line count reduction (verify 40-60%)
10. Run full test suite (verify 100% passing)
11. Commit with message: "Refactor cleanup to DRY helper (REQ-QUEUE-DRY-010/020)"

### After Implementation

1. **Execute Phase 9: Post-Implementation Review (MANDATORY)**
   - Run 7-step technical debt discovery process
   - Generate technical debt report
   - Update this summary with findings

2. **Verification:**
   - Run all 21 tests (expect 100% passing)
   - Verify traceability matrix 100% complete
   - Verify no ERROR logs in integration tests
   - Measure line count reduction (verify 40-60%)

3. **Documentation:**
   - Create final implementation report
   - Update SPEC029 if clarifications needed
   - Document any deviations from plan

4. **Archive Plan:**
   - Use `/archive-plan PLAN022` when complete
   - Preserves plan in archive branch
   - Removes from wip/ folder

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- `requirements_index.md` - All requirements with priorities (~300 lines)
- `scope_statement.md` - In/out scope, assumptions, constraints (~500 lines)
- `dependencies_map.md` - Files, crates, integration points (~400 lines)
- `01_specification_issues.md` - Phase 2 analysis (~600 lines)

**Test Specifications:**
- `02_test_specifications/test_index.md` - All tests quick reference (~250 lines)
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests mapping (~300 lines)
- Individual test specs: `02_test_specifications/tc_*.md` (see test_index for list)

**Source Specification:**
- `wip/SPEC029-queue_handling_resilience.md` (1201 lines) - Reference for implementation patterns

**For Implementation (Per Increment):**
- Read this summary (~450 lines)
- Read relevant SPEC029 section (~200-300 lines per increment)
- Read relevant test specs (~150-200 lines per increment)
- **Total context per increment:** ~800-950 lines (optimal for AI/human)

---

## Plan Status

**Phase 1-3 Status:** ✅ Complete
- ✅ Phase 1: Scope Definition - 7 requirements, scope clear, dependencies mapped
- ✅ Phase 2: Specification Verification - 0 Critical issues, ready to proceed
- ✅ Phase 3: Test Definition - 21 tests, 100% coverage, traceability verified

**Phases 4-8 Status:** Not Applicable (Week 1 deliverable - Phases 1-3 only)

**Current Status:** ✅ **READY FOR IMPLEMENTATION**

**Estimated Timeline:** 5 hours over 1 working day (3 increments @ 1.5h + 0.5h final integration)

**Implementation Approach:** Test-Driven Development (TDD)
- Write tests first (expect failures)
- Implement minimum code to pass tests
- Refactor while maintaining passing tests
- Verify no regressions (existing tests still pass)

---

## Approval and Sign-Off

**Plan Created:** 2025-11-06
**Plan Status:** ✅ Ready for Implementation Review

**Phase 1-3 Deliverables:**
- ✓ Requirements extracted and documented (7 requirements)
- ✓ Scope defined with clear boundaries
- ✓ Specification analyzed (0 blocking issues)
- ✓ Tests defined with 100% coverage (21 tests)
- ✓ Traceability matrix complete
- ✓ Implementation guidance provided (SPEC029 sections referenced)

**Next Action:** User reviews plan summary and approves implementation start

**User Checklist:**
- [ ] Understand problem being solved
- [ ] Agree with solution approach (idempotent + dedup + DRY)
- [ ] Confirm test coverage is adequate (21 tests, 100%)
- [ ] Accept estimated timeline (5 hours, 1 day)
- [ ] Approve starting Increment 1 (Idempotent Operations)

**Approval:** [Pending user review]

---

**End of Plan Summary**

**For Implementation:** Start with Increment 1 after approval. Read SPEC029 Section 4.1 for detailed implementation patterns.

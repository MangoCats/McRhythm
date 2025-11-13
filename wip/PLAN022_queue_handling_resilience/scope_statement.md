# Scope Statement: PLAN022 Queue Handling Resilience

**Plan:** PLAN022 Queue Handling Resilience Improvements
**Specification:** wip/SPEC029-queue_handling_resilience.md
**Date:** 2025-11-06

---

## ✅ In Scope

### What WILL Be Implemented

**1. Idempotent Database Operations** (REQ-QUEUE-IDEMP-010, REQ-QUEUE-IDEMP-020)
- Modify `wkmp-ap/src/db/queue.rs::remove_from_queue()` to return `Result<bool>`
- Return `Ok(true)` on successful removal, `Ok(false)` on idempotent no-op
- Update all callers to handle new return type
- Eliminate ERROR logs for duplicate removal attempts

**2. PassageComplete Event Deduplication** (REQ-QUEUE-DEDUP-010, REQ-QUEUE-DEDUP-020, REQ-QUEUE-DEDUP-030)
- Add `completed_passages` field to PlaybackEngine (Arc<RwLock<HashMap<Uuid, Instant>>>)
- Implement duplicate detection in diagnostics event handler
- Track processed queue_entry_ids for 5 seconds
- Automatic cleanup of stale deduplication entries
- Thread-safe concurrent access via RwLock

**3. DRY Cleanup Refactoring** (REQ-QUEUE-DRY-010, REQ-QUEUE-DRY-020)
- Create single `cleanup_queue_entry()` helper method
- Refactor `skip_next()` to use helper
- Refactor `remove_queue_entry()` to use helper
- Refactor `complete_passage_removal()` to use helper
- Ensure correct cleanup step ordering (6 steps)
- Reduce code duplication by 40-60%

**4. Comprehensive Test Coverage**
- 9 unit tests (idempotency, deduplication, DRY helper)
- 3 integration tests (removal flow, queue advancement, promotion triggers decode)
- 2 system tests (rapid skip, EOF before crossfade)
- Test coverage: 100% for new code paths
- All existing tests must continue passing (regression prevention)

**5. Documentation Updates**
- Inline code comments with requirement IDs
- Update affected function documentation
- Traceability matrix (requirements ↔ tests ↔ implementation)

---

## ❌ Out of Scope

### What Will NOT Be Implemented

**Future Enhancements (Explicitly Deferred):**

1. **Telemetry SSE Events** (SPEC029:1170)
   - `WkmpEvent::DuplicateEventDetected` SSE event
   - Tracking duplicate event count in developer UI
   - Rationale: Enhancement for monitoring, not core resilience fix
   - Defer to: Future monitoring/telemetry work

2. **Configurable Deduplication Window** (SPEC029:1173)
   - Setting `deduplication_window_ms` in database
   - Tunable window for different hardware/workloads
   - Rationale: 5-second window sufficient for all observed cases
   - Defer to: Future if empirical evidence shows need

3. **Event Source Consolidation** (SPEC029:1176)
   - Consolidating EOF markers into single PassageComplete mechanism
   - Mixer/marker architecture changes
   - Rationale: Higher risk, requires deep marker system changes
   - Defer to: Future marker system refactoring

4. **Background Cleanup Task** (SPEC029:342-362)
   - Alternative to per-event spawn for deduplication cleanup
   - Periodic background task (runs every 10 seconds)
   - Rationale: Per-event spawn is simpler, proven acceptable
   - Defer to: Future if profiling shows performance issue

**Other Explicit Exclusions:**

5. **Performance Optimization**
   - Profiling and optimization of deduplication mechanism
   - Rationale: Focus on correctness first, optimize if needed
   - Defer to: Future based on empirical performance data

6. **New External Dependencies**
   - No new crates or libraries
   - Rationale: Use existing Rust/Tokio/SQLite stack
   - Constraint: WKMP architecture requirement

7. **HTTP API Changes**
   - No breaking changes to existing REST endpoints
   - No new API endpoints
   - Rationale: Internal implementation only
   - Constraint: Backward compatibility requirement

8. **SSE Event Format Changes**
   - Existing event formats unchanged
   - Rationale: Consumers depend on current schema
   - Constraint: Backward compatibility requirement

---

## Assumptions

**Assumption 1: SQLite Atomicity**
- **Statement:** SQLite DELETE operations are atomic
- **Justification:** SQLite ACID guarantees
- **Impact if False:** Would need explicit transaction management
- **Mitigation:** Standard SQLite behavior, well-documented
- **Verification:** Covered by unit tests

**Assumption 2: Tokio Concurrency Primitives**
- **Statement:** Arc<RwLock<HashMap>> provides correct concurrent access
- **Justification:** Standard Rust concurrency pattern
- **Impact if False:** Data races in deduplication state
- **Mitigation:** Covered by REQ-QUEUE-DEDUP-030, verified by stress tests
- **Verification:** Concurrent passage completion tests

**Assumption 3: 5-Second Window Sufficiency**
- **Statement:** 5 seconds covers all duplicate event windows
- **Justification:** Observed duplicates within 7ms (SPEC029:369), 5s >> 7ms
- **Impact if False:** Duplicates outside window not caught
- **Mitigation:** Window significantly larger than observed maximum
- **Verification:** Integration tests with rapid events

**Assumption 4: Existing Test Suite Coverage**
- **Statement:** Existing wkmp-ap tests will catch regressions
- **Justification:** Comprehensive test suite exists (event_driven_playback_tests.rs, mixer_tests/)
- **Impact if False:** Behavioral regressions slip through
- **Mitigation:** Run full test suite before/after, verify 100% passing
- **Verification:** CI/CD test execution

**Assumption 5: No Concurrent Database Writers**
- **Statement:** Only PlaybackEngine writes to queue table
- **Justification:** Microservices architecture, wkmp-ap owns queue table
- **Impact if False:** External modifications could cause inconsistencies
- **Mitigation:** Architecture enforces single writer (design assumption)
- **Verification:** Code review, architecture compliance

---

## Constraints

### Technical Constraints

**TC-01: Backward Compatibility**
- **Constraint:** No breaking changes to HTTP REST API endpoints
- **Impact:** Cannot change `/playback/queue/*` request/response formats
- **Enforcement:** API integration tests must pass unchanged

**TC-02: Event Format Stability**
- **Constraint:** SSE event formats unchanged (existing consumers depend on them)
- **Impact:** Cannot modify `WkmpEvent::QueueChanged`, `WkmpEvent::PassageCompleted` schemas
- **Enforcement:** SSE integration tests verify unchanged schemas

**TC-03: Technology Stack**
- **Constraint:** Use existing Rust/Tokio/SQLite stack, no new dependencies
- **Impact:** Cannot introduce new crates (e.g., specialized deduplication libraries)
- **Enforcement:** `Cargo.toml` reviewed during code review

**TC-04: Coding Conventions**
- **Constraint:** Follow WKMP coding conventions (IMPL002)
- **Impact:** Code style, documentation, error handling must match existing patterns
- **Enforcement:** Code review, rustfmt, clippy

**TC-05: Async/Await Patterns**
- **Constraint:** Use Tokio async patterns consistently
- **Impact:** Deduplication state access must be async-safe
- **Enforcement:** Tokio best practices, RwLock for shared state

### Process Constraints

**PC-01: Test-Driven Development**
- **Constraint:** Write tests before implementation (TDD approach per SPEC029)
- **Impact:** Tests define "done" for each requirement
- **Enforcement:** Test files committed before implementation files

**PC-02: Code Review Required**
- **Constraint:** All changes require code review approval
- **Impact:** Cannot merge without peer review
- **Enforcement:** GitHub PR workflow

**PC-03: No Regression Policy**
- **Constraint:** All existing tests must continue passing
- **Impact:** Zero tolerance for behavioral regressions
- **Enforcement:** CI/CD gate (all tests must pass)

### Timeline Constraints

**TL-01: One-Day Implementation Target**
- **Constraint:** Estimated 5 hours total (complete within one working day)
- **Impact:** Prioritize P0 requirements, defer enhancements
- **Timeline Breakdown:**
  - Idempotency: 1.5 hours
  - Deduplication: 1.5 hours
  - DRY Refactoring: 1.5 hours
  - Integration Testing: 0.5 hours

**TL-02: Incremental Delivery**
- **Constraint:** Each phase must be independently testable
- **Impact:** Cannot implement all-at-once (must be incremental)
- **Enforcement:** 3 separate increments (Idempotency → Deduplication → DRY)

### Quality Constraints

**QC-01: Test Coverage Target**
- **Constraint:** 100% test coverage for new code paths
- **Impact:** Every new function, branch, error case must have test
- **Enforcement:** Code coverage tool (cargo-tarpaulin), review checklist

**QC-02: Zero ERROR Logs**
- **Constraint:** Duplicate events must not produce ERROR logs
- **Impact:** Success metric (ERROR count: current >0 → target 0)
- **Enforcement:** Integration test log analysis

**QC-03: Code Duplication Reduction**
- **Constraint:** 40-60% reduction in cleanup code duplication
- **Impact:** Measurable improvement in maintainability
- **Enforcement:** Line count before/after (quantitative metric)

---

## Dependencies

### Code Dependencies (Files to Modify)

**Primary Implementation Files:**

1. **wkmp-ap/src/db/queue.rs**
   - Function: `remove_from_queue()`
   - Change: Return type `Result<()>` → `Result<bool>`
   - Lines Modified: ~15 lines

2. **wkmp-ap/src/playback/engine/core.rs**
   - Struct: `PlaybackEngine`
   - Change: Add `completed_passages` field
   - Lines Modified: ~5 lines (field declaration)

3. **wkmp-ap/src/playback/engine/diagnostics.rs**
   - Function: PassageComplete event handler
   - Change: Add deduplication logic
   - Lines Modified: ~30 lines (before event processing)

4. **wkmp-ap/src/playback/engine/queue.rs**
   - New Function: `cleanup_queue_entry()` (helper)
   - Functions to Refactor: `skip_next()`, `remove_queue_entry()`, `complete_passage_removal()`
   - Lines Modified: ~200 lines (new helper + 3 refactorings)

**Test Files (New):**

5. **wkmp-ap/tests/queue_deduplication_tests.rs**
   - New file: Unit tests for deduplication logic
   - Lines: ~200 lines

6. **wkmp-ap/tests/cleanup_helper_tests.rs**
   - New file: Unit tests for DRY helper
   - Lines: ~150 lines

7. **wkmp-ap/tests/queue_removal_integration_tests.rs**
   - New file: Integration tests for removal flow
   - Lines: ~100 lines

8. **wkmp-ap/tests/system_queue_resilience_tests.rs**
   - New file: System tests for real-world scenarios
   - Lines: ~100 lines

### Document Dependencies (Read-Only)

**Architecture Documents:**

1. **docs/SPEC028-playback_orchestration.md**
   - Purpose: Event-driven architecture reference
   - Usage: Understand marker-driven events, watchdog patterns
   - Lines: ~900 lines (read summary only)

2. **docs/SPEC016-decoder_buffer_design.md**
   - Purpose: Chain lifecycle reference
   - Usage: Understand cleanup ordering rationale
   - Lines: ~600 lines (read lifecycle section only)

3. **docs/REQ001-requirements.md**
   - Purpose: Upstream requirements reference
   - Usage: Verify no conflicts with existing requirements
   - Lines: ~1500 lines (search for queue-related requirements only)

**Analysis Documents:**

4. **wip/queue_handling_mechanism_analysis.md**
   - Purpose: Root cause analysis
   - Usage: Understand problem context and failure scenarios
   - Lines: ~850 lines (read executive summary + Section 3)

### External Dependencies (None)

**No New External Dependencies:**
- No new Cargo crates required
- No external services required
- No new environment dependencies

**Existing Dependencies (Unchanged):**
- Rust stable channel (existing)
- Tokio async runtime (existing)
- SQLite with JSON1 extension (existing)
- sqlx database library (existing)

### Integration Points

**1. Database Schema**
- Table: `queue` (existing, no schema changes)
- Operations: DELETE only (already used)
- No migrations required

**2. HTTP REST API**
- Endpoints: `/playback/skip`, `/playback/queue/{id}`, `/playback/queue` (existing, no changes)
- Request/Response: Unchanged
- No API version bump required

**3. SSE Events**
- Events: `QueueChanged`, `PassageCompleted` (existing, no format changes)
- Consumers: wkmp-ui, developer UI (no changes required)
- No SSE schema version bump required

**4. Existing Test Infrastructure**
- Test helpers: `wkmp-ap/tests/test_engine.rs` (existing)
- Audio generators: `wkmp-ap/tests/helpers/audio_generator.rs` (existing)
- Mock components: Spy pattern established (reuse existing patterns)

---

## Boundary Conditions

### What Happens at the Edges

**Edge Condition 1: Concurrent Removal Attempts**
- **Scenario:** Two threads attempt to remove same queue_entry_id simultaneously
- **Expected Behavior:** One succeeds (Ok(true)), one gets idempotent no-op (Ok(false))
- **Covered By:** REQ-QUEUE-IDEMP-010, TC-U-IDEMP-003

**Edge Condition 2: Duplicate Events > 5 Seconds Apart**
- **Scenario:** Two PassageComplete events for same queue_entry_id, 6 seconds apart
- **Expected Behavior:** Both process (deduplication window expired)
- **Coverage:** Not a bug - 6 seconds is long enough for genuine re-queue
- **Covered By:** TC-U-DEDUP-004 (verifies cleanup)

**Edge Condition 3: Queue Entry Never Existed**
- **Scenario:** Attempt to remove queue_entry_id that was never in database
- **Expected Behavior:** Idempotent no-op (Ok(false)), no ERROR log
- **Covered By:** REQ-QUEUE-IDEMP-010, TC-U-IDEMP-003

**Edge Condition 4: Database Connection Failure**
- **Scenario:** SQLite database unavailable during removal
- **Expected Behavior:** Return Err (propagate database error)
- **Coverage:** True error (not idempotent case)
- **Covered By:** Existing error handling, integration tests

**Edge Condition 5: Memory Pressure (Deduplication HashMap)**
- **Scenario:** Thousands of passages complete, deduplication map grows large
- **Expected Behavior:** Automatic cleanup keeps size bounded (5-second window)
- **Coverage:** HashMap size <= (passages completing per 5 seconds)
- **Covered By:** TC-U-DEDUP-004, long-running system test

---

## Success Criteria

### Quantitative Success Metrics

**Metric 1: ERROR Log Elimination**
- **Current State:** >0 ERROR logs per duplicate event
- **Target State:** 0 ERROR logs per duplicate event
- **Measurement:** Integration test log analysis (TC-I-REMOVAL-002)
- **Pass Criteria:** Zero occurrences of "Failed to remove entry from database: Queue error: Queue entry not found"

**Metric 2: Code Duplication Reduction**
- **Current State:** ~120 lines of duplicated cleanup logic across 3 methods
- **Target State:** <70 lines (40-60% reduction)
- **Measurement:** Line count before/after refactoring
- **Pass Criteria:** Lines reduced by ≥40% AND functionality unchanged

**Metric 3: Test Coverage**
- **Current State:** Edge cases not covered (duplicate events, idempotent removal)
- **Target State:** 100% coverage for new code paths
- **Measurement:** cargo-tarpaulin or similar coverage tool
- **Pass Criteria:** All new functions/branches have tests, coverage report shows 100%

**Metric 4: Existing Test Pass Rate**
- **Current State:** Baseline (all tests passing before changes)
- **Target State:** 100% passing (no regressions)
- **Measurement:** Full test suite execution
- **Pass Criteria:** ALL existing tests pass unchanged

### Qualitative Success Metrics

**Quality 1: Log Clarity**
- **Assessment:** Logs clearly distinguish idempotent no-op (DEBUG) from errors (ERROR)
- **Verification:** Log output review during integration testing
- **Pass Criteria:** Debug log says "already removed", not ERROR

**Quality 2: Code Maintainability**
- **Assessment:** Single cleanup implementation easier to understand and modify
- **Verification:** Code review feedback, documentation quality
- **Pass Criteria:** Reviewer confirms improved clarity

**Quality 3: Deduplication Transparency**
- **Assessment:** Future developers can understand deduplication mechanism easily
- **Verification:** Inline documentation, clear variable names
- **Pass Criteria:** Code review confirms self-documenting implementation

---

## Risks and Mitigation

### Implementation Risks (See SPEC029 Section 7.1 for complete analysis)

**Risk 1: Idempotency Logic Bug**
- **Probability:** Low
- **Impact:** Medium (false positives/negatives)
- **Mitigation:** Comprehensive unit tests for all edge cases
- **Status:** Covered by test specifications (TC-U-IDEMP-*)

**Risk 2: Deduplication State Memory Leak**
- **Probability:** Low
- **Impact:** Low (gradual memory growth)
- **Mitigation:** Automatic cleanup after 5 seconds, unit test for cleanup
- **Status:** Covered by REQ-QUEUE-DEDUP-010, TC-U-DEDUP-004

**Risk 3: Refactoring Introduces Regression**
- **Probability:** Low
- **Impact:** High (behavioral change)
- **Mitigation:** Maintain existing test suite passing, add new edge case tests
- **Status:** Test-first approach, no-regression policy

---

## Scope Sign-Off

**Scope Defined:** 2025-11-06
**Defined By:** Claude Code (Plan Workflow Phase 1)
**Reviewed By:** [Pending user review]
**Status:** Ready for Phase 2 (Specification Completeness Verification)

**User Checkpoint:**
- [ ] In-scope items are clear and complete
- [ ] Out-of-scope exclusions are acceptable
- [ ] Assumptions are reasonable
- [ ] Constraints are understood
- [ ] Success criteria are measurable

**Approval to Proceed:** [Pending]

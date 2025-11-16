# PLAN026: Batch Writes Optimization - Requirements Index

**Source:** User request + conversation analysis (database lock contention issues)
**Created:** 2025-01-15
**Priority System:** P0 (Critical) / P1 (High) / P2 (Medium)

---

## Requirements Summary

**Total Requirements:** 11
- P0 (Critical): 5
- P1 (High): 4
- P2 (Medium): 2

---

## Functional Requirements

| Req ID | Priority | Brief Description | Category | Source |
|--------|----------|-------------------|----------|--------|
| REQ-BW-010 | P0 | Reduce database lock acquisitions by 10-20× per file in import pipeline | Performance | Conversation analysis |
| REQ-BW-020 | P0 | Batch database writes within single transactions | Performance | passage_recorder.rs pattern |
| REQ-BW-030 | P1 | Pre-fetch reads outside transaction boundaries | Performance | passage_recorder.rs:103-118 |
| REQ-BW-040 | P1 | Maintain transaction atomicity (all-or-nothing semantics) | Correctness | SQLite transaction semantics |
| REQ-BW-050 | P2 | Preserve existing retry logic for transient lock errors | Reliability | db_retry.rs |

## Dead Code Removal Requirements

| Req ID | Priority | Brief Description | Category | Source |
|--------|----------|-------------------|----------|--------|
| REQ-DC-010 | P0 | Remove all unused functions and modules before batch writes implementation | Code Quality | User requirement |
| REQ-DC-020 | P0 | Remove all unused functions and modules after batch writes implementation | Code Quality | User requirement |
| REQ-DC-030 | P1 | Remove unused imports and dependencies | Code Quality | Rust best practices |
| REQ-DC-040 | P1 | Document rationale for any code marked dead but kept | Code Quality | Maintainability |

## Non-Functional Requirements

| Req ID | Priority | Brief Description | Category | Source |
|--------|----------|-------------------|----------|--------|
| REQ-NF-010 | P0 | No regression in test coverage (maintain current %) | Quality | Testing standards |
| REQ-NF-020 | P2 | Improve import throughput by measurable amount | Performance | Performance optimization goal |

---

## Requirements Detail

### REQ-BW-010: Reduce Lock Acquisitions (P0)

**Statement:** The import pipeline SHALL reduce database lock acquisitions by 10-20× per file through batching.

**Rationale:**
- Current implementation: ~70% writes in per-file pipeline
- Each write acquires exclusive lock (SQLite WAL limitation)
- 16 workers competing for single write slot creates contention
- Evidence: Git history shows ongoing lock contention issues

**Acceptance Criteria:**
- Measure lock acquisitions before/after optimization
- Target: 1-2 lock acquisitions per file (vs. 10-20 currently)
- Verify through transaction monitoring

**Priority:** P0 - Addresses root cause of database lock issues

---

### REQ-BW-020: Batch Writes in Transactions (P0)

**Statement:** Database write operations SHALL be batched within single transactions where atomicity allows.

**Rationale:**
- Single transaction = single lock acquisition
- Example: Insert file + passages + relationships in one commit
- Proven pattern exists in passage_recorder.rs:103-145

**Acceptance Criteria:**
- Per-file pipeline uses ≤2 transactions (setup + commit)
- All related writes batched together
- Transaction duration <100ms (target from passage_recorder.rs:80)

**Priority:** P0 - Core optimization technique

---

### REQ-BW-030: Pre-fetch Reads Outside Transactions (P1)

**Statement:** Read operations SHALL be executed outside transaction boundaries to minimize connection hold time.

**Rationale:**
- Read operations don't need transaction protection
- Moving reads outside reduces transaction duration
- Pattern demonstrated in passage_recorder.rs:103-118

**Acceptance Criteria:**
- All SELECT queries executed before BEGIN TRANSACTION
- Results cached for use within transaction
- Transaction contains only INSERT/UPDATE operations

**Priority:** P1 - Significant performance improvement

---

### REQ-BW-040: Maintain Transaction Atomicity (P1)

**Statement:** All batched operations SHALL maintain atomic commit semantics (all succeed or all fail).

**Rationale:**
- Data consistency requires all-or-nothing semantics
- Database relationships must remain valid
- No partial imports allowed

**Acceptance Criteria:**
- Transaction failure rolls back all changes
- No orphaned database records after errors
- Foreign key constraints remain valid

**Priority:** P1 - Correctness requirement

---

### REQ-BW-050: Preserve Retry Logic (P2)

**Statement:** Existing retry logic for transient database lock errors SHALL be preserved.

**Rationale:**
- db_retry.rs provides exponential backoff (10ms → 1000ms)
- Handles "database is locked" gracefully
- Proven infrastructure should not be discarded

**Acceptance Criteria:**
- retry_on_lock() continues to wrap batch operations
- Backoff behavior unchanged
- ai_database_max_lock_wait_ms setting honored

**Priority:** P2 - Nice to have, but batching may eliminate need

---

### REQ-DC-010: Pre-Implementation Dead Code Removal (P0)

**Statement:** All unused code in wkmp-ai SHALL be removed BEFORE implementing batch writes, including test-support code that is not used by the live application.

**Rationale:**
- Clean baseline simplifies understanding scope
- Reduces merge conflicts during implementation
- Easier to identify what changes during batch writes work

**Scope - Code to Remove:**
- Unused functions, structs, modules in src/
- Test helper functions/fixtures used ONLY by tests (tests will be refactored as needed)
- Code behind #[cfg(test)] that serves no test
- Unused imports and dependencies

**Scope - Code to Keep:**
- Live application code (used by main.rs or lib.rs)
- Code used in production builds
- Documented intentionally-retained code (#[allow(dead_code)] with rationale)

**Acceptance Criteria:**
- cargo build succeeds with no warnings
- cargo clippy --all-targets reports zero unused warnings
- Test-support code removed if not used by application
- Tests refactored to remove dependencies on dead test utilities
- All remaining tests continue to pass after removal

**Priority:** P0 - User requirement, establishes clean baseline

---

### REQ-DC-020: Post-Implementation Dead Code Removal (P0)

**Statement:** All unused code created or exposed by batch writes optimization SHALL be removed AFTER implementation, including obsolete test-support code.

**Rationale:**
- Batch writes may obsolete old write paths
- Helper functions may no longer be needed
- Test utilities for old patterns no longer necessary
- Keep codebase lean and maintainable

**Scope - Code to Remove:**
- Obsolete write helper functions replaced by batch writes
- Old transaction patterns no longer used
- Test-support code for old patterns (tests refactored to use new patterns)
- Unused imports exposed by refactoring

**Acceptance Criteria:**
- cargo build succeeds with no warnings
- cargo clippy --all-targets reports zero unused warnings
- No dead code introduced by batch writes refactoring
- Old write patterns removed (replaced by batch writes)
- Tests refactored to use new batch write patterns
- All tests continue to pass after removal

**Priority:** P0 - User requirement, maintains code quality

---

### REQ-DC-030: Remove Unused Imports (P1)

**Statement:** Unused imports and dependencies SHALL be removed during dead code cleanup.

**Rationale:**
- Reduces compilation time
- Simplifies dependency graph
- Improves code readability

**Acceptance Criteria:**
- cargo build shows no "unused import" warnings
- Cargo.toml dependencies all referenced in code
- rustfmt and clippy pass cleanly

**Priority:** P1 - Code quality improvement

---

### REQ-DC-040: Document Retained Dead Code (P1)

**Statement:** Any code identified as unused but intentionally retained SHALL be documented with rationale.

**Rationale:**
- Future readers need to understand why code exists
- Prevents accidental removal of needed code
- Makes intentional decisions visible

**Acceptance Criteria:**
- #[allow(dead_code)] with comment explaining why
- Module-level documentation for experimental/future code
- Clear markers for "used by external tools" code

**Priority:** P1 - Documentation quality

---

### REQ-NF-010: No Test Coverage Regression (P0)

**Statement:** Test coverage percentage SHALL NOT decrease due to batch writes implementation or dead code removal.

**Rationale:**
- Quality bar must not regress
- Batch writes change implementation, not functionality
- Dead code removal should increase coverage % (fewer lines)

**Acceptance Criteria:**
- Run coverage before/after (cargo tarpaulin or similar)
- Coverage % ≥ baseline
- All existing tests continue to pass

**Priority:** P0 - Quality gate

---

### REQ-NF-020: Measurable Throughput Improvement (P2)

**Statement:** Import throughput SHOULD improve by measurable amount after batch writes optimization.

**Rationale:**
- Validates optimization effectiveness
- Provides data for future optimization decisions
- Confirms lock contention reduction

**Acceptance Criteria:**
- Benchmark import of 100-file test dataset
- Record time before/after optimization
- Throughput increase of 20%+ desirable (not required)

**Priority:** P2 - Success metric, but not blocking

---

## Scope Boundaries

### ✅ In Scope

- Batch write optimization in wkmp-ai import pipeline
- Dead code removal before implementation
- Dead code removal after implementation
- Transaction optimization (pre-fetch reads, batch writes)
- Preserve retry logic and error handling
- Maintain test coverage

### ❌ Out of Scope

- Connection pool tuning (separate optimization)
- WAL checkpoint tuning (separate optimization)
- Dedicated writer task (rejected approach)
- Changes to other microservices (wkmp-ap, wkmp-ui, etc.)
- New features or functionality
- Performance optimization beyond batch writes

---

## Dependencies

### Existing Code
- wkmp-ai/src/utils/db_retry.rs (retry logic)
- wkmp-ai/src/utils/pool_monitor.rs (transaction monitoring)
- wkmp-ai/src/services/passage_recorder.rs (reference pattern)
- wkmp-ai/src/db/*.rs (database helper modules)

### External Libraries
- sqlx (database access)
- tokio (async runtime)

### Configuration
- ai_database_max_lock_wait_ms setting (retry timeout)

---

## Constraints

**Technical:**
- SQLite WAL mode (1 writer at a time)
- 16-worker parallelism (CPU-bound workload)
- Must preserve transaction atomicity
- Rust async/await patterns (tokio)

**Process:**
- Test-driven: Verify no regression
- Incremental: Small, verifiable steps
- Documented: Rationale for all changes

**Quality:**
- Zero compiler warnings after dead code removal
- All tests must pass after each increment
- No decrease in test coverage percentage

---

## Assumptions

1. Current implementation has ~102 write operations across 27 files
2. passage_recorder.rs pattern (lines 103-145) is proven effective
3. Import pipeline is bottlenecked on database writes (70% of operations)
4. Dead code removal will not break external integrations
5. cargo clippy and cargo build are sufficient for dead code detection

---

## Success Metrics

**Quantitative:**
- Lock acquisitions: 10-20 per file → 1-2 per file
- Transaction count: Reduced by 80%+
- Compiler warnings: Current → 0
- Test coverage: No regression

**Qualitative:**
- Code easier to understand (dead code removed)
- Import pipeline more maintainable
- Lock contention reduced (observable in logs)

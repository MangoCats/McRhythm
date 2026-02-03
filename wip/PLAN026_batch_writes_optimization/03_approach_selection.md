# PLAN026: Approach Selection

**Plan:** PLAN026 - Batch Writes Optimization
**Phase:** 4 - Approach Selection
**Created:** 2025-01-15

---

## Executive Summary

**Approaches Evaluated:** 2
1. Batch Writes Optimization (RECOMMENDED)
2. Dedicated Writer Task (REJECTED)

**Selected Approach:** Batch Writes Optimization
**Rationale:** Lowest residual risk (LOW) with proven effectiveness. Dedicated Writer Task has MEDIUM-HIGH risk with uncertain benefit.

**Decision Framework:** Per CLAUDE.md Risk-First Framework
- **Primary criterion:** Residual risk (after mitigation)
- **Secondary criterion:** Quality characteristics (when risks equivalent)
- **Tertiary criterion:** Effort (when risk and quality equivalent)

---

## Approach 1: Batch Writes Optimization (RECOMMENDED)

### Description

Apply proven pattern from passage_recorder.rs (lines 103-145) to all write-heavy paths in import pipeline.

**Core Technique:**
1. **Pre-fetch reads OUTSIDE transactions** (minimize connection hold time)
2. **Batch related writes in SINGLE transaction** (reduce lock acquisitions)
3. **Transaction duration target: <100ms** (reduce contention window)

**Implementation Strategy:**
- Add batch write helper functions (batch_insert_songs, batch_insert_artists, etc.)
- Refactor orchestrator phases to use batch writes
- Preserve existing retry logic (db_retry.rs)
- Incremental implementation (one phase at a time)

---

### Risk Assessment

#### Failure Mode 1: Transaction Atomicity Violation

**Description:** Batching multiple writes into single transaction accidentally breaks existing commit semantics, leading to partial commits or rollback failures.

**Probability:** LOW
- Pattern proven in passage_recorder.rs (production code)
- SQLite ACID guarantees strong
- Test coverage comprehensive (TC-U-BW-040-01, TC-I-BW-040-02)

**Impact:** HIGH
- Data corruption possible
- Inconsistent database state
- Foreign key violations

**Mitigation Strategies:**
1. **Follow proven pattern:** Replicate passage_recorder.rs:130-145 transaction structure exactly
2. **Comprehensive testing:** TC-U-BW-040-01 (rollback test), TC-I-BW-040-02 (no partial commits)
3. **Code review:** Manual review of all transaction boundaries before merging
4. **Incremental rollout:** One phase at a time, verify each before proceeding

**Residual Risk (after mitigation):** LOW
- Pattern proven effective
- Tests verify behavior
- Incremental approach allows early detection

---

#### Failure Mode 2: Performance Regression

**Description:** Batching introduces unexpected overhead (larger transactions, memory usage) that negates lock reduction benefits.

**Probability:** LOW
- passage_recorder.rs demonstrates <100ms transactions
- Batching reduces total work (fewer lock acquisitions)
- Pre-fetching moves slow operations outside transaction

**Impact:** MEDIUM
- Import throughput does not improve (or worsens)
- Wasted implementation effort
- May need to revert changes

**Mitigation Strategies:**
1. **Measure baseline:** TC-U-BW-010-01 (baseline locks), TC-S-NF-020-01 (baseline throughput)
2. **Monitor transaction duration:** TC-I-BW-020-02 (verify <100ms target)
3. **Benchmark after implementation:** TC-S-NF-020-02 (compare throughput)
4. **Rollback plan:** Git-based revert if performance degrades

**Residual Risk (after mitigation):** LOW
- Measurement allows objective comparison
- Pattern already proven fast
- Rollback plan available

---

#### Failure Mode 3: Dead Code Removal Breaks Tests

**Description:** Code identified as "unused" by cargo clippy is actually needed (used via macros, external tools, conditional compilation).

**Probability:** MEDIUM
- Clippy is accurate but not perfect
- Macro usage may hide dependencies
- #[cfg] conditional compilation can mislead

**Impact:** LOW
- Tests fail during dead code removal
- Easy to detect (test suite runs after each removal)
- Easy to fix (git revert individual file)

**Mitigation Strategies:**
1. **Incremental removal:** One file at a time (TC-M-DC-010-01 procedure)
2. **Test after each removal:** TC-M-DC-010-02 (verify tests pass)
3. **Git-based revert:** Immediate rollback if tests fail
4. **Manual review:** Check for #[cfg], macro usage, external tool dependencies
5. **Documentation:** Mark retained code with #[allow(dead_code)] + comment (REQ-DC-040)

**Residual Risk (after mitigation):** LOW
- Incremental approach enables fast detection
- Revert procedure well-defined
- Impact limited to single file

---

### Quality Characteristics

**Maintainability:** HIGH
- Pattern already understood (passage_recorder.rs reference)
- Batch functions follow existing db/*.rs conventions
- Clear transaction boundaries
- Well-documented with rationale

**Test Coverage:** HIGH
- 21 tests covering all requirements
- 100% requirement traceability
- Unit + Integration + System tests
- Baseline + verification measurements

**Architectural Alignment:** HIGH
- No architectural changes required
- Preserves microservices pattern
- Uses existing retry infrastructure (db_retry.rs)
- Follows Rust async/await conventions

---

### Implementation Effort

**Estimated:** 20-34 hours

**Breakdown:**
- Phase 1 (Pre-dead code): 4-8 hours
- Phase 2 (Batch writes): 10-17 hours
- Phase 3 (Post-dead code): 3-4 hours
- Phase 4 (Verification): 3-5 hours

**Complexity Factors:**
- Proven pattern reduces learning curve
- 102 write operations across 27 files (need to identify batch candidates)
- Incremental implementation allows parallel work (if multiple developers)

---

### Summary: Approach 1

| Criterion | Assessment | Notes |
|-----------|------------|-------|
| **Residual Risk** | **LOW** | All failure modes mitigated effectively |
| **Maintainability** | HIGH | Proven pattern, clear structure |
| **Test Coverage** | HIGH | 100% requirement coverage |
| **Architectural Alignment** | HIGH | No architectural changes |
| **Effort** | 20-34 hours | Moderate effort, proven pattern reduces risk |

**Recommendation:** ✅ **APPROVE** - Lowest residual risk with high quality characteristics.

---

## Approach 2: Dedicated Writer Task (REJECTED)

### Description

Create single dedicated task that handles all database writes via message-passing channel.

**Architecture:**
- 16 worker threads send write operations to mpsc channel
- 1 dedicated writer task drains channel, executes writes
- Workers use oneshot channels to receive write confirmation
- Transactions coordinated via message protocol

**Rationale:** Eliminate lock contention by serializing writes in dedicated task, allowing workers to continue CPU-bound work without blocking.

---

### Risk Assessment

#### Failure Mode 1: Channel Deadlock

**Description:** Writer task panics or stalls, all workers hang waiting for responses on oneshot channels.

**Probability:** MEDIUM
- Async coordination bugs (tokio runtime edge cases)
- Panic handling in writer task required
- Channel capacity exhaustion possible

**Impact:** CRITICAL
- Total system hang (all workers blocked)
- Import pipeline completely halted
- Requires process restart

**Mitigation Strategies:**
1. **Panic handler:** Catch panics in writer task, log, restart task
2. **Health check:** Periodic ping/pong to verify writer task alive
3. **Timeout:** Workers timeout oneshot receives after N seconds
4. **Channel monitoring:** Alert if channel depth exceeds threshold

**Residual Risk (after mitigation):** MEDIUM
- Complex async coordination still error-prone
- Panic recovery adds code complexity
- Timeout tuning difficult (how long is "too long"?)

---

#### Failure Mode 2: Response Delivery Failure

**Description:** Writer task sends response, but caller's oneshot receiver already dropped (shutdown, timeout, task cancellation).

**Probability:** LOW-MEDIUM
- Race condition on shutdown
- Worker task cancellation during request
- Tokio runtime scheduling edge cases

**Impact:** MEDIUM
- Silent data loss possible (write succeeded, confirmation lost)
- Worker may retry, causing duplicate writes
- Difficult to debug (no error, data just missing)

**Mitigation Strategies:**
1. **Idempotent writes:** Use INSERT...ON CONFLICT UPDATE (already present)
2. **Write-ahead log:** Log all writes before confirmation (adds overhead)
3. **Graceful shutdown:** Drain channel before terminating
4. **Monitoring:** Track sent vs. confirmed writes

**Residual Risk (after mitigation):** LOW-MEDIUM
- Idempotency helps, but not complete solution
- WAL adds significant overhead
- Monitoring detects but doesn't prevent

---

#### Failure Mode 3: Transaction Atomicity Violation

**Description:** Multi-operation transactions (insert song + artists + passages) cannot be represented cleanly in message protocol, leading to partial commits.

**Probability:** LOW
- Can design batch message types (WriteOp::RecordPassageBatch)
- Transaction token pattern possible (begin/commit/rollback messages)

**Impact:** CRITICAL
- Data corruption (partial transaction commits)
- Foreign key violations
- Inconsistent database state

**Mitigation Strategies:**
1. **Batch message types:** Define mega-operations (WriteOp::CompleteFileImport with all data)
2. **Transaction tokens:** Coordinate multi-message transactions via token
3. **Comprehensive testing:** Verify atomicity under failures

**Residual Risk (after mitigation):** LOW
- Batch messages solve most cases
- Transaction tokens complex but feasible
- Testing can verify

---

#### Failure Mode 4: Workers Still Block on Responses

**Description:** Workers send write request, then immediately await oneshot response, defeating purpose (workers still blocked waiting).

**Probability:** HIGH (architectural issue)
- Many writes need confirmation (passage ID for later updates)
- Cannot proceed without write result
- Fire-and-forget not possible for most operations

**Impact:** HIGH
- Eliminates benefit of dedicated writer (workers still blocked)
- Added complexity without performance gain
- Wasted implementation effort

**Mitigation Strategies:**
1. **Batch worker operations:** Buffer multiple operations, send batch, await batch response
2. **Continuation passing:** Send callback with write request (complex)
3. **Accept blocking:** Acknowledge this is just fancy connection pooling

**Residual Risk (after mitigation):** MEDIUM-HIGH
- Fundamental architectural issue
- Batching helps but adds worker complexity
- May not provide benefit over simpler approach

---

### Quality Characteristics

**Maintainability:** LOW-MEDIUM
- Complex message protocol (15+ WriteOp variants)
- Transaction coordination via messages error-prone
- Difficult to debug (async message passing)
- Panic recovery adds code complexity

**Test Coverage:** MEDIUM
- Can test message protocol
- Difficult to test edge cases (race conditions, panics)
- Integration tests complex (simulate channel scenarios)

**Architectural Alignment:** LOW
- Significant architectural change
- New async coordination patterns
- Departure from proven patterns

---

### Implementation Effort

**Estimated:** 40-60 hours

**Breakdown:**
- Define WriteOp enum: 4-6 hours (15+ variants)
- Implement writer task: 8-12 hours (message loop, panic handling)
- Refactor 102 callsites: 16-24 hours (message passing conversion)
- Transaction protocol: 6-10 hours (batch messages or token pattern)
- Testing: 6-8 hours (unit + integration tests)

**Complexity Factors:**
- Novel architecture (no reference implementation)
- Transaction coordination complex
- Error handling difficult (async edge cases)
- 102 callsites to convert (large surface area)

---

### Summary: Approach 2

| Criterion | Assessment | Notes |
|-----------|------------|-------|
| **Residual Risk** | **MEDIUM-HIGH** | Multiple complex failure modes, uncertain mitigation effectiveness |
| **Maintainability** | LOW-MEDIUM | Complex async coordination, difficult debugging |
| **Test Coverage** | MEDIUM | Edge cases hard to test |
| **Architectural Alignment** | LOW | Significant departure from patterns |
| **Effort** | 40-60 hours | High effort, novel implementation |

**Recommendation:** ❌ **REJECT** - Higher residual risk, lower quality, higher effort. Not justified by uncertain benefit.

---

## Comparative Analysis

### Risk Comparison

| Approach | Residual Risk | Highest Severity Failure Mode |
|----------|---------------|-------------------------------|
| **Batch Writes** | **LOW** | Transaction atomicity violation (LOW prob, HIGH impact, LOW residual) |
| **Dedicated Writer** | **MEDIUM-HIGH** | Channel deadlock (MEDIUM prob, CRITICAL impact, MEDIUM residual) |

**Winner:** Batch Writes (lower residual risk)

---

### Quality Comparison

| Criterion | Batch Writes | Dedicated Writer |
|-----------|--------------|------------------|
| Maintainability | HIGH | LOW-MEDIUM |
| Test Coverage | HIGH | MEDIUM |
| Architectural Alignment | HIGH | LOW |
| **Overall Quality** | **HIGH** | **LOW-MEDIUM** |

**Winner:** Batch Writes (higher quality across all dimensions)

---

### Effort Comparison

| Approach | Estimated Effort | Complexity |
|----------|------------------|------------|
| **Batch Writes** | **20-34 hours** | Moderate (proven pattern) |
| **Dedicated Writer** | **40-60 hours** | High (novel architecture) |

**Winner:** Batch Writes (50% less effort)

---

## Decision Matrix (Risk-First Framework)

**Per CLAUDE.md Decision-Making Framework:**

### Step 1: Rank by Residual Risk (Primary Criterion)

1. **Batch Writes:** LOW risk
2. **Dedicated Writer:** MEDIUM-HIGH risk

**Batch Writes has lower residual risk → Batch Writes is higher priority.**

---

### Step 2: Evaluate Quality (Secondary Criterion)

**Are residual risks equivalent?** NO
- Batch Writes: LOW
- Dedicated Writer: MEDIUM-HIGH

**Risks are NOT equivalent** → Quality comparison not needed (risk already decided).

---

### Step 3: Evaluate Effort (Tertiary Criterion)

**Are residual risks AND quality equivalent?** NO

**Effort comparison not needed** → Risk already decided.

---

### Conclusion

**SELECTED APPROACH: Batch Writes Optimization**

**Justification (Per Risk-First Framework):**
- **Primary criterion (Risk):** Batch Writes has LOW residual risk vs. Dedicated Writer MEDIUM-HIGH
- **Decision:** Choose Batch Writes (lowest residual risk)

**Quality and effort support decision but are not deciding factors:**
- Quality: Batch Writes is HIGH vs. Dedicated Writer LOW-MEDIUM (reinforces choice)
- Effort: Batch Writes is 20-34 hrs vs. Dedicated Writer 40-60 hrs (reinforces choice)

**Per CLAUDE.md:** "Risk of failure to achieve [quality-absolute goals] outweighs implementation time."

---

## Architecture Decision Record (ADR)

### Status

**ACCEPTED**

### Date

2025-01-15

### Context

**Problem:** Database lock contention in wkmp-ai import pipeline causing performance issues and thread starvation.

**Evidence:**
- Git history shows "still fighting database lock issues"
- 16 workers competing for single SQLite write slot
- ~70% write operations in per-file pipeline
- 10-20+ lock acquisitions per file

**Goal:** Reduce lock contention by 10-20× to improve import throughput.

**Constraints:**
- SQLite WAL mode (1 writer at a time - unchangeable)
- Must preserve transaction atomicity
- Must maintain test coverage
- Cannot break existing microservices architecture

---

### Decision

**We will implement batch writes optimization using the proven pattern from passage_recorder.rs (lines 103-145).**

**Approach:**
1. Pre-fetch reads outside transaction boundaries
2. Batch related writes into single transactions
3. Target <100ms transaction duration
4. Reduce lock acquisitions from 10-20 per file to 1-2 per file

**Implementation:**
- Add batch write helper functions to wkmp-ai/src/db/*.rs
- Refactor workflow_orchestrator phases to use batch writes
- Preserve existing retry logic (db_retry.rs)
- Incremental rollout (one phase at a time)

---

### Alternatives Considered

**Dedicated Writer Task (REJECTED)**
- **Description:** Single writer task handling all writes via message channel
- **Pros:** Theoretically eliminates lock contention
- **Cons:**
  - MEDIUM-HIGH residual risk (channel deadlock, response delivery failures)
  - Workers still block awaiting responses (defeats purpose)
  - 40-60 hours effort (2× batch writes)
  - Complex async coordination, difficult debugging
  - Novel architecture (no proven pattern)
- **Rejection Reason:** Higher residual risk not justified by uncertain benefit

**Connection Pool Tuning Only (REJECTED)**
- **Description:** Adjust pool size, tune SQLite pragmas
- **Pros:** Very low effort (1-2 hours)
- **Cons:**
  - Doesn't address root cause (70% writes competing for 1 slot)
  - Marginal improvement expected
  - SQLite WAL limitation unchangeable
- **Rejection Reason:** Insufficient impact on problem

---

### Consequences

**Positive:**
- ✅ 80-90% reduction in lock acquisitions (10-20 → 1-2 per file)
- ✅ Improved import throughput (measurable via TC-S-NF-020-02)
- ✅ Reduced worker idle time (less lock contention)
- ✅ Cleaner codebase after dead code removal
- ✅ Proven pattern (passage_recorder.rs) reduces implementation risk
- ✅ No architectural changes (preserves microservices pattern)

**Negative:**
- ⚠️ Transaction boundaries change (requires careful review)
- ⚠️ Larger transactions (more data in single commit)
- ⚠️ Slightly increased memory usage (caching reads before transaction)
- ⚠️ Dead code removal may expose edge cases (mitigated by incremental testing)

**Mitigations:**
- Comprehensive test coverage (21 tests, 100% requirement coverage)
- Incremental implementation (one phase at a time)
- Transaction duration monitoring (TC-I-BW-020-02)
- Git-based rollback plan if performance degrades

**Risks Accepted:**
- Transaction atomicity violation: LOW residual risk (mitigated by testing)
- Performance regression: LOW residual risk (mitigated by benchmarking + rollback)
- Dead code removal breaks tests: LOW residual risk (mitigated by incremental removal)

---

### Compliance with Project Charter

**Per PCH001 (Project Charter) Goals:**
- **"Flawless audio playback":** Not affected (optimization is internal to import pipeline)
- **"Listener experience reminiscent of 1970s FM radio":** Not affected
- **Quality-absolute goals:** Preserved via comprehensive testing (REQ-NF-010)

**Risk-first approach aligns with charter:**
- Chose lowest-risk approach (batch writes) over higher-risk alternative (dedicated writer)
- Quality characteristics prioritized (maintainability, test coverage)
- Effort differential (2×) accepted to achieve lower risk

---

## Sign-Off

**Approach Selected:** Batch Writes Optimization
**Decision Date:** 2025-01-15
**Decision Framework:** Risk-First (per CLAUDE.md)
**Residual Risk:** LOW
**Estimated Effort:** 20-34 hours

**Phase 4 Complete:** ✅
**Status:** Ready for Phase 5 (Implementation Breakdown)

# PLAN026: Batch Writes Optimization - Scope Statement

**Plan:** PLAN026 - Batch Writes Optimization
**Created:** 2025-01-15

---

## In Scope ✅

### 1. Dead Code Removal (Pre-Implementation)
- Analyze wkmp-ai codebase for unused functions, modules, imports
- Remove all dead code identified by cargo clippy
- Ensure zero compiler warnings after removal
- Verify all tests pass after cleanup

### 2. Batch Writes Optimization
- Identify write-heavy paths in import pipeline
- Implement batch write pattern from passage_recorder.rs
- Pre-fetch reads outside transaction boundaries
- Consolidate related writes into single transactions
- Reduce database lock acquisitions by 10-20× per file

### 3. Dead Code Removal (Post-Implementation)
- Identify obsolete code paths replaced by batch writes
- Remove unused helper functions no longer needed
- Clean up any dead code exposed by refactoring
- Ensure zero compiler warnings after removal

### 4. Quality Assurance
- Maintain existing test coverage percentage
- Verify transaction atomicity preserved
- Ensure retry logic continues to work
- Document all changes and rationale

---

## Out of Scope ❌

### 1. Connection Pool Architecture Changes
- Connection pool size tuning (separate optimization)
- Min/max connection configuration
- Pool monitoring infrastructure
- Dedicated writer task architecture (rejected approach)

### 2. SQLite Configuration Changes
- WAL checkpoint tuning
- Busy timeout adjustments
- Journal mode changes
- PRAGMA optimization

### 3. Other Microservices
- wkmp-ap (Audio Player) - no changes
- wkmp-ui (User Interface) - no changes
- wkmp-pd (Program Director) - no changes
- wkmp-le (Lyric Editor) - no changes
- wkmp-dr (Database Review) - no changes
- wkmp-common library - minimal changes if needed

### 4. New Features
- No new functionality added
- No API changes (internal optimization only)
- No UI changes
- No new configuration settings

### 5. Performance Optimization Beyond Batch Writes
- CPU profiling and optimization
- Memory usage optimization
- Network I/O optimization
- Algorithm complexity improvements

---

## Assumptions

### Technical Assumptions

1. **Current Bottleneck:** Database write contention is the primary performance bottleneck
   - Evidence: Git history shows lock contention issues
   - Evidence: 70% write operations in import pipeline
   - Evidence: 16 workers competing for single write slot

2. **Pattern Effectiveness:** passage_recorder.rs pattern (lines 103-145) demonstrates effective batching
   - Pre-fetch reads outside transaction
   - Batch writes within single transaction
   - Target: <100ms transaction hold time

3. **Dead Code Detection:** cargo clippy accurately identifies unused code
   - --all-targets flag catches all unused warnings
   - Manual review may find additional dead code
   - Some "dead" code may be kept intentionally (documented)

4. **Test Coverage:** Existing test suite adequately covers functionality
   - Tests will detect regression
   - No new tests needed (optimization, not new features)
   - Coverage percentage measurable via cargo tarpaulin

5. **SQLite WAL Mode:** Database operates in WAL mode
   - 1 writer at a time (fundamental limitation)
   - Multiple concurrent readers allowed
   - Batch writes work within this constraint

### Process Assumptions

1. **Incremental Implementation:** Changes made in small, testable increments
2. **Test-First Verification:** All tests must pass after each increment
3. **No Breaking Changes:** Public APIs remain unchanged
4. **Documentation:** All significant changes documented with rationale

### Organizational Assumptions

1. **User Availability:** User available to approve scope and review issues
2. **Time Availability:** Sufficient time for careful, incremental implementation
3. **Testing Resources:** Can run full test suite multiple times
4. **No Urgent Deadlines:** Quality over speed

---

## Constraints

### Technical Constraints

1. **SQLite WAL Limitation:** Only 1 concurrent writer
   - Cannot parallelize writes beyond batching
   - Must work within this architectural constraint

2. **Rust Async Patterns:** Must use tokio async/await correctly
   - Transactions must complete within async context
   - retry_on_lock uses async closures

3. **Existing Architecture:** Must preserve microservices architecture
   - HTTP APIs unchanged
   - Event bus communication unchanged
   - Database schema unchanged

4. **Transaction Atomicity:** All-or-nothing semantics required
   - Cannot partially commit batches
   - Rollback must work correctly

### Quality Constraints

1. **Zero Compiler Warnings:** After each dead code removal phase
   - cargo build --all-targets must succeed cleanly
   - cargo clippy --all-targets must report zero warnings

2. **Test Coverage:** No regression from baseline
   - All existing tests must pass
   - Coverage percentage must not decrease

3. **Performance:** Import throughput should improve
   - Measurable reduction in lock acquisitions
   - Desirable: 20%+ throughput increase

### Process Constraints

1. **Incremental Changes:** Small, verifiable steps
   - Each increment must be testable
   - No "big bang" refactoring

2. **Documentation:** Changes must be documented
   - Rationale for batch write consolidations
   - Explanation of dead code removal

3. **User Approval:** Checkpoints for user review
   - After Phase 2 (specification issues)
   - After Phase 3 (test coverage)
   - Before final implementation

---

## Risk Factors

### High Risk

1. **Transaction Atomicity Violation:** Batching breaks existing commit semantics
   - Mitigation: Careful review of transaction boundaries
   - Mitigation: Comprehensive test coverage

2. **Dead Code Removal Breaks Production:** Code thought unused is actually needed
   - Mitigation: Run full test suite after removal
   - Mitigation: Check for #[cfg(test)] and conditional compilation
   - Mitigation: Review for external tool dependencies

### Medium Risk

1. **Performance Regression:** Batching introduces unexpected overhead
   - Mitigation: Benchmark before/after
   - Mitigation: Profile transaction execution time

2. **Lock Contention Remains:** Batching doesn't solve root cause
   - Mitigation: Measure lock acquisitions before/after
   - Mitigation: Have rollback plan if ineffective

### Low Risk

1. **Test Coverage Regression:** Removing dead code reduces coverage %
   - Mitigation: Measure coverage before/after
   - Mitigation: Dead code removal should increase coverage

---

## Success Criteria

### Quantitative Metrics

1. **Lock Acquisitions Reduced:** 10-20 per file → 1-2 per file (80-90% reduction)
2. **Compiler Warnings:** Current → 0 (after each dead code removal phase)
3. **Test Coverage:** Baseline % → Same or higher
4. **Transaction Count:** Reduced by 80%+ per file

### Qualitative Criteria

1. **Code Quality:** Codebase easier to understand (dead code removed)
2. **Maintainability:** Import pipeline simpler to modify
3. **Reliability:** No new bugs introduced
4. **Documentation:** Changes well-documented with rationale

### Verification Methods

1. **Lock Acquisitions:** Monitor transaction logs, count BEGIN/COMMIT
2. **Compiler Warnings:** cargo build and cargo clippy output
3. **Test Coverage:** cargo tarpaulin or similar coverage tool
4. **Performance:** Benchmark 100-file import before/after

---

## Dependencies

### Internal Dependencies (Existing Code)

| Component | Location | Purpose | Status |
|-----------|----------|---------|--------|
| retry_on_lock | wkmp-ai/src/utils/db_retry.rs | Retry logic for lock errors | Exists, preserve |
| begin_monitored | wkmp-ai/src/utils/pool_monitor.rs | Transaction monitoring | Exists, use |
| passage_recorder | wkmp-ai/src/services/passage_recorder.rs | Reference pattern | Exists, replicate |
| Database helpers | wkmp-ai/src/db/*.rs | Save/update functions | Exists, may modify |

### External Dependencies

| Dependency | Version | Purpose | Status |
|------------|---------|---------|--------|
| sqlx | Current | Database access | Stable |
| tokio | Current | Async runtime | Stable |
| parking_lot | Current | RwLock for statistics | Stable |

### Configuration Dependencies

| Setting | Location | Purpose | Status |
|---------|----------|---------|--------|
| ai_database_max_lock_wait_ms | settings table | Retry timeout | Exists, preserve |
| ingest_max_concurrent_jobs | settings table | Worker count | Exists, unchanged |

---

## Timeline Estimate (Rough)

**Phase 1: Dead Code Removal (Pre-Implementation)**
- Analysis: 1-2 hours
- Removal: 2-4 hours
- Testing: 1-2 hours
- **Subtotal:** 4-8 hours

**Phase 2: Batch Writes Optimization**
- Analysis: 2-3 hours
- Implementation: 6-10 hours (incremental)
- Testing: 2-4 hours
- **Subtotal:** 10-17 hours

**Phase 3: Dead Code Removal (Post-Implementation)**
- Analysis: 1 hour
- Removal: 1-2 hours
- Testing: 1 hour
- **Subtotal:** 3-4 hours

**Phase 4: Verification and Documentation**
- Benchmarking: 1-2 hours
- Documentation: 2-3 hours
- **Subtotal:** 3-5 hours

**Total Estimated Effort:** 20-34 hours

**Timeline:** 3-5 days (assuming 6-8 hour work days)

---

## Approval and Sign-Off

**Scope Defined:** 2025-01-15
**Status:** Ready for Phase 2 (Specification Verification)

**Next Actions:**
1. User reviews and approves scope
2. Proceed to Phase 2: Specification Completeness Verification
3. Identify any gaps in requirements

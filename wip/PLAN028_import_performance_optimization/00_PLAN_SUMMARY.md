# PLAN028: Import Performance Optimization - In-Memory Progress & Write Queue

**Status:** Ready for Implementation
**Created:** 2025-11-16
**Specification Source:** wip/PERF002_comprehensive_import_performance_fix.md
**Plan Location:** `wip/PLAN028_import_performance_optimization/`

---

## Executive Summary

The wkmp-ai import process suffers from severe performance bottlenecks due to excessive database I/O. Currently, every file processed triggers a database write and SSE broadcast (5,736+ operations for a typical import). This plan implements three architectural fixes to reduce database writes by 50-100x while maintaining real-time progress updates.

### Problems Being Solved
- **5,736 database writes** per import causing SQLite contention
- **63-second stalls** from write lock contention
- **Pool exhaustion** and cascading timeouts
- **Synchronous blocking** of processing pipeline

### Solution Approach
1. **In-memory progress tracking** with periodic sync (10-second intervals)
2. **Decoupled SSE broadcasting** for real-time updates without database I/O
3. **Write queue manager** to serialize database operations through single writer

### Expected Impact
- **50-100x reduction** in database write operations
- **Elimination** of timeout failures
- **3-5x throughput improvement**
- **Smooth real-time progress** without stalls

---

## Requirements Summary

**Total Requirements:** 8 (5 HIGH, 3 MEDIUM)

| ID | Description | Priority |
|----|-------------|----------|
| REQ-PERF-001 | In-memory progress tracking with periodic sync | HIGH |
| REQ-PERF-002 | Decouple SSE from database operations | HIGH |
| REQ-PERF-003 | Write-queue manager for serialization | HIGH |
| REQ-PERF-004 | Reduce writes from O(files) to O(1) | HIGH |
| REQ-PERF-005 | Maintain <10 second data loss window | MEDIUM |
| REQ-PERF-006 | Support real-time progress updates via SSE | HIGH |
| REQ-PERF-007 | Batch database operations for passages | MEDIUM |
| REQ-PERF-008 | Single writer thread for SQLite | HIGH |

---

## Implementation Roadmap

### Increment 1: ProgressManager Core (4-6 hours)
**Objective:** Create in-memory state management with RwLock for progress tracking.
**Deliverables:**
- `progress_manager.rs` with ProgressManager struct
- In-memory state management
- Update methods for progress and statistics
- Unit tests TC-U-001-01, TC-U-001-02
**Success Criteria:** Can store and retrieve progress without database

### Increment 2: Background Sync Task (2-3 hours)
**Objective:** Implement 10-second periodic sync to database.
**Deliverables:**
- Spawn background sync task
- 10-second interval timer
- Sync to database when dirty
- Unit tests TC-U-001-03, TC-U-001-04
**Success Criteria:** Automatic sync every 10 seconds when dirty

### Increment 3: SSE Decoupling (2-3 hours)
**Objective:** Enable real-time broadcasts without database dependency.
**Deliverables:**
- Immediate SSE broadcast on update
- Remove database dependency from broadcast path
- Unit tests TC-U-002-01, TC-U-002-02
**Success Criteria:** SSE events sent without database I/O

### Increment 4: WriteQueue Implementation (4-5 hours)
**Objective:** Build serialized write queue with single executor.
**Deliverables:**
- `write_queue.rs` with WriteQueue struct
- Bounded queue (1000 items) with backpressure
- Single writer task
- Unit tests TC-U-003-01, TC-U-003-02, TC-U-003-03
**Success Criteria:** All writes serialized through single thread

### Increment 5: Batch Passage Recording (2-3 hours)
**Objective:** Convert individual inserts to batch transactions.
**Deliverables:**
- Update `passage_recorder.rs` for batch operations
- Transaction-based batch inserts
- Integration test TC-I-002-01
**Success Criteria:** Multiple passages in single transaction

### Increment 6: Integration & Testing (3-4 hours)
**Objective:** Wire components together and verify performance.
**Deliverables:**
- Update `workflow_orchestrator/mod.rs`
- Integration tests TC-I-001-01, TC-I-003-01
- Performance verification
**Success Criteria:** 50-100x reduction in database writes

**Total Estimated Effort:** 17-24 hours

---

## Test Coverage

**Total Tests:** 12 (9 unit, 3 integration)
**Coverage:** 100% - All requirements have acceptance tests

Key test scenarios:
- In-memory state management
- Periodic sync behavior
- SSE broadcast timing
- Write queue serialization
- Batch operation correctness
- End-to-end performance verification

---

## Success Metrics

**Quantitative:**
- ✅ Database writes reduced from 5,736 to <60 per import
- ✅ No connection pool timeouts
- ✅ Progress updates every 1-2 seconds
- ✅ <10 second data loss window on crash

**Qualitative:**
- ✅ Smooth progress bar movement
- ✅ No UI freezes or stalls
- ✅ Predictable import completion time

---

## Technical Approach

**Architecture:** Hybrid Sync/Async
- RwLock for in-memory state (synchronous, simple)
- Async for I/O operations (database, SSE)
- Single writer thread for database operations
- Bounded queue to prevent memory issues

**Key Design Decisions:**
- **10-second sync interval:** Balances durability vs performance
- **1000-item queue limit:** Prevents unbounded memory growth
- **Single writer thread:** Respects SQLite single-writer architecture
- **RwLock over async Mutex:** Lower overhead for in-memory state

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation | Residual |
|------|------------|--------|------------|----------|
| Data loss on crash | Medium | Medium | 10-second sync interval | Low |
| Write queue overflow | Low | High | Bounded queue with backpressure | Low |
| Sync task failure | Medium | Low | Log and retry, continue operation | Low |
| Performance regression | Low | High | Benchmark before/after | Low |

**Overall Residual Risk:** LOW

---

## Next Steps

### Immediate Actions
1. Review this plan summary
2. Create new branch: `perf/import-optimization`
3. Begin Increment 1: ProgressManager implementation

### Implementation Sequence
1. ProgressManager core → 2. Sync task → 3. SSE decoupling → 4. WriteQueue → 5. Batch ops → 6. Integration

### Testing Strategy
- Unit test each component in isolation
- Integration test after Increment 5
- Performance benchmark after Increment 6
- Use test import of 100 files for verification
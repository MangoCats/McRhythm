# PERF001: Import Performance Optimization Specification

**Status:** DRAFT
**Created:** 2025-11-16
**Author:** Claude (AI Agent Analysis)
**Context:** testQ.log analysis (build ba0949a5) - import session failed after 3.4 minutes

---

## Executive Summary

The wkmp-ai import pipeline exhibits severe performance degradation due to **architectural mismatch** between SQLite's single-writer design and the application's 21-concurrent-writer configuration. Research and log analysis reveal:

1. **Root Cause:** SQLite WAL mode allows only ONE writer at a time, yet the application spawns 21 concurrent async tasks attempting simultaneous database writes
2. **Symptom:** 63-second stall at 14:06:04-14:07:07 with pool exhaustion and cascading timeouts
3. **Impact:** 96-connection pool provides ZERO write concurrency benefit; large pool increases lock contention overhead
4. **Resolution:** Reduce pool from 96→10 connections, increase busy_timeout from 250ms→10s, limit concurrent writers to 2-4

**Time Budget:** 8-16 hours implementation + 4 hours testing
**Risk Level:** LOW (research-backed, authoritative guidance)
**Expected Improvement:** 3-5x throughput increase, elimination of timeout failures

---

## Problem Analysis

### Observed Symptoms (testQ.log)

| Issue | Evidence | Impact |
|-------|----------|--------|
| **Pool Timeout** | `14:06:42` - "pool timed out while waiting for an open connection" | 1 file failure |
| **Session Save Timeout** | `14:07:34` - "Database locked after 9 attempts (5658 ms)" | Complete import failure |
| **Long Transactions** | `11710ms` held by `passage_recorder::record` | Pool saturation |
| **63-Second Stall** | No activity `14:06:04` → `14:07:07` | Gridlock situation |
| **Slow Acquisitions** | `6.69 seconds` to acquire connection | Write queueing backlog |

### Research Findings

**SQLite Architecture (sqlite.org/wal.html):**
> "Since there is only one WAL file, there can only be one writer at a time."

**Connection Pool Sizing (HikariCP):**
> Formula: `connections = (cores × 2) + spindle_count`
> For 20-core system with SSD: **10-12 connections optimal**

**Busy Timeout Best Practice:**
> "A busy timeout of 5 seconds (5000 milliseconds) is commonly recommended" (sqlite.org)

**Current vs. Optimal Configuration:**

| Parameter | Current | Optimal | Rationale |
|-----------|---------|---------|-----------|
| `pool_size` | 96 | 10 | SQLite single-writer; large pools cause overhead |
| `busy_timeout` | 250ms | 10,000ms | Allow writers to queue through 11s transactions |
| `concurrent_writers` | 21 | 2-4 | Match SQLite's write serialization model |
| `transaction_duration` | 11,710ms | <500ms | Batch commits, reduce lock hold time |

---

## Root Cause: Single-Writer Bottleneck

**Gridlock Scenario (Observed in testQ.log):**

1. **T=14:06:04:** All 21 workers reach Phase 7 (Recording) simultaneously
2. **Worker 1** acquires write lock, holds for 11+ seconds (passage_recorder transaction)
3. **Workers 2-21** block waiting for lock with 250ms `busy_timeout`
4. **Workers 2-21** retry ~40 times (250ms × 40 = 10 seconds), exhausting connection pool
5. **T=14:06:42:** Pool exhausted - new operations timeout waiting for connections
6. **T=14:07:07:** Worker 1 finally commits, releases lock
7. **Cascading failures:** Retry logic exhausted, session save fails

**Bottleneck Visualization:**

```
Time →
14:06:04  ┌─ Worker 1: [===== 11.7s WRITE TRANSACTION =====]
          ├─ Worker 2: [wait][wait][wait][TIMEOUT]
          ├─ Worker 3: [wait][wait][wait][TIMEOUT]
          ⋮  (Workers 4-21 similar)
14:06:42  └─ Pool exhaustion: All connections held by waiters
14:07:07     Worker 1 commits → Lock released
14:07:34     Import fails: Retry budget exhausted
```

---

## Specification for Fix

### Phase 1: Connection Pool Reconfiguration (IMMEDIATE - 2 hours)

**File:** `wkmp-ai/src/models/bootstrap_config.rs`

**Change 1.1:** Reduce default pool size
```rust
// Line ~97: Change pool size calculation
COALESCE(
    (SELECT value FROM settings WHERE key = 'ai_database_connection_pool_size'),
    '10'  // CHANGED FROM '96'
) as pool_size,
```

**Change 1.2:** Increase busy_timeout
```rust
// Line ~102: Change lock retry timeout
COALESCE(
    (SELECT value FROM settings WHERE key = 'ai_database_lock_retry_ms'),
    '10000'  // CHANGED FROM '250'
) as lock_retry,
```

**Change 1.3:** Update pool size validation
```rust
// Line ~150: Update recommended pool size warning
if pool_size < 8 || pool_size > 16 {
    tracing::warn!(
        "ai_database_connection_pool_size ({}) outside recommended range (8-16). \
        SQLite WAL mode supports one writer at a time; large pools cause overhead.",
        pool_size
    );
}
```

**Expected Impact:**
- ✅ Eliminate pool exhaustion (10 connections sufficient for 2-4 concurrent writers + readers)
- ✅ Allow long transactions to complete (10s busy_timeout > 11s observed transaction time)
- ⚠️ Still susceptible to gridlock if all 21 workers write simultaneously

---

### Phase 2: Write Concurrency Limiting (HIGH PRIORITY - 4 hours)

**File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs`

**Problem:** `ingest_max_concurrent_jobs=12` allows too many concurrent writers for SQLite

**Change 2.1:** Add write semaphore
```rust
// Add to WorkflowOrchestrator struct
pub struct WorkflowOrchestrator {
    db: Pool<Sqlite>,
    event_bus: EventBus,
    max_workers: Arc<RwLock<usize>>,
    worker_activities: Arc<RwLock<HashMap<usize, WorkerActivity>>>,

    // NEW: Limit concurrent database writes
    write_semaphore: Arc<Semaphore>,  // Max 2-4 concurrent writes
}
```

**Change 2.2:** Acquire write permit before Phase 7 (Recording)
```rust
// Line ~2551: Before passage_recorder.record_passages()
tracing::debug!("Acquiring write permit for database recording");
let _write_permit = self.write_semaphore.acquire().await
    .map_err(|e| Error::Internal(format!("Write semaphore error: {}", e)))?;

tracing::debug!("Write permit acquired, proceeding with recording");
let result = passage_recorder.record_passages(file_id, &matches).await?;
// Permit auto-released when _write_permit drops
```

**Change 2.3:** Initialize semaphore
```rust
// In new() constructor
let write_semaphore = Arc::new(Semaphore::new(2)); // Start with 2 concurrent writers
```

**Configuration Setting:**
```sql
INSERT INTO settings (key, value, description) VALUES (
    'ai_max_concurrent_writers',
    '2',
    'Maximum concurrent database write transactions (1-4 recommended for SQLite WAL mode)'
);
```

**Expected Impact:**
- ✅ Eliminate write gridlock (only 2-4 writers compete for lock at once)
- ✅ Smooth write queueing (remaining workers wait for semaphore, not database lock)
- ✅ Maintain read parallelism (reads not affected by write semaphore)
- ⚠️ Slightly increased total import time (sequential writes vs. failed parallel attempts)

---

### Phase 3: Transaction Batching (MEDIUM PRIORITY - 6 hours)

**File:** `wkmp-ai/src/services/passage_recorder.rs`

**Problem:** 11.7-second transactions hold write lock too long

**Current Behavior:**
```rust
// Single transaction for all passages (4-20 passages per file)
let mut tx = begin_monitored(db, "passage_recorder::record").await?;
// ... insert ALL songs ...
// ... insert ALL passages ...
tx.commit().await?; // Held lock for 11.7 seconds
```

**Change 3.1:** Batch commits every N operations
```rust
// Line ~146: Replace single transaction with batched approach
const BATCH_SIZE: usize = 5; // Commit every 5 passages

for chunk in matches_ref.chunks(BATCH_SIZE) {
    let mut tx = begin_monitored(db_ref, "passage_recorder::record_batch").await?;

    // Process chunk (songs + passages)
    // ... (existing logic) ...

    tx.commit().await?;
    tracing::debug!(batch_size = chunk.len(), "Batch committed");

    // Yield to allow other tasks to acquire lock
    tokio::task::yield_now().await;
}
```

**Expected Impact:**
- ✅ Reduce transaction hold time from 11.7s → <500ms per batch
- ✅ Allow interleaved writes (other workers can acquire lock between batches)
- ⚠️ Slightly more complex error handling (partial commits on failure)

---

### Phase 4: Enhanced Metrics (LOW PRIORITY - 4 hours)

**File:** `wkmp-ai/src/utils/pool_monitor.rs`

**Add Metrics:**

```rust
// Track connection pool state
pub struct PoolMetrics {
    pub size: u32,
    pub idle: u32,
    pub in_use: u32,
    pub wait_queue_depth: usize,
}

// Log pool state every 10 seconds
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;
        let state = pool.state();
        tracing::info!(
            pool_size = state.size,
            idle_connections = state.connections,
            "Connection pool utilization"
        );
    }
});
```

**File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs`

**Add Phase Timing:**

```rust
// Wrap each phase with timing
let phase_start = std::time::Instant::now();
// ... execute phase ...
let phase_duration = phase_start.elapsed();

tracing::info!(
    phase = phase_name,
    duration_ms = phase_duration.as_millis(),
    "Phase completed"
);
```

---

## Implementation Plan

### Milestone 1: Emergency Fixes (Day 1 - 6 hours)
- [ ] Phase 1.1: Reduce pool size 96→10
- [ ] Phase 1.2: Increase busy_timeout 250ms→10s
- [ ] **Test:** Run import with 35 files, verify no timeouts
- **Success Criteria:** Zero "pool timed out" or "Database locked" errors

### Milestone 2: Structural Improvements (Day 2 - 8 hours)
- [ ] Phase 2.1: Add write semaphore (limit to 2 writers)
- [ ] Phase 2.2: Wrap Phase 7 with semaphore acquisition
- [ ] Phase 3.1: Implement batched commits in passage_recorder
- [ ] **Test:** Run import with 100 files, measure throughput
- **Success Criteria:** 3-5x throughput improvement, <1% failure rate

### Milestone 3: Observability (Day 3 - 4 hours)
- [ ] Phase 4: Add pool utilization metrics
- [ ] Phase 4: Add per-phase timing logs
- [ ] **Test:** Analyze logs for remaining bottlenecks
- **Success Criteria:** Clear visibility into pipeline phases and resource usage

---

## Testing Strategy

### Test Case 1: Baseline (Current Config)
```
Files: 35 (10,000 Maniacs + 4 Non Blondes)
Expected: Timeout failures after ~3 minutes
Metric: Failure rate, time to completion
```

### Test Case 2: Pool Size Reduction Only
```
Config: pool_size=10, busy_timeout=10000ms
Expected: Improved but may still have gridlock
Metric: Reduction in timeout errors
```

### Test Case 3: Write Semaphore (2 writers)
```
Config: pool_size=10, busy_timeout=10000ms, max_writers=2
Expected: No gridlock, smooth progress
Metric: Zero failures, consistent throughput
```

### Test Case 4: Full Optimization
```
Config: All phases implemented
Files: 500+ (stress test)
Expected: Linear scaling up to hardware limits
Metric: Files/second throughput, CPU utilization
```

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Reduced pool size causes read starvation | LOW | MEDIUM | Pool size 10 supports 2 writers + 8 readers |
| Write semaphore slows throughput | MEDIUM | LOW | SQLite already serializes writes; semaphore just makes it explicit |
| Batched commits complicate error handling | MEDIUM | MEDIUM | Wrap in retry logic, log partial progress |
| Configuration changes break existing installs | LOW | HIGH | Provide migration script, update defaults only |

**Overall Risk Level:** LOW
**Confidence Level:** HIGH (research-backed, authoritative sources)

---

## References

1. **SQLite WAL Mode:** https://sqlite.org/wal.html
2. **HikariCP Pool Sizing:** https://github.com/brettwooldridge/HikariCP/wiki/About-Pool-Sizing
3. **SQLite Busy Timeout:** https://sqlite.org/c3ref/busy_timeout.html
4. **Tokio Cooperative Scheduling:** https://tokio.rs/blog/2020-04-preemption
5. **SQLx Pool Options:** https://docs.rs/sqlx/latest/sqlx/pool/struct.PoolOptions.html

---

## Appendix A: Evidence from testQ.log

**Pool Timeout (14:06:42):**
```
[ERROR] File processing failed [file_index=37]
  error=Database error: pool timed out while waiting for an open connection
```

**Long Transaction (14:07:19):**
```
[WARN] LONG TRANSACTION - Connection held for extended period
  caller="passage_recorder::record" held_ms=11710
```

**Session Save Failure (14:07:34):**
```
[ERROR] Database operation failed: max retry time exceeded
  operation="save_session" attempt=9 elapsed_ms=5658 max_wait_ms=5000
```

**63-Second Stall:**
- Last recording: `14:06:04` (file_id=40d97eb3)
- Next recording: `14:07:07` (file_id=4072851d)
- **Gap: 63 seconds** (all workers blocked)

---

## Appendix B: Configuration Migration

**Database Migration Script:**
```sql
-- Update pool size default
UPDATE settings
SET value = '10'
WHERE key = 'ai_database_connection_pool_size'
  AND value = '96';

-- Update busy timeout
UPDATE settings
SET value = '10000'
WHERE key = 'ai_database_lock_retry_ms'
  AND value = '250';

-- Add new setting
INSERT OR IGNORE INTO settings (key, value, description) VALUES (
    'ai_max_concurrent_writers',
    '2',
    'Maximum concurrent database write transactions (1-4 recommended for SQLite WAL mode)'
);
```

---

**END OF SPECIFICATION**

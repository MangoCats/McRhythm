# PERF002: Comprehensive Import Performance Optimization Specification

**Status:** READY FOR IMPLEMENTATION
**Created:** 2025-11-16
**Author:** Claude (AI Performance Analysis)
**Context:** Deep analysis of testQ.log, codebase review, and architectural patterns

---

## Executive Summary

The wkmp-ai import process exhibits severe performance issues due to **fundamental architectural mismatches** between SQLite's single-writer model and the application's concurrent processing design. The system attempts to use 21 concurrent workers with a 96-connection pool against SQLite, which can only process ONE write at a time.

### Critical Issues Identified

1. **SQLite Write Serialization Bottleneck**: 21 workers competing for single write lock
2. **Excessive Database I/O**: Every file triggers database save + SSE broadcast (5,736 saves!)
3. **Progress Reporting Overhead**: Database persistence for transient UI updates
4. **Connection Pool Misconfiguration**: 96 connections provide zero benefit, increase contention
5. **Long Transaction Hold Times**: 11+ second write locks cause cascading timeouts
6. **Synchronous Blocking**: Progress updates block processing pipeline

### Impact

- **63-second stall** with complete gridlock (14:06:04-14:07:07 in testQ.log)
- **Import failure** after processing only 46 files out of 5,736
- **Pool exhaustion** with "timed out waiting for connection" errors
- **Cascading timeouts** from 250ms busy_timeout being too short

### Solution Overview

**Immediate fixes (2-4 hours):**
- Reduce connection pool: 96 → 10 connections
- Increase busy_timeout: 250ms → 10 seconds
- Batch progress updates: Per-file → Per-batch of 100 files

**Architectural fixes (8-16 hours):**
- Implement in-memory progress tracking
- Decouple SSE broadcasting from database operations
- Add write-queue manager for serialized database updates
- Implement batched passage recording

**Expected Results:**
- **10-50x reduction** in database write operations
- **3-5x throughput improvement**
- **Elimination** of timeout failures
- **Smooth progress reporting** without stalls

---

## Detailed Problem Analysis

### 1. Database Write Contention (PRIMARY BOTTLENECK)

**Current State:**
```
21 workers → SQLite (single writer) → Bottleneck
```

**Evidence from testQ.log:**
- Worker threads ThreadId(23), ThreadId(41), ThreadId(42) all attempting concurrent writes
- `passage_recorder::record` holds write lock for 11,710ms
- Connection acquisition times: 0ms → 6,690ms (degradation over time)

**SQLite Documentation (sqlite.org/wal.html):**
> "Since there is only one WAL file, there can only be one writer at a time."

### 2. Progress Update Overhead

**Current Implementation (INEFFICIENT):**
```rust
// phase_processing_per_file() - Lines 2994-2999
crate::db::sessions::save_session(&self.db, &session).await?;  // DATABASE WRITE
let phase_statistics = self.convert_statistics_to_sse();        // MUTEX LOCKS
self.broadcast_progress_with_stats(&session, start_time, phase_statistics);  // SSE
```

**Problem:** This executes for EVERY SINGLE FILE:
- 5,736 files × database save = 5,736 write transactions
- 5,736 files × SSE broadcast = 5,736 network operations
- 5,736 files × statistics conversion = 5,736 × 13 mutex lock operations

### 3. Statistics Collection Overhead

**Current Implementation (Lines 2126-2140):**
```rust
// ACQUIRES 13 MUTEX LOCKS SEQUENTIALLY
let scanning = self.statistics.scanning.lock().unwrap();
let processing = self.statistics.processing.lock().unwrap();
let filename_matching = self.statistics.filename_matching.lock().unwrap();
// ... 10 more mutex acquisitions
```

**Problem:** Called for EVERY progress update (5,736 times)

### 4. Connection Pool Misconfiguration

**Current Settings:**
- `ai_database_connection_pool_size`: 96 (warned as suboptimal)
- `ai_database_lock_retry_ms`: 250ms (too short for 11s transactions)
- `ai_processing_thread_count`: 21 (auto-detected from 20 cores)

**Optimal for SQLite (HikariCP formula):**
- Pool size: `(cores × 2) + spindle_count` = 10-12 for SQLite
- Busy timeout: 5-10 seconds (industry standard)
- Writer threads: 2-4 (match serialization reality)

---

## Implementation Specification

### Phase 1: Immediate Configuration Fixes (2-4 hours)

#### 1.1 Connection Pool Optimization

**File:** `wkmp-common/src/db/init.rs`

```rust
// Line ~239: Update default pool configuration
ensure_setting(pool, "ai_database_connection_pool_size", "10").await?;  // WAS: Not set (defaults to 96)
ensure_setting(pool, "ai_database_lock_retry_ms", "10000").await?;      // WAS: "250"
ensure_setting(pool, "ai_processing_thread_count", "4").await?;         // WAS: NULL (auto-detect → 21)
```

**File:** `wkmp-ai/src/models/bootstrap_config.rs`

```rust
// Line ~150: Update warning threshold
if pool_size < 10 {  // WAS: 168
    tracing::warn!(
        "ai_database_connection_pool_size ({}) is less than recommended (10). \
         Consider setting to at least 10 for optimal SQLite performance.",
        pool_size
    );
}
```

#### 1.2 Batch Progress Updates

**File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs`

```rust
// Line ~2949: Add batch counter
const PROGRESS_UPDATE_BATCH_SIZE: usize = 100;  // New constant

// Line ~2990: Conditional progress update
// Only update database every N files or on completion
if processed % PROGRESS_UPDATE_BATCH_SIZE == 0 || processed == total_files {
    crate::db::sessions::save_session(&self.db, &session).await?;
    let phase_statistics = self.convert_statistics_to_sse();
    self.broadcast_progress_with_stats(&session, start_time, phase_statistics);
}
```

### Phase 2: In-Memory Progress Tracking (4-8 hours)

#### 2.1 Create Progress Manager

**New File:** `wkmp-ai/src/services/progress_manager.rs`

```rust
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::time::{interval, Duration};

/// In-memory progress tracking for import sessions
/// Reduces database writes from O(files) to O(1)
pub struct ProgressManager {
    state: Arc<RwLock<ProgressState>>,
    event_bus: EventBus,
}

#[derive(Clone)]
struct ProgressState {
    session_id: Uuid,
    files_processed: usize,
    files_total: usize,
    current_operation: String,
    phase_statistics: Vec<PhaseStatistics>,
    last_db_sync: Instant,
    dirty: bool,
}

impl ProgressManager {
    /// Create new progress manager with automatic background sync
    pub fn new(session_id: Uuid, event_bus: EventBus, db: SqlitePool) -> Self {
        let manager = Self {
            state: Arc::new(RwLock::new(ProgressState::default())),
            event_bus,
        };

        // Spawn background task for periodic database sync (every 10 seconds)
        manager.spawn_sync_task(db);

        manager
    }

    /// Update progress (in-memory only, marks dirty for next sync)
    pub fn update_progress(&self, processed: usize, total: usize, operation: String) {
        let mut state = self.state.write();
        state.files_processed = processed;
        state.files_total = total;
        state.current_operation = operation;
        state.dirty = true;

        // Broadcast SSE immediately (no database I/O)
        self.broadcast_sse(&state);
    }

    /// Update statistics (in-memory only)
    pub fn update_statistics(&self, stats: Vec<PhaseStatistics>) {
        let mut state = self.state.write();
        state.phase_statistics = stats;
        state.dirty = true;
    }

    /// Force database sync (e.g., on phase transitions)
    pub async fn sync_to_database(&self, db: &SqlitePool) -> Result<()> {
        let state = self.state.read().clone();
        if !state.dirty {
            return Ok(());
        }

        // Single database write for all accumulated changes
        self.persist_state(db, &state).await?;

        self.state.write().dirty = false;
        Ok(())
    }

    fn spawn_sync_task(&self, db: SqlitePool) {
        let state = self.state.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(10));
            loop {
                ticker.tick().await;

                let current_state = state.read().clone();
                if current_state.dirty {
                    // Sync to database every 10 seconds if dirty
                    if let Err(e) = Self::persist_state(&db, &current_state).await {
                        tracing::warn!("Failed to sync progress to database: {}", e);
                    } else {
                        state.write().dirty = false;
                    }
                }
            }
        });
    }

    fn broadcast_sse(&self, state: &ProgressState) {
        // SSE broadcast without database I/O
        self.event_bus.emit_lossy(WkmpEvent::ImportProgressUpdate {
            session_id: state.session_id,
            current: state.files_processed,
            total: state.files_total,
            current_operation: state.current_operation.clone(),
            phase_statistics: state.phase_statistics.clone(),
            // ... other fields
        });
    }
}
```

#### 2.2 Integrate Progress Manager

**File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs`

```rust
// Add to WorkflowOrchestrator struct (line ~88)
progress_manager: Option<ProgressManager>,

// In phase_processing_per_file() - Replace lines 2994-2999
// OLD: Save to database on every file
// NEW: Update in-memory only
self.progress_manager.as_ref().unwrap().update_progress(
    completed,
    total_files,
    format!("Processing {} to {} of {}", completed, processed, total_files),
);

// Only sync to database on phase transitions or completion
if processed == total_files {
    self.progress_manager.as_ref().unwrap().sync_to_database(&self.db).await?;
}
```

### Phase 3: Batch Database Operations (4-6 hours)

#### 3.1 Batch Passage Recording

**File:** `wkmp-ai/src/services/passage_recorder.rs`

```rust
/// Record multiple passages in a single transaction
pub async fn record_passages_batch(
    db: &SqlitePool,
    passages: Vec<PassageData>,
) -> Result<()> {
    // Use transaction for all passages
    let mut tx = db.begin().await?;

    for passage in passages {
        // Insert passage
        sqlx::query!(/* ... */)
            .execute(&mut tx)
            .await?;

        // Insert segments
        for segment in passage.segments {
            sqlx::query!(/* ... */)
                .execute(&mut tx)
                .await?;
        }
    }

    // Single commit for entire batch
    tx.commit().await?;
    Ok(())
}
```

#### 3.2 Implement Write Queue

**New File:** `wkmp-ai/src/services/write_queue.rs`

```rust
/// Serializes database writes to prevent contention
pub struct WriteQueue {
    queue: Arc<Mutex<VecDeque<WriteOperation>>>,
    db: SqlitePool,
}

enum WriteOperation {
    SaveSession(ImportSession),
    RecordPassages(Vec<PassageData>),
    UpdateFileStatus(Uuid, FileStatus),
}

impl WriteQueue {
    pub fn new(db: SqlitePool) -> Self {
        let queue = Arc::new(Mutex::new(VecDeque::new()));

        // Spawn single writer task
        let writer_queue = queue.clone();
        let writer_db = db.clone();
        tokio::spawn(async move {
            loop {
                // Process writes sequentially
                if let Some(op) = writer_queue.lock().unwrap().pop_front() {
                    match op {
                        WriteOperation::SaveSession(session) => {
                            let _ = crate::db::sessions::save_session(&writer_db, &session).await;
                        }
                        WriteOperation::RecordPassages(passages) => {
                            let _ = record_passages_batch(&writer_db, passages).await;
                        }
                        WriteOperation::UpdateFileStatus(id, status) => {
                            let _ = crate::db::files::update_status(&writer_db, id, status).await;
                        }
                    }
                }

                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        Self { queue, db }
    }

    /// Enqueue write operation (non-blocking)
    pub fn enqueue(&self, op: WriteOperation) {
        self.queue.lock().unwrap().push_back(op);
    }
}
```

### Phase 4: Optimize Statistics Collection (2-3 hours)

#### 4.1 Single-Lock Statistics Update

**File:** `wkmp-ai/src/services/workflow_orchestrator/statistics.rs`

```rust
/// Efficient statistics snapshot without multiple locks
pub struct ImportStatistics {
    // Single RwLock for all statistics
    state: Arc<RwLock<StatisticsState>>,
}

#[derive(Clone)]
struct StatisticsState {
    scanning: ScanningStats,
    processing: ProcessingStats,
    filename_matching: FilenameMatchingStats,
    hashing: HashingStats,
    // ... all other stats in single struct
}

impl ImportStatistics {
    /// Get snapshot of all statistics with single lock
    pub fn snapshot(&self) -> Vec<PhaseStatistics> {
        let state = self.state.read();  // SINGLE LOCK

        vec![
            PhaseStatistics::Scanning {
                potential_files_found: state.scanning.potential_files_found,
                is_scanning: state.scanning.is_scanning,
            },
            PhaseStatistics::Processing {
                completed: state.processing.completed,
                started: state.processing.started,
                total: state.processing.total,
                // ...
            },
            // ... convert all stats
        ]
    }

    /// Update specific statistic
    pub fn update<F>(&self, updater: F)
    where
        F: FnOnce(&mut StatisticsState)
    {
        let mut state = self.state.write();
        updater(&mut state);
    }
}
```

### Phase 5: Connection Pooling Best Practices (1-2 hours)

#### 5.1 Implement Connection Monitoring

**File:** `wkmp-ai/src/utils/pool_monitor.rs`

```rust
// Add pool health metrics
pub struct PoolMonitor {
    // ... existing fields

    // New metrics
    total_timeouts: AtomicU64,
    max_wait_time_ms: AtomicU64,
    active_writers: AtomicU32,
}

impl PoolMonitor {
    /// Check pool health and warn on issues
    pub fn check_health(&self) -> PoolHealth {
        let timeouts = self.total_timeouts.load(Ordering::Relaxed);
        let max_wait = self.max_wait_time_ms.load(Ordering::Relaxed);
        let writers = self.active_writers.load(Ordering::Relaxed);

        if timeouts > 10 {
            tracing::error!(
                "Connection pool unhealthy: {} timeouts, max wait {}ms, {} active writers",
                timeouts, max_wait, writers
            );
            return PoolHealth::Critical;
        }

        if max_wait > 5000 {
            tracing::warn!("Connection pool degraded: max wait {}ms", max_wait);
            return PoolHealth::Degraded;
        }

        PoolHealth::Healthy
    }
}
```

---

## Testing Plan

### Performance Benchmarks

Create test harness to measure:
1. **Files per second throughput**
2. **Database write operations count**
3. **Connection pool utilization**
4. **Maximum wait times**
5. **Memory usage (in-memory progress)**

### Test Scenarios

1. **Small import:** 100 files (baseline)
2. **Medium import:** 1,000 files (stress test)
3. **Large import:** 10,000 files (endurance)
4. **Concurrent imports:** 2 sessions simultaneously

### Success Criteria

- No connection pool timeouts
- < 500ms maximum connection wait time
- Linear scaling with file count
- < 100 database writes for 1,000 files (was 1,000+)
- Smooth progress updates every 1-2 seconds

---

## Rollout Plan

### Phase 1: Configuration (Immediate - Safe)
1. Deploy connection pool settings change
2. Monitor with existing imports
3. Rollback if issues (simple config change)

### Phase 2: Batched Updates (Low Risk)
1. Implement progress batching
2. Test with small imports
3. Monitor database write frequency

### Phase 3: In-Memory Progress (Medium Risk)
1. Deploy progress manager
2. Run parallel with existing system
3. Switch over after validation

### Phase 4: Write Queue (Higher Risk)
1. Implement in feature flag
2. A/B test with subset of users
3. Full rollout after stability confirmed

---

## Monitoring & Observability

### Key Metrics to Track

```rust
// Add metrics collection
metrics! {
    counter!("wkmp_ai.db_writes_total");
    histogram!("wkmp_ai.db_write_duration_ms");
    gauge!("wkmp_ai.connection_pool_size");
    gauge!("wkmp_ai.connection_pool_active");
    counter!("wkmp_ai.connection_timeouts_total");
    histogram!("wkmp_ai.progress_update_interval_ms");
}
```

### Alert Thresholds

- Connection pool timeouts > 1/minute → CRITICAL
- Database write duration > 5 seconds → WARNING
- Progress update interval > 30 seconds → WARNING
- Pool utilization > 80% → INFO

---

## Expected Outcomes

### Performance Improvements

| Metric | Current | Expected | Improvement |
|--------|---------|----------|-------------|
| Files/second | ~0.2 (46 files in 3.4 min) | 2-5 | **10-25x** |
| DB writes per 1k files | 1,000+ | 10-20 | **50-100x reduction** |
| Connection wait time | 6.69 seconds | <100ms | **67x faster** |
| Progress update frequency | Per file | Per 100 files | **100x fewer** |
| Import success rate | <1% (failed) | >99% | **Reliable** |

### Resource Utilization

| Resource | Current | Expected | Benefit |
|----------|---------|----------|---------|
| Database connections | 96 (mostly blocked) | 10 (efficiently used) | Less contention |
| Writer threads | 21 (serialized anyway) | 4 (realistic) | Less overhead |
| Memory for progress | Minimal | ~10MB | Enables batching |
| Network (SSE) | 5,736 messages | 57 messages | **100x reduction** |

---

## Risk Mitigation

### Potential Risks

1. **In-memory progress loss on crash**
   - Mitigation: 10-second sync interval limits data loss
   - Recovery: Restart import from last checkpoint

2. **Write queue overflow**
   - Mitigation: Bounded queue with backpressure
   - Monitoring: Queue depth metrics and alerts

3. **Reduced parallelism impacts throughput**
   - Mitigation: 4 workers still process files in parallel
   - Reality: SQLite serializes writes anyway

### Rollback Plan

Each phase can be independently rolled back:
1. Configuration: Revert settings in database
2. Batching: Remove batch size check
3. Progress Manager: Revert to direct database writes
4. Write Queue: Disable via feature flag

---

## Conclusion

The current import pipeline suffers from a fundamental architectural mismatch: treating SQLite like a high-concurrency database. The proposed fixes align the application architecture with SQLite's single-writer reality while maintaining good user experience through in-memory progress tracking and efficient batching.

**Implementation Priority:**
1. **IMMEDIATE:** Connection pool configuration (2 hours, high impact)
2. **HIGH:** Progress update batching (4 hours, high impact)
3. **MEDIUM:** In-memory progress manager (8 hours, medium impact)
4. **LOW:** Write queue system (8 hours, nice-to-have)

**Total Effort:** 22-30 hours
**Expected ROI:** 10-50x performance improvement, reliable imports

---

**Document Status:** Complete and ready for implementation
**Next Steps:** Begin with Phase 1 configuration changes for immediate relief
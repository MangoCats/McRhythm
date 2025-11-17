# PLAN029: Critical Performance Bottleneck Fixes - Implementation Handoff

**Priority:** CRITICAL - System currently unusable (0.5 files/minute)
**Created:** 2025-11-16
**For:** Sonnet 4.5 Implementation
**Context:** Post-PLAN028 analysis revealed new bottlenecks

---

## Executive Summary

The PLAN028 implementation successfully reduced database writes from 5,736 to 4 (excellent!) but created critical new bottlenecks:
- **Connection pool too small** (10 connections causing 3-4 second wait times)
- **Memory leak** (2.3GB usage, should be <500MB)
- **Processing speed** (0.5 files/minute, should be 20-50)

This plan provides comprehensive fixes for ALL issues, not just quick patches.

---

## Critical Issues to Fix

| Issue | Current | Target | Impact |
|-------|---------|--------|--------|
| **Memory Usage** | 2.3GB | <500MB | Process exhaustion |
| **Connection Wait** | 3-4 seconds | <100ms | 40x slowdown |
| **Files/minute** | 0.5 | 20-50 | 8-day import! |
| **Pool Size** | 10 | 25-30 | Starvation |
| **Busy Timeout** | 250ms | 5000ms | Excessive retries |

---

## Implementation Tasks

### Task 1: Implement Dual-Pool Architecture (6-8 hours)

**Problem:** SQLite supports concurrent reads but only one writer. Current single pool causes readers to wait for writers.

**Solution:** Separate read and write pools with appropriate sizing.

#### 1.1 Create New Pool Manager

**File:** `wkmp-ai/src/services/pool_manager.rs` (NEW)

```rust
use sqlx::{Pool, Sqlite, SqlitePool};
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::Result;

/// Manages separate read and write pools for optimal SQLite performance
pub struct PoolManager {
    /// Read pool - larger for parallel reads
    read_pool: SqlitePool,
    /// Write pool - smaller, respects single writer
    write_pool: SqlitePool,
    /// Tracks pool statistics
    stats: Arc<RwLock<PoolStats>>,
}

#[derive(Default)]
struct PoolStats {
    read_acquisitions: u64,
    write_acquisitions: u64,
    max_read_wait_ms: u64,
    max_write_wait_ms: u64,
}

impl PoolManager {
    pub async fn new(db_path: &str) -> Result<Self> {
        // Create read pool with more connections
        let read_pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::from_str(db_path)?
                .read_only(true)  // IMPORTANT: Read-only mode
                .busy_timeout(Duration::from_millis(5000))
                .journal_mode(SqliteJournalMode::Wal)
        )
        .max_connections(20)  // 20 read connections
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .build()?;

        // Create write pool with fewer connections
        let write_pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::from_str(db_path)?
                .busy_timeout(Duration::from_millis(5000))
                .journal_mode(SqliteJournalMode::Wal)
        )
        .max_connections(5)  // Only 5 write connections
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(30))  // Longer timeout for writes
        .build()?;

        Ok(Self {
            read_pool,
            write_pool,
            stats: Arc::new(RwLock::new(PoolStats::default())),
        })
    }

    /// Get connection for read operations
    pub async fn read_conn(&self) -> Result<PoolConnection<Sqlite>> {
        let start = Instant::now();
        let conn = self.read_pool.acquire().await?;
        let wait_ms = start.elapsed().as_millis() as u64;

        let mut stats = self.stats.write();
        stats.read_acquisitions += 1;
        stats.max_read_wait_ms = stats.max_read_wait_ms.max(wait_ms);

        if wait_ms > 100 {
            tracing::warn!("Slow read connection acquisition: {}ms", wait_ms);
        }

        Ok(conn)
    }

    /// Get connection for write operations
    pub async fn write_conn(&self) -> Result<PoolConnection<Sqlite>> {
        let start = Instant::now();
        let conn = self.write_pool.acquire().await?;
        let wait_ms = start.elapsed().as_millis() as u64;

        let mut stats = self.stats.write();
        stats.write_acquisitions += 1;
        stats.max_write_wait_ms = stats.max_write_wait_ms.max(wait_ms);

        if wait_ms > 500 {
            tracing::warn!("Slow write connection acquisition: {}ms", wait_ms);
        }

        Ok(conn)
    }

    /// Use for queries that may write (safer default)
    pub async fn conn(&self) -> Result<PoolConnection<Sqlite>> {
        self.write_conn().await
    }

    pub fn log_stats(&self) {
        let stats = self.stats.read();
        tracing::info!(
            "Pool stats - Reads: {} (max wait {}ms), Writes: {} (max wait {}ms)",
            stats.read_acquisitions,
            stats.max_read_wait_ms,
            stats.write_acquisitions,
            stats.max_write_wait_ms
        );
    }
}
```

#### 1.2 Update WorkflowOrchestrator

**File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs`

Replace single pool with PoolManager:

```rust
// Line ~85
pub struct WorkflowOrchestrator {
    // REPLACE: db: SqlitePool,
    pool_manager: Arc<PoolManager>,  // NEW: Dual-pool manager
    // ... rest of fields
}

// Update constructor (~line 140)
impl WorkflowOrchestrator {
    pub async fn new(db_path: &str, /* ... */) -> Result<Self> {
        let pool_manager = Arc::new(PoolManager::new(db_path).await?);

        Ok(Self {
            pool_manager,
            // ... rest of initialization
        })
    }
}

// Update all database operations to use appropriate pool:
// For SELECT queries - use read pool:
let file = {
    let conn = self.pool_manager.read_conn().await?;
    sqlx::query!("SELECT * FROM files WHERE guid = ?", file_id)
        .fetch_one(conn)
        .await?
};

// For INSERT/UPDATE/DELETE - use write pool:
let result = {
    let conn = self.pool_manager.write_conn().await?;
    sqlx::query!("UPDATE files SET status = ? WHERE guid = ?", status, file_id)
        .execute(conn)
        .await?
};
```

---

### Task 2: Fix Memory Leaks (4-6 hours)

**Problem:** Audio buffers and fingerprint data accumulating in memory (2.3GB!)

#### 2.1 Add Explicit Buffer Cleanup

**File:** `wkmp-ai/src/utils/audio_decoder.rs`

```rust
// Add drop implementations to ensure cleanup
pub struct AudioBuffer {
    samples: Vec<f32>,
    // ... other fields
}

impl AudioBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(capacity),
        }
    }

    /// Clear buffer and release memory
    pub fn clear(&mut self) {
        self.samples.clear();
        self.samples.shrink_to_fit();  // IMPORTANT: Actually release memory
    }
}

impl Drop for AudioBuffer {
    fn drop(&mut self) {
        self.clear();
        tracing::trace!("AudioBuffer dropped, {} samples freed", self.samples.capacity());
    }
}

// In decode_audio_file function:
pub async fn decode_audio_file(path: &Path) -> Result<AudioData> {
    let mut buffer = AudioBuffer::new(estimated_size);

    // ... decoding logic ...

    // IMPORTANT: Process in chunks to avoid holding entire file
    const CHUNK_SIZE: usize = 1024 * 1024;  // 1MB chunks

    while let Some(chunk) = decoder.next_chunk(CHUNK_SIZE)? {
        process_chunk(chunk)?;

        // Yield periodically to prevent blocking
        if processed % (CHUNK_SIZE * 10) == 0 {
            tokio::task::yield_now().await;
        }
    }

    // Ensure buffer is cleared after use
    defer! {
        buffer.clear();
    }

    Ok(result)
}
```

#### 2.2 Add Memory Monitoring

**File:** `wkmp-ai/src/utils/memory_monitor.rs` (NEW)

```rust
use sysinfo::{System, SystemExt, ProcessExt, PidExt};
use std::sync::Arc;
use parking_lot::RwLock;

pub struct MemoryMonitor {
    system: Arc<RwLock<System>>,
    high_water_mark: Arc<AtomicU64>,
    warning_threshold: u64,  // Bytes
}

impl MemoryMonitor {
    pub fn new() -> Self {
        Self {
            system: Arc::new(RwLock::new(System::new())),
            high_water_mark: Arc::new(AtomicU64::new(0)),
            warning_threshold: 500_000_000,  // 500MB
        }
    }

    pub fn check_memory(&self) -> MemoryStatus {
        let mut sys = self.system.write();
        sys.refresh_process(sysinfo::get_current_pid().unwrap());

        let current_pid = sysinfo::get_current_pid().unwrap();
        if let Some(process) = sys.process(current_pid) {
            let memory_bytes = process.memory() * 1024;  // KB to bytes

            // Update high water mark
            self.high_water_mark.fetch_max(memory_bytes, Ordering::Relaxed);

            if memory_bytes > self.warning_threshold * 2 {
                tracing::error!(
                    "CRITICAL: Memory usage {}MB exceeds 2x threshold",
                    memory_bytes / 1_000_000
                );
                return MemoryStatus::Critical(memory_bytes);
            } else if memory_bytes > self.warning_threshold {
                tracing::warn!(
                    "High memory usage: {}MB (threshold {}MB)",
                    memory_bytes / 1_000_000,
                    self.warning_threshold / 1_000_000
                );
                return MemoryStatus::Warning(memory_bytes);
            }

            MemoryStatus::Normal(memory_bytes)
        } else {
            MemoryStatus::Unknown
        }
    }

    pub async fn monitor_task(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            match self.check_memory() {
                MemoryStatus::Critical(bytes) => {
                    // Force garbage collection attempt
                    tracing::error!("Attempting memory recovery, current: {}MB", bytes / 1_000_000);

                    // Clear any caches
                    self.clear_caches().await;

                    // If still critical after cleanup, consider pausing
                    tokio::time::sleep(Duration::from_secs(5)).await;

                    if let MemoryStatus::Critical(_) = self.check_memory() {
                        tracing::error!("Memory still critical after cleanup, pausing operations");
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    }
                }
                MemoryStatus::Warning(bytes) => {
                    tracing::info!("Memory check: {}MB (warning level)", bytes / 1_000_000);
                }
                MemoryStatus::Normal(bytes) => {
                    tracing::debug!("Memory check: {}MB (normal)", bytes / 1_000_000);
                }
                MemoryStatus::Unknown => {
                    tracing::warn!("Unable to check memory usage");
                }
            }
        }
    }

    async fn clear_caches(&self) {
        // Trigger any cache clearing operations
        tracing::info!("Clearing caches to free memory");
        // Implementation depends on what caches exist
    }
}

pub enum MemoryStatus {
    Normal(u64),
    Warning(u64),
    Critical(u64),
    Unknown,
}
```

#### 2.3 Integrate Memory Monitoring

**File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs`

```rust
// Add to WorkflowOrchestrator struct
memory_monitor: Arc<MemoryMonitor>,

// In phase_processing_per_file() - check memory every 10 files
if completed % 10 == 0 {
    if let MemoryStatus::Critical(_) = self.memory_monitor.check_memory() {
        tracing::error!("Pausing processing due to critical memory usage");
        tokio::time::sleep(Duration::from_secs(30)).await;

        // Force cleanup of any accumulated data
        self.cleanup_processing_state().await?;
    }
}

// Add cleanup method
async fn cleanup_processing_state(&self) -> Result<()> {
    // Clear any accumulated state
    // Force drop of temporary data
    // Trigger garbage collection hints
    tracing::info!("Cleaning up processing state to free memory");
    Ok(())
}
```

---

### Task 3: Fix Configuration Settings (2-3 hours)

#### 3.1 Update Default Configuration

**File:** `wkmp-common/src/db/init.rs`

```rust
// Line ~235-250 - Update ALL settings for optimal performance
pub async fn ensure_default_settings(pool: &SqlitePool) -> Result<()> {
    // Connection pool - balanced for read/write operations
    ensure_setting(pool, "ai_database_connection_pool_size", "30").await?;  // Total connections

    // Timeouts - appropriate for SQLite
    ensure_setting(pool, "ai_database_busy_timeout_ms", "5000").await?;     // 5 seconds
    ensure_setting(pool, "ai_database_lock_retry_ms", "5000").await?;       // Match busy timeout
    ensure_setting(pool, "ai_database_max_lock_wait_ms", "30000").await?;   // 30 seconds max

    // Threading - balanced for I/O and CPU
    ensure_setting(pool, "ai_processing_thread_count", "8").await?;         // Not 21!
    ensure_setting(pool, "ai_max_concurrent_operations", "4").await?;       // Limit parallelism

    // Memory limits
    ensure_setting(pool, "ai_max_memory_mb", "500").await?;                 // Enforce limit
    ensure_setting(pool, "ai_audio_buffer_size_mb", "50").await?;          // Per-file limit

    // Progress updates
    ensure_setting(pool, "ai_progress_update_interval_files", "100").await?; // Batch updates
    ensure_setting(pool, "ai_progress_sync_interval_seconds", "10").await?;  // Periodic sync

    Ok(())
}
```

#### 3.2 Update Bootstrap Configuration

**File:** `wkmp-ai/src/models/bootstrap_config.rs`

```rust
// Line ~140-160 - Update validation logic
impl BootstrapConfig {
    pub fn validate(&self) -> Result<()> {
        // Pool size validation - new balanced approach
        if self.pool_size < 20 {
            tracing::warn!(
                "Pool size {} is below recommended minimum (20). \
                 This may cause connection starvation.",
                self.pool_size
            );
        } else if self.pool_size > 50 {
            tracing::warn!(
                "Pool size {} exceeds recommended maximum (50). \
                 This provides no benefit with SQLite.",
                self.pool_size
            );
        }

        // Thread count validation
        let cpu_cores = num_cpus::get();
        let recommended_threads = (cpu_cores / 2).max(4).min(8);

        if self.processing_thread_count > recommended_threads * 2 {
            tracing::warn!(
                "Thread count {} significantly exceeds recommendation ({}). \
                 This will cause contention, not improve performance.",
                self.processing_thread_count,
                recommended_threads
            );
        }

        // Busy timeout validation
        if self.lock_retry_ms < 1000 {
            tracing::error!(
                "lock_retry_ms {} is too short. SQLite operations can take seconds. \
                 Minimum recommended: 1000ms",
                self.lock_retry_ms
            );
        }

        Ok(())
    }
}
```

---

### Task 4: Optimize Worker Coordination (3-4 hours)

#### 4.1 Implement Adaptive Worker Management

**File:** `wkmp-ai/src/services/adaptive_worker_manager.rs` (NEW)

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;
use std::time::Instant;

/// Dynamically adjusts worker count based on performance
pub struct AdaptiveWorkerManager {
    /// Semaphore controlling concurrent operations
    semaphore: Arc<Semaphore>,
    /// Target time per operation
    target_time_ms: u64,
    /// Current performance metrics
    metrics: Arc<RwLock<WorkerMetrics>>,
}

#[derive(Default)]
struct WorkerMetrics {
    operations_completed: u64,
    total_time_ms: u64,
    recent_times: VecDeque<u64>,  // Last 10 operation times
}

impl AdaptiveWorkerManager {
    pub fn new(initial_workers: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(initial_workers)),
            target_time_ms: 2000,  // Target 2 seconds per file
            metrics: Arc::new(RwLock::new(WorkerMetrics::default())),
        }
    }

    /// Acquire permit to process, may wait if at capacity
    pub async fn acquire(&self) -> WorkerPermit {
        let permit = self.semaphore.acquire().await.unwrap();
        WorkerPermit {
            _permit: permit,
            start_time: Instant::now(),
            manager: self.clone(),
        }
    }

    /// Record operation completion and adjust workers
    fn record_completion(&self, duration_ms: u64) {
        let mut metrics = self.metrics.write();

        metrics.operations_completed += 1;
        metrics.total_time_ms += duration_ms;

        // Keep last 10 times for recent performance
        metrics.recent_times.push_back(duration_ms);
        if metrics.recent_times.len() > 10 {
            metrics.recent_times.pop_front();
        }

        // Adjust workers every 10 operations
        if metrics.operations_completed % 10 == 0 {
            self.adjust_workers(&metrics);
        }
    }

    fn adjust_workers(&self, metrics: &WorkerMetrics) {
        if metrics.recent_times.len() < 5 {
            return;  // Not enough data
        }

        let avg_time: u64 = metrics.recent_times.iter().sum::<u64>() / metrics.recent_times.len() as u64;
        let current_permits = self.semaphore.available_permits();

        if avg_time > self.target_time_ms * 2 && current_permits > 2 {
            // Too slow, reduce workers
            tracing::info!(
                "Reducing workers: avg time {}ms exceeds target, reducing from {}",
                avg_time, current_permits
            );
            // Reduce by 1 (by not adding back a permit)
            self.semaphore.forget_permits(1);

        } else if avg_time < self.target_time_ms / 2 && current_permits < 8 {
            // Too fast (might indicate we could do more), add workers
            tracing::info!(
                "Increasing workers: avg time {}ms below target, increasing from {}",
                avg_time, current_permits
            );
            self.semaphore.add_permits(1);
        }
    }
}

pub struct WorkerPermit {
    _permit: SemaphorePermit<'static>,
    start_time: Instant,
    manager: AdaptiveWorkerManager,
}

impl Drop for WorkerPermit {
    fn drop(&mut self) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        self.manager.record_completion(duration_ms);
    }
}
```

#### 4.2 Integrate Adaptive Management

**File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs`

```rust
// Add to WorkflowOrchestrator
worker_manager: Arc<AdaptiveWorkerManager>,

// In phase_processing_per_file()
// Replace fixed parallelism with adaptive:
while let Some(result) = tasks.next().await {
    // Get permit before spawning next task
    let permit = self.worker_manager.acquire().await;

    // Spawn next file processing with permit
    if let Some((idx, (file_id, file_path))) = file_iter.next() {
        let task = self.process_file_with_permit(
            idx,
            file_id,
            file_path,
            permit,  // Pass permit to track completion
        );
        tasks.push(task);
    }
}
```

---

### Task 5: Performance Testing & Validation (2-3 hours)

#### 5.1 Create Performance Benchmark

**File:** `wkmp-ai/tests/performance_benchmark.rs` (NEW)

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_import_performance() {
        // Setup
        let test_dir = setup_test_files(100).await;  // 100 test files
        let orchestrator = WorkflowOrchestrator::new(test_db()).await.unwrap();

        // Measure
        let start = Instant::now();
        let result = orchestrator.import_directory(&test_dir).await.unwrap();
        let duration = start.elapsed();

        // Assert performance targets
        let files_per_second = 100.0 / duration.as_secs_f64();
        assert!(
            files_per_second > 0.3,  // Minimum 0.3 files/second (20 files/minute)
            "Import too slow: {} files/second",
            files_per_second
        );

        // Check memory usage
        let memory = get_process_memory();
        assert!(
            memory < 500_000_000,  // Less than 500MB
            "Memory usage too high: {}MB",
            memory / 1_000_000
        );

        // Check connection pool metrics
        orchestrator.pool_manager.log_stats();

        // Verify all files processed
        assert_eq!(result.files_processed, 100);
        assert_eq!(result.errors.len(), 0);
    }

    #[tokio::test]
    async fn test_connection_pool_performance() {
        let pool_manager = PoolManager::new(test_db()).await.unwrap();

        // Parallel read test
        let mut read_tasks = vec![];
        for _ in 0..20 {
            let pm = pool_manager.clone();
            read_tasks.push(tokio::spawn(async move {
                let start = Instant::now();
                let _conn = pm.read_conn().await.unwrap();
                start.elapsed().as_millis()
            }));
        }

        let read_times: Vec<u128> = futures::future::join_all(read_tasks)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        let avg_read_time = read_times.iter().sum::<u128>() / read_times.len() as u128;
        assert!(
            avg_read_time < 100,
            "Read connection acquisition too slow: {}ms average",
            avg_read_time
        );

        // Serial write test
        let mut write_times = vec![];
        for _ in 0..5 {
            let start = Instant::now();
            let _conn = pool_manager.write_conn().await.unwrap();
            write_times.push(start.elapsed().as_millis());
        }

        let avg_write_time = write_times.iter().sum::<u128>() / write_times.len() as u128;
        assert!(
            avg_write_time < 500,
            "Write connection acquisition too slow: {}ms average",
            avg_write_time
        );
    }
}
```

---

## Implementation Order

### Phase 1: Critical Fixes (Day 1)
1. **Task 3.1**: Update configuration defaults (1 hour)
2. **Task 1.1**: Implement PoolManager (4 hours)
3. **Task 1.2**: Integrate dual pools (2 hours)

### Phase 2: Memory Management (Day 2)
1. **Task 2.1**: Fix buffer cleanup (2 hours)
2. **Task 2.2**: Add memory monitoring (2 hours)
3. **Task 2.3**: Integrate monitoring (1 hour)

### Phase 3: Optimization (Day 2-3)
1. **Task 4.1**: Adaptive worker management (2 hours)
2. **Task 4.2**: Integration (2 hours)
3. **Task 5.1**: Performance testing (2 hours)

**Total Estimated Time:** 20-26 hours

---

## Success Criteria

After implementation, the system MUST achieve:

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| **Memory Usage** | <500MB | Memory monitor logs |
| **Files/minute** | >20 | Import completion time |
| **Connection Acquisition** | <100ms avg | Pool manager stats |
| **Database Writes** | <100 per import | SQL query logging |
| **Import Success Rate** | >99% | Error count / total files |
| **Worker Efficiency** | >80% active | Worker manager metrics |

---

## Testing Checklist

Before declaring complete:

- [ ] Run import with 100 files, verify <5 minutes completion
- [ ] Run import with 1000 files, verify <50 minutes completion
- [ ] Monitor memory usage throughout, verify stays <500MB
- [ ] Check connection pool stats, verify <100ms average acquisition
- [ ] Run concurrent imports (2 sessions), verify both complete
- [ ] Kill process mid-import, verify graceful recovery
- [ ] Check database write count, verify <100 for 1000 files
- [ ] Verify progress updates every 10 seconds in UI

---

## Configuration for Testing

```sql
-- Apply these settings before testing
UPDATE settings SET value = '30' WHERE key = 'ai_database_connection_pool_size';
UPDATE settings SET value = '5000' WHERE key = 'ai_database_busy_timeout_ms';
UPDATE settings SET value = '5000' WHERE key = 'ai_database_lock_retry_ms';
UPDATE settings SET value = '30000' WHERE key = 'ai_database_max_lock_wait_ms';
UPDATE settings SET value = '8' WHERE key = 'ai_processing_thread_count';
UPDATE settings SET value = '4' WHERE key = 'ai_max_concurrent_operations';
UPDATE settings SET value = '500' WHERE key = 'ai_max_memory_mb';
UPDATE settings SET value = '50' WHERE key = 'ai_audio_buffer_size_mb';
UPDATE settings SET value = '100' WHERE key = 'ai_progress_update_interval_files';
UPDATE settings SET value = '10' WHERE key = 'ai_progress_sync_interval_seconds';
```

---

## Rollback Plan

If issues occur after implementation:

1. **Revert to single pool**: Comment out PoolManager, use original SqlitePool
2. **Increase connection limit**: Set pool size to 50 temporarily
3. **Disable memory monitoring**: Comment out memory checks
4. **Restore original workers**: Set thread count back to 21

Each component can be independently disabled for debugging.

---

## Key Implementation Notes

1. **Dual pools are CRITICAL** - SQLite supports many readers, one writer
2. **Memory leaks MUST be fixed** - 2.3GB will crash the process
3. **Don't over-optimize** - 30 connections is plenty, 100 would be wasteful
4. **Monitor everything** - Add logging to track performance
5. **Test incrementally** - Verify each phase before proceeding

---

## Expected Outcomes

With these fixes implemented:

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Import Time (5736 files) | 191 hours | 4-5 hours | **40x faster** |
| Memory Usage | 2.3GB | <500MB | **5x reduction** |
| Connection Wait | 3-4 sec | <100ms | **40x faster** |
| Success Rate | ~50% | >99% | **Reliable** |

---

## Contact for Questions

If any part of this plan is unclear:
1. Check the analysis in `wip/PERF002_comprehensive_import_performance_fix.md`
2. Review PLAN028 for context on what was already implemented
3. Test logs are in `wkmp-ai/testS.log` (before) and `wkmp-ai/testT.log` (after)

**Critical Success Factor:** The dual-pool architecture is the most important fix. Without separating reads from writes, SQLite will always bottleneck.

---

**Ready for Implementation:** This plan provides everything needed to fix the critical performance issues and make the import process production-ready.
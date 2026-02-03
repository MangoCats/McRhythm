# Handoff Package: PLAN028 Implementation for Sonnet 4.5

**Task:** Implement performance optimizations for wkmp-ai import process
**Estimated Effort:** 17-24 hours
**Priority:** HIGH - System currently fails on large imports

---

## Quick Context

**Problem:** The wkmp-ai import process makes 5,736+ database writes for a typical music library import, causing SQLite write lock contention and 63-second stalls. The root cause is attempting concurrent writes against SQLite which only supports ONE writer at a time.

**Solution:** Implement in-memory progress tracking with periodic sync, decouple SSE broadcasting from database operations, and add a write queue to serialize all database operations.

---

## Files to Read (In Order)

1. **This handoff document** - Start here
2. **`00_PLAN_SUMMARY.md`** - Complete implementation plan (400 lines)
3. **`test_specifications.md`** - All test cases (150 lines)
4. **`wkmp-ai/src/services/workflow_orchestrator/mod.rs`** (lines 2860-3050) - Current implementation to modify
5. **`wkmp-common/src/events.rs`** - Event definitions for SSE

---

## Implementation Order

### Increment 1: Create `wkmp-ai/src/services/progress_manager.rs` (4-6 hours)

```rust
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::time::{interval, Duration};
use uuid::Uuid;
use chrono::Utc;
use sqlx::SqlitePool;
use anyhow::Result;

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
```

**Key Requirements:**
- Use `parking_lot::RwLock` (NOT std::sync)
- Mark dirty on ANY update
- Broadcast SSE immediately (no database wait)
- Spawn background task for 10-second sync

### Increment 2: Add Background Sync Task (2-3 hours)

Add to ProgressManager:
```rust
fn spawn_sync_task(&self, db: SqlitePool) {
    let state = self.state.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(10));
        loop {
            ticker.tick().await;
            // Sync if dirty
        }
    });
}
```

### Increment 3: SSE Decoupling (2-3 hours)

Update `broadcast_sse()` to work without database:
- Remove all database operations from SSE path
- Use in-memory state only
- Send immediately on update

### Increment 4: Create `wkmp-ai/src/services/write_queue.rs` (4-5 hours)

```rust
use std::collections::VecDeque;
use tokio::sync::Mutex;

pub struct WriteQueue {
    queue: Arc<Mutex<VecDeque<WriteOperation>>>,
    db: SqlitePool,
}

enum WriteOperation {
    SaveSession(ImportSession),
    RecordPassages(Vec<PassageData>),
    UpdateFileStatus(Uuid, FileStatus),
}
```

**Key Requirements:**
- Bounded queue (1000 items max)
- Single writer task (respects SQLite)
- Process operations sequentially
- Handle backpressure

### Increment 5: Batch Passage Recording (2-3 hours)

Modify `wkmp-ai/src/services/passage_recorder.rs`:
```rust
pub async fn record_passages_batch(
    db: &SqlitePool,
    passages: Vec<PassageData>,
) -> Result<()> {
    let mut tx = db.begin().await?;
    // Insert all passages in single transaction
    tx.commit().await?;
}
```

### Increment 6: Integration (3-4 hours)

Update `workflow_orchestrator/mod.rs` line ~2994:

**OLD:**
```rust
crate::db::sessions::save_session(&self.db, &session).await?;
let phase_statistics = self.convert_statistics_to_sse();
self.broadcast_progress_with_stats(&session, start_time, phase_statistics);
```

**NEW:**
```rust
self.progress_manager.as_ref().unwrap().update_progress(
    completed,
    total_files,
    format!("Processing {} of {}", completed, total_files),
);
// Only sync to database on completion
if processed == total_files {
    self.progress_manager.as_ref().unwrap().sync_to_database(&self.db).await?;
}
```

---

## Critical Implementation Notes

1. **SQLite Single Writer:** The root cause is SQLite can only have ONE writer. All our optimizations work around this limitation.

2. **10-Second Sync:** This interval balances data durability (max 10s loss on crash) with performance (6 syncs per minute vs 5,736).

3. **Bounded Queue:** MUST implement backpressure at 1000 items to prevent memory exhaustion.

4. **Use parking_lot:** More efficient than std::sync for our use case.

5. **Test with Small Dataset:** Use 100 files for testing, not 5,736.

---

## Testing Strategy

Run tests after each increment:

```bash
# After Increment 1
cargo test progress_manager

# After Increment 2
cargo test sync_task

# After Increment 4
cargo test write_queue

# After Increment 6 - Integration test
cargo test --test import_performance_test
```

---

## Success Verification

Before optimization:
- 5,736 database writes for 5,736 files
- 63-second stalls
- Connection pool timeouts

After optimization:
- <60 database writes for 5,736 files (100x reduction)
- No stalls >1 second
- Zero timeouts

---

## Common Pitfalls to Avoid

1. **Don't use std::sync::Mutex** - Use parking_lot::RwLock
2. **Don't forget bounded queue** - Memory will explode otherwise
3. **Don't sync on every update** - That defeats the purpose
4. **Don't use multiple writer threads** - SQLite doesn't support it
5. **Don't skip the tests** - They verify correctness

---

## Questions/Blockers

If you encounter these, ask for clarification:
- Existing EventBus implementation details
- Database schema for sessions table
- Current SSE event structure

---

## Definition of Done

- [ ] All 12 tests pass
- [ ] Database writes reduced by 50-100x
- [ ] No connection timeouts in 1000-file test
- [ ] Progress updates still real-time (<1s delay)
- [ ] Code review complete
- [ ] Performance benchmark documented

---

**Ready to Start:** Begin with Increment 1 - Creating ProgressManager
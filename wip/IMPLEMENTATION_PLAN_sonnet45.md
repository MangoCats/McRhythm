# Implementation Plan for Sonnet 4.5: wkmp-ai Critical Fixes & Improvements

**Created:** 2025-11-16
**Total Scope:** ~90-100 hours (3-4 weeks)
**Priority:** CRITICAL - System currently failing on production workloads

---

## Overview

This plan consolidates three critical work streams:
1. **Performance emergency fixes** (PLAN031) - System taking 3+ hours for imports
2. **Technical debt remediation** (TECH_DEBT_REPORT) - 30+ runtime panic risks
3. **Memory threshold configuration** (HANDOFF) - False warnings on 3.6GB usage

---

## Phase 1: Emergency Stabilization (Day 1-2)
**Goal:** Stop crashes and unblock imports
**Time:** 8-12 hours

### Task 1.1: Fix Compilation Error (30 min) ⚠️ BLOCKER
**File:** `wkmp-ai/src/main.rs:208-212`
**Issue:** AppState constructor missing parameter
**Fix:**
```rust
// Find where AppState::new is called (around line 208)
let app_state = AppState::new(
    db.clone(),
    event_bus.clone(),
    bootstrap_config.processing_thread_count,
    bootstrap_config.memory_usage_threshold_bytes,  // ADD THIS LINE
);
```

### Task 1.2: Kill AcoustID Integration (1 hour) 🔥 CRITICAL
**File:** `wkmp-ai/src/services/acoustid_client.rs`
**Fix:** Add emergency kill switch per PLAN031:
```rust
const ACOUSTID_ENABLED: bool = false; // EMERGENCY KILL SWITCH
const ACOUSTID_TIMEOUT: Duration = Duration::from_secs(1); // Was 17 seconds!

pub async fn lookup(&self, fingerprint: &str) -> Result<Option<String>> {
    if !ACOUSTID_ENABLED {
        return Ok(None);
    }
    // Add timeout wrapper
    match timeout(ACOUSTID_TIMEOUT, self.client.post(URL).send()).await {
        Ok(Ok(response)) => Ok(parse_response(response)),
        _ => Ok(None) // Fail fast
    }
}
```

### Task 1.3: Skip Tiny Passages (<10s) (1 hour)
**Files:**
- `wkmp-ai/src/services/passage_fingerprinter.rs`
- `wkmp-ai/src/services/fingerprinter.rs`
- `wkmp-ai/src/services/passage_amplitude_analyzer.rs`

**Add to passage_fingerprinter.rs:**
```rust
const MIN_PASSAGE_DURATION_FOR_FINGERPRINT: f64 = 10.0;

pub async fn fingerprint_passages(&self, passages: Vec<Passage>) -> Result<Vec<FingerprintResult>> {
    let mut results = Vec::new();
    for passage in passages {
        let duration_seconds = (passage.end_ticks - passage.start_ticks) as f64
                              / TICKS_PER_SECOND as f64;

        if duration_seconds < MIN_PASSAGE_DURATION_FOR_FINGERPRINT {
            info!("Skipping fingerprint for {:.1}s passage (too short)", duration_seconds);
            results.push(FingerprintResult::Skipped {
                reason: "Passage too short",
                duration: duration_seconds,
            });
            continue;
        }
        // Process normally...
    }
    Ok(results)
}
```

### Task 1.4: Fix Pool Configuration (1 hour)
**File:** `wkmp-ai/src/models/bootstrap_config.rs`
**Current:** 96 connections, 250ms timeout
**Target:** 30 connections, 5000ms timeout

```rust
// Line ~35: Update defaults
pub connection_pool_size: u32 = 30,  // Was 96
pub lock_retry_ms: u64 = 5000,       // Was 250

// In create_production_pool():
SqlitePoolOptions::new()
    .max_connections(30)      // HARD-CODED temporarily
    .min_connections(15)
    .acquire_timeout(Duration::from_secs(5))
    .busy_timeout(Duration::from_millis(5000))  // Critical fix
```

### Task 1.5: Reduce Worker Count (30 min)
**File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs`
```rust
const EMERGENCY_WORKER_COUNT: usize = 4; // Was 8+

pub fn start_workers(&self) -> Vec<JoinHandle<()>> {
    (0..EMERGENCY_WORKER_COUNT)
        .map(|_| self.spawn_worker())
        .collect()
}
```

### Task 1.6: Memory Threshold Implementation (2 hours)
**Implement per HANDOFF_memory_threshold_implementation.md**

1. Add to `wkmp-common/src/db/init.rs:260`:
```rust
ensure_setting(pool, "ai_memory_usage_threshold_bytes", "12884901888").await?;
```

2. Update `wkmp-ai/src/models/bootstrap_config.rs`:
   - Add field to struct
   - Add to SQL query
   - Parse and validate value

3. Update `wkmp-ai/src/services/workflow_orchestrator/mod.rs:164`:
```rust
memory_monitor: Arc::new(
    crate::utils::MemoryMonitor::with_threshold(bootstrap_config.memory_usage_threshold_bytes)
),
```

### Task 1.7: Remove All unwrap() Calls (4 hours) 🔥 CRITICAL
**Priority files with most unwraps:**
- `services/fingerprinter.rs:202` - Lock acquisition
- `models/bootstrap_config.rs:281-375` - 29 calls
- `services/acoustid_client.rs:500-567` - 10+ calls
- `fusion/extractors/audio_extractor.rs:243,268`

**Replace pattern:**
```rust
// BAD: Will panic
let _lock = CHROMAPRINT_LOCK.lock().unwrap();

// GOOD: Propagates error
let _lock = CHROMAPRINT_LOCK.lock()
    .map_err(|e| anyhow!("Failed to acquire chromaprint lock: {}", e))?;
```

---

## Phase 2: Memory & Resource Management (Day 3-4)
**Goal:** Fix memory leaks and resource exhaustion
**Time:** 12-16 hours

### Task 2.1: Fix Cancellation Token Memory Leak (2 hours)
**File:** `wkmp-ai/src/lib.rs:41`
**Issue:** HashMap grows unbounded

Add cleanup method:
```rust
impl AppState {
    pub fn cleanup_completed_import(&self, session_id: Uuid) {
        let mut tokens = self.cancellation_tokens.write();
        tokens.remove(&session_id);

        // Also clean up any tokens older than 24 hours
        let cutoff = Utc::now() - Duration::hours(24);
        tokens.retain(|_, token| !token.is_cancelled());
    }
}
```

Call from workflow_orchestrator on import completion.

### Task 2.2: Increase Event Bus Capacity (1 hour)
**File:** `wkmp-ai/src/main.rs:204`
```rust
// Make configurable
let event_capacity = bootstrap_config.event_bus_capacity.unwrap_or(1000);
let event_bus = EventBus::new(event_capacity);
```

### Task 2.3: Batch Database Operations (3 hours)
**File:** `wkmp-ai/src/services/passage_recorder.rs`

Implement from PLAN031:
```rust
pub async fn record_file_complete(&self, file_id: Uuid, passages: Vec<Passage>) -> Result<()> {
    let mut tx = self.db.begin().await?;

    // Batch insert all passages
    for chunk in passages.chunks(100) {
        let values = chunk.iter()
            .map(|p| format!("('{}', '{}', {}, {})", p.id, file_id, p.start, p.end))
            .join(",");

        sqlx::query(&format!("INSERT INTO passages VALUES {}", values))
            .execute(&mut tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}
```

### Task 2.4: Convert to Async File I/O (4 hours)
**Priority files:**
- `main.rs:126` - Config loading
- `services/file_scanner.rs:283-288` - File scanning
- `services/hash_deduplicator.rs:55-69` - Hash computation
- `workflow/boundary_detector.rs:129-134,276-281`

**Pattern:**
```rust
// BAD: Blocks executor
let contents = std::fs::read_to_string(path)?;

// GOOD: Async
let contents = tokio::fs::read_to_string(path).await?;

// GOOD: For CPU-intensive ops
let contents = tokio::task::spawn_blocking(move || {
    std::fs::read_to_string(path)
}).await??;
```

### Task 2.5: Replace Sync Locks with Async (3 hours)
**Files:**
- `workflow_orchestrator/mod.rs:89,92` - Worker state
- `services/fingerprinter.rs:202` - Chromaprint lock

```rust
// BAD: parking_lot::RwLock
Arc<RwLock<HashMap<String, WorkerActivity>>>

// GOOD: tokio::sync::RwLock
Arc<tokio::sync::RwLock<HashMap<String, WorkerActivity>>>
```

---

## Phase 3: Architecture Refactoring (Week 2)
**Goal:** Split monoliths, eliminate duplication
**Time:** 40-50 hours

### Task 3.1: Split 3,382-line Orchestrator (16 hours)
**File:** `workflow_orchestrator/mod.rs`

Create modules:
```
workflow_orchestrator/
├── mod.rs (300 lines - coordination only)
├── phase_scanning.rs (existing)
├── phase_extraction.rs (NEW - move Phase 1-3)
├── phase_segmentation.rs (NEW - move Phase 4)
├── phase_fingerprinting.rs (NEW - move Phase 5)
├── phase_matching.rs (NEW - move Phase 6-7)
├── phase_amplitude.rs (NEW - move Phase 8)
├── phase_recording.rs (NEW - move Phase 9-10)
└── worker_pool.rs (NEW - worker management)
```

### Task 3.2: Unify Duplicate Extractors (8 hours)
Create trait-based system:

**File:** `wkmp-ai/src/extractors/mod.rs` (NEW)
```rust
pub trait MetadataExtractor: Send + Sync {
    async fn extract(&self, file: &Path) -> Result<Metadata>;
}

pub trait AudioExtractor: Send + Sync {
    async fn extract(&self, file: &Path) -> Result<AudioData>;
}

// Implementations
pub struct Id3Extractor;
impl MetadataExtractor for Id3Extractor { ... }

pub struct MusicBrainzExtractor;
impl MetadataExtractor for MusicBrainzExtractor { ... }
```

Delete duplicates:
- Keep ONE: `extractors/id3_extractor.rs`
- Delete: `fusion/extractors/id3_extractor.rs`
- Keep ONE: `services/musicbrainz_client.rs`
- Delete: `extractors/musicbrainz_client.rs`, `fusion/extractors/musicbrainz_client.rs`

### Task 3.3: Fix N+1 Query Patterns (6 hours)
**Files:** `db/passages.rs`, `db/files.rs`

Convert loops to batch operations:
```rust
// BAD: N queries
for passage in passages {
    sqlx::query("INSERT INTO passages...")
        .bind(&passage.id)
        .execute(pool).await?;
}

// GOOD: 1 query
let query = format!(
    "INSERT INTO passages VALUES {}",
    passages.iter()
        .map(|p| format!("('{}', ...)", p.id))
        .collect::<Vec<_>>()
        .join(",")
);
sqlx::query(&query).execute(pool).await?;
```

### Task 3.4: Extract Configuration (4 hours)
Move all hardcoded values to settings:

**Add to `wkmp-common/src/db/init.rs`:**
```rust
ensure_setting(pool, "ai_bind_address", "127.0.0.1:5723").await?;
ensure_setting(pool, "ai_event_bus_capacity", "1000").await?;
ensure_setting(pool, "ai_db_retry_timeout_ms", "5000").await?;
ensure_setting(pool, "ai_default_sample_rate", "44100").await?;
ensure_setting(pool, "ai_max_pagination_limit", "1000").await?;
```

### Task 3.5: Clean Up Static Asset Handlers (2 hours)
**File:** `api/ui/static_assets.rs`

Replace 6 duplicate handlers with generic:
```rust
pub async fn serve_static_file(
    path: &str,
    content_type: &str,
) -> Result<impl IntoResponse> {
    let content = STATIC_FILES.get(path)
        .ok_or_else(|| StatusCode::NOT_FOUND)?;

    Ok((
        [(header::CONTENT_TYPE, content_type)],
        content.data.into(),
    ))
}
```

---

## Phase 4: Testing & Documentation (Week 3)
**Goal:** Comprehensive test coverage
**Time:** 30-40 hours

### Task 4.1: Remove unwrap() from Tests (8 hours)
**Files:** All test modules

Pattern:
```rust
// BAD: Test with unwrap
#[test]
fn test_something() {
    let result = function().unwrap();
}

// GOOD: Test returns Result
#[test]
fn test_something() -> Result<()> {
    let result = function()?;
    Ok(())
}
```

### Task 4.2: Add Integration Tests (12 hours)
Create `wkmp-ai/tests/integration/`:
- `import_workflow_test.rs` - Full import flow
- `cancellation_test.rs` - Cancel handling
- `memory_threshold_test.rs` - Memory monitoring
- `batch_operations_test.rs` - Database batching

### Task 4.3: Update Documentation (4 hours)
- Update `workflow_orchestrator/mod.rs:20-22` - Fix line count (3,382 not 1,459)
- Add module-level docs to all new modules
- Document new settings parameters

### Task 4.4: Add Error Context (6 hours)
Wrap all `?` operators with context:
```rust
// BAD: No context
let file = File::open(path)?;

// GOOD: With context
let file = File::open(path)
    .with_context(|| format!("Failed to open file: {}", path.display()))?;
```

---

---

## Testing Checklist

After each phase, verify:

### Phase 1 Verification:
```bash
# No more AcoustID timeouts
grep "operation timed out" testW.log | wc -l  # Should be 0

# Pool size correct
grep "pool_size=30" testW.log

# Memory threshold applied
grep "threshold: 12884MB" testW.log

# No panics
cargo test --no-fail-fast
```

### Phase 2 Verification:
```bash
# Memory stable
ps aux | grep wkmp-ai  # RSS should stabilize

# Events flowing
curl http://localhost:5723/api/events  # Should see updates

# Batch operations working
grep "INSERT INTO passages" testW.log  # Should see batch inserts
```

### Phase 3 Verification:
```bash
# File sizes reasonable
find wkmp-ai/src -name "*.rs" -exec wc -l {} \; | sort -rn | head -10
# No file should exceed 500 lines

# No duplicates
find wkmp-ai -name "*_extractor.rs" | wc -l  # Should be reduced
```

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking changes | Feature flag new code paths initially |
| Performance regression | Benchmark before/after each phase |
| Data loss | Test on copy of production database |
| Integration failures | Comprehensive integration test suite |

---

## Success Metrics

**Before:**
- Import time: 3+ hours for 327 files
- Completion rate: 52%
- AcoustID timeouts: 725
- Memory usage: Unbounded growth
- Panic risks: 30+ unwrap() calls

**After:**
- Import time: <30 minutes for 5,000 files
- Completion rate: 95%+
- AcoustID timeouts: 0 (disabled)
- Memory usage: Stable at <4GB
- Panic risks: 0 unwrap() in production

---

## Delivery Schedule

**Week 1:** Phase 1 & 2 - Emergency fixes + Resource management
**Week 2:** Phase 3 - Architecture refactoring
**Week 3:** Phase 4 - Testing & documentation
**Week 4:** Buffer for issues & final validation

---

## Notes for Sonnet

1. **Start with Phase 1** - System is unusable until these fixes are applied
2. **Test after each task** - Don't accumulate untested changes
3. **Use feature flags** if concerned about breaking changes
4. **Coordinate with user** on timing for production deployment
5. **Keep detailed logs** of changes for rollback if needed

The most critical items are marked with 🔥 and should be done first within each phase.
# PLAN031: Critical Performance Fixes - testV.log Analysis

**Status:** URGENT - System near-unusable
**Created:** 2025-11-16
**Context:** testV.log 3+ hour run analysis showing severe stalling

---

## Critical Findings

### System Performance Metrics (3+ hours runtime)
- **Files started:** 327
- **Files completed:** 171 (52% completion rate)
- **Files stuck:** 156 (48% failure rate)
- **Throughput:** ~1 file/minute (should be 20-30)
- **AcoustID timeouts:** 725 occurrences
- **CPU utilization:** 15-20% (from testU.log)

### Root Causes Identified

#### 1. AcoustID API Catastrophic Failure
- **Timeout duration:** 17 seconds per request
- **Impact:** 725 timeouts × 17 seconds = 3.4 hours of blocking
- **Fallback:** System falls back to metadata-only matching
- **Song identification:** COMPLETELY BROKEN

#### 2. Connection Pool Starvation
- **Pool size:** 14 connections (should be 30)
- **Acquisition time:** 2-5 seconds (should be <100ms)
- **Utilization:** Only 42% (6 of 14 in use)
- **Workers blocked:** Waiting for connections

#### 3. Memory-Intensive Operations
- **Amplitude analysis:** Processing tiny segments (0.1-0.9 seconds)
- **Phase 8 duration:** Still taking seconds for microsecond segments
- **Pattern:** Excessive small I/O operations

#### 4. Configuration Not Applied
- **Expected:** 30 connections, 5000ms timeout
- **Actual:** 14 connections, unknown timeout
- **Worker count:** Still appears to be 8 (not optimized)

---

## IMMEDIATE FIXES REQUIRED

### Fix 1: Kill AcoustID Integration (30 minutes)
**Problem:** AcoustID is completely broken and blocking everything
**Solution:** Circuit breaker with 1-second timeout

```rust
// wkmp-ai/src/services/acoustid_client.rs

const ACOUSTID_ENABLED: bool = false; // EMERGENCY KILL SWITCH
const ACOUSTID_TIMEOUT: Duration = Duration::from_secs(1); // Was 17 seconds!

pub async fn lookup(&self, fingerprint: &str) -> Result<Option<String>> {
    if !ACOUSTID_ENABLED {
        return Ok(None); // Skip entirely
    }

    // If we must try, use aggressive timeout
    match timeout(ACOUSTID_TIMEOUT, self.client.post(URL).send()).await {
        Ok(Ok(response)) => Ok(parse_response(response)),
        _ => Ok(None) // Fail fast, move on
    }
}
```

### Fix 1b: Skip Fingerprinting for Tiny Passages (30 minutes)
**Problem:** Passages <10 seconds don't have enough audio for meaningful fingerprinting
**Solution:** Skip chromaprint/AcoustID entirely for short passages

```rust
// wkmp-ai/src/services/passage_fingerprinter.rs

const MIN_PASSAGE_DURATION_FOR_FINGERPRINT: f64 = 10.0; // 10 seconds minimum

pub async fn fingerprint_passages(&self, passages: Vec<Passage>) -> Result<Vec<FingerprintResult>> {
    let mut results = Vec::new();

    for passage in passages {
        let duration_seconds = (passage.end_ticks - passage.start_ticks) as f64
                              / TICKS_PER_SECOND as f64;

        if duration_seconds < MIN_PASSAGE_DURATION_FOR_FINGERPRINT {
            // Skip tiny passages - not enough audio for reliable fingerprinting
            info!("Skipping fingerprint for {:.1}s passage (too short)", duration_seconds);
            results.push(FingerprintResult::Skipped {
                reason: "Passage too short for fingerprinting",
                duration: duration_seconds,
            });
            continue;
        }

        // Process normally for passages >= 10 seconds
        let fingerprint = self.generate_chromaprint(&passage).await?;

        // Only attempt AcoustID if fingerprint generation succeeded
        if let Some(fp) = fingerprint {
            let acoustid_result = self.acoustid_client.lookup(&fp).await?;
            results.push(FingerprintResult::Success {
                fingerprint: fp,
                acoustid_match: acoustid_result,
            });
        }
    }

    Ok(results)
}
```

**Additional optimization in chromaprint generation:**
```rust
// wkmp-ai/src/services/fingerprinter.rs

pub async fn generate_chromaprint(&self, passage: &Passage) -> Result<Option<String>> {
    let duration_seconds = passage.duration_seconds();

    // Early exit for passages too short
    if duration_seconds < 10.0 {
        debug!("Passage too short for chromaprint: {:.1}s", duration_seconds);
        return Ok(None);
    }

    // Existing chromaprint generation code...
}
```

**Impact:**
- Eliminates hundreds of unnecessary fingerprint attempts
- Reduces AcoustID API calls by ~50% (many passages are intro/outro segments)
- Saves ~5-10 seconds per tiny passage (fingerprint generation + API timeout)
- Improves accuracy (short passages produce unreliable fingerprints)

### Fix 2: Fix Pool Configuration (1 hour)
**Problem:** Configuration not being applied correctly
**Solution:** Hard-code values temporarily

```rust
// wkmp-ai/src/models/bootstrap_config.rs

pub fn create_production_pool(config: &BootstrapConfig) -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(&config.database_path)
        .busy_timeout(Duration::from_millis(5000)) // HARD-CODED
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    SqlitePoolOptions::new()
        .max_connections(30) // HARD-CODED
        .min_connections(15) // Keep minimum ready
        .acquire_timeout(Duration::from_secs(5)) // Fail fast
        .idle_timeout(Duration::from_secs(600))
        .connect_with(options)
        .await?
}
```

### Fix 3: Skip Amplitude Analysis for Tiny Passages (30 minutes)
**Problem:** Analyzing passages <10 seconds wastes resources for minimal benefit
**Solution:** Skip amplitude analysis for short passages (align with fingerprinting threshold)

```rust
// wkmp-ai/src/services/passage_amplitude_analyzer.rs

const MIN_PASSAGE_DURATION_SECONDS: f64 = 10.0; // Aligned with fingerprinting threshold

pub async fn analyze_amplitude(&self, passage: &Passage) -> Result<()> {
    let duration_seconds = (passage.end_ticks - passage.start_ticks) as f64
                          / TICKS_PER_SECOND as f64;

    if duration_seconds < MIN_PASSAGE_DURATION_SECONDS {
        // Skip tiny passages - not worth analyzing
        debug!("Skipping amplitude analysis for {:.1}s passage", duration_seconds);
        return Ok(AmplitudeResult::default_for_short_passage());
    }

    // Continue with normal analysis...
}
```

**Rationale for 10-second threshold:**
- Passages <10 seconds are typically intro/outro segments
- Insufficient audio for reliable lead-in/lead-out detection
- Aligns with fingerprinting threshold for consistency
- Eliminates processing of hundreds of micro-segments (0.1-9.9 seconds)

### Fix 4: Batch Database Operations (2 hours)
**Problem:** Individual operations for every passage
**Solution:** Batch all operations per file

```rust
// wkmp-ai/src/services/passage_recorder.rs

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

    // Single status update
    sqlx::query("UPDATE files SET status = 'COMPLETE' WHERE guid = ?")
        .bind(file_id)
        .execute(&mut tx)
        .await?;

    tx.commit().await?;
    Ok(())
}
```

### Fix 5: Emergency Worker Reduction (30 minutes)
**Problem:** Too many workers fighting for resources
**Solution:** Reduce to 4 workers temporarily

```rust
// wkmp-ai/src/services/workflow_orchestrator/mod.rs

const EMERGENCY_WORKER_COUNT: usize = 4; // Was 8

pub fn start_workers(&self) -> Vec<JoinHandle<()>> {
    (0..EMERGENCY_WORKER_COUNT)
        .map(|_| self.spawn_worker())
        .collect()
}
```

---

## Expected Improvements

### Before (testV.log)
- **Throughput:** 1 file/minute
- **Completion rate:** 52%
- **AcoustID timeouts:** 725
- **Connection waits:** 2-5 seconds
- **3-hour progress:** 171 files

### After Emergency Fixes
- **Throughput:** 10-15 files/minute
- **Completion rate:** 95%+
- **AcoustID timeouts:** 0 (disabled)
- **Connection waits:** <100ms
- **3-hour progress:** 1800+ files

---

## Implementation Priority

1. **FIRST: Kill AcoustID** (30 min)
   - This alone will unblock most operations
   - Song matching will use metadata only

2. **SECOND: Skip Tiny Passages (<10s)** (30 min)
   - Implement Fix 1b for fingerprinting
   - Reduces API calls by ~50%
   - Prevents timeouts on unreliable short segments

3. **THIRD: Fix Pool Config** (1 hour)
   - Hard-code values to ensure they apply
   - Verify with startup logs

4. **FOURTH: Skip Amplitude for Tiny Passages** (30 min)
   - Align with fingerprinting threshold (10 seconds)
   - Eliminate wasteful micro-operations

5. **FIFTH: Reduce Workers** (30 min)
   - Less contention immediately
   - Can increase later when stable

6. **SIXTH: Batch Operations** (2 hours)
   - Bigger improvement but more complex

---

## Monitoring After Fixes

```bash
# Check AcoustID is disabled
grep "ACOUSTID_ENABLED" testW.log

# Verify pool size
grep "pool_size=30" testW.log

# Monitor throughput
grep "Phase 10 completed" testW.log | wc -l

# Check for timeouts
grep "operation timed out" testW.log | wc -l

# Watch completion rate
watch -n 10 'grep "Starting per-file" testW.log | wc -l; grep "Phase 10 completed" testW.log | wc -l'
```

---

## Long-term Solutions (After Emergency Fixes)

1. **Replace AcoustID:**
   - Move to local fingerprinting only
   - Or implement proper caching layer
   - Or find alternative service

2. **Streaming Architecture:**
   - Don't load entire files
   - Process in chunks
   - Reduce memory pressure

3. **Proper Queue Management:**
   - Separate I/O queue
   - CPU-bound queue
   - Network queue

4. **Database Optimization:**
   - Prepared statements
   - Connection pooling per operation type
   - Consider PostgreSQL for better concurrency

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| No song identification | Temporary - metadata matching still works |
| Reduced worker count causes slowdown | Monitor and adjust if needed |
| Hard-coded values | Document for later cleanup |
| Batch operations fail | Rollback to individual ops |

---

## Success Criteria

- [ ] Zero AcoustID timeouts
- [ ] Connection acquisition <100ms
- [ ] 10+ files/minute throughput
- [ ] 95%+ completion rate
- [ ] No stalled workers after 30 minutes

---

**URGENT ACTION REQUIRED:** System is effectively broken. Implement Fix #1 (Kill AcoustID) immediately to restore basic functionality. The 17-second timeout is causing cascade failures throughout the system.
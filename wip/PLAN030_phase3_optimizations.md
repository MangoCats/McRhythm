# PLAN030: Phase 3 Performance Optimizations - Addressing 15% CPU Utilization

**Status:** Ready for Implementation
**Created:** 2025-11-16
**Context:** testU.log analysis after PLAN029 Phases 1-2

---

## Executive Summary

After implementing dual-pool architecture (PLAN029 Phases 1-2), CPU utilization remains at 15% due to:
1. Incorrect pool configuration (96 connections, 250ms timeout)
2. Sequential processing of large audio files (80+ minutes, 1.7GB RAM)
3. Blocking network I/O (60-second AcoustID timeouts)
4. Workers holding connections during long operations (15-43 second waits)

**Solution:** Implement streaming audio processing, async fingerprinting, and proper pool configuration.

---

## Critical Issues Found

### Issue 1: Pool Configuration Not Applied
**Current:** 96 connections, 250ms busy_timeout
**Expected:** 30 connections, 5000ms busy_timeout
**Impact:** Connection starvation, 15-43 second acquisition waits

### Issue 2: Large File Memory Explosion
**Current:** Loading entire 80-minute files (1.7GB) into RAM
**Impact:** Sequential processing, memory pressure, worker blocking

### Issue 3: Synchronous Network I/O
**Current:** Workers block for 60 seconds on AcoustID timeouts
**Impact:** Connection held unusable for entire timeout period

### Issue 4: Phase Serialization
**Current:** Phase 5 (27-90s) and Phase 8 (74-139s) per file
**Impact:** Only 1-2 files processing simultaneously despite 21 workers

---

## Phase 3 Implementation Plan

### 3.1: Fix Pool Configuration (1 hour)
```rust
// wkmp-ai/src/models/bootstrap_config.rs

// CHANGE FROM:
const DEFAULT_POOL_SIZE: u32 = 96;
const DEFAULT_BUSY_TIMEOUT_MS: u32 = 250;

// CHANGE TO:
const DEFAULT_POOL_SIZE: u32 = 30;
const DEFAULT_BUSY_TIMEOUT_MS: u32 = 5000;
```

**Verification:**
```bash
grep "pool_size=30" testV.log
grep "busy_timeout=5000ms" testV.log
```

### 3.2: Streaming Audio Processing (4-6 hours)

**Problem:** Loading entire audio files into memory causes:
- 1.7GB RAM per 80-minute file
- Sequential processing bottleneck
- Workers blocked during decode

**Solution:** Process audio in chunks
```rust
// wkmp-ai/src/utils/streaming_decoder.rs

pub struct StreamingDecoder {
    path: PathBuf,
    chunk_size_samples: usize, // 10 seconds = 441,000 samples
}

impl StreamingDecoder {
    pub async fn process_chunks<F>(&self, mut processor: F) -> Result<()>
    where
        F: FnMut(AudioChunk) -> Result<()>,
    {
        let reader = BufReader::with_capacity(1024 * 1024, File::open(&self.path)?);
        let source = Decoder::new(reader)?;

        let mut buffer = Vec::with_capacity(self.chunk_size_samples);
        for sample in source {
            buffer.push(sample);
            if buffer.len() >= self.chunk_size_samples {
                processor(AudioChunk::new(&buffer))?;
                buffer.clear();
            }
        }
        Ok(())
    }
}
```

### 3.3: Async Fingerprinting Pipeline (3-4 hours)

**Problem:** Sequential fingerprinting of passages
**Solution:** Pipeline with bounded concurrency

```rust
// wkmp-ai/src/services/async_fingerprinter.rs

pub struct AsyncFingerprinter {
    semaphore: Arc<Semaphore>, // Limit to 4 concurrent
}

impl AsyncFingerprinter {
    pub async fn fingerprint_passages(&self, passages: Vec<Passage>) -> Vec<Result<Fingerprint>> {
        let futures = passages.into_iter().map(|passage| {
            let sem = self.semaphore.clone();
            async move {
                let _permit = sem.acquire().await?;

                // Process in background task
                tokio::task::spawn_blocking(move || {
                    generate_fingerprint(passage)
                }).await?
            }
        });

        futures::future::join_all(futures).await
    }
}
```

### 3.4: Network I/O Timeout Management (2 hours)

**Problem:** 60-second AcoustID timeouts block workers
**Solution:** Aggressive timeouts with circuit breaker

```rust
// wkmp-ai/src/services/acoustid_client.rs

const ACOUSTID_TIMEOUT: Duration = Duration::from_secs(5);
const CIRCUIT_BREAKER_THRESHOLD: u32 = 3;

pub struct AcoustIDClient {
    client: Client,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
}

impl AcoustIDClient {
    pub async fn lookup(&self, fingerprint: &str) -> Result<Option<String>> {
        if self.circuit_breaker.lock().await.is_open() {
            return Ok(None); // Skip if circuit open
        }

        match timeout(ACOUSTID_TIMEOUT, self.client.post(URL).send()).await {
            Ok(Ok(response)) => {
                self.circuit_breaker.lock().await.on_success();
                Ok(parse_response(response))
            }
            Ok(Err(e)) | Err(_) => {
                self.circuit_breaker.lock().await.on_failure();
                Ok(None) // Return None instead of blocking
            }
        }
    }
}
```

### 3.5: Worker Pool Optimization (2 hours)

**Problem:** 21 workers but only 2-3 active
**Solution:** Reduce workers, increase per-worker efficiency

```rust
// wkmp-ai/src/services/workflow_orchestrator/mod.rs

// CHANGE FROM:
const DEFAULT_WORKER_COUNT: usize = num_cpus::get() + 1; // 21 on 20-core

// CHANGE TO:
const DEFAULT_WORKER_COUNT: usize = 8; // Optimal for I/O-bound work

// Add worker efficiency monitoring
pub struct WorkerMetrics {
    active_time: Duration,
    idle_time: Duration,
    files_processed: u32,
}
```

### 3.6: Phase Parallelization (3 hours)

**Problem:** Phases 5 and 8 run sequentially
**Solution:** Run independent phases in parallel

```rust
// wkmp-ai/src/services/workflow_orchestrator/mod.rs

async fn process_file(&self, file: &FileInfo) -> Result<()> {
    // Phases 1-4: Sequential (dependencies)
    let metadata = self.extract_metadata(file).await?;
    let passages = self.segment_passages(file, &metadata).await?;

    // Phases 5 & 8: Parallel (independent)
    let (fingerprints, amplitudes) = tokio::join!(
        self.fingerprint_passages(&passages),
        self.analyze_amplitudes(&passages)
    );

    // Phases 6-7, 9-10: Sequential (use results)
    self.match_songs(&fingerprints?).await?;
    self.record_passages(&passages, &amplitudes?).await?;

    Ok(())
}
```

---

## Expected Improvements

### Before (testU.log)
- **CPU Utilization:** 15%
- **Files/minute:** 2-3
- **Connection waits:** 15-43 seconds
- **Memory/file:** 1.7GB for large files
- **Phase 5 duration:** 27-90 seconds
- **Phase 8 duration:** 74-139 seconds

### After (Expected)
- **CPU Utilization:** 60-80%
- **Files/minute:** 15-20
- **Connection waits:** <1 second
- **Memory/file:** 200MB streaming chunks
- **Phase 5 duration:** 5-15 seconds
- **Phase 8 duration:** 10-20 seconds

---

## Implementation Order

1. **Fix pool configuration** (30 minutes)
   - Update bootstrap_config.rs constants
   - Verify in startup logs

2. **Add network timeouts** (2 hours)
   - Implement circuit breaker
   - Add 5-second timeout

3. **Reduce worker count** (1 hour)
   - Change to 8 workers
   - Add metrics collection

4. **Parallelize phases** (3 hours)
   - Run phases 5 & 8 concurrently
   - Test with tokio::join!

5. **Streaming decoder** (4-6 hours)
   - Implement chunk processing
   - Test with large files

6. **Async fingerprinting** (3-4 hours)
   - Add semaphore concurrency
   - Pipeline processing

---

## Testing Strategy

### Test 1: Pool Configuration
```bash
cargo test --test pool_config_test
# Verify: pool_size=30, busy_timeout=5000ms
```

### Test 2: Large File Handling
```bash
# Create 80-minute test file
cargo test --test large_file_test -- --nocapture
# Monitor memory usage: should stay under 500MB
```

### Test 3: CPU Utilization
```bash
# Run import with monitoring
cargo run --release -- import-folder /test/data
# Check: CPU should reach 60-80%
```

### Test 4: Network Resilience
```bash
# Block AcoustID API
cargo test --test network_timeout_test
# Verify: No worker blocking, circuit breaker activates
```

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Streaming breaks fingerprinting | Keep fallback to full-file mode |
| Circuit breaker too aggressive | Configurable thresholds |
| Reduced workers causes slowdown | Monitor and adjust dynamically |
| Parallel phases cause deadlock | Careful dependency analysis |

---

## Success Criteria

- [ ] CPU utilization reaches 60-80%
- [ ] No connection acquisition > 2 seconds
- [ ] Memory usage < 500MB per file
- [ ] 5x throughput improvement (15+ files/minute)
- [ ] Zero timeout-related failures

---

## Monitoring Commands

```bash
# Watch CPU utilization
watch -n 1 "grep 'Pool utilization' testV.log | tail -5"

# Check connection waits
grep "exceeded slow threshold" testV.log | wc -l

# Monitor phase durations
grep "Phase [58] completed" testV.log | awk '{print $NF}'

# Track memory usage
ps aux | grep wkmp-ai | awk '{print $6}'
```

---

## Next Steps

1. Review this plan with team
2. Create branch `perf/phase3-optimizations`
3. Implement in order (config first, streaming last)
4. Run benchmarks after each component
5. Document results in testV.log

---

**Estimated Total Time:** 16-20 hours
**Priority:** CRITICAL - System unusable with current performance
**Dependencies:** Requires PLAN029 Phases 1-2 completed
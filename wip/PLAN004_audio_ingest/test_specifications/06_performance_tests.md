# Performance Tests - wkmp-ai

**Requirements:** AIA-PERF-010, AIA-PERF-020
**Priority:** P1 (High)
**Test Count:** 6

---

## TEST-068: 100 Files in 2-5 Minutes

**Requirement:** AIA-PERF-010
**Type:** Performance
**Priority:** P1

**Given:**
- Test library with 100 MP3 files (average 3 minutes each)
- MusicBrainz API responsive (or mock with realistic delays)
- Parameters: import_parallelism = 4

**When:**
- Start import session

**Then:**
- Import completes within 5 minutes
- Minimum 2 minutes (network latency exists)
- All 100 files processed successfully

**Acceptance Criteria:**
- ✅ Total duration: 2-5 minutes (120-300 seconds)
- ✅ All files imported
- ✅ No timeouts or failures
- ✅ Performance within PERF-010 target

---

## TEST-069: 1000 Files in 20-40 Minutes

**Requirement:** AIA-PERF-010
**Type:** Performance
**Priority:** P1

**Given:**
- Large library with 1000 files
- Real or mock MusicBrainz API (1 req/s rate limit)
- Parameters: import_parallelism = 4

**When:**
- Start import session

**Then:**
- Import completes within 40 minutes
- Minimum 20 minutes (rate limiting floor: 1000 files × ~1s = 16.7 min)
- files_processed = 1000

**Acceptance Criteria:**
- ✅ Total duration: 20-40 minutes (1200-2400 seconds)
- ✅ No memory leaks (RSS stable)
- ✅ CPU usage reasonable (<80% average)
- ✅ Performance within PERF-010 target (±20%)

---

## TEST-070: Rate Limit Compliance (MusicBrainz 1/s)

**Requirement:** AIA-PERF-010
**Type:** Performance
**Priority:** P1

**Given:**
- Import session with 100 files requiring MusicBrainz lookups
- MusicBrainz rate limiter active (1 req/s)

**When:**
- Monitor MusicBrainz request timing

**Then:**
- No requests faster than 1 second apart
- Average request interval: ~1000ms
- No 503 (rate limit exceeded) errors from MusicBrainz

**Acceptance Criteria:**
- ✅ Min interval between requests ≥ 1000ms
- ✅ No rate limit violations
- ✅ No exponential backoff triggered
- ✅ Consistent pacing throughout import

---

## TEST-071: Cache Hit Reduces API Calls

**Requirement:** AIA-PERF-020
**Type:** Performance
**Priority:** P1

**Given:**
- First import: 100 files (populate cache)
- Second import: Same 100 files (cache hit)

**When:**
- Run second import

**Then:**
- AcoustID API calls: 0 (100% cache hit)
- MusicBrainz API calls: 0 (100% cache hit)
- AcousticBrainz API calls: 0 (100% cache hit)
- Import completes much faster (~1-2 minutes vs 3-5 minutes)

**Acceptance Criteria:**
- ✅ Zero external API calls on second import
- ✅ Duration reduced by >50%
- ✅ All data retrieved from cache tables
- ✅ Data accuracy identical to first import

---

## TEST-072: Batch Inserts (100 at a time)

**Requirement:** AIA-PERF-020
**Type:** Performance
**Priority:** P1

**Given:**
- Import session creating 500 passages
- Batch size: 100 passages per transaction

**When:**
- Monitor database inserts

**Then:**
- 5 transactions total (500 / 100 = 5)
- Each transaction inserts 100 passages
- Transaction time: <1 second per batch

**Acceptance Criteria:**
- ✅ Batch size = 100 (not 1-by-1)
- ✅ Transaction overhead minimized
- ✅ All passages inserted successfully
- ✅ Performance gain vs sequential inserts (>5x faster)

---

## TEST-073: Parallel Processing Speedup

**Requirement:** AIA-PERF-020
**Type:** Performance
**Priority:** P1

**Given:**
- 40 files to process
- Test A: import_parallelism = 1 (sequential)
- Test B: import_parallelism = 4 (parallel)

**When:**
- Run both tests

**Then:**
- Test A duration: T seconds
- Test B duration: ~T/3 seconds (not T/4 due to rate limiting)
- Speedup factor: ~3x (accounting for MusicBrainz bottleneck)

**Acceptance Criteria:**
- ✅ Parallel import faster than sequential
- ✅ Speedup factor ≥ 2.5x
- ✅ Both produce identical database state
- ✅ No race conditions or data corruption

---

## Test Implementation Notes

**Framework:** `cargo test --test performance_tests -p wkmp-ai -- --test-threads=1`

**Performance Measurement:**
```rust
use std::time::Instant;

#[tokio::test]
async fn test_100_files_performance() {
    let start = Instant::now();

    let library = create_test_library(100);
    let session_id = start_import(&library).await;
    wait_for_completion(session_id).await;

    let duration = start.elapsed();

    assert!(
        duration >= Duration::from_secs(120),
        "Import too fast (no network latency?): {}s", duration.as_secs()
    );
    assert!(
        duration <= Duration::from_secs(300),
        "Import too slow (timeout): {}s", duration.as_secs()
    );
}
```

**Rate Limit Monitoring:**
```rust
#[tokio::test]
async fn test_rate_limit_compliance() {
    let mut last_request_time = None;
    let mut intervals = Vec::new();

    // Mock MusicBrainz client with timing capture
    let client = create_monitored_mb_client(|request_time| {
        if let Some(last) = last_request_time {
            let interval = request_time.duration_since(last);
            intervals.push(interval);
        }
        last_request_time = Some(request_time);
    });

    // Run import...

    // Verify intervals
    for interval in intervals {
        assert!(
            interval >= Duration::from_millis(1000),
            "Rate limit violated: {}ms interval", interval.as_millis()
        );
    }
}
```

**Memory Leak Detection:**
```rust
#[tokio::test]
async fn test_no_memory_leaks() {
    let initial_rss = get_process_rss();

    for _ in 0..10 {
        let session_id = start_import(&test_library_100).await;
        wait_for_completion(session_id).await;
    }

    let final_rss = get_process_rss();
    let growth = final_rss - initial_rss;

    assert!(
        growth < 100_000_000, // <100MB growth
        "Memory leak detected: {}MB growth", growth / 1_000_000
    );
}
```

**Performance Variance Handling:**
- Allow ±20% variance from targets (PERF-010 specification)
- Run tests on dedicated hardware (no CI shared runners)
- Median of 3 runs to reduce noise
- Fail only if all 3 runs exceed threshold

---

End of performance tests

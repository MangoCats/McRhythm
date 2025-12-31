# Album Matcher 28 - Deadlock Fix (COMPLETED)

## Summary

Fixed rayon::scope() deadlock that caused run28c to stall after ~60 albums when running with 16 concurrent albums. Run28d completed successfully with 200 albums in ~16 minutes.

## Problem Analysis (2025-11-26)

### Problem Discovered
Run28c with `MAX_CONCURRENT_ALBUMS=16` **deadlocked completely** after processing ~60 albums. Albums A9, A12, A13, A14, A58-A67 were stuck for 30+ minutes with heartbeats showing "MusicBrainz search complete" but no further progress.

### Root Cause Analysis
The deadlock occurs in `main.rs` lines 370-461:

```rust
rayon::scope(|s| {
    for (edition_idx, edition) in editions.iter().enumerate() {
        // ... setup ...
        s.spawn(move |_| {
            // edition processing work
        });

        // THIS IS THE PROBLEM: blocking sleep INSIDE rayon::scope
        if edition_idx < editions.len() - 1 {
            std::thread::sleep(Duration::from_secs(EDITION_FEED_DELAY_SECS));  // 4 seconds
        }
    }
});  // BLOCKS until ALL spawned work completes
```

**Deadlock mechanism:**
1. `rayon::scope()` blocks the calling thread until ALL spawned work completes
2. With 16 albums, each album's tokio task blocks in `rayon::scope()` waiting for rayon threads
3. Rayon has only 20 threads in its pool
4. Each album holds its scope open for `N editions × 4 seconds` due to the sleep
5. **Deadlock**: Albums waiting for spawned work to complete, but rayon threads are blocked by OTHER albums' scope waits

**Why 6 concurrent albums worked but 16 doesn't:**
- 6 albums × limited rayon demands = sufficient threads available
- 16 albums × blocking scopes with 4s delays = thread pool exhaustion and deadlock

### Fix Implemented

Changed from blocking `rayon::scope()` to non-blocking `rayon::spawn()` with tokio channels:

```rust
// Wrap shared data in Arc for rayon::spawn (no lifetime constraints)
let silence_cache = Arc::new(silence_cache);
let rms_profile = Arc::new(rms_profile);
// ... other Arc wraps ...

// Channel for collecting results
let (result_tx, mut result_rx) = tokio::sync::mpsc::channel::<EditionTestResult>(editions.len());

for (edition_idx, edition) in editions.iter().enumerate() {
    // Clone Arc handles
    let silence_cache = Arc::clone(&silence_cache);
    let result_tx = result_tx.clone();
    let edition = edition.clone();

    // Non-blocking spawn - doesn't hold a scope open
    rayon::spawn(move || {
        let result = test_single_edition(...);
        let _ = result_tx.blocking_send(result);
    });

    // Delay OUTSIDE rayon - async sleep doesn't block rayon threads
    if edition_idx < editions.len() - 1 {
        sleep(Duration::from_secs(EDITION_FEED_DELAY_SECS)).await;  // tokio async sleep
    }
}

drop(result_tx);  // Close channel when done spawning

// Collect results asynchronously
while let Some(result) = result_rx.recv().await {
    edition_results.push(result);
}
```

**Key changes:**
1. `rayon::scope()` -> `rayon::spawn()` (non-blocking)
2. `std::thread::sleep()` -> `tokio::time::sleep().await` (async, doesn't block rayon)
3. `Mutex<Vec<Result>>` -> `tokio::sync::mpsc::channel` (async-friendly result collection)
4. Borrowed references -> `Arc` clones (required for `rayon::spawn` which has no lifetime bounds)

### Previous Blocker (RESOLVED)

The wkmp-ai library had pre-existing compile errors in `src/matching/editions/grouping.rs`. These were fixed by another agent, allowing the deadlock fix to be tested successfully.

### Files Modified

1. **`wkmp-ai/examples/am28/main.rs`** - Lines 364-478
   - Replaced `rayon::scope()` pattern with `rayon::spawn()` + channel pattern
   - Changed sleep from blocking to async

### Expected Behavior After Fix

1. 16 albums can process concurrently without deadlock
2. 4-second delay between edition launches is preserved (now async)
3. Rayon threads are not blocked by album-level coordination
4. Each album's edition processing runs independently on rayon's thread pool

### Test Results - Run28d (SUCCESS)

**Lib fixed by another agent. Deadlock fix tested and confirmed working.**

#### Run28d Summary
- **Status:** COMPLETED SUCCESSFULLY
- **Albums processed:** 200 (199 FINAL RESULT messages)
- **Successfully analyzed:** 191
- **Total runtime:** ~16 minutes (12:48:14 to 13:04:00)
- **No deadlock** - passed the ~60 album mark where run28c stalled

#### Matching Results
| Metric | Value |
|--------|-------|
| Average match percentage | 98.6% |
| Mean error | 3.40s |
| Perfect track count matches | 185/191 (96.9%) |

#### Confidence Distribution
- Excellent (≥80%): 190 albums
- Good (60-79%): 5 albums
- Fair (40-59%): 0 albums
- Poor (<40%): 0 albums

#### Stage Distribution
| Stage | Albums |
|-------|--------|
| Stage 2 (silence detection) | 138 |
| Stage 3 (DP assembly) | 52 |
| Stage 4 (guided) | 5 |
| Stage 6 (merging) | 0 |

#### Comparison vs Run28c (before fix)
| Metric | Run28c | Run28d |
|--------|--------|--------|
| Completed albums | ~60 (then deadlocked) | 200 |
| Deadlock | YES (30+ min stall) | NO |
| MAX_CONCURRENT_ALBUMS | 16 | 16 |

**Conclusion:** The deadlock fix works. Replacing `rayon::scope()` + blocking sleep with `rayon::spawn()` + async sleep eliminates the thread pool exhaustion that caused run28c to deadlock.

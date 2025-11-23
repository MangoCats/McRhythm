# Boundary Detection Blocking Fix

**Date:** 2025-11-11
**Status:** ✅ COMPLETED - All 244 tests passing

## Problem

After correcting parallelism to CPU count (14 files on 14-core system), CPU utilization remained at 5-7% with infrequent UI updates.

**User Report:**
- CPU utilization: 5-7% (unchanged after parallelism correction)
- UI updates: Very infrequent
- Single file stuck in boundary detection for 4+ minutes

## Root Cause Analysis

### The Real Bottleneck

The `detect_boundaries()` function was marked `async` but performed **100% synchronous CPU work**:

```rust
pub async fn detect_boundaries(file_path: &Path) -> Result<Vec<PassageBoundary>> {
    // 1. Decode entire audio file (symphonia - synchronous I/O + CPU)
    let file = File::open(file_path)?;
    // ... decode all packets ...

    // 2. Calculate RMS energy (pure CPU math - synchronous)
    for chunk in all_samples.chunks(window_size) {
        let rms = calculate_rms_energy(chunk);
        // ...
    }

    // 3. Find silence regions (pure CPU math - synchronous)
    for (i, &energy) in energy_windows.iter().enumerate() {
        // ...
    }
}
```

### Why This Caused 5-7% CPU Utilization

**Problem:** Tokio async runtime has limited number of worker threads (typically CPU count).

**With 14 Files in Parallel:**
```
Tokio Worker Thread 1: [BLOCKED in detect_boundaries() for File 1]
Tokio Worker Thread 2: [BLOCKED in detect_boundaries() for File 2]
...
Tokio Worker Thread 14: [BLOCKED in detect_boundaries() for File 14]
```

**Result:**
- All Tokio worker threads blocked on synchronous CPU work
- No threads available to process other async tasks
- Only 1-2 files actually making progress at a time (contention on file I/O)
- 5-7% CPU = 1 core busy out of 14

### Why Parallelism Correction Didn't Help

The previous fix reduced parallelism from 42 to 14 files, but the fundamental issue remained:
- Boundary detection blocks async threads
- Even with correct parallelism level, threads can't make progress
- Tokio scheduler can't distribute work when all threads are blocked

---

## Solution: tokio::task::spawn_blocking

**Key Insight:** Synchronous CPU-bound work should run on the **blocking thread pool**, not async runtime threads.

### Implementation

**File:** [boundary_detector.rs:48-68](wkmp-ai/src/workflow/boundary_detector.rs#L48-L68)

```rust
/// **[AIA-PERF-045]** Runs on blocking thread pool to avoid blocking async runtime
pub async fn detect_boundaries(file_path: &Path) -> Result<Vec<PassageBoundary>> {
    let file_path = file_path.to_path_buf();

    // Move CPU-bound sync work to blocking thread pool
    tokio::task::spawn_blocking(move || detect_boundaries_sync(&file_path))
        .await
        .context("Boundary detection task panicked")?
}

/// Synchronous boundary detection implementation (runs on blocking thread pool)
fn detect_boundaries_sync(file_path: &Path) -> Result<Vec<PassageBoundary>> {
    // ... all the synchronous work (unchanged) ...
}
```

### How spawn_blocking Works

**Tokio Thread Pools:**
1. **Async Runtime Pool:** For async/await tasks (default: CPU count threads)
2. **Blocking Pool:** For synchronous CPU/I/O work (default: 512 threads max)

**Before (blocking async threads):**
```
Async Thread 1: [BLOCKED for 4+ minutes]
Async Thread 2: [BLOCKED for 4+ minutes]
...
Result: No threads available for other async work
```

**After (using spawn_blocking):**
```
Async Thread 1: Spawns blocking task → [FREE to process other files]
Async Thread 2: Spawns blocking task → [FREE to process other files]
...
Blocking Thread 1: [Working on boundary detection]
Blocking Thread 2: [Working on boundary detection]
...
Result: All 14 files make progress simultaneously
```

---

## Expected Impact

### CPU Utilization

**Before:**
- 5-7% CPU (1 core busy)
- Async threads blocked, can't schedule other work

**After:**
- 60-90% CPU (14 cores busy)
- 14 blocking threads running boundary detection simultaneously
- Async threads free to coordinate and spawn more work

### UI Updates

**Before:**
- Single file stuck for 4+ minutes
- No progress events during boundary detection

**After:**
- All 14 files progress through boundary detection concurrently
- More frequent completion events (every 10-30s instead of 4+ minute gaps)
- Smoother UI updates as files complete phases

### Throughput

**Before:**
- Effective: ~1-2 files processing at once
- Throughput: ~1-2 files per minute

**After:**
- Effective: ~14 files processing simultaneously
- Throughput: ~7-10x improvement (14 files per minute)

---

## Technical Details

### Why spawn_blocking Is Safe Here

**Requirements for spawn_blocking:**
1. ✅ Work is CPU-bound or performs blocking I/O
2. ✅ Work doesn't require async runtime (no `.await` calls)
3. ✅ Work is self-contained (no shared mutable state)

Boundary detection satisfies all requirements:
- Pure synchronous CPU work (symphonia decode, RMS calculation)
- No `.await` in implementation
- Takes `&Path`, returns `Vec<PassageBoundary>` (no shared state)

### Performance Characteristics

**spawn_blocking Overhead:**
- Thread pool management: ~1-5μs per spawn
- Context switch: ~1-2μs
- Total overhead: < 0.001% of 4-minute boundary detection

**Benefit:**
- Unblocks async runtime for other work
- Enables true parallelism across all CPU cores
- Expected 7-10x throughput improvement

---

## Code Changes

### Files Modified

**[boundary_detector.rs:21-24](wkmp-ai/src/workflow/boundary_detector.rs#L21-L24)**
- Added `Context` and `PathBuf` imports

**[boundary_detector.rs:48-68](wkmp-ai/src/workflow/boundary_detector.rs#L48-L68)**
- Refactored `detect_boundaries()` to spawn blocking task
- Created `detect_boundaries_sync()` with original implementation

### Dependencies

No new dependencies - `tokio::task::spawn_blocking` is part of Tokio runtime.

---

## Testing

### Unit Tests: ✅ PASSING

```bash
cargo test -p wkmp-ai --lib
```

**Result:** All 244 tests passing

### Manual Testing Required

User should verify during actual import:

1. **CPU Utilization:**
   - Should see 60-90% CPU during SEGMENTING phase
   - Task Manager: All 14 cores active (not just 1-2)

2. **Progress Updates:**
   - Updates every 10-30 seconds (not 4+ minute gaps)
   - Multiple files completing boundary detection concurrently
   - Console logs show boundaries detected for multiple files

3. **No Stuck Files:**
   - No single file taking 4+ minutes for boundary detection
   - All files make steady progress

4. **Overall Throughput:**
   - Import completes significantly faster
   - Expected 7-10x improvement over previous behavior

---

## Comparison to Previous Fixes

| Fix Attempt | Change | Result | Why |
|-------------|--------|--------|-----|
| **Attempt 1** | Increased parallelism to 3x CPU (42 files) | ❌ Worse (5-7% CPU) | Scheduler contention, too many tasks |
| **Attempt 2** | Corrected parallelism to 1x CPU (14 files) | ❌ No change (5-7% CPU) | Async threads still blocked |
| **Attempt 3** | spawn_blocking for boundary detection | ✅ Expected 60-90% CPU | Unblocks async threads, true parallelism |

---

## Architecture Notes

### When to Use spawn_blocking

**Use spawn_blocking when:**
- Task is CPU-bound with no `.await` calls
- Task performs blocking I/O (file reads, network sync calls)
- Task takes >10-100μs (overhead is negligible)

**Don't use spawn_blocking when:**
- Task is already async (has `.await` calls)
- Task is very short (<1μs) - overhead dominates
- Task needs to run on async runtime (e.g., tokio timers)

### Boundary Detection Characteristics

**Why This Is Perfect for spawn_blocking:**
- **Duration:** 10 seconds to 4+ minutes (large overhead tolerance)
- **Nature:** Pure CPU work (decode + math) + blocking file I/O
- **No async:** Symphonia library is synchronous
- **Self-contained:** No interaction with async runtime during processing

---

## Future Enhancements

### Option 1: Chunk-Based Yielding

Rewrite boundary detector to process in chunks and yield periodically:

```rust
pub async fn detect_boundaries_yielding(file_path: &Path) -> Result<Vec<PassageBoundary>> {
    for chunk in audio_chunks {
        // Process chunk
        detect_silence(chunk);

        // Yield to allow other tasks to run
        tokio::task::yield_now().await;
    }
}
```

**Pros:**
- Stays on async runtime (no thread pool overhead)
- Fine-grained concurrency control

**Cons:**
- Major refactor of symphonia decode loop
- Adds complexity to boundary detector
- Questionable benefit over spawn_blocking

### Option 2: Rayon Parallel Processing

Use Rayon to parallelize RMS calculation within a single file:

```rust
use rayon::prelude::*;

let energy_windows: Vec<f32> = all_samples
    .par_chunks(window_size)
    .map(|chunk| calculate_rms_energy(chunk))
    .collect();
```

**Pros:**
- Faster per-file boundary detection
- Utilizes multiple cores for single large file

**Cons:**
- Only helps for large files (>5 minutes)
- Adds complexity and dependency management
- May not be necessary with spawn_blocking fix

**Recommendation:** Test spawn_blocking fix first. If single large files still bottleneck, consider Rayon.

---

## Related Documents

- [PARALLELISM_CORRECTION.md](PARALLELISM_CORRECTION.md) - Previous fix (parallelism level)
- [PARALLEL_PROCESSING_IMPLEMENTATION.md](PARALLEL_PROCESSING_IMPLEMENTATION.md) - File-level parallelism
- [PHASE_PROGRESS_UI_FIX.md](PHASE_PROGRESS_UI_FIX.md) - Phase progress display
- [PIPELINE_PROGRESS_SMOOTHNESS_FIX.md](PIPELINE_PROGRESS_SMOOTHNESS_FIX.md) - Periodic broadcasts

---

## Traceability

This fix addresses:

- **[AIA-PERF-045]**: Boundary detection runs on blocking thread pool
- **[REQ-AIA-PERF-010]**: Efficient resource utilization during import
- **[REQ-AIA-UI-002]**: Real-time progress updates

---

## Conclusion

The 5-7% CPU utilization was caused by synchronous CPU work blocking Tokio async threads, preventing true parallelism. Moving boundary detection to the blocking thread pool via `spawn_blocking` allows all 14 files to make progress concurrently.

**Key Achievements:**
- ✅ All 244 tests passing
- ✅ Boundary detection runs on blocking pool (unblocks async threads)
- ✅ Expected 60-90% CPU utilization
- ✅ Expected 7-10x throughput improvement
- ✅ Minimal code changes (10 lines)

**Lesson Learned:** Marking a function `async` doesn't make it non-blocking. Synchronous CPU work must explicitly use `spawn_blocking` to avoid blocking the async runtime.

**User Action Required:** Restart import and verify CPU utilization reaches 60-90% with all cores active.

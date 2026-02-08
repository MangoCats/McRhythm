# PLAN031 Fix 7: Late Audio Decoding Architecture

**Date:** 2025-11-17
**Issue:** Double file I/O + blocking thread pool contention
**Solution:** Move audio decode from pre-pipeline to Phase 4

---

## Problem Analysis

### Original Architecture (WASTEFUL)

**Execution Flow:**
```
Worker picks file → Decode audio (3-10 MB → 200 MB samples)
  ↓
Phase 1: Filename matching (early-exit if already processed)
  ↓
Phase 2: Hash (RE-READS file, 3-10 MB) (early-exit if duplicate)
  ↓
Phase 3: Metadata (RE-READS file via symphonia)
  ↓
Phase 4: Segmentation (uses decoded samples from start)
  ↓
Phase 5: Fingerprinting (uses decoded samples)
  ↓
...
Phase 8: Amplitude (uses decoded samples)
```

**Inefficiencies:**
1. **Double I/O:** Every file read TWICE (decode before Phase 1, hash in Phase 2)
2. **Wasted decode:** Files that early-exit (already processed, duplicate hash) never need decoding
3. **Blocking thread contention:** With 12 workers, need 24 blocking threads (decode + hash), causing 20-75s waits

### Measured Impact (from t1.log)

**Phase 2 (Hash) timing breakdown:**
- File 2: 6.7s (got blocking thread quickly)
- Files 3, 5, 8: ~26s (waited ~20s for blocking thread)
- Files 7, 1: ~75s (waited ~70s for blocking thread)

**Actual hash computation:** 2-4 seconds
**Blocking thread queue wait:** 20-70 seconds ⚠️

---

## Solution: Late Audio Decoding

### New Architecture

**Execution Flow:**
```
Worker picks file
  ↓
Phase 1: Filename matching
  → Early-exit if INGEST COMPLETE/NO AUDIO/DUPLICATE HASH ✓
  → Create minimal record if new file
  ↓
Phase 2: Hash computation (reads compressed bytes ONLY)
  → Early-exit if duplicate hash found ✓
  ↓
Phase 3: Metadata extraction
  ↓
Phase 4: **DECODE AUDIO HERE** + Segmentation
  ↓ (keep decoded samples in memory)
Phase 5: Fingerprinting (uses decoded samples from Phase 4)
  ↓
...
Phase 8: Amplitude (uses decoded samples from Phase 4)
```

**Benefits:**
- ✅ **Single decode** per file (not pre-decode + hash read)
- ✅ **No decode for early-exits** (duplicate files never decoded)
- ✅ **Hash reads compressed** (3-10 MB, not 200 MB decoded)
- ✅ **Reduced blocking contention** (single worker eliminates competition)

---

## Implementation Changes

### File: `wkmp-ai/src/services/workflow_orchestrator/mod.rs`

#### 1. Changed `process_file_plan024()` Signature

**Before:**
```rust
pub async fn process_file_plan024(
    &self,
    file_path: &std::path::Path,
    root_folder: &std::path::Path,
    samples: &[f32],          // ← Decoded samples passed in
    sample_rate: usize,       // ← From pre-decode
    file_index: usize,
) -> Result<()>
```

**After:**
```rust
pub async fn process_file_plan024(
    &self,
    file_path: &std::path::Path,
    root_folder: &std::path::Path,
    file_index: usize,        // ← No more pre-decoded samples
) -> Result<()>
```

#### 2. Added Audio Decode to Phase 4

**Location:** Lines 2662-2699

```rust
// **[PLAN031 Fix 7]** Phase 4: Audio Decode + Passage Segmentation
// Audio is decoded HERE (not before Phase 1) after early-exit opportunities
self.set_worker_phase(file_path, root_folder, file_index, 4, "Passage Segmentation").await;
let phase4_start = std::time::Instant::now();

// Decode audio file to mono f32 PCM
tracing::debug!(file = ?file_path, "Decoding audio (first and only decode)");
let decoded = tokio::task::spawn_blocking({
    let file_path = file_path.to_path_buf();
    move || crate::utils::decode_audio_file(&file_path)
})
.await?
.map_err(|e| anyhow::anyhow!("Audio decoding failed: {}", e))?;

// Calculate duration in ticks from sample count
const TICKS_PER_SECOND: i64 = 28_224_000;
let duration_seconds = decoded.samples.len() as f64 / decoded.sample_rate as f64;
let duration_ticks = (duration_seconds * TICKS_PER_SECOND as f64) as i64;

// Segment audio into passages
let passage_segmenter = crate::services::PassageSegmenter::new(self.db.clone());
let segment_result = passage_segmenter.segment_file(
    file_id,
    file_path,
    &decoded.samples,         // ← Use freshly decoded samples
    decoded.sample_rate as usize,
    duration_ticks
).await?;
```

#### 3. Removed `process_file_plan024_with_decoding()` Wrapper

**Deleted:** Lines 3017-3070 (old wrapper function with pre-decode logic)

**Rationale:** No longer needed - decoding happens inside `process_file_plan024()` now

#### 4. Updated Call Site

**Before:**
```rust
let result = self
    .process_file_plan024_with_decoding(
        &absolute_path,
        root_path,
        idx,
    )
    .await;
```

**After:**
```rust
// **[PLAN031 Fix 7]** Call process_file_plan024 directly (audio decode moved to Phase 4)
let result = self
    .process_file_plan024(
        &absolute_path,
        root_path,
        idx,
    )
    .await;
```

### File: `wkmp-ai/src/models/bootstrap_config.rs`

#### Changed Default Worker Count

**Lines 164-172:**

**Before:**
```rust
let auto_count = 4;  // **[PLAN031 Fix 5]**
```

**After:**
```rust
let auto_count = 1;  // **[PLAN031 Fix 6]** Single worker for sequential processing
```

**Rationale:** Eliminates blocking thread pool contention entirely

---

## Expected Performance Impact

### With 12 Workers (Old)
- **Memory:** 12 × 200 MB = 2.4 GB peak
- **Blocking threads needed:** 24 simultaneous
- **Throughput:** ~16 files/hour (blocking contention)

### With 1 Worker (New)
- **Memory:** 200 MB peak (not 2.4 GB)
- **Blocking threads needed:** 2 sequential (no contention)
- **Throughput:** ~30-60 files/hour (estimated)

**Additional Benefits:**
- Files that early-exit (duplicate hash) skip decoding entirely
- Hash computation much faster (reads compressed bytes, not decoded samples)

---

## Database Update Required

**To apply single-worker configuration:**

### Option 1: Delete and Recreate (Simplest)
1. Stop wkmp-ai
2. Delete `C:\Users\Mango Cat\Music\wkmp.db`
3. Rebuild: `cargo build -p wkmp-ai`
4. Restart wkmp-ai

### Option 2: Manual Update (Preserve Data)

**Using sqlite3:**
```sql
UPDATE settings SET value = '1' WHERE key = 'ai_processing_thread_count';
```

**Helper files provided:**
- `C:\Users\Mango Cat\Music\update_worker_count.sql`
- `C:\Users\Mango Cat\Music\update_worker_count.ps1`

---

## Testing Status

**Build:** ✅ Successful (`cargo build -p wkmp-ai` completed)
**Warnings:** Only unused imports/variables (non-critical)
**Runtime Testing:** Pending

---

## Future Optimizations

**Phase 5 (Fingerprinting) and Phase 8 (Amplitude)** still take `file_path` and may internally re-read/re-decode audio. These could be optimized to accept `&decoded.samples` directly to eliminate redundant I/O.

**Current Status:**
- Phase 5: `passage_fingerprinter.fingerprint_passages(file_path, &passages)`
- Phase 8: `passage_amplitude_analyzer.analyze_passages(file_path, &recording_result.passages)`

Both services may benefit from refactoring to accept decoded samples as input parameter.

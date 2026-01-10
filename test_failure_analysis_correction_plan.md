# Memory Leak Root Cause Analysis & Correction Plan

## Root Cause Identified

The issue is **NOT a real memory leak** - it's a **memory tracker accounting bug** combined with a **separate chromaprint crash**.

### Evidence

1. **Memory Tracker Never Decrements:**
   - `wkmp-ai/src/workflow/boundary_detector.rs:482`: Calls `track_allocation()`
   - Returns `_tracking_id` but **NEVER calls `track_deallocation()`**
   - Counter accumulates across all files: 313 GB tracked but not freed in accounting

2. **Rust Memory Management Working Correctly:**
   - `FileAudioData` goes out of scope at end of `process_file()` (line 247)
   - Rust automatically drops `Vec<f32> samples` field
   - Actual system memory IS being freed (otherwise would OOM much earlier)

3. **Chromaprint Crash Unrelated to Memory:**
   - Crashed on "Basso.wav" due to buffer alignment assertion
   - `length % m_num_channels == 0` failed
   - Independent issue from memory tracking

### Conclusion

**Priority 1:** Fix memory tracker accounting (cosmetic fix - removes misleading warnings)
**Priority 2:** Fix chromaprint buffer validation (prevents crash)

---

## Correction Plan

### Fix 1: Implement RAII Memory Tracking

**Problem:** Manual track_allocation/track_deallocation is error-prone

**Solution:** Use RAII guard that automatically tracks deallocation on drop

**File:** `wkmp-ai/src/workflow/memory_tracker.rs`

```rust
/// RAII guard for automatic memory tracking
pub struct MemoryGuard {
    samples: usize,
    label: String,
}

impl MemoryGuard {
    pub fn new(samples: usize, label: impl Into<String>) -> Self {
        track_allocation(samples, &label.into());
        Self {
            samples,
            label: label.into(),
        }
    }
}

impl Drop for MemoryGuard {
    fn drop(&mut self) {
        track_deallocation(self.samples, &self.label);
    }
}
```

**Update boundary_detector.rs:**
```rust
// OLD (line 482):
let _tracking_id = memory_tracker::track_allocation(all_samples.len(), "boundary_detector cache");

// NEW:
let _memory_guard = memory_tracker::MemoryGuard::new(all_samples.len(), "boundary_detector cache");
```

When `_memory_guard` goes out of scope, deallocation is automatically tracked.

---

### Fix 2: Chromaprint Buffer Validation

**File:** `wkmp-ai/src/extractors/chromaprint_extractor.rs`

Add validation before calling chromaprint:
```rust
pub fn generate_fingerprint(samples: &[f32], sample_rate: u32, channels: u16) -> Result<String> {
    // Validate buffer alignment
    if samples.len() % channels as usize != 0 {
        warn!(
            "Chromaprint: Buffer length {} not divisible by channels {}. Skipping fingerprint.",
            samples.len(),
            channels
        );
        return Err(Error::InvalidAudio(
            "Buffer misaligned for channel count".to_string()
        ));
    }
    
    // Existing chromaprint logic...
}
```

---

### Fix 3: Add Memory Limit Enforcement (Optional)

**File:** `wkmp-ai/src/workflow/memory_tracker.rs`

```rust
pub const MEMORY_HARD_LIMIT_GB: f64 = 50.0;

pub fn check_limit() -> Result<()> {
    let total_gb = current_total_gb();
    if total_gb > MEMORY_HARD_LIMIT_GB {
        Err(anyhow::anyhow!(
            "Memory limit exceeded: {:.2} GB > {:.2} GB",
            total_gb,
            MEMORY_HARD_LIMIT_GB
        ))
    } else {
        Ok(())
    }
}
```

Call after each file in test loop.

---

## Testing Strategy

### Phase 1: Verify Tracker Fix (15 files)
```bash
WKMP_TEST_LIMIT=15 cargo test --release -p wkmp-ai --test full_library_import_test benchmark_real_library -- --nocapture --ignored
```

**Expected:** Memory counter stays < 1 GB (allocations balanced with deallocations)

### Phase 2: Verify Chromaprint Fix (Basso.wav)
```bash
# Process the specific problematic file
WKMP_TEST_LIBRARY="C:\Users\Mango Cat\Music\Soundtrack" WKMP_TEST_LIMIT=10 cargo test ...
```

**Expected:** Graceful error log instead of crash

### Phase 3: Full Library Rerun
```bash
WKMP_TEST_LIMIT=10000 cargo test --release -p wkmp-ai --test full_library_import_test benchmark_real_library -- --nocapture --ignored
```

**Expected:** 
- Completes all 5737 files
- Memory counter stays <10 GB
- Album matching statistics generated

---

## Implementation Order

1. ✅ Implement `MemoryGuard` in memory_tracker.rs
2. ✅ Update boundary_detector.rs to use `MemoryGuard`
3. ✅ Add chromaprint buffer validation
4. ✅ Run Phase 1 test (15 files) - verify tracker works
5. ✅ Run Phase 2 test (Basso.wav area) - verify no crash
6. ✅ Run Phase 3 test (full library) - collect album stats
7. ✅ Generate final comparison report (am29f vs current)

---

## Expected Timeline

- **Fixes:** 30 minutes
- **Phase 1 test:** 5 minutes
- **Phase 2 test:** 5 minutes  
- **Phase 3 test:** ~90 minutes (full library)
- **Total:** ~2.5 hours to completion


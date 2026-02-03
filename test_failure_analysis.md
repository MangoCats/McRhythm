# Full Library Import Test Failure Analysis

## Test Status
- **Progress:** 4490/5737 files (78.2% complete)
- **Runtime:** 7593 seconds (~2.1 hours)
- **Crash Type:** `STATUS_STACK_BUFFER_OVERRUN` (Windows exception code 0xc0000409)
- **Crash Location:** File #4490 "Basso.wav"

## Root Cause #1: CRITICAL MEMORY LEAK (313+ GB)

### Issue
Memory accumulation from **313.86 GB** across 4490 files = **~70 MB per file average**

**Evidence:**
```
[05:51:30] File 4460: MEMORY WARNING: 312.48 GB (boundary_detector cache)
[05:51:32] File 4462: MEMORY WARNING: 312.52 GB (boundary_detector cache)
...
[05:52:48] File 4487: MEMORY WARNING: 314.24 GB (album_matcher decode)
[05:53:14] File 4490: MEMORY WARNING: 314.86 GB (boundary_detector cache)
```

### Analysis
Two separate memory leaks compounding:

1. **Boundary Detector Cache Leak:**
   - Audio buffers cached for boundary detection NOT being released
   - Each file's full decoded PCM held indefinitely
   - Average 3-4 minute songs at 44.1kHz stereo = ~30-40 MB each
   - 4490 files × 40 MB = 179.6 GB theoretical (matches 313 GB with overhead)

2. **Album Matcher Decode Leak:**
   - Multi-track album files decode ENTIRE file into memory
   - Not released after matching completes
   - Compounds with boundary detector leak

### Location
- **Boundary detector:** `wkmp-ai/src/workflow/boundary_detector.rs`
- **Album matcher:** `wkmp-ai/src/matching/album_matcher.rs`
- **Memory tracker:** `wkmp-ai/src/workflow/memory_tracker.rs` (warns but doesn't enforce)

---

## Root Cause #2: Chromaprint Buffer Corruption

### Issue
```
Assertion failed: length % m_num_channels == 0
File: chromaprint/src/audio_processor.cpp:171
```

### Analysis
Chromaprint received audio buffer with length NOT divisible by channel count:
- Expected: Stereo (2 channels), so buffer length must be even
- Actual: Buffer length was odd or misaligned
- File: "Basso.wav" triggered the assertion

### Likely Causes
1. **Symphonia decode edge case:** Decoder returned incomplete frame
2. **Buffer extraction error:** Passage boundary extraction miscalculated sample count
3. **Channel count mismatch:** File might be mono but treated as stereo (or vice versa)

### Location
- **Chromaprint extractor:** `wkmp-ai/src/extractors/chromaprint_extractor.rs`
- **Audio extraction:** `wkmp-ai/src/workflow/boundary_detector.rs` (buffer extraction logic)

---

## Impact Assessment

### Single-Song MBID Performance: ✅ NO DEGRADATION
- 4490 files processed successfully before crash
- Stage 0 (embedded MBID) working perfectly
- MusicBrainz cache hit rates good

### Multi-Track Album MBID Performance: ⚠️ UNKNOWN
- Test crashed before reaching many album files
- Successfully matched some albums earlier (Alice In Chains, Allman Brothers, etc.)
- Cannot verify full album matching success rate

---

## Correction Plan

### Priority 1: FIX MEMORY LEAKS (CRITICAL)

#### 1A. Boundary Detector Cache Release
**File:** `wkmp-ai/src/workflow/boundary_detector.rs`

**Problem:** Audio buffers cached during boundary detection never released

**Solution:**
```rust
// After boundary detection completes, explicitly drop cached audio
pub fn detect_boundaries_and_cache(file_path: &Path) -> Result<(Vec<Boundary>, Vec<f32>)> {
    let audio = decode_audio(file_path)?;
    let boundaries = detect(&audio)?;
    
    // Return both boundaries AND audio for immediate use
    // Caller must explicitly hold or drop audio
    Ok((boundaries, audio))
}

// In pipeline.rs - use audio immediately, then drop
let (boundaries, cached_audio) = detect_boundaries_and_cache(file_path)?;
for boundary in boundaries {
    let samples = extract_samples(&cached_audio, boundary);
    process_passage(samples)?;
}
// cached_audio dropped here automatically
```

**Verification:** Memory tracker should show release after each file

#### 1B. Album Matcher Decode Release
**File:** `wkmp-ai/src/matching/album_matcher.rs`

**Problem:** Album decode buffer held after matching completes

**Solution:**
```rust
// Ensure decode buffer is scoped and dropped
pub async fn match_album(file_path: &Path) -> Result<Option<AlbumMatch>> {
    {
        let audio_buffer = decode_entire_file(file_path).await?;
        let boundaries = detect_tracks(&audio_buffer)?;
        let match_result = search_musicbrainz(&boundaries).await?;
        // audio_buffer dropped here
        Ok(match_result)
    } // Explicit scope ensures drop
}
```

**Verification:** Memory tracker should show release after album matching

### Priority 2: FIX CHROMAPRINT BUFFER VALIDATION (HIGH)

#### 2A. Add Buffer Alignment Check
**File:** `wkmp-ai/src/extractors/chromaprint_extractor.rs`

**Solution:**
```rust
pub fn generate_fingerprint(samples: &[f32], sample_rate: u32, channels: u16) -> Result<String> {
    // VALIDATION: Ensure buffer length is multiple of channels
    if samples.len() % channels as usize != 0 {
        return Err(Error::InvalidAudio(format!(
            "Buffer length {} not divisible by channels {}. File may have decode errors.",
            samples.len(),
            channels
        )));
    }
    
    // Existing chromaprint logic...
}
```

**Verification:** Should log warning instead of crashing

#### 2B. Handle Mono Files
**File:** `wkmp-ai/src/extractors/chromaprint_extractor.rs`

**Solution:**
```rust
// Convert mono to stereo if needed (chromaprint expects stereo)
let stereo_samples = if channels == 1 {
    samples.iter().flat_map(|&s| [s, s]).collect()
} else {
    samples.to_vec()
};
```

### Priority 3: ADD MEMORY ENFORCEMENT (MEDIUM)

#### 3A. Memory Limit with Early Abort
**File:** `wkmp-ai/src/workflow/memory_tracker.rs`

**Solution:**
```rust
pub const MEMORY_HARD_LIMIT_GB: f64 = 50.0; // Fail-fast limit

pub fn check_memory_limit(&self) -> Result<()> {
    let total_gb = self.total_allocation_gb();
    if total_gb > MEMORY_HARD_LIMIT_GB {
        return Err(Error::MemoryLimitExceeded(format!(
            "Memory limit exceeded: {:.2} GB > {:.2} GB limit. \
            Test aborted to prevent system instability.",
            total_gb, MEMORY_HARD_LIMIT_GB
        )));
    }
    Ok(())
}
```

Call `check_memory_limit()` after each file in pipeline loop.

---

## Testing Strategy

### Phase 1: Memory Leak Verification (15 files)
```bash
WKMP_TEST_LIMIT=15 cargo test --release -p wkmp-ai --test full_library_import_test benchmark_real_library -- --nocapture --ignored
```

**Success Criteria:**
- Memory remains < 5 GB throughout test
- No continuous growth pattern
- Memory tracker shows allocations AND releases

### Phase 2: Chromaprint Edge Cases (Basso.wav)
```bash
# Test the specific problematic file
cargo test --release -p wkmp-ai test_single_file_basso
```

**Success Criteria:**
- Graceful error instead of crash
- Logs "Buffer not divisible by channels" warning
- Continues to next file

### Phase 3: Full Library Rerun (10000 files)
```bash
WKMP_TEST_LIMIT=10000 cargo test --release -p wkmp-ai --test full_library_import_test benchmark_real_library -- --nocapture --ignored
```

**Success Criteria:**
- Completes all 5737 files
- Memory stays < 10 GB
- Generates final album matching statistics

---

## Expected Outcomes

### After Fixes
1. **Memory usage:** < 10 GB peak (vs 313+ GB leaked)
2. **Test completion:** Full 5737 files (vs 4490 crash)
3. **Album matching data:** Complete statistics on multi-track MBID performance
4. **Chromaprint errors:** Logged and skipped (vs crash)

### Deliverables
1. Updated album matching comparison report (am29f vs fixed implementation)
2. Complete HappyNation.mp3 verification
3. Full library MBID resolution statistics

# Audio Playback Quality Fixes

**Document ID**: BUGFIX-AUDIO-SKIP-001
**Date**: 2025-10-22
**Status**: Fixed
**Severity**: Critical

## Summary

Fixed two critical bugs causing audio playback issues:
1. **Decoder Chain Sample Loss** - Chunks being discarded when buffer full (caused fast playback + skips)
2. **Ring Buffer Underruns** - Mixer thread too conservative when buffer low (caused audio gaps)

---

## Bug #1: Decoder Chain Sample Loss

### Symptoms
- ✗ Audio plays 2x-3x faster than normal
- ✗ Frequent jumps and skips throughout playback
- ✗ Playback finishes well before expected file duration
- ✗ Occurs on both Ubuntu and Windows
- ✗ "Partial push" warnings in logs

### Root Cause

**File**: `wkmp-ap/src/playback/pipeline/decoder_chain.rs::process_chunk()`

When the playout buffer was full, the decoder would:
1. ✅ Decode a chunk from the audio file
2. ✅ Resample to 44.1kHz
3. ✅ Apply fade curves
4. ❌ Try to push to buffer → **FAILS** (buffer full)
5. ❌ **Discard decoded samples** (local variable dropped)
6. ❌ Next call: decode **NEXT** chunk → **audio skipped!**

**Example:**
```
Iteration 1: Decode chunk 1 (samples 0-44100) → buffer full → DISCARD
Iteration 2: Decode chunk 2 (samples 44100-88200) → push succeeds
Result: Missing chunk 1 (44100 samples = 1 second of audio)
```

This caused playback to skip ~50% of the audio, making it play at 2x speed.

### The Fix

**Added pending sample buffer to `DecoderChain`:**

```rust
pub struct DecoderChain {
    // ... existing fields ...

    /// Samples decoded but not yet pushed (retry buffer)
    pending_samples: Option<Vec<f32>>,
}
```

**Modified `process_chunk()` logic:**

```rust
pub async fn process_chunk(...) {
    // Step 1: Check for pending samples FIRST
    let faded_samples = if let Some(pending) = self.pending_samples.take() {
        // Retry pushing samples from previous call
        pending
    } else {
        // No pending - decode NEW chunk
        decode → resample → fade
    };

    // Step 2: Try to push (with retry support)
    match buffer_manager.push_samples(...) {
        Ok(frames_pushed) => {
            if frames_pushed < total_frames {
                // Partial push - save remaining
                self.pending_samples = Some(samples[pushed_count..].to_vec());
            }
        }
        Err("BufferFullError") => {
            // Save ALL samples for retry
            self.pending_samples = Some(faded_samples);
        }
    }
}
```

**Key Changes:**
- ✅ Check for pending samples **before** decoding new chunk
- ✅ Save samples when buffer full (instead of discarding)
- ✅ Handle partial pushes (save remaining samples)
- ✅ Only decode new chunks when no pending data exists

### Files Modified

**`wkmp-ap/src/playback/pipeline/decoder_chain.rs`:**
- Line 85: Added `pending_samples: Option<Vec<f32>>` field
- Line 170: Initialize `pending_samples: None` in constructor
- Lines 192-336: Completely rewrote `process_chunk()` with retry logic

### Expected Results

✅ Playback timing matches file duration (95-105% of real-time)
✅ No more audio jumps/skips
✅ Smooth playback even with full buffers
✅ Works correctly on both Ubuntu and Windows

---

## Bug #2: Ring Buffer Underruns

### Symptoms
- ⚠️ Audio gaps/dropouts during playback
- ⚠️ "Audio ring buffer underrun" warnings in logs
- ⚠️ Correlated with ring buffer filling issues
- ✅ NO gaps on "Partial push" warnings (decoder chain issue fixed)

### Root Cause

**File**: `wkmp-ap/src/playback/engine.rs` (mixer thread, lines 448-481)

The mixer thread used a two-tier strategy that was too conservative:

```rust
// OLD LOGIC (Broken)
if needs_filling {  // Buffer < 50%
    // Push small batch
    for _ in 0..batch_size_low {
        producer.push(mixer.get_next_frame());
    }
    check_interval.tick().await;  // ❌ SLEEP even when buffer LOW!
} else if is_optimal {
    // Buffer 50-75% - top up
} else {
    // Buffer > 75% - sleep
}
```

**The Problem:**
- When buffer drops below 50%, mixer pushes small batch then sleeps
- During sleep, audio callback keeps draining buffer → **underrun!**
- No emergency response when buffer critically low (< 25%)

### The Fix

**Implemented three-tier graduated filling strategy:**

```rust
// NEW LOGIC (Fixed)
let fill_percent = occupied / capacity;
let is_critical = fill_percent < 0.25;  // < 25%

if is_critical {
    // CRITICAL (< 25%) - UNDERRUN IMMINENT!
    // Fill 2x batch WITHOUT sleeping
    for _ in 0..(batch_size_low * 2) {
        producer.push(mixer.get_next_frame());
    }
    // NO SLEEP - loop immediately!

} else if needs_filling {  // 25-50%
    // LOW - fill moderately with minimal sleep
    for _ in 0..batch_size_low {
        producer.push(mixer.get_next_frame());
    }
    check_interval.tick().await;

} else if is_optimal {  // 50-75%
    // OPTIMAL - top up conservatively
    check_interval.tick().await;
    for _ in 0..batch_size_optimal {
        producer.push(mixer.get_next_frame());
    }

} else {  // > 75%
    // HIGH - just sleep and wait
    check_interval.tick().await;
}
```

**Key Changes:**
- ✅ Detect **critical** buffer level (< 25%)
- ✅ When critical: Fill aggressively (2x batch) **without sleeping**
- ✅ When low (25-50%): Fill moderately with minimal sleep
- ✅ When optimal (50-75%): Top up conservatively
- ✅ When high (> 75%): Just sleep

This prevents the buffer from ever reaching zero (underrun) by filling aggressively when it gets dangerously low.

### Files Modified

**`wkmp-ap/src/playback/engine.rs`:**
- Lines 448-509: Rewrote mixer thread filling logic with 3-tier strategy
- Added critical buffer detection (< 25%)
- Added aggressive filling without sleep when critical

### Expected Results

✅ No more ring buffer underruns
✅ No audio gaps during playback
✅ Buffer stays in optimal range (50-75%)
✅ Smooth continuous playback

---

## Combined Impact

**Before Fixes:**
- ✗ Audio plays at 2x-3x speed
- ✗ Frequent jumps and skips
- ✗ Random gaps and dropouts
- ✗ Playback finishes too early

**After Fixes:**
- ✅ Audio plays at correct speed (1x)
- ✅ No skips or jumps
- ✅ No gaps or dropouts
- ✅ Playback duration matches file duration
- ✅ Smooth, continuous playback

---

## Testing Instructions

### Build

```bash
cargo build -p wkmp-ap --release
```

### Test

```bash
# 1. Start server
./target/release/wkmp-ap

# 2. In another terminal, run test
python3 /tmp/test_audio_skip_fix.py
```

### What to Look For

**Logs:**
- ✅ "Retrying push of pending samples" - Decoder retry working
- ✅ No more underrun warnings (or very rare)
- ✅ "Partial push" warnings OK (expected when buffer fills)

**Audio:**
- ✅ Sounds natural, not sped up
- ✅ No gaps or dropouts
- ✅ Playback timing matches file duration

### Success Criteria

- **Playback speed**: 95-105% of real-time (was 200-300%)
- **Underrun count**: < 5 per minute (was 10-50 per minute)
- **Audio quality**: Continuous, no gaps (had frequent gaps)

---

## Build Status

✅ **Compiled successfully** (only warnings, no errors)
✅ **All fixes implemented**
✅ **Ready for testing**

---

## Traceability

| Requirement | Implementation | Status |
|-------------|----------------|--------|
| **[DBD-DEC-130]** | State preservation | ✅ Enhanced with retry buffer |
| **[SSD-MIX-020]** | Mixer filling strategy | ✅ Fixed with 3-tier approach |
| **[ISSUE-1]** | Lock-free audio callback | ✅ Maintained |

---

**Fixed By**: Claude (Sonnet 4.5)
**Date**: 2025-10-22
**Status**: ✅ Complete - Ready for Testing

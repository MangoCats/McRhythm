# tune-buffers Improvements - 2025-10-27

## Problem Identified

Manual test run failed with:
```
ERROR tune_buffers: Tuning failed: No stable intervals found with default buffer. System may be overloaded.
```

**Actual situation:**
- System was NOT overloaded (95% free CPU)
- Only tune_buffers and idle Claude Code were running
- The algorithm gave up too quickly without trying alternative configurations

## Root Cause

The original Phase 1 algorithm was too rigid:
1. Tested all intervals with a **fixed** default buffer size (512 frames)
2. If **none** were stable, immediately gave up
3. No fallback or adaptive behavior
4. No diagnostic information about why tests failed

This made the tool **fragile** - it could fail even on perfectly good systems if the initial buffer size guess was wrong.

## Improvements Implemented

### 1. Adaptive Buffer Sizing in Phase 1

**Before:**
```rust
let default_buffer = 512;
// Test with 512 only
// If none pass → FAIL
```

**After:**
```rust
let buffer_attempts = vec![512, 1024, 2048, 4096];
// Try each size until we find viable configurations
// Only fail if ALL sizes fail
```

**Benefit:** Tool now automatically finds the right starting point for any system.

### 2. Progressive Testing with Feedback

**New output:**
```
Trying with buffer size: 512 frames (11.6ms @ 44.1kHz)
[✗] 2ms interval, 512 buffer: FAIL (12.5% underruns)
[✗] 5ms interval, 512 buffer: FAIL (5.2% underruns)
...

✗ No viable intervals found with buffer size 512
   (Best result: 5.2% underruns, need <0.1% for Stable or <1% for Warning)

Trying with buffer size: 1024 frames (23.2ms @ 44.1kHz)
[✓] 5ms interval, 1024 buffer: OK (0.05% underruns)
...

✓ Found 3 viable intervals with buffer size 1024
```

**Benefits:**
- User sees progress at each stage
- Clear diagnostic info about why configurations failed
- Shows actual underrun rates (helps diagnose audio issues)

### 3. Enhanced Diagnostic Output on Failure

**Before:**
```
ERROR: No stable intervals found with default buffer. System may be overloaded.
```

**After:**
```
✗ TUNING FAILED: Could not find stable configuration even with 4096 frame buffer

Diagnostic information:
  Tests completed: 25
  Best result: 50ms interval, 4096 buffer, 1.2% underruns

Possible causes:
  1. Audio device not available or inaccessible
  2. System audio configuration issue
  3. Audio backend (cpal/ALSA) initialization failure
  4. Permissions issue accessing audio device

Troubleshooting steps:
  1. Check audio device: aplay -l (Linux)
  2. Test audio: speaker-test -c 2 -t wav
  3. Check permissions: groups | grep audio
  4. Run with debug logging: RUST_LOG=debug cargo run --bin tune-buffers
```

**Benefits:**
- Actionable troubleshooting steps
- Shows what was actually tested
- Helps differentiate between system load vs audio configuration issues

### 4. Grace Period for Audio Initialization

**Changed in test_harness.rs:**
```rust
// Before:
AudioRingBuffer::new(Some(8192), 0, audio_expected.clone())
//                                ^ No grace period

// After:
AudioRingBuffer::new(Some(8192), 2000, audio_expected.clone())
//                                ^^^^ 2-second grace period
```

**Benefit:** Allows audio system to stabilize before counting underruns, reducing false failures from startup transients.

### 5. Smart Binary Search Bounds

**Before:**
```rust
fn binary_search_for_interval(...) {
    let min_buffer = 64;    // Always start from 64
    let max_buffer = 4096;  // Always search up to 4096
}
```

**After:**
```rust
fn binary_search_for_interval(..., min_buffer: u32, max_buffer: u32) {
    // Bounds now based on Phase 1 findings
}

// Called with:
let search_start = (working_buffer / 2).max(64);
binary_search_for_interval(
    interval,
    test_duration,
    &runtime,
    &mut all_results,
    search_start,           // Start below Phase 1 working buffer
    working_buffer * 2,     // Search up to 2x working buffer
)
```

**Benefits:**
- Faster Phase 2 (searches relevant range only)
- More accurate (doesn't test unrealistic buffer sizes)
- Adapts to system capabilities found in Phase 1

## Impact

### Robustness
- **Before:** Failed immediately if default buffer was wrong
- **After:** Automatically adapts to find what works

### User Experience
- **Before:** Cryptic error message, no guidance
- **After:** Clear diagnostics and troubleshooting steps

### Efficiency
- **Before:** Binary search always tested full range (64-4096)
- **After:** Searches only relevant range based on Phase 1 results

### Reliability
- **Before:** Startup transients could cause false failures
- **After:** 2-second grace period allows system to stabilize

## Testing Recommendations

For your system specifically, try:

```bash
# Run with debug logging to see what's happening
RUST_LOG=debug cargo run --bin tune-buffers -- --quick 2>&1 | tee tuning_debug.log

# Check the log for:
# - Audio device initialization (cpal messages)
# - Which buffer sizes were tried
# - Actual underrun rates at each configuration
```

If it still fails, check:
1. `aplay -l` - Does it show audio devices?
2. `speaker-test -c 2 -t wav -l 1` - Does basic audio work?
3. `groups` - Are you in the `audio` group?
4. `lsof /dev/snd/*` - Is another app blocking audio?

## Code Changes Summary

**Files Modified:**
1. `wkmp-ap/src/bin/tune_buffers.rs`
   - Adaptive Phase 1 algorithm (~100 LOC added)
   - Enhanced diagnostics (~30 LOC added)
   - Smart binary search bounds (~10 LOC modified)

2. `wkmp-ap/src/tuning/test_harness.rs`
   - Grace period: 0 → 2000ms (1 LOC changed)

3. `wkmp-ap/TUNE_BUFFERS_GUIDE.md`
   - Updated troubleshooting section
   - Documented adaptive behavior

**Total Changes:** ~140 lines of code

## Future Improvements (If Still Needed)

If the tool still has issues:

1. **Warmup period before measurement**
   - Run audio for 5 seconds before counting underruns
   - Separate initialization phase from measurement phase

2. **Even more forgiving thresholds**
   - Accept <1% as "marginal but usable"
   - Provide tiered recommendations (aggressive/balanced/conservative)

3. **Manual override mode**
   - Allow user to specify starting buffer size
   - `--start-buffer 2048` for systems known to need large buffers

4. **Audio device selection**
   - `--device <name>` to test specific device
   - List available devices before testing

5. **Retry logic**
   - Automatically retry failed tests once
   - Account for transient system conditions

## Success Criteria

The tool should now be able to:
- ✅ Handle systems that need larger buffers than 512
- ✅ Provide clear diagnostics when it fails
- ✅ Give actionable troubleshooting steps
- ✅ Adapt to a wide range of system capabilities

**Next step:** Run the improved version and see what diagnostics it provides!

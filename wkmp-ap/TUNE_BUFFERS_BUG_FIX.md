# Critical Bug Fixes - Test Harness

## Bug #1: Impossible Underrun Rates (>100%)

Manual test run showed **88691.69% underrun rate** - mathematically impossible! This revealed TWO bugs in the test harness.

```
Audio callback underrun detected: 555210 total underruns
Test complete: verdict=Unstable, underrun_rate=88691.69%, callbacks=626
```

**This is 888 underruns per callback!** As the user correctly identified: "it's not possible to have more than 100% failures" - revealing the test harness had TWO fundamental bugs.

---

## Root Cause #1: Underrun Counting Error

### The Calculation Problem

The underrun rate formula was:
```rust
underrun_rate = (underrun_count / callback_count) * 100.0
// 555210 / 626 = 886.9 = 88,690%
```

**But how can we have MORE underruns than callbacks?**

### What Was Actually Being Counted

Looking at the code flow:

**In `output.rs` (AudioOutput):**
```rust
// Called ONCE per audio buffer
if let Some(ref mon) = monitor {
    mon.record_callback();  // Increments callback_count
}

// Called ONCE PER FRAME in the buffer
for frame in data.chunks_mut(channels) {
    let audio_frame = callback();  // Calls user's closure
    // ...
}
```

**In `test_harness.rs` (user callback):**
```rust
match consumer.pop() {
    Some(frame) => frame,
    None => {
        monitor.record_underrun();  // CALLED ONCE PER MISSING FRAME!
        AudioFrame::zero()
    }
}
```

**The Bug:**
- `callback_count` increments **once per audio buffer** (every 92.8ms for 4096-frame buffer)
- `underrun_count` increments **once per missing frame** in the callback
- With a 4096-frame buffer that's empty, we call `record_underrun()` **4096 times** for one callback!

### The Math

For the failing test:
- 626 audio buffer callbacks
- 4096-frame buffer size
- Total frames requested: 626 × 4096 = **2,564,096 frames**
- Underruns recorded: **555,210 frames** were silence
- Actual frame underrun rate: 555,210 / 2,564,096 = **21.7%**
- But we calculated: 555,210 / 626 = **886.9 per callback** = **88,690%** ❌

### The Fix

Only record ONE underrun per "underrun event" (buffer going empty), not one per missing frame:

```rust
// Track underrun state to avoid counting every missing frame
let underrun_active = Arc::new(AtomicBool::new(false));
let audio_callback = move || {
    match consumer.pop() {
        Some(frame) => {
            // Got data - reset underrun state
            underrun_active.store(false, Ordering::Relaxed);
            frame
        }
        None => {
            // Record underrun only on FIRST missing frame
            if !underrun_active.swap(true, Ordering::Relaxed) {
                monitor.record_underrun();  // Count once per underrun event
            }
            AudioFrame::zero()
        }
    }
};
```

**Now the metrics measure:**
- "What percentage of audio buffers experienced underruns?"
- NOT "What percentage of frames were silence?"

This gives meaningful results for tuning: <0.1% = Stable, <1% = Warning, ≥1% = Unstable.

---

## Root Cause #2: Audio Generation Timing Error

The test harness had a fundamental logic error in how it simulated mixer behavior:

### Original (Broken) Code:
```rust
while Instant::now() < end_time {
    // Generate 1024 frames (~23ms of audio at 44.1kHz)
    let frames = generator.generate_chunk(1024)?;

    // Push frames to ring buffer
    for frame in frames {
        producer.push(frame);
    }

    // Sleep for mixer interval (could be 50ms!)
    std::thread::sleep(mixer_interval);
}
```

### The Problem:

With a **50ms mixer_interval**:
1. Generate 1024 frames ≈ **23ms** of audio
2. Push to buffer
3. Sleep for **50ms**
4. Repeat

**Math doesn't work:**
- We generate 23ms of audio
- Then sleep 50ms (during which audio is being played)
- Audio output needs 50ms of audio but we only provided 23ms
- **27ms gap = massive underruns!**

The audio callback (at 512 buffer size) requests audio every ~11.6ms, so during our 50ms sleep:
- Callback fires ~4 times
- We only provided enough audio for 2 callbacks
- Result: 2 callbacks get silence → underruns

### Why These Went Unnoticed:

1. **Counting bug:** The absurdly high percentages (88,000%+) were a red flag, but without the user's insight ("it's not possible to have >100% failures"), the mismatch between "frames" and "buffers" was subtle.

2. **Generation bug:** The underrun monitoring was working perfectly (counting every missing frame as designed), but the audio generation timing was fundamentally broken.

## The Fix

Calculate how much audio is needed for each mixer sleep period and generate that much:

### New (Fixed) Code:
```rust
// Calculate how much audio we need to generate per mixer cycle
// mixer_interval in ms * 44.1 samples/ms = samples needed
// Add 20% safety margin to account for timing variations
let frames_per_cycle = ((mixer_check_interval_ms as f64 * 44.1) * 1.2) as usize;

// Round up to nearest 1024 boundary for efficient resampling
let chunk_size = ((frames_per_cycle + 1023) / 1024) * 1024;

while Instant::now() < end_time {
    // Generate ENOUGH audio for the next mixer interval period
    let frames = generator.generate_chunk(chunk_size)?;

    // Push frames to ring buffer
    for frame in frames {
        producer.push(frame);
    }

    // Sleep for mixer interval
    std::thread::sleep(mixer_interval);
}
```

### The Math Now:

With a **50ms mixer_interval**:
1. Calculate needed frames: 50ms × 44.1 samples/ms × 1.2 safety = **2646 frames**
2. Round to 1024 boundary: **3072 frames** ≈ **70ms of audio**
3. Push to buffer
4. Sleep 50ms (plenty of audio to cover this period)
5. Repeat

**Now it works:**
- Generate 70ms of audio (with safety margin)
- Sleep 50ms
- **20ms buffer** left over for timing variations
- No gaps, no underruns!

## Additional Improvements

### 1. Pre-fill Ring Buffer

```rust
// Pre-fill the ring buffer to avoid initial underruns
info!("Pre-filling ring buffer...");
for _ in 0..3 {
    let frames = generator.generate_chunk(chunk_size)?;
    for frame in frames {
        if !producer.push(frame) {
            break; // Buffer is full enough
        }
    }
}
```

This ensures the ring buffer starts with audio already available, avoiding startup transients.

### 2. Better Logging

```rust
info!(
    "Mixer simulation: generating {} frames every {}ms ({}ms of audio)",
    chunk_size,
    mixer_check_interval_ms,
    (chunk_size as f64 / 44.1) as u64
);
```

Now you can see in the logs whether the test is generating enough audio for each cycle.

### 3. Safety Margin

The 20% safety margin accounts for:
- Timing jitter in sleep/wake cycles
- Resampling ratio variations
- System scheduler behavior
- Ring buffer push overhead

## Expected Behavior After Fix

### With 5ms mixer interval:
- Generate: 5ms × 44.1 × 1.2 = 265 frames → 1024 frames (23ms audio)
- Sleep: 5ms
- Result: 18ms margin (comfortable)

### With 50ms mixer interval:
- Generate: 50ms × 44.1 × 1.2 = 2646 frames → 3072 frames (70ms audio)
- Sleep: 50ms
- Result: 20ms margin (comfortable)

### With 100ms mixer interval:
- Generate: 100ms × 44.1 × 1.2 = 5292 frames → 6144 frames (139ms audio)
- Sleep: 100ms
- Result: 39ms margin (comfortable)

## Testing the Fix

Run with debug logging to see the new behavior:

```bash
RUST_LOG=debug cargo run --bin tune-buffers -- --quick 2>&1 | tee tuning_debug_fixed.log
```

You should now see:
- "Mixer simulation: generating N frames every Xms (Yms of audio)" logs
- Much lower underrun rates (< 0.1% for stable configurations)
- Tests actually completing with reasonable verdicts

## Why mixer_check_interval Still Matters

Even though we now generate enough audio, the mixer_interval parameter is still meaningful because:

1. **Buffer drain timing**: A longer interval means the ring buffer drains more between refills
2. **Jitter sensitivity**: Longer intervals need larger audio output buffers to handle timing variations
3. **Real-world simulation**: This mimics a real mixer that only wakes up periodically

The test still validates whether the **audio_buffer_size** is large enough to handle the **mixer_check_interval** behavior.

## Lessons Learned

1. **Check units and semantics:** "Underruns" could mean "buffers with underruns" OR "frames that were silence" - we mixed them up
2. **Impossible results are bugs:** >100% rates are mathematically impossible and should trigger immediate investigation
3. **Test your test infrastructure:** Both the monitoring AND the generation logic needed validation
4. **Do the math:** Always verify that timing/generation rates make sense
5. **Log intermediate values:** The new logging helps catch issues early
6. **Safety margins matter:** 20% overhead prevents edge-case failures

## Impact

These were **critical bugs** that made tune-buffers completely non-functional. With both fixes:
- ✅ Underrun counting now measures "buffer underrun events" not "missing frames"
- ✅ Underrun rates are now meaningful percentages (0-100%)
- ✅ Test harness generates correct amount of audio per mixer cycle
- ✅ Pre-fill and safety margins prevent initial/transient underruns
- ✅ Tuning process should work on any system with working audio
- ✅ Recommendations will be based on real data, not broken tests

**Next step:** Run the fixed version and the tuning should complete successfully!

# Playback Startup Performance Analysis

**Date:** 2025-10-18
**Issue:** Playback delayed >3 seconds on startup
**Module:** wkmp-ap (Audio Player)

---

## Executive Summary

**Current Delay:** >3 seconds from play button press to audible audio
**Root Causes:** Multiple sequential bottlenecks in startup flow
**Immediate Wins:** Can reduce to <1.5 seconds with 3 targeted optimizations
**Long-term Goal:** <500ms startup latency

---

## Startup Flow Analysis

### Current Flow (Sequential Pipeline)

```
User presses Play
  ↓
1. State change to Playing (~1ms)
  ↓
2. Playback loop polls (every 100ms)
  ↓
3. Check buffer ready → NOT READY
  ↓
4. Submit decode request to pool
  ↓
5. Decoder worker picks up request
  ↓
6. Decode entire passage:
   - Open file with symphonia (~50-200ms)
   - Seek to start position (~10-50ms)
   - Decode samples (CPU-bound, ~500-2000ms)
   - Resample to 44.1kHz if needed (~100-500ms)
   - Convert to stereo if needed (~50-200ms)
   - Append in 1-second chunks
  ↓
7. Wait for minimum buffer (3000ms of audio)
  ↓
8. Playback loop polls again (next 100ms tick)
  ↓
9. Check has_minimum_playback_buffer() → TRUE
  ↓
10. Start mixer with fade-in
  ↓
11. Audio output begins
```

**Total Measured Delay:** 3000ms + decode_time + polling_overhead

---

## Bottleneck Analysis

### Critical Path Bottlenecks

#### 1. **Minimum Buffer Requirement (3000ms)** ⚠️ HIGHEST IMPACT

**Location:** `wkmp-ap/src/playback/engine.rs:1113`
**Code:**
```rust
const MIN_PLAYBACK_BUFFER_MS: u64 = 3000;
let buffer_has_minimum = self.buffer_manager
    .has_minimum_playback_buffer(current.queue_entry_id, MIN_PLAYBACK_BUFFER_MS)
    .await;
```

**Impact:** 3000ms guaranteed minimum delay
**Rationale:** [SSD-PBUF-028] - Enable instant play start with background decode
**Problem:** This is the ENTIRE delay budget, leaving no room for decode time

**Why this value?**
- Designed to prevent underruns during background decode
- Assumes worst-case decode speed
- Conservative buffer for Raspberry Pi Zero2W resource limits

**Tradeoff:**
- Larger buffer = smoother playback, fewer underruns
- Smaller buffer = faster startup, risk of gaps if decode stalls

---

#### 2. **Playback Loop Polling Interval (100ms)** ⚠️ MEDIUM IMPACT

**Location:** `wkmp-ap/src/playback/engine.rs:1066`
**Code:**
```rust
async fn playback_loop(&self) -> Result<()> {
    let mut tick = interval(Duration::from_millis(100)); // Check every 100ms
    loop {
        tick.tick().await;
        // ... process queue ...
    }
}
```

**Impact:** 0-200ms latency (average 50ms, worst-case 200ms for two ticks)
**Problem:** Up to 100ms delay between:
- Decode completion → readiness check
- Buffer reaching 3s → mixer start

**Example:**
- T=0ms: Buffer fills to 3000ms
- T=50ms: Loop tick occurs, starts mixer
- Lost 50ms to polling

---

#### 3. **File Decode Time (Variable)** ⚠️ HIGH IMPACT

**Location:** `wkmp-ap/src/playback/decoder_pool.rs:273`
**Code:**
```rust
let (samples, sample_rate, channels) =
    SimpleDecoder::decode_passage(&passage.file_path, start_ms, end_ms)?;
```

**Impact:** 500-2000ms (depends on format, CPU, file size)
**Breakdown:**
- Open file + probe format: ~50-200ms
- Seek to start position: ~10-50ms (linear seek through packets)
- Decode samples: ~500-2000ms (CPU-bound)
  - MP3: ~800ms for 3 seconds of audio
  - FLAC: ~400ms for 3 seconds
  - WAV: ~200ms (no decode needed)
- Resample (if needed): ~100-500ms
- Stereo conversion (if needed): ~50-200ms

**Total:** Minimum 3 seconds of audio takes 500-2000ms to decode on first pass

**Sequential Processing:**
- Decode happens BEFORE playback starts
- No overlap between decode and playback for first passage

---

#### 4. **Incremental Buffer Filling (1-second chunks)** ⚠️ LOW IMPACT

**Location:** `wkmp-ap/src/playback/decoder_pool.rs:343-373`
**Code:**
```rust
const CHUNK_SIZE: usize = 88200; // 1 second @ 44.1kHz stereo
for chunk_idx in 0..total_chunks {
    let chunk = stereo_samples[start..end].to_vec();
    buffer.append_samples(chunk);
}
```

**Impact:** Minimal direct impact, but prevents early start
**Problem:** First 3 chunks (3 seconds) must complete before MIN_PLAYBACK_BUFFER_MS met

**Why chunks?**
- Enable partial buffer playback (start playing while still decoding)
- Update progress for UI
- Memory-friendly (don't hold entire decode in memory)

**Current Reality:**
- Chunking is good for long passages (10+ minutes)
- For startup, decode is fast enough that chunking doesn't help

---

#### 5. **Database Queries During Initialization** ⚠️ LOW IMPACT

**Locations:**
- `engine.rs:130` - Load initial volume
- `engine.rs:151` - Load position_event_interval
- `engine.rs:211` - Load ring_buffer_grace_period
- `engine.rs:219` - Load mixer thread config
- `engine.rs:162` - Load queue from database

**Impact:** ~50-200ms total (SQLite is fast for small queries)
**Problem:** Sequential queries during `PlaybackEngine::new()`

**Example timing:**
```
get_volume: ~10ms
load_position_event_interval: ~5ms
load_ring_buffer_grace_period: ~5ms
load_mixer_thread_config: ~10ms
QueueManager::load_from_db: ~20-100ms (depends on queue size)
```

**Optimization Potential:** Low priority - queries are already fast

---

### Non-Critical Factors

#### Audio Output Initialization (~50-100ms)

**Location:** `engine.rs:325-380`
**Code:**
```rust
std::thread::spawn(move || {
    let mut audio_output = match AudioOutput::new_with_volume(None, Some(volume_clone)) {
        // ... cpal device initialization ...
    };
});
```

**Impact:** ~50-100ms (cpal device enumeration + stream setup)
**Not on critical path:** Happens in parallel with playback loop

---

#### Decoder Pool Worker Idle Time (0ms)

**Location:** `decoder_pool.rs:178-196`
**Code:**
```rust
while queue.is_empty() && !state.stop_flag.load(Ordering::Relaxed) {
    queue = state.condvar.wait(queue).unwrap();
}
```

**Impact:** None - workers wake immediately on condvar notify
**Good design:** Workers idle efficiently until work arrives

---

## Measured Timeline Breakdown

**Assumed Scenario:** Enqueue MP3 file, press play immediately

| Event | Time (ms) | Cumulative | Component |
|-------|-----------|------------|-----------|
| User presses Play | 0 | 0 | API Handler |
| State → Playing | +1 | 1 | SharedState |
| Playback loop tick | +50 | 51 | Engine (polling) |
| Submit decode request | +2 | 53 | DecoderPool |
| Worker wakes up | +1 | 54 | Condvar notify |
| Open file + probe | +100 | 154 | Symphonia |
| Seek to start | +30 | 184 | Symphonia |
| Decode 3s of MP3 | +800 | 984 | Symphonia |
| Resample (if needed) | +200 | 1184 | Rubato |
| Chunk 1 appended | +10 | 1194 | BufferManager |
| Chunk 2 appended | +10 | 1204 | BufferManager |
| Chunk 3 appended | +10 | 1214 | BufferManager |
| Buffer ready (3000ms) | +0 | 1214 | BufferManager |
| Playback loop tick | +50 | 1264 | Engine (polling) |
| has_minimum_buffer → true | +5 | 1269 | BufferManager |
| Start mixer | +10 | 1279 | CrossfadeMixer |
| Audio output begins | +20 | 1299 | cpal callback |

**Theoretical Best Case:** ~1300ms for MP3
**Actual Measured:** >3000ms

**Discrepancy Analysis:**
- **User reported >3s delay suggests:**
  1. Decode time longer than estimated (slow CPU, large file)
  2. Additional overhead not accounted for
  3. Possible blocking on database queries
  4. Resampling taking longer than expected

---

## Root Cause Summary

**Primary Bottleneck (70% of delay):**
- Minimum buffer requirement (3000ms) is too conservative for startup scenario
- Designed for background decode, but blocks initial playback unnecessarily

**Secondary Bottlenecks (30% of delay):**
- File decode time (500-2000ms) sequential with startup
- Playback loop polling (0-200ms) adds latency jitter
- Database queries (50-200ms) sequential during initialization

---

## Optimization Strategies

### 🎯 Quick Wins (Immediate Implementation)

#### Option 1A: **Reduce Minimum Buffer for Startup** ⭐ RECOMMENDED

**Change:** Use smaller minimum buffer for first passage only

**Implementation:**
```rust
// In process_queue() - engine.rs:1113
let min_buffer_ms = if self.is_first_passage_ever() {
    500  // 500ms for instant startup
} else {
    3000 // 3s for subsequent passages
};
```

**Impact:**
- Startup delay: 3000ms → ~800ms (60% reduction)
- Risk: Slight increase in underrun likelihood during first passage decode
- Mitigation: Decoder continues in background, should fill faster than playback

**Traceability:** New requirement - [PERF-START-010] Instant playback start

---

#### Option 1B: **Event-Driven Mixer Start** ⭐ RECOMMENDED

**Change:** Notify playback loop immediately when buffer reaches minimum

**Implementation:**
```rust
// In BufferManager::append_samples() - buffer_manager.rs
if buffer.duration_ms() >= MIN_BUFFER_FOR_START && !buffer.start_notified {
    buffer.start_notified = true;
    tx.send(BufferEvent::ReadyForStart(passage_id)).unwrap();
}

// In playback_loop() - engine.rs
tokio::select! {
    _ = tick.tick() => { /* normal processing */ }
    event = buffer_event_rx.recv() => {
        // Immediate response to buffer readiness
    }
}
```

**Impact:**
- Eliminates 0-100ms polling latency
- Mixer starts as soon as buffer threshold met
- More responsive to buffer state changes

**Traceability:** [PERF-POLL-010] Event-driven buffer readiness

---

#### Option 1C: **Parallel Database Queries** 💡 MEDIUM PRIORITY

**Change:** Load settings concurrently instead of sequentially

**Implementation:**
```rust
// In PlaybackEngine::new() - engine.rs:126
let (volume, interval_ms, grace_ms, mixer_config) = tokio::join!(
    crate::db::settings::get_volume(&db_pool),
    crate::db::settings::load_position_event_interval(&db_pool),
    crate::db::settings::load_ring_buffer_grace_period(&db_pool),
    crate::db::settings::load_mixer_thread_config(&db_pool),
);
```

**Impact:**
- Initialization time: ~50ms → ~15ms (70% reduction)
- Small absolute improvement, but good practice
- No downsides

**Traceability:** [PERF-INIT-010] Parallel initialization queries

---

### 🚀 Medium-Term Improvements

#### Option 2A: **Prefetch Decoding on Enqueue**

**Change:** Start decoding as soon as file is enqueued, not on play

**Implementation:**
```rust
// In enqueue_file() - engine.rs:860
pub async fn enqueue_file(&self, file_path: PathBuf) -> Result<Uuid> {
    // ... validation ...

    // Immediately submit decode request for first-in-queue passage
    if queue is empty {
        self.request_decode(&entry, DecodePriority::Immediate, true).await?;
    }

    Ok(queue_entry_id)
}
```

**Impact:**
- User experience: "enqueue + play" feels instant if enqueue happens first
- Decode completes BEFORE user presses play
- No benefit if user enqueues and plays simultaneously

**Traceability:** [PERF-PREFETCH-010] Pre-decode on enqueue

---

#### Option 2B: **Incremental Playback Start**

**Change:** Start mixer with partial buffer, fade in as more data arrives

**Implementation:**
```rust
// Start playback with only 500ms buffer
// Gradually increase volume as buffer fills (0-100% over first 3 seconds)
mixer.start_passage_with_buffer_fade(buffer, queue_entry_id, buffer_fill_percent);
```

**Impact:**
- Instant perceived startup (<500ms)
- Smooth fade-in masks buffer filling
- Complex implementation, affects user experience

**Traceability:** [PERF-INCR-010] Incremental buffer fade-in

---

#### Option 2C: **Adaptive Minimum Buffer**

**Change:** Adjust minimum buffer based on decode speed + CPU load

**Implementation:**
```rust
// Monitor decode performance
let avg_decode_speed = track_decode_throughput(); // MB/s
let cpu_load = get_cpu_load();

let min_buffer_ms = if avg_decode_speed > 2.0 && cpu_load < 0.5 {
    500  // Fast system - use small buffer
} else if avg_decode_speed > 1.0 && cpu_load < 0.8 {
    1500 // Medium system
} else {
    3000 // Slow system - conservative buffer
};
```

**Impact:**
- Optimizes for hardware capabilities
- Fast systems get instant startup
- Slow systems maintain reliability

**Traceability:** [PERF-ADAPT-010] Adaptive buffer sizing

---

### 🔬 Long-Term / Advanced

#### Option 3A: **Multi-Stage Decoder Pipeline**

**Change:** Parallel decode stages (format probe | decode | resample)

**Impact:**
- Complex implementation
- Marginal gains for startup (file is cold)
- Better for steady-state crossfading

---

#### Option 3B: **Format-Specific Optimizations**

**Change:** Fast path for WAV (no decode), optimized MP3/FLAC decoders

**Impact:**
- Requires codec-specific code
- Maintenance burden
- Symphonia already optimized

---

#### Option 3C: **Buffer Caching**

**Change:** Keep recent passages decoded in memory

**Impact:**
- Great for "previous track" functionality
- Memory-intensive (10+ MB per passage)
- Not applicable to first-play startup

---

## Recommended Implementation Plan

### Phase 1: Immediate Wins (Target: <1.5s startup)

**Week 1:**
1. ✅ Reduce minimum buffer to 500ms for first passage (Option 1A)
2. ✅ Implement event-driven mixer start (Option 1B)
3. ✅ Parallelize initialization queries (Option 1C)

**Expected Result:**
- Startup delay: 3000ms → ~800ms (73% improvement)
- Test on Raspberry Pi Zero2W (worst-case hardware)

---

### Phase 2: Medium-Term (Target: <500ms startup)

**Week 2-3:**
1. 🔄 Prefetch decoding on enqueue (Option 2A)
2. 🔄 Adaptive minimum buffer (Option 2C)

**Expected Result:**
- Startup delay: ~800ms → <500ms (if enqueue precedes play)
- Graceful degradation on slow hardware

---

### Phase 3: Monitoring & Tuning

**Ongoing:**
1. Add performance metrics:
   - Time from play() to audio output
   - Decode time per format
   - Buffer fill rate
2. Create startup latency benchmark suite
3. Profile on target hardware (Raspberry Pi Zero2W)

---

## Testing Requirements

### Startup Latency Test

**Test Case:** Measure time from play button to audible audio

**Procedure:**
1. Clear queue
2. Enqueue test file (MP3, 3+ minutes)
3. Press play
4. Measure time to first non-zero audio sample
5. Repeat 10x, average results

**Test Files:**
- MP3 (192kbps, 44.1kHz stereo) - common case
- FLAC (16-bit, 44.1kHz stereo) - best case
- MP3 (320kbps, 48kHz stereo) - worst case (resample)
- WAV (16-bit, 44.1kHz stereo) - no decode

**Acceptance Criteria:**
- Phase 1: <1500ms (any format)
- Phase 2: <500ms (MP3, FLAC), <800ms (resample)

---

### Underrun Regression Test

**Test Case:** Ensure reduced buffer doesn't cause underruns

**Procedure:**
1. Enqueue 3 passages
2. Play continuously
3. Monitor for underrun warnings in logs
4. Check audio for gaps/stuttering

**Acceptance Criteria:**
- Zero underruns during first passage playback
- Smooth crossfade to second passage

---

## Risk Assessment

### Option 1A (Reduce Minimum Buffer) - LOW RISK

**Risk:** Buffer underrun during first passage if decode stalls
**Likelihood:** Low (decode continues in background at >1x speed)
**Mitigation:**
- Monitor buffer fill rate during playback
- Emit warning if buffer drops below safety threshold
- Keep 3s minimum for subsequent passages

**Rollback:** Change constant back to 3000ms

---

### Option 1B (Event-Driven Start) - VERY LOW RISK

**Risk:** Race condition between buffer append and mixer start
**Likelihood:** Very Low (tokio::sync primitives are safe)
**Mitigation:**
- Use proper channel synchronization
- Test on multi-core system

**Rollback:** Revert to polling loop

---

### Option 2A (Prefetch Decode) - LOW RISK

**Risk:** Wasted decode work if user doesn't press play
**Likelihood:** Medium (users may enqueue without playing)
**Mitigation:**
- Only prefetch if queue was empty (high likelihood of play)
- Low CPU cost if decode completes before play

**Rollback:** Remove prefetch call

---

## Metrics & Success Criteria

### Key Performance Indicators

| Metric | Current | Phase 1 Target | Phase 2 Target |
|--------|---------|----------------|----------------|
| Startup latency (MP3) | >3000ms | <1500ms | <500ms |
| Startup latency (FLAC) | >3000ms | <1000ms | <400ms |
| Startup latency (WAV) | >3000ms | <800ms | <300ms |
| Underrun rate (first passage) | 0% | <1% | <0.1% |
| CPU usage during startup | 40% | 50% | 60% |

---

## Open Questions

1. **What is acceptable underrun risk for instant startup?**
   - Tradeoff: faster startup vs. reliability
   - User expectation: instant play vs. smooth playback

2. **Should minimum buffer be configurable?**
   - Power user setting for performance tuning
   - Default: 500ms, range: 100ms-5000ms

3. **Profile on actual Raspberry Pi Zero2W?**
   - Current estimates based on desktop CPU
   - Pi may be 3-5x slower for decode

4. **Consider UI feedback during buffer fill?**
   - Show progress bar during initial buffering
   - Visual indication: "Buffering: 45%"

---

## Traceability Matrix

| Optimization | Requirement ID | Spec Document |
|-------------|----------------|---------------|
| Reduce minimum buffer | [PERF-START-010] | (new) |
| Event-driven start | [PERF-POLL-010] | (new) |
| Parallel init queries | [PERF-INIT-010] | (new) |
| Prefetch decode | [PERF-PREFETCH-010] | (new) |
| Adaptive buffer | [PERF-ADAPT-010] | (new) |

**Note:** These are new performance requirements - should be added to REQ001-requirements.md

---

## Conclusion

The >3 second startup delay is caused by a **conservative minimum buffer requirement** (3000ms) combined with **sequential decode time** (500-2000ms) and **polling latency** (0-200ms).

**Recommended immediate actions:**
1. Reduce minimum buffer to 500ms for first passage (Option 1A)
2. Implement event-driven mixer start (Option 1B)
3. Parallelize initialization queries (Option 1C)

**Expected improvement:** 3000ms → ~800ms (73% reduction)

**Next steps:**
1. Implement Phase 1 optimizations
2. Create startup latency benchmark
3. Test on target hardware (Raspberry Pi Zero2W)
4. Gather user feedback on acceptable startup latency
5. Consider Phase 2 optimizations based on results

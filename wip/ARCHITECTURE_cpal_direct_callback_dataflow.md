# Architecture: cpal Direct Audio Callback - Dataflow and Timing

**Document Purpose:** Explain the dataflow and timing triggers in WKMP's cpal-based direct audio callback architecture.

**Date:** 2025-11-02

---

## Executive Summary

WKMP uses a **lock-free direct callback architecture** for audio output:

1. **cpal audio callback** (real-time audio thread) pulls pre-mixed samples from ring buffer
2. **Mixer thread** (tokio async) periodically wakes to fill ring buffer from passage buffers
3. **Decoder chains** (tokio async) pre-decode and buffer audio chunks ahead of playback

**Key Design Principle:** Minimize work in real-time audio callback - all mixing, decoding, and resampling happens BEFORE audio callback runs.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        TIMING TRIGGERS                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Audio Device (Hardware)         Mixer Thread (Software)           │
│         │                                │                         │
│    Every ~50ms                      Every ~10ms                     │
│    (2208 frames)               (mixer_check_interval_ms)            │
│         │                                │                         │
│         ▼                                ▼                         │
│  ┌──────────────┐              ┌──────────────────┐               │
│  │ Audio        │              │ Tokio Interval   │               │
│  │ Callback     │◄─────────────│ Timer Tick       │               │
│  │ (cpal)       │  Ring Buffer │ (async)          │               │
│  └──────────────┘              └──────────────────┘               │
│         │                                │                         │
│         │                                │                         │
└─────────┼────────────────────────────────┼─────────────────────────┘
          │                                │
          ▼                                ▼
    PULL samples                    PUSH samples
    (consumer)                      (producer)
          │                                │
          └────────► RING BUFFER ◄─────────┘
                   (lock-free queue)
                   Capacity: 2048 frames
                   ~46ms @ 44.1kHz
```

---

## Component Dataflow

### 1. Audio Callback (Real-Time Thread)

**Location:** [wkmp-ap/src/playback/engine/core.rs:785-795](wkmp-ap/src/playback/engine/core.rs#L785-L795)

**Purpose:** Provide samples to audio device on-demand (hardware-driven timing)

**Timing Trigger:** **Audio device hardware** - callback invoked when device buffer needs samples

**Frequency:**
- Determined by `audio_buffer_size` (DBD-PARAM-110, default: 2208 frames)
- **~50ms interval** @ 44.1kHz (2208 frames / 44100 Hz = 50.1ms)
- **~46ms interval** @ 48kHz (2208 frames / 48000 Hz = 46ms)

**Dataflow:**
```rust
// LOCK-FREE audio callback - runs on real-time audio thread
let audio_callback = move || {
    // Lock-free read from ring buffer
    match consumer.pop() {
        Some(frame) => frame,           // ← Return pre-mixed sample
        None => {
            monitor_clone.record_underrun();
            AudioFrame::zero()          // ← Underrun: return silence
        }
    }
};
```

**Characteristics:**
- ✅ **Lock-free:** Only pops from ring buffer (no locks, no async)
- ✅ **Fast:** <1μs typical execution time (single ring buffer pop)
- ✅ **No allocation:** No heap allocations in callback
- ✅ **No blocking:** Never blocks or waits
- ⚠️ **Critical path:** Any delay here causes audio glitches

**Called By:** [wkmp-ap/src/audio/output.rs:268-304](wkmp-ap/src/audio/output.rs#L268-L304)

cpal invokes callback once per buffer:
```rust
self.device.build_output_stream(
    &self.config,
    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // Record callback timing ONCE per buffer
        if let Some(ref mon) = monitor {
            mon.record_callback();
        }

        // Fetch frames from user callback
        for frame in data.chunks_mut(channels) {
            let audio_frame = callback();  // ← Calls lock-free callback above

            // Apply volume
            let left = audio_frame.left * current_volume;
            let right = audio_frame.right * current_volume;

            // Clamp and write to output buffer
            frame[0] = left.clamp(-1.0, 1.0);
            frame[1] = right.clamp(-1.0, 1.0);
        }
    },
    // ... error callback ...
)
```

**Buffer Size Impact:**
- **Smaller buffer (e.g., 512 frames):** Lower latency (11ms), higher CPU usage (more frequent callbacks)
- **Larger buffer (e.g., 4096 frames):** Higher latency (93ms), lower CPU usage (fewer callbacks)
- **Default (2208 frames):** Balance between latency and stability (50ms)

---

### 2. Ring Buffer (Lock-Free Queue)

**Purpose:** Decouple mixer thread from audio callback timing

**Capacity:** 2048 frames (default) = ~46ms @ 44.1kHz

**Implementation:** Lock-free SPSC (single-producer, single-consumer) queue
- **Consumer:** Audio callback (real-time thread)
- **Producer:** Mixer thread (tokio async thread)

**Fill Levels:**
```
0%        25%       50%       75%      100%
├─────────┼─────────┼─────────┼─────────┤
│CRITICAL │  LOW    │ OPTIMAL │  HIGH   │FULL
└─────────┴─────────┴─────────┴─────────┘
    ▲         ▲         ▲         ▲
    │         │         │         │
Underrun   Needs    Target    Stop
 Risk!    Refill    Range    Filling
```

**Adaptive Filling Strategy:** [core.rs:638-690](wkmp-ap/src/playback/engine/core.rs#L638-L690)
- **< 25% (CRITICAL):** Fill aggressively without sleeping (underrun imminent!)
- **25-50% (LOW):** Fill moderately with minimal sleep
- **50-75% (OPTIMAL):** Top up conservatively
- **> 75% (HIGH):** Just sleep and wait

**Why Ring Buffer?**
- ✅ Tolerates jitter in mixer thread wake timing
- ✅ Smooths over brief CPU spikes
- ✅ Allows mixer to work in larger batches (efficiency)
- ✅ No locks between audio callback and mixer

---

### 3. Mixer Thread (Async Tokio Thread)

**Location:** [wkmp-ap/src/playback/engine/core.rs:455-704](wkmp-ap/src/playback/engine/core.rs#L455-L704)

**Purpose:** Fill ring buffer with mixed audio from passage buffers

**Timing Trigger:** **Software timer** - `tokio::time::interval()`

**Frequency:**
- Controlled by `mixer_check_interval_ms` (DBD-PARAM-111, default: 10ms)
- **Every 10ms** the mixer wakes and checks ring buffer fill level

**Dataflow:**
```
Mixer Thread Loop (every 10ms):
1. Wake from tokio::time::interval timer
2. Check ring buffer fill level
3. If needs filling:
   a. Call mixer.mix_single() → reads from passage buffers
   b. Push mixed frames to ring buffer (producer.push())
4. Sleep until next interval tick
```

**Batch Mixing:** [core.rs:465-538](wkmp-ap/src/playback/engine/core.rs#L465-L538)
```rust
// Fixed batch size for efficiency
const BATCH_SIZE_FRAMES: usize = 512;  // ~11ms @ 44.1kHz

async fn mix_and_push_batch(...) {
    // Allocate output buffer (stereo: 2 samples per frame)
    let mut output = vec![0.0f32; frames_to_mix * 2];

    let mut mixer_guard = mixer.write().await;  // ← Acquire mixer lock

    // Mix batch from passage buffer
    let events = mixer_guard.mix_single(
        buffer_manager,
        passage_id,
        &mut output
    ).await?;

    drop(mixer_guard);  // ← Release mixer lock before ring buffer push

    // Push frames to ring buffer (lock-free)
    for i in (0..output.len()).step_by(2) {
        let frame = AudioFrame {
            left: output[i],
            right: output[i + 1],
        };
        if !producer.push(frame) {
            break;  // Ring buffer full
        }
    }
}
```

**Adaptive Filling Logic:** [core.rs:652-690](wkmp-ap/src/playback/engine/core.rs#L652-L690)
```rust
let fill_percent = occupied as f32 / capacity as f32;

if is_critical {  // < 25%
    // UNDERRUN IMMINENT! Fill aggressively
    let frames_to_mix = BATCH_SIZE_FRAMES * 2;  // 1024 frames
    mix_and_push_batch(...).await;
    // NO SLEEP - immediately check again

} else if needs_filling {  // 25-50%
    // Buffer low - fill with standard batch
    let frames_to_mix = BATCH_SIZE_FRAMES;  // 512 frames
    mix_and_push_batch(...).await;
    check_interval.tick().await;  // Minimal sleep

} else if is_optimal {  // 50-75%
    // Buffer healthy - top up conservatively
    let frames_to_mix = BATCH_SIZE_FRAMES / 2;  // 256 frames
    mix_and_push_batch(...).await;
    check_interval.tick().await;

} else {  // > 75%
    // Buffer full - just sleep
    check_interval.tick().await;
}
```

**Why 10ms Check Interval?**
- Too low (1-5ms): Async overhead dominates, wasted CPU cycles
- Too high (50-100ms): Risk of buffer underruns during playback
- **10ms**: Sweet spot for balance between responsiveness and efficiency

**Empirical Tuning:** Default 10ms provides VeryHigh stability confidence (from auto-tuning system)

---

### 4. Mixer (Sample Provider)

**Location:** [wkmp-ap/src/playback/mixer.rs](wkmp-ap/src/playback/mixer.rs)

**Purpose:** Read pre-faded samples from passage buffers, apply master volume

**Key Method:** `mix_single()` [mixer.rs:554-681](wkmp-ap/src/playback/mixer.rs#L554-L681)

**Dataflow:**
```rust
pub async fn mix_single(
    &mut self,
    buffer_manager: &Arc<BufferManager>,
    _passage_id: Uuid,
    output: &mut [f32]
) -> Result<Vec<MarkerEvent>> {
    // Get buffer from BufferManager (keyed by queue_entry_id)
    let queue_entry_id = self.current_queue_entry_id
        .ok_or_else(|| Error::Config("No current queue entry ID set"))?;

    let buffer_arc = buffer_manager
        .get_buffer(queue_entry_id).await
        .ok_or_else(|| Error::Config(format!("No buffer found")))?;

    let frames_requested = output.len() / 2;
    let mut frames_read = 0;

    // Read frames from PlayoutRingBuffer
    while frames_read < frames_requested {
        match buffer_arc.pop_frame() {  // ← Pull from passage buffer
            Ok(frame) => {
                // Apply master volume
                let left = frame.left * self.master_volume;
                let right = frame.right * self.master_volume;

                // Apply resume fade-in if active
                if let Some(ref resume) = self.resume_state {
                    let fade_multiplier = resume.fade_in_curve
                        .calculate_fade_in(fade_position);
                    left *= fade_multiplier;
                    right *= fade_multiplier;
                }

                output[output_idx] = left;
                output[output_idx + 1] = right;
                frames_read += 1;
            }
            Err(_) => {
                // Buffer underrun - fill remainder with silence
                break;
            }
        }
    }

    // Update position tracking (tick counter)
    let tick_increment = wkmp_common::timing::samples_to_ticks(
        frames_read,
        sample_rate
    );
    self.current_tick += tick_increment;

    // Check position markers and return triggered events
    let events = self.check_markers();

    Ok(events)
}
```

**Responsibilities:**
1. ✅ Read pre-faded samples from passage buffer (no fade calculation here!)
2. ✅ Apply master volume
3. ✅ Apply resume fade-in (if resuming from pause)
4. ✅ Track playback position (tick counter)
5. ✅ Detect position markers (events like crossfade start, passage end)
6. ✅ Handle buffer underruns gracefully (fill with silence)

**NOT Responsible For:**
- ❌ Fade curve calculation (done by Fader before buffering)
- ❌ Crossfade mixing (future feature, currently single passage only)
- ❌ Decoding (done by decoder chains)
- ❌ Resampling (done by decoder chains)

---

### 5. Passage Buffers (Per-Passage Ring Buffers)

**Location:** [wkmp-ap/src/playback/playout_ring_buffer.rs](wkmp-ap/src/playback/playout_ring_buffer.rs)

**Purpose:** Store pre-decoded, pre-faded audio for each passage

**Capacity:** `playout_ringbuffer_size` (DBD-PARAM-070, default: 661941 samples = 15.01s @ 44.1kHz)

**Characteristics:**
- **Lock-free:** AtomicUsize for read/write pointers
- **Pre-faded:** Samples already have fade curves applied (by Fader)
- **Per-passage:** Each queued passage has its own buffer
- **Managed by:** BufferManager (tracks all passage buffers)

**Filled By:** Decoder chains (decode → resample → fade → buffer)

---

### 6. Decoder Chains (Background Workers)

**Location:** [wkmp-ap/src/playback/pipeline/decoder_chain.rs](wkmp-ap/src/playback/pipeline/decoder_chain.rs)

**Purpose:** Pre-decode audio chunks and push to passage buffers

**Timing Trigger:** **Event-driven** - wake when passage buffer needs refill

**Refill Logic:** [buffer_manager.rs](wkmp-ap/src/playback/buffer_manager.rs)
- **Pause threshold:** free_space ≤ `playout_ringbuffer_headroom` (4410 samples = 0.1s)
- **Resume threshold:** free_space ≥ `decoder_resume_hysteresis_samples` + headroom (48510 samples)

**Dataflow:**
```
Decoder Chain Pipeline:
1. Decode chunk from audio file
   ├─ Chunk size: chunk_duration_ms (1000ms)
   └─ Output: PCM samples at source rate

2. Resample to working_sample_rate
   ├─ Input: Variable source rate (48kHz, 96kHz, etc.)
   └─ Output: Fixed working rate (44.1kHz)

3. Apply fade curves
   ├─ Fade-in at passage start
   ├─ Fade-out at passage end
   └─ Output: Pre-faded samples

4. Push to passage buffer
   └─ Fills playout ring buffer ahead of playback
```

**Wake Triggers:**
- **Initial:** Buffer created → decode first chunk immediately
- **Ongoing:** Buffer free space crosses resume threshold → decode next chunk
- **Paused:** Buffer full (free_space ≤ headroom) → sleep until space available

**Chunk Size:** `chunk_duration_ms` (DBD-PARAM-065, default: 1000ms)
- Why time-based? Consistent decode time across variable source rates
- 1000ms @ 48kHz = 48000 samples
- 1000ms @ 96kHz = 96000 samples
- Same decode duration regardless of source

---

## Timing Diagram

```
TIME ──────────────────────────────────────────────────►

Audio Device (Hardware):
│◄─────50ms────►│◄─────50ms────►│◄─────50ms────►│
├───────────────┼───────────────┼───────────────┤
│   Callback    │   Callback    │   Callback    │  (2208 frames each)
└───────────────┴───────────────┴───────────────┘
        │               │               │
        ▼               ▼               ▼
  Pull 2208 frames from ring buffer each time


Mixer Thread (Software):
│◄10ms►│◄10ms►│◄10ms►│◄10ms►│◄10ms►│◄10ms►│
├──────┼──────┼──────┼──────┼──────┼──────┤
│ Mix  │ Mix  │ Mix  │ Mix  │ Mix  │ Mix  │  (512 frames each)
└──────┴──────┴──────┴──────┴──────┴──────┘
    │      │      │      │      │      │
    ▼      ▼      ▼      ▼      ▼      ▼
Push 512 frames to ring buffer each 10ms


Ring Buffer State:
┌────────────────────────────────────────┐
│██████████████████░░░░░░░░░░░░░░░░░░░░░│  50% full (OPTIMAL)
└────────────────────────────────────────┘
  ▲                                    ▲
  │                                    │
Consumer                           Producer
(audio callback)                  (mixer thread)
  │                                    │
  └─── Pulls at ~50ms intervals       │
                                       │
                    Pushes at 10ms intervals ─┘
```

**Key Insight:** Mixer wakes ~5x more frequently than audio callback, keeping ring buffer well-filled.

---

## Timing Parameters Summary

| Parameter | Default | Purpose | Impact |
|-----------|---------|---------|--------|
| **audio_buffer_size** | 2208 frames | Audio callback interval | 50ms latency @ 44.1kHz |
| **mixer_check_interval_ms** | 10 ms | Mixer thread wake frequency | Responsiveness vs CPU |
| **chunk_duration_ms** | 1000 ms | Decoder chunk size | Memory vs decode overhead |
| **playout_ringbuffer_size** | 661941 samples | Passage buffer capacity | 15s buffering @ 44.1kHz |
| **playout_ringbuffer_headroom** | 4410 samples | Decoder pause threshold | 0.1s safety margin |
| **decoder_resume_hysteresis_samples** | 44100 samples | Decoder resume threshold | 1.0s hysteresis gap |

---

## Why This Architecture?

### Problem: Audio Glitches in Callback-Based Playback

**Challenge:** Audio callbacks run on real-time threads with strict timing constraints
- Callbacks invoked every ~50ms by hardware
- Any delay > buffer duration causes audible glitches
- Cannot do slow operations (decoding, resampling, I/O) in callback

### Solution: Lock-Free Ring Buffer with Background Mixer

**Key Principles:**
1. **Minimize callback work:** Audio callback only pops from ring buffer (~1μs)
2. **Decouple timing:** Mixer thread can tolerate jitter (10ms timer)
3. **Pre-compute everything:** All expensive work done before callback
4. **Lock-free path:** No locks between mixer and audio callback

**Benefits:**
- ✅ **Low latency:** 50ms (audio buffer size)
- ✅ **High stability:** Ring buffer tolerates mixer jitter
- ✅ **Efficient:** Batch mixing reduces syscall overhead
- ✅ **Simple callback:** Lock-free, allocation-free, fast
- ✅ **Graceful degradation:** Underruns output silence (no crashes)

---

## Comparison: Direct Callback vs Output Ring Buffer

### Current Architecture (Direct Callback)

**Pros:**
- ✅ Minimal latency (50ms from buffer fill to output)
- ✅ Simple architecture (no intermediate output buffer)
- ✅ Low memory overhead (only passage buffers + small ring buffer)

**Cons:**
- ⚠️ Mixer thread must wake frequently (10ms) to prevent underruns
- ⚠️ Ring buffer small (2048 frames = 46ms) - less tolerance for jitter

### Alternative: Output Ring Buffer (UNUSED Parameters)

**SPEC016 Defined But Not Implemented:**
- `output_ringbuffer_size` (88200 samples = 2.0s @ 44.1kHz)
- `output_refill_period` (90ms)

**How It Would Work:**
```
Mixer Thread:
├─ Wake every 90ms (output_refill_period)
├─ Fill output ring buffer (88200 samples capacity)
└─ Audio callback pulls from output ring buffer

Benefits:
- Mixer can wake less frequently (90ms vs 10ms)
- Larger buffer = more tolerance for CPU spikes

Costs:
- Higher latency (2.0s vs 0.05s)
- More memory (88200 samples = 706KB per channel)
```

**Why Not Used?**
- Consumer playback (current use case) works well with direct callback
- Low latency more important than extreme stability
- 10ms mixer interval acceptable on modern systems

**When Useful?**
- Pro audio workflows requiring extreme stability
- Systems with unreliable mixer timing
- High background CPU load scenarios

---

## Event-Driven Architecture

### Position Markers

**Purpose:** Signal when playback reaches specific time points

**Flow:**
```
1. PlaybackEngine calculates WHEN events should occur (tick count)
   ├─ Crossfade start tick = end_tick - crossfade_duration
   ├─ Position update tick = current_tick + interval
   └─ Passage end tick = passage_end_tick

2. PlaybackEngine adds markers to Mixer
   mixer.add_marker(PositionMarker {
       tick: crossfade_start_tick,
       passage_id: current_passage_id,
       event_type: MarkerEvent::StartCrossfade { next_passage_id },
   });

3. Mixer checks markers during mix_single()
   ├─ After mixing each batch, check if current_tick >= marker.tick
   ├─ If reached, pop marker and return event
   └─ Mixer thread converts MarkerEvents to PlaybackEvents

4. PlaybackEngine receives events and takes action
   ├─ StartCrossfade → Start decoding next passage
   ├─ PassageComplete → Advance queue
   └─ PositionUpdate → Emit SSE to UI
```

**Marker Types:**
- `PositionUpdate { position_ms }` - Regular position updates for UI
- `StartCrossfade { next_passage_id }` - Begin crossfade to next passage
- `PassageComplete` - Current passage finished
- `SongBoundary { new_song_id }` - Track changed (for cooldowns)
- `EndOfFile { ... }` - Reached end of audio file

**Why Event-Driven?**
- ✅ Precise timing (sample-accurate triggering)
- ✅ Decouples engine logic from mixer execution
- ✅ Supports complex sequences (crossfades, markers, etc.)

---

## Performance Characteristics

### Typical Resource Usage (12 Decoder Chains, 44.1kHz)

**CPU:**
- Audio callback: <0.1% (lock-free ring buffer pop)
- Mixer thread: ~0.6% (10ms wake interval, 512-frame batches)
- Decoder chains: ~2-5% (on-demand, 1000ms chunks)

**Memory:**
- Ring buffer: 2048 frames × 8 bytes = 16 KB
- Passage buffers: 12 × 661941 samples × 8 bytes = ~60 MB
- Decoder chunks: 12 × 1000ms × 44100 Hz × 8 bytes = ~4.2 MB

**Latency:**
- End-to-end: ~100ms (decoder → buffer → ring buffer → audio out)
- Audio callback to output: ~50ms (audio buffer size)

---

## Conclusion

WKMP's cpal direct callback architecture achieves:

✅ **Low latency** (50ms) for responsive playback
✅ **High stability** (lock-free callback, ring buffer jitter tolerance)
✅ **Efficient resource usage** (batch mixing, on-demand decoding)
✅ **Graceful degradation** (underruns → silence, not crashes)

**Core Design:** Decouple real-time audio callback from background processing via lock-free ring buffer, enabling complex audio operations (decode, resample, fade) without risking audio glitches.

---

**Related Documents:**
- SPEC016: Audio Buffer Design (DBD-PARAM specifications)
- PLAN018: Centralized Global Parameters (parameter migration)
- [audio/output.rs](wkmp-ap/src/audio/output.rs) - cpal audio output implementation
- [playback/mixer.rs](wkmp-ap/src/playback/mixer.rs) - Mixer implementation
- [playback/engine/core.rs](wkmp-ap/src/playback/engine/core.rs) - Ring buffer and mixer thread

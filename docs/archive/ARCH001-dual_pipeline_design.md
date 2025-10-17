# Dual Pipeline Design for Audio Playback with Crossfading

**📦 ARCHIVED DOCUMENT - BEING REPLACED**

**Archive Date:** 2025-10-16
**Status:** Partially implemented but being abandoned in favor of single-stream architecture.
**Replacement Document:** See [single-stream-playback.md](../SPEC013-single_stream_playback.md) for the new architecture.

**Decision Rationale:**
- Single stream provides 500-2500x better crossfade precision (0.02ms vs 10-50ms)
- 6x lower memory usage (27 MB vs 170 MB for 5 buffered passages)
- Simpler architecture with direct per-sample control
- Pure Rust implementation reduces external dependencies
- See [single-stream-migration-proposal.md](ARCH004-single_stream_migration_proposal.md) for complete migration details

This document is preserved for historical reference and to document the partially-implemented dual pipeline code.

---

> **Note (Historical):** This document described the **implementation** using GStreamer. An alternative **Single Stream** architecture was considered - see `single-stream-design.md` for details and `architecture-comparison.md` for a full comparison of both approaches.

## Overview

The WKMP Audio Player dual pipeline architecture uses GStreamer's `audiomixer` element to enable crossfading between audio tracks. This design allows pre-loading the next track while the current track is playing, enabling transitions.

**Status at Archive:** ✅ Partially implemented - Basic playback functional, crossfading partially implemented.

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                     Main Pipeline                            │
│                                                              │
│  ┌──────────────┐                                           │
│  │  Pipeline A  │──┐                                        │
│  │    (Bin)     │  │         ┌─────────────┐               │
│  └──────────────┘  ├────────►│             │               │
│                     │         │ audiomixer  │──► master    │
│  ┌──────────────┐  │         │             │    volume ──► │
│  │  Pipeline B  │──┘         └─────────────┘    autoaudio- │
│  │    (Bin)     │                                  sink     │
│  └──────────────┘                                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Component Structure

#### DualPipeline
The main structure that manages the entire playback system:

```rust
pub struct DualPipeline {
    main_pipeline: gst::Pipeline,
    pipeline_a: PipelineComponents,
    pipeline_b: PipelineComponents,
    audiomixer: gst::Element,
    master_volume: gst::Element,
    audio_sink: gst::Element,
    active: Arc<RwLock<ActivePipeline>>,
    master_volume_level: Arc<RwLock<f64>>,
}
```

#### PipelineComponents
Each pipeline bin contains a complete audio processing chain:

```rust
struct PipelineComponents {
    filesrc: gst::Element,
    decodebin: gst::Element,
    audioconvert: gst::Element,
    audioresample: gst::Element,
    volume: gst::Element,
    volume_level: Arc<RwLock<f64>>,
    bin: gst::Bin,
}
```

### Element Chain per Pipeline

Each pipeline (A and B) contains:

1. **filesrc** - Reads audio file from disk
2. **decodebin** - Automatically detects and decodes audio format
3. **audioconvert** - Converts audio to a common format
4. **audioresample** - Resamples audio to a common sample rate
5. **volume** - Individual volume control for crossfading
6. **bin** - Container with ghost pad exposing the output

These are linked statically except for decodebin, which creates pads dynamically when the audio format is detected.

### Main Pipeline Elements

- **audiomixer** - Mixes audio from both pipeline bins
- **master_volume** - Global volume control
- **autoaudiosink** - Automatic audio output device selection

## Key Design Decisions

### 1. Ghost Pad Management

**Critical Discovery:** Manual ghost pad activation caused pipeline state transition failures.

**Solution:** Let GStreamer automatically activate ghost pads during state transitions.

```rust
// Create ghost pad but don't manually activate it
let ghost_pad = gst::GhostPad::with_target(&volume_src_pad)?;
// Note: Don't call set_active(true) - GStreamer handles activation
bin.add_pad(&ghost_pad)?;
```

**Why:** When ghost pads were manually activated before their target elements were ready, GStreamer refused to transition the pipeline to PLAYING state because it detected activated pads pointing to inactive source elements.

### 2. Dummy File Initialization

**Problem:** Empty bins with no file loaded had inactive pads, preventing pipeline state transitions.

**Solution:** Initialize both pipelines with `/dev/null` as dummy files.

```rust
// Pipeline A - initially silent with dummy file
pipeline_a.filesrc.set_property("location", "/dev/null");
pipeline_a.volume.set_property("volume", 0.0f64);

// Pipeline B - always silent with dummy file
pipeline_b.filesrc.set_property("location", "/dev/null");
pipeline_b.volume.set_property("volume", 0.0f64);
```

**Why:** This ensures pads are always in a valid state and can activate properly when the main pipeline transitions to PLAYING.

### 3. State Management Strategy

**Approach:** Minimal manual state management - let GStreamer's state propagation do the work.

```rust
pub async fn load_file(&self, pipeline: ActivePipeline, file_path: &PathBuf) -> Result<()> {
    // Set bin to NULL to change filesrc location
    components.bin.set_state(gst::State::Null)?;

    // Set new file location
    components.filesrc.set_property("location", file_path.to_str().unwrap());

    // Set volume for active playback
    components.volume.set_property("volume", 1.0f64);

    // Don't set state here - let main pipeline propagation handle it
    Ok(())
}
```

**Why:** Manually setting bin states to READY or PAUSED caused race conditions and pad activation issues. Letting the main pipeline's state change propagate down to bins via `sync_state_with_parent()` works reliably.

### 4. Dynamic Pad Linking

Decodebin creates audio pads dynamically after analyzing the file format:

```rust
decodebin.connect_pad_added(move |_element, pad| {
    let pad_caps = pad.current_caps().unwrap();
    let pad_struct = pad_caps.structure(0).unwrap();
    let pad_name = pad_struct.name();

    if pad_name.starts_with("audio/") {
        let sink_pad = audioconvert_clone.static_pad("sink").unwrap();
        if !sink_pad.is_linked() {
            pad.link(&sink_pad)?;
        }
    }
});
```

## Playback Flow

### Initial Load and Play

1. **Load first track:**
   - Set Pipeline A to NULL
   - Update filesrc location to actual audio file
   - Set volume to 1.0
   - Pipeline A stays in NULL

2. **Play:**
   - Main pipeline transitions to PLAYING
   - State change propagates to bins via `sync_state_with_parent()`
   - Pipeline A goes: NULL → READY → PAUSED → PLAYING
   - Decodebin analyzes file and creates audio pad
   - Audio flows: filesrc → decodebin → audioconvert → audioresample → volume → audiomixer → master_volume → sink

3. **Result:**
   - Audio plays through PulseAudio
   - Position queries work
   - State is "Playing"

### Crossfade (Planned)

1. **Pre-load next track:**
   - Load file into inactive pipeline (e.g., Pipeline B)
   - Set its volume to 0.0 initially
   - Pipeline B prerolls in background

2. **Crossfade:**
   - Gradually decrease active pipeline volume (A: 1.0 → 0.0)
   - Simultaneously increase next pipeline volume (B: 0.0 → 1.0)
   - Both pipelines play simultaneously during transition

3. **Switch active:**
   - Mark Pipeline B as active
   - Set Pipeline A to NULL
   - Ready for next track load into Pipeline A

## State Transitions

### Main Pipeline States

```
NULL → READY → PAUSED → PLAYING
         ↓         ↓        ↓
    (bins sync) (bins sync) (bins sync)
         ↓         ↓        ↓
   Bin A/B    Bin A/B   Bin A/B
```

### Key State Transition Points

1. **NULL → READY:** Elements allocate resources
2. **READY → PAUSED:** Elements preroll (decode headers, create pads)
3. **PAUSED → PLAYING:** Clock starts, data flows

**Critical:** Bins automatically follow main pipeline state via `sync_state_with_parent()` called in `play()` method.

## Position and Duration Queries

Queries target the **active pipeline bin**, not the main pipeline:

```rust
pub async fn position_ms(&self) -> Option<i64> {
    let active_pipeline = *self.active.read().await;
    let components = match active_pipeline {
        ActivePipeline::A => &self.pipeline_a,
        ActivePipeline::B => &self.pipeline_b,
    };

    components.bin
        .query_position::<gst::ClockTime>()
        .map(|pos| pos.mseconds() as i64)
}
```

**Why:** The main pipeline only knows about the mixer, not individual media files.

## Volume Control

### Two-Level Volume System

1. **Per-pipeline volume:** Controls individual pipeline loudness (used for crossfading)
   ```rust
   pipeline_a.volume.set_property("volume", 1.0f64);
   ```

2. **Master volume:** Global volume control after mixing
   ```rust
   master_volume.set_property("volume", 0.75f64);
   ```

### Crossfade Implementation

Crossfading uses per-pipeline volumes:
- **Active pipeline:** volume = 1.0 (full)
- **Inactive pipeline:** volume = 0.0 (silent)
- **During crossfade:** Both volumes transition smoothly

## Critical Lessons Learned

### 1. Manual Pad Activation is Harmful

**Problem:** Calling `ghost_pad.set_active(true)` before the pipeline was ready prevented proper state transitions.

**Error message:** `"pad not activated yet"` from GStreamer

**Solution:** Remove all manual pad activation and let GStreamer handle it automatically.

### 2. State Change Timing Matters

**Problem:** Setting bin states manually before the main pipeline was ready caused conflicts.

**Solution:** Only set bin to NULL when changing files. Let main pipeline state changes propagate down.

### 3. Bins Need Valid Sources

**Problem:** Bins with no file loaded had inactive elements, blocking state transitions.

**Solution:** Always initialize with dummy files (`/dev/null`) so elements are in valid states.

### 4. Async State Changes Require Patience

GStreamer state changes can be asynchronous:
```rust
Ok(gst::StateChangeSuccess::Async) => {
    // State change in progress, will complete eventually
}
```

**Solution:** Use `sync_state_with_parent()` after main pipeline state change to ensure bins follow.

## Testing and Validation

### Successful Tests

✅ **Audio Output:** Verified via PulseAudio sink-input listing
```bash
pactl list sink-inputs | grep -A 10 "application.name"
# Shows: application.name = "wkmp-ap"
```

✅ **Playback State:** API returns correct state
```json
{"playback_state":"playing","queue_size":1,...}
```

✅ **Pipeline State:** Main pipeline reaches PLAYING without errors

### Known Issues

⚠️ **Position Tracking:** Position endpoint returns empty (TODO: investigate)

⚠️ **Clock Warning:** "Pipeline has no clock!" during state transition (benign - clock assigned when PLAYING is reached)

## File Locations

- **Implementation:** `wkmp-ap/src/playback/pipeline/dual.rs`
- **Integration:** `wkmp-ap/src/playback/engine.rs`
- **Monitoring:** `wkmp-ap/src/playback/monitor.rs`

## Future Enhancements

1. **Implement actual crossfading:**
   - Add crossfade duration parameter
   - Implement volume ramping over time
   - Trigger crossfade near end of track

2. **Improve position tracking:**
   - Debug why position queries return None
   - Ensure position advances during playback

3. **Add crossfade controls:**
   - Crossfade duration setting
   - Crossfade curve options (linear, exponential, etc.)

4. **Optimize state transitions:**
   - Reduce latency when switching tracks
   - Pre-load tracks earlier for smoother transitions

## Alternative Architectures

### Single Stream Approach

An alternative **Single Stream** architecture using manual buffer management has been designed. Key differences:

**Advantages of Single Stream:**
- ✅ **Sample-accurate crossfading** (0.02ms vs 10-50ms precision)
- ✅ **Lower memory usage** (31 MB vs 170 MB)
- ✅ **Pure Rust dependencies** (static binary, no GStreamer runtime)
- ✅ **Easier debugging** (your code vs framework internals)
- ✅ **Better cross-platform** (single binary works everywhere)

**Advantages of Dual Pipeline (this approach):**
- ✅ **Faster implementation** (working now vs 2-4 weeks)
- ✅ **Battle-tested framework** (GStreamer powers millions of apps)
- ✅ **Automatic format detection** (handles exotic codecs)

**Documentation:**
- Full design: See `single-stream-design.md`
- Side-by-side comparison: See `architecture-comparison.md`
- Original concept: See `../single_stream_crossfade.txt`

**Recommendation:** The Single Stream approach is better suited for WKMP's professional audio requirements (sample-accurate crossfading, low resource usage, cross-platform). Consider migrating after validating the current implementation.

## References

- GStreamer Documentation: https://gstreamer.freedesktop.org/documentation/
- audiomixer element: https://gstreamer.freedesktop.org/documentation/audiomixer/
- Bin state synchronization: https://gstreamer.freedesktop.org/documentation/gstreamer/gstbin.html

---

**Document Version:** 1.1
**Last Updated:** 2025-10-16
**Status:** Playback working, crossfading pending implementation
**Related:** `single-stream-design.md`, `architecture-comparison.md`

# Revised Mixer Architecture - Event-Driven Position Tracking

**Date:** 2025-01-30
**Context:** PLAN014 Sub-Increment 4a - Architectural clarification

---

## Executive Summary

**Previous Misconception:** Mixer should be completely stateless with no position awareness.

**Corrected Understanding:** Mixer should not perform fade **calculations**, but it is uniquely positioned to know **playback reality** (exact frames delivered to audio device). The mixer should use an event-driven architecture where PlaybackEngine calculates points of interest and the mixer signals when those points are reached.

**Key Insight:** The mixer is the **execution layer** (knows what's actually happening), while PlaybackEngine is the **calculation layer** (determines what should happen and when).

---

## The Problem with Timer-Based Position Tracking

### Current Approach (Legacy Mixer)

```rust
// ❌ PROBLEM: Timer-based polling (every 100ms)
self.frame_counter += 1;

if self.frame_counter >= self.position_event_interval_frames {
    self.frame_counter = 0; // Reset counter

    // Emit position event
    tx.send(PositionUpdate {
        queue_entry_id: self.current_passage_id,
        position_ms: self.calculate_position_ms(),
    });
}
```

**Issues:**
1. **Arbitrary polling interval** (100ms has no relationship to actual events)
2. **Unnecessary events** (emits even if nothing interesting happening)
3. **Missed precision** (can't signal exact moment of interest)
4. **CPU waste** (constant timer checking)

---

## Event-Driven Architecture (Correct Approach)

### Architectural Principle

**Separation of Concerns:**
- **PlaybackEngine (Calculation Layer):** Determines WHAT should happen and WHEN
- **Mixer (Execution Layer):** Executes actions and signals WHEN events actually occur

### Division of Responsibilities

#### PlaybackEngine Responsibilities

**What it knows:**
- Passage timing metadata (start_time_ticks, end_time_ticks, fade points)
- Queue state (current, next, queued passages)
- Desired event frequency (e.g., "emit position update every 5 seconds")

**What it calculates:**
- **Crossfade timing:** When to start crossfade (tick count in current passage)
- **Position markers:** Specific points of interest (song boundaries, position milestones)
- **Event thresholds:** When to signal position updates

**What it does:**
```rust
// Example: Calculate crossfade start point
let crossfade_start_tick = current_passage.end_time_ticks - crossfade_duration_ticks;

// Tell mixer about this point
mixer.add_marker(PositionMarker {
    tick: crossfade_start_tick,
    passage_id: current_passage.passage_id,
    event_type: MarkerEvent::StartCrossfade {
        next_passage_id: next_passage.passage_id,
    },
});

// Example: Add position update markers every 5 seconds
for position_tick in (0..end_tick).step_by(5_seconds_in_ticks) {
    mixer.add_marker(PositionMarker {
        tick: position_tick,
        passage_id: current_passage.passage_id,
        event_type: MarkerEvent::PositionUpdate,
    });
}
```

#### Mixer Responsibilities

**What it knows:**
- **Exact playback position:** How many frames delivered to audio device
- **Output buffer state:** How full the audio device's buffer is
- **Current passage buffer state:** How many samples available

**What it tracks:**
- Current tick count (frames mixed from current passage)
- Position markers (sorted list of upcoming events)

**What it signals:**
```rust
// Example: Check markers during mixing
pub fn mix_single(&mut self, passage_buffer: &mut RingBuffer, output: &mut [f32]) -> Result<Vec<MarkerEvent>> {
    // ... mix samples ...

    self.current_tick += frames_mixed;

    // Check if any markers reached
    let mut triggered_events = Vec::new();
    while let Some(marker) = self.markers.peek() {
        if self.current_tick >= marker.tick {
            let marker = self.markers.pop().unwrap();
            triggered_events.push(marker.event_type);
        } else {
            break; // Markers are sorted, stop checking
        }
    }

    Ok(triggered_events)
}
```

---

## Position Marker System Design

### Data Structures

#### PositionMarker

```rust
/// Position marker for event-driven signaling
///
/// PlaybackEngine calculates when events should occur (tick count),
/// Mixer signals when those ticks are actually reached during playback.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionMarker {
    /// Tick count when this marker should trigger
    /// (relative to passage start)
    pub tick: i64,

    /// Which passage this marker belongs to
    pub passage_id: Uuid,

    /// What event to signal when reached
    pub event_type: MarkerEvent,
}

impl Ord for PositionMarker {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.tick.cmp(&other.tick)
    }
}
```

#### MarkerEvent

```rust
/// Event types that mixer can signal
#[derive(Debug, Clone, PartialEq)]
pub enum MarkerEvent {
    /// Position update milestone reached
    PositionUpdate {
        position_ms: u64,
    },

    /// Start crossfade to next passage
    StartCrossfade {
        next_passage_id: Uuid,
    },

    /// Song boundary crossed (for multi-song passages)
    SongBoundary {
        new_song_id: Option<Uuid>,
    },

    /// Passage playback completed
    PassageComplete,

    /// Custom application-defined marker
    Custom {
        marker_id: String,
    },
}
```

#### Mixer Marker Storage

```rust
use std::collections::BinaryHeap;

pub struct Mixer {
    // ... existing fields ...

    /// Position markers (min-heap sorted by tick)
    /// PlaybackEngine adds markers, mixer pops when reached
    markers: BinaryHeap<Reverse<PositionMarker>>,

    /// Current tick count in current passage
    /// Incremented as frames are mixed
    current_tick: i64,

    /// Current passage ID (for marker validation)
    current_passage_id: Option<Uuid>,
}
```

**Why BinaryHeap?**
- Automatically sorted by tick (soonest marker at top)
- O(log n) insertion, O(log n) removal
- Efficient "peek next marker" (O(1))

---

## Crossfade Timing (Critical Use Case)

### Problem Statement

The mixer must know **exactly when** to start mixing the next passage into the output buffer. This timing is critical for sample-accurate crossfades.

### Current Approach (Legacy Mixer)

**Legacy mixer tracks everything:**
```rust
enum MixerState {
    SinglePassage {
        passage_id: Uuid,
        frame_count: usize,
        fade_out_start_frame: Option<usize>,  // ❌ Mixer calculates timing
        // ...
    },
    Crossfading {
        current_passage_id: Uuid,
        next_passage_id: Uuid,
        crossfade_duration_frames: usize,
        // ...
    },
}

// ❌ Mixer decides when to crossfade
if self.frame_count >= self.fade_out_start_frame {
    self.start_crossfade();
}
```

**Problem:** Mixer duplicates passage timing logic (should be PlaybackEngine's job).

### Event-Driven Approach (Correct)

**PlaybackEngine calculates crossfade timing:**
```rust
// In PlaybackEngine::start_passage()
pub async fn start_passage(&mut self, queue_entry: QueueEntry) -> Result<()> {
    let passage = get_passage_timing(&self.db_pool, &queue_entry).await?;

    // Calculate crossfade start point
    let crossfade_duration_ticks = self.get_crossfade_duration_ticks(&passage);
    let crossfade_start_tick = passage.end_time_ticks.unwrap_or(0) - crossfade_duration_ticks;

    // Tell mixer about this marker
    let mut mixer = self.mixer.write().await;
    mixer.add_marker(PositionMarker {
        tick: crossfade_start_tick,
        passage_id: queue_entry.passage_id.unwrap(),
        event_type: MarkerEvent::StartCrossfade {
            next_passage_id: next_queue_entry.passage_id.unwrap(),
        },
    });

    // Tell mixer to start playing this passage
    mixer.set_current_passage(queue_entry.passage_id.unwrap());

    Ok(())
}
```

**Mixer executes crossfade when marker reached:**
```rust
// In Mixer::mix()
pub fn mix(&mut self, buffers: &BufferSet, output: &mut [f32]) -> Result<Vec<MarkerEvent>> {
    // Mix samples
    let frames_mixed = self.do_mixing(buffers, output)?;

    // Update position
    self.current_tick += frames_mixed as i64;

    // Check markers
    let mut events = Vec::new();
    while let Some(marker) = self.markers.peek() {
        if self.current_tick >= marker.0.tick {
            let marker = self.markers.pop().unwrap().0;

            match marker.event_type {
                MarkerEvent::StartCrossfade { next_passage_id } => {
                    // START CROSSFADE NOW
                    self.begin_crossfade(next_passage_id);
                    events.push(marker.event_type);
                }
                _ => {
                    events.push(marker.event_type);
                }
            }
        } else {
            break; // Sorted, stop checking
        }
    }

    Ok(events)
}
```

**Key Benefits:**
1. **Engine calculates** (what/when to crossfade)
2. **Mixer executes** (starts crossfade when tick reached)
3. **No duplication** (passage timing only in engine)
4. **Sample-accurate** (mixer knows exact frame delivered)

---

## Position Update Events (Event-Driven)

### Problem with Current Timer-Based Approach

```rust
// ❌ Emits every 100ms regardless of interest
if frame_counter % 4410 == 0 {  // Every 100ms @ 44.1kHz
    emit_position_event();
}
```

**Issues:**
- Emits 10 events/second (excessive if only displaying once per 5 seconds)
- No relationship to actual points of interest
- Wastes CPU and channel bandwidth

### Event-Driven Approach

**PlaybackEngine sets markers at desired intervals:**
```rust
pub async fn start_passage(&mut self, queue_entry: QueueEntry) -> Result<()> {
    let passage = get_passage_timing(&self.db_pool, &queue_entry).await?;

    // Load desired position update interval from settings
    let update_interval_ms = self.get_position_update_interval_ms().await;  // e.g., 5000ms
    let update_interval_ticks = ms_to_ticks(update_interval_ms);

    // Add markers every 5 seconds
    let passage_duration_ticks = passage.end_time_ticks.unwrap_or(0) - passage.start_time_ticks;
    for tick in (0..passage_duration_ticks).step_by(update_interval_ticks as usize) {
        let position_ms = ticks_to_ms(tick);

        mixer.add_marker(PositionMarker {
            tick: passage.start_time_ticks + tick,
            passage_id: queue_entry.passage_id.unwrap(),
            event_type: MarkerEvent::PositionUpdate { position_ms },
        });
    }

    Ok(())
}
```

**Mixer signals when markers reached:**
```rust
// In mix() return value
Ok(vec![
    MarkerEvent::PositionUpdate { position_ms: 5000 },  // 5 seconds
    MarkerEvent::PositionUpdate { position_ms: 10000 }, // 10 seconds
    // ... only when actually reached
])
```

**PlaybackEngine processes signaled events:**
```rust
// In playback loop or audio callback handler
let events = mixer.mix(&buffers, &mut output)?;

for event in events {
    match event {
        MarkerEvent::PositionUpdate { position_ms } => {
            self.emit_position_event(position_ms).await;
        }
        MarkerEvent::StartCrossfade { next_passage_id } => {
            // Crossfade already started by mixer, just log
            debug!("Crossfade started to passage {}", next_passage_id);
        }
        // ... other events
    }
}
```

---

## Output Buffer Awareness

### Why Mixer Needs to Know Output Buffer State

The mixer is uniquely positioned to know:
1. **Frames delivered to audio device** (exact playback position)
2. **Audio device buffer fill level** (how much audio queued)

This is critical for:
- **Accurate position reporting** (account for buffered audio not yet heard)
- **Underrun detection** (audio device buffer empty)
- **Latency compensation** (adjust timing for buffer depth)

### Implementation

**Mixer tracks output buffer state:**
```rust
pub struct Mixer {
    // ... existing fields ...

    /// Frames written to output ring buffer
    frames_written: u64,

    /// Audio device buffer size (from cpal)
    device_buffer_frames: usize,
}

impl Mixer {
    /// Get actual playback position (accounting for buffered audio)
    pub fn get_playback_position_ms(&self) -> u64 {
        // Frames delivered = frames written - frames still in buffer
        let frames_delivered = self.frames_written.saturating_sub(self.device_buffer_frames as u64);

        // Convert to milliseconds
        (frames_delivered * 1000) / 44100
    }
}
```

**PlaybackEngine uses this for accurate position reporting:**
```rust
// Instead of calculating from tick count, ask mixer for reality
let actual_position_ms = mixer.get_playback_position_ms();

self.state.set_current_passage(Some(CurrentPassage {
    queue_entry_id,
    passage_id,
    position_ms: actual_position_ms,  // ✅ Accurate (accounts for buffering)
    duration_ms,
})).await;
```

---

## Revised Mixer API

### Core Mixing Methods

```rust
impl Mixer {
    /// Mix single passage into output buffer
    ///
    /// Returns list of marker events triggered during this mix operation.
    pub fn mix_single(
        &mut self,
        passage_buffer: &mut RingBuffer,
        output: &mut [f32]
    ) -> Result<Vec<MarkerEvent>> {
        // ... existing mixing logic ...

        // Update position
        self.current_tick += frames_mixed as i64;
        self.frames_written += frames_mixed as u64;

        // Check markers and return triggered events
        Ok(self.check_markers())
    }

    /// Mix two passages with crossfade
    pub fn mix_crossfade(
        &mut self,
        current_buffer: &mut RingBuffer,
        next_buffer: &mut RingBuffer,
        output: &mut [f32]
    ) -> Result<Vec<MarkerEvent>> {
        // ... existing crossfade logic ...

        // Update position
        self.current_tick += frames_mixed as i64;
        self.frames_written += frames_mixed as u64;

        // Check markers
        Ok(self.check_markers())
    }
}
```

### Marker Management Methods

```rust
impl Mixer {
    /// Add position marker
    ///
    /// PlaybackEngine calculates when events should occur (tick count),
    /// mixer signals when those ticks are reached.
    pub fn add_marker(&mut self, marker: PositionMarker) {
        self.markers.push(Reverse(marker));
    }

    /// Clear all markers for a passage
    ///
    /// Used when skipping or stopping a passage.
    pub fn clear_markers(&mut self, passage_id: Uuid) {
        self.markers.retain(|m| m.0.passage_id != passage_id);
    }

    /// Clear all markers
    pub fn clear_all_markers(&mut self) {
        self.markers.clear();
    }

    /// Check markers and return triggered events
    fn check_markers(&mut self) -> Vec<MarkerEvent> {
        let mut events = Vec::new();

        while let Some(marker_ref) = self.markers.peek() {
            if self.current_tick >= marker_ref.0.tick {
                let marker = self.markers.pop().unwrap().0;

                // Handle crossfade markers internally
                match &marker.event_type {
                    MarkerEvent::StartCrossfade { next_passage_id } => {
                        // Mixer handles crossfade start automatically
                        self.begin_crossfade(*next_passage_id);
                    }
                    _ => {
                        // Other events just signaled to engine
                    }
                }

                events.push(marker.event_type);
            } else {
                break; // Markers sorted, stop checking
            }
        }

        events
    }
}
```

### Passage Management Methods

```rust
impl Mixer {
    /// Set current passage
    ///
    /// Resets tick counter and clears position state.
    pub fn set_current_passage(&mut self, passage_id: Uuid) {
        self.current_passage_id = Some(passage_id);
        self.current_tick = 0;
    }

    /// Get current tick position
    pub fn get_current_tick(&self) -> i64 {
        self.current_tick
    }

    /// Get actual playback position (accounting for output buffer)
    pub fn get_playback_position_ms(&self) -> u64 {
        let frames_delivered = self.frames_written.saturating_sub(self.device_buffer_frames as u64);
        (frames_delivered * 1000) / 44100
    }
}
```

---

## Migration Strategy

### Phase 1: Add Marker System to Correct Mixer

**Step 1:** Add marker data structures
```rust
// In wkmp-ap/src/playback/mixer.rs

use std::collections::BinaryHeap;
use std::cmp::Reverse;

pub struct PositionMarker {
    pub tick: i64,
    pub passage_id: Uuid,
    pub event_type: MarkerEvent,
}

pub enum MarkerEvent {
    PositionUpdate { position_ms: u64 },
    StartCrossfade { next_passage_id: Uuid },
    SongBoundary { new_song_id: Option<Uuid> },
    PassageComplete,
}
```

**Step 2:** Add marker storage to Mixer struct
```rust
pub struct Mixer {
    // ... existing fields ...
    markers: BinaryHeap<Reverse<PositionMarker>>,
    current_tick: i64,
    current_passage_id: Option<Uuid>,
    frames_written: u64,
    device_buffer_frames: usize,
}
```

**Step 3:** Implement marker management methods
- `add_marker()`
- `clear_markers()`
- `check_markers()`

**Step 4:** Update `mix_single()` and `mix_crossfade()` to return `Vec<MarkerEvent>`

### Phase 2: Update PlaybackEngine to Use Markers

**Step 1:** When starting passage, add markers
```rust
// In PlaybackEngine::start_passage()

// Add position update markers
for tick in (0..duration_ticks).step_by(5_seconds_in_ticks) {
    mixer.add_marker(PositionMarker {
        tick,
        passage_id,
        event_type: MarkerEvent::PositionUpdate { position_ms: ticks_to_ms(tick) },
    });
}

// Add crossfade start marker
mixer.add_marker(PositionMarker {
    tick: crossfade_start_tick,
    passage_id,
    event_type: MarkerEvent::StartCrossfade { next_passage_id },
});
```

**Step 2:** Process marker events from mixer
```rust
// In audio callback or playback loop
let events = mixer.mix_single(&mut buffer, &mut output)?;

for event in events {
    match event {
        MarkerEvent::PositionUpdate { position_ms } => {
            self.handle_position_update(position_ms).await;
        }
        MarkerEvent::StartCrossfade { next_passage_id } => {
            debug!("Crossfade started to {}", next_passage_id);
        }
        // ... other events
    }
}
```

### Phase 3: Remove Timer-Based Position Tracking

**Step 1:** Remove position event channel from mixer instantiation
```rust
// REMOVE:
// mixer.set_event_channel(position_event_tx.clone());
// mixer.set_position_event_interval_ms(interval_ms);
```

**Step 2:** Remove frame counter logic from mixer

**Step 3:** Verify events still work with marker-based approach

---

## Benefits of Event-Driven Architecture

### Vs. Timer-Based Polling

| Aspect | Timer-Based (Legacy) | Event-Driven (Correct) |
|--------|---------------------|------------------------|
| **Event frequency** | Fixed (e.g., 10/second) | Variable (only when needed) |
| **Precision** | ±100ms (timer granularity) | Sample-accurate (exact tick) |
| **CPU usage** | Constant (always checking timer) | Minimal (only check when mixing) |
| **Flexibility** | Hard-coded interval | Configurable per-passage |
| **Relevant events only** | No (emits even if nothing happening) | Yes (only at points of interest) |

### Separation of Concerns

**PlaybackEngine (Calculation Layer):**
- ✅ Knows passage metadata (timing, fade points)
- ✅ Calculates when events should occur
- ✅ Determines event frequency
- ❌ Does NOT know actual playback position (buffering, device state)

**Mixer (Execution Layer):**
- ✅ Knows actual playback position (frames delivered to device)
- ✅ Knows output buffer state (latency)
- ✅ Signals when calculated events actually occur
- ❌ Does NOT calculate when events should occur (receives markers from engine)

---

## Open Questions

### Q1: How to handle passage changes during crossfade?

**Scenario:** Crossfade from Passage A → B. Which passage do markers belong to?

**Options:**
1. **All markers have passage_id** (current design)
   - Mixer clears markers for old passage when crossfade completes
   - Position updates continue for both passages during crossfade

2. **Markers transition at crossfade midpoint**
   - First half of crossfade: A's markers
   - Second half: B's markers

**Decision needed:** Depends on desired UX (what passage ID to show during crossfade)

### Q2: How to handle seek operations?

**Scenario:** User seeks to 30 seconds into passage.

**Approach:**
```rust
pub fn seek(&mut self, target_tick: i64) {
    // Update position
    self.current_tick = target_tick;

    // Clear markers before target (already passed)
    self.markers.retain(|m| m.0.tick >= target_tick);
}
```

### Q3: Should mixer auto-advance to next passage?

**Current design:** Mixer signals `PassageComplete` event, engine decides what to do.

**Alternative:** Mixer auto-advances if next passage buffer ready.

**Recommendation:** Keep current design (engine decides) for flexibility.

---

## Summary

**Revised Understanding:**

1. **Mixer is NOT completely stateless** - it tracks:
   - Current tick position (frames mixed from current passage)
   - Position markers (upcoming events)
   - Output buffer state (frames delivered to device)

2. **Mixer does NOT calculate timing** - it receives markers from PlaybackEngine:
   - Engine: "Start crossfade at tick 100,000"
   - Mixer: "Tick 100,000 reached, starting crossfade, signaling event"

3. **Event-driven, not timer-based:**
   - No periodic polling (no 100ms timer)
   - Events triggered when exact ticks reached
   - Only relevant events signaled

4. **Clean separation:**
   - **Engine = Calculation** (what/when should happen)
   - **Mixer = Execution** (make it happen, signal when done)

**Next Steps:**
1. Implement marker system in correct mixer
2. Update PlaybackEngine to use markers instead of timer-based events
3. Test event-driven position tracking
4. Migrate to correct mixer

---

**Date:** 2025-01-30
**Author:** Claude (PLAN014 Implementation)

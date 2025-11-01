# Position Tracking Analysis - Current vs. Target Architecture

**Date:** 2025-01-30
**Context:** PLAN014 Sub-Increment 4a - Understanding position tracking before migration

---

## Current Architecture (Legacy Mixer Emits Events)

### Data Flow Overview

```
QueueManager → PlaybackEngine.state → Legacy Mixer → PositionUpdate Events → position_event_handler → SharedState
     ↓                                      ↑                                         ↓
   Tracks                             Emits events                              Updates state
 queue_entry_id                       periodically                              current_passage
```

### Components and Responsibilities

#### 1. QueueManager
**Location:** `wkmp-ap/src/playback/queue_manager.rs`

**What it knows:**
- **Current:** Currently playing queue entry
- **Next:** Next queue entry to play
- **Queued:** All queued entries

**Data Structure:**
```rust
pub struct QueueEntry {
    pub queue_entry_id: Uuid,         // Unique ID for this queue entry
    pub passage_id: Option<Uuid>,     // Passage ID (may be ephemeral)
    pub file_path: PathBuf,
    pub start_time_ticks: i64,
    pub end_time_ticks: Option<i64>,
    // ... other fields
}
```

**Key Method:**
```rust
pub fn current(&self) -> Option<&QueueEntry>
```

**Responsibility:** Knows WHICH passage should be playing (by queue_entry_id and passage_id)

---

#### 2. Legacy Mixer (CrossfadeMixer)
**Location:** `wkmp-ap/src/playback/pipeline/mixer.rs`

**What it knows:**
- **Current passage_id:** Extracted from MixerState (SinglePassage or Crossfading)
- **Frame position:** Tracks frames mixed since passage started
- **Sample rate:** For converting frames to milliseconds

**MixerState (Stateful - PROBLEM):**
```rust
enum MixerState {
    SinglePassage {
        passage_id: Uuid,           // ❌ Mixer tracks passage ID
        frame_count: usize,         // ❌ Mixer tracks position
        // ...
    },
    Crossfading {
        current_passage_id: Uuid,   // ❌ Mixer tracks passage ID
        crossfade_frame_count: usize, // ❌ Mixer tracks position
        // ...
    },
    None,
}
```

**Position Event Emission (lines 614-642):**
```rust
// In get_next_frame() - called ~44,100 times/second
self.frame_counter += 1;

if self.frame_counter >= self.position_event_interval_frames {
    self.frame_counter = 0; // Reset counter

    // Emit PositionUpdate event if channel configured
    if let Some(tx) = &self.event_tx {
        if let Some(passage_id) = self.get_current_passage_id() {
            let position_ms = self.calculate_position_ms();

            // Non-blocking send (use try_send to avoid blocking audio thread)
            let _ = tx.send(PlaybackEvent::PositionUpdate {
                queue_entry_id: passage_id,
                position_ms,
            });
        }
    }
}
```

**Helper Methods:**
```rust
// Get current passage ID from mixer state
pub fn get_current_passage_id(&self) -> Option<Uuid> {
    match &self.state {
        MixerState::SinglePassage { passage_id, .. } => Some(*passage_id),
        MixerState::Crossfading { current_passage_id, .. } => Some(*current_passage_id),
        MixerState::None => None,
    }
}

// Calculate position in milliseconds
fn calculate_position_ms(&self) -> u64 {
    let position_frames = self.get_position();
    (position_frames as u64 * 1000) / self.sample_rate as u64
}

// Get frame position from mixer state
pub fn get_position(&self) -> usize {
    match &self.state {
        MixerState::SinglePassage { frame_count, .. } => *frame_count,
        MixerState::Crossfading { crossfade_frame_count, .. } => *crossfade_frame_count,
        MixerState::None => 0,
    }
}
```

**Responsibility:** ❌ **INCORRECTLY** emits position events (should be PlaybackEngine's job)

---

#### 3. PlaybackEngine
**Location:** `wkmp-ap/src/playback/engine.rs`

**What it knows:**
- **QueueManager:** Which passages are current/next/queued
- **BufferManager:** Buffer states and fill levels
- **SharedState:** Global playback state (current passage, position, etc.)

**Position Tracking Fields:**
```rust
pub struct PlaybackEngine {
    // Queue manager (tracks current/next/queued)
    queue: Arc<RwLock<QueueManager>>,

    // Shared state
    state: Arc<SharedState>,

    // Position event channel sender
    // Mixer sends position events to handler via this channel
    position_event_tx: mpsc::UnboundedSender<PlaybackEvent>,

    // Position event channel receiver
    // Taken by position_event_handler on start
    position_event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<PlaybackEvent>>>>,

    // Current playback position
    // [ISSUE-8] Uses internal atomics for lock-free frame position updates
    position: PlaybackPosition,
}
```

**PlaybackPosition Struct (lines 36-48):**
```rust
struct PlaybackPosition {
    /// Current passage UUID (queue entry) - updated infrequently
    queue_entry_id: Arc<RwLock<Option<Uuid>>>,

    /// Current frame position in buffer - updated every loop iteration
    /// [ISSUE-8] AtomicU64 for lock-free updates in hot path
    frame_position: Arc<AtomicU64>,
}
```

**Current Responsibility:** ✅ **RECEIVES** position events from mixer, processes them in `position_event_handler()`

**Target Responsibility:** ✅ **EMITS** position events (not mixer)

---

#### 4. position_event_handler (PlaybackEngine method)
**Location:** `wkmp-ap/src/playback/engine.rs` lines 2478-2618

**What it does:**

1. **Receives PositionUpdate events** from legacy mixer
   ```rust
   match rx.recv().await {
       Some(PlaybackEvent::PositionUpdate { queue_entry_id, position_ms }) => {
           // Process event...
       }
   }
   ```

2. **Checks song boundaries** (multi-song passages)
   - Emits `CurrentSongChanged` event when crossing song boundary

3. **Emits PlaybackProgress events** (every 5 seconds by default)
   ```rust
   if position_ms >= last_progress_position_ms + progress_interval_ms {
       self.state.broadcast_event(WkmpEvent::PlaybackProgress {
           passage_id,
           position_ms,
           duration_ms,
           timestamp: chrono::Utc::now(),
       });
   }
   ```

4. **Updates SharedState.current_passage**
   ```rust
   self.state.set_current_passage(Some(CurrentPassage {
       queue_entry_id: current.queue_entry_id,
       passage_id: current.passage_id,
       position_ms,
       duration_ms,
   })).await;
   ```

**Responsibility:** ✅ Processes position events, emits derived events, updates shared state

---

#### 5. SharedState
**Location:** `wkmp-ap/src/state.rs`

**CurrentPassage Struct (lines 16-27):**
```rust
pub struct CurrentPassage {
    /// Queue entry ID
    pub queue_entry_id: Uuid,
    /// Passage ID (may be None for ephemeral passages)
    pub passage_id: Option<Uuid>,
    /// Current position in milliseconds
    pub position_ms: u64,
    /// Total duration in milliseconds
    pub duration_ms: u64,
}
```

**SharedState Struct (lines 34-46):**
```rust
pub struct SharedState {
    /// Current playback state (Playing or Paused)
    pub playback_state: RwLock<PlaybackState>,

    /// Currently playing passage (None if queue empty)
    pub current_passage: RwLock<Option<CurrentPassage>>,

    /// Master volume (0.0-1.0, system-level scale)
    pub volume: RwLock<f32>,

    /// Event broadcaster for SSE events
    pub event_tx: broadcast::Sender<WkmpEvent>,
}
```

**Key Methods:**
```rust
// Get current passage information
pub async fn get_current_passage(&self) -> Option<CurrentPassage>

// Set current passage
pub async fn set_current_passage(&self, passage: Option<CurrentPassage>)
```

**Responsibility:** ✅ Holds global playback state (what's playing, where we are)

---

## How Position Tracker Knows What's Playing (Current Architecture)

### The Complete Flow

```
Step 1: QueueManager knows what SHOULD be playing
  ├─ queue.current() → QueueEntry { queue_entry_id, passage_id, ... }
  └─ Updated by PlaybackEngine when advancing queue

Step 2: PlaybackEngine tells Mixer to start passage
  ├─ mixer.start_passage(queue_entry_id, passage_id, ...)
  └─ Mixer stores passage_id in MixerState (PROBLEM: mixer becomes stateful)

Step 3: Mixer emits position events (WRONG LOCATION)
  ├─ Every ~100ms (configurable interval)
  ├─ get_next_frame() increments frame counter
  ├─ When interval reached: emit PositionUpdate { queue_entry_id, position_ms }
  └─ Sends to position_event_handler via mpsc channel

Step 4: position_event_handler processes events
  ├─ Receives PositionUpdate events
  ├─ Checks song boundaries (multi-song passages)
  ├─ Emits PlaybackProgress events (every 5 seconds)
  └─ Updates SharedState.current_passage

Step 5: UI/API reads SharedState
  ├─ state.get_current_passage() → CurrentPassage { queue_entry_id, passage_id, position_ms, duration_ms }
  └─ Displayed to user
```

### Data Sources for "What's Playing"

| Component | Source of Truth | How It Knows |
|-----------|----------------|--------------|
| **QueueManager** | Queue entries | Loaded from database + in-memory updates |
| **Legacy Mixer** | Mixer state (passage_id) | PlaybackEngine calls `start_passage()` |
| **PlaybackEngine** | QueueManager | Reads `queue.current()` |
| **SharedState** | position_event_handler | Updated from PositionUpdate events |
| **UI/API** | SharedState | Reads `state.get_current_passage()` |

**Problem:** Legacy mixer duplicates passage tracking (stateful design)

---

## Target Architecture (PlaybackEngine Emits Events)

### Simplified Data Flow

```
QueueManager → PlaybackEngine → PositionUpdate Events → position_event_handler → SharedState
     ↓              ↓                                            ↓
   Tracks       Emits events                              Updates state
 queue_entry     periodically                            current_passage
```

### Key Changes

#### 1. Remove Position Event Logic from Mixer

**Current (Legacy Mixer):**
```rust
// ❌ Mixer tracks passage_id and frame_count
enum MixerState {
    SinglePassage { passage_id: Uuid, frame_count: usize, ... },
    Crossfading { current_passage_id: Uuid, crossfade_frame_count: usize, ... },
}

// ❌ Mixer emits position events
if self.frame_counter >= self.position_event_interval_frames {
    tx.send(PositionUpdate { queue_entry_id, position_ms });
}
```

**Target (Correct Mixer):**
```rust
// ✅ Mixer is stateless (no passage tracking)
pub enum MixerState {
    Playing,
    Paused,
}

// ✅ Mixer just mixes (no position events)
pub fn mix_single(&mut self, passage_buffer: &mut RingBuffer, output: &mut [f32]) -> Result<()> {
    // Read pre-faded samples, apply master volume, output
}
```

---

#### 2. Add Position Event Logic to PlaybackEngine

**Implementation Location:** In audio callback or similar hot path

**Required Fields:**
```rust
pub struct PlaybackEngine {
    // ... existing fields

    // Position tracking for event emission
    position_event_interval_frames: usize,  // NEW: e.g., 4410 frames = 100ms @ 44.1kHz
    total_frames_mixed: usize,              // NEW: Total frames output by mixer
    current_queue_entry_id: Option<Uuid>,   // NEW: Tracks which passage is playing
}
```

**Event Emission Logic:**
```rust
// In audio callback or playback loop (after mixer outputs frames)
fn after_mixer_output(&mut self, frames_output: usize) {
    self.total_frames_mixed += frames_output;

    // Check if interval elapsed
    if self.total_frames_mixed % self.position_event_interval_frames == 0 {
        if let Some(queue_entry_id) = self.current_queue_entry_id {
            // Calculate position in milliseconds
            let position_ms = (self.total_frames_mixed as u64 * 1000) / 44100;

            // Emit PositionUpdate event
            self.position_event_tx.try_send(PlaybackEvent::PositionUpdate {
                queue_entry_id,
                position_ms,
            }).ok();
        }
    }
}
```

**Source of queue_entry_id:**
```rust
// PlaybackEngine already knows current passage from QueueManager
let queue = self.queue.read().await;
let current = queue.current();
self.current_queue_entry_id = current.map(|e| e.queue_entry_id);
```

---

## Why PlaybackEngine Should Emit Position Events

### Architectural Reasons

1. **Separation of Concerns**
   - **Mixer:** Low-level audio processing (mix samples, apply volume)
   - **PlaybackEngine:** High-level orchestration (track state, emit events)

2. **Single Source of Truth**
   - PlaybackEngine already knows what's playing via QueueManager
   - Mixer shouldn't duplicate this state (stateful vs. stateless)

3. **SPEC016 Compliance**
   - Mixer should be stateless per [DBD-MIX-042]
   - Position tracking is state management (not mixing)

4. **Testability**
   - Stateless mixer is easy to test (pure function: samples in → samples out)
   - Stateful mixer is hard to test (requires setup, state verification)

### Practical Reasons

1. **Engine Already Has Position Tracking**
   ```rust
   struct PlaybackPosition {
       queue_entry_id: Arc<RwLock<Option<Uuid>>>,
       frame_position: Arc<AtomicU64>,  // Already exists!
   }
   ```

2. **Engine Already Has Queue Access**
   ```rust
   let queue = self.queue.read().await;
   let current = queue.current();  // Already knows what's playing!
   ```

3. **Engine Already Processes Position Events**
   - `position_event_handler()` already exists
   - Just need to emit from engine instead of receiving from mixer

---

## Migration Strategy

### Phase 1: Add Position Event Emission to PlaybackEngine

**Step 1:** Add position tracking fields
```rust
pub struct PlaybackEngine {
    // ... existing fields
    position_event_interval_frames: usize,
    total_frames_mixed: usize,
    current_queue_entry_id: Option<Uuid>,
}
```

**Step 2:** Load interval from settings
```rust
// In PlaybackEngine::new()
let interval_ms = load_position_interval(&db_pool).await.unwrap_or(100);
let position_event_interval_frames = (interval_ms as f32 / 1000.0 * 44100.0) as usize;
```

**Step 3:** Emit events in playback loop
```rust
// After mixer outputs frames
self.total_frames_mixed += frames_output;
if self.total_frames_mixed % self.position_event_interval_frames == 0 {
    // Emit PositionUpdate event
}
```

**Step 4:** Update current_queue_entry_id when advancing
```rust
// When starting new passage
self.current_queue_entry_id = Some(queue_entry.queue_entry_id);
```

### Phase 2: Remove Position Event Logic from Mixer Instantiation

**Step 1:** Remove event channel setup
```rust
// REMOVE:
// mixer.set_event_channel(position_event_tx.clone());
// mixer.set_position_event_interval_ms(interval_ms);
```

**Step 2:** Verify events still work
- Run application
- Check UI receives position updates
- Verify event frequency unchanged

### Phase 3: Migrate to Correct Mixer

**Step 1:** Replace CrossfadeMixer with Mixer
```rust
// OLD:
let mut mixer = CrossfadeMixer::new();

// NEW:
let mixer = Mixer::new(master_volume);
```

**Step 2:** Remove stateful mixer method calls
- No `start_passage()` calls (mixer doesn't track passages)
- No `get_current_passage_id()` calls
- Mixer becomes pure function: samples in → samples out

---

## Open Questions

### Q1: Where exactly does the "playback loop" call the mixer?

**Answer Needed:** Find where mixer.get_next_frame() or equivalent is called. This is where we'll add position event emission.

**Likely Location:** Audio callback or dedicated playback loop in PlaybackEngine.

### Q2: How does frame_position (AtomicU64) get updated?

**Answer Needed:** Check if PlaybackEngine.position.frame_position is already being updated.

**If YES:** Can use existing frame counter instead of adding new total_frames_mixed.
**If NO:** Need to add frame tracking in playback loop.

### Q3: What happens when passage changes (crossfade)?

**Answer Needed:** How is current_queue_entry_id updated during crossfade?

**Options:**
- Update when crossfade starts (current passage = fading out passage)
- Update when crossfade completes (current passage = faded in passage)
- Update at crossfade midpoint

**Decision Needed:** Match legacy mixer behavior for consistency.

---

## Summary

**Current Architecture:**
- ❌ Legacy mixer emits position events (architectural violation)
- ❌ Mixer is stateful (tracks passage_id, frame_count)
- ❌ Duplicates state (mixer AND engine know what's playing)

**Target Architecture:**
- ✅ PlaybackEngine emits position events (correct location)
- ✅ Mixer is stateless (pure mixing function)
- ✅ Single source of truth (engine knows what's playing via QueueManager)

**Next Step:** Find audio callback / playback loop to implement position event emission.

---

**Date:** 2025-01-30
**Author:** Claude (PLAN014 Implementation)

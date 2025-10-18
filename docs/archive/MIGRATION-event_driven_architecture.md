# MIGRATION: Event-Driven Position Tracking Implementation Plan

**📋 TIER R - REVIEW & CHANGE CONTROL**

Comprehensive migration plan for implementing event-driven position tracking in wkmp-ap module. This is an operational guide (mutable during implementation). See [Document Hierarchy](GOV001-document_hierarchy.md) for Tier R classification details.

**Authority:** Operational guidance - updated as implementation progresses

**Status:** Active (implementation in progress)
**Date:** 2025-10-18
**Type:** Migration Plan
**Related Documents:**
- [REV002-event_driven_architecture_update.md](REV002-event_driven_architecture_update.md) - Architecture change baseline
- [CHANGELOG-event_driven_architecture.md](CHANGELOG-event_driven_architecture.md) - Documentation changes audit
- [ADDENDUM-interval_configurability.md](ADDENDUM-interval_configurability.md) - Interval configuration specs
- [SPEC001-architecture.md](SPEC001-architecture.md) - Architecture specification
- [SPEC011-event_system.md](SPEC011-event_system.md) - Event system design

---

## Executive Summary

**Goal:** Replace timer-driven position polling with true event-driven architecture for position tracking and song boundary detection in the wkmp-ap audio player.

**Scope:**
- wkmp-ap module only (no changes to other microservices)
- 3 new modules (~400 LOC)
- 2 modified modules (~150 LOC changes)
- Zero functional requirement changes (all user-visible behavior preserved)

**Benefits:**
- **50% CPU reduction** (eliminate polling overhead)
- **20x latency improvement** (0-500ms → <50ms for song boundaries)
- **Architectural consistency** (event-driven throughout)
- **Sample-accurate** (events tied to actual playback frames)

**Timeline:** 2-3 days for implementation, 1-2 days for testing/validation

---

## Current State Analysis

### Existing Architecture (Timer-Driven)

#### Position Tracking Loop
**Location:** `wkmp-ap/src/playback/engine.rs:1144-1220`

```rust
async fn position_tracking_loop(&self) {
    let mut tick = interval(Duration::from_millis(1000)); // 1 second polling

    loop {
        tick.tick().await;

        // Poll mixer for position
        let mixer_position_frames = mixer.get_position();

        // Update shared state
        self.state.set_current_passage(...);

        // Emit PlaybackProgress every 5 iterations (5 seconds)
        if progress_counter >= 5 {
            self.state.broadcast_event(WkmpEvent::PlaybackProgress { ... });
        }
    }
}
```

**Issues:**
1. **Polling overhead**: Wakes up every 1 second regardless of playback state
2. **No song boundary detection**: `CurrentSongChanged` event never emitted
3. **Wall-clock time**: Uses timer, not tied to actual audio generation
4. **Fixed interval**: Cannot be configured via database settings

#### Missing Components

- ❌ **No internal event system**: No `PlaybackEvent` types
- ❌ **No song timeline logic**: No boundary detection
- ❌ **No passage-song loading**: Cannot read song timeline from database
- ❌ **No event emission from mixer**: Mixer just generates frames silently

#### What Works (Must Preserve)

- ✅ `PlaybackProgress` SSE event emission (~5 seconds)
- ✅ Position tracking and state updates
- ✅ Crossfade timing calculations
- ✅ All playback control (play/pause/seek/skip)

---

## Target State Definition

### Event-Driven Architecture

#### Data Flow

```
[1] Mixer Thread (Audio Generation)
    └─> mixer.get_next_frame()
        └─> Every position_event_interval_ms (default: 1000ms)
            └─> SEND PositionUpdate { position_ms, queue_entry_id }
                └─> MPSC channel (capacity: 100 events)

[2] Position Event Handler (Reactive)
    └─> RECEIVE PositionUpdate
        ├─> Check song_timeline.check_boundary(position_ms)
        │   └─> If boundary crossed:
        │       └─> BROADCAST CurrentSongChanged SSE event
        └─> Check if PlaybackProgress interval elapsed (5 seconds)
            └─> BROADCAST PlaybackProgress SSE event
```

#### New Components

| Component | File | LOC | Purpose |
|-----------|------|-----|---------|
| `PlaybackEvent` enum | `playback/events.rs` | ~50 | Internal event types (not SSE) |
| `SongTimeline` struct | `playback/song_timeline.rs` | ~200 | Boundary detection logic |
| `load_song_timeline()` | `db/passage_songs.rs` | ~100 | Load from `passage_songs` table |
| Position event channel | `playback/engine.rs` | ~20 | MPSC sender/receiver |
| `position_event_handler()` | `playback/engine.rs` | ~80 | Event-driven handler task |

**Total New Code:** ~450 LOC

#### Modified Components

| Component | File | Change | LOC |
|-----------|------|--------|-----|
| `CrossfadeMixer` | `playback/pipeline/mixer.rs` | Emit events in `get_next_frame()` | ~50 |
| `PlaybackEngine` | `playback/engine.rs` | Replace timer loop with event handler | ~100 |

**Total Modified Code:** ~150 LOC

---

## Implementation Phases

### Phase 1: Foundation (New Modules) - Day 1

**Goal:** Create new modules without modifying existing code (zero risk, parallel work)

#### Task 1.1: Create Internal Event Types
**File:** `wkmp-ap/src/playback/events.rs`

```rust
//! Internal playback events (not exposed via SSE)
//!
//! [SPEC011-event_system.md] Internal vs External Events

use uuid::Uuid;

/// Internal playback events (mixer → engine communication)
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    /// Position update from mixer
    ///
    /// Emitted periodically by mixer during frame generation.
    /// Frequency controlled by `position_event_interval_ms` setting.
    PositionUpdate {
        queue_entry_id: Uuid,
        position_ms: u64,
    },

    /// Playback state changed (future use)
    StateChanged {
        // Reserved for future implementation
    },
}
```

**Testing:** Unit tests for event creation and cloning

**Estimated Time:** 1 hour

#### Task 1.2: Create Song Timeline Module
**File:** `wkmp-ap/src/playback/song_timeline.rs`

```rust
//! Song timeline boundary detection
//!
//! [ARCH-SNGC-041] Song Timeline Data Structure
//! [ARCH-SNGC-042] Efficient Boundary Detection Algorithm

use uuid::Uuid;

/// Song timeline entry
#[derive(Debug, Clone)]
pub struct SongTimelineEntry {
    pub song_id: Option<Uuid>,  // None for gaps
    pub start_time_ms: u64,
    pub end_time_ms: u64,
}

/// Song timeline for a passage
#[derive(Debug, Clone)]
pub struct SongTimeline {
    entries: Vec<SongTimelineEntry>,  // Sorted by start_time_ms
    current_index: usize,             // Cache for current entry
}

impl SongTimeline {
    /// Create new song timeline
    pub fn new(mut entries: Vec<SongTimelineEntry>) -> Self {
        // Sort by start_time_ms
        entries.sort_by_key(|e| e.start_time_ms);

        Self {
            entries,
            current_index: 0,
        }
    }

    /// Check if position crossed a song boundary
    ///
    /// [ARCH-SNGC-042] O(n) worst-case, but typically O(1) with caching
    ///
    /// Returns (crossed_boundary, new_song_id) tuple
    pub fn check_boundary(&mut self, position_ms: u64) -> (bool, Option<Uuid>) {
        if self.entries.is_empty() {
            return (false, None);
        }

        // Check current cached entry first (hot path)
        if self.current_index < self.entries.len() {
            let entry = &self.entries[self.current_index];

            if position_ms >= entry.start_time_ms && position_ms < entry.end_time_ms {
                // Still in same entry, no boundary crossed
                return (false, entry.song_id);
            }
        }

        // Position changed entries - search for new entry
        let old_index = self.current_index;

        for (i, entry) in self.entries.iter().enumerate() {
            if position_ms >= entry.start_time_ms && position_ms < entry.end_time_ms {
                self.current_index = i;
                let crossed = i != old_index;
                return (crossed, entry.song_id);
            }
        }

        // Position is in a gap (not within any song)
        self.current_index = self.entries.len(); // Mark as "gap"
        (true, None)
    }

    /// Get current song ID at position (without boundary check)
    pub fn get_current_song(&self, position_ms: u64) -> Option<Uuid> {
        for entry in &self.entries {
            if position_ms >= entry.start_time_ms && position_ms < entry.end_time_ms {
                return entry.song_id;
            }
        }
        None
    }
}
```

**Testing:**
- Unit test: Empty timeline
- Unit test: Single song
- Unit test: Multiple songs with gaps
- Unit test: Boundary crossing detection
- Unit test: Forward seek across multiple songs
- Unit test: Backward seek

**Estimated Time:** 3 hours

#### Task 1.3: Create Database Loader
**File:** `wkmp-ap/src/db/passage_songs.rs`

```rust
//! Load song timeline from passage_songs table
//!
//! [DB-PS-010] passage_songs table schema

use crate::error::Result;
use crate::playback::song_timeline::{SongTimeline, SongTimelineEntry};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

/// Load song timeline for a passage
///
/// Queries `passage_songs` table and builds sorted timeline.
/// Returns empty timeline if no songs (full passage is a gap).
pub async fn load_song_timeline(
    pool: &Pool<Sqlite>,
    passage_id: Uuid,
) -> Result<SongTimeline> {
    let rows = sqlx::query!(
        r#"
        SELECT
            song_guid,
            start_time_ms,
            end_time_ms
        FROM passage_songs
        WHERE passage_guid = ?
        ORDER BY start_time_ms ASC
        "#,
        passage_id.to_string()
    )
    .fetch_all(pool)
    .await?;

    let entries: Vec<SongTimelineEntry> = rows
        .into_iter()
        .map(|row| {
            let song_id = row.song_guid
                .and_then(|s| Uuid::parse_str(&s).ok());

            SongTimelineEntry {
                song_id,
                start_time_ms: row.start_time_ms as u64,
                end_time_ms: row.end_time_ms as u64,
            }
        })
        .collect();

    Ok(SongTimeline::new(entries))
}
```

**Testing:**
- Integration test with in-memory database
- Test: Empty passage_songs
- Test: Single song
- Test: Multiple songs

**Estimated Time:** 2 hours

#### Task 1.4: Update Module Declarations
**Files:**
- `wkmp-ap/src/playback/mod.rs` (add `pub mod events; pub mod song_timeline;`)
- `wkmp-ap/src/db/mod.rs` (add `pub mod passage_songs;`)

**Estimated Time:** 15 minutes

**Phase 1 Total Time:** ~6.5 hours

---

### Phase 2: Event Emission (Mixer Modification) - Day 1-2

**Goal:** Make mixer emit position events without changing behavior

#### Task 2.1: Add Event Channel to Mixer

**Modification:** `wkmp-ap/src/playback/pipeline/mixer.rs`

```rust
use tokio::sync::mpsc;
use crate::playback::events::PlaybackEvent;

pub struct CrossfadeMixer {
    state: MixerState,
    sample_rate: u32,

    // Event emission
    event_tx: Option<mpsc::UnboundedSender<PlaybackEvent>>,
    frame_counter: usize,
    position_event_interval_frames: usize,  // From database setting
}

impl CrossfadeMixer {
    pub fn new() -> Self {
        CrossfadeMixer {
            state: MixerState::None,
            sample_rate: STANDARD_SAMPLE_RATE,
            event_tx: None,
            frame_counter: 0,
            position_event_interval_frames: 44100, // Default: 1 second
        }
    }

    /// Set event channel for position updates
    pub fn set_event_channel(&mut self, tx: mpsc::UnboundedSender<PlaybackEvent>) {
        self.event_tx = Some(tx);
    }

    /// Set position event interval (from database setting)
    pub fn set_position_event_interval_ms(&mut self, interval_ms: u32) {
        self.position_event_interval_frames =
            ((interval_ms as f32 / 1000.0) * self.sample_rate as f32) as usize;
    }
}
```

**Testing:** Unit tests for configuration methods

**Estimated Time:** 1 hour

#### Task 2.2: Emit Events in get_next_frame()

```rust
pub async fn get_next_frame(&mut self) -> AudioFrame {
    // Existing frame generation logic...
    let frame = match &mut self.state {
        // ... existing code ...
    };

    // NEW: Emit position event if interval elapsed
    self.frame_counter += 1;
    if self.frame_counter >= self.position_event_interval_frames {
        self.frame_counter = 0; // Reset counter

        if let Some(tx) = &self.event_tx {
            if let Some(passage_id) = self.get_current_passage_id() {
                let position_ms = self.calculate_position_ms();

                // Non-blocking send
                let _ = tx.send(PlaybackEvent::PositionUpdate {
                    queue_entry_id: passage_id,
                    position_ms,
                });
            }
        }
    }

    frame
}

/// Calculate current position in milliseconds
fn calculate_position_ms(&self) -> u64 {
    let position_frames = self.get_position();
    (position_frames as u64 * 1000) / self.sample_rate as u64
}
```

**Testing:**
- Unit test: Verify event emission frequency
- Unit test: Verify event contents
- Unit test: Verify non-blocking (no deadlock)

**Estimated Time:** 2 hours

**Phase 2 Total Time:** ~3 hours

---

### Phase 3: Event Handler (Engine Modification) - Day 2

**Goal:** Replace timer loop with event-driven handler

#### Task 3.1: Create Position Event Channel in Engine

**Modification:** `wkmp-ap/src/playback/engine.rs`

```rust
use tokio::sync::mpsc;
use crate::playback::events::PlaybackEvent;
use crate::playback::song_timeline::SongTimeline;

pub struct PlaybackEngine {
    // ... existing fields ...

    /// Position event channel (mixer → handler)
    position_event_tx: mpsc::UnboundedSender<PlaybackEvent>,
    position_event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<PlaybackEvent>>>>,

    /// Song timeline for current passage
    current_song_timeline: Arc<RwLock<Option<SongTimeline>>>,
}

impl PlaybackEngine {
    pub async fn new(db_pool: Pool<Sqlite>, state: Arc<SharedState>) -> Result<Self> {
        // ... existing initialization ...

        // Create position event channel
        let (tx, rx) = mpsc::unbounded_channel();

        // Configure mixer with event channel
        let mut mixer = CrossfadeMixer::new();
        mixer.set_event_channel(tx.clone());

        // Load position_event_interval_ms from database
        let interval_ms = load_position_event_interval(&db_pool).await?;
        mixer.set_position_event_interval_ms(interval_ms);

        Ok(Self {
            // ... existing fields ...
            position_event_tx: tx,
            position_event_rx: Arc::new(RwLock::new(Some(rx))),
            current_song_timeline: Arc::new(RwLock::new(None)),
            mixer: Arc::new(RwLock::new(mixer)),
        })
    }
}
```

**Testing:** Integration test for channel creation

**Estimated Time:** 1 hour

#### Task 3.2: Create Position Event Handler

```rust
/// Position event handler (replaces position_tracking_loop)
///
/// [ARCH-SNGC-030] Event-driven position tracking
async fn position_event_handler(&self) {
    // Take ownership of receiver (only one handler allowed)
    let mut rx = self.position_event_rx.write().await.take()
        .expect("Position event receiver already taken");

    let mut last_progress_position_ms = 0u64;
    let progress_interval_ms = load_progress_interval(&self.db_pool)
        .await
        .unwrap_or(5000); // Default: 5 seconds

    loop {
        // Wait for position event
        match rx.recv().await {
            Some(PlaybackEvent::PositionUpdate { queue_entry_id, position_ms }) => {
                // [1] Check song boundary
                let mut timeline = self.current_song_timeline.write().await;
                if let Some(timeline) = timeline.as_mut() {
                    let (crossed, new_song_id) = timeline.check_boundary(position_ms);

                    if crossed {
                        // Emit CurrentSongChanged event
                        self.state.broadcast_event(WkmpEvent::CurrentSongChanged {
                            passage_id: queue_entry_id,
                            song_id: new_song_id,
                            position_ms,
                            timestamp: chrono::Utc::now(),
                        });
                    }
                }
                drop(timeline);

                // [2] Check if PlaybackProgress interval elapsed
                if position_ms - last_progress_position_ms >= progress_interval_ms {
                    last_progress_position_ms = position_ms;

                    // Get duration from buffer
                    if let Some(buffer_ref) = self.buffer_manager.get_buffer(queue_entry_id).await {
                        let buffer = buffer_ref.read().await;
                        let duration_ms = buffer.duration_ms();

                        self.state.broadcast_event(WkmpEvent::PlaybackProgress {
                            passage_id: queue_entry_id,
                            position_ms,
                            duration_ms,
                            timestamp: chrono::Utc::now(),
                        });
                    }
                }

                // [3] Update shared state
                // ... existing position tracking logic ...
            }

            Some(PlaybackEvent::StateChanged { .. }) => {
                // Future: Handle state change events
            }

            None => {
                // Channel closed, handler should exit
                break;
            }
        }
    }
}
```

**Testing:**
- Integration test: Event reception
- Integration test: Song boundary detection
- Integration test: PlaybackProgress emission

**Estimated Time:** 3 hours

#### Task 3.3: Load Song Timeline on Passage Start

```rust
// In playback_loop(), when starting a passage:
if mixer.get_current_passage_id().is_none() {
    // ... existing buffer loading ...

    // NEW: Load song timeline
    if let Some(passage_id) = current.passage_id {
        match crate::db::passage_songs::load_song_timeline(&self.db_pool, passage_id).await {
            Ok(timeline) => {
                *self.current_song_timeline.write().await = Some(timeline);

                // Emit initial CurrentSongChanged if passage starts within a song
                let position_ms = 0; // Passage just started
                let song_id = timeline.get_current_song(position_ms);

                self.state.broadcast_event(WkmpEvent::CurrentSongChanged {
                    passage_id,
                    song_id,
                    position_ms,
                    timestamp: chrono::Utc::now(),
                });
            }
            Err(e) => {
                warn!("Failed to load song timeline for passage {}: {}", passage_id, e);
                // Continue playback without song boundary detection
            }
        }
    }

    // ... start mixer ...
}
```

**Testing:** Integration test for timeline loading

**Estimated Time:** 1 hour

#### Task 3.4: Start Event Handler in Engine::start()

```rust
pub async fn start(&self) -> Result<()> {
    // ... existing code ...

    // Start position event handler (replaces position_tracking_loop)
    let self_clone = self.clone_handles();
    tokio::spawn(async move {
        self_clone.position_event_handler().await;
    });

    // ... rest of existing code ...
}
```

**Testing:** Integration test for handler lifecycle

**Estimated Time:** 30 minutes

#### Task 3.5: Remove Old position_tracking_loop()

**Delete:** `engine.rs:1144-1220`

**Estimated Time:** 5 minutes

**Phase 3 Total Time:** ~5.5 hours

---

### Phase 4: Database Configuration - Day 2

**Goal:** Load interval settings from database

#### Task 4.1: Add Database Setting Loaders

**File:** `wkmp-ap/src/db/settings.rs`

```rust
/// Load position_event_interval_ms from settings table
pub async fn load_position_event_interval(pool: &Pool<Sqlite>) -> Result<u32> {
    let row = sqlx::query!(
        "SELECT value FROM settings WHERE key = 'position_event_interval_ms'"
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let value: u32 = row.value
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000); // Default: 1 second
            Ok(value)
        }
        None => Ok(1000), // Default if setting not found
    }
}

/// Load playback_progress_interval_ms from settings table
pub async fn load_progress_interval(pool: &Pool<Sqlite>) -> Result<u64> {
    let row = sqlx::query!(
        "SELECT value FROM settings WHERE key = 'playback_progress_interval_ms'"
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let value: u64 = row.value
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000); // Default: 5 seconds
            Ok(value)
        }
        None => Ok(5000), // Default if setting not found
    }
}
```

**Testing:** Integration tests with in-memory database

**Estimated Time:** 1 hour

**Phase 4 Total Time:** ~1 hour

---

### Phase 5: Testing & Validation - Day 3

**Goal:** Comprehensive testing before production deployment

#### Task 5.1: Unit Tests

**Coverage:**
- `events.rs`: Event creation, cloning
- `song_timeline.rs`: Boundary detection algorithms (6 test cases)
- `mixer.rs`: Event emission frequency, contents
- `passage_songs.rs`: Database loading (3 test cases)

**Estimated Time:** 2 hours

#### Task 5.2: Integration Tests

**Test Scenarios:**
1. Single passage, no songs (gap only)
2. Single passage, single song
3. Single passage, multiple songs with gaps
4. Song boundary crossing during playback
5. Seek across song boundaries
6. Crossfade between passages (verify timeline reload)

**Estimated Time:** 3 hours

#### Task 5.3: Manual Testing

**Test Cases:**
1. Enqueue test file with multi-song passage
2. Verify `CurrentSongChanged` SSE events in browser
3. Verify `PlaybackProgress` SSE events (5-second interval)
4. Seek and verify immediate boundary check
5. Skip and verify timeline reload
6. Monitor CPU usage (verify <1%)

**Estimated Time:** 2 hours

#### Task 5.4: Performance Validation

**Metrics to Verify:**
- CPU usage: <1% (vs ~2% with old timer loops)
- Song boundary latency: <50ms (measure with debug logs)
- Event emission frequency: ~1 event/second (verify with logs)
- Memory overhead: <10KB (measure with process stats)

**Estimated Time:** 1 hour

**Phase 5 Total Time:** ~8 hours

---

## Risk Assessment

### High Risk Items

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **MPSC channel deadlock** | Critical - audio stops | Medium | Use `UnboundedSender` + `try_send()` (non-blocking) |
| **Position event flood** | High - channel overflow | Low | Verify frame counter logic in tests |
| **Song timeline lookup O(n)** | Medium - CPU spike | Low | Cache current index (already in design) |
| **Database load failure** | Medium - no events | Medium | Graceful fallback (continue without boundaries) |

### Medium Risk Items

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **Backward compatibility** | Medium - old data | Low | Handle missing `passage_songs` table gracefully |
| **Event handler crash** | Medium - no events | Low | Comprehensive error handling + logging |
| **Race condition on timeline** | Low - incorrect events | Low | Use `RwLock` for timeline access |

### Low Risk Items

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **Configuration load failure** | Low - use defaults | Low | Always provide fallback defaults |
| **Event serialization** | None - not serialized | N/A | Internal events only (not SSE) |

---

## Testing Strategy

### Test Pyramid

```
           ┌─────────────┐
           │   Manual    │  ~10% effort
           │  Testing    │  (2 hours)
           └─────────────┘
         ┌────────────────────┐
         │  Integration Tests │  ~30% effort
         │   (6 scenarios)    │  (3 hours)
         └────────────────────┘
   ┌──────────────────────────────────┐
   │        Unit Tests                │  ~60% effort
   │  (events, timeline, emission)    │  (2 hours)
   └──────────────────────────────────┘
```

### Validation Phases

#### Phase A: Isolated Module Testing
- Run unit tests for each new module independently
- Verify: No panics, all edge cases covered
- **Gate:** All unit tests pass (100% of new code)

#### Phase B: Integration Testing
- Run integration tests with in-memory database
- Verify: Event flow end-to-end
- **Gate:** All integration scenarios pass

#### Phase C: Manual Validation
- Run wkmp-ap with real audio files
- Monitor SSE events in browser DevTools
- **Gate:** Visual confirmation of `CurrentSongChanged` events

#### Phase D: Performance Testing
- Run with 100-song passage (worst-case timeline size)
- Monitor CPU, memory, latency
- **Gate:** CPU <1%, latency <50ms

---

## Rollback Plan

### Immediate Rollback (Within 1 Hour)

If critical issues discovered during Phase C testing:

1. **Revert Git Commit**
   ```bash
   git revert <event-driven-commit-hash>
   ```

2. **Verify Old Behavior**
   - Check `position_tracking_loop()` is back
   - Verify `PlaybackProgress` events work
   - Confirm no `CurrentSongChanged` events (expected)

3. **Root Cause Analysis**
   - Review logs for panics/errors
   - Check MPSC channel statistics
   - Profile CPU usage

**Rollback Time:** <15 minutes

### Delayed Rollback (After Production Deployment)

If issues discovered in production (unlikely, but prepared):

1. **Emergency Patch**
   - Deploy previous version binary
   - Update documentation to mark as "delayed"

2. **Investigation**
   - Collect production logs
   - Reproduce issue in staging
   - Fix and re-deploy

**Rollback SLA:** <1 hour from issue detection

---

## Detailed Task Breakdown

### Implementation Checklist

#### Phase 1: Foundation
- [ ] Create `wkmp-ap/src/playback/events.rs`
  - [ ] Define `PlaybackEvent` enum
  - [ ] Add documentation comments
  - [ ] Write unit tests
- [ ] Create `wkmp-ap/src/playback/song_timeline.rs`
  - [ ] Define `SongTimelineEntry` struct
  - [ ] Define `SongTimeline` struct
  - [ ] Implement `check_boundary()` method
  - [ ] Implement `get_current_song()` method
  - [ ] Write 6 unit tests (empty, single, multiple, forward seek, backward seek, gaps)
- [ ] Create `wkmp-ap/src/db/passage_songs.rs`
  - [ ] Implement `load_song_timeline()` function
  - [ ] Add error handling
  - [ ] Write 3 integration tests
- [ ] Update module declarations
  - [ ] `playback/mod.rs`: Add `pub mod events;` and `pub mod song_timeline;`
  - [ ] `db/mod.rs`: Add `pub mod passage_songs;`

#### Phase 2: Event Emission
- [ ] Modify `wkmp-ap/src/playback/pipeline/mixer.rs`
  - [ ] Add `event_tx`, `frame_counter`, `position_event_interval_frames` fields
  - [ ] Add `set_event_channel()` method
  - [ ] Add `set_position_event_interval_ms()` method
  - [ ] Add `calculate_position_ms()` helper
  - [ ] Modify `get_next_frame()` to emit events
  - [ ] Write 3 unit tests
- [ ] Verify compilation
- [ ] Run unit tests

#### Phase 3: Event Handler
- [ ] Modify `wkmp-ap/src/playback/engine.rs`
  - [ ] Add `position_event_tx`, `position_event_rx`, `current_song_timeline` fields
  - [ ] Update `new()` to create channel and configure mixer
  - [ ] Implement `position_event_handler()` method
  - [ ] Update `start()` to spawn event handler
  - [ ] Update `playback_loop()` to load song timeline on passage start
  - [ ] Delete `position_tracking_loop()` method (lines 1144-1220)
  - [ ] Write 3 integration tests
- [ ] Verify compilation
- [ ] Run integration tests

#### Phase 4: Database Configuration
- [ ] Modify `wkmp-ap/src/db/settings.rs`
  - [ ] Add `load_position_event_interval()` function
  - [ ] Add `load_progress_interval()` function
  - [ ] Write 2 integration tests
- [ ] Update `engine.rs` to call loaders
- [ ] Verify compilation

#### Phase 5: Testing
- [ ] Run all unit tests
  - [ ] `cargo test --package wkmp-ap events::`
  - [ ] `cargo test --package wkmp-ap song_timeline::`
  - [ ] `cargo test --package wkmp-ap mixer::`
  - [ ] `cargo test --package wkmp-ap passage_songs::`
- [ ] Run all integration tests
  - [ ] 6 integration scenarios
- [ ] Manual testing
  - [ ] Test case 1: Single song passage
  - [ ] Test case 2: Multi-song passage
  - [ ] Test case 3: Seek across boundaries
  - [ ] Test case 4: Skip to next passage
  - [ ] Test case 5: Crossfade timeline reload
- [ ] Performance validation
  - [ ] Measure CPU usage
  - [ ] Measure latency
  - [ ] Verify event frequency

---

## Dependencies

### External Dependencies
- ✅ `tokio::sync::mpsc` (already in Cargo.toml)
- ✅ `sqlx` (already in Cargo.toml)
- ✅ `uuid` (already in Cargo.toml)

### Internal Dependencies
- ✅ `wkmp-common::events::WkmpEvent` (already implemented)
- ⚠️ `passage_songs` database table (must exist)
  - **Action Required:** Check if table exists in production database
  - **Fallback:** Gracefully handle missing table (no song boundaries)

### Documentation Dependencies
- ✅ [SPEC001-architecture.md](SPEC001-architecture.md) - Already updated
- ✅ [SPEC011-event_system.md](SPEC011-event_system.md) - Already updated
- ✅ [IMPL001-database_schema.md](IMPL001-database_schema.md) - Already updated
- ✅ [REV002](REV002-event_driven_architecture_update.md) - Baseline document

---

## Success Criteria

### Functional Requirements (Must All Pass)

- [x] `PlaybackProgress` SSE event emitted every ~5 seconds (configurable)
- [ ] `CurrentSongChanged` SSE event emitted when song boundary crossed
- [ ] `CurrentSongChanged` emitted immediately on passage start (if within song)
- [ ] Song boundary detection latency <50ms
- [ ] Position tracking accuracy maintained (±50ms)
- [ ] All playback controls work (play/pause/seek/skip)
- [ ] Crossfade behavior unchanged

### Performance Requirements

- [ ] CPU usage <1% (down from ~2%)
- [ ] Memory overhead <10KB
- [ ] No audio glitches/dropouts
- [ ] Event emission frequency ~1 event/second (default)

### Code Quality Requirements

- [ ] All unit tests pass (100% new code coverage)
- [ ] All integration tests pass
- [ ] No compiler warnings
- [ ] No `unsafe` code
- [ ] All public APIs documented
- [ ] Error handling for all failure modes

---

## Timeline Estimate

| Phase | Duration | Prerequisites |
|-------|----------|---------------|
| Phase 1: Foundation | 6.5 hours | None (can start immediately) |
| Phase 2: Event Emission | 3 hours | Phase 1 complete |
| Phase 3: Event Handler | 5.5 hours | Phase 1 & 2 complete |
| Phase 4: Database Config | 1 hour | Phase 1 complete |
| Phase 5: Testing | 8 hours | Phase 1-4 complete |

**Total Implementation Time:** 24 hours (3 full working days)

**Recommended Schedule:**
- **Day 1 (8h):** Phase 1 (6.5h) + Phase 2 start (1.5h)
- **Day 2 (8h):** Phase 2 finish (1.5h) + Phase 3 (5.5h) + Phase 4 (1h)
- **Day 3 (8h):** Phase 5 (8h testing/validation)

---

## Monitoring & Observability

### Log Points to Add

```rust
// In mixer.rs::get_next_frame()
if self.frame_counter == 0 && self.event_tx.is_some() {
    debug!("Emitting PositionUpdate: position={}ms, queue_entry={}",
           position_ms, queue_entry_id);
}

// In engine.rs::position_event_handler()
if crossed {
    info!("Song boundary crossed: old_song={:?}, new_song={:?}, position={}ms",
          old_song_id, new_song_id, position_ms);
}

// In song_timeline.rs::check_boundary()
if crossed {
    trace!("Boundary detected: index {} -> {}, song {:?}",
           old_index, self.current_index, song_id);
}
```

### Metrics to Track

1. **Event Frequency**
   - Metric: `position_events_per_second`
   - Expected: ~1.0 (±0.1)

2. **Song Boundary Latency**
   - Metric: `song_boundary_detection_latency_ms`
   - Expected: <50ms

3. **Timeline Lookup Performance**
   - Metric: `song_timeline_lookup_us`
   - Expected: <10µs (with caching)

4. **Channel Overflow**
   - Metric: `position_event_send_errors`
   - Expected: 0 (UnboundedSender should never overflow)

---

## Post-Implementation Tasks

### Phase 1: Validation (Week 1)
- [ ] Deploy to staging environment
- [ ] Monitor for 48 hours
- [ ] Verify all metrics within expected ranges
- [ ] Collect user feedback

### Phase 2: Production Deployment (Week 2)
- [ ] Deploy to production
- [ ] Monitor closely for 24 hours
- [ ] Verify SSE events in production logs
- [ ] Document any edge cases discovered

### Phase 3: Optimization (Week 3-4)
- [ ] Profile timeline lookup performance
- [ ] Optimize event handler if needed
- [ ] Add additional metrics if gaps found
- [ ] Update documentation with production learnings

### Phase 4: Documentation Cleanup (Week 4)
- [ ] Archive REV002 as complete
- [ ] Update CHANGELOG with final statistics
- [ ] Remove ADDENDUM (integrate into SPEC/IMPL docs)
- [ ] Update EXEC001 implementation order

---

## Appendix A: File Modification Summary

### New Files (3)
1. `wkmp-ap/src/playback/events.rs` (~50 LOC)
2. `wkmp-ap/src/playback/song_timeline.rs` (~200 LOC)
3. `wkmp-ap/src/db/passage_songs.rs` (~100 LOC)

### Modified Files (4)
1. `wkmp-ap/src/playback/pipeline/mixer.rs` (+50 LOC, ~5 methods added)
2. `wkmp-ap/src/playback/engine.rs` (+80 LOC, -76 LOC, 1 method replaced)
3. `wkmp-ap/src/db/settings.rs` (+30 LOC, 2 functions added)
4. `wkmp-ap/src/playback/mod.rs` (+2 LOC, module declarations)
5. `wkmp-ap/src/db/mod.rs` (+1 LOC, module declaration)

### Total LOC Change
- **Added:** ~515 LOC
- **Removed:** ~76 LOC (old position_tracking_loop)
- **Net Change:** +439 LOC

---

## Appendix B: Database Schema Requirements

### Required Table: `passage_songs`

```sql
CREATE TABLE passage_songs (
    guid TEXT PRIMARY KEY NOT NULL,
    passage_guid TEXT NOT NULL,
    song_guid TEXT,  -- NULL for gaps
    start_time_ms INTEGER NOT NULL,
    end_time_ms INTEGER NOT NULL,
    FOREIGN KEY (passage_guid) REFERENCES passages(guid) ON DELETE CASCADE,
    FOREIGN KEY (song_guid) REFERENCES songs(guid) ON DELETE SET NULL
);

CREATE INDEX idx_passage_songs_passage ON passage_songs(passage_guid);
CREATE INDEX idx_passage_songs_time ON passage_songs(passage_guid, start_time_ms);
```

**Migration Check:**
```bash
# Check if table exists in production database
sqlite3 wkmp.db "SELECT name FROM sqlite_master WHERE type='table' AND name='passage_songs';"
```

**Fallback Strategy:**
If table doesn't exist:
- `load_song_timeline()` returns empty timeline
- No `CurrentSongChanged` events emitted
- Playback continues normally (degraded feature, not error)

---

## Appendix C: Configuration Settings

### Database Settings (settings table)

| Key | Default | Range | Description |
|-----|---------|-------|-------------|
| `position_event_interval_ms` | 1000 | 100-5000 | How often mixer emits PositionUpdate events |
| `playback_progress_interval_ms` | 5000 | 1000-60000 | How often PlaybackProgress SSE emitted |

**Load Priority:**
1. Database value (if exists)
2. Default value (fallback)

**Runtime Reconfiguration:**
- ⚠️ Not supported in v1 (requires engine restart)
- 📋 Future enhancement: Add API endpoint to update settings

---

**End of Migration Plan**

**Status:** Ready for Implementation
**Next Step:** Begin Phase 1 (Foundation module creation)

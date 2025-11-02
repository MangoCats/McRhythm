# SPEC028: Playback Loop Orchestration

**Version:** 1.0
**Status:** Active
**Last Updated:** 2025-11-02
**Related Documents:**
- [SPEC001: Architecture](SPEC001-architecture.md) - System overview
- [SPEC013: Single-Stream Playback](SPEC013-single_stream_playback.md) - Audio pipeline design
- [SPEC016: Mixer Architecture](SPEC016-mixer_architecture.md) - Mixer implementation
- [ADR-002: Event-Driven Position Tracking](ADR-002-event_driven_position_tracking.md) - Position tracking rationale

---

## 1. Executive Summary

This document explains the **core orchestration loop** in wkmp-ap that coordinates:
- Queue management (every 100ms)
- Mixer thread (batch mixing with ring buffer)
- Decoder worker (background decode)
- Buffer manager (PCM buffer lifecycle)
- Event system (marker-driven playback events)

**Key Insight:** The 100ms playback loop checks **in-memory queue state** only — no database polling. All database access is event-driven.

---

## 2. Playback Loop Architecture

### 2.1 Overview

**Location:** [wkmp-ap/src/playback/engine/core.rs:1292-1318](../wkmp-ap/src/playback/engine/core.rs#L1292-L1318)

The playback loop runs every 100ms (tokio::time::interval) and calls `process_queue()` to orchestrate:

```rust
async fn playback_loop(&self) -> Result<()> {
    let mut tick = interval(Duration::from_millis(100)); // Check every 100ms

    loop {
        tick.tick().await;

        // Check if we should continue running
        if !*self.running.read().await {
            break;
        }

        // Check playback state
        let playback_state = self.state.get_playback_state().await;
        if playback_state != PlaybackState::Playing {
            continue; // Paused, skip processing
        }

        // Process queue
        self.process_queue().await?;
    }

    Ok(())
}
```

**Why 100ms?**
- Balance between responsiveness (skip detection, queue changes) and CPU efficiency
- Does NOT poll database — only checks in-memory QueueManager state
- Lower values (10ms) waste CPU; higher values (500ms) feel sluggish for skip commands

---

### 2.2 Queue Manager Architecture

**Location:** [wkmp-ap/src/playback/queue_manager.rs:98-299](../wkmp-ap/src/playback/queue_manager.rs#L98-L299)

Queue state is **entirely in-memory**:

```rust
pub struct QueueManager {
    current: Option<QueueEntry>,  // Currently playing passage
    next: Option<QueueEntry>,      // Next to crossfade into
    queued: Vec<QueueEntry>,       // Remaining queue
}

pub fn current(&self) -> Option<&QueueEntry> { self.current.as_ref() }
pub fn next(&self) -> Option<&QueueEntry> { self.next.as_ref() }
pub fn queued(&self) -> &[QueueEntry] { &self.queued }
```

**Queue Operations:**
- `advance_queue()` - Move next → current, promote first queued → next
- `set_current()` - Replace current passage (skip/seek)
- `add_to_queue()` - Append to queued list
- `clear_queue()` - Empty queued list (current/next unchanged)

**Persistence:** Queue changes trigger database writes (queue_persistence table) but playback loop ONLY reads in-memory state.

---

## 3. process_queue() Orchestration Hub

**Location:** [wkmp-ap/src/playback/engine/core.rs:1324-1680](../wkmp-ap/src/playback/engine/core.rs#L1324-L1680)

### 3.1 Critical Design Note

```rust
/// **CRITICAL:** This is the core orchestration hub - do NOT split this method!
/// It coordinates mixer, buffer_manager, and decoder_worker in complex interplay.
pub(super) async fn process_queue(&self) -> Result<()> {
    // ...
}
```

**Why single method?** Splitting would:
- Require holding locks across awaits (deadlock risk)
- Lose synchronization between mixer/buffer/decoder state
- Complicate error handling across boundaries

---

### 3.2 process_queue() Flow

```rust
// Step 1: Clone queue entries (lock dropped immediately)
let (current_entry, next_entry, queued_entries) = {
    let queue = self.queue.read().await;
    (
        queue.current().cloned(),
        queue.next().cloned(),
        queue.queued().to_vec(),
    )
}; // Lock dropped - in-memory only, no DB queries

// Step 2: Trigger decode for current passage if buffer missing
if let Some(current) = &current_entry {
    if !self.buffer_manager.is_managed(current.queue_entry_id).await {
        self.request_decode(current, DecodePriority::Immediate, true).await?;
    }
}

// Step 3: Start mixer when current passage has minimum playback buffer (3 seconds)
if let Some(current) = &current_entry {
    const MIN_PLAYBACK_BUFFER_MS: u64 = 3000;
    let buffer_has_minimum = self.buffer_manager
        .has_minimum_playback_buffer(current.queue_entry_id, MIN_PLAYBACK_BUFFER_MS)
        .await;

    if buffer_has_minimum {
        let mixer = self.mixer.read().await;
        let mixer_idle = mixer.get_current_passage_id().is_none();

        if mixer_idle {
            // Start playback: set passage, add markers, emit PassageStarted
            // (Detailed logic in Section 3.3)
        }
    }
}

// Step 4: Check for crossfade triggering (legacy stub - now marker-driven)
// [Historically checked position_ms vs fade_out_point_ticks]
// [SUB-INC-4B] Crossfading now triggered by StartCrossfade marker in mixer thread

// Step 5: Trigger decode for next passage if buffer missing
if let Some(next) = &next_entry {
    if !self.buffer_manager.is_managed(next.queue_entry_id).await {
        self.request_decode(next, DecodePriority::Next, true).await?;
    }
}

// Step 6: Trigger decode for queued passages (up to max_decode_streams - 2)
for queued in queued_entries.iter().take(max_queued_decodes) {
    if !self.buffer_manager.is_managed(queued.queue_entry_id).await {
        self.request_decode(queued, DecodePriority::Queued, false).await?;
    }
}
```

**Key Performance Characteristics:**
- No database queries in hot path (queue state is in-memory)
- Lock-free buffer checks via BufferManager atomic state tracking
- Decode requests idempotent (duplicates filtered by `is_managed()`)

---

### 3.3 Starting Playback

**When:** Mixer is idle AND current passage has ≥3 seconds decoded

**Location:** [wkmp-ap/src/playback/engine/core.rs:1351-1579](../wkmp-ap/src/playback/engine/core.rs#L1351-L1579)

```rust
// Get passage timing information from database (one-time fetch)
let passage = self.get_passage_timing(current).await?;

// Load song timeline for passage (boundary detection)
if let Some(passage_id) = current.passage_id {
    match crate::db::passage_songs::load_song_timeline(&self.db_pool, passage_id).await {
        Ok(timeline) => {
            // Emit initial CurrentSongChanged event
            *self.current_song_timeline.write().await = Some(timeline);
        }
        Err(e) => {
            warn!("Failed to load song timeline: {}", e);
            *self.current_song_timeline.write().await = None;
        }
    }
}

// Calculate fade-in duration from passage timing points
let fade_in_duration_ticks = passage.fade_in_point_ticks.saturating_sub(passage.start_time_ticks);
let fade_in_duration_samples = wkmp_common::timing::ticks_to_samples(fade_in_duration_ticks, 44100);
let fade_in_curve = current.fade_in_curve.as_ref()
    .and_then(|s| wkmp_common::FadeCurve::from_str(s).ok())
    .unwrap_or(wkmp_common::FadeCurve::Exponential);

// Start mixer with marker-based event system
{
    let mut mixer = self.mixer.write().await;

    // Set current passage
    let mixer_passage_id = current.passage_id.unwrap_or_else(|| Uuid::nil());
    mixer.set_current_passage(mixer_passage_id, current.queue_entry_id, 0);

    // Add position update markers (configurable interval from settings)
    let position_interval_ms = self.position_interval_ms as i64;
    let position_interval_ticks = wkmp_common::timing::ms_to_ticks(position_interval_ms);

    let passage_end_ticks = passage.end_time_ticks.unwrap_or(...);
    let passage_duration_ticks = passage_end_ticks.saturating_sub(passage.start_time_ticks);

    let marker_count = (passage_duration_ticks / position_interval_ticks) as usize;
    for i in 1..=marker_count {
        let tick = i as i64 * position_interval_ticks;
        let position_ms = wkmp_common::timing::ticks_to_ms(tick) as u64;

        mixer.add_marker(PositionMarker {
            tick,
            passage_id: mixer_passage_id,
            event_type: MarkerEvent::PositionUpdate { position_ms },
        });
    }

    // Add crossfade marker (if next passage exists AND current is not ephemeral)
    if current.passage_id.is_some() {
        if let Some(next_passage_id) = next_entry.as_ref().and_then(|n| n.passage_id) {
            let fade_out_start_tick = passage.fade_out_point_ticks
                .unwrap_or(passage.lead_out_point_ticks.unwrap_or(...).saturating_sub(ms_to_ticks(5000)));

            mixer.add_marker(PositionMarker {
                tick: fade_out_start_tick,
                passage_id: mixer_passage_id,
                event_type: MarkerEvent::StartCrossfade { next_passage_id },
            });
        }
    }

    // Add passage complete marker (at fade-out end)
    let complete_tick = passage.fade_out_point_ticks
        .unwrap_or(passage_duration_ticks);

    mixer.add_marker(PositionMarker {
        tick: complete_tick,
        passage_id: mixer_passage_id,
        event_type: MarkerEvent::PassageComplete,
    });

    // Handle fade-in via start_resume_fade()
    if fade_in_duration_samples > 0 {
        mixer.start_resume_fade(fade_in_duration_samples as usize, fade_in_curve);
    }
}

// Mark buffer as playing
self.buffer_manager.mark_playing(current.queue_entry_id).await;

// Emit PassageStarted event
self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageStarted {
    passage_id: current.passage_id.unwrap_or_else(|| Uuid::nil()),
    album_uuids,
    timestamp: chrono::Utc::now(),
});
```

**Database Queries (One-Time):**
- `get_passage_timing()` - Fetch start/end/fade/lead points (cached for passage duration)
- `load_song_timeline()` - Fetch song boundaries for CurrentSongChanged events
- `get_passage_album_uuids()` - Fetch album UUIDs for PassageStarted event

---

## 4. Mixer Thread Architecture

**Location:** [wkmp-ap/src/playback/engine/core.rs:443-650](../wkmp-ap/src/playback/engine/core.rs#L443-L650)

### 4.1 Mixer Thread Design

Runs independently from playback loop:

```rust
tokio::spawn(async move {
    let mut check_interval = interval(Duration::from_micros(check_interval_us)); // Default: 5000μs
    const BATCH_SIZE_FRAMES: usize = 512; // ~11ms @ 44.1kHz

    loop {
        check_interval.tick().await;

        // Check if audio expected
        if !audio_expected_clone.load(Ordering::Relaxed) {
            continue; // Paused, skip mixing
        }

        // Mix batch and push to ring buffer
        mix_and_push_batch(
            &mixer_clone,
            &buffer_manager_clone,
            &position_event_tx_clone,
            &mut producer,
            &mut is_crossfading,
            &mut current_passage_id,
            &mut current_queue_entry_id,
            &mut next_passage_id,
            BATCH_SIZE_FRAMES,
        ).await;
    }
});
```

**Key Parameters:**
- `check_interval_us` - Time between batch mix attempts (default: 5000μs = 5ms)
- `BATCH_SIZE_FRAMES` - Frames to mix per batch (512 frames = ~11ms @ 44.1kHz)
- Ring buffer capacity: 2048 frames (~46ms @ 44.1kHz)

**Relationship to Playback Loop:**
- **Independent:** Mixer thread runs at higher frequency than playback loop
- **Coordinated:** Playback loop sets current passage, mixer thread consumes buffers
- **Event-driven:** Markers trigger crossfade, not position polling

---

### 4.2 Batch Mixing and Event Processing

```rust
async fn mix_and_push_batch(
    mixer: &Arc<RwLock<Mixer>>,
    buffer_manager: &Arc<BufferManager>,
    event_tx: &mpsc::UnboundedSender<PlaybackEvent>,
    producer: &mut AudioProducer,
    is_crossfading: &mut bool,
    current_passage_id: &mut Option<Uuid>,
    current_queue_entry_id: &mut Option<Uuid>,
    _next_passage_id: &mut Option<Uuid>,
    frames_to_mix: usize,
) {
    let mut output = vec![0.0f32; frames_to_mix * 2];

    let mut mixer_guard = mixer.write().await;

    // Update current passage ID and queue entry ID if changed
    *current_passage_id = mixer_guard.get_current_passage_id();
    *current_queue_entry_id = mixer_guard.get_current_queue_entry_id();

    // If no passage playing, fill with silence
    let Some(passage_id) = *current_passage_id else {
        for _ in 0..frames_to_mix {
            if !producer.push(AudioFrame::zero()) { break; }
        }
        return;
    };

    // Mix batch (single passage mixing - crossfade handled via markers)
    let events = mixer_guard.mix_single(buffer_manager, passage_id, &mut output)
        .await
        .unwrap_or_else(|e| {
            error!("Mix error: {}", e);
            vec![]
        });

    // Release mixer lock before pushing to ring buffer
    drop(mixer_guard);

    // Handle marker events
    handle_marker_events(events, event_tx, is_crossfading, current_queue_entry_id);

    // Push frames to ring buffer
    for i in (0..output.len()).step_by(2) {
        let frame = AudioFrame { left: output[i], right: output[i + 1] };
        if !producer.push(frame) {
            break; // Ring buffer full, stop pushing
        }
    }
}
```

**Performance Characteristics:**
- Lock held only during `mix_single()` (~11ms @ 512 frames)
- Ring buffer push is lock-free (no contention with audio callback)
- Event handling after lock released (no blocking)

---

### 4.3 Marker Event Handling

**Location:** [wkmp-ap/src/playback/engine/core.rs:540-590](../wkmp-ap/src/playback/engine/core.rs#L540-L590)

```rust
fn handle_marker_events(
    events: Vec<MarkerEvent>,
    event_tx: &mpsc::UnboundedSender<PlaybackEvent>,
    is_crossfading: &mut bool,
    current_queue_entry_id: &Option<Uuid>,
) {
    for event in events {
        match event {
            MarkerEvent::PositionUpdate { position_ms } => {
                if let Some(queue_entry_id) = *current_queue_entry_id {
                    event_tx.send(PlaybackEvent::PositionUpdate { queue_entry_id, position_ms }).ok();
                }
            }
            MarkerEvent::StartCrossfade { next_passage_id: _ } => {
                *is_crossfading = true;
                // Crossfade start now marker-driven via REV002 system
                // State tracked automatically; mixer applies fade curves to pre-decoded samples
            }
            MarkerEvent::PassageComplete => {
                *is_crossfading = false;
                // Send PassageComplete event to trigger queue advancement
                if let Some(queue_entry_id) = *current_queue_entry_id {
                    event_tx.send(PlaybackEvent::PassageComplete { queue_entry_id }).ok();
                }
            }
            MarkerEvent::SongBoundary { new_song_id: _ } => {
                // Reserved for future cooldown system integration
            }
            MarkerEvent::EndOfFile { unreachable_markers } => {
                warn!("EOF reached with {} unreachable markers", unreachable_markers.len());
                // Treat EOF as passage complete
                *is_crossfading = false;
                if let Some(queue_entry_id) = *current_queue_entry_id {
                    event_tx.send(PlaybackEvent::PassageComplete { queue_entry_id }).ok();
                }
            }
            MarkerEvent::EndOfFileBeforeLeadOut { planned_crossfade_tick, .. } => {
                warn!("EOF before crossfade at tick {}", planned_crossfade_tick);
                // Treat early EOF as passage complete
                *is_crossfading = false;
                if let Some(queue_entry_id) = *current_queue_entry_id {
                    event_tx.send(PlaybackEvent::PassageComplete { queue_entry_id }).ok();
                }
            }
        }
    }
}
```

**Event Flow:**
1. Mixer checks markers during `mix_single()` (via `check_markers()`)
2. Markers at or before current tick are returned as `MarkerEvent` vector
3. `handle_marker_events()` converts to `PlaybackEvent` and sends to event channel
4. Event handlers consume events asynchronously (position_event_handler, buffer_event_handler)

---

## 5. Event Processing Loops

### 5.1 Position Event Handler

**Location:** [wkmp-ap/src/playback/engine/diagnostics.rs:451-540](../wkmp-ap/src/playback/engine/diagnostics.rs#L451-L540)

Spawned at engine start:

```rust
tokio::spawn(async move {
    self_clone.position_event_handler().await;
});
```

**Purpose:**
- Consume `PositionUpdate` events from mixer thread
- Check song boundaries (emit `CurrentSongChanged` events)
- Emit playback progress events (configurable interval, default: 5s)

```rust
pub(super) async fn position_event_handler(&self) {
    let mut rx = self.position_event_rx.write().await.take().unwrap();
    let progress_interval_ms = crate::db::settings::load_progress_interval(&self.db_pool)
        .await.unwrap_or(5000);

    let mut last_progress_position_ms = 0u64;

    loop {
        match rx.recv().await {
            Some(PlaybackEvent::PositionUpdate { queue_entry_id, position_ms }) => {
                // [1] Check song boundary
                let mut timeline = self.current_song_timeline.write().await;
                if let Some(timeline) = timeline.as_mut() {
                    let (crossed, new_song_id) = timeline.check_boundary(position_ms);

                    if crossed {
                        // Emit CurrentSongChanged event
                        self.state.broadcast_event(wkmp_common::events::WkmpEvent::CurrentSongChanged {
                            passage_id,
                            song_id: new_song_id,
                            song_albums,
                            position_ms,
                            timestamp: chrono::Utc::now(),
                        });
                    }
                }

                // [2] Emit playback progress event (every progress_interval_ms)
                if position_ms >= last_progress_position_ms + progress_interval_ms {
                    self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackProgress {
                        queue_entry_id,
                        position_ms,
                        timestamp: chrono::Utc::now(),
                    });
                    last_progress_position_ms = position_ms;
                }
            }
            Some(PlaybackEvent::PassageComplete { queue_entry_id }) => {
                // Advance queue and emit QueueChanged event
                self.advance_queue_on_complete(queue_entry_id).await.ok();
            }
            None => {
                warn!("Position event channel closed");
                break;
            }
        }
    }
}
```

**Database Queries:**
- `load_progress_interval()` - One-time fetch at startup
- `get_passage_album_uuids()` - Per song boundary crossing (infrequent)

---

### 5.2 Buffer Event Handler

**Location:** [wkmp-ap/src/playback/engine/diagnostics.rs:542-580](../wkmp-ap/src/playback/engine/diagnostics.rs#L542-L580)

**Purpose:**
- Instantly start mixer when buffer becomes ready (no 100ms delay)
- Emit `BufferReady` events for monitoring

```rust
pub(super) async fn buffer_event_handler(&self) {
    let mut rx = self.buffer_event_rx.write().await.take().unwrap();

    loop {
        match rx.recv().await {
            Some(BufferEvent::BufferReady { queue_entry_id, is_current }) => {
                // Check if current passage and mixer idle → start immediately
                if is_current {
                    let queue = self.queue.read().await;
                    let current = queue.current();

                    if let Some(current) = current {
                        if current.queue_entry_id == queue_entry_id {
                            drop(queue);

                            let mixer = self.mixer.read().await;
                            let mixer_idle = mixer.get_current_passage_id().is_none();
                            drop(mixer);

                            if mixer_idle {
                                // Buffer ready and mixer idle → trigger start
                                // (process_queue() will detect and start)
                                self.update_audio_expected_flag().await;
                            }
                        }
                    }
                }

                // Emit BufferReady event
                self.state.broadcast_event(wkmp_common::events::WkmpEvent::BufferReady {
                    queue_entry_id,
                    timestamp: chrono::Utc::now(),
                });
            }
            None => {
                warn!("Buffer event channel closed");
                break;
            }
        }
    }
}
```

**Performance Benefit:**
- Without buffer event handler: Up to 100ms delay before mixer starts (waits for next playback loop tick)
- With buffer event handler: Instant start when buffer ready (0-5ms latency)

---

## 6. Decoder Worker Integration

**Location:** [wkmp-ap/src/playback/decoder_worker.rs](../wkmp-ap/src/playback/decoder_worker.rs)

### 6.1 Decode Request Flow

```
playback_loop (100ms)
  └─> process_queue()
      └─> request_decode(queue_entry, priority, is_full_decode)
          └─> decoder_worker.request(DecodeRequest { queue_entry_id, passage_id, priority, full_decode })
              └─> [Background thread] decode audio file
                  └─> buffer_manager.store_buffer(queue_entry_id, buffer)
                      └─> [Emit BufferEvent::BufferReady]
                          └─> buffer_event_handler
                              └─> Start mixer if current passage
```

**Decode Priorities:**
- `Immediate` - Current passage (decode ASAP, full buffer)
- `Next` - Next passage (decode before crossfade, full buffer)
- `Queued` - Remaining queue (decode opportunistically, partial buffer)

**Full vs Partial Decode:**
- Full decode: Entire passage decoded (used for current/next)
- Partial decode: First 30 seconds decoded (used for queued passages, enables instant skip)

---

## 7. Data Access Patterns

### 7.1 In-Memory Operations (High Frequency)

**No database queries:**
- Queue state checks (every 100ms in playback loop)
- Buffer state checks (every 100ms in playback loop)
- Mixer position checks (every 5ms in mixer thread)
- Ring buffer operations (every 5ms in mixer thread)

**Why in-memory?**
- SQLite query overhead too high for 100ms loop (5-10ms per query)
- Lock contention with writes (queue persistence, timestamp updates)
- Event-driven design more robust (no missed updates)

---

### 7.2 Database Reads (Event-Driven)

**Triggered by events, not polling:**

1. **Passage start:**
   - `get_passage_timing()` - Fetch start/end/fade/lead points
   - `load_song_timeline()` - Fetch song boundaries
   - `get_passage_album_uuids()` - Fetch album UUIDs

2. **Song boundary crossing:**
   - `get_passage_album_uuids()` - Fetch album UUIDs for CurrentSongChanged event

3. **Queue advancement:**
   - `load_queue()` - Fetch next passage from Program Director selection

4. **Settings changes:**
   - `load_position_interval()` - Fetch marker interval (one-time at startup)
   - `load_progress_interval()` - Fetch progress event interval (one-time at startup)

**Frequency:** ~1-5 queries per passage start (every 3-5 minutes during normal playback)

---

### 7.3 Database Writes (Infrequent)

**Background writes:**

1. **Queue persistence:**
   - `save_queue()` - Write queue state to `queue_persistence` table
   - **Frequency:** After each queue change (add, remove, advance)

2. **Timestamp updates:**
   - `update_last_played_at()` - Write passage/song/artist/work timestamps
   - **Frequency:** Once per passage start

3. **Progress tracking:**
   - `save_playback_position()` - Write last playback position
   - **Frequency:** Configurable (default: every 5 seconds)

**Write Strategy:**
- All writes asynchronous (non-blocking)
- Failures logged but not fatal (playback continues)
- No writes in playback loop or mixer thread critical path

---

## 8. State Transitions

### 8.1 Playback State Machine

```
Stopped
  └─> [User clicks Play] → Playing
      └─> playback_loop starts (100ms ticks)
          └─> process_queue() → Start mixer when buffer ready
              └─> Mixer thread generates audio
                  └─> [MarkerEvent::PassageComplete] → Advance queue
                      └─> [Repeat until queue empty]
                          └─> [Queue empty] → Stopped
```

**State Changes:**
- `Stopped` → `Playing` - User clicks Play, `playback_loop` spawned
- `Playing` → `Paused` - User clicks Pause, mixer thread skips mixing (audio_expected = false)
- `Paused` → `Playing` - User clicks Resume, mixer thread resumes mixing
- `Playing` → `Stopped` - Queue empty + PassageComplete, `playback_loop` exits

---

### 8.2 Queue State Transitions

```
Empty Queue
  └─> [User adds passage] → Current only
      └─> [process_queue] → Decode current
          └─> [Buffer ready] → Start mixer
              └─> [User adds passage] → Current + Next
                  └─> [process_queue] → Decode next
                      └─> [MarkerEvent::StartCrossfade] → Begin crossfade
                          └─> [MarkerEvent::PassageComplete] → Advance queue
                              └─> Next → Current, empty Next
                                  └─> [Repeat]
```

**Queue Advancement Trigger:**
- `PassageComplete` marker fires in mixer thread
- `handle_marker_events()` sends `PlaybackEvent::PassageComplete`
- `position_event_handler()` receives event, calls `advance_queue_on_complete()`
- Queue updated: next → current, first queued → next
- `process_queue()` detects new current/next, triggers decodes

---

## 9. Threading Model

### 9.1 Thread Overview

```
[Main Tokio Runtime]
  ├─ HTTP Server Thread (Axum)
  │   └─> API handlers (play, pause, skip, add to queue)
  │
  ├─ Playback Loop Thread (tokio::spawn)
  │   └─> process_queue() every 100ms
  │
  ├─ Mixer Thread (tokio::spawn)
  │   └─> Batch mix every 5ms → push to ring buffer
  │
  ├─ Position Event Handler Thread (tokio::spawn)
  │   └─> Consume PositionUpdate/PassageComplete events
  │
  ├─ Buffer Event Handler Thread (tokio::spawn)
  │   └─> Consume BufferReady events
  │
  ├─ Decoder Worker Thread Pool (rayon)
  │   └─> Background audio decode (symphonia + rubato)
  │
  └─ Audio Callback Thread (cpal - OS-managed)
      └─> Pull frames from ring buffer → hardware output
```

**Synchronization:**
- `RwLock<QueueManager>` - Shared queue state
- `RwLock<Mixer>` - Shared mixer state
- `Arc<BufferManager>` - Shared buffer state (internal locks)
- `mpsc::unbounded_channel` - Event communication (position, buffer)
- `AudioRingBuffer` - Lock-free ring buffer (mixer → audio callback)

---

### 9.2 Lock Hierarchy (Deadlock Prevention)

**Rule:** Always acquire locks in this order:

1. `queue` (RwLock)
2. `mixer` (RwLock)
3. `buffer_manager` (internal locks)
4. `current_song_timeline` (RwLock)

**Critical Pattern - ASYNC-LOCK-001:**

```rust
// ✅ CORRECT: Clone data immediately, drop lock before await
let (current_entry, next_entry, queued_entries) = {
    let queue = self.queue.read().await;
    (queue.current().cloned(), queue.next().cloned(), queue.queued().to_vec())
}; // Lock dropped before await points

// ❌ INCORRECT: Holding lock across await (deadlock risk)
let queue = self.queue.read().await;
let current = queue.current();
self.do_something().await; // Deadlock if do_something() tries to acquire queue lock
```

**Why?** Holding `RwLock` read guard across await points can cause deadlocks when:
- Other tasks try to acquire read locks
- Write lock request is pending (blocks all new read locks)
- Current task awaits, never releases read lock

---

## 10. Performance Characteristics

### 10.1 Latency Budget

| Operation | Target Latency | Actual Measured |
|-----------|----------------|-----------------|
| Skip command → mixer start | <100ms | 0-100ms (depends on playback loop tick) |
| Buffer ready → mixer start | <5ms | 0-5ms (buffer_event_handler) |
| Marker event → queue advance | <10ms | 5-10ms (event handler + process_queue) |
| Decode request → buffer ready | <500ms | 200-500ms (symphonia + rubato) |

**Critical Path:**
- Audio callback: <1ms (lock-free ring buffer read)
- Mixer thread: ~11ms (512 frames @ 44.1kHz)
- Playback loop: 100ms (non-blocking, event-driven)

---

### 10.2 CPU Usage

| Thread | CPU Usage (Idle) | CPU Usage (Playing) |
|--------|------------------|---------------------|
| Playback loop | <0.1% | <0.5% |
| Mixer thread | 0% | 1-2% |
| Audio callback | 0% | <0.1% |
| Decoder worker | 0% | 5-15% (bursty) |
| Position event handler | <0.1% | <0.1% |
| Buffer event handler | <0.1% | <0.1% |

**Total:** ~7-18% CPU during active playback (single core)

---

### 10.3 Memory Usage

| Component | Memory (Idle) | Memory (Playing) |
|-----------|---------------|------------------|
| Queue state | <1 KB | <10 KB |
| Mixer state | <10 KB | <10 KB |
| Ring buffer | ~384 KB | ~384 KB (2048 frames × 2 channels × 4 bytes × 24 buffers) |
| Decode buffers | 0 KB | ~20-60 MB (2-6 passages × 10 MB each) |
| Song timeline | 0 KB | <1 KB |

**Total:** ~20-60 MB during active playback

---

## 11. Error Handling

### 11.1 Playback Loop Errors

```rust
tokio::spawn(async move {
    if let Err(e) = self_clone.playback_loop().await {
        error!("Playback loop error: {}", e);
    }
});
```

**Error Types:**
- Database query failure (passage timing, song timeline)
- Buffer manager errors (buffer not found)
- Decoder worker errors (file not found, unsupported format)

**Recovery:**
- Playback loop logs error, continues next iteration
- Failed passage skipped (emits PassageSkipped event)
- Queue advancement continues (next passage attempted)

---

### 11.2 Mixer Thread Errors

```rust
let events = mixer_guard.mix_single(buffer_manager, passage_id, &mut output)
    .await
    .unwrap_or_else(|e| {
        error!("Mix error: {}", e);
        vec![]
    });
```

**Error Types:**
- Buffer underrun (decode not complete)
- Buffer not found (rare race condition)
- Resample error (rubato failure)

**Recovery:**
- Mixer fills output with silence
- Logs error, continues next batch
- Buffer event handler retries decode if needed

---

### 11.3 Event Handler Errors

```rust
self.advance_queue_on_complete(queue_entry_id).await.ok();
```

**Error Types:**
- Database query failure (next passage selection)
- Queue state inconsistency (race condition)

**Recovery:**
- Event handler logs error, continues listening
- Failed queue advance retried on next PassageComplete
- Playback continues with current passage (no crash)

---

## 12. Traceability Matrix

| Requirement | Implementation | Location |
|-------------|----------------|----------|
| [SSD-FLOW-010] Core orchestration logic | `playback_loop()` + `process_queue()` | core.rs:1292-1680 |
| [SSD-MIX-030] Single passage playback initiation | `process_queue()` start mixer logic | core.rs:1351-1579 |
| [SSD-PBUF-028] Start playback with minimum buffer (3s) | `has_minimum_playback_buffer()` check | core.rs:1354-1362 |
| [SSD-FBUF-011] Full decode for current/next | `request_decode(priority=Immediate/Next)` | core.rs:1343, 1641 |
| [SSD-PBUF-010] Partial buffer decode for queued | `request_decode(priority=Queued, full=false)` | core.rs:1646-1680 |
| [SSD-RBUF-014] Ring buffer grace period | `AudioRingBuffer::new(grace_period_ms)` | core.rs:429 |
| [SSD-OUT-012] Lock-free audio callback | `AudioRingBuffer::split()` | core.rs:430 |
| [SUB-INC-4B] Marker-based event system | `mixer.add_marker()` + `check_markers()` | core.rs:1484-1532, mixer.rs:467 |
| [ASYNC-LOCK-001] Clone before await | Queue entry cloning pattern | core.rs:1328-1335 |
| [PERF-POLL-010] Buffer event handler for instant start | `buffer_event_handler()` | diagnostics.rs:542-580 |
| [SSE-UI-030] PlaybackPosition emission (1s) | `playback_position_emitter()` | diagnostics.rs:410-412 |
| [ARCH-SNGC-030] Configurable position interval | `load_position_interval()` | core.rs:1469-1471 |
| [REQ-MIX-EOF-001] Unreachable markers on EOF | `collect_unreachable_markers()` | mixer.rs:494-500 |

---

## 13. Future Enhancements

### 13.1 Adaptive Playback Loop Interval

**Current:** Fixed 100ms interval

**Proposal:** Dynamic interval based on queue state:
- Empty queue: 500ms (low CPU)
- Current only: 100ms (normal)
- Current + Next: 50ms (responsive during crossfade)

**Benefit:** Reduced CPU usage during idle periods

**Risk:** Complexity in interval adjustment logic

---

### 13.2 Predictive Decode Scheduling

**Current:** Decode triggered when `is_managed() == false`

**Proposal:** Predictive decode scheduling:
- Estimate time until next/queued passage needed
- Schedule decode to complete just before playback
- Reduce memory usage (fewer pre-decoded buffers)

**Benefit:** Lower memory footprint (20-60 MB → 10-30 MB)

**Risk:** Decode timing prediction errors → buffer underruns

---

### 13.3 Crossfade Marker Optimization

**Current:** Crossfade marker added at passage start (even if no next passage)

**Proposal:** Defer crossfade marker until next passage added to queue

**Benefit:** Fewer unnecessary markers (reduces marker heap size)

**Risk:** Race condition if next passage added after crossfade point

---

## 14. Glossary

- **Playback Loop:** 100ms tokio interval that calls `process_queue()`
- **Mixer Thread:** High-frequency (5ms) thread that batch-mixes audio and pushes to ring buffer
- **process_queue():** Core orchestration method coordinating mixer, buffer_manager, and decoder_worker
- **Ring Buffer:** Lock-free SPSC buffer between mixer thread and audio callback
- **Marker:** Time-based event trigger (position update, crossfade start, passage complete)
- **Queue Entry:** In-memory queue element (current, next, or queued passage)
- **Buffer Manager:** Component managing PCM buffer lifecycle (decode, playback, cleanup)
- **Decoder Worker:** Background thread pool for audio file decoding
- **Event Handler:** Async task consuming PlaybackEvent or BufferEvent streams

---

## 15. References

- [SPEC001: Architecture](SPEC001-architecture.md) - System overview and microservices architecture
- [SPEC013: Single-Stream Playback](SPEC013-single_stream_playback.md) - Audio pipeline design
- [SPEC016: Mixer Architecture](SPEC016-mixer_architecture.md) - Mixer implementation details
- [ADR-002: Event-Driven Position Tracking](ADR-002-event_driven_position_tracking.md) - Marker-based event system rationale
- [SPEC002: Crossfade](SPEC002-crossfade.md) - Crossfade timing and fade curve specifications

---

**Version History:**
- 1.0 (2025-11-02) - Initial document extracted from inline code comments and analysis

**Maintained By:** WKMP Development Team

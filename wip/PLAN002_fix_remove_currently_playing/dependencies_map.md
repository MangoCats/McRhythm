# Dependencies Map - Fix Remove Currently Playing Passage Bug

**Plan ID:** PLAN002
**Date:** 2025-10-27

---

## Existing Code (Will Modify)

| File | Function/Module | Status | Purpose | Changes Needed |
|------|----------------|--------|---------|----------------|
| `wkmp-ap/src/playback/engine.rs` | `remove_queue_entry()` | Exists (broken) | Remove queue entry from in-memory queue | Add: Detect if current, clear decoder/mixer |
| `wkmp-ap/src/playback/queue_manager.rs` | `QueueManager::remove()` | Exists (broken) | Update queue data structure | May need: Return info about what was removed |
| `wkmp-ap/src/playback/decoder_worker.rs` | TBD | Unknown | Decoder command handling | May need: Add stop/clear command |
| `wkmp-ap/src/playback/pipeline/mixer.rs` | TBD | Unknown | Mixer state management | May need: Add stop/clear method |

---

## Existing Code (Will Reference/Use)

| File | Function/Module | Status | Purpose | How Used |
|------|----------------|--------|---------|----------|
| `wkmp-ap/src/api/handlers.rs` | `remove_from_queue()` | Exists (working) | HTTP API handler | Calls `engine.remove_queue_entry()` - unchanged |
| `wkmp-ap/src/db/queue.rs` | `remove_from_queue()` | Exists (working) | Database removal | Database operation - unchanged |
| `wkmp-ap/src/playback/queue_manager.rs` | `QueueManager::current()` | Exists | Get currently playing entry | Use to check if removing current |
| `wkmp-ap/src/playback/engine.rs` | Playback start logic | Exists | Start playing a passage | Reference for "start next" logic |

---

## Existing Infrastructure (Will Leverage)

### Communication Mechanisms
| Mechanism | Location | Status | Purpose |
|-----------|----------|--------|---------|
| `tokio::sync::mpsc` channels | Throughout engine | Exists | Async messaging | Use for decoder commands |
| Shared `Arc<RwLock<T>>` state | Throughout engine | Exists | Shared state access | Use for mixer state |
| SSE event system | `wkmp-common/events` | Exists | UI notifications | Emit events after removal |

### Playback State
| Component | Location | Status | Purpose |
|-----------|----------|--------|---------|
| `PlaybackState` enum | `wkmp-ap/src/state.rs` | Exists | Playing/Paused/Stopped state | Update on removal |
| Chain assignment map | `wkmp-ap/src/playback/engine.rs` | Exists (assumed) | Queue entry → decoder chain | Clear on removal |
| Audio ring buffer | `wkmp-ap/src/playback/ring_buffer.rs` | Exists | PCM sample buffer | May need to clear/drain |

---

## External Dependencies

### Rust Crates (Already in Use)
- `tokio` - Async runtime (channels, tasks, synchronization)
- `uuid` - Queue entry IDs
- `tracing` - Logging

### No New External Dependencies Required

---

## Subsystem Interaction Map

```
HTTP API Handler (handlers.rs)
    ↓
    calls
    ↓
PlaybackEngine::remove_queue_entry() [THIS IS WHERE WE FIX]
    ↓
    delegates to
    ↓
QueueManager::remove()
    ↓ (queue structure updated)
    ↑ (returns: was it current?)
    ↓
[NEW] Check if removed entry was current
    ↓ (if current)
    ↓
[NEW] Clear Decoder Chain
    ↓
    sends command to
    ↓
Decoder Worker
    ↓ (stops decoding, releases resources)
    ↓
[NEW] Clear Mixer State
    ↓
    updates
    ↓
Mixer
    ↓ (stops reading from old chain)
    ↓
[NEW] Start Next Passage (if queue non-empty)
    ↓
    uses existing playback start logic
```

---

## Investigation Needed

Before implementing, need to answer:

### Question 1: How does decoder worker receive commands?
- **Look at:** `wkmp-ap/src/playback/decoder_worker.rs`
- **Find:** Command channel, message types
- **Options:**
  - Channel already exists → add new command type
  - No channel → need to create communication mechanism

### Question 2: How to clear mixer state?
- **Look at:** `wkmp-ap/src/playback/pipeline/mixer.rs`
- **Find:** Mixer state structure, stop/reset methods
- **Options:**
  - Mixer has shared state → update it
  - Mixer is autonomous → need signaling mechanism

### Question 3: How to identify decoder chain for removed passage?
- **Look at:** `wkmp-ap/src/playback/engine.rs`
- **Find:** Chain assignment tracking
- **Need:** Map from `queue_entry_id` → `chain_id`

### Question 4: How does natural passage end work?
- **Look at:** Playback loop in `engine.rs`
- **Find:** Logic for "current passage finished → start next"
- **Reuse:** Same mechanism for "removed current → start next"

---

## Specification Dependencies

### Requirements Affected by Fix
- **REQ-QUE-070:** Queue management operations - will now work correctly
- **REQ-PB-010:** Playback control (stop) - will now stop on removal
- **REQ-PB-040:** Queue advancement (skip) - will now advance correctly

### Architecture Constraints
- **Single-stream audio design** (per SPEC001)
  - Only one passage playing at a time
  - Sequential advancement through queue
  - Fix must preserve this model

- **Sample-accurate crossfading** (per SPEC002)
  - Removal interrupts crossfade if in progress
  - Need to cleanly abort crossfade state

---

## Testing Dependencies

### Test Infrastructure Needed
- Ability to enqueue test audio files
- Ability to trigger removal programmatically
- Ability to verify:
  - Playback state (playing/stopped)
  - Queue contents
  - Decoder chain status (if visible)

### Test Audio Files
- Sample audio files for testing
- Can use any audio format supported by symphonia
- Short files (1-5 seconds) for fast tests

---

## Risks from Dependencies

| Dependency | Risk | Mitigation |
|------------|------|------------|
| Decoder command mechanism unclear | May need significant refactoring | Investigate first, find simplest approach |
| Mixer state not accessible | Can't directly stop mixer | Use indirect method (clear ring buffer, update flags) |
| Chain assignment tracking missing | Can't identify which chain to clear | Add tracking if needed (should exist already) |
| No precedent for "stop current" | Don't know correct pattern | Find similar operations (pause? stop?) |

---

## Summary

**Existing Infrastructure:** Solid foundation (queue, decoder, mixer all exist)
**Changes Needed:** Coordination logic between components
**New Components:** None - only coordination improvements
**Investigation Required:** 4 key questions about existing mechanisms
**Risk Level:** Medium - touches critical playback path

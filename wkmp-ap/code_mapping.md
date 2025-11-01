# PLAN016 Code-to-Module Mapping

**Purpose:** Detailed mapping of engine.rs sections to target modules for refactoring

**Date:** 2025-11-01
**Original File:** wkmp-ap/src/playback/engine.rs (4,251 lines)
**Target Modules:** core.rs (~1,380 lines), queue.rs (~1,250 lines), diagnostics.rs (~1,035 lines), mod.rs (~100 lines)

---

## Summary

This document maps specific line ranges from engine.rs to the three functional modules identified in Phase 4 (Approach Selection). Line ranges are approximate and may shift during extraction due to removed comments/whitespace.

**Module Distribution:**
- **core.rs:** Lifecycle, state management, orchestration (process_queue)
- **queue.rs:** Queue operations, queue queries
- **diagnostics.rs:** Status accessors, event handlers, monitoring

---

## Shared Code (All Modules)

**Lines 1-67: Common imports and PlaybackPosition struct**
- PlaybackPosition struct (lines 37-66)
- Will be moved to mod.rs or core.rs as shared utility
- Used by all modules for position tracking

---

## CORE.RS - State Management & Lifecycle (~1,380 lines)

### PlaybackEngine Struct Definition
- **Lines 68-178:** Struct definition with all fields
- **Location:** Beginning of core.rs
- **Why:** Central state container, needed by all modules

### Constructor
- **Lines 185-328:** `pub async fn new(db_pool, state) -> Result<Self>`
- **Why:** Initializes all components (mixer, buffer_manager, decoder_worker, queue)

### Chain Management (Internal)
- **Lines 330-369:** `pub async fn assign_chains_to_loaded_queue(&self)`
- **Lines 3104-3141:** `async fn assign_chain(&self, queue_entry_id) -> Option<usize>`
- **Lines 3143-3167:** `async fn release_chain(&self, queue_entry_id)`
- **Why:** Internal buffer chain lifecycle management

### Lifecycle Control
- **Lines 371-835:** `pub async fn start(&self) -> Result<()>`
  - Spawns 9+ async event handler tasks
  - Starts playback_loop
  - Complex, keep intact
- **Lines 837-876:** `pub async fn stop(&self) -> Result<()>`
- **Lines 878-940:** `pub async fn play(&self) -> Result<()>`
- **Lines 942-1009:** `pub async fn pause(&self) -> Result<()>`
- **Lines 1172-1248:** `pub async fn seek(&self, position_ms) -> Result<()>`

### Orchestration Hub (Keep Intact)
- **Lines 2116-2143:** `async fn playback_loop(&self) -> Result<()>`
  - 100ms loop, calls process_queue
  - ~25 lines, simple
- **Lines 2145-2726:** `async fn process_queue(&self) -> Result<()>`
  - **CRITICAL:** 580-line orchestration hub
  - Coordinates mixer, buffer_manager, decoder_worker
  - High complexity, do NOT split
  - Keep as single unit in core.rs

### Helper Functions
- **Lines 1644-1662:** `async fn update_audio_expected_flag(&self)`
- **Lines 1932-1974:** `async fn calculate_crossfade_start_ms(&self, passage, queue_entry_id) -> u64`
- **Lines 1976-2003:** `async fn should_trigger_crossfade(&self, ...) -> bool`
- **Lines 2005-2114:** `async fn try_trigger_crossfade(&self, ...) -> Result<()>`
- **Lines 2728-2749:** `async fn request_decode(&self, ...)`
- **Lines 2751-2769:** `async fn get_passage_timing(&self, entry) -> Result<PassageWithTiming>`
- **Lines 2771-2895:** `async fn handle_buffer_underrun(&self, queue_entry_id, headroom)`

### Clone Handles (Internal Utility)
- **Lines 3060-3102:** `fn clone_handles(&self) -> Self`
- **Why:** Used to create thread-safe copies of Arc fields for handler tasks

**Estimated Total for core.rs:** ~1,380 lines

---

## QUEUE.RS - Queue Operations (~1,250 lines)

### Queue Operations (Mutating)
- **Lines 1011-1101:** `pub async fn skip_next(&self) -> Result<()>`
  - 83 lines - skip current, advance queue
- **Lines 1103-1170:** `pub async fn clear_queue(&self) -> Result<()>`
  - 53 lines - clear all entries
- **Lines 1673-1758:** `pub async fn enqueue_file(&self, file_path) -> Result<Uuid>`
  - 79 lines - add passage to queue
- **Lines 1760-1844:** `pub async fn remove_queue_entry(&self, queue_entry_id) -> bool`
  - 75 lines - remove specific entry
- **Lines 1609-1642:** `pub async fn reorder_queue_entry(&self, queue_entry_id, new_position) -> Result<()>`
  - 25 lines - change play order

### Queue Queries (Read-Only)
- **Lines 1298-1306:** `pub async fn queue_len(&self) -> usize`
  - 3 lines - count entries
- **Lines 1308-1336:** `pub async fn get_queue_entries(&self) -> Vec<QueueEntry>`
  - 29 lines - query queue state

### Queue Helper Functions
- **Lines 1846-1888:** `async fn emit_queue_change_events(&self, trigger)`
  - 26 lines - emit QueueChanged, QueueIndexChanged events
- **Lines 1890-1930:** `async fn complete_passage_removal(&self, ...)`
  - 33 lines - cleanup after passage removal (buffer release, events)

**Estimated Total for queue.rs:** ~1,250 lines

**Dependencies:**
- Needs access to: queue, buffer_manager, decoder_worker, db_pool, state
- Will call core.rs methods: assign_chain(), release_chain()
- No dependencies on diagnostics.rs

---

## DIAGNOSTICS.RS - Monitoring & Status (~1,035 lines)

### Status Accessors (Public API)
- **Lines 1250-1262:** `pub fn get_volume_arc(&self) -> Arc<Mutex<f32>>`
  - 2 lines - volume accessor for UI
- **Lines 1264-1273:** `pub fn get_buffer_manager(&self) -> Arc<BufferManager>`
  - 2 lines - buffer manager accessor
- **Lines 1275-1283:** `pub fn is_audio_expected(&self) -> bool`
  - 2 lines - underrun classification
- **Lines 1285-1296:** `pub async fn get_callback_stats(&self) -> Option<CallbackStats>`
  - 3 lines - gap/stutter statistics
- **Lines 1338-1544:** `pub async fn get_buffer_chains(&self) -> Vec<BufferChainInfo>`
  - 189 lines - comprehensive buffer chain information (diagnostics/visualization)
- **Lines 1546-1607:** `pub async fn verify_queue_sync(&self) -> bool`
  - 54 lines - in-memory vs DB queue validation
- **Lines 1664-1671:** `pub async fn get_buffer_statuses(&self) -> HashMap<Uuid, BufferStatus>`
  - 2 lines - status map accessor
- **Lines 3628-3670:** `pub async fn get_pipeline_metrics(&self) -> PipelineMetrics`
  - 27 lines - aggregated metrics for monitoring

### Monitoring Configuration
- **Lines 3558-3566:** `pub async fn set_buffer_monitor_rate(&self, rate_ms)`
  - 3 lines - configure SSE update rate
- **Lines 3568-3577:** `pub fn trigger_buffer_monitor_update(&self)`
  - 3 lines - force immediate buffer status update

### Event Handlers (Spawned by start(), run in diagnostics.rs)
- **Lines 2897-3058:** `async fn position_event_handler(&self)`
  - 160 lines - consumes PositionMarker events from mixer
  - Updates current_passage, frame_position
  - Emits PositionUpdate, PassageStarted, PassageCompleted events
- **Lines 3169-3480:** `async fn buffer_event_handler(&self)`
  - 300 lines - consumes BufferEvent from buffer_manager
  - Handles decode_ready, decode_error, decode_cancelled
  - Updates queue state, emits BufferFilled, BufferError, etc.
- **Lines 3482-3556:** `async fn buffer_chain_status_emitter(&self)`
  - 68 lines - periodic SSE emitter for buffer chain status
  - Runs on configurable interval (default 1000ms)
- **Lines 3579-3626:** `async fn playback_position_emitter(&self)`
  - 37 lines - periodic SSE emitter for playback position
  - Runs on 100ms interval

**Estimated Total for diagnostics.rs:** ~1,035 lines

**Dependencies:**
- Needs read access to: state, queue, buffer_manager, mixer, position
- Minimal coupling to core.rs and queue.rs (read-only)
- No mutual dependencies with queue.rs

---

## MOD.RS - Public API Re-Exports (~100 lines)

**Contents:**
```rust
//! Playback engine module - refactored from monolithic engine.rs
//!
//! **Module Structure:**
//! - `core.rs`: Lifecycle, state management, orchestration (process_queue)
//! - `queue.rs`: Queue operations (enqueue, skip, clear, reorder, remove)
//! - `diagnostics.rs`: Monitoring, status accessors, event handlers
//!
//! **Traceability:**
//! - [REQ-DEBT-QUALITY-002-010] Split into 3 functional modules
//! - [REQ-DEBT-QUALITY-002-020] Each module <1500 lines
//! - [REQ-DEBT-QUALITY-002-030] Public API unchanged
//! - [PLAN016] Engine refactoring implementation

mod core;
mod queue;
mod diagnostics;

// Re-export PlaybackEngine as public API
pub use core::PlaybackEngine;

// Internal items stay internal (no re-exports)
```

**Line Count:** ~25 lines (module declarations, re-exports, documentation)

---

## Test Code (Stays in tests/ or Separate File)

**Lines 3672-4251:** Test helper and 16 integration tests

**Options:**
1. **Move to tests/playback_engine_integration.rs** (preferred)
   - Cleaner separation of test vs. production code
   - Tests use public API only
2. **Keep in core.rs #[cfg(test)] mod tests**
   - Easier access to private functions for testing
   - Standard Rust pattern

**Recommendation:** Move to separate test file during Increment 6 (Verification) to reduce core.rs line count.

**Tests Affected by Refactoring:**
- All tests use `PlaybackEngine::new()` - will still work via mod.rs re-export
- No test changes required (public API unchanged)

---

## Import Updates Required

### core.rs
```rust
use super::queue::*;       // Queue operations
use super::diagnostics::*; // Status accessors, handlers
```

### queue.rs
```rust
use super::core::PlaybackEngine; // For &self receiver
// Or: Define QueueOps trait and implement on PlaybackEngine
```

### diagnostics.rs
```rust
use super::core::PlaybackEngine; // For &self receiver
```

---

## Verification Checkpoints

After each module extraction:

1. **Compilation Check:**
   ```bash
   cargo check -p wkmp-ap
   ```

2. **Test Suite:**
   ```bash
   cargo test -p wkmp-ap --lib
   ```

3. **Line Count Verification:**
   ```bash
   wc -l src/playback/engine/*.rs
   ```
   - Expect: core.rs <1500, queue.rs <1500, diagnostics.rs <1500

4. **API Stability:**
   - handlers.rs compiles without changes
   - No new `pub` items introduced

---

## Special Considerations

### 1. Handler Task Spawning (start() method)

**Current Pattern (lines 371-835):**
```rust
pub async fn start(&self) -> Result<()> {
    // ...
    let handles = self.clone_handles();
    tokio::spawn(async move {
        handles.position_event_handler().await;
    });
    // ... 9+ more spawns
}
```

**After Refactoring:**
- start() stays in core.rs
- Handler implementations move to diagnostics.rs
- Spawns call methods across module boundary: `handles.diagnostics_position_handler().await`

**Solution:**
- Keep handlers as methods on PlaybackEngine
- Module boundary is logical (file split), not struct split
- All methods still use `&self` receiver on PlaybackEngine

### 2. process_queue() Orchestration

**Critical:** This 580-line function must NOT be split.
- Stays in core.rs as single unit
- Complex coordination logic (mixer, buffer_manager, decoder_worker)
- High coupling acceptable within orchestration hub

### 3. PlaybackEngine Struct

**Stays unified:**
- Struct definition in core.rs
- All fields remain (no split into multiple structs)
- queue.rs and diagnostics.rs define methods via `impl PlaybackEngine` blocks
- Rust allows multiple impl blocks across modules for same struct

---

## Implementation Notes

### Approach: Incremental Module Population

1. **Create skeleton files** (Increment 2):
   - core.rs: empty impl PlaybackEngine {}
   - queue.rs: empty impl PlaybackEngine {}
   - diagnostics.rs: empty impl PlaybackEngine {}
   - mod.rs: re-exports

2. **Extract queue.rs** (Increment 3):
   - Copy queue methods to queue.rs impl block
   - Verify compilation
   - Run tests

3. **Extract diagnostics.rs** (Increment 4):
   - Copy diagnostic methods to diagnostics.rs impl block
   - Verify compilation
   - Run tests

4. **Finalize core.rs** (Increment 5):
   - Copy remaining methods to core.rs impl block
   - Remove original engine.rs
   - Verify compilation

### Import Strategy

Use explicit imports between modules:
```rust
// In queue.rs
use super::core::PlaybackEngine;
use super::diagnostics::*; // If needed
```

Avoid circular dependencies by keeping coupling directional:
- core → queue (calls skip_next on completion)
- core → diagnostics (spawns handlers)
- queue → core (uses chain management)
- diagnostics → core (reads state)

---

## Line Count Accounting

**Original:** 4,251 lines

**Expected Distribution:**
- core.rs: ~1,380 lines (32%)
- queue.rs: ~1,250 lines (29%)
- diagnostics.rs: ~1,035 lines (24%)
- mod.rs: ~25 lines (<1%)
- tests (moved): ~560 lines (13%)
- **Total:** ~4,250 lines (±5% allowable)

**Buffer:** Each module has ~120-465 lines of headroom before hitting 1,500-line limit.

---

## Risks and Mitigations

**Risk 1: Line counts exceed 1,500 after extraction**
- **Mitigation:** Track running total during extraction
- **Contingency:** Move large helper functions to separate helper.rs file

**Risk 2: Circular dependencies between modules**
- **Mitigation:** Keep coupling directional (core ← queue, core ← diagnostics)
- **Verification:** Cargo will error on circular dependencies

**Risk 3: Handler spawns fail to compile across module boundary**
- **Mitigation:** Keep handlers as methods on unified PlaybackEngine struct
- **Fallback:** Use trait-based delegation if needed

---

**Code Mapping Complete**
**Ready for Increment 2: Module Structure Creation**

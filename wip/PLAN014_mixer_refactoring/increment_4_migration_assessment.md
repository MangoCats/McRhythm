# PLAN014 Increment 4 Migration Assessment

**Date:** 2025-01-30
**Status:** BLOCKED - Additional work required before migration

---

## Problem Statement

**Objective:** Migrate PlaybackEngine to use correct mixer (SPEC016-compliant) instead of legacy mixer.

**Blocker:** Legacy mixer has features that PlaybackEngine depends on that don't exist in correct mixer:

| Feature | Legacy Mixer | Correct Mixer | Impact |
|---------|--------------|---------------|--------|
| **Position Events** | ✅ Emits PositionUpdate events | ❌ Not implemented | PlaybackEngine expects events |
| **Buffer Manager Integration** | ✅ set_buffer_manager() | ❌ Not implemented | Underrun detection |
| **Mixer Min Start Level** | ✅ set_mixer_min_start_level() | ❌ Not implemented | Buffer threshold |
| **Position Event Interval** | ✅ set_position_event_interval_ms() | ❌ Not implemented | Event frequency config |

---

## Engine Dependencies Analysis

### Current Engine Instantiation (engine.rs lines 233-252)

```rust
// Create mixer
let mut mixer = CrossfadeMixer::new();

// Configure mixer with event channel
mixer.set_event_channel(position_event_tx.clone());

// Configure position event interval
mixer.set_position_event_interval_ms(interval_ms);

// Configure mixer with buffer manager for underrun detection
mixer.set_buffer_manager(Arc::clone(&buffer_manager));

// Configure minimum buffer level before playback start
mixer.set_mixer_min_start_level(mixer_min_start_level);

let mixer = Arc::new(RwLock::new(mixer));
```

**Dependencies:**
- `position_event_tx` - Channel for position events
- `interval_ms` - Event emission frequency
- `buffer_manager` - For underrun detection
- `mixer_min_start_level` - Buffer threshold setting

### Migration Challenge

**Cannot simply replace `CrossfadeMixer::new()` with `Mixer::new()`** because:

1. **Correct Mixer API is different:**
   - Takes `master_volume` parameter (not present in legacy)
   - No setter methods (simpler interface)
   - No position event support

2. **PlaybackEngine expects position events:**
   - Uses `position_event_rx` channel (line 234)
   - Likely consumed elsewhere in engine for UI updates

3. **Architectural mismatch:**
   - Legacy mixer is stateful (tracks passages, emits events)
   - Correct mixer is stateless (simple mixing only)

---

## Migration Options

### Option A: Move Position Events to PlaybackEngine FIRST ✅ RECOMMENDED

**Approach:**
1. **Add position tracking to PlaybackEngine**
   - Track current passage ID, frame position
   - Emit PositionUpdate events from engine (not mixer)
2. **Remove position event dependencies from mixer call site**
3. **THEN migrate to correct mixer**
4. **Delete legacy mixer**

**Effort:** 3-5 hours (significant refactoring)

**Risk:** Low-Medium - Requires careful testing of position event behavior

**Benefit:**
- Correct architectural separation (engine tracks position, mixer just mixes)
- Aligns with Increment 3 architectural assessment (REQ-MIX-008 decision)
- Clean migration (no temporary workarounds)

---

### Option B: Temporary Stub Methods ❌ NOT RECOMMENDED

**Approach:**
1. Add stub methods to correct mixer (set_event_channel, etc.) that do nothing
2. Migrate to correct mixer with stubs
3. Move position events to PlaybackEngine later
4. Remove stubs

**Effort:** 1-2 hours initially, 3-4 hours cleanup later

**Risk:** High - Creates technical debt, confusing "dead" methods

**Downsides:**
- Violates clean architecture (mixer pretends to support features it doesn't)
- Position events silently stop working (UI breaks)
- Deferred cleanup may never happen

---

### Option C: Parallel Migration ❌ NOT RECOMMENDED

**Approach:**
1. Keep legacy mixer active
2. Add correct mixer alongside
3. Gradually migrate engine to use correct mixer
4. Remove legacy mixer when fully migrated

**Effort:** 5-7 hours (dual maintenance)

**Risk:** High - Two mixers active simultaneously, confusion

**Downsides:**
- Code complexity (which mixer for which code path?)
- Testing burden (must test both mixers)
- Merge conflicts / maintenance overhead

---

## Recommended Approach: Option A (Phased Migration)

### Sub-Increment 4a: Move Position Events to PlaybackEngine (3-4 hours)

**Objective:** Make PlaybackEngine emit position events (not mixer)

**Implementation:**

1. **Add position tracking to PlaybackEngine**:
   ```rust
   struct PlaybackEngine {
       // ... existing fields
       position_event_tx: mpsc::UnboundedSender<PositionUpdate>,
       position_event_interval_frames: usize,
       total_frames_mixed: usize,
   }
   ```

2. **Emit position events in playback loop**:
   ```rust
   // In get_next_frame() or audio callback
   self.total_frames_mixed += 1;
   if self.total_frames_mixed % self.position_event_interval_frames == 0 {
       let position_ms = self.calculate_position_ms();
       self.position_event_tx.try_send(PositionUpdate {
           passage_id: self.current_passage_id,
           position_ms,
       }).ok();
   }
   ```

3. **Remove position event dependencies from mixer instantiation**:
   ```rust
   // Remove these lines:
   // mixer.set_event_channel(position_event_tx.clone());
   // mixer.set_position_event_interval_ms(interval_ms);
   ```

4. **Test position events still work**:
   - Verify UI receives updates
   - Verify event frequency matches configured interval

**Deliverables:**
- Position event logic moved to PlaybackEngine
- Legacy mixer still active but simplified (no event channel)
- Tests pass (position events still functional)

---

### Sub-Increment 4b: Migrate to Correct Mixer (1-2 hours)

**Objective:** Replace legacy mixer with correct mixer

**Prerequisites:**
- Sub-Increment 4a complete (position events in engine)
- Master volume value determined (from settings/database)

**Implementation:**

1. **Update import**:
   ```rust
   use crate::playback::mixer::Mixer;  // Instead of CrossfadeMixer
   ```

2. **Load master volume from settings**:
   ```rust
   let master_volume = wkmp_common::config::get_master_volume(&db_pool).await?
       .unwrap_or(1.0);  // Default 100%
   ```

3. **Instantiate correct mixer**:
   ```rust
   let mixer = Mixer::new(master_volume);
   let mixer = Arc::new(RwLock::new(mixer));
   ```

4. **Remove buffer manager integration** (if underrun detection not needed):
   ```rust
   // Remove these lines:
   // mixer.set_buffer_manager(Arc::clone(&buffer_manager));
   // mixer.set_mixer_min_start_level(mixer_min_start_level);
   ```

   **OR** move underrun detection to PlaybackEngine if needed.

5. **Update pause/resume calls**:
   - Legacy: `mixer.pause()`, `mixer.resume(fade_ms, curve_name)`
   - Correct: `mixer.set_state(MixerState::Paused)`, `mixer.start_resume_fade(...)`

6. **Test playback functionality**:
   - Verify playback works
   - Verify pause/resume works with fade-in
   - Verify master volume control works

**Deliverables:**
- PlaybackEngine uses correct mixer
- All playback features functional
- Tests pass

---

### Sub-Increment 4c: Remove Legacy Mixer (30min)

**Objective:** Delete obsolete code

**Implementation:**

1. Delete `wkmp-ap/src/playback/pipeline/mixer.rs` (legacy mixer, 1,969 lines)
2. Remove from `wkmp-ap/src/playback/pipeline/mod.rs`
3. Verify no references remain (`cargo check`)

**Deliverables:**
- Legacy mixer deleted
- Codebase simplified (-1,969 lines)
- Clean build

---

## Revised PLAN014 Timeline

| Increment | Task | Effort | Status |
|-----------|------|--------|--------|
| 1-2 | Investigation + Documentation | 2-3h | ✅ COMPLETE |
| 3 | Feature Porting (Resume Fade-In) | 1-2h | ✅ COMPLETE |
| **4a** | **Move Position Events to Engine** | **3-4h** | ⏸️ **PENDING** |
| **4b** | **Migrate to Correct Mixer** | **1-2h** | ⏸️ BLOCKED (needs 4a) |
| **4c** | **Remove Legacy Mixer** | **30min** | ⏸️ BLOCKED (needs 4b) |
| 5 | Unit Tests | 2-3h | ⏸️ PENDING |
| 6 | Integration Tests | 3-4h | ⏸️ PENDING |
| 7 | Crossfade Tests | 2-3h | ⏸️ PENDING |

**Total Remaining Effort:** 12-17 hours (was 10-15 hours, +2h for position event migration)

---

## Position Event Migration Details

### Current Implementation (Legacy Mixer)

**Location:** `wkmp-ap/src/playback/pipeline/mixer.rs` lines 614-642

```rust
// In get_next_frame() per-frame processing
self.total_frames_mixed += 1;

if self.total_frames_mixed % self.position_event_interval_frames == 0 {
    if let Some(ref tx) = self.event_tx {
        let position_ms = self.calculate_position_ms();

        let current_passage_id = match &self.state {
            MixerState::SinglePassage { queue_entry_id, .. } => Some(*queue_entry_id),
            MixerState::Crossfading { next_queue_entry_id, .. } => Some(*next_queue_entry_id),
            _ => None,
        };

        if let Some(passage_id) = current_passage_id {
            tx.try_send(PlaybackEvent::PositionUpdate {
                passage_id,
                position_ms,
            }).ok();
        }
    }
}
```

### Proposed Implementation (PlaybackEngine)

**Location:** `wkmp-ap/src/playback/engine.rs` (in audio callback or similar)

```rust
// In PlaybackEngine struct
pub struct PlaybackEngine {
    // ... existing fields
    position_event_tx: mpsc::UnboundedSender<PlaybackEvent>,
    position_event_interval_frames: usize,
    total_frames_mixed: usize,
    current_passage_id: Option<Uuid>,
}

// In audio callback / get_next_frame equivalent
fn process_audio_frame(&mut self) {
    // ... mix audio using correct mixer

    // Emit position events
    self.total_frames_mixed += 1;
    if self.total_frames_mixed % self.position_event_interval_frames == 0 {
        if let Some(passage_id) = self.current_passage_id {
            let position_ms = self.calculate_position_ms();
            self.position_event_tx.try_send(PlaybackEvent::PositionUpdate {
                passage_id,
                position_ms,
            }).ok();
        }
    }
}
```

**Key Differences:**
- Engine already tracks `current_passage_id` (mixer doesn't need to)
- Position calculation uses engine's frame counter (not mixer's)
- Event emission happens at engine level (decoupled from mixing)

---

## Open Questions

### Q1: Where does master_volume come from?

**Answer Needed:** Check settings table or configuration for master volume value.

**Temporary Solution:** Use default 1.0 (100%) if not configured.

### Q2: Is buffer underrun detection still needed?

**Current State:** Legacy mixer has active underrun detection via `set_buffer_manager()`.

**Correct Mixer State:** Passive underrun handling (outputs silence).

**Decision Needed:**
- If active monitoring required → Move to PlaybackEngine
- If passive handling sufficient → Remove buffer_manager integration

### Q3: What is mixer_min_start_level used for?

**Current State:** Configured via `set_mixer_min_start_level(mixer_min_start_level)`.

**Likely Purpose:** Minimum buffer level before starting playback (avoid immediate underrun).

**Decision Needed:**
- If needed → Move to PlaybackEngine (check buffer before starting)
- If not needed → Remove configuration

---

## Recommendation

**Proceed with Sub-Increment 4a: Move Position Events to PlaybackEngine**

**Rationale:**
1. Aligns with architectural assessment (REQ-MIX-008 - position events belong in engine)
2. Unblocks mixer migration (removes dependency on legacy mixer features)
3. Clean separation of concerns (engine tracks state, mixer just mixes)
4. Enables future cleanup (delete legacy mixer)

**Estimated Total Effort for Increment 4:**
- Sub-Increment 4a: 3-4 hours
- Sub-Increment 4b: 1-2 hours
- Sub-Increment 4c: 30 minutes
- **Total: 5-6.5 hours**

---

**Next Step:** Implement Sub-Increment 4a (move position events to PlaybackEngine), OR defer Increment 4 until after testing (Increments 5-7) to validate correct mixer in isolation first.

**Alternative Approach:** Defer full migration, focus on testing correct mixer in isolation (Increments 5-7), THEN migrate engine (Increment 4) once correct mixer is fully validated.

---

**Report Complete**
**Date:** 2025-01-30
**Author:** Claude (PLAN014 Implementation)

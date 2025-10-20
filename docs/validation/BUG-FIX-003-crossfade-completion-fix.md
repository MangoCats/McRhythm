# Bug Fix Plan: Crossfade Completion Handling (BUG-003)

**Date:** 2025-10-20
**Priority:** Critical
**Complexity:** Medium
**Estimated Time:** 2-3 hours

---

## Problem Statement

When a crossfade completes, the mixer correctly promotes the incoming passage to current, but the engine:
1. Doesn't detect this transition
2. Incorrectly stops the mixer when checking completion
3. Restarts the same passage, causing it to play twice

---

## Root Cause

### Current Incomplete Implementation

**`is_current_finished()` in mixer.rs:555-569:**

```rust
pub async fn is_current_finished(&self) -> bool {
    match &self.state {
        MixerState::SinglePassage { buffer, position, .. } => {
            buf.is_exhausted(*position)
        }
        _ => false,  // ❌ Returns false during Crossfading!
    }
}
```

**Problem:** During crossfading, this always returns `false`, so the engine never knows when the outgoing passage finishes.

### Missing Functionality

The system needs to:
1. **Detect** when a crossfade transition occurs (Crossfading → SinglePassage)
2. **Notify** the engine that the outgoing passage is done
3. **Advance** the queue WITHOUT stopping the mixer (incoming passage continues)

---

## Proposed Solution

### Option A: Add Crossfade Completion Detection (Recommended)

**Changes Required:**

#### 1. Add state tracking to mixer
```rust
pub struct CrossfadeMixer {
    state: MixerState,
    // NEW: Track if crossfade just completed
    crossfade_completed_passage: Option<Uuid>,
    ...
}
```

#### 2. Set flag when crossfade completes
```rust
// In get_next_frame(), line 440
if *fade_out_progress >= *fade_out_duration_samples
    && *fade_in_progress >= *fade_in_duration_samples
{
    // Store the outgoing passage ID before transition
    let outgoing_passage_id = *current_passage_id;

    // Transition to SinglePassage
    self.state = MixerState::SinglePassage { ... };

    // NEW: Mark crossfade completion
    self.crossfade_completed_passage = Some(outgoing_passage_id);
}
```

#### 3. Add method to check/consume flag
```rust
pub fn take_crossfade_completed(&mut self) -> Option<Uuid> {
    self.crossfade_completed_passage.take()
}
```

#### 4. Update engine completion logic
```rust
// In process_queue(), BEFORE checking is_current_finished()

// Check if a crossfade just completed
if let Some(completed_id) = self.mixer.write().await.take_crossfade_completed() {
    // Crossfade completed - advance queue WITHOUT stopping mixer
    let queue_read = self.queue.read().await;
    if let Some(current) = queue_read.current() {
        if current.queue_entry_id == completed_id {
            drop(queue_read);

            // Emit PassageCompleted event for outgoing passage
            self.state.broadcast_event(WkmpEvent::PassageCompleted { ... });

            // Advance queue
            self.queue.write().await.advance();

            // Clean up outgoing passage's buffer
            self.buffer_manager.remove(completed_id).await;

            // ✅ DO NOT stop mixer - incoming passage continues!

            return Ok(());
        }
    }
}

// Then continue with normal is_current_finished() check...
```

**Pros:**
- Clean separation of concerns
- Explicit crossfade completion signaling
- No mixer stop/restart for crossfades

**Cons:**
- Requires state variable in mixer
- Two code paths for passage completion

---

### Option B: Fix is_current_finished() to Handle Crossfading

**Changes Required:**

#### 1. Check outgoing buffer during crossfade
```rust
pub async fn is_current_finished(&self) -> bool {
    match &self.state {
        MixerState::SinglePassage { buffer, position, .. } => {
            if let Ok(buf) = buffer.try_read() {
                buf.is_exhausted(*position)
            } else {
                false
            }
        }
        MixerState::Crossfading { current_buffer, current_position, .. } => {
            // NEW: Check if outgoing passage exhausted
            if let Ok(buf) = current_buffer.try_read() {
                buf.is_exhausted(*current_position)
            } else {
                false
            }
        }
        _ => false,
    }
}
```

#### 2. Update engine to handle "finished during crossfade"
```rust
if is_finished {
    // Check if mixer is crossfading
    let mixer_read = self.mixer.read().await;
    let is_crossfading = matches!(mixer_read.get_state(), MixerState::Crossfading { .. });
    drop(mixer_read);

    if is_crossfading {
        // Advance queue but DON'T stop mixer
        self.queue.write().await.advance();
        // ... cleanup ...
        // ✅ No mixer.stop() call
        return Ok(());
    } else {
        // Normal completion - stop and restart
        self.queue.write().await.advance();
        self.mixer.write().await.stop();
        return Ok(());
    }
}
```

**Pros:**
- Uses existing completion detection path
- Simpler logic

**Cons:**
- Checking outgoing buffer position during crossfade may be racy
- Still needs special case in engine

---

### Option C: Separate Crossfade Manager (Future Refactoring)

**Concept:** Extract crossfade logic into a separate state machine that coordinates mixer and queue.

**Pros:**
- Cleaner architecture
- Centralized crossfade coordination

**Cons:**
- Major refactoring
- Out of scope for immediate bug fix

---

## Recommendation: Option A

**Rationale:**
1. **Explicit signaling** makes the crossfade→completion transition clear
2. **No race conditions** - completion is signaled atomically when transition occurs
3. **Easier to test** - can verify crossfade completion event
4. **Maintainable** - clear separation between crossfade and normal completion

---

## Implementation Plan

### Phase 1: Add Crossfade Completion Tracking (30 min)
- [ ] Add `crossfade_completed_passage: Option<Uuid>` to CrossfadeMixer
- [ ] Set flag in crossfade transition code (line 440-453)
- [ ] Add `take_crossfade_completed()` method

### Phase 2: Update Engine Logic (45 min)
- [ ] Add crossfade completion check before `is_current_finished()`
- [ ] Emit PassageCompleted event for outgoing passage
- [ ] Advance queue without stopping mixer
- [ ] Clean up outgoing buffer

### Phase 3: Testing (60 min)
- [ ] Unit test: Crossfade completion flag
- [ ] Integration test: Three passages with crossfades
- [ ] Verify passage 2 plays only ONCE
- [ ] Verify no mixer stop during crossfade
- [ ] Verify queue advances correctly

### Phase 4: Documentation (15 min)
- [ ] Update crossfade state machine documentation
- [ ] Add comments explaining completion signaling
- [ ] Update bug report with fix details

---

## Test Cases

### Test 1: Crossfade Completion Flag Set
```rust
#[tokio::test]
async fn test_crossfade_sets_completion_flag() {
    let mut mixer = CrossfadeMixer::new();

    // Start passage 1
    mixer.start_passage(buffer1, passage1_id, None, 0).await;

    // Start crossfade to passage 2
    mixer.start_crossfade(buffer2, passage2_id, ...).await;

    // Read frames until crossfade completes
    while matches!(mixer.get_state(), MixerState::Crossfading { .. }) {
        mixer.get_next_frame().await;
    }

    // Verify completion flag set
    let completed = mixer.take_crossfade_completed();
    assert_eq!(completed, Some(passage1_id), "Should signal passage 1 completed");
}
```

### Test 2: No Double-Play After Crossfade
```rust
#[tokio::test]
async fn test_no_double_play_after_crossfade() {
    // Enqueue 3 passages
    // Monitor PassageStarted events
    // Verify passage 2 gets exactly ONE PassageStarted event
    // Verify passage 2 plays for expected duration (not 2x)
}
```

### Test 3: Queue Advances Without Mixer Stop
```rust
#[tokio::test]
async fn test_queue_advances_seamlessly() {
    // Start passage 1
    // Trigger crossfade to passage 2
    // Wait for crossfade completion
    // Verify queue advanced (1→2)
    // Verify mixer still playing passage 2 (not stopped)
    // Verify no mixer restart
}
```

---

## Risk Assessment

### Low Risk
- Adding state tracking field (isolated change)
- Crossfade completion detection (well-defined trigger point)

### Medium Risk
- Engine logic changes (affects main playback loop)
- Queue advancement timing (must not break normal completion)

### Mitigation
- Comprehensive test coverage before deployment
- Preserve existing completion logic as fallback
- Add extensive logging during development

---

## Success Criteria

✅ Passage 2 plays exactly ONCE after passage 1
✅ No mixer stop/restart during crossfade transitions
✅ Queue display updates correctly after crossfade
✅ All existing tests continue to pass
✅ New crossfade completion tests pass

---

## Next Steps

1. ✅ Analysis complete (this document)
2. ⏳ Implement Option A (est. 2 hours)
3. ⏳ Create comprehensive test suite
4. ⏳ Manual verification with 3-file scenario
5. ⏳ Update documentation

Would you like me to proceed with implementing Option A?

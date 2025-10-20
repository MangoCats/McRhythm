# Deep Investigation: Crossfade Completion Handling Bug (BUG-003)

**Date:** 2025-10-20
**Status:** Root Cause Identified
**Severity:** Critical
**Component:** Mixer ↔ Engine State Synchronization

---

## Executive Summary

When a crossfade completes (fade-out and fade-in finish), the **mixer correctly transitions** the incoming passage to become the new current passage (`Crossfading → SinglePassage`). However, the **engine remains unaware** of this transition and continues tracking the outgoing passage as "current" in its queue.

This causes a **state desynchronization** where:
- **Mixer thinks:** Passage 2 is current and playing
- **Engine thinks:** Passage 1 is current, then detects it "finished"
- **Engine incorrectly:** Stops the mixer and restarts Passage 2

**Result:** Passage 2 plays twice - once during/after crossfade, then again after mixer restart.

---

## The Crossfade State Machine

### Mixer State Transitions (CORRECT)

**Location:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/mixer.rs:437-453`

```rust
// Inside get_next_frame() for Crossfading state
if *fade_out_progress >= *fade_out_duration_samples
    && *fade_in_progress >= *fade_in_duration_samples
{
    // ✅ CORRECT: Transition to SinglePassage with INCOMING passage
    self.state = MixerState::SinglePassage {
        buffer: next_buffer.clone(),
        passage_id: next_passage_id,     // ← Passage 2 becomes current!
        position: next_position,
        fade_in_curve: None,
        fade_in_duration_samples: 0,
    };
}
```

**What this does:**
1. Checks if both fade-out (outgoing) and fade-in (incoming) have completed
2. Automatically promotes **incoming passage** to be the new current passage
3. Discards the **outgoing passage** (it's done fading out)

**This is CORRECT behavior** - the incoming passage should seamlessly continue as the current passage after crossfade.

---

## The Engine's Completion Detection (INCORRECT)

### Problem: Engine Unaware of Mixer Transitions

**Location:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/engine.rs:1446-1518`

```rust
// Engine checks if current passage finished
let mixer = self.mixer.read().await;
let is_finished = mixer.is_current_finished().await;  // ← Checks MIXER's current
drop(mixer);

if is_finished {
    // Get QUEUE's current passage
    if let Some(current) = queue.current() {  // ← This is QUEUE's current!
        info!("Passage {} completed", current.queue_entry_id);

        // ... advance queue ...

        // ❌ BUG: Stop mixer even though incoming passage is still playing!
        self.mixer.write().await.stop();  // Line 1512

        return Ok(());  // ← Next iteration will restart the same passage!
    }
}
```

### What `is_current_finished()` Checks

**Location:** `mixer.rs:555-569`

```rust
pub async fn is_current_finished(&self) -> bool {
    match &self.state {
        MixerState::SinglePassage { buffer, position, .. } => {
            if let Ok(buf) = buffer.try_read() {
                buf.is_exhausted(*position)  // ← Checks MIXER's current buffer
            } else {
                false
            }
        }
        _ => false,  // Returns false during Crossfading
    }
}
```

**Key Insight:**
- During `Crossfading`: Returns `false` (nothing finished yet)
- After transition to `SinglePassage(P2)`: Checks if **P2's buffer** is exhausted
- **Never checks** if P1's buffer is exhausted after crossfade!

---

## The Bug Timeline

### Actual Sequence of Events

```
T0: 16:25:06  Passage 1 starts playing
              Queue: current=P1, next=P2
              Mixer: SinglePassage(P1)

T1: 16:25:11  Crossfade trigger reached
              Engine: start_crossfade(P1 → P2)
              Mixer: Crossfading { current=P1, next=P2 }

T2: 16:25:17  Crossfade completes (fades finish)
              Mixer: SinglePassage(P2)  ← P2 is NOW current in mixer!
              Queue: current=P1, next=P2  ← Still thinks P1 is current!

T3: 16:27:18  P1's buffer exhausts (all samples played)
              Engine: is_current_finished() checks P2's buffer
              ??? How does engine detect P1 finished ???

T4: 16:27:18  Engine advances queue
              Queue: current=P2, next=P3
              Mixer: SinglePassage(P2) - already playing!

T5: 16:27:18  Engine stops mixer (line 1512)
              Mixer: None  ← STOPS while P2 was playing!

T6: 16:27:18  Next iteration of process_queue()
              Engine: "mixer idle, start P2"
              Mixer: SinglePassage(P2) - RESTARTED from beginning!
```

---

## The Critical Misunderstanding

### What SHOULD Happen

After crossfade completes (T2):
1. Mixer transitions to `SinglePassage(P2)` - **CORRECT**
2. Engine should detect this transition
3. Engine should advance queue: `current=P2, next=P3`
4. P1's buffer should be cleaned up
5. **No mixer stop/restart** - P2 continues seamlessly

### What ACTUALLY Happens

After crossfade completes (T2):
1. Mixer transitions to `SinglePassage(P2)` - **CORRECT**
2. ❌ Engine remains unaware of transition
3. ❌ Queue still shows `current=P1`
4. ❌ Later, engine detects P1's buffer exhausted somehow
5. ❌ Engine stops mixer and restarts P2

---

## Open Question: How Does P1's Completion Get Detected?

**Mystery:** At T3, how does the engine know P1 finished?

**Analysis:**

`is_current_finished()` checks the mixer's current buffer:
```rust
MixerState::SinglePassage { buffer, position, .. } => {
    buf.is_exhausted(*position)  // Checks mixer's current buffer
}
```

After T2, mixer's current buffer is **P2**, not P1. So checking `is_exhausted()` should check P2's position against P2's total_frames, which would return `false` (P2 just started).

**Hypothesis 1:** Maybe the buffer manager tracks P1 separately?

Let me check if there's another completion check...

Actually, looking at the debug log pattern from Agent 1's analysis, the "Passage completed" message at 16:27:18.417 suggests `is_finished` returned `true` at line 1447.

**Hypothesis 2:** P1's buffer is somehow being checked by `is_current_finished()`

Wait! What if the crossfade completion transition (line 446) happens AFTER the engine's completion check? Race condition?

Let me think about the timing:
- Mixer runs at 44100 Hz (22.7μs per frame)
- Engine's process_queue runs at 100ms intervals
- Crossfade duration: ~5 seconds

**Timeline possibility:**
```
T1: Engine checks is_current_finished()
    → Mixer in Crossfading state
    → Returns false

T2: [98ms later] Mixer processes frames
    → Fade completes, transitions to SinglePassage(P2)

T3: [2ms later] Engine checks is_current_finished() AGAIN
    → Mixer in SinglePassage(P2) state
    → Checks P2's buffer
    → P2 is NOT exhausted (just started)
    → Returns false
```

This doesn't explain how P1's completion is detected either!

**Hypothesis 3:** The issue is in buffer exhaustion tracking

Let me check if `mark_exhausted()` is called somewhere for P1...

Actually, re-reading line 1467:
```rust
self.buffer_manager.mark_exhausted(current.queue_entry_id).await;
```

This marks the QUEUE's current entry (P1) as exhausted, not the mixer's current passage!

But this happens INSIDE the `if is_finished` block, so it's a consequence, not a cause.

**Hypothesis 4:** P2's buffer gets exhausted prematurely

What if P2's buffer only has 15 seconds of data (like in BUG-002), and after playing for those 15 seconds (during + after crossfade), it exhausts? Then `is_current_finished()` would return true for P2, triggering the completion logic which uses queue.current() (P1) to log the message!

Let me check Agent 1's analysis about P2's buffer size...

From Agent 1: "Passage 2 (Pipeline) plays TWICE in full, lasting 253 seconds instead of the expected 137.6 seconds."

So P2 plays for 126.5 seconds each time, suggesting the buffer has full audio data.

**Wait, I think I found it!**

Looking at Agent 1's timeline:
```
16:25:11.925  Passage 2 starts (1st time)
16:27:18.417  Passage 1 completes
```

Time difference: 126.5 seconds

Agent 1 states passage 2's expected duration is 137.6 seconds. So P2 doesn't exhaust during this time.

But Agent 1 also says: "Passage 1 completes after 132 seconds (normal)"

So P1's ACTUAL buffer exhaustion happens at 16:27:18, which is when the engine detects completion.

**I think the issue is:**

The engine is checking if **P1's buffer** is exhausted directly, not through the mixer's `is_current_finished()`!

Let me search for alternative completion checks...

Actually, maybe the `is_current_finished()` implementation has a bug where it checks the WRONG buffer after crossfade transition?

**Aha! I bet the issue is this:**

During Crossfading state, the mixer holds BOTH buffers:
```rust
Crossfading {
    current_buffer: Arc<RwLock<PassageBuffer>>,  // P1
    next_buffer: Arc<RwLock<PassageBuffer>>,     // P2
    ...
}
```

When `is_current_finished()` is called during Crossfading, it returns `false` (line 567).

But what if it should check if the **outgoing buffer (current_buffer)** is exhausted? That would tell the engine that P1 finished!

**This is the bug:** `is_current_finished()` returns `false` during Crossfading, but it SHOULD check if the outgoing passage has exhausted!


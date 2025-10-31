# Increment 3 Architecture Assessment

**Date:** 2025-01-30
**Status:** Architecture Review Required
**Issue:** Feature porting approach needs reconsideration

---

## Problem Statement

PLAN014 originally assumed we would port three features from legacy mixer to correct mixer:
- REQ-MIX-007: Buffer underrun detection
- REQ-MIX-008: Position event emission
- REQ-MIX-009: Resume fade-in support

However, after examining both implementations in detail, there's a fundamental architectural mismatch.

---

## Architectural Comparison

### Legacy Mixer Architecture (`pipeline/mixer.rs`, 1,969 lines)

**Responsibilities:**
- Buffer management (holds references to BufferManager)
- Passage tracking (stores passage IDs in state machine)
- Underrun detection (monitors buffer levels per passage)
- Position tracking (tracks frame counts per passage)
- Event emission (emits PositionUpdate events)
- Fade curve application ❌ (VIOLATES SPEC016)
- Crossfade timing coordination
- Resume fade-in application ✅

**State Machine:**
```rust
enum MixerState {
    None,
    SinglePassage { passage_id, fade_curves, frame_count, ... },
    Crossfading { current_id, next_id, fade_curves, crossfade_frame_count, ... },
}
```

**Architecture:** Stateful, complex, tightly coupled to buffer management

### Correct Mixer Architecture (`mixer.rs`, 359 lines)

**Responsibilities:**
- Read pre-faded samples from buffers (simple)
- Sum overlapping samples (simple addition)
- Apply master volume
- Pause mode decay
- **NO buffer management**
- **NO passage tracking**
- **NO fade curve application** ✅ (COMPLIANT with SPEC016)

**State Machine:**
```rust
enum MixerState {
    Playing,
    Paused,
}
```

**Architecture:** Stateless, simple, decoupled from buffer management

---

## Feature-by-Feature Analysis

### REQ-MIX-007: Buffer Underrun Detection

**Legacy Implementation (lines 422-611):**
- Monitors `BufferManager` per passage
- Tracks `underrun_state` with passage ID, position, flatline frame
- Auto-resumes when buffer refills
- Emits warnings

**Assessment:**
✅ **Already handled by correct mixer** (line 172-175):
```rust
// Fill remainder with silence if buffer underrun
for i in samples.len()..output.len() {
    output[i] = 0.0;
}
```

**Difference:**
- Legacy: Active underrun detection (monitors buffer, logs warnings, auto-resume logic)
- Correct: Passive underrun handling (outputs silence if buffer empty)

**Recommendation:**
- Correct mixer's approach is simpler and sufficient
- **No porting needed** - behavior is already correct (outputs silence on underrun)
- If active monitoring is needed, it should be at **PlaybackEngine level**, not mixer level

---

### REQ-MIX-008: Position Event Emission

**Legacy Implementation (lines 614-642):**
```rust
// Track frames and emit position events
self.total_frames_mixed += 1;
if self.total_frames_mixed % self.position_event_interval_frames == 0 {
    // Emit PositionUpdate event with passage_id, position_ms
    self.event_tx.try_send(PositionUpdate { ... });
}
```

**Assessment:**
❌ **Cannot port to correct mixer** - architectural mismatch

**Reasons:**
1. Correct mixer doesn't track passage IDs (stateless by design)
2. Correct mixer doesn't know which passage is playing
3. Position tracking belongs at **PlaybackEngine level**, not mixer level

**Recommendation:**
- **Move to PlaybackEngine** - engine knows which passage is playing, tracks position
- Mixer should remain stateless (SPEC016 compliant)
- **No mixer changes needed**

---

### REQ-MIX-009: Resume Fade-In Support

**Legacy Implementation (lines 644-671):**
```rust
// After mixing, apply resume fade-in multiplicatively
if let Some(ref mut resume) = self.resume_state {
    let fade_position = resume.frames_since_resume as f32 / resume.fade_duration_samples as f32;
    let resume_fade_multiplier = resume.fade_in_curve.calculate_fade_in(fade_position);
    final_frame.apply_volume(resume_fade_multiplier);
    resume.frames_since_resume += 1;
}
```

**Assessment:**
✅ **Can and should port to correct mixer**

**Reasons:**
1. Resume fade-in is **mixer-level fade** (orthogonal to passage-level fades)
2. Applied AFTER mixing (multiplicative, to final output)
3. Doesn't require passage tracking (applies to entire mixed output)
4. Maintains SPEC016 compliance (not a passage-level fade)

**Recommendation:**
- **Port this feature** to correct mixer
- Add `ResumeState` struct
- Apply fade multiplicatively after mixing (line 236, after master volume)
- Maintain architectural separation (no passage-level fade knowledge)

---

## Revised Implementation Plan

### What to Port (1 feature)

**REQ-MIX-009 ONLY: Resume Fade-In Support**

**Implementation:**
1. Add `ResumeState` struct to correct mixer:
   ```rust
   struct ResumeState {
       fade_duration_samples: usize,
       fade_in_curve: FadeCurve,
       frames_since_resume: usize,
   }
   ```

2. Add field to `Mixer` struct:
   ```rust
   resume_state: Option<ResumeState>,
   ```

3. Add methods:
   ```rust
   pub fn start_resume_fade(&mut self, fade_duration_samples: usize, fade_in_curve: FadeCurve)
   pub fn is_resume_fading(&self) -> bool
   ```

4. Apply fade in `mix_single()` and `mix_crossfade()`:
   ```rust
   // After applying master volume, before outputting
   if let Some(ref mut resume) = self.resume_state {
       let fade_position = resume.frames_since_resume as f32 / resume.fade_duration_samples as f32;
       if fade_position < 1.0 {
           let multiplier = resume.fade_in_curve.calculate_fade_in(fade_position);
           output[i] *= multiplier;
           resume.frames_since_resume += 1;
       } else {
           // Fade complete, remove state
           self.resume_state = None;
       }
   }
   ```

**Estimated Effort:** 1-2 hours

### What NOT to Port (2 features)

**REQ-MIX-007: Buffer Underrun Detection**
- **Status:** Already handled (outputs silence on underrun)
- **Action:** None needed in mixer
- **Note:** If active monitoring wanted, add to PlaybackEngine, not mixer

**REQ-MIX-008: Position Event Emission**
- **Status:** Architectural mismatch (mixer is stateless)
- **Action:** Move to PlaybackEngine (engine tracks passages, positions)
- **Note:** Mixer should NOT track passage IDs or positions

---

## Impact on PLAN014

### Updated Migration Strategy

**Phase 2 (Current):** Specification clarifications ✅ Complete

**Phase 3 (Revised):** Port resume fade-in ONLY (1 feature, not 3)
- Estimated effort: 1-2 hours (down from 3-5 hours)
- **Skip REQ-MIX-007, REQ-MIX-008** (not needed or architectural mismatch)

**Phase 4:** Migrate PlaybackEngine to use correct mixer
- Update instantiation
- **Add position event emission to PlaybackEngine** (move from mixer)
- Update method calls

**Phase 5-7:** Testing (unit, integration, crossfade)

**Phase 8:** Remove legacy mixer

### Test Adjustments

**TC-F-007-01 (Underrun Detection):**
- **Status:** PASS by default (correct mixer already handles underrun)
- **Test:** Verify silence output when buffer empty
- **No implementation needed**

**TC-F-008-01 (Position Events):**
- **Status:** Move to PlaybackEngine tests
- **Not a mixer test** (architectural change)
- **PlaybackEngine implementation required**

**TC-F-009-01 (Resume Fade-In):**
- **Status:** Implement as planned
- **Test:** Verify fade starts at 0.0, reaches 1.0, correct curve

---

## Recommendation

**Proceed with simplified Increment 3:**

1. ✅ Port resume fade-in to correct mixer (REQ-MIX-009)
2. ❌ Skip underrun detection (already handled)
3. ❌ Skip position events (move to PlaybackEngine, separate from mixer refactoring)

**Rationale:**
- Maintains architectural separation (mixer stateless, engine stateful)
- Reduces implementation effort (1-2 hours vs. 3-5 hours)
- Avoids architectural violations (mixer shouldn't track passages)
- Correct mixer remains simple and SPEC016 compliant

**Decision Required:**
- Approve simplified Increment 3 (1 feature instead of 3)?
- Defer position events to separate PlaybackEngine enhancement?
- Mark REQ-MIX-007/008 as "Not Applicable" or "Moved to Engine"?

---

**Assessment Complete**
**Date:** 2025-01-30

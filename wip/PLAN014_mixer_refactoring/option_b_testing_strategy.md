# PLAN014 Option B: Testing-First Strategy

**Date:** 2025-01-30
**Status:** Active - Revert Complete, Testing Phase Beginning
**Decision:** Test mixer in isolation BEFORE full PlaybackEngine integration

---

## Decision Context

After completing Sub-Increment 4a (architecture phase), attempted Sub-Increment 4b (PlaybackEngine integration) revealed significant complexity:

**Integration Analysis Results:**
- 20+ missing methods when engine uses correct mixer
- Legacy mixer API fundamentally different (stateful/pull vs. event-driven/push)
- Estimated effort: 13-19 hours (medium-high risk)
- 5 integration phases required (playback loop, state management, markers, crossfade, testing)

**Options Evaluated:**
- **Option A:** Full integration immediately (13-19 hours, higher risk)
- **Option B:** Test mixer in isolation first, then integrate (7-10 hours testing + 13-19 hours integration, lower risk) **← CHOSEN**
- **Option C:** Adapter layer (4-6 hours + technical debt, NOT RECOMMENDED)

---

## Why Option B?

### Risk Mitigation
- Validate marker system works correctly BEFORE large refactor
- Discover issues in isolated environment (easier debugging)
- Build confidence in architecture before committing to integration
- Lower risk of breaking existing playback during testing phase

### Effort Optimization
- 7-10 hours testing vs. 13-19 hours integration
- Testing phase validates approach or reveals issues early
- If issues found, cheaper to fix in isolation than during integration
- If successful, integration proceeds with confidence

### Architecture Validation
- Marker system implementation unproven in real use
- Event-driven timing untested
- Position tracking accuracy unverified
- Testing phase proves architecture before full commitment

---

## Option B Implementation Status

### Phase 1: Revert Changes (COMPLETE)

**Files Reverted:**

1. **[wkmp-ap/src/playback/engine.rs](../../wkmp-ap/src/playback/engine.rs)**
   - Line 18: Import restored to `CrossfadeMixer`
   - Line 88: Type restored to `Arc<RwLock<CrossfadeMixer>>`
   - Lines 239-244: Legacy mixer instantiation restored with all configuration

2. **[wkmp-ap/src/playback/mod.rs](../../wkmp-ap/src/playback/mod.rs)**
   - Line 17: Commented out `pub mod mixer` (temporarily disabled)
   - Reason: Correct mixer has build errors (PlayoutRingBuffer API mismatch)
   - Will be re-enabled during testing phase after fixing API

**Build Status:** ✅ SUCCESS
- `cargo check` passes with warnings only
- Engine uses legacy mixer (temporary)
- Correct mixer code preserved (commented out in mod.rs)

---

## Testing Strategy (Increments 5-7)

### Phase 2: Fix Correct Mixer API (NEXT)

**Problem:** Correct mixer calls `PlayoutRingBuffer::pop()` which doesn't exist

**Root Cause Analysis:**
- Legacy mixer reads from `BufferManager` (abstracts ring buffer access)
- Correct mixer attempts to read directly from `PlayoutRingBuffer`
- `PlayoutRingBuffer` has different API (no `pop()` method)

**Solution Options:**
1. Add `pop()` method to `PlayoutRingBuffer` (cleanest)
2. Use `BufferManager` in correct mixer like legacy (maintains abstraction)
3. Call existing `PlayoutRingBuffer` methods directly (requires understanding API)

**Recommended:** Option 2 - Use `BufferManager` in correct mixer
- Maintains existing abstraction layer
- Proven pattern (legacy mixer uses this)
- Minimal changes to correct mixer
- Consistent with architecture

**Implementation Steps:**
1. Update `mix_single()` signature: replace `PlayoutRingBuffer` with `&BufferManager` + `chain_index`
2. Update `mix_crossfade()` signature: replace two `PlayoutRingBuffer` params with `&BufferManager` + two chain indices
3. Inside mixing methods: call `buffer_manager.read_frames(chain_index, frames_requested)`
4. Update error handling to match `BufferManager` API
5. Re-enable `pub mod mixer` in `mod.rs`
6. Verify `cargo check` passes

### Phase 3: Unit Tests (Increment 5)

**Estimated Effort:** 2-3 hours

**Test Coverage:**

1. **Marker System Tests:**
   - Add marker, verify stored in heap
   - Add multiple markers, verify sorted by tick (min-heap)
   - Check markers during mixing, verify events emitted at correct ticks
   - Clear markers for passage, verify only relevant markers removed
   - Clear all markers, verify heap empty

2. **Position Tracking Tests:**
   - Set current passage, verify `get_current_tick()` returns 0
   - Mix frames, verify `current_tick` advances by frame count
   - Verify `frames_written` increments correctly
   - Verify `get_frames_written()` returns correct total

3. **Marker Lifecycle Tests:**
   - Set marker at tick 1000, mix to tick 999, verify no event
   - Mix 1 more frame past tick 1000, verify event emitted
   - Verify marker removed from heap after emission
   - Set marker for different passage, mix current passage, verify no event

4. **Event Types Tests:**
   - Verify PositionUpdate event contains correct position_ms
   - Verify StartCrossfade event contains correct next_passage_id
   - Verify SongBoundary event contains correct new_song_id
   - Verify PassageComplete event emitted at end

**Test Harness Requirements:**
- Mock `BufferManager` (or use real instance with test data)
- Pre-filled audio buffers with known sample data
- Tick calculation helpers (frames to ticks conversion)
- Event verification helpers (assert event type, payload)

### Phase 4: Integration Tests (Increment 6)

**Estimated Effort:** 3-4 hours

**Test Scenarios:**

1. **Single Passage Playback:**
   - Load passage buffer with test audio
   - Set position markers at 100ms intervals
   - Mix entire passage
   - Verify position events emitted at correct times
   - Verify PassageComplete event at end

2. **Crossfade Timing:**
   - Set up two passage buffers
   - Calculate crossfade start tick
   - Set StartCrossfade marker
   - Mix until marker reached
   - Verify StartCrossfade event emitted at exact tick
   - Switch to crossfade mixing mode
   - Verify crossfade completes correctly

3. **Pause and Resume:**
   - Mix passage to midpoint
   - Call `set_state(MixerState::Paused)`
   - Verify mixing paused (output silent)
   - Call `set_state(MixerState::Playing)` + `start_resume_fade()`
   - Verify fade-in applied correctly
   - Verify position tracking continues accurately

4. **Volume Control:**
   - Set master volume to 0.5
   - Mix frames, verify output at half amplitude
   - Change volume to 1.0
   - Mix more frames, verify output at full amplitude

**Test Infrastructure:**
- Real `BufferManager` instance
- Pre-decoded test audio files (WAV format, known content)
- Sample-accurate timing verification
- Audio content verification (silence, amplitude, crossfade curves)

### Phase 5: Crossfade Accuracy Tests (Increment 7)

**Estimated Effort:** 2-3 hours

**Test Focus:** Sample-accurate timing validation

1. **Crossfade Start Precision:**
   - Set crossfade marker at tick 100,000
   - Mix frames in batches (varying sizes)
   - Verify StartCrossfade event triggers exactly when `current_tick >= 100000`
   - Verify no event at tick 99,999
   - Verify event at tick 100,000

2. **Crossfade Completion Detection:**
   - Mix crossfade with known durations (0.5s, 1s, 2s)
   - Verify PassageComplete event at exact end tick
   - Verify no frames mixed beyond completion

3. **Edge Cases:**
   - Marker at tick 0 (immediate event)
   - Multiple markers at same tick (all emitted)
   - Marker beyond passage length (not emitted)
   - Passage change with pending markers (cleared correctly)

4. **Performance Tests:**
   - 1000 markers in heap (verify O(log n) performance)
   - Rapid marker additions during mixing (stress test)
   - Memory usage validation (no leaks)

**Success Criteria:**
- All crossfade events sample-accurate (±1 frame tolerance)
- No position drift (calculated vs. actual)
- No memory leaks or performance degradation

---

## Testing Phase Completion Criteria

**Architecture Validated:**
- ✅ Marker system stores/retrieves events correctly
- ✅ Position tracking sample-accurate
- ✅ Events emitted at exact ticks (no drift)
- ✅ BinaryHeap performance acceptable

**API Proven:**
- ✅ `add_marker()` works correctly
- ✅ `mix_single()` / `mix_crossfade()` return events
- ✅ `set_current_passage()` initializes tracking
- ✅ `clear_markers_*()` methods work as documented

**Ready for Integration:**
- ✅ No fundamental architecture issues
- ✅ Edge cases handled correctly
- ✅ Performance acceptable for production
- ✅ Test coverage demonstrates correctness

---

## Post-Testing: Integration Phase (Sub-Increment 4b)

**When Testing Complete:**
- Proceed with PlaybackEngine integration (13-19 hours)
- Use proven mixer with confidence
- Reference test cases during integration
- Validate integration with existing test suite

**Integration Approach:**
- Incremental replacement (phase by phase per integration_requirements.md)
- Keep legacy mixer functional until full integration complete
- Run tests after each integration phase
- Rollback capability if issues discovered

---

## Files Modified (Option B Revert)

**Code Changes:**
- [wkmp-ap/src/playback/engine.rs](../../wkmp-ap/src/playback/engine.rs) - Restored legacy mixer usage
- [wkmp-ap/src/playback/mod.rs](../../wkmp-ap/src/playback/mod.rs) - Disabled correct mixer (temporary)

**Documentation Created:**
- [integration_requirements.md](integration_requirements.md) - Integration analysis
- [option_b_testing_strategy.md](option_b_testing_strategy.md) - This document

**Documentation Updated:**
- [implementation_status.md](implementation_status.md) - Reflects Option B decision
- [phase_complete_summary.md](phase_complete_summary.md) - Architecture complete, integration pending

---

## Next Actions

1. **Fix Correct Mixer API** (30 min - 1 hour)
   - Update `mix_single()` / `mix_crossfade()` to use `BufferManager`
   - Re-enable in `mod.rs`
   - Verify build succeeds

2. **Create Test Harness** (1-2 hours)
   - Mock infrastructure for unit tests
   - Test audio file generation
   - Event verification helpers

3. **Execute Increment 5** (2-3 hours)
   - Write and run unit tests
   - Fix any discovered issues
   - Document test results

4. **Execute Increment 6** (3-4 hours)
   - Write and run integration tests
   - Validate sample accuracy
   - Document test results

5. **Execute Increment 7** (2-3 hours)
   - Write and run crossfade accuracy tests
   - Performance validation
   - Final architecture sign-off

6. **Proceed to Sub-Increment 4b** (13-19 hours)
   - Full PlaybackEngine integration
   - Phase-by-phase replacement
   - Production validation

---

**Report Date:** 2025-01-30
**Status:** Revert Complete - Testing Phase Ready to Begin
**Estimated Testing Phase Duration:** 7-10 hours
**Estimated Total Remaining (Testing + Integration):** 20-29 hours

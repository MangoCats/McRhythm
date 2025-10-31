# PLAN014 Testing Plan: Increments 5-7

**Date:** 2025-01-30
**Status:** Ready to Execute
**Objective:** Validate correct mixer architecture in isolation before PlaybackEngine integration

---

## Test Environment Setup

### Prerequisites

**1. Fix Mixer API (Required First):**
- Update `mix_single()` signature: `&mut BufferManager, chain_index: usize` instead of `&mut PlayoutRingBuffer`
- Update `mix_crossfade()` signature: `&mut BufferManager, current_chain: usize, next_chain: usize` instead of two `&mut PlayoutRingBuffer`
- Re-enable `pub mod mixer` in [playback/mod.rs](../../wkmp-ap/src/playback/mod.rs)
- Verify `cargo check` passes

**2. Test Infrastructure:**
- Create `wkmp-ap/tests/mixer_tests/` directory
- Mock or use real `BufferManager` instances
- Generate test audio files (sine waves, known amplitudes)
- Helper functions for tick calculations and event verification

---

## Increment 5: Unit Tests (2-3 hours)

**Goal:** Verify marker system mechanics and position tracking

### Test Suite 1: Marker Storage and Retrieval

**File:** `wkmp-ap/tests/mixer_tests/test_marker_storage.rs`

**Tests:**
1. `test_add_single_marker()`
   - Create mixer, add marker at tick 1000
   - Verify marker stored (internal state inspection or indirect behavior)

2. `test_add_multiple_markers_sorted()`
   - Add markers at ticks: 2000, 500, 1500, 1000
   - Mix frames to each tick sequentially
   - Verify events emitted in correct order: 500, 1000, 1500, 2000

3. `test_marker_min_heap_property()`
   - Add 10 markers with random ticks
   - Mix gradually, verify events emitted in ascending tick order

4. `test_clear_markers_for_passage()`
   - Add markers for passage A (ticks 100, 200, 300)
   - Add markers for passage B (ticks 150, 250)
   - Call `clear_markers_for_passage(passage_a_id)`
   - Mix to tick 400
   - Verify only passage B events emitted (150, 250)

5. `test_clear_all_markers()`
   - Add multiple markers
   - Call `clear_all_markers()`
   - Mix to large tick value
   - Verify no events emitted

### Test Suite 2: Position Tracking

**File:** `wkmp-ap/tests/mixer_tests/test_position_tracking.rs`

**Tests:**
1. `test_initial_position_zero()`
   - Create mixer, set current passage
   - Verify `get_current_tick()` returns 0
   - Verify `get_frames_written()` returns 0

2. `test_tick_advancement_single_mix()`
   - Set current passage, tick = 0
   - Mix 1024 frames (stereo: 512 samples per channel)
   - Verify `get_current_tick()` == 512
   - Mix 1024 more frames
   - Verify `get_current_tick()` == 1024

3. `test_frames_written_accumulation()`
   - Mix 1024 frames → verify `get_frames_written()` == 1024
   - Mix 512 frames → verify `get_frames_written()` == 1536
   - Verify counter persists across passage changes

4. `test_position_reset_on_passage_change()`
   - Set current passage A, mix to tick 1000
   - Set current passage B (new passage)
   - Verify `get_current_tick()` resets to 0
   - Verify `get_frames_written()` continues accumulating (NOT reset)

### Test Suite 3: Marker Event Emission

**File:** `wkmp-ap/tests/mixer_tests/test_marker_events.rs`

**Tests:**
1. `test_event_emission_exact_tick()`
   - Add marker at tick 1000
   - Mix 1998 frames (999 samples) → verify no event
   - Mix 2 more frames (1 sample) → verify event emitted
   - Verify `current_tick` == 1000 when event emitted

2. `test_event_emission_past_tick()`
   - Add marker at tick 100
   - Mix 1000 frames (jumps past marker)
   - Verify event emitted even though tick exceeded

3. `test_multiple_events_same_batch()`
   - Add markers at ticks 10, 20, 30
   - Mix 100 frames (covers all markers)
   - Verify all 3 events returned in single `Vec<MarkerEvent>`
   - Verify events in ascending tick order

4. `test_marker_removed_after_emission()`
   - Add marker at tick 500
   - Mix to tick 600 → verify event emitted
   - Mix to tick 700 → verify no duplicate event
   - Verify marker no longer in heap

5. `test_marker_for_different_passage_ignored()`
   - Set current passage A
   - Add marker for passage B at tick 100
   - Mix passage A to tick 200
   - Verify no events emitted
   - Set current passage B, mix to tick 200
   - Verify event emitted (if marker still valid)

### Test Suite 4: Event Type Verification

**File:** `wkmp-ap/tests/mixer_tests/test_event_types.rs`

**Tests:**
1. `test_position_update_event()`
   - Add `PositionUpdate { position_ms: 500 }` marker
   - Mix to tick, verify event payload correct

2. `test_start_crossfade_event()`
   - Add `StartCrossfade { next_passage_id: uuid }` marker
   - Verify event contains correct passage UUID

3. `test_song_boundary_event()`
   - Add `SongBoundary { new_song_id: Some(uuid) }` marker
   - Verify event payload correct

4. `test_passage_complete_event()`
   - Add `PassageComplete` marker at end tick
   - Verify event emitted at passage completion

---

## Increment 6: Integration Tests (3-4 hours)

**Goal:** Validate mixer behavior in realistic playback scenarios

### Test Suite 5: Single Passage Playback

**File:** `wkmp-ap/tests/mixer_tests/test_single_passage.rs`

**Test Audio:** 5-second sine wave at 440Hz, 44.1kHz sample rate

**Tests:**
1. `test_play_full_passage_with_position_markers()`
   - Load passage buffer (5s = 220,500 frames)
   - Set position markers every 100ms (4,410 frames)
   - Mix entire passage in batches
   - Verify 50 position events emitted at correct ticks
   - Verify PassageComplete event at end

2. `test_position_accuracy()`
   - Set markers at known ticks (convert ms to ticks)
   - Capture emitted position_ms values
   - Verify position_ms matches expected within ±1ms tolerance

3. `test_audio_output_correctness()`
   - Mix passage, capture output buffer
   - Verify output is sine wave at 440Hz (FFT analysis or zero-crossing count)
   - Verify no clipping or distortion

### Test Suite 6: Crossfade Timing

**File:** `wkmp-ap/tests/mixer_tests/test_crossfade_timing.rs`

**Test Audio:** Two 10-second passages (different frequencies: 440Hz, 880Hz)

**Tests:**
1. `test_crossfade_marker_triggers_correctly()`
   - Passage A: 10s duration (441,000 frames)
   - Crossfade duration: 2s (88,200 frames)
   - Crossfade start tick: 441,000 - 88,200 = 352,800
   - Set `StartCrossfade` marker at tick 352,800
   - Mix to tick 352,799 → verify no event
   - Mix 1 more frame → verify `StartCrossfade` event emitted
   - Verify `next_passage_id` correct in event

2. `test_crossfade_mixing_sample_accurate()`
   - After StartCrossfade event, switch to `mix_crossfade()`
   - Mix until passage A completes (88,200 frames)
   - Verify output contains both frequencies (FFT shows 440Hz + 880Hz)
   - Verify fade curves applied (amplitude decreases for A, increases for B)
   - Verify PassageComplete event at exact tick

3. `test_crossfade_completion_detection()`
   - Set PassageComplete marker at end of crossfade
   - Mix until completion
   - Verify event emitted at exact tick
   - Verify no frames mixed beyond completion

### Test Suite 7: Pause and Resume

**File:** `wkmp-ap/tests/mixer_tests/test_pause_resume.rs`

**Tests:**
1. `test_pause_stops_mixing()`
   - Mix to midpoint (tick 50,000)
   - Call `set_state(MixerState::Paused)`
   - Mix frames → verify output is silent (all zeros)
   - Verify `current_tick` does NOT advance

2. `test_resume_with_fade_in()`
   - Pause at tick 50,000
   - Call `set_state(MixerState::Playing)` + `start_resume_fade(500)` (500ms fade)
   - Mix 22,050 frames (500ms)
   - Verify output starts at low amplitude, ramps to full
   - Verify `is_resume_fading()` returns true during fade
   - After fade complete, verify `is_resume_fading()` returns false

3. `test_position_tracking_during_pause()`
   - Set marker at tick 60,000
   - Pause at tick 50,000
   - Mix (output silent)
   - Resume, mix to tick 65,000
   - Verify marker event emitted at tick 60,000 (not lost during pause)

### Test Suite 8: Volume Control

**File:** `wkmp-ap/tests/mixer_tests/test_volume_control.rs`

**Tests:**
1. `test_master_volume_applied()`
   - Set master volume to 0.5
   - Mix frames with known amplitude (1.0 peak)
   - Verify output amplitude ~0.5 peak

2. `test_volume_change_during_playback()`
   - Mix 10,000 frames at volume 0.5
   - Set master volume to 1.0
   - Mix 10,000 more frames
   - Verify first batch at half amplitude, second at full amplitude

3. `test_volume_with_crossfade()`
   - Set master volume to 0.5
   - Mix crossfade between two passages
   - Verify crossfaded output at correct amplitude (both passages affected)

---

## Increment 7: Crossfade Accuracy Tests (2-3 hours)

**Goal:** Prove sample-accurate timing with stress tests and edge cases

### Test Suite 9: Sample-Accurate Crossfade Timing

**File:** `wkmp-ap/tests/mixer_tests/test_crossfade_accuracy.rs`

**Tests:**
1. `test_crossfade_start_exact_tick()`
   - Set marker at tick 100,000
   - Mix in varying batch sizes (64, 128, 512, 1024 frames)
   - Verify StartCrossfade event triggers exactly when `current_tick >= 100,000`
   - Test with marker at tick boundary and mid-batch

2. `test_multiple_crossfade_durations()`
   - Test crossfades: 0.5s, 1s, 2s, 5s, 10s
   - Verify each completes at exact calculated tick
   - Verify no frames lost or duplicated

3. `test_crossfade_no_position_drift()`
   - Mix 100 passages with crossfades
   - Track expected tick vs. actual tick after each passage
   - Verify no cumulative drift (< 10 frames total error)

### Test Suite 10: Edge Cases

**File:** `wkmp-ap/tests/mixer_tests/test_edge_cases.rs`

**Tests:**
1. `test_marker_at_tick_zero()`
   - Set marker at tick 0
   - Mix first batch
   - Verify event emitted immediately

2. `test_multiple_markers_same_tick()`
   - Add 3 markers at tick 1000 (different event types)
   - Mix to tick 1001
   - Verify all 3 events emitted

3. `test_marker_beyond_passage_length()`
   - Passage length: 100,000 frames
   - Set marker at tick 150,000
   - Mix entire passage
   - Verify marker NOT emitted (passage ends first)

4. `test_passage_change_clears_pending_markers()`
   - Add markers for passage A at ticks 1000, 2000, 3000
   - Mix to tick 1500
   - Change to passage B (call `set_current_passage(passage_b_id, 0)`)
   - Mix to tick 3500
   - Verify markers for passage A not emitted after change

5. `test_empty_buffer_handling()`
   - Attempt to mix with empty buffer
   - Verify error handled gracefully (no panic)
   - Verify position tracking unaffected

### Test Suite 11: Performance and Stress Tests

**File:** `wkmp-ap/tests/mixer_tests/test_performance.rs`

**Tests:**
1. `test_large_marker_heap_performance()`
   - Add 1,000 markers at random ticks
   - Measure time to add all markers
   - Mix through all markers, measure total time
   - Verify O(log n) performance characteristics

2. `test_rapid_marker_additions()`
   - Mix in small batches (128 frames)
   - Add 10 new markers after each batch
   - Verify no performance degradation over 10,000 iterations

3. `test_memory_leak_check()`
   - Mix 1,000 passages with markers
   - Check memory usage before and after
   - Verify no significant memory growth (markers cleaned up)

4. `test_concurrent_marker_access()` (if applicable)
   - Simulate concurrent marker additions and mixing
   - Verify thread safety (if mixer used across threads)

---

## Test Execution Plan

### Phase 1: Setup (1 hour)
1. Fix mixer API (BufferManager integration)
2. Create test infrastructure (helpers, mock buffers)
3. Generate test audio files

### Phase 2: Increment 5 - Unit Tests (2-3 hours)
1. Write Test Suites 1-4 (~12-15 tests)
2. Run tests, fix any issues discovered
3. Verify 100% pass rate

### Phase 3: Increment 6 - Integration Tests (3-4 hours)
1. Write Test Suites 5-8 (~10-12 tests)
2. Run tests, validate sample accuracy
3. Fix any timing or audio output issues

### Phase 4: Increment 7 - Accuracy & Stress Tests (2-3 hours)
1. Write Test Suites 9-11 (~10-12 tests)
2. Run performance tests, verify O(log n)
3. Run stress tests, verify no memory leaks

### Phase 5: Documentation (30 min)
1. Document test results
2. Create test report (pass/fail rates, discovered issues)
3. Update implementation_status.md

---

## Success Criteria

**Test Coverage:**
- ✅ 30+ tests covering all mixer functionality
- ✅ Marker system mechanics verified
- ✅ Position tracking sample-accurate
- ✅ Event emission timing correct
- ✅ Crossfade precision validated

**Performance:**
- ✅ O(log n) marker operations confirmed
- ✅ No memory leaks detected
- ✅ Acceptable mixing latency (<10ms for typical batch sizes)

**Correctness:**
- ✅ All events emitted at exact ticks (±1 frame tolerance)
- ✅ No position drift over extended playback
- ✅ Edge cases handled correctly
- ✅ Audio output quality verified

**Ready for Integration:**
- ✅ Architecture proven in isolation
- ✅ No fundamental design flaws discovered
- ✅ API surface validated
- ✅ Confidence high for PlaybackEngine integration

---

## Test Results Template

After execution, document results in `testing_results_increments_5_7.md`:

```markdown
# PLAN014 Testing Results: Increments 5-7

**Date:** [execution_date]
**Status:** [PASS/FAIL/PARTIAL]

## Summary
- Total Tests: X
- Passed: Y
- Failed: Z
- Skipped: W

## Increment 5: Unit Tests
[test suite results]

## Increment 6: Integration Tests
[test suite results]

## Increment 7: Accuracy Tests
[test suite results]

## Issues Discovered
[list of bugs/issues found during testing]

## Architecture Validation
[Pass/Fail for each success criterion]

## Recommendation
[Proceed with integration / Fix issues first / Redesign needed]
```

---

**Report Date:** 2025-01-30
**Status:** Ready to Execute
**Estimated Duration:** 7-10 hours
**Next Action:** Fix mixer API to use BufferManager

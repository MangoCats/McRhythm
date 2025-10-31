# Increment 6: Integration Tests - Execution Plan

**Date:** 2025-01-30
**Status:** In Progress
**Objective:** Validate mixer behavior in realistic playback scenarios (isolated testing)

---

## Scope Adjustment

**Original Plan:** Full integration tests with real audio files and PlaybackEngine
**Adjusted Plan:** Integration-style tests within isolated test environment

**Rationale:**
- Mixer not yet integrated with PlaybackEngine (Sub-Increment 4b pending)
- No audio file loading infrastructure in test environment
- Current BufferManager test infrastructure sufficient for validation
- Focus on realistic playback patterns, not full system integration

**What This Tests:**
- Realistic mixing patterns (longer passages, continuous playback)
- Crossfade behavior with overlapping buffers
- Pause/resume state transitions
- Volume changes during playback
- Extended playback scenarios (stress testing)

**What This Does NOT Test:**
- Actual audio file decoding
- PlaybackEngine integration
- Real-time audio output
- Full end-to-end playback

---

## Test Suites

### Suite 1: Extended Playback Scenarios (4 tests)

**Focus:** Longer passages with realistic marker patterns

**Tests:**
1. `test_long_passage_with_frequent_markers` - 10-second passage, markers every 100ms
2. `test_continuous_playback_multiple_passages` - Sequential playback of 5 passages
3. `test_playback_with_varying_batch_sizes` - Mix with 64, 128, 512, 1024 frame batches
4. `test_passage_completion_detection` - Verify PassageComplete at exact end

### Suite 2: Crossfade Integration (3 tests)

**Focus:** Crossfade with overlapping buffers

**Tests:**
1. `test_crossfade_with_dual_buffers` - Mix two passages simultaneously
2. `test_crossfade_marker_timing` - StartCrossfade event triggers correctly
3. `test_crossfade_passage_transition` - Smooth transition from passage A to B

### Suite 3: State Transitions (4 tests)

**Focus:** Pause/resume and state changes

**Tests:**
1. `test_pause_mid_playback` - Pause during mixing, verify silence
2. `test_resume_after_pause` - Resume with fade-in
3. `test_pause_resume_position_tracking` - Markers work across pause
4. `test_rapid_pause_resume_cycles` - Stress test state transitions

### Suite 4: Volume and Mixing (3 tests)

**Focus:** Master volume and mixing correctness

**Tests:**
1. `test_master_volume_scaling` - Volume affects output amplitude
2. `test_volume_change_during_playback` - Mid-playback volume change
3. `test_volume_with_markers` - Volume doesn't affect marker timing

---

## Implementation Strategy

### Phase 1: Extended Playback Tests (1-1.5 hours)

**Test File:** `test_integration_extended.rs`

**Key Scenarios:**
- 10-second passages (441,000 frames @ 44.1kHz)
- Markers every 100ms (4,410 frames apart)
- Multiple passages in sequence
- Varying batch sizes to test frame boundaries

### Phase 2: Crossfade Tests (1-1.5 hours)

**Test File:** `test_integration_crossfade.rs`

**Key Scenarios:**
- Two buffers active simultaneously
- Crossfade duration: 2 seconds (88,200 frames)
- StartCrossfade marker at calculated point
- Overlapping mix validation

### Phase 3: State Transition Tests (1 hour)

**Test File:** `test_integration_state.rs`

**Key Scenarios:**
- Pause mid-passage
- Resume with fade-in (500ms)
- Position tracking across pause
- Rapid state changes

### Phase 4: Volume Tests (30 minutes)

**Test File:** `test_integration_volume.rs`

**Key Scenarios:**
- Volume scaling (0.0, 0.5, 1.0)
- Mid-playback volume change
- Volume independence from marker timing

---

## Success Criteria

**Must Pass:**
- All 14 integration tests passing
- No panics or crashes
- Position tracking accurate (±1 frame)
- Markers fire at correct ticks
- State transitions clean (no glitches)

**Performance:**
- Mix 10-second passage < 100ms (fast enough for real-time)
- No memory leaks detected
- Consistent timing across varying batch sizes

---

## Test Data Specifications

**Synthetic Audio:**
- Sample rate: 44.1kHz (44,100 samples/second)
- Channel count: 2 (stereo)
- Frame = 2 samples (left + right)
- Test amplitudes: 0.5 (prevents clipping, easy to verify)

**Passage Lengths:**
- Short: 1 second (44,100 frames)
- Medium: 5 seconds (220,500 frames)
- Long: 10 seconds (441,000 frames)

**Marker Patterns:**
- Sparse: Every 1 second (44,100 frames)
- Dense: Every 100ms (4,410 frames)
- Very dense: Every 10ms (441 frames)

---

## Risk Assessment

**Low Risk:**
- Using existing test infrastructure (BufferManager helpers)
- Similar patterns to Increment 5 tests (proven approach)
- No external dependencies (audio files, PlaybackEngine)

**Medium Risk:**
- Crossfade tests more complex (two buffers)
- Extended playback may reveal memory issues
- State transition edge cases

**Mitigation:**
- Start with simpler tests, build complexity gradually
- Use existing helpers where possible
- Add memory usage checks for long playback tests

---

## Estimated Timeline

**Total:** 4-5 hours

| Phase | Duration | Tests |
|-------|----------|-------|
| Extended Playback | 1-1.5 hours | 4 tests |
| Crossfade | 1-1.5 hours | 3 tests |
| State Transitions | 1 hour | 4 tests |
| Volume | 30 minutes | 3 tests |

**Buffer:** 1 hour for debugging and iteration

---

## Dependencies

**Prerequisites:**
- ✅ Increment 5 complete (test infrastructure exists)
- ✅ BufferManager test helpers functional
- ✅ Mixer API stable

**Blockers:** None

---

## Next Steps After Increment 6

**If All Tests Pass:**
- Proceed to Increment 7 (Accuracy Tests)
- High confidence for PlaybackEngine integration

**If Tests Reveal Issues:**
- Fix mixer implementation
- Re-run Increment 5 tests to ensure no regression
- Iterate until all tests pass

---

**Plan Created:** 2025-01-30
**Ready to Execute**

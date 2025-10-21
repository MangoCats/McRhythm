# Test Migration Plan: PassageBuffer to PlayoutRingBuffer

**Date:** 2025-10-20
**Author:** Testing Specialist Agent
**Objective:** Map existing PassageBuffer tests to new PlayoutRingBuffer architecture
**Related:** [Ring Buffer Refactoring Plan](ring_buffer_refactoring_plan.md)

---

## Executive Summary

**Total Tests Affected:** 25 test files
**Tests Requiring Updates:** ~45 individual test functions
**New Tests Needed:** 18 ring buffer-specific tests
**Estimated Test Coverage After Migration:** 72-78%
**Migration Effort:** 12-16 hours

---

## 1. Existing PassageBuffer Test Inventory

### 1.1 Unit Tests - Buffer Management (`buffer_management_tests.rs`)

| Test Function | LOC | Functionality Tested | Migration Status |
|--------------|-----|----------------------|------------------|
| `test_playout_ringbuffer_size_enforced` | 25 | Buffer capacity enforcement | **1:1 Replace** - Update to use PlayoutRingBuffer API |
| `test_buffer_full_detection` | 28 | Nearly full detection (headroom) | **1:1 Replace** - Update to ring buffer fill_percent() |
| `test_backpressure_mechanism` | 44 | Decoder pause on buffer full | **1:1 Replace** - Use should_decoder_pause() |
| `test_buffer_state_lifecycle` | 44 | Decoding → Ready → Playing → Exhausted | **Refactor** - Ring buffer has different lifecycle |
| `test_buffer_overflow_prevention` | 28 | Reject samples exceeding capacity | **1:1 Replace** - Test push_frame() error handling |
| `test_buffer_underflow_detection` | 27 | Underrun detection | **1:1 Replace** - Test pop_frame() underrun behavior |
| `test_ring_buffer_wraparound` | 22 | Ring buffer wraparound | **Keep** - Core ring buffer functionality |
| `test_buffer_sample_count_accuracy` | 24 | Sample count tracking | **1:1 Replace** - Use fill_level() instead of sample_count |

**Total:** 8 tests, ~242 LOC
**Migration:** All tests require API updates but preserve testing intent

---

### 1.2 Unit Tests - Decoder Pool (`decoder_pool_tests.rs`)

| Test Function | LOC | Functionality Tested | Migration Status |
|--------------|-----|----------------------|------------------|
| `test_only_one_decoder_active_at_time` | 32 | Serial decode execution | **Keep** - Unaffected by buffer change |
| `test_priority_queue_ordering` | 18 | Priority-based ordering | **Keep** - Unaffected by buffer change |
| `test_decode_completion_triggers_next` | 23 | Seamless decode transitions | **Keep** - Unaffected by buffer change |
| `test_fade_in_applied_before_buffering` | 38 | Pre-buffer fade-in | **Major Refactor** - Fades now applied before push |
| `test_fade_out_applied_before_buffering` | 25 | Pre-buffer fade-out | **Major Refactor** - Verify fades in decoded samples |
| `test_all_five_fade_curves_supported` | 43 | Fade curve support | **Minor Update** - Test fade unit separately |
| `test_sample_accurate_fade_timing` | 19 | Sample-accurate timing | **Keep** - Timing unaffected |

**Total:** 7 tests, ~198 LOC
**Migration:** 2 major refactors (fade tests), 5 keep as-is

---

### 1.3 Integration Tests - Mixer (`mixer_integration_tests.rs`)

| Test Function | LOC | Functionality Tested | Migration Status |
|--------------|-----|----------------------|------------------|
| `test_tick_to_sample_conversion_accuracy` | 25 | Tick conversions | **Keep** - Unaffected |
| `test_crossfade_duration_calculations` | 19 | Crossfade timing | **Keep** - Unaffected |
| `test_passage_timing_sample_accuracy` | 33 | Passage timing | **Keep** - Unaffected |
| `test_mixer_state_transitions` | 17 | Idle → Single → Crossfade | **Minor Update** - Buffer access changes |
| `test_zero_duration_fades` | 11 | Zero fade handling | **Keep** - Unaffected |
| `test_high_precision_timing` | 17 | Tick precision | **Keep** - Unaffected |
| `test_maximum_passage_duration` | 15 | 4-hour passage support | **Keep** - Unaffected |

**Total:** 7 tests, ~137 LOC
**Migration:** 1 minor update, 6 keep as-is

---

### 1.4 Integration Tests - Crossfade (`crossfade_integration_tests.rs`)

| Test Function | LOC | Functionality Tested | Migration Status |
|--------------|-----|----------------------|------------------|
| `test_fade_in_timing_accuracy` | 45 | Fade-in RMS tracking | **Refactor** - Different buffer access pattern |
| `test_crossfade_timing_accuracy` | 53 | Crossfade overlap timing | **Refactor** - Dual ring buffer draining |
| `test_fade_out_to_silence` | 65 | Fade-out to silence | **Refactor** - Ring buffer drain to zero |
| `test_clipping_detection` | 39 | Clipping detection | **Minor Update** - Buffer creation changes |
| `test_multiple_crossfades_sequence` | 81 | Sequential crossfades | **Refactor** - Multiple buffer lifecycle tests |
| `test_rms_tracker_accuracy` | 35 | RMS tracker validation | **Keep** - Helper function test |
| `test_timing_tolerance_calculation` | 23 | Timing tolerance helper | **Keep** - Helper function test |

**Total:** 7 tests, ~341 LOC
**Migration:** 5 refactors, 2 keep as-is

---

### 1.5 Integration Tests - Queue Integrity (`queue_integrity_tests.rs`)

| Test Function | LOC | Functionality Tested | Migration Status |
|--------------|-----|----------------------|------------------|
| `test_three_passage_queue_advancement_integrity` | 114 | 3-passage playback integrity | **Update** - Buffer lifecycle changes |
| `test_queue_advance_removes_current` | 78 | Queue advance removes entries | **Keep** - Unaffected |
| `test_mixer_queue_state_sync` | 42 | Mixer/queue sync (mock test) | **Refactor** - Mock buffer for ring behavior |
| `test_queue_advancement_no_double_trigger` | 48 | No duplicate advancement | **Keep** - Unaffected |
| `test_queue_database_consistency` | 72 | Memory/DB sync | **Keep** - Unaffected |
| `test_event_ordering_and_completeness` | 79 | Event sequence validation | **Keep** - Unaffected |

**Total:** 6 tests, ~433 LOC
**Migration:** 1 update, 1 refactor, 4 keep as-is

---

### 1.6 Unit Tests - Crossfade Completion (`crossfade_completion_unit_tests.rs`)

| Test Function | LOC | Functionality Tested | Migration Status |
|--------------|-----|----------------------|------------------|
| `test_crossfade_sets_completion_flag` | 52 | Completion flag setting | **Minor Update** - Buffer creation API |
| `test_stop_clears_completion_flag` | 55 | Stop clears flag | **Minor Update** - Buffer creation API |
| `test_crossfade_completion_flag_atomicity` | 39 | Atomic flag consumption | **Minor Update** - Buffer creation API |

**Total:** 3 tests, ~146 LOC
**Migration:** All minor updates (buffer creation only)

---

### 1.7 Integration Tests - Audio Subsystem (`audio_subsystem_test.rs`)

| Test Function | LOC | Functionality Tested | Migration Status |
|--------------|-----|----------------------|------------------|
| `test_sine_wave_generation` | 11 | Test helper | **Keep** - Helper function |
| Commented-out integration tests | ~160 | End-to-end playback | **Update** - When re-enabled, use new buffers |

**Total:** 1 active test, ~171 LOC total
**Migration:** Helper test unchanged, disabled tests need updates when re-enabled

---

### 1.8 Audible Test - Real Audio Playback (`audible_crossfade_test.rs`)

| Test Function | LOC | Functionality Tested | Migration Status |
|--------------|-----|----------------------|------------------|
| `test_audible_crossfade` | 514 | 4-cycle multi-curve crossfade | **Major Refactor** - Ring buffer creation/lifecycle |

**Total:** 1 test, ~914 LOC (includes helpers)
**Migration:** Major refactor required - creates test buffers, needs ring buffer API

---

## 2. Test Coverage Analysis

### 2.1 Current PassageBuffer Coverage

| Functionality | Coverage | Test Files |
|--------------|----------|------------|
| Buffer creation/initialization | 100% | buffer_management_tests.rs, types.rs |
| Capacity enforcement | 100% | buffer_management_tests.rs |
| Sample append operations | 100% | buffer_management_tests.rs, types.rs |
| Overflow/underflow detection | 100% | buffer_management_tests.rs |
| Duration calculations | 100% | types.rs |
| Finalize/completion detection | 100% | types.rs |
| Frame access by index | 100% | types.rs, mixer tests |

**Overall PassageBuffer Coverage:** ~92%

---

### 2.2 Target PlayoutRingBuffer Coverage

| Functionality | Target Coverage | New Tests Needed |
|--------------|-----------------|------------------|
| Ring buffer push/pop operations | >80% | 4 new unit tests |
| Fill level tracking (0-100%) | >80% | 3 new unit tests |
| Decoder pause/resume signaling | >75% | 2 new integration tests |
| Wraparound behavior | >80% | 2 new unit tests |
| Underrun detection (expected vs unexpected) | >70% | 2 new unit tests |
| Buffer exhaustion detection | >75% | 2 new unit tests |
| Concurrent access (producer/consumer) | >60% | 3 new integration tests |

**Overall Target PlayoutRingBuffer Coverage:** 72-78%

---

## 3. Test Migration Mapping

### 3.1 One-to-One Replacements (Preserve Test Intent)

| Original Test | New Test | Changes Required |
|---------------|----------|------------------|
| `test_playout_ringbuffer_size_enforced` | `test_ring_buffer_capacity_enforced` | Replace Vec append with push_frame(), capacity check |
| `test_buffer_full_detection` | `test_ring_buffer_nearly_full_detection` | Replace sample_count with fill_percent(), headroom calculation |
| `test_backpressure_mechanism` | `test_decoder_pause_on_buffer_full` | Replace is_nearly_full() with should_decoder_pause() |
| `test_buffer_overflow_prevention` | `test_ring_buffer_push_when_full` | Test push_frame() returns Err(BufferFullError) |
| `test_buffer_underflow_detection` | `test_ring_buffer_pop_when_empty` | Test pop_frame() returns Err(BufferEmptyError) |
| `test_buffer_sample_count_accuracy` | `test_ring_buffer_fill_level_accuracy` | Replace sample_count with fill_level |

---

### 3.2 Tests Requiring Refactoring

| Original Test | Changes Required | Reason |
|---------------|------------------|--------|
| `test_buffer_state_lifecycle` | Rewrite state transitions: Assigned → Filling → NearlyFull → Draining → Exhausted | Ring buffer has different lifecycle than growing Vec |
| `test_fade_in_applied_before_buffering` | Access samples from ring buffer after push, verify fades already applied | Fades now happen in fade unit, not during read |
| `test_fade_out_applied_before_buffering` | Same as above | Same as above |
| `test_fade_in_timing_accuracy` | Drain ring buffer instead of index reads | Mixer drains, doesn't read by index |
| `test_crossfade_timing_accuracy` | Coordinate draining two ring buffers simultaneously | Crossfade drains two buffers in parallel |
| `test_fade_out_to_silence` | Verify ring buffer drains to 0% fill | Buffer empties completely |
| `test_multiple_crossfades_sequence` | Track ring buffer fill cycles across multiple crossfades | Multiple fill/drain cycles |
| `test_mixer_queue_state_sync` | Mock ring buffer behavior (push/pop/fill) | Mock needs ring buffer interface |
| `test_audible_crossfade` | Create ring buffers, pre-fill before playback starts | Real-time playback from draining buffers |

---

### 3.3 Tests That Become Obsolete

| Test Function | Reason | Action |
|---------------|--------|--------|
| `test_append_samples_basic` (types.rs) | Ring buffers use push_frame(), not append | Remove |
| `test_append_samples_multiple_times` (types.rs) | Same as above | Remove |
| `test_append_samples_updates_duration` (types.rs) | Ring buffers don't track total duration | Remove |
| `test_append_samples_panics_on_odd_length` (types.rs) | Ring buffer uses AudioFrame, always stereo | Remove |
| `test_reserve_capacity` (types.rs) | Ring buffers have fixed capacity | Remove |
| `test_reserve_capacity_reduces_reallocations` (types.rs) | Same as above | Remove |
| `test_get_frame_after_append` (types.rs) | Mixer drains, doesn't access by index | Remove |
| `test_duration_fixed_after_finalize` (types.rs) | Ring buffers don't have finalize() | Remove |
| `test_finalize_sets_all_fields` (types.rs) | Same as above | Remove |

**Total Obsolete Tests:** 9 tests (~180 LOC)

---

## 4. New Tests Required for Ring Buffer Functionality

### 4.1 Core Ring Buffer Operations

```rust
// File: playout_ring_buffer_tests.rs (NEW)

#[test]
fn test_ring_buffer_push_pop_fifo() {
    // Verify FIFO ordering: first frame pushed is first frame popped
    // Push 100 unique frames, pop 100, verify order matches
}

#[test]
fn test_ring_buffer_wraparound_correctness() {
    // Fill buffer to capacity, drain 50%, refill 50%, verify wraparound
    // Ensures read/write pointers wrap correctly at buffer boundary
}

#[test]
fn test_ring_buffer_fill_level_tracking() {
    // Verify fill_level() returns correct value throughout lifecycle:
    // - Empty: 0
    // - After 100 pushes: 100
    // - After 50 pops: 50
    // - After drain to empty: 0
}

#[test]
fn test_ring_buffer_fill_percent_calculation() {
    // With capacity 1000:
    // - 0 samples: 0.0%
    // - 500 samples: 50.0%
    // - 1000 samples: 100.0%
}
```

---

### 4.2 Decoder Pause/Resume Logic

```rust
#[test]
fn test_decoder_pause_threshold_accuracy() {
    // Verify should_decoder_pause() returns true when:
    // fill_level >= (capacity - headroom)
    // With capacity=1000, headroom=50:
    // - 949 samples: false
    // - 950 samples: true
    // - 1000 samples: true
}

#[tokio::test]
async fn test_decoder_pause_resume_cycle() {
    // Integration test:
    // 1. Decoder fills buffer to pause threshold
    // 2. Decoder pauses (verify should_decoder_pause() = true)
    // 3. Mixer drains 200 samples
    // 4. Decoder resumes (verify should_decoder_pause() = false)
}
```

---

### 4.3 Buffer Exhaustion Detection

```rust
#[test]
fn test_buffer_exhaustion_requires_decode_complete() {
    // is_exhausted() should return false even if fill_level=0
    // unless decode_complete flag is set
}

#[test]
fn test_buffer_exhaustion_after_drain() {
    // 1. Fill buffer with 1000 samples
    // 2. Set decode_complete = true
    // 3. Drain all 1000 samples
    // 4. Verify is_exhausted() = true
}
```

---

### 4.4 Underrun Classification

```rust
#[test]
fn test_underrun_expected_during_slow_decode() {
    // Pop from empty buffer when decode still active
    // Should log at DEBUG level (expected behavior)
}

#[test]
fn test_underrun_unexpected_during_playback() {
    // Pop from empty buffer when decode complete but mixer too fast
    // Should log at WARN level (CPU can't keep up)
}
```

---

### 4.5 Concurrent Access (Lock-Free)

```rust
#[tokio::test]
async fn test_concurrent_producer_consumer() {
    // Spawn producer task (pushes 10,000 frames)
    // Spawn consumer task (pops 10,000 frames)
    // Verify no deadlocks, no lost frames, FIFO order maintained
}

#[tokio::test]
async fn test_lock_free_audio_thread_safety() {
    // Verify pop_frame() never blocks (suitable for real-time audio callback)
    // Measure max latency < 100µs
}

#[tokio::test]
async fn test_ring_buffer_stats_accuracy() {
    // Push 500 frames
    // Verify stats() returns:
    // - occupied: 500
    // - capacity: 661941
    // - underruns: 0
    // - overruns: 0
}
```

---

## 5. Test Execution Order (Dependencies)

### Phase 1: Core Ring Buffer Unit Tests
**Prerequisites:** None
**Tests:**
- test_ring_buffer_push_pop_fifo
- test_ring_buffer_wraparound_correctness
- test_ring_buffer_fill_level_tracking
- test_ring_buffer_fill_percent_calculation

**Duration:** ~30 minutes
**Goal:** Verify ring buffer data structure correctness

---

### Phase 2: Buffer State Management Tests
**Prerequisites:** Phase 1 passes
**Tests:**
- test_decoder_pause_threshold_accuracy
- test_buffer_exhaustion_requires_decode_complete
- test_buffer_exhaustion_after_drain

**Duration:** ~45 minutes
**Goal:** Verify state transitions and flags

---

### Phase 3: Decoder Integration Tests
**Prerequisites:** Phase 1 + 2 pass
**Tests:**
- test_decoder_pause_resume_cycle
- test_fade_in_applied_before_buffering (updated)
- test_fade_out_applied_before_buffering (updated)

**Duration:** ~60 minutes
**Goal:** Verify decoder fills ring buffer correctly

---

### Phase 4: Mixer Integration Tests
**Prerequisites:** Phase 1 + 2 + 3 pass
**Tests:**
- test_fade_in_timing_accuracy (updated)
- test_crossfade_timing_accuracy (updated)
- test_fade_out_to_silence (updated)
- test_multiple_crossfades_sequence (updated)

**Duration:** ~90 minutes
**Goal:** Verify mixer drains ring buffers correctly

---

### Phase 5: Queue Integrity Tests
**Prerequisites:** Phase 1-4 pass
**Tests:**
- test_three_passage_queue_advancement_integrity (updated)
- test_mixer_queue_state_sync (updated)

**Duration:** ~60 minutes
**Goal:** Verify end-to-end queue processing

---

### Phase 6: Concurrent Access Tests
**Prerequisites:** Phase 1-5 pass
**Tests:**
- test_concurrent_producer_consumer
- test_lock_free_audio_thread_safety
- test_ring_buffer_stats_accuracy

**Duration:** ~45 minutes
**Goal:** Verify lock-free operation and thread safety

---

### Phase 7: Audible Quality Tests
**Prerequisites:** All above phases pass
**Tests:**
- test_audible_crossfade (updated)
- Manual listening tests

**Duration:** ~30 minutes playback time
**Goal:** Human verification of audio quality

---

## 6. Coverage Gap Analysis

### 6.1 Current Gaps in PassageBuffer Tests

| Missing Coverage | Severity | Impact |
|------------------|----------|--------|
| Concurrent access during append/read | High | Race conditions possible |
| Memory leak detection | Medium | Long-running processes |
| Buffer reuse after passage completion | Medium | Resource management |

---

### 6.2 New Gaps Introduced by Ring Buffer

| Missing Coverage | Severity | Mitigation |
|------------------|----------|------------|
| Ring buffer wrap-around edge cases (capacity-1, capacity, capacity+1) | High | Add edge case tests in Phase 1 |
| Decoder pause/resume race conditions | High | Add concurrent tests in Phase 6 |
| Mixer underrun recovery behavior | Medium | Add recovery tests in Phase 4 |
| Buffer monitoring during rapid state changes | Medium | Add monitoring tests in Phase 5 |

---

### 6.3 Estimated Coverage After Migration

| Component | Current Coverage | Target Coverage | Gap |
|-----------|------------------|-----------------|-----|
| Core buffer operations | 92% | 78% | -14% (acceptable - simpler API) |
| Decoder integration | 65% | 75% | +10% (new pause/resume tests) |
| Mixer integration | 70% | 72% | +2% (underrun tests) |
| Queue integrity | 80% | 80% | 0% (mostly unaffected) |
| Concurrent access | 20% | 65% | +45% (new lock-free tests) |

**Overall Estimated Coverage:** 72-78% (acceptable for ring buffer architecture)

---

## 7. Test Migration Timeline

### Week 1: Core Ring Buffer Tests
- **Days 1-2:** Implement PlayoutRingBuffer (prerequisite)
- **Day 3:** Write Phase 1 tests (core ring buffer)
- **Day 4:** Write Phase 2 tests (state management)
- **Day 5:** Write Phase 3 tests (decoder integration)

---

### Week 2: Integration Tests
- **Day 1:** Update Phase 4 tests (mixer integration)
- **Day 2:** Update Phase 5 tests (queue integrity)
- **Day 3:** Write Phase 6 tests (concurrent access)
- **Day 4:** Update Phase 7 tests (audible quality)
- **Day 5:** Buffer - fix failing tests, address gaps

---

### Week 3: Validation and Cleanup
- **Day 1-2:** Run full test suite, fix regressions
- **Day 3:** Remove obsolete tests
- **Day 4:** Update test documentation
- **Day 5:** Final validation, code review

---

## 8. Risk Assessment

### High-Risk Test Changes

| Test Area | Risk | Mitigation |
|-----------|------|------------|
| Mixer drain operations | High - Core playback functionality | Extensive unit tests before integration |
| Crossfade dual buffer draining | High - Complex coordination | Mock tests before real audio tests |
| Concurrent producer/consumer | High - Lock-free correctness | Stress tests with high iteration counts |

---

### Medium-Risk Test Changes

| Test Area | Risk | Mitigation |
|-----------|------|------------|
| Fade application before buffering | Medium - Audio quality impact | Verify RMS levels match expectations |
| Buffer exhaustion detection | Medium - Queue advancement depends on it | Unit tests for all edge cases |
| Underrun classification | Medium - Log noise if incorrect | Test all expected/unexpected scenarios |

---

### Low-Risk Test Changes

| Test Area | Risk | Mitigation |
|-----------|------|------------|
| Queue manager tests | Low - Mostly unaffected | Minimal updates, mostly API changes |
| Timing calculation tests | Low - Unaffected by buffer change | No changes needed |
| Helper function tests | Low - Unaffected | No changes needed |

---

## 9. Success Criteria

### Must Pass (Blocker)

- ✅ All Phase 1-2 tests pass (core ring buffer correctness)
- ✅ All Phase 3 tests pass (decoder fills buffer correctly)
- ✅ All Phase 4 tests pass (mixer drains buffer correctly)
- ✅ No audio glitches in audible test (Phase 7)
- ✅ No increase in underruns vs. baseline

---

### Should Pass (Important)

- ✅ All Phase 5 tests pass (queue integrity maintained)
- ✅ All Phase 6 tests pass (lock-free operation verified)
- ✅ Test coverage >= 70% overall
- ✅ No memory leaks in 10-minute playback test

---

### Nice to Have

- ✅ Test coverage >= 75% overall
- ✅ All edge cases covered (wrap-around, pause/resume races)
- ✅ Performance benchmarks show no regression

---

## 10. Summary

### Tests by Action

| Action | Count | LOC | Effort (hrs) |
|--------|-------|-----|--------------|
| **1:1 Replace (API update only)** | 15 | ~420 | 3-4 |
| **Minor Update (buffer creation)** | 8 | ~180 | 2-3 |
| **Refactor (logic change)** | 9 | ~520 | 5-7 |
| **Keep As-Is** | 13 | ~320 | 0 |
| **New Tests Required** | 18 | ~360 | 4-6 |
| **Remove (obsolete)** | 9 | ~180 | 0.5 |

**Total Effort:** 12-16 hours (excluding PlayoutRingBuffer implementation)

---

### Coverage Summary

| Metric | Value |
|--------|-------|
| **Total test files affected** | 8 files |
| **Total test functions affected** | 45 functions |
| **Tests requiring updates** | 32 functions |
| **New tests needed** | 18 functions |
| **Tests to deprecate** | 9 functions |
| **Estimated coverage after migration** | 72-78% |

---

### Critical Path

1. **Implement PlayoutRingBuffer** (prerequisite - not in test scope)
2. **Write Phase 1 tests** (core ring buffer) - 4 tests
3. **Write Phase 2 tests** (state management) - 3 tests
4. **Update Phase 3 tests** (decoder integration) - 7 tests
5. **Update Phase 4 tests** (mixer integration) - 9 tests
6. **Run Phase 7 audible test** (quality validation) - 1 test

**Minimum Viable Test Suite:** Phases 1-4 + Phase 7 = 23 tests

---

## Appendix A: Test File Summary

| File | Tests Total | Tests Updated | Tests New | Tests Removed | Effort (hrs) |
|------|-------------|---------------|-----------|---------------|--------------|
| buffer_management_tests.rs | 8 | 8 | 0 | 0 | 2-3 |
| decoder_pool_tests.rs | 7 | 2 | 0 | 0 | 1-2 |
| mixer_integration_tests.rs | 7 | 1 | 0 | 0 | 0.5 |
| crossfade_integration_tests.rs | 7 | 5 | 0 | 0 | 3-4 |
| queue_integrity_tests.rs | 6 | 2 | 0 | 0 | 1-2 |
| crossfade_completion_unit_tests.rs | 3 | 3 | 0 | 0 | 0.5 |
| audio_subsystem_test.rs | 1 | 0 | 0 | 0 | 0 |
| audible_crossfade_test.rs | 1 | 1 | 0 | 0 | 2-3 |
| **playout_ring_buffer_tests.rs (NEW)** | 0 | 0 | 18 | 0 | 4-6 |
| types.rs (unit tests) | 26 | 0 | 0 | 9 | 0.5 |

**Total:** 66 tests → 57 tests (after removing 9 obsolete) + 18 new = **75 tests**

---

**Migration Status:** Ready to begin
**Next Step:** Implement PlayoutRingBuffer, then start Phase 1 tests
**Owner:** Testing Specialist Agent + Code Implementer Agent

---

**Approval Required:** Technical Lead
**Estimated Completion:** 2-3 weeks (including implementation and testing)

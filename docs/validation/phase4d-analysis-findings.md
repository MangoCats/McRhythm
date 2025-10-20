# Phase 4D Analysis: Mixer and Crossfade Integration

**Date:** 2025-10-19
**Analyst:** Claude (Phase 4D Implementation Agent)
**Status:** Analysis Complete - Implementation Plan Ready

---

## Executive Summary

Phase 4D requires integrating the mixer with tick-based timing, event-driven buffer management, and crossfade overlap handling. Analysis reveals that:

1. **Current mixer implementation is ~80% complete** - State machine, dual-buffer mixing, and pause/resume already implemented
2. **Primary gap: Timing system migration** - System currently uses milliseconds, needs conversion to ticks
3. **Secondary gap: API changes** - Mixer interface needs to accept samples instead of milliseconds
4. **Tertiary gap: Test coverage** - Need 5 critical integration tests

---

## Current State Analysis

### What's Already Implemented ✅

1. **MixerState Enum** (mixer.rs:145-187)
   - ✅ None, SinglePassage, Crossfading states
   - ✅ State machine transitions implemented
   - ✅ Pause/Resume states (pause_state, resume_state)

2. **Dual-Buffer Mixing** (mixer.rs:393-460)
   - ✅ Crossfading state reads from 2 buffers simultaneously
   - ✅ Applies fade curves to each buffer
   - ✅ Mixes samples: `mixed.add(&next_frame)`
   - **Conclusion:** Crossfade overlap IS implemented (Phase 1 analysis was incorrect)

3. **Event System Integration** (mixer.rs:221-239)
   - ✅ set_event_channel() for position events
   - ✅ Emits PositionUpdate events (REV002)
   - ✅ Configurable event interval

4. **Buffer Manager Integration** (mixer.rs:211-213)
   - ✅ set_buffer_manager() method
   - ✅ Underrun detection using buffer manager
   - ⚠️  Does NOT call buffer_manager.start_playback()
   - ⚠️  Does NOT listen for BufferEvent::ReadyForStart (done in engine.rs)

5. **Pause/Resume** (mixer.rs:583-649)
   - ✅ Exponential decay on pause
   - ✅ Configurable fade-in on resume
   - ✅ Linear and exponential curves supported

### What's Missing ❌

1. **Tick-Based Timing** - CRITICAL
   - ❌ PassageWithTiming uses `start_time_ms: u64` (should be `start_time_ticks: i64`)
   - ❌ Mixer API accepts `fade_in_duration_ms: u32` (should be `fade_in_duration_samples: usize`)
   - ❌ Internal `ms_to_samples()` conversion (should use `ticks_to_samples()`)

2. **Sample-Based API** - HIGH
   - ❌ `start_passage(..., fade_in_duration_ms: u32)`
   - ❌ `start_crossfade(..., fade_out_duration_ms: u32, fade_in_duration_ms: u32)`
   - Should accept samples directly: `fade_in_duration_samples: usize`

3. **Test Coverage** - HIGH
   - ❌ No tests for dual-buffer crossfade overlap
   - ❌ No tests for tick-to-sample conversion
   - ❌ No tests for event-driven playback start
   - ❌ No integration tests with BufferManager

---

## Required Changes

### Change 1: Update PassageWithTiming Struct

**File:** `wkmp-ap/src/db/passages.rs`

**Before:**
```rust
pub struct PassageWithTiming {
    pub start_time_ms: u64,
    pub end_time_ms: Option<u64>,
    pub fade_in_point_ms: u64,
    pub fade_out_point_ms: Option<u64>,
    pub lead_in_point_ms: u64,
    pub lead_out_point_ms: Option<u64>,
    // ...
}
```

**After:**
```rust
pub struct PassageWithTiming {
    pub start_time_ticks: i64,
    pub end_time_ticks: Option<i64>,
    pub fade_in_point_ticks: i64,
    pub fade_out_point_ticks: Option<i64>,
    pub lead_in_point_ticks: i64,
    pub lead_out_point_ticks: Option<i64>,
    // ...
}
```

**Impact:** Breaking change to all passage loading code

---

### Change 2: Update Mixer API

**File:** `wkmp-ap/src/playback/pipeline/mixer.rs`

**Before:**
```rust
pub async fn start_passage(
    &mut self,
    buffer: Arc<RwLock<PassageBuffer>>,
    passage_id: Uuid,
    fade_in_curve: Option<FadeCurve>,
    fade_in_duration_ms: u32,  // ❌ Milliseconds
)
```

**After:**
```rust
pub async fn start_passage(
    &mut self,
    buffer: Arc<RwLock<PassageBuffer>>,
    passage_id: Uuid,
    fade_in_curve: Option<FadeCurve>,
    fade_in_duration_samples: usize,  // ✅ Samples
)
```

**Similarly for `start_crossfade()`:**
- Change `fade_out_duration_ms: u32` → `fade_out_duration_samples: usize`
- Change `fade_in_duration_ms: u32` → `fade_in_duration_samples: usize`

**Remove:**
- `fn ms_to_samples(&self, ms: u32) -> usize` (line 757)
- Replace with direct sample counts

---

### Change 3: Update Engine Mixer Calls

**File:** `wkmp-ap/src/playback/engine.rs`

**Before (line ~1295-1320):**
```rust
let fade_in_duration_ms = passage.fade_in_point_ms
    .saturating_sub(passage.start_time_ms) as u32;

self.mixer.write().await.start_passage(
    buffer,
    current.queue_entry_id,
    Some(fade_in_curve),
    fade_in_duration_ms,  // ❌ Milliseconds
).await;
```

**After:**
```rust
use wkmp_common::timing::ticks_to_samples;

let fade_in_duration_ticks = passage.fade_in_point_ticks
    .saturating_sub(passage.start_time_ticks);
let fade_in_duration_samples = ticks_to_samples(
    fade_in_duration_ticks,
    44100  // STANDARD_SAMPLE_RATE
);

self.mixer.write().await.start_passage(
    buffer,
    current.queue_entry_id,
    Some(fade_in_curve),
    fade_in_duration_samples,  // ✅ Samples
).await;
```

**Locations to update:**
1. Line ~1295-1320: start_passage in process_queue
2. Line ~1775-1795: start_passage in buffer_event_handler
3. Line ~1138-1162: start_crossfade in try_trigger_crossfade

---

### Change 4: Update Database Schema/Queries

**Files:**
- `wkmp-ap/src/db/passages.rs` - Update queries
- `migrations/*.sql` - Add migration to convert ms→ticks

**Database Migration Required:**
```sql
-- Convert existing millisecond values to ticks
-- 1 ms = 28,224 ticks

UPDATE passages SET
    start_time = start_time * 28224,
    end_time = end_time * 28224,
    fade_in_point = fade_in_point * 28224,
    fade_out_point = fade_out_point * 28224,
    lead_in_point = lead_in_point * 28224,
    lead_out_point = lead_out_point * 28224;

-- Update column types to INTEGER (from REAL)
-- (Already INTEGER in most cases, verify schema)
```

---

## Implementation Plan

### Phase 1: Database Migration (1-2 hours)
1. Create migration file to convert ms→ticks
2. Update PassageWithTiming struct to use `*_ticks: i64`
3. Update all database queries in passages.rs
4. Verify database tests still pass

### Phase 2: Mixer API Changes (2-3 hours)
1. Change mixer.start_passage() to accept samples
2. Change mixer.start_crossfade() to accept samples
3. Remove ms_to_samples() function
4. Update all internal fade duration handling
5. Update mixer tests

### Phase 3: Engine Integration (3-4 hours)
1. Add timing module imports to engine.rs
2. Update start_passage calls (2 locations)
3. Update start_crossfade call (1 location)
4. Convert tick durations to samples using ticks_to_samples()
5. Verify crossfade calculations use ticks

### Phase 4: Testing (4-5 hours)
1. Write test_mixer_single_passage_playback
2. Write test_crossfade_overlap_detection
3. Write test_dual_buffer_mixing
4. Write test_event_driven_playback_start
5. Write test_pause_exponential_decay
6. Run full test suite, verify no regressions

### Phase 5: Documentation (1-2 hours)
1. Create phase4d-mixer-implementation.md
2. Create phase4d-implementation-log.json
3. Create phase4d-test-results.md
4. Update architecture diagrams if needed

---

## Estimated Effort

**Total Time:** 11-16 hours (1.5-2 days)

**Breakdown:**
- Database migration: 1-2 hours
- Mixer API changes: 2-3 hours
- Engine integration: 3-4 hours
- Testing: 4-5 hours
- Documentation: 1-2 hours

---

## Risk Assessment

### Low Risk ✅
- Mixer state machine already complete
- Dual-buffer mixing already working
- Event system already integrated

### Medium Risk ⚠️
- API changes require updating many call sites
- Database migration could affect existing data
- Timing conversion errors could cause playback issues

### Mitigation Strategies
1. **Comprehensive testing** - Write tests before changing code
2. **Incremental changes** - Change one subsystem at a time
3. **Data backup** - Test migration on copy first
4. **Type safety** - Rust type system will catch API mismatches

---

## Success Criteria

Phase 4D is complete when:

1. ✅ PassageWithTiming uses i64 ticks (not u64 milliseconds)
2. ✅ Mixer API accepts duration_samples (not duration_ms)
3. ✅ Engine converts ticks→samples before calling mixer
4. ✅ All 5 critical tests passing:
   - test_mixer_single_passage_playback
   - test_crossfade_overlap_detection
   - test_dual_buffer_mixing
   - test_event_driven_playback_start
   - test_pause_exponential_decay
5. ✅ BufferManager integration verified (start_playback called)
6. ✅ Documentation complete (3 files)
7. ✅ No regressions in existing tests

---

## Files to Modify

**Total: ~12 files**

1. `wkmp-ap/src/db/passages.rs` - PassageWithTiming struct + queries
2. `wkmp-ap/src/playback/pipeline/mixer.rs` - API changes
3. `wkmp-ap/src/playback/engine.rs` - Mixer call updates (3 locations)
4. `wkmp-ap/tests/mixer_integration_tests.rs` - NEW: 5 integration tests
5. `migrations/NNNN_convert_ms_to_ticks.sql` - NEW: Database migration
6. `docs/validation/phase4d-mixer-implementation.md` - NEW: Report
7. `docs/validation/phase4d-implementation-log.json` - NEW: Change log
8. `docs/validation/phase4d-test-results.md` - NEW: Test results

**Minor updates:**
9. `wkmp-ap/src/db/queue.rs` - Queue entry timing conversions
10. `wkmp-ap/src/playback/queue_manager.rs` - Queue timing handling
11. `wkmp-ap/src/api/playback.rs` - API endpoint ms→ticks conversions
12. `wkmp-common/src/lib.rs` - Export timing module (already done)

---

## Compilation Errors Fixed

During analysis, fixed blocking error in engine.rs:

**File:** `wkmp-ap/src/playback/engine.rs:1817-1822`
**Issue:** Non-exhaustive match in buffer_event_handler
**Fix:** Added match arms for StateChanged, Exhausted, Finished events

---

## Next Steps

1. **Review this analysis** with technical lead
2. **Approve implementation plan** and timeline
3. **Begin Phase 1** (Database migration)
4. **Proceed sequentially** through Phases 2-5
5. **Submit for Phase 4 completion review**

---

## References

- **Mission Brief:** Phase 4D directive (provided)
- **SPEC016:** Decoder Buffer Design (`docs/SPEC016-decoder_buffer_design.md`)
- **Test Spec:** IMPL-TESTS-001 (`docs/validation/IMPL-TESTS-001-unit-test-specs.md`)
- **Timing Module:** `wkmp-common/src/timing.rs` (implemented in Phase 3C)
- **Buffer Events:** `wkmp-ap/src/playback/buffer_events.rs` (Phase 4C)

---

**Analysis Complete**
**Status:** Ready for Implementation
**Blocking Issues:** None
**Dependencies Met:** Phases 4A-4C complete

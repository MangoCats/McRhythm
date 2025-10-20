# Feature De-Prioritization: Seek Within Current Passage

**Date:** 2025-10-19
**Status:** DEFERRED INDEFINITELY
**Priority:** LOW (was: MEDIUM)
**Decision:** Remove from active development scope

---

## Decision Summary

The ability for users to seek to arbitrary positions within a currently playing passage has been **deferred indefinitely** due to low value-to-effort ratio.

**Rationale:**
- **Low user value:** Users primarily care about smooth crossfade transitions between passages
- **Complex implementation:** Seek requires:
  - Decoder rewind/fast-forward logic
  - Buffer synchronization
  - Crossfade state preservation
  - UI controls and feedback
- **Better alternatives:** Users can skip to next passage or replay from start

**Future evaluation:** May be removed entirely if user research confirms low demand.

---

## Implementation Impact

### Features Removed from Scope

1. **User-initiated seek** within current passage
   - No seek bar/slider in UI
   - No keyboard shortcuts for seek (e.g., arrow keys)
   - No API endpoint for `/playback/seek` or `/playback/position`

2. **Programmatic seek** (internal use only)
   - Decoder seek-to-position preserved for decode-and-skip (different use case)
   - Sample-accurate start position preserved (required for passage playback)

### Features Preserved

✅ **Decode-and-skip** - Decoder seeking to passage start_time (CRITICAL for performance)
✅ **Skip to next passage** - User can advance to next passage in queue
✅ **Replay passage** - User can restart current passage from beginning
✅ **Position reporting** - Current playback position still reported for UI display

---

## Code Changes

### No Changes Required

The following components are **unaffected** by this de-prioritization:

- ✅ SerialDecoder seek-to-start_time (Phase 4A) - **PRESERVED** (decode-and-skip optimization)
- ✅ Mixer sample-accurate positioning (Phase 4D) - **PRESERVED** (required for crossfade)
- ✅ Tick-based timing system (Phase 3C) - **PRESERVED** (core functionality)
- ✅ Buffer management (Phase 4C) - **PRESERVED** (core functionality)

### Components Deferred

The following components are **not implemented** and **not planned**:

- ⏸️ User seek controls in UI (was: never planned)
- ⏸️ `/playback/seek` API endpoint (was: never implemented)
- ⏸️ Seek-during-crossfade logic (was: never planned)
- ⏸️ Seek performance optimization (was: not needed)

---

## Test Impact

### Tests Removed from Scope

The following tests are **not required**:

- ❌ `test_user_seek_within_passage` - Not needed
- ❌ `test_seek_during_crossfade` - Not needed
- ❌ `test_seek_to_fade_boundary` - Not needed
- ❌ `test_seek_performance` - Not needed

### Tests Preserved

The following tests are **still required**:

- ✅ `test_decode_and_skip_with_seek_tables` - Required for Phase 4A (decode-and-skip)
- ✅ `test_sample_accurate_start_position` - Required for Phase 4D (mixer)
- ✅ `test_position_reporting` - Required for UI progress display

---

## Requirements Impact

### Requirements Deferred

The following requirements are **deferred indefinitely**:

- **[SEEK-010]** User-initiated seek within passage - **DEFERRED**
- **[SEEK-020]** Seek during crossfade preservation - **DEFERRED**
- **[SEEK-030]** Seek performance (<100ms response) - **DEFERRED**

### Requirements Preserved

The following requirements remain **ACTIVE**:

- ✅ **[DBD-DEC-060]** Decode-and-skip to passage start_time - **ACTIVE** (different use case)
- ✅ **[DBD-MIX-010]** Sample-accurate playback position - **ACTIVE** (required for crossfade)
- ✅ **[POS-010]** Position reporting for UI display - **ACTIVE** (read-only)

---

## API Impact

### Endpoints Deferred

The following API endpoints are **not implemented**:

- ❌ `POST /playback/seek` - Seek to position in current passage - **DEFERRED**
- ❌ `PUT /playback/position` - Set playback position - **DEFERRED**

### Endpoints Preserved

The following API endpoints remain **ACTIVE**:

- ✅ `GET /playback/position` - Get current position (read-only) - **ACTIVE**
- ✅ `POST /playback/skip` - Skip to next passage - **ACTIVE**
- ✅ `POST /playback/restart` - Restart current passage - **ACTIVE**

---

## Documentation Updates

### Documents Updated

1. **REQ001-requirements.md** - Mark seek requirements as DEFERRED
2. **SPEC016-decoder_buffer_design.md** - Note: seek-to-start preserved for decode-and-skip
3. **SPEC007-api_design.md** - Remove seek API endpoints from spec
4. **Implementation plans** - Remove seek-related tasks

### Documents Unaffected

- SPEC017-sample_rate_conversion.md - No seek-related content
- Phase 4A-4D implementation reports - Decode-and-skip is different use case

---

## Terminology Clarification

**"Seek" has two meanings in WKMP:**

1. **Decode-and-skip seek** (ACTIVE, CRITICAL)
   - Decoder jumps to passage start_time in audio file
   - Performance optimization: decode passage region only (not entire file)
   - User-invisible: happens during decode initialization
   - **Status:** ✅ IMPLEMENTED (Phase 4A)

2. **User-initiated seek** (DEFERRED, LOW PRIORITY)
   - User manually seeks to arbitrary position within current passage
   - UI feature: seek bar, keyboard shortcuts
   - Requires decoder rewind/fast-forward logic
   - **Status:** ⏸️ DEFERRED INDEFINITELY

**This de-prioritization affects ONLY meaning #2.** Meaning #1 is preserved and critical.

---

## Migration Path (If Re-Prioritized)

If user research shows demand for seek functionality, implementation would require:

**Phase 1: API Design (1 day)**
- Define seek endpoint schema
- Define seek-during-crossfade behavior
- Design error handling (seek past end, seek during transition)

**Phase 2: Decoder Integration (2-3 days)**
- Implement decoder seek-to-sample()
- Handle codec limitations (not all formats support arbitrary seek)
- Test seek accuracy and performance

**Phase 3: Buffer Synchronization (2-3 days)**
- Clear buffer on seek
- Re-decode from new position
- Preserve crossfade state if mid-transition

**Phase 4: UI Integration (3-4 days)**
- Seek bar component
- Keyboard shortcuts
- Progress indicator
- Seek feedback (loading state)

**Total Effort:** 8-11 days (if re-prioritized)

---

## Conclusion

**Decision:** Seek within current passage is **DEFERRED INDEFINITELY**.

**Impact:** Minimal - decode-and-skip functionality preserved, user seek removed from scope.

**Benefit:** Simplifies implementation, reduces testing burden, allows focus on core playback and crossfade quality.

**Review Date:** TBD (based on user feedback)

---

**Approved By:** User (2025-10-19)
**Documented By:** Claude Code Implementation Agent
**Status:** ✅ ACTIVE - Feature officially deferred

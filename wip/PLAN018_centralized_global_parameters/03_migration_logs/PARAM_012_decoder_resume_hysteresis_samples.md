# Migration Log: decoder_resume_hysteresis_samples (DBD-PARAM-085)

**Parameter:** decoder_resume_hysteresis_samples
**DBD-PARAM Tag:** DBD-PARAM-085
**Default Value:** 44100
**Type:** u64
**Tier:** 3 (High-risk - decoder pause/resume timing-critical)

---

## Executive Summary

**Migration Pattern:** Same as PARAM_010/011 - database-driven parameter with 3 production code replacements.

**Value Meaning:** 44100 samples = 1.0 second @ 44.1kHz (hysteresis gap between pause and resume)

---

## Migration Steps

### Pre-Migration Test
**Result:** ✅ ALL TESTS PASSED

### Production Code Replacements (3 total)

1. **buffer_manager.rs:85** - BufferManager::new() default
   - After: Read from `PARAMS.decoder_resume_hysteresis_samples` (with `as usize` cast)

2. **playout_ring_buffer.rs:57-59** - Added default_resume_hysteresis() function
   - Pattern: Same as default_capacity() and default_headroom()

3. **playout_ring_buffer.rs:211** - Usage
   - Before: `resume_hysteresis.unwrap_or(44100)`
   - After: `resume_hysteresis.unwrap_or_else(default_resume_hysteresis)`

4. **settings.rs:293** - Database load fallback
   - After: Read default from PARAMS before `load_clamped_setting`

### Post-Migration Test
**Result:** ✅ ALL TESTS PASSED

---

## Parameter Semantics

**Value:** 44100 samples = 1.0 second @ 44.1kHz

**Purpose:** Hysteresis gap prevents decoder pause/resume oscillation
- **Pause decoder:** When `free_space ≤ 4410` (headroom threshold)
- **Resume decoder:** When `free_space ≥ 44100 + 4410 = 48510` (hysteresis + headroom)
- **Hysteresis gap:** 44100 samples (1.0 second)

**Valid Range:** 882-88200 samples (0.02-2.0 seconds @ 44.1kHz)

**Why 44100 (1.0s)?**
- Large enough gap (44100 samples) prevents rapid pause/resume cycling
- Small enough to avoid excessive buffer fill delays
- Matches 1 second of audio @ 44.1kHz (easy to reason about)

**Architecture:** Database-driven parameter
- DB load @ [core.rs:201](wkmp-ap/src/playback/engine/core.rs#L201)
- Set via `set_resume_hysteresis` @ [core.rs:244](wkmp-ap/src/playback/engine/core.rs#L244)

---

## Type Note

**Parameter type is u64** (not usize like capacity/headroom) to match database settings table convention for sample counts. Cast to usize where needed in production code.

---

## Commit

```
Migrate decoder_resume_hysteresis_samples to GlobalParams (PARAM 12/15)

Replace 3 hardcoded instances of 44100 (decoder resume hysteresis)
with centralized GlobalParams.decoder_resume_hysteresis_samples.

CHANGES:
- wkmp-ap/src/playback/buffer_manager.rs: Replace hardcoded default
- wkmp-ap/src/playback/playout_ring_buffer.rs: Add default_resume_hysteresis()
  function, update unwrap_or usage
- wkmp-ap/src/db/settings.rs: Update get_decoder_resume_hysteresis

VALUE MEANING:
- 44100 samples = 1.0 second @ 44.1kHz
- Hysteresis gap prevents pause/resume oscillation
- Resume threshold = hysteresis (44100) + headroom (4410) = 48510 samples

[PLAN018] [DBD-PARAM-085]
```

---

**Status:** ✅ MIGRATION COMPLETE (DATABASE-DRIVEN PARAMETER)
**Date:** 2025-11-02
**Next Parameter:** mixer_min_start_level (DBD-PARAM-088, default: 22050)

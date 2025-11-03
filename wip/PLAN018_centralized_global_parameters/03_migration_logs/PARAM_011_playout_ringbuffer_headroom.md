# Migration Log: playout_ringbuffer_headroom (DBD-PARAM-080)

**Parameter:** playout_ringbuffer_headroom
**DBD-PARAM Tag:** DBD-PARAM-080
**Default Value:** 4410
**Type:** usize
**Tier:** 3 (High-risk - buffer pause/resume timing-critical)

---

## Executive Summary

**Migration Pattern:** Identical to PARAM_010 (playout_ringbuffer_size) - database-driven parameter with 4 production code replacements.

**Value Meaning:** 4410 stereo samples = 0.1 seconds @ 44.1kHz (pause threshold for decoder)

---

## Migration Steps

### Pre-Migration Test
**Test Command:** `cargo test -p wkmp-common params::tests -p wkmp-ap --lib`
**Result:** ✅ ALL TESTS PASSED

### Production Code Replacements (4 total)

1. **buffer_manager.rs:87** - BufferManager::new() default
   - Before: `buffer_headroom: Arc::new(RwLock::new(4_410))`
   - After: Read from `PARAMS.playout_ringbuffer_headroom`

2. **playout_ring_buffer.rs:53** - DEFAULT_HEADROOM const
   - Before: `const DEFAULT_HEADROOM: usize = 4410;`
   - After: `fn default_headroom() -> usize { *PARAMS.playout_ringbuffer_headroom.read().unwrap() }`

3. **playout_ring_buffer.rs:204** - Usage of constant
   - Before: `headroom.unwrap_or(DEFAULT_HEADROOM)`
   - After: `headroom.unwrap_or_else(default_headroom)`

4. **settings.rs:515** - Database load fallback
   - Before: `load_clamped_setting(..., 4_410)`
   - After: `load_clamped_setting(..., default)` where default = PARAMS read

### Post-Migration Test
**Result:** ✅ ALL TESTS PASSED

---

## Parameter Semantics

**Value:** 4410 stereo samples = 100ms @ 44.1kHz

**Purpose:** Pause threshold for decoder when buffer nearly full
- **Pause decoder:** When `free_space ≤ 4410 samples`
- **Resume decoder:** When `free_space ≥ decoder_resume_hysteresis + 4410` (48510 samples)

**Valid Range:** 1000-44100 samples (0.023-1.0 seconds @ 44.1kHz)

**Why 4410 (0.1s)?**
- Small enough: Allows buffer to fill nearly to capacity (maximize pre-buffering)
- Large enough: Provides safety margin for late resampler samples
- Hysteresis gap: 44100 samples (1.0s) prevents pause/resume oscillation

**Architecture:** Database-driven parameter (same pattern as PARAM_010)
- DB load @ [core.rs:205](wkmp-ap/src/playback/engine/core.rs#L205)
- Set via `set_buffer_headroom` @ [core.rs:248](wkmp-ap/src/playback/engine/core.rs#L248)
- GlobalParams provides fallback for tests

---

## Commit

```
Migrate playout_ringbuffer_headroom to GlobalParams (PARAM 11/15)

Replace 4 hardcoded instances of 4410 (buffer headroom threshold)
with centralized GlobalParams.playout_ringbuffer_headroom.

CHANGES:
- wkmp-ap/src/playback/buffer_manager.rs: Replace hardcoded default
- wkmp-ap/src/playback/playout_ring_buffer.rs: Convert DEFAULT_HEADROOM
  const to default_headroom() function
- wkmp-ap/src/db/settings.rs: Update load_playout_ringbuffer_headroom

VALUE MEANING:
- 4410 stereo samples = 0.1 seconds @ 44.1kHz
- Pause threshold: free_space ≤ 4410 (buffer nearly full)
- Resume threshold: free_space ≥ 48510 (pause + 1s hysteresis)

[PLAN018] [DBD-PARAM-080]
```

---

**Status:** ✅ MIGRATION COMPLETE (DATABASE-DRIVEN PARAMETER)
**Date:** 2025-11-02
**Next Parameter:** decoder_resume_hysteresis_samples (DBD-PARAM-085, default: 44100)

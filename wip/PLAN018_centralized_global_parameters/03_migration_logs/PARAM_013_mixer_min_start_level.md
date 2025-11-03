# Migration Log: mixer_min_start_level (DBD-PARAM-088)

**Parameter:** mixer_min_start_level
**DBD-PARAM Tag:** DBD-PARAM-088
**Default Value:** 22050
**Type:** usize
**Tier:** 3 (High-risk - mixer startup timing-critical)

---

## Executive Summary

**Parameter Status:** RESERVED FOR FUTURE USE (loaded but not actively used in production)

**Migration:** Single database load function default replacement

**BUG FIX:** Database load function had WRONG default (44100), corrected to SPEC016 value (22050)

---

## Migration Steps

### Pre-Migration Test
**Result:** ✅ ALL TESTS PASSED

### Production Code Replacements (1 total)

**settings.rs:483** - Database load fallback
- Before: `load_clamped_setting(..., 44100usize)` ← **WRONG DEFAULT**
- After: `load_clamped_setting(..., default)` where default = PARAMS.mixer_min_start_level (22050)

**Bug Fixed:** Database load function was using 44100 (1.0s) instead of 22050 (0.5s) from SPEC016

### Post-Migration Test
**Result:** ✅ ALL TESTS PASSED

---

## Parameter Semantics

**Value:** 22050 samples = 0.5 seconds @ 44.1kHz

**Purpose:** Minimum buffer level before mixer starts playback (reserved for future implementation)

**Valid Range:** 8820-220500 samples (0.2-5.0 seconds @ 44.1kHz)

**Why 22050 (0.5s)?**
- Fast enough startup (0.5s delay acceptable)
- Large enough to ensure smooth playback start
- Smaller than chunk_duration_ms (1000ms) - allows start within first decoded chunk

**Current Status:**
- DB load @ [core.rs:202](wkmp-ap/src/playback/engine/core.rs#L202)
- Stored in `_mixer_min_start_level` (underscore-prefixed = unused)
- Reserved for future mixer start logic

**Future Implementation:** When implemented, mixer will wait until buffer has ≥22050 samples before starting audio output.

---

## Commit

```
Migrate mixer_min_start_level to GlobalParams (PARAM 13/15)

Fix incorrect default in database load function (44100 → 22050).

CHANGES:
- wkmp-ap/src/db/settings.rs: Update load_mixer_min_start_level
  to use PARAMS default (22050)

BUG FIX: Database load function had wrong default
- Before: 44100 samples (1.0s @ 44.1kHz)
- After: 22050 samples (0.5s @ 44.1kHz) ← SPEC016 correct value

PARAMETER STATUS: Reserved for future use
- Loaded @ core.rs:202 but not actively used
- Will control minimum buffer before mixer starts playback

[PLAN018] [DBD-PARAM-088]
```

---

**Status:** ✅ MIGRATION COMPLETE (RESERVED PARAMETER + BUG FIX)
**Date:** 2025-11-02
**Next Parameter:** audio_buffer_size (DBD-PARAM-110, default: 2208)

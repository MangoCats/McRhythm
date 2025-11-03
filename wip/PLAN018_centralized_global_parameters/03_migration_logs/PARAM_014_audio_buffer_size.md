# Migration Log: audio_buffer_size (DBD-PARAM-110)

**Parameter:** audio_buffer_size
**DBD-PARAM Tag:** DBD-PARAM-110
**Default Value:** 2208
**Type:** u32
**Tier:** 3 (High-risk - audio callback buffer sizing timing-critical)

---

## Executive Summary

**Parameter Status:** ACTIVELY USED (database-driven, stored in Engine struct)

**Migration:** Single database load function default replacement

---

## Migration Steps

### Pre-Migration Test
**Result:** ✅ ALL TESTS PASSED

### Production Code Replacements (1 total)

**settings.rs:498** - Database load fallback
- After: Read default from `PARAMS.audio_buffer_size` (2208)

### Post-Migration Test
**Result:** ✅ ALL TESTS PASSED

---

## Parameter Semantics

**Value:** 2208 frames = 50.1ms @ 44.1kHz

**Purpose:** Audio output buffer size (frames per cpal callback)

**Valid Range:** 64-65536 frames

**Why 2208 frames (50.1ms)?**
- Empirically tuned for VeryHigh stability confidence
- Balance between latency and CPU efficiency
- 50ms is standard for audio applications (human perception threshold ~10ms)

**Architecture:** Database-driven parameter with database initialization
- DB init @ [init.rs:39](wkmp-ap/src/db/init.rs#L39) - `("audio_buffer_size", "2208")`
- DB load @ [core.rs:203](wkmp-ap/src/playback/engine/core.rs#L203)
- Stored in Engine struct @ [core.rs:166](wkmp-ap/src/playback/engine/core.rs#L166)
- Used for audio output creation @ [core.rs:731](wkmp-ap/src/playback/engine/core.rs#L731)

**Tuning System:** Auto-tuning system may adjust based on system performance
- See `wkmp-ap/src/tuning/` for buffer auto-tuning logic
- Smaller = lower latency, higher CPU load
- Larger = higher latency, more stable on slow systems

---

## Commit

```
Migrate audio_buffer_size to GlobalParams (PARAM 14/15)

Replace hardcoded default in database load function.

CHANGES:
- wkmp-ap/src/db/settings.rs: Update load_audio_buffer_size
  to use PARAMS default (2208 frames = 50.1ms @ 44.1kHz)

PARAMETER STATUS: Actively used in production
- Database initialized @ init.rs:39
- Loaded @ core.rs:203
- Stored in Engine struct @ core.rs:166
- Used for audio output buffer sizing @ core.rs:731

VALUE MEANING:
- 2208 frames = 50.1ms @ 44.1kHz
- Empirically tuned for VeryHigh stability confidence
- Balance between latency (50ms) and stability

[PLAN018] [DBD-PARAM-110]
```

---

**Status:** ✅ MIGRATION COMPLETE (DATABASE-DRIVEN PARAMETER - ACTIVE)
**Date:** 2025-11-02
**Next Parameter:** mixer_check_interval_ms (DBD-PARAM-111, default: 10) - FINAL PARAMETER

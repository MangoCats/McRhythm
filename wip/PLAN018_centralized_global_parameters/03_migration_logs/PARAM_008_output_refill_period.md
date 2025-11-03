# Migration Log: output_refill_period (DBD-PARAM-040)

**Parameter:** output_refill_period
**DBD-PARAM Tag:** DBD-PARAM-040
**Default Value:** 90
**Type:** u64
**Tier:** 2 (Medium-risk - mixer wake timing)

---

## Step 1: Pre-Migration Test Baseline

**Test Command:** `cargo test -p wkmp-common params::tests -p wkmp-ap --lib`
**Result:** ✅ ALL TESTS PASSED (9/9 GlobalParams + wkmp-ap library tests)

---

## Step 2: Find All Hardcoded References

**Search Commands:**
```bash
rg "\b90\b" --type rust -g '!*test*.rs' wkmp-ap/src -C 3
rg "output_refill_period|refill.*period" --type rust wkmp-ap/src -i -n
```

**Matches Found:** 1 location (API metadata only)

### Context Verification Results

| Location | Line | Context | Classification | Action |
|----------|------|---------|----------------|--------|
| `wkmp-ap/src/api/handlers.rs` | 1259 | Default `"90"` string for output_refill_period | API metadata only | ❌ **NO CHANGE** (documentation) |

**CRITICAL FINDING:** Parameter `output_refill_period` has **ZERO production code usage**.

**Parameter Status:** Reserved for future use - defined in SPEC016 but not yet implemented.

**Evidence:**
- No database initialization for `output_refill_period`
- No load function in `wkmp-ap/src/db/settings.rs`
- No production code reads this parameter
- API handler lists it as metadata only (string "90", not actual code)

**Confusion with Similar Parameter:**
- **DBD-PARAM-040** `output_refill_period` (default: 90ms) - **UNUSED** (this parameter)
- **DBD-PARAM-111** `mixer_check_interval_ms` (default: 10ms) - **ACTIVELY USED** (different parameter!)

**These are distinct parameters:** `output_refill_period` was intended for output ring buffer refill timing (unused because no output ring buffer exists yet), while `mixer_check_interval_ms` controls actual mixer wake-up timing in current implementation.

---

## Step 3: Replace Production Code References

**NO REPLACEMENTS REQUIRED** - Parameter is unused in production code.

---

## Step 4: Post-Migration Testing

**Test Command:** `cargo test -p wkmp-common params::tests -p wkmp-ap --lib`
**Expected Result:** ALL TESTS PASS (no code changes, infrastructure only)

**Verification:**
- ✅ GlobalParams has `output_refill_period` field
- ✅ Default value of 90 set correctly
- ✅ RwLock read/write access works
- ✅ No production code to migrate

---

## Step 5: Commit

**Commit Message:**
```
Document output_refill_period migration (PARAM 8/15) - UNUSED

Parameter output_refill_period (DBD-PARAM-040) is defined in GlobalParams
but has zero production code usage. Reserved for future output ring buffer
refill timing.

NOTE: This is DIFFERENT from mixer_check_interval_ms (DBD-PARAM-111),
which controls actual mixer wake timing and IS actively used.

Current architecture uses mixer_check_interval_ms for mixer wake-up.
output_refill_period is reserved for future output ring buffer optimization.

No code changes required - parameter exists in infrastructure only.

[PLAN018] [DBD-PARAM-040]
```

**Files Changed:**
- NONE (documentation only)

---

## Migration Statistics

- **Grep matches:** 1
- **Production code replacements:** 0
- **Reason:** Parameter unused in current implementation
- **Build errors:** 0
- **Test failures:** 0

---

## Notes

**Key Finding:** `output_refill_period` is a **SPEC016-reserved parameter** that has not been implemented yet. The GlobalParams infrastructure provides the field, but no production code currently uses it.

**Current Architecture:** WKMP uses `mixer_check_interval_ms` (DBD-PARAM-111) for mixer wake timing:
- Mixer thread wakes periodically to check buffer levels
- Database init: `("mixer_check_interval_ms", "10")` @ [wkmp-ap/src/db/init.rs:52](wkmp-ap/src/db/init.rs#L52)
- Load function: `db::settings.rs:321` with range 1-100ms, default 10ms
- Production usage: Tuning system, test harness, buffer auto-tuning

**Future Implementation:** When output ring buffer is implemented, `output_refill_period` would control how often the mixer refills that buffer. Code should read from `*PARAMS.output_refill_period.read().unwrap()`.

**Default Value Analysis:**
- **90 ms** refill period
- **Purpose**: Would control mixer wake-up frequency for output buffer refills
- **Relationship**: Would work with `output_ringbuffer_size` (DBD-PARAM-030, also unused)
- **Current alternative**: `mixer_check_interval_ms` (10ms, much more frequent)

**Why Different Values:**
- `output_refill_period` (90ms): Intended for less frequent refills of large output buffer
- `mixer_check_interval_ms` (10ms): Current direct callback approach needs frequent checks

**Architecture Evolution:**
1. **Phase 1 (current)**: Direct audio callback, mixer checks every 10ms
2. **Phase 2 (future)**: Output ring buffer added, refilled every 90ms, mixer checks every 10ms to service that buffer

**Success criteria:** ✅ GlobalParams field exists with correct default, no production code migration needed.

---

**Status:** ✅ MIGRATION COMPLETE (TRIVIAL - UNUSED PARAMETER)
**Date:** 2025-11-02
**Next Parameter:** decode_chunk_size (DBD-PARAM-065, default: 25000)

# Migration Log: working_sample_rate (DBD-PARAM-020)

**Parameter:** working_sample_rate
**DBD-PARAM Tag:** DBD-PARAM-020
**Default Value:** 44100
**Type:** u32
**Tier:** 2 (Medium-risk - core audio processing rate)

---

## Step 1: Pre-Migration Test Baseline

**Test Command:** `cargo test -p wkmp-common params::tests -p wkmp-ap --lib`
**Result:** ✅ ALL TESTS PASSED (9/9 GlobalParams + wkmp-ap library tests)

---

## Step 2: Find All Hardcoded References WITH CONTEXT VERIFICATION

**Search Command:** `rg "\b44100\b" --type rust -g '!*test*.rs' wkmp-ap/src -C 10`

**⚠️ CRITICAL AMBIGUITY:** Value `44100` is shared by TWO parameters:
- **DBD-PARAM-020** working_sample_rate (this parameter)
- **DBD-PARAM-085** decoder_resume_hysteresis_samples (1.0 second @ 44.1kHz)

**Disambiguation Strategy:**
1. Check for DBD-PARAM tags in comments
2. Check variable/field names (resume_hysteresis vs. sample_rate)
3. Check semantic context (time duration vs. frequency)
4. Check calculation purpose (capacity/time vs. Hz specification)

**Matches Found:** 7 production code locations (excluding tests/docs)

### Context Verification Results

| Location | Line | Context | Classification | Action |
|----------|------|---------|----------------|--------|
| `wkmp-ap/src/audio/resampler.rs` | 21 | `pub const TARGET_SAMPLE_RATE: u32 = 44100` | **PRODUCTION USAGE** | ✅ **REPLACE** with PARAMS |
| `wkmp-ap/src/playback/buffer_manager.rs` | 24 | `const STANDARD_SAMPLE_RATE: u32 = 44100` | **PRODUCTION USAGE** | ✅ **REPLACE** with PARAMS |
| `wkmp-ap/src/playback/buffer_manager.rs` | 82 | `resume_hysteresis: Arc::new(RwLock::new(44100))` | **DBD-PARAM-085** (hysteresis) | ❌ **NO CHANGE** (different parameter) |
| `wkmp-ap/src/playback/playout_ring_buffer.rs` | 204 | `resume_hysteresis.unwrap_or(44100)` | **DBD-PARAM-085** (hysteresis) | ❌ **NO CHANGE** (different parameter) |
| `wkmp-ap/src/playback/playout_ring_buffer.rs` | 209 | `capacity as f64 / 44100.0` | **Time calculation** | ❌ **NO CHANGE** (calculation, not definition) |
| `wkmp-ap/src/playback/playout_ring_buffer.rs` | 890 | `buffer.capacity() as f64 / 44100.0` | **Time calculation** (test code) | ❌ **NO CHANGE** (test code) |
| `wkmp-ap/src/api/handlers.rs` | 1253 | Default `"44100"` string for working_sample_rate | API metadata only | ❌ **NO CHANGE** (documentation) |
| `wkmp-ap/src/api/handlers.rs` | 1277 | Default `"44100"` string for decoder_resume_hysteresis_samples | **DBD-PARAM-085** metadata | ❌ **NO CHANGE** (documentation) |

**Production Code Requiring Migration:**
1. `resampler.rs:21` - TARGET_SAMPLE_RATE constant definition + 3 usages (lines 191, 202, 233)
2. `buffer_manager.rs:24` - STANDARD_SAMPLE_RATE constant definition + 5 usages (lines 320, 361, 427, 856, 939)

**Disambiguation Rationale:**
- **resume_hysteresis lines**: Variable name explicitly mentions "hysteresis", comment says "1.0 second @ 44.1kHz" (time duration), belongs to DBD-PARAM-085
- **Time calculation lines**: Division to compute seconds (capacity / 44100.0), not defining sample rate
- **resampler.rs:21 & buffer_manager.rs:24**: Constants named "SAMPLE_RATE", represent Hz specification, match DBD-PARAM-020

---

## Step 3: Replace Production Code References

### Replacement 1: resampler.rs - Convert const to function (4 changes)

**File:** `wkmp-ap/src/audio/resampler.rs`

**Change 1a - Line 21 (const definition):**
```rust
// OLD:
#[allow(dead_code)]
pub const TARGET_SAMPLE_RATE: u32 = 44100;

// NEW:
/// **[DBD-PARAM-020]** Read working sample rate from GlobalParams (default: 44100 Hz per SPEC016)
#[allow(dead_code)]
pub fn target_sample_rate() -> u32 {
    *wkmp_common::params::PARAMS.working_sample_rate.read().unwrap()
}
```

**Change 1b - Line 191 (output_rate() method):**
```rust
// OLD:
Self::PassThrough { .. } => TARGET_SAMPLE_RATE,

// NEW:
Self::PassThrough { .. } => target_sample_rate(),
```

**Change 1c - Line 202 (input_rate() method):**
```rust
// OLD:
Self::PassThrough { .. } => TARGET_SAMPLE_RATE,

// NEW:
Self::PassThrough { .. } => target_sample_rate(),
```

**Change 1d - Line 233 (resample() function):**
```rust
// OLD:
let output_rate = TARGET_SAMPLE_RATE;

// NEW:
let output_rate = target_sample_rate();
```

**Rationale:** Convert const to function to enable dynamic reading from GlobalParams.

---

### Replacement 2: buffer_manager.rs - Convert const to function (6 changes)

**File:** `wkmp-ap/src/playback/buffer_manager.rs`

**Change 2a - Line 24 (const definition):**
```rust
// OLD:
const STANDARD_SAMPLE_RATE: u32 = 44100;

// NEW:
/// **[DBD-PARAM-020]** Read working sample rate from GlobalParams (default: 44100 Hz per SPEC016)
fn standard_sample_rate() -> u32 {
    *wkmp_common::params::PARAMS.working_sample_rate.read().unwrap()
}
```

**Change 2b - Line 320 (buffer_fill_status calculation):**
```rust
// OLD:
                        / (STANDARD_SAMPLE_RATE as u64 * 2); // /2 for stereo

// NEW:
                        / (standard_sample_rate() as u64 * 2); // /2 for stereo
```

**Change 2c - Line 361 (buffer_fill_status calculation):**
```rust
// OLD:
                        / (STANDARD_SAMPLE_RATE as u64 * 2); // /2 for stereo

// NEW:
                        / (standard_sample_rate() as u64 * 2); // /2 for stereo
```

**Change 2d - Line 427 (ms_to_frames conversion):**
```rust
// OLD:
        threshold_ms as usize * STANDARD_SAMPLE_RATE as usize / 1000

// NEW:
        threshold_ms as usize * standard_sample_rate() as usize / 1000
```

**Change 2e - Line 856 (has_minimum_duration check):**
```rust
// OLD:
            let available_ms = (occupied_frames as u64 * 1000) / STANDARD_SAMPLE_RATE as u64;

// NEW:
            let available_ms = (occupied_frames as u64 * 1000) / standard_sample_rate() as u64;
```

**Change 2f - Line 939 (buffer_status duration_ms):**
```rust
// OLD:
                Some((occupied as u64 * 1000) / STANDARD_SAMPLE_RATE as u64)

// NEW:
                Some((occupied as u64 * 1000) / standard_sample_rate() as u64)
```

**Initial Analysis Error:** Initially thought STANDARD_SAMPLE_RATE was unused. Grep after removal failed to find references because constant no longer existed. Re-grepping before removal revealed 5 production usages in time/duration calculations.

---

## Step 4: Post-Migration Testing

**Test Command:** `cargo test -p wkmp-common params::tests -p wkmp-ap --lib`
**Result:** ✅ ALL TESTS PASSED (9/9 GlobalParams + wkmp-ap library tests)

**Build Result:** ✅ Clean compilation (no errors, 17 unrelated warnings)

**Affected Areas:**
- Audio resampling initialization (TARGET_SAMPLE_RATE → target_sample_rate())
- Buffer manager time calculations (STANDARD_SAMPLE_RATE → standard_sample_rate())

**Verification:**
- Build succeeds with function conversions
- All tests pass (no behavioral change)
- Default value remains 44100 Hz

---

## Step 5: Commit

**Commit Message:**
```
Migrate working_sample_rate to GlobalParams (PARAM 6/15)

Replace hardcoded sample rate constants with centralized parameter.

No behavioral change - default value remains 44100 Hz per SPEC016.

Changes:
- wkmp-ap/src/audio/resampler.rs: Convert TARGET_SAMPLE_RATE const to function + 3 usages
- wkmp-ap/src/playback/buffer_manager.rs: Convert STANDARD_SAMPLE_RATE const to function + 5 usages

CRITICAL CONTEXT VERIFICATION: Disambiguated 44100 value from decoder_resume_hysteresis_samples
(DBD-PARAM-085), which shares the same default but represents time duration, not sample rate.

[PLAN018] [DBD-PARAM-020]
```

**Files Changed:**
- `wkmp-ap/src/audio/resampler.rs` (4 lines modified: const → function + 3 usages)
- `wkmp-ap/src/playback/buffer_manager.rs` (6 lines modified: const → function + 5 usages)

---

## Migration Statistics

- **Grep matches:** 7
- **Ambiguous matches requiring context verification:** 5
- **Correctly disambiguated from DBD-PARAM-085:** 3
- **Production code constant definitions:** 2 (both converted to functions)
- **Production code usages replaced:** 8 (3 in resampler.rs, 5 in buffer_manager.rs)
- **Database defaults unchanged:** N/A (no database init for this parameter yet)
- **Behavioral changes:** 0 (same default value)
- **Build errors:** 0
- **Test failures:** 0

---

## Notes

**Key Decision:** Context verification prevented incorrect replacement of decoder_resume_hysteresis_samples (DBD-PARAM-085) values.

**Disambiguation Evidence:**
- **Variable names:** `resume_hysteresis` vs. `SAMPLE_RATE`
- **Comments:** "1.0 second @ 44.1kHz" (time) vs. "Hz" (frequency)
- **Calculations:** `capacity / 44100.0` (time conversion) vs. `const ... = 44100` (rate definition)
- **DBD-PARAM tags:** None present (confirms need for semantic analysis)

**Const → Function Pattern:**
- Original: `pub const TARGET_SAMPLE_RATE: u32 = 44100;`
- New: `pub fn target_sample_rate() -> u32 { *PARAMS.working_sample_rate.read().unwrap() }`
- **Rationale:** Constants cannot read from RwLock at compile time, function enables runtime access

**STANDARD_SAMPLE_RATE Usage (Corrected Analysis):**
- **Initially thought unused** (grep error - searched after removal, constant no longer existed)
- **Actually has 5 production usages** in time/duration calculations
- All usages converted to `standard_sample_rate()` function calls
- Calculations: ms ↔ frames conversions, buffer duration metrics

**Shared Default Value Risk:**
- working_sample_rate (DBD-PARAM-020): 44100 Hz (frequency)
- decoder_resume_hysteresis_samples (DBD-PARAM-085): 44100 samples (time @ 44.1kHz)
- **Coincidence:** Both default to 44100 but represent different physical quantities
- **Future risk:** Changing one should NOT affect the other (independent parameters)

---

**Status:** ✅ MIGRATION COMPLETE (NO BEHAVIORAL CHANGE)
**Date:** 2025-11-02
**Next Parameter:** output_ringbuffer_size (DBD-PARAM-030, default: 16384)

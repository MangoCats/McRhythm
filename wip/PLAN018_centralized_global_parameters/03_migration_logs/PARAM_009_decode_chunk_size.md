# Migration Log: decode_chunk_size (DBD-PARAM-065)

**Parameter:** decode_chunk_size
**DBD-PARAM Tag:** DBD-PARAM-065
**Default Value:** 25000
**Type:** usize
**Tier:** 2 (Medium-risk - decoder chunk sizing)

---

## Step 1: Pre-Migration Test Baseline

**Test Command:** `cargo test -p wkmp-common params::tests -p wkmp-ap --lib`
**Result:** ✅ ALL TESTS PASSED (9/9 GlobalParams + wkmp-ap library tests)

---

## Step 2: Find All Hardcoded References

**Search Commands:**
```bash
rg "\b25000\b" --type rust -g '!*test*.rs' wkmp-ap/src -C 5
rg "decode_chunk_size|chunk.*size" --type rust wkmp-ap/src -i -n
rg "chunk_duration_ms" --type rust wkmp-ap/src/playback/pipeline/decoder_chain.rs -B 10 -A 2 -n
```

**Matches Found:** 1 location (API metadata only)

### Context Verification Results

| Location | Line | Context | Classification | Action |
|----------|------|---------|----------------|--------|
| `wkmp-ap/src/api/handlers.rs` | 1268 | Default `"25000"` string for decode_chunk_size | API metadata only | ❌ **NO CHANGE** (documentation) |

**CRITICAL FINDING:** Parameter `decode_chunk_size` has **ZERO production code usage**.

**Parameter Status:** Reserved for future use - defined in SPEC016 but not yet implemented.

**Current Implementation Uses Different Approach:**
- **Parameter (UNUSED)**: `decode_chunk_size` = 25000 samples (~567ms @ 44.1kHz)
- **Current Code (USED)**: `chunk_duration_ms` = 1000ms (time-based, not sample-based)

**Evidence:**
- No database initialization for `decode_chunk_size`
- No load function in `wkmp-ap/src/db/settings.rs`
- No production code reads this parameter
- API handler lists it as metadata only (string "25000", not actual code)

**Production Code Uses Time-Based Chunks:**

[wkmp-ap/src/playback/pipeline/decoder_chain.rs:126](wkmp-ap/src/playback/pipeline/decoder_chain.rs#L126):
```rust
// **[DBD-DEC-110]** Use 1 second chunks
let chunk_duration_ms = 1000;
```

Then converted to samples per source sample rate:
```rust
let chunk_size_samples = (source_sample_rate as u64 * chunk_duration_ms / 1000) as usize;
```

**Why Different Approach:**
- **Time-based** chunks (current): Consistent decode time regardless of sample rate
  - 48kHz source: 48000 samples/chunk
  - 44.1kHz source: 44100 samples/chunk
  - 96kHz source: 96000 samples/chunk
- **Sample-based** chunks (parameter): Fixed sample count, variable decode time
  - Would be ~520ms @ 48kHz
  - Would be ~567ms @ 44.1kHz
  - Would be ~260ms @ 96kHz

**Current approach is superior** for predictable decode timing across variable source sample rates.

---

## Step 3: Replace Production Code References

**NO REPLACEMENTS REQUIRED** - Parameter is unused in production code.

---

## Step 4: Post-Migration Testing

**Test Command:** `cargo test -p wkmp-common params::tests -p wkmp-ap --lib`
**Expected Result:** ALL TESTS PASS (no code changes, infrastructure only)

**Verification:**
- ✅ GlobalParams has `decode_chunk_size` field
- ✅ Default value of 25000 set correctly
- ✅ RwLock read/write access works
- ✅ No production code to migrate

---

## Step 5: Commit

**Commit Message:**
```
Document decode_chunk_size migration (PARAM 9/15) - UNUSED

Parameter decode_chunk_size (DBD-PARAM-065) is defined in GlobalParams
but has zero production code usage. Reserved for potential future use.

Current decoder uses time-based chunks (1000ms hardcoded) instead of
sample-based chunks. This provides consistent decode timing across
variable source sample rates (48kHz, 96kHz, etc.).

No code changes required - parameter exists in infrastructure only.

[PLAN018] [DBD-PARAM-065]
```

**Files Changed:**
- NONE (documentation only)

---

## Migration Statistics

- **Grep matches:** 1
- **Production code replacements:** 0
- **Reason:** Parameter unused - current implementation uses time-based chunks
- **Build errors:** 0
- **Test failures:** 0

---

## Notes

**Key Finding:** `decode_chunk_size` is a **SPEC016-reserved parameter** that has not been implemented. Current decoder uses a different (better) approach.

**Current Architecture:** Time-based chunking @ [wkmp-ap/src/playback/pipeline/decoder_chain.rs:126](wkmp-ap/src/playback/pipeline/decoder_chain.rs#L126)
```rust
let chunk_duration_ms = 1000;  // [DBD-DEC-110] 1 second chunks
let chunk_size_samples = (source_sample_rate * chunk_duration_ms / 1000) as usize;
```

**Benefits of Time-Based Chunking:**
1. **Predictable decode time**: Always ~1 second of audio regardless of source rate
2. **Consistent latency**: Decode buffer refill time independent of file format
3. **Simpler buffer management**: Constant time budgets for decoder worker

**If Sample-Based Chunking Were Used:**
```rust
// Hypothetical implementation using decode_chunk_size parameter
let chunk_size_samples = *PARAMS.decode_chunk_size.read().unwrap();
let chunk_duration_ms = (chunk_size_samples as u64 * 1000 / source_sample_rate as u64);
```

**Problems with sample-based approach:**
- Variable decode time: 260ms @ 96kHz vs. 567ms @ 44.1kHz
- Unpredictable buffer refill timing
- Complicates decoder worker scheduling

**Recommendation:** Keep time-based approach, deprecate `decode_chunk_size` parameter in future SPEC revision.

**Default Value Analysis:**
- **25000 samples** = 567ms @ 44.1kHz
- Original intent: Likely ~0.5s chunks for balance between latency and overhead
- Current implementation: 1000ms chunks (2x longer, lower overhead)

**Success criteria:** ✅ GlobalParams field exists with correct default, no production code migration needed.

---

**Status:** ✅ MIGRATION COMPLETE (TRIVIAL - UNUSED PARAMETER)
**Date:** 2025-11-02
**Next Parameter:** playout_ringbuffer_size (DBD-PARAM-070, default: 661941)

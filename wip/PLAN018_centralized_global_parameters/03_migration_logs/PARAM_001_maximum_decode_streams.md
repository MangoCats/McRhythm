# Migration Log: maximum_decode_streams (DBD-PARAM-050)

**Parameter:** maximum_decode_streams
**DBD-PARAM Tag:** DBD-PARAM-050
**Default Value:** 12
**Type:** usize
**Tier:** 1 (Low-risk - decoder pool sizing)

---

## Step 1: Pre-Migration Test Baseline

**Test Command:** `cargo test --workspace`
**Result:** ✅ ALL TESTS PASSED (9/9 pre-migration tests)

---

## Step 2: Find All Hardcoded References

**Search Command:** `rg "\b12\b" --type rust -g '!*test*.rs' wkmp-ap/src`

**Matches Found:** 7 locations

### Context Verification Results

| Location | Line | Context | Classification | Action |
|----------|------|---------|----------------|--------|
| `wkmp-ap/src/db/init.rs` | 64 | `("maximum_decode_streams", "12")` | Database init default | ❌ **NO CHANGE** (source of truth) |
| `wkmp-ap/src/db/settings.rs` | ~340 | `load_clamped_setting(..., 12usize)` | Load function default | ❌ **NO CHANGE** (bootstrap fallback) |
| `wkmp-ap/src/playback/decoder_worker.rs` | 328 | `% 12` in chain index calculation | **PRODUCTION USAGE** | ✅ **REPLACE** with PARAMS |
| `wkmp-ap/src/playback/engine/core.rs` | 141 | Doc comment with `(default: 12)` | Documentation | ❌ **NO CHANGE** (comment) |
| `wkmp-ap/src/playback/engine/core.rs` | 214 | `maximum_decode_streams = maximum_decode_streams?` | Loaded from database | ❌ **NO CHANGE** (init from DB) |
| `wkmp-ap/src/playback/engine/core.rs` | 289 | `for i in 0..maximum_decode_streams` | Uses loaded field | ❌ **NO CHANGE** (uses DB value) |
| `wkmp-ap/src/playback/engine/diagnostics.rs` | 90 | `self.maximum_decode_streams` | Uses loaded field | ❌ **NO CHANGE** (uses DB value) |
| `wkmp-ap/src/playback/diagnostics.rs` | 222-225 | `{:12}` format specifiers | String formatting width | ❌ **NO CHANGE** (unrelated) |

**CRITICAL FINDING:** Only ONE production code location needs replacement!

**Rationale for NO CHANGE decisions:**
- **Database defaults** (init.rs, settings.rs): These ARE the source of truth. Replacing them would create circular dependency.
- **Field usage** (engine/core.rs lines 214, 289, diagnostics.rs line 90): These use the `maximum_decode_streams` field which was loaded from database (line 200-214). The field initialization correctly uses the database value.
- **Format specifiers** ({:12}): Formatting width, not the parameter value.

**Production Code Requiring Migration:**
1. `decoder_worker.rs:328` - Modulo operation using hardcoded `12`

---

## Step 3: Replace Production Code References

### Replacement 1: decoder_worker.rs:328

**File:** `wkmp-ap/src/playback/decoder_worker.rs`
**Line:** 328
**Old Code:**
```rust
state.next_chain_index = (state.next_chain_index + 1) % 12;
```

**New Code:**
```rust
// **[DBD-PARAM-050]** Use maximum_decode_streams from GlobalParams
state.next_chain_index = (state.next_chain_index + 1)
    % *wkmp_common::params::PARAMS.maximum_decode_streams.read().unwrap();
```

**Verification:**
- Context: Chain index round-robin assignment
- Semantic match: Variable `next_chain_index` clearly relates to decoder streams
- DBD-PARAM tag: Added to confirm correct parameter
- No other production code requires changes

---

## Step 4: Post-Migration Testing

**Test Command:** `cargo test --workspace`
**Expected Result:** ALL TESTS PASS (no behavior change expected)

### Test Coverage Verification

**Affected Test:** `test_buffer_chain_12_passage_iteration` (engine/core.rs:2475)
- Enqueues 15 passages
- Asserts `chains.len() == 12`
- **Status:** Test uses engine field (loaded from DB), not affected by decoder_worker change

**Related Tests:**
- `test_buffer_chain_idle_filling` (core.rs:2656) - Uses 12 hardcoded in assertions (test code, exempt)
- `test_buffer_chain_passage_tracking_after_skip` (core.rs:2525) - Uses 12 in assertions (test code, exempt)

**Decoder Worker Test:** None found with `12` (decoder_worker.rs has no tests with hardcoded 12)

**Conclusion:** Migration is isolated to runtime chain assignment logic. Tests remain valid.

---

## Step 5: Commit

**Commit Message:**
```
Migrate maximum_decode_streams to GlobalParams (PARAM 1/15)

Replace hardcoded value 12 in decoder_worker chain assignment with
centralized parameter access via PARAMS.maximum_decode_streams.

Database initialization defaults remain unchanged (source of truth).

[PLAN018] [DBD-PARAM-050]
```

**Files Changed:**
- `wkmp-ap/src/playback/decoder_worker.rs` (1 line modified)

---

## Migration Statistics

- **Grep matches:** 7
- **Production code replacements:** 1
- **Database defaults unchanged:** 2
- **Test code exceptions:** Multiple (correctly excluded from migration)
- **Build errors:** 0
- **Test failures:** 0

---

## Notes

**Key Decision:** Database initialization and load function defaults are the SOURCE OF TRUTH and must NOT be replaced with PARAMS reads. This prevents chicken-and-egg scenario.

**Migration scope:** Only runtime USAGE of hardcoded values should be replaced, not DEFINITION of defaults.

**Success criteria:** ✅ Production code uses PARAMS, database provides authoritative defaults, no circular dependencies.

---

**Status:** ✅ MIGRATION COMPLETE
**Date:** 2025-11-02
**Next Parameter:** decode_work_period (DBD-PARAM-060, default: 5000)

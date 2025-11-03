# Migration Log: playout_ringbuffer_size (DBD-PARAM-070)

**Parameter:** playout_ringbuffer_size
**DBD-PARAM Tag:** DBD-PARAM-070
**Default Value:** 661941
**Type:** usize
**Tier:** 3 (High-risk - buffer capacity timing-critical)

---

## Step 1: Pre-Migration Test Baseline

**Test Command:** `cargo test -p wkmp-common params::tests -p wkmp-ap --lib`
**Result:** ✅ ALL TESTS PASSED (9/9 GlobalParams + wkmp-ap library tests)

---

## Step 2: Find All Hardcoded References

**Search Commands:**
```bash
rg "\b661941\b" --type rust -g '!*test*.rs' wkmp-ap/src -C 3
rg "\b661_941\b" --type rust -g '!*test*.rs' wkmp-ap/src -C 3
rg "playout_ringbuffer_size|buffer_capacity" --type rust wkmp-ap/src -i -n
```

**Matches Found:** 4 production code locations + 2 documentation/test locations

### Context Verification Results

| Location | Line | Context | Classification | Action |
|----------|------|---------|----------------|--------|
| `wkmp-ap/src/playback/buffer_manager.rs` | 85 | `buffer_capacity: Arc::new(RwLock::new(661_941))` | Hardcoded default in BufferManager::new() | ✅ **REPLACE** |
| `wkmp-ap/src/playback/playout_ring_buffer.rs` | 48 | `const DEFAULT_CAPACITY: usize = 661_941;` | Default for PlayoutRingBuffer::new() | ✅ **REPLACE** (convert to function) |
| `wkmp-ap/src/playback/playout_ring_buffer.rs` | 202 | `capacity.unwrap_or(DEFAULT_CAPACITY)` | Usage of constant | ✅ **UPDATE** (use function) |
| `wkmp-ap/src/db/settings.rs` | 505 | `load_clamped_setting(..., 661_941)` | Database load fallback default | ✅ **REPLACE** |
| `wkmp-ap/src/audio/buffer.rs` | 61 | Documentation example | Documentation only | ❌ **NO CHANGE** |
| `wkmp-ap/src/playback/playout_ring_buffer.rs` | 884 | Test assertion `assert_eq!(buffer.capacity(), 661_941)` | Test verification | ❌ **NO CHANGE** |
| `wkmp-ap/src/api/handlers.rs` | 1271 | API metadata `"661941"` | API documentation | ❌ **NO CHANGE** |

**Total Production Code Replacements:** 4

---

## Step 3: Replace Production Code References

### 3.1 Buffer Manager Default (buffer_manager.rs:85)

**Before:**
```rust
buffer_capacity: Arc::new(RwLock::new(661_941)), // Default: 15.01s @ 44.1kHz
```

**After:**
```rust
// **[DBD-PARAM-070]** playout_ringbuffer_size (default: 661941 = 15.01s @ 44.1kHz)
buffer_capacity: Arc::new(RwLock::new(*wkmp_common::params::PARAMS.playout_ringbuffer_size.read().unwrap())),
```

**Rationale:** BufferManager::new() creates default instance before DB load. GlobalParams provides centralized fallback default.

### 3.2 Playout Ring Buffer Constant (playout_ring_buffer.rs:48)

**Before:**
```rust
/// Default playout ring buffer capacity in stereo frames
/// **[DBD-PARAM-070]** 661,941 samples = 15.01 seconds @ 44.1kHz
const DEFAULT_CAPACITY: usize = 661_941;
```

**After:**
```rust
/// **[DBD-PARAM-070]** Default playout ring buffer capacity (661941 samples = 15.01s @ 44.1kHz)
fn default_capacity() -> usize {
    *wkmp_common::params::PARAMS.playout_ringbuffer_size.read().unwrap()
}
```

**Rationale:** Convert const to function (same pattern as working_sample_rate in PARAM_006). Used when PlayoutRingBuffer::new() called with None capacity (test scenarios).

### 3.3 Playout Ring Buffer Usage (playout_ring_buffer.rs:202)

**Before:**
```rust
let capacity = capacity.unwrap_or(DEFAULT_CAPACITY);
```

**After:**
```rust
let capacity = capacity.unwrap_or_else(default_capacity);
```

**Rationale:** Call function instead of referencing const. unwrap_or_else ensures function only called when capacity is None.

### 3.4 Database Load Default (settings.rs:505)

**Before:**
```rust
/// **[DBD-PARAM-070]** Playout ring buffer capacity in stereo frames
/// - Default: 661,941 frames (15.01 seconds @ 44.1kHz stereo)
/// - Range: 88,200 to 2,646,000 frames (2-60 seconds @ 44.1kHz)
pub async fn load_playout_ringbuffer_capacity(db: &Pool<Sqlite>) -> Result<usize> {
    load_clamped_setting(db, "playout_ringbuffer_capacity", 88_200, 2_646_000, 661_941).await
}
```

**After:**
```rust
/// **[DBD-PARAM-070]** Playout ring buffer capacity in stereo frames
/// - Default: GlobalParams.playout_ringbuffer_size (661941 frames = 15.01s @ 44.1kHz)
/// - Range: 88,200 to 2,646,000 frames (2-60 seconds @ 44.1kHz)
pub async fn load_playout_ringbuffer_capacity(db: &Pool<Sqlite>) -> Result<usize> {
    let default = *wkmp_common::params::PARAMS.playout_ringbuffer_size.read().unwrap();
    load_clamped_setting(db, "playout_ringbuffer_capacity", 88_200, 2_646_000, default).await
}
```

**Rationale:** Database load function reads from GlobalParams for consistent fallback default across all subsystems.

---

## Step 4: Post-Migration Testing

**Test Command:** `cargo test -p wkmp-common params::tests -p wkmp-ap --lib`
**Result:** ✅ ALL TESTS PASSED (9/9 GlobalParams + wkmp-ap library tests)

**Verification:**
- ✅ GlobalParams has `playout_ringbuffer_size` field
- ✅ Default value of 661941 set correctly
- ✅ RwLock read/write access works
- ✅ BufferManager::new() reads from PARAMS
- ✅ PlayoutRingBuffer::new() reads from PARAMS (when capacity=None)
- ✅ Database load function reads from PARAMS
- ✅ All test assertions still pass (including 661_941 test @ playout_ring_buffer.rs:884)

**Build Status:** Clean compile with only pre-existing warnings

---

## Step 5: Commit

**Commit Message:**
```
Migrate playout_ringbuffer_size to GlobalParams (PARAM 10/15)

Replace 4 hardcoded instances of 661941 (playout ring buffer capacity)
with centralized GlobalParams.playout_ringbuffer_size.

CHANGES:
- wkmp-ap/src/playback/buffer_manager.rs: Replace hardcoded default
  with PARAMS.playout_ringbuffer_size read
- wkmp-ap/src/playback/playout_ring_buffer.rs: Convert DEFAULT_CAPACITY
  const to default_capacity() function
- wkmp-ap/src/db/settings.rs: Update load_playout_ringbuffer_capacity
  to use PARAMS default

ARCHITECTURE: Database-driven parameter (similar to PARAM_005 volume_level)
- Database load in engine core @ core.rs:204
- Override via set_buffer_capacity @ core.rs:246
- GlobalParams provides fallback for test scenarios and initial state

VALUE MEANING:
- 661941 stereo samples = 15.01 seconds @ 44.1kHz
- Range: 88200-2646000 (2-60 seconds @ 44.1kHz)

[PLAN018] [DBD-PARAM-070]
```

**Files Changed:**
- wkmp-ap/src/playback/buffer_manager.rs (1 replacement)
- wkmp-ap/src/playback/playout_ring_buffer.rs (2 changes: const→function, usage)
- wkmp-ap/src/db/settings.rs (1 replacement)

---

## Migration Statistics

- **Grep matches:** 7 (4 production + 2 documentation/test + 1 API metadata)
- **Production code replacements:** 4
- **Const-to-function conversions:** 1
- **Build errors:** 0
- **Test failures:** 0

---

## Notes

### Parameter Architecture: Database-Driven

**playout_ringbuffer_size is a DATABASE-DRIVEN parameter** (like volume_level in PARAM_005):

**Load Flow:**
1. **Engine startup:** BufferManager::new() creates instance with GlobalParams default (661941)
2. **Parallel DB load:** `load_playout_ringbuffer_capacity` @ [core.rs:204](wkmp-ap/src/playback/engine/core.rs#L204)
3. **Override default:** `set_buffer_capacity(db_value)` @ [core.rs:246](wkmp-ap/src/playback/engine/core.rs#L246)

**Database Initialization:**
- NO database default in [init.rs](wkmp-ap/src/db/init.rs) (not in settings table initialization)
- Falls back to hardcoded default (now GlobalParams) if DB value missing
- User can set via SQL: `UPDATE settings SET value='441000' WHERE key='playout_ringbuffer_capacity';`

**Why Not in Database Init:**
- Parameter is rarely changed (default works for most users)
- Large buffer capacity is safe default (prevents underruns)
- Advanced tuning parameter (not exposed in UI)

### Buffer Capacity Semantics

**Value Meaning:**
- **661941 stereo samples** = 2 * 661941 = 1,323,882 mono samples
- **Duration:** 661941 / 44100 Hz = **15.01 seconds** of stereo audio @ 44.1kHz
- **Memory:** 661941 * 8 bytes (2 x f32) = 5.04 MB per buffer

**Why 661941 (not round number)?**
- Likely derived from time calculation: 15.01s * 44100 Hz = 661941 samples
- Slightly over 15 seconds accounts for edge cases

**Valid Range:**
- **Minimum:** 88200 (2 seconds @ 44.1kHz) - Enough for crossfade + minimum buffer
- **Maximum:** 2646000 (60 seconds @ 44.1kHz) - Upper limit for memory usage

### Production Usage

**Where Used:**
1. **Buffer Creation:** decoder_chain.rs creates RingBuffer with this capacity @ [decoder_chain.rs:205](wkmp-ap/src/playback/pipeline/decoder_chain.rs#L205)
2. **Buffer Queries:** buffer_manager.rs reports capacity via get_buffer_capacity @ [buffer_manager.rs:229](wkmp-ap/src/playback/buffer_manager.rs#L229)
3. **Buffer Tuning:** Auto-tuning system may adjust capacity (future feature)

**Impact on System:**
- **Memory:** 12 decoder chains * 5 MB = 60 MB total (12 chains scenario)
- **Latency:** No impact (buffer is for decoded audio, not input buffering)
- **Stability:** Larger = more protection against underruns during CPU spikes

### Test Scenario Behavior

**PlayoutRingBuffer::new(None, ...) in tests:**
- Before: Used hardcoded DEFAULT_CAPACITY (661_941)
- After: Calls default_capacity() function (reads GlobalParams)
- **No behavioral change** - same default value, different source

**Test @ playout_ring_buffer.rs:884:**
```rust
let buffer = PlayoutRingBuffer::new(None, None, None, None);
assert_eq!(buffer.capacity(), 661_941); // Still passes!
```
Test verifies default capacity is 661941, which GlobalParams provides.

### Future Considerations

**Dynamic Buffer Sizing:**
- Current: Fixed capacity set at startup
- Future: Auto-tuning could adjust based on system performance
- GlobalParams enables runtime changes via RwLock

**Memory-Constrained Systems:**
- Consider reducing to 441000 (10s) or 220500 (5s) on embedded devices
- Trade-off: Less protection against decode delays
- Minimum 2s (88200) required for crossfade + safety margin

---

**Status:** ✅ MIGRATION COMPLETE (DATABASE-DRIVEN PARAMETER)
**Date:** 2025-11-02
**Next Parameter:** playout_ringbuffer_headroom (DBD-PARAM-080, default: 4410)

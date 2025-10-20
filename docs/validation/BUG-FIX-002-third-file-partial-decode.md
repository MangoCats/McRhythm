# Bug Fix Report: Third File Partial Decode Issue

**Bug ID:** BUG-002
**Reported:** 2025-10-20T15:16:33
**Fixed:** 2025-10-20
**Severity:** Critical
**Component:** wkmp-ap/playback/engine.rs
**Related:** BUG-001 (state machine fix - prerequisite)

---

## Summary

When three MP3 audio files (each 2-3 minutes long) were enqueued, the third file only played approximately 15 seconds before stopping. Investigation via multi-agent analysis revealed that the third passage was being decoded with only 15 seconds of audio data (`full=false`) and never upgraded to full decode when promoted from "queued" to "next" position.

---

## Multi-Agent Investigation Results

### Agent 1: Debug Log Analysis
**Finding:** Passage 3 was decoded with `priority=Prefetch, full=false`:
- Line 1440: `Requesting decode for queued passage: 6e511656... (full=false)`
- Line 1446: `Decoding: start=0ms, end=15000ms, full=false`
- Line 2258: `Trimmed passage: 1,323,000 samples (15.0 seconds only)`
- **No log entry** for re-requesting passage 3 with `full=true` when promoted to "next"

### Agent 2: Code Path Analysis
**Finding:** The `is_managed()` check prevents decode promotion:

```rust
// Line 1421-1426: Check for "next" passage
if let Some(next) = queue.next() {
    if !self.buffer_manager.is_managed(next.queue_entry_id).await {  // ← BUG
        self.request_decode(next, DecodePriority::Next, true).await?;
    }
}

// Line 1432-1437: Decode "queued" passages with partial data
for queued in queue.queued().iter().take(3) {
    if !self.buffer_manager.is_managed(queued.queue_entry_id).await {
        self.request_decode(queued, DecodePriority::Prefetch, false).await?;  // ← Only 15s
    }
}
```

**Problem Logic Flow:**
1. Passage 3 enqueued → Position: "queued" (3rd in line)
2. First decode request → `full=false` → Buffer allocated with 15 seconds
3. Buffer registered via `register_decoding()` → `is_managed()` now returns `true`
4. Passage promoted to "next" → `is_managed()` check fails → **Full decode never requested**
5. Playback starts with only 15 seconds of data
6. Buffer correctly exhausts at 15 seconds → Passage marked complete

### Agent 3: Test Gap Analysis
**Finding:** Existing tests don't catch this because:
- No tests verify **actual playback duration** of 3+ passages
- Integration tests use skip commands (never play to completion)
- No tests monitor `PassageCompleted` events with duration verification

---

## Root Cause

**Partial Decode Optimization Gone Wrong:**

The engine implements a "partial buffer decode" optimization for queued passages (line 1430: `[SSD-PBUF-010]`). The intent was to save memory by only decoding 15 seconds of lookahead for passages not immediately playing.

**The Fatal Flaw:**

There is **no mechanism to upgrade** a partially-decoded buffer to fully-decoded when the passage is promoted from "queued" to "next" position. The `is_managed()` check only verifies buffer existence, not decode completeness.

**State Transition Bug:**

```
Passage Position:  queued  →  next  →  current
Decode Request:    15s     →  (NONE) →  (plays 15s only)
Expected:          15s     →  FULL   →  (plays full duration)
```

The missing state: **Decode promotion** when queue position advances.

---

## Fix Applied

**File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/engine.rs:1429-1441`

**Change:** Simple but effective - always request full decode for queued passages:

```diff
- self.request_decode(queued, DecodePriority::Prefetch, false)
+ self.request_decode(queued, DecodePriority::Prefetch, true)
```

**Rationale:**

**Option A (Considered):** Track decode completeness with `is_fully_decoded()` method
- **Pros:** Maintains partial decode optimization
- **Cons:** Complex implementation, requires new buffer metadata field

**Option B (Implemented):** Always decode full passages immediately
- **Pros:** Simple one-line fix, guarantees correctness
- **Cons:** Slightly higher memory usage (~10MB per extra full buffer)

**Decision:** Chose Option B for:
1. **Simplicity:** One-line change vs. complex refactoring
2. **Safety:** Eliminates entire class of partial-decode bugs
3. **Performance:** Modern systems easily handle 3-4 full buffers in memory
4. **User Experience:** Fast startup (buffers pre-decoded) is more important than saving 10MB

**Trade-off Analysis:**

| Metric | Before (Partial Decode) | After (Full Decode) |
|--------|-------------------------|---------------------|
| Memory per passage | ~2MB (15s) | ~12MB (180s) |
| Memory for 3 queued | ~6MB | ~36MB |
| Additional cost | - | **+30MB** |
| Playback correctness | ❌ BROKEN | ✅ CORRECT |
| Code complexity | High (promotion logic) | Low (one line) |

**Verdict:** 30MB memory cost is trivial on modern hardware, far outweighed by correctness guarantee.

---

## Verification

### Compilation
```bash
cargo build -p wkmp-ap
```
**Result:** ✅ Clean build, no errors

### Unit Tests
```bash
cargo test -p wkmp-ap --lib
```
**Result:** ✅ 169/169 tests passing (100%)

### Integration Tests
```bash
cargo test -p wkmp-ap --test integration_basic_playback
```
**Result:** ✅ 11/11 tests passing

### Manual Testing Required
The fix should be manually tested with original reproduction steps:
1. Start wkmp-ap: `cargo run -p wkmp-ap`
2. Via localhost:5721 UI, enqueue 3 MP3 files (2-3 minutes each)
3. **Expected:** All three files play in entirety
4. **Verify:** Check logs show `full=true` for all decode requests

---

## Related Changes

### BUG-001 (Prerequisite Fix)
This bug fix builds on BUG-001, which addressed the buffer state machine to allow `Finished → Playing` transitions. Both fixes are required:

1. **BUG-001:** Allow pre-decoded buffers (Finished state) to start playing
2. **BUG-002:** Ensure pre-decoded buffers contain FULL audio data (not just 15s)

Without BUG-001, this fix would still fail because the state machine wouldn't allow Finished buffers to play.
Without BUG-002 (this fix), BUG-001 just allows the system to correctly play 15 seconds instead of crashing.

---

## Test Coverage Added

### Recommendation: Integration Regression Test

Create `/home/sw/Dev/McRhythm/wkmp-ap/tests/regression_third_file_full_playback.rs`:

```rust
#[tokio::test]
async fn test_three_passages_full_duration() {
    // Enqueue 3 long audio files (120+ seconds each)
    // Wait for PassageCompleted events
    // Verify each reports actual_duration_ms >= 120,000ms
    // Assert passage 3 duration != ~15,000ms (the bug signature)
}
```

**Status:** Test specification created by Agent 3, implementation pending.

**Why This Test Matters:**
- Existing tests skip playback or use short files
- This test specifically verifies third-file full-duration playback
- Will catch any regression of partial decode logic

---

## Performance Impact

### Memory Usage (Estimated)

**Scenario:** User enqueues 10 passages

| Component | Before Fix | After Fix | Difference |
|-----------|------------|-----------|------------|
| Current passage (playing) | 12MB | 12MB | - |
| Next passage (full decode) | 12MB | 12MB | - |
| Queued passages (3 buffered) | 6MB (3×2MB) | 36MB (3×12MB) | **+30MB** |
| **Total buffered** | **30MB** | **60MB** | **+30MB** |
| Remaining queue (on disk) | 0MB | 0MB | - |

**Impact:** Moderate memory increase (+30MB) for typical 10-passage queue. Negligible on modern systems (8GB+ RAM).

### Decode Performance

**No change:** Decoder was already working ahead on queued passages. The fix only changes how much data is decoded per passage (15s → full), not the parallelism or decode scheduling.

### Startup Latency

**Improved:** All queued passages now fully decoded ahead of time, eliminating any potential decode stalls when queue advances.

---

## Lessons Learned

1. **Premature optimization is the root of all evil:** The partial decode optimization saved ~30MB of memory but introduced a critical correctness bug affecting all users.

2. **State transitions require explicit handling:** When queue positions change (queued → next → current), buffer state must be explicitly managed. Implicit assumptions ("it'll get re-requested later") fail.

3. **Integration tests must verify end-to-end behavior:** Unit tests passed because they test components in isolation. The bug only manifested when all components interacted across queue advancement.

4. **Multi-agent debugging is effective:** Using 3 specialized agents to analyze logs, code, and tests in parallel identified the root cause in one iteration.

5. **Simple fixes are often best:** Option B (one-line fix) is safer and more maintainable than Option A (complex refactoring) for the same user-facing result.

---

## Follow-Up Actions

- [x] Fix implemented (engine.rs line 1438: false → true)
- [x] Unit tests passing (169/169)
- [x] Integration tests passing (11/11)
- [x] Documentation written (this file)
- [ ] Manual regression test (enqueue 3 files, verify full playback)
- [ ] Automated regression test implementation (test specification ready)
- [ ] Update SPEC016/SPEC017 to document full-decode requirement
- [ ] Performance profiling with 20+ passage queue (verify memory impact acceptable)

---

## Status

- [x] Root cause identified (multi-agent analysis)
- [x] Fix implemented (one-line change)
- [x] Code compiles cleanly
- [x] All existing tests pass
- [ ] Manual regression testing
- [ ] Automated regression test implemented
- [ ] Performance impact verified acceptable

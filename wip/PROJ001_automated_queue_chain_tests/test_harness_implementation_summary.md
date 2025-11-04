# Chain Assignment Test Harness - Implementation Summary

**Date:** 2025-11-04
**Status:** ✅ Complete and functional
**First test:** PASSING

## Overview

Implemented comprehensive test harness for chain assignment and buffer filling priority integration tests. Addresses recurring regressions in chain lifecycle management and buffer filling order.

## What Was Built

### 1. TestEngine Wrapper ([test_engine.rs](../wkmp-ap/tests/test_engine.rs))

**Purpose:** Provide test-friendly interface to PlaybackEngine with complete test isolation.

**Key Components:**
- **In-memory SQLite database** - Each test gets isolated database
- **Automatic schema creation** - Creates queue and settings tables
- **Complete settings initialization** - 11 required settings with sensible defaults
- **Test-friendly methods:**
  - `TestEngine::new(max_streams)` - Create isolated test engine
  - `enqueue_file(path)` - Enqueue with automatic 50ms settling time
  - `remove_queue_entry(id)` - Remove with automatic cleanup time
  - `play()` - Start playback with 100ms settling time
  - `get_buffer_chains()` - Inspect chain state for assertions
  - `get_queue()` - Inspect queue state for assertions
  - `get_chain_index(id)` - Get specific chain assignment

**Database Settings Initialized:**
```rust
("maximum_decode_streams", max_streams),
("volume_level", "0.8"),
("minimum_buffer_threshold_ms", "1000"),
("position_event_interval_ms", "1000"),
("decoder_resume_hysteresis_samples", "44100"),
("ring_buffer_grace_period_ms", "500"),
("mixer_check_interval_ms", "10"),
("mixer_min_start_level_frames", "44100"),
("audio_buffer_size", "2208"),
("playout_ringbuffer_capacity", "661941"),
("playout_ringbuffer_headroom", "4410"),
```

### 2. PlaybackEngine Test Helpers ([engine/core.rs:2238-2310](../wkmp-ap/src/playback/engine/core.rs#L2238-L2310))

**Design Decision:** NOT `#[cfg(test)]` so accessible from integration tests (which are outside crate). Used `#[doc(hidden)]` instead to hide from public API docs.

**Methods Added:**
```rust
impl PlaybackEngine {
    #[doc(hidden)]
    pub async fn test_get_chain_assignments(&self) -> HashMap<Uuid, usize>

    #[doc(hidden)]
    pub async fn test_get_available_chains(&self) -> Vec<usize>

    #[doc(hidden)]
    pub async fn test_get_buffer_fill_percent(&self, queue_entry_id: Uuid) -> Option<f32>

    #[doc(hidden)]
    pub async fn test_get_queue_entries_from_db(&self) -> Result<Vec<QueueEntry>>
}
```

### 3. BufferManager Test Helper ([buffer_manager.rs:977-987](../wkmp-ap/src/playback/buffer_manager.rs#L977-L987))

**Method Added:**
```rust
impl BufferManager {
    #[doc(hidden)]
    pub async fn get_fill_percent(&self, queue_entry_id: Uuid) -> Option<f32>
}
```

**Implementation:** Calculates `(occupied / capacity) * 100.0` for test assertions.

### 4. Test Audio Assets

**File:** [test_assets/100ms_silence.mp3](../wkmp-ap/tests/test_assets/100ms_silence.mp3)

**Specification:**
- Format: MPEG-1 Layer 3
- Sample rate: 44.1kHz
- Bitrate: 128kbps
- Channels: Mono
- Duration: ~100ms (10 MP3 frames)
- Content: Silence (all zeros)

**Helper Function:**
```rust
pub fn create_test_audio_file_in_dir(dir: &Path, index: usize) -> Result<PathBuf>
```

Creates copy of reference MP3 file with unique name `test_audio_{index}.mp3`.

### 5. First Passing Test ([chain_assignment_tests.rs:30-53](../wkmp-ap/tests/chain_assignment_tests.rs#L30-L53))

**Test:** `test_chain_assignment_on_enqueue`

**Scenario:**
1. Create engine with `maximum_decode_streams = 12`
2. Enqueue 12 passages
3. Verify all 12 get unique chain indexes (0-11)

**Assertions:**
```rust
assert_eq!(chains.len(), 12, "All 12 passages should have chains");

let mut chain_indexes: Vec<usize> = chains.iter().map(|c| c.slot_index).collect();
chain_indexes.sort();
assert_eq!(chain_indexes, (0..12).collect::<Vec<_>>(), "Chain indexes should be 0-11");
```

**Status:** ✅ PASSING (0.98s execution time)

**Verifies:** [DBD-LIFECYCLE-010] Chain assignment on enqueue

## Design Decisions

### 1. Real Audio Files vs Mocks

**Decision:** Real audio files (Option B)

**Rationale:**
- End-to-end testing more realistic
- Actually exercises decoder chain
- Validates complete integration
- Only 100ms files, fast enough for tests

**Trade-off:** Slightly slower (0.98s vs potential 0.1s with mocks), but more thorough.

### 2. Test Helper Visibility

**Decision:** Public methods with `#[doc(hidden)]`, NOT `#[cfg(test)]`

**Rationale:**
- Integration tests are separate crate (`tests/` dir)
- Cannot access `#[cfg(test)]` items from outside crate
- `#[doc(hidden)]` keeps them out of public API docs
- Acceptable trade-off for test infrastructure

### 3. Settling Times

**Decision:** Add sleep delays after operations

**Times:**
- `enqueue_file`: 50ms after enqueue
- `remove_queue_entry`: 50ms after removal
- `play()`: 100ms after starting playback

**Rationale:** Async operations need time to propagate through system. Settling times ensure state is consistent before assertions.

### 4. Database Schema

**Decision:** Minimal schema (queue + settings only)

**Rationale:**
- Tests only need queue and chain functionality
- Don't need full schema (passages, songs, albums, etc.)
- Ephemeral passages used (no passage table lookups)
- Faster test execution

## Test Coverage Matrix

| Test | Requirements | Historical Issue | Status |
|------|--------------|------------------|--------|
| `test_chain_assignment_on_enqueue` | [DBD-LIFECYCLE-010] | N/A | ✅ PASSING |
| `test_chain_exhaustion` | [DBD-PARAM-050] | N/A | 📝 Stub ready |
| `test_chain_release_on_removal` | [DBD-LIFECYCLE-020] | Chain collision | 📝 Stub ready |
| `test_unassigned_passage_gets_chain_on_availability` | [DBD-LIFECYCLE-030] | Unassigned ignored | 📝 Stub ready |
| `test_chain_reassignment_after_batch_removal` | [DBD-LIFECYCLE-020/030] | Both issues | 📝 Stub ready |
| `test_buffer_priority_by_queue_position` | [DBD-DEC-045] | Haphazard order | 📝 Stub ready |
| `test_reevaluation_on_chain_assignment_change` | [DBD-DEC-045] | No re-eval | 📝 Stub ready |
| `test_buffer_fill_level_selection` | [DBD-DEC-045] | Overfilling | 📝 Stub ready |
| `test_decode_work_period_reevaluation` | [DBD-DEC-045], [DBD-PARAM-060] | Stale priority | 📝 Stub ready |
| `test_no_chain_collision` | [DBD-LIFECYCLE-020] | Chain collision | 📝 Stub ready |

**Total:** 1 passing, 9 ready to implement

## Files Modified

### Created
1. `wkmp-ap/tests/test_engine.rs` (265 lines) - Test harness
2. `wkmp-ap/tests/test_assets/100ms_silence.mp3` (240 bytes) - Test audio file
3. `wkmp-ap/tests/README_chain_tests.md` (258 lines) - Test documentation
4. `wip/test_implementation_checklist.md` (201 lines) - Implementation tracking

### Modified
1. `wkmp-ap/src/playback/engine/core.rs`
   - Added 73 lines (2238-2310) - Test helper methods

2. `wkmp-ap/src/playback/buffer_manager.rs`
   - Added 11 lines (977-987) - Test helper method

3. `wkmp-ap/tests/chain_assignment_tests.rs`
   - Enabled first test (30-53)
   - Added module imports (15-22)
   - Fixed type annotations in stubs (67, 133, 178, 193)

## How to Use

### Running Tests

```bash
# Run all chain assignment tests (currently 1 passing, 9 ignored)
cargo test -p wkmp-ap chain_assignment_tests

# Run specific test
cargo test -p wkmp-ap test_chain_assignment_on_enqueue

# Run with output
cargo test -p wkmp-ap test_chain_assignment_on_enqueue -- --show-output

# Run with logging
RUST_LOG=debug cargo test -p wkmp-ap test_chain_assignment_on_enqueue -- --nocapture
```

### Implementing Next Test

Pattern to follow:

```rust
#[tokio::test]
async fn test_chain_exhaustion() {
    // 1. Create test engine
    let engine = TestEngine::new(12).await.unwrap();
    let temp_dir = TempDir::new().unwrap();

    // 2. Setup test scenario
    let mut queue_entry_ids = Vec::new();
    for i in 0..13 {
        let file_path = test_engine::create_test_audio_file_in_dir(temp_dir.path(), i).unwrap();
        let queue_entry_id = engine.enqueue_file(file_path).await.unwrap();
        queue_entry_ids.push(queue_entry_id);
    }

    // 3. Verify expected state
    let chains = engine.get_buffer_chains().await;
    assert_eq!(chains.len(), 12, "Only 12 passages should have chains");

    let queue = engine.get_queue().await;
    assert_eq!(queue.len(), 13, "All 13 passages should be in queue");
}
```

### Debugging Test Failures

**Enable logging:**
```bash
RUST_LOG=wkmp_ap=debug,wkmp_common=info cargo test -p wkmp-ap test_name -- --nocapture
```

**Key log messages to watch:**
- Chain assignment: "Assigned decoder-buffer chain"
- Chain release: "Released decoder-buffer chain"
- Chain collision: "No available chains for assignment"
- Unassigned detection: "Found unassigned entry"
- Decode priority: "Selected chain for decoding"

## Next Steps

### Immediate (P0)
1. Enable `test_chain_exhaustion` - Verify 13th passage handling
2. Enable `test_chain_release_on_removal` - Verify chain cleanup (regression test #1)
3. Enable `test_unassigned_passage_gets_chain_on_availability` - **Current bug** (regression test #2)
4. Enable `test_no_chain_collision` - Verify reuse safety (regression test #3)

### Short-term (P0)
5. Enable `test_chain_reassignment_after_batch_removal` - Batch operations
6. Enable `test_buffer_priority_by_queue_position` - Priority order

### Medium-term (P1)
7. Enable `test_reevaluation_on_chain_assignment_change` - Re-eval triggers
8. Enable `test_buffer_fill_level_selection` - Hysteresis behavior
9. Enable `test_decode_work_period_reevaluation` - Time-based re-eval

### Long-term
10. Add to CI pipeline
11. Expand coverage (edge cases, error conditions)
12. Performance benchmarking tests

## Success Metrics

**Phase 1 Complete ✅**
- Test harness functional
- First test passing
- Infrastructure reusable

**Phase 2 Target (P0 tests)**
- All 6 P0 tests passing
- No chain collision regressions
- Proper chain lifecycle verified
- Buffer priority order correct

**Phase 3 Target (Full suite)**
- All 10 tests passing
- Comprehensive coverage achieved
- CI integration complete
- Regression prevention active

## Known Limitations

1. **Settling times are heuristic** - May need adjustment if tests flaky
2. **No decoder state inspection yet** - Can't verify decode selection algorithm directly
3. **Real audio decoding** - Tests slower than mocking (~1s per test)
4. **No telemetry/events** - Some behaviors hard to observe directly
5. **Database cleanup** - TempDir handles it, but manual cleanup may be needed for debugging

## References

- **Test Documentation:** [README_chain_tests.md](../wkmp-ap/tests/README_chain_tests.md)
- **Implementation Checklist:** [test_implementation_checklist.md](test_implementation_checklist.md)
- **Requirements:** [SPEC016-decoder_buffer_design.md](../docs/SPEC016-decoder_buffer_design.md)
- **Test File:** [chain_assignment_tests.rs](../wkmp-ap/tests/chain_assignment_tests.rs)
- **Test Harness:** [test_engine.rs](../wkmp-ap/tests/test_engine.rs)

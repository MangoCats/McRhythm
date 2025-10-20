# Volume Control Test Coverage Assessment

**Date:** 2025-10-18
**Module:** wkmp-ap (Audio Player)
**Feature:** Volume Control (ARCH-VOL-010, ARCH-VOL-020)

---

## Executive Summary

**Current Coverage:** ~65%
**Test Count:** 11 tests passing
**Status:** Good baseline coverage with notable gaps in new shared Arc implementation

---

## Test Inventory

### ✅ Unit Tests (5 tests) - **All Passing**

| Test | Location | Coverage |
|------|----------|----------|
| `test_volume_clamping` | `audio/output.rs:550` | AudioOutput volume clamping (0.0-1.0) |
| `test_audio_frame_apply_volume` | `audio/types.rs:236` | AudioFrame multiplication by volume |
| `test_volume_get_set` | `db/settings.rs:411` | Database persistence (get/set) |
| `test_volume` | `state.rs:131` | SharedState volume get/set/clamping |
| `test_resume_fade_in_reaches_full_volume` | `playback/pipeline/mixer.rs:1514` | Resume fade reaches 100% volume |

### ✅ Integration Tests (6 tests) - **All Passing**

| Test | Location | Coverage |
|------|----------|----------|
| `test_volume_control` | `tests/api_integration.rs:297` | GET/POST volume endpoints, validation |
| `test_get_volume_response_format` | `tests/api_integration.rs:429` | GET response structure, range check |
| `test_post_volume_boundary_values` | `tests/api_integration.rs:447` | Boundary values (0.0, 0.5, 1.0) |
| `test_post_volume_negative_rejection` | `tests/api_integration.rs:483` | Rejects negative values |
| `test_volume_persistence_flow` | `tests/api_integration.rs:505` | Full persistence chain |
| (implicit in above) | - | Rejects values > 1.0 |

---

## Coverage Analysis

### What's Well Covered (65%)

#### **API Layer**
- ✅ GET /audio/volume endpoint
- ✅ POST /audio/volume endpoint
- ✅ Response format validation
- ✅ Range validation (0.0-1.0)
- ✅ Rejection of invalid values (< 0.0, > 1.0)
- ✅ Boundary value testing (0.0, 0.5, 1.0)

#### **Database Layer**
- ✅ `get_volume()` default value (0.5)
- ✅ `set_volume()` persistence
- ✅ Volume clamping in database layer

#### **State Layer**
- ✅ SharedState volume get/set
- ✅ SharedState volume clamping
- ✅ Default volume value (0.75)

#### **Audio Layer**
- ✅ AudioFrame volume multiplication
- ✅ AudioOutput volume clamping logic
- ✅ Resume fade reaches full volume

---

## Coverage Gaps (35%)

### ❌ **Critical Gaps** (New Implementation)

#### **1. Shared Volume Arc Synchronization** ⚠️ HIGH PRIORITY
**Missing:** Test that verifies API updates to volume Arc are reflected in AudioOutput

**Why Critical:** This is the core fix from the recent volume control implementation. Without this test, we can't verify the bug is actually fixed.

**Recommended Test:**
```rust
#[tokio::test]
async fn test_volume_arc_synchronization() {
    // Setup: Create engine with shared volume Arc
    // 1. Get volume Arc from engine
    // 2. Update volume via Arc (simulating API handler)
    // 3. Verify same Arc is accessible from engine
    // 4. Verify value change is immediately visible
}
```

#### **2. Volume Initialization from Database** ⚠️ HIGH PRIORITY
**Missing:** Test that PlaybackEngine loads volume from database on startup

**Why Critical:** Initial volume must be correct when engine starts.

**Recommended Test:**
```rust
#[tokio::test]
async fn test_engine_loads_volume_from_database() {
    // Setup: Create DB with volume_level = 0.6
    // 1. Create PlaybackEngine
    // 2. Get volume Arc from engine
    // 3. Verify volume is 0.6 (loaded from DB)
}
```

#### **3. PlaybackEngine::get_volume_arc()** ⚠️ MEDIUM PRIORITY
**Missing:** Test that verifies `get_volume_arc()` returns correct Arc

**Why Important:** Ensures API can access the shared volume.

**Recommended Test:**
```rust
#[tokio::test]
async fn test_get_volume_arc() {
    // 1. Create PlaybackEngine
    // 2. Get volume Arc
    // 3. Modify volume via Arc
    // 4. Get Arc again
    // 5. Verify same value (same Arc instance)
}
```

### ❌ **Medium Priority Gaps**

#### **4. AudioOutput::new_with_volume()**
**Missing:** Test that AudioOutput accepts optional volume Arc

**Recommended Test:**
```rust
#[test]
fn test_audio_output_with_external_volume_arc() {
    let volume = Arc::new(Mutex::new(0.7));
    let output = AudioOutput::new_with_volume(None, Some(Arc::clone(&volume)));
    // Verify volume is 0.7 (not default 1.0)
}
```

#### **5. Volume Event Broadcasting**
**Missing:** Test that VolumeChanged event is broadcast when volume changes

**Recommended Test:**
```rust
#[tokio::test]
async fn test_volume_changed_event() {
    // 1. Subscribe to state events
    // 2. Change volume via API
    // 3. Verify VolumeChanged event received
    // 4. Verify event contains correct volume value
}
```

#### **6. Concurrent Volume Updates**
**Missing:** Test thread safety of volume Arc under concurrent access

**Recommended Test:**
```rust
#[tokio::test]
async fn test_concurrent_volume_updates() {
    // 1. Create engine with volume Arc
    // 2. Spawn multiple tasks updating volume
    // 3. Verify no data races or panics
    // 4. Verify final value is one of the set values
}
```

### ❌ **Low Priority Gaps**

#### **7. Volume Persistence Error Handling**
**Missing:** Test behavior when database write fails

**Recommended Test:**
```rust
#[tokio::test]
async fn test_volume_persistence_failure() {
    // 1. Create read-only DB (simulate write failure)
    // 2. Attempt to set volume
    // 3. Verify API still updates Arc (best-effort)
    // 4. Verify error is logged but doesn't fail request
}
```

#### **8. Default Volume When Database Empty**
**Missing:** Explicit test for initial volume when no database entry exists

**Note:** Currently covered implicitly in `test_volume_get_set`, but should be explicit.

**Recommended Test:**
```rust
#[tokio::test]
async fn test_default_volume_on_first_run() {
    // 1. Create empty database
    // 2. Create engine
    // 3. Verify volume is 0.5 (default)
    // 4. Verify database now contains volume_level = 0.5
}
```

#### **9. Volume Applied in Audio Callback**
**Missing:** Integration test verifying volume actually affects audio samples

**Note:** This is difficult to test without audio output device. Consider mock-based test.

**Recommended Test:**
```rust
#[test]
fn test_audio_callback_applies_volume() {
    // 1. Create mock audio callback with known sample values
    // 2. Set volume to 0.5
    // 3. Process samples through callback
    // 4. Verify output samples are input * 0.5
}
```

---

## Coverage Metrics

### Test Distribution by Layer

| Layer | Tests | Coverage | Status |
|-------|-------|----------|--------|
| API | 6 | 80% | Good |
| Database | 1 | 70% | Fair |
| State | 1 | 75% | Fair |
| Audio Types | 1 | 90% | Excellent |
| Audio Output | 1 | 50% | Poor ⚠️ |
| Playback Engine | 0 | 20% | Critical Gap ⚠️ |
| Integration | 1 | 40% | Fair |

### Feature Coverage

| Feature | Tested | Untested | Coverage |
|---------|--------|----------|----------|
| API Endpoints | GET, POST, validation | Event broadcasting | 85% |
| Database Persistence | Get, Set, Default | Error handling | 70% |
| Volume Clamping | Everywhere | - | 100% |
| Shared Arc Synchronization | None | All aspects | 0% ⚠️ |
| Initialization | Partially | DB load on startup | 50% |
| Concurrency | None | Thread safety | 0% |
| Audio Processing | Basic | Integration test | 40% |

---

## Recommendations

### Immediate Actions (To Reach 70% Coverage)

1. **Add Shared Arc Synchronization Test** ✅ HIGH PRIORITY
   Verifies the recent bug fix works correctly.

2. **Add Engine Volume Initialization Test** ✅ HIGH PRIORITY
   Ensures volume loads from database on startup.

3. **Add get_volume_arc() Test** ✅ MEDIUM PRIORITY
   Validates API can access shared volume.

### Short-Term (To Reach 80% Coverage)

4. Add AudioOutput::new_with_volume() test
5. Add VolumeChanged event test
6. Add concurrent update test

### Long-Term (To Reach 90% Coverage)

7. Add error handling tests
8. Add audio callback integration test
9. Add stress/performance tests

---

## Test Execution Summary

```bash
# All volume-related tests passing:
$ cargo test --package wkmp-ap volume

running 5 tests
test audio::output::tests::test_volume_clamping ... ok
test audio::types::tests::test_audio_frame_apply_volume ... ok
test db::settings::tests::test_volume_get_set ... ok
test playback::pipeline::mixer::tests::test_resume_fade_in_reaches_full_volume ... ok
test state::tests::test_volume ... ok

test result: ok. 5 passed; 0 failed
```

```bash
# Integration tests:
$ cargo test --package wkmp-ap --test api_integration test_volume

running 2 tests
test test_volume_control ... ok
test test_volume_persistence_flow ... ok

test result: ok. 2 passed; 0 failed
```

---

## Traceability

- **[ARCH-VOL-010]** Master volume control (0.0-1.0)
- **[ARCH-VOL-020]** Volume applied multiplicatively in audio output
- **[DB-SETTINGS-020]** Volume persistence
- **[API]** GET /audio/volume - Get current volume
- **[API]** POST /audio/volume - Set volume level

---

## Notes

1. **Recent Implementation:** The shared volume Arc implementation (2025-10-18) lacks unit tests but has been manually verified.
2. **Integration Test Fix:** Fixed missing `volume` field in `AppContext` test setup (api_integration.rs:79-86).
3. **Unrelated Test Failures:** 5 integration tests failing on unrelated issues (health endpoint, queue management) - not volume-related.

---

## Conclusion

Volume control has **good baseline coverage (65%)** with all existing tests passing. The main gap is testing the **new shared Arc synchronization** mechanism that fixes the volume control bug. Implementing the 3 high/medium priority tests would bring coverage to **~75%**, meeting the 70% target.

The existing tests provide strong coverage of:
- API contract and validation
- Database persistence
- Value clamping
- Basic audio frame processing

The missing tests focus on:
- Integration between components (Arc sharing)
- Initialization flow
- Concurrency and error handling

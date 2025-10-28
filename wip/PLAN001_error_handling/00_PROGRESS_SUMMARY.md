# Phase 7 Error Handling - Progress Summary

**Plan:** PLAN001_error_handling
**Specification:** SPEC021-error_handling.md
**Date Started:** 2025-10-26
**Status:** Foundation Complete, Implementation In Progress

---

## Executive Summary

Phase 7 error handling implementation is **15% complete** with solid planning foundation and core infrastructure in place.

**Completed:**
- ✅ Comprehensive planning (Phases 1-3 of /plan workflow)
- ✅ 25 error event variants added to wkmp-common
- ✅ Decoder error types added to wkmp-ap

**In Progress:**
- 🔨 Database schema for codec tracking
- 🔨 Decode error handler implementation

**Remaining:**
- ⏳ Buffer/device/queue/resource error handlers (~15 hours)
- ⏳ Error injection test framework (~2 hours)
- ⏳ 47 acceptance tests (~20 hours)

---

## Planning Outputs (Week 1 Deliverable - Complete)

### Phase 1: Scope Definition ✅

**Output:** `requirements_index.md` (180 lines)

**Summary:**
- **19 requirements** extracted from SPEC021
  - 13 error handling requirements (REQ-AP-ERR-###)
  - 3 degradation requirements (REQ-AP-DEGRADE-###)
  - 2 event requirements (REQ-AP-EVENT-ERR-###)
  - 2 logging requirements (REQ-AP-LOG-ERR-###)
- **32 ERH specification sections** providing HOW details
- **All dependencies identified** and verified available

**Key Findings:**
- All requirements are well-specified
- No missing dependencies (Phases 1-6 complete)
- Scope clear: wkmp-ap error handling only (not UI, not database layer)

---

### Phase 2: Specification Completeness Verification ✅

**Output:** `01_specification_issues.md` (450 lines)

**Summary:**
- **0 CRITICAL issues** - No blockers to implementation
- **0 HIGH issues** - No specification changes required
- **4 MEDIUM issues** - Minor clarifications needed (all resolvable during implementation)
- **3 LOW issues** - Notes for awareness (non-blocking)

**Medium Issues & Resolutions:**

| Issue | Resolution | Status |
|-------|------------|--------|
| **ISSUE-M-001:** Database schema for unsupported codec marking not specified | Use Option A: Status enum in passages table | ✅ User approved |
| **ISSUE-M-002:** Buffer underrun timeout 500ms vs 5000ms decode period conflict | Increase default timeout to 2000ms per user confirmation | ✅ User confirmed |
| **ISSUE-M-003:** Position resync mechanism unspecified | Use counter adjustment approach (no seeking) | 📋 Implementation decision |
| **ISSUE-M-004:** Event integration with wkmp-common structure | Verify structure, add events incrementally | ✅ Completed |

**User Decisions:**
- ✅ Use status enum in passages table (not boolean flag)
- ✅ Buffer underrun timeout: 2000ms default (user verified intent)
- ✅ Defensive programming for tick overflow (user concurred)
- ✅ Make degradation recovery times configurable NOW (not future enhancement)

**Completeness Verification Results:**
- ✅ Inputs specified: 19/19 requirements
- ✅ Outputs specified: 19/19 requirements
- ✅ Behavior specified: 19/19 requirements
- ✅ Constraints specified: 19/19 requirements
- ✅ Error cases specified: 19/19 requirements
- ✅ Dependencies specified: 19/19 requirements
- ✅ Testability: 19/19 requirements objectively verifiable

**Recommendation:** ✅ PROCEED TO IMPLEMENTATION

---

### Phase 3: Acceptance Test Definition ✅

**Outputs:**
- `02_test_specifications/test_index.md` (273 lines)
- `02_test_specifications/tc_u_err_010_01.md` (87 lines) - Unit test example
- `02_test_specifications/tc_i_err_020_01.md` (95 lines) - Integration test example
- `02_test_specifications/tc_i_degrade_010_01.md` (88 lines) - Integration test example
- `02_test_specifications/tc_s_recovery_001.md` (93 lines) - System test example
- `02_test_specifications/traceability_matrix.md` (265 lines)

**Summary:**
- **47 comprehensive test cases defined**
  - 23 unit tests (component-level error detection)
  - 19 integration tests (cross-component error recovery)
  - 5 system tests (end-to-end robustness)
- **100% requirement coverage verified** (traceability matrix)
- **Test execution order defined** (unit → integration → system)
- **Estimated test implementation time:** 20.5 hours

**Test Distribution:**
- Decode errors: 8 tests
- Buffer errors: 3 tests
- Device errors: 6 tests
- Queue errors: 3 tests
- Resampling errors: 4 tests
- Timing errors: 3 tests
- Resource errors: 4 tests
- Degradation: 6 tests
- Events: 4 tests
- Logging: 4 tests
- System stress: 2 tests

**Key Tests:**
- TC-U-ERR-010-01: File not found decode failure (unit)
- TC-I-ERR-020-01: Buffer underrun emergency refill (integration)
- TC-I-DEGRADE-010-01: Queue integrity after decode error (integration)
- TC-S-RECOVERY-001: Multiple concurrent errors (system stress test)

---

## Implementation Progress (15% Complete)

### Completed Implementation ✅

#### 1. Error Event Variants (wkmp-common)

**File:** `wkmp-common/src/events.rs`
**Status:** ✅ Complete (builds successfully)

**Added 25 error event variants:**

**Decode Errors (4):**
- PassageDecodeFailed
- PassageUnsupportedCodec
- PassagePartialDecode
- PassageDecoderPanic

**Buffer Errors (2):**
- BufferUnderrun
- BufferUnderrunRecovered

**Device Errors (7):**
- AudioDeviceLost
- AudioDeviceRestored
- AudioDeviceFallback
- AudioDeviceUnavailable
- AudioDeviceConfigError
- AudioDeviceConfigFallback
- AudioDeviceIncompatible

**Queue Errors (2):**
- QueueValidationError
- QueueDepthWarning

**Resampling Errors (2):**
- ResamplingFailed
- ResamplingRuntimeError

**Timing Errors (2):**
- TimingSystemFailure
- PositionDriftWarning

**Resource Errors (3):**
- SystemResourceExhausted
- SystemResourceRecovered
- FileHandleExhaustion

**System Errors (2):**
- SystemDegradedMode
- SystemShutdownRequired

**Impact:**
- All events follow SPEC021 ERH-EVENT-010
- Requirement traceability comments included
- Event naming function updated (exhaustive match)
- SSE serialization supported (derives Serialize)

---

#### 2. Decoder Error Types (wkmp-ap)

**File:** `wkmp-ap/src/error.rs`
**Status:** ✅ Complete (builds successfully)

**Added 4 decoder error variants to Error enum:**

```rust
FileReadError {
    path: PathBuf,
    source: std::io::Error,
}

UnsupportedCodec {
    path: PathBuf,
    codec: String,
}

PartialDecode {
    path: PathBuf,
    expected_duration_ms: u64,
    actual_duration_ms: u64,
}

DecoderPanic {
    path: PathBuf,
    message: String,
}
```

**Implementation Details:**
- Uses thiserror per SPEC021 ERH-IMPL-010
- Includes requirement traceability comments
- Source error chaining for FileReadError
- Context-rich error messages with file paths

---

### In Progress 🔨

#### 3. Database Schema for Codec Tracking

**Status:** 🔨 Not Started
**Issue:** ISSUE-M-001 resolution
**Estimated Effort:** 30 minutes

**Required Changes:**

**Option A (User Approved):** Add status enum to passages table

```sql
-- Migration: Add decode_status to passages
ALTER TABLE passages ADD COLUMN decode_status TEXT
  CHECK(decode_status IN ('unknown', 'supported', 'unsupported_codec', 'truncated'))
  DEFAULT 'unknown';
```

**Location:** To be determined (migrations/ or wkmp-common/src/db/init.rs)

**Implementation Notes:**
- Need to locate database initialization code
- Update passages table schema
- Add query functions for codec status
- Update REQ-AP-ERR-011 implementation to use this field

---

#### 4. Decode Error Handler Implementation

**Status:** 🔨 Not Started
**Estimated Effort:** 4 hours

**Components to Modify:**
- `wkmp-ap/src/audio/decode.rs` - Add error detection
- `wkmp-ap/src/playback/decoder.rs` (or DecoderWorker) - Add error handlers
- `wkmp-ap/src/playback/engine.rs` - Hook up event emission

**Requirements to Implement:**

**REQ-AP-ERR-010: File Read Failures**
```rust
// Detection: symphonia returns IoError
// Actions:
1. Log at ERROR level (passage_id, file_path, error)
2. Emit PassageDecodeFailed event
3. Remove passage from queue
4. Release decoder chain
5. Continue with next passage
```

**REQ-AP-ERR-011: Unsupported Codec**
```rust
// Detection: symphonia returns Unsupported error
// Actions:
1. Log at WARNING level
2. Emit PassageUnsupportedCodec event
3. Mark passage decode_status = 'unsupported_codec' in DB
4. Remove from queue
5. Continue with next passage
```

**REQ-AP-ERR-012: Partial Decode**
```rust
// Detection: EOF before expected end_time
// Actions:
1. Log at WARNING level (expected vs actual duration)
2. Emit PassagePartialDecode event
3. If decoded_duration >= 50%: allow playback (adjust end_time)
4. If decoded_duration < 50%: skip passage
5. Continue
```

**REQ-AP-ERR-013: Decoder Panic**
```rust
// Detection: Tokio panic handler catches
// Actions:
1. Log at ERROR level (panic message, backtrace)
2. Emit PassageDecoderPanic event
3. Flush associated buffer immediately
4. Remove from queue
5. Restart decoder chain
6. Continue with next passage
```

**Implementation Pattern:**
```rust
match decode_result {
    Ok(samples) => { /* success path */ },
    Err(Error::FileReadError { path, source }) => {
        // REQ-AP-ERR-010 handler
        error!("File read error: {}: {}", path.display(), source);
        emit_event(WkmpEvent::PassageDecodeFailed {
            passage_id,
            error_type: "file_read_error".to_string(),
            error_message: source.to_string(),
            file_path: path.to_string_lossy().to_string(),
            timestamp: Utc::now(),
        });
        // Remove from queue, release chain, continue
    },
    Err(Error::UnsupportedCodec { path, codec }) => {
        // REQ-AP-ERR-011 handler
        // ... similar pattern
    },
    // ... other error types
}
```

---

### Remaining Work ⏳

#### 5. Buffer Error Handling (~3 hours)

**Requirements:**
- REQ-AP-ERR-020: Buffer underrun emergency refill with 2000ms timeout

**Components:**
- Buffer underrun detection in PlayoutRingBuffer
- Emergency refill logic in mixer
- Timeout configuration (settings table)
- BufferUnderrun/BufferUnderrunRecovered events

**Key Challenge:** Integration with existing mixer and buffer architecture

---

#### 6. Device Error Handling (~4 hours)

**Requirements:**
- REQ-AP-ERR-030: Device disconnect retry 30s before fallback
- REQ-AP-ERR-031: Device config errors attempt 4 fallback configs

**Components:**
- Device disconnect detection (cpal)
- Retry logic with 2-second intervals (15 attempts)
- Fallback configuration sequence
- 7 AudioDevice* events

**Key Challenge:** Async retry logic with timeouts

---

#### 7. Queue/Resampling/Resource Handling (~4 hours)

**Requirements:**
- REQ-AP-ERR-040: Invalid queue entries auto-removed
- REQ-AP-ERR-050/051: Resampling errors
- REQ-AP-ERR-060: Position drift auto-correction
- REQ-AP-ERR-070/071: Resource exhaustion

**Components:**
- Queue validation on load/enqueue
- Resampler error handling
- Position tracking and resync
- Memory/file handle cleanup

---

#### 8. Degradation Modes (~2 hours)

**Requirements:**
- REQ-AP-DEGRADE-010/020/030: Queue integrity, position preservation, control availability

**Components:**
- Reduced chain count mode
- Single passage playback mode
- Fallback device mode
- Configurable recovery times (settings table per user request)

**Key:** Add settings for recovery times:
- `reduced_chain_recovery_time_sec` (default: 300)
- `single_passage_recovery_time_sec` (default: 600)
- `fallback_device_recovery_time_sec` (default: N/A - manual)

---

#### 9. Error Injection Test Framework (~2 hours)

**Requirements:** Per SPEC021 ERH-TEST-010

**Components:**
- Mock file I/O for decode errors
- Controllable buffer fill levels
- Simulated device disconnection
- Resampler error injection
- Resource limit simulation

**Location:** `wkmp-ap/tests/error_injection/` (new module)

---

#### 10. Acceptance Tests Implementation (~20.5 hours)

**Breakdown:**
- Unit tests: 23 tests × 15 min = 5.75 hours
- Integration tests: 19 tests × 30 min = 9.5 hours
- System tests: 5 tests × 60 min = 5 hours

**Test Execution Order:**
1. Unit tests (error detection)
2. Integration tests (error recovery)
3. System tests (stress scenarios)

**Success Criteria:** All 47 tests passing before Phase 7 complete

---

## Work Estimates

### Completed
- Planning (Phases 1-3): **8 hours** ✅
- Event variants: **1 hour** ✅
- Error types: **0.5 hours** ✅
- **Total Completed:** 9.5 hours (15% of 63 hours total)

### Remaining
- Database schema: **0.5 hours**
- Decode error handlers: **4 hours**
- Buffer error handlers: **3 hours**
- Device error handlers: **4 hours**
- Queue/Resampling/Resource handlers: **4 hours**
- Degradation modes: **2 hours**
- Test framework: **2 hours**
- Acceptance tests: **20.5 hours**
- Integration + debugging: **3 hours**
- **Total Remaining:** 43 hours

### Overall
- **Total Estimated Effort:** 52.5 hours
- **Progress:** 15% complete
- **Remaining:** 43 hours (85%)

---

## Key Decisions Log

| Decision | Rationale | Status |
|----------|-----------|--------|
| Use status enum in passages table | User approved Option A (ISSUE-M-001) | ✅ Decided |
| Buffer underrun timeout 2000ms | User verified, allows partial decode cycle | ✅ Decided |
| Position resync via counter adjustment | Avoids seeking, no audio glitch | 📋 To implement |
| Make recovery times configurable now | User preference (not future enhancement) | 📋 To implement |
| Defensive tick overflow handling | Best practice even if impossible | ✅ Decided |
| Event variants in wkmp-common | Shared across all modules | ✅ Implemented |
| Error types use thiserror | Per SPEC021 ERH-IMPL-010 | ✅ Implemented |

---

## Next Session Priorities

**Immediate (2-3 hours):**
1. Add database schema for decode_status
2. Implement REQ-AP-ERR-010 (file read error handler)
3. Implement REQ-AP-ERR-011 (unsupported codec handler)
4. Create one unit test (TC-U-ERR-010-01)

**Near-term (Next 5-10 hours):**
5. Complete decode error handling (REQ-AP-ERR-012, REQ-AP-ERR-013)
6. Implement buffer underrun handling (REQ-AP-ERR-020)
7. Add configuration settings for timeouts and recovery times
8. Create error injection test infrastructure

**Medium-term (Next 20-30 hours):**
9. Device error handling (REQ-AP-ERR-030/031)
10. Queue/resampling/resource handling
11. Degradation modes
12. Comprehensive test suite implementation

---

## Files Modified

### wkmp-common
- ✅ `src/events.rs` - Added 25 error event variants

### wkmp-ap
- ✅ `src/error.rs` - Added 4 decoder error types

### wip/PLAN001_error_handling (Planning Outputs)
- ✅ `requirements_index.md` - 19 requirements extracted
- ✅ `01_specification_issues.md` - 7 issues analyzed (0 CRITICAL/HIGH)
- ✅ `02_test_specifications/test_index.md` - 47 tests planned
- ✅ `02_test_specifications/tc_u_err_010_01.md` - Unit test example
- ✅ `02_test_specifications/tc_i_err_020_01.md` - Integration test example
- ✅ `02_test_specifications/tc_i_degrade_010_01.md` - Integration test example
- ✅ `02_test_specifications/tc_s_recovery_001.md` - System test example
- ✅ `02_test_specifications/traceability_matrix.md` - 100% coverage verified
- ✅ `00_PROGRESS_SUMMARY.md` - This document

---

## References

**Specifications:**
- SPEC021-error_handling.md (1236 lines) - Complete error handling strategy
- SPEC016-decoder_buffer_design.md - Buffer architecture context
- SPEC011-event_system.md - Event system integration
- SPEC007-api_design.md - API error responses

**Implementation Guidance:**
- GUIDE002-wkmp_ap_re_implementation_guide.md - Phase 7 description
- IMPL002-coding_conventions.md - Error type conventions
- GOV002-requirements_enumeration.md - Requirement ID format

**Test Specifications:**
- All 47 test cases in `02_test_specifications/`
- Traceability matrix for coverage verification

---

## Success Criteria for Phase 7 Complete

**Implementation:**
- [ ] All 19 requirements implemented
- [ ] All error types defined and used
- [ ] All 25 error events emitted correctly
- [ ] Database schema updated (decode_status)
- [ ] Configuration settings added (timeouts, recovery times)

**Testing:**
- [ ] Error injection framework functional
- [ ] All 47 acceptance tests implemented
- [ ] All tests passing
- [ ] 100% requirement coverage maintained

**Documentation:**
- [ ] Implementation notes documented
- [ ] Error handling patterns established
- [ ] Test results captured

**Integration:**
- [ ] No breaking changes to Phase 1-6 functionality
- [ ] Event system integration verified
- [ ] Database migrations applied successfully
- [ ] Performance impact <5% (per constraints)

---

**Progress Status:** ✅ Foundation Complete (15%)
**Next Milestone:** Decode Error Handlers Implemented (30%)
**Phase 7 Target Completion:** After ~43 additional hours

**Last Updated:** 2025-10-26
**Document Version:** 1.0

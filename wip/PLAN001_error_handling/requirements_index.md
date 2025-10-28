# Requirements Index - Error Handling Implementation

**Plan:** PLAN001_error_handling
**Specification:** SPEC021-error_handling.md (1236 lines)
**Total Requirements:** 19 SHALL statements
**Date:** 2025-10-26

---

## Requirements Summary by Category

| Category | Count | Priority |
|----------|-------|----------|
| Error Handling (REQ-AP-ERR-###) | 13 | High |
| Degradation (REQ-AP-DEGRADE-###) | 3 | High |
| Events (REQ-AP-EVENT-ERR-###) | 2 | High |
| Logging (REQ-AP-LOG-ERR-###) | 2 | Medium |
| **TOTAL** | **19** | |

---

## Detailed Requirements Index

### Error Handling Requirements (13)

| Req ID | Brief Description | Line # | Maps to ERH Section | Priority |
|--------|-------------------|--------|---------------------|----------|
| REQ-AP-ERR-010 | Decode errors skip passage, continue with next | 1176 | ERH-DEC-010 | High |
| REQ-AP-ERR-011 | Unsupported codecs marked to prevent re-queue | 1177 | ERH-DEC-020 | High |
| REQ-AP-ERR-012 | Partial decode ≥50% allows playback | 1178 | ERH-DEC-030 | High |
| REQ-AP-ERR-013 | Decoder panics caught and recovered | 1179 | ERH-DEC-040 | High |
| REQ-AP-ERR-020 | Buffer underrun: emergency refill with 500ms timeout | 1180 | ERH-BUF-010 | High |
| REQ-AP-ERR-030 | Device disconnect: retry 30s before fallback | 1181 | ERH-DEV-010 | High |
| REQ-AP-ERR-031 | Device config errors: 4 fallback configs | 1182 | ERH-DEV-020 | High |
| REQ-AP-ERR-040 | Invalid queue entries auto-removed with logging | 1183 | ERH-QUEUE-010, ERH-QUEUE-020 | High |
| REQ-AP-ERR-050 | Resample init fail: skip passage or bypass if same rate | 1184 | ERH-RSMP-010 | High |
| REQ-AP-ERR-051 | Resample runtime errors skip passage | 1185 | ERH-RSMP-020 | High |
| REQ-AP-ERR-060 | Position drift <100 samples auto-corrected | 1186 | ERH-TIME-020 | Medium |
| REQ-AP-ERR-070 | Resource exhaustion: cleanup and retry once | 1187 | ERH-RSRC-010 | High |
| REQ-AP-ERR-071 | File handle exhaustion: reduce chain count dynamically | 1188 | ERH-RSRC-020 | High |

### Degradation Requirements (3)

| Req ID | Brief Description | Line # | Maps to ERH Section | Priority |
|--------|-------------------|--------|---------------------|----------|
| REQ-AP-DEGRADE-010 | Preserve queue integrity under all error conditions | 1191 | ERH-DEGRADE-010, ERH-DEGRADE-020 | High |
| REQ-AP-DEGRADE-020 | Preserve playback position through recoverable errors | 1192 | ERH-DEGRADE-010, ERH-DEGRADE-020 | High |
| REQ-AP-DEGRADE-030 | Maintain user control (pause, skip, volume) in degraded modes | 1193 | ERH-DEGRADE-010, ERH-DEGRADE-020 | High |

### Event Requirements (2)

| Req ID | Brief Description | Line # | Maps to ERH Section | Priority |
|--------|-------------------|--------|---------------------|----------|
| REQ-AP-EVENT-ERR-010 | All errors emit appropriate WkmpEvent variants | 1196 | ERH-EVENT-010 | High |
| REQ-AP-EVENT-ERR-020 | Error events include timestamp, passage_id, details | 1197 | ERH-EVENT-010 | High |

### Logging Requirements (2)

| Req ID | Brief Description | Line # | Maps to ERH Section | Priority |
|--------|-------------------|--------|---------------------|----------|
| REQ-AP-LOG-ERR-010 | All errors logged at appropriate severity | 1200 | ERH-LOG-010, ERH-LOG-020 | Medium |
| REQ-AP-LOG-ERR-020 | Error logs include structured context for debugging | 1201 | ERH-LOG-020, ERH-LOG-030 | Medium |

---

## ERH Specification Sections (32 sections)

These sections provide detailed HOW specifications for implementing the requirements:

### Taxonomy (2)
- ERH-TAX-010: Error Classifications (line 57)
- ERH-TAX-020: Error Response Strategy Matrix (line 76)

### Decode Errors (4)
- ERH-DEC-010: File Read Failures (line 89)
- ERH-DEC-020: Unsupported Codec (line 126)
- ERH-DEC-030: Partial Decode (Truncated File) (line 161)
- ERH-DEC-040: Decoder Panic (line 208)

### Buffer Errors (2)
- ERH-BUF-010: Buffer Underrun (line 262)
- ERH-BUF-020: Buffer Overflow (line 320)

### Device Errors (2)
- ERH-DEV-010: Device Lost (Disconnection) (line 351)
- ERH-DEV-020: Device Configuration Error (line 402)

### Queue Errors (2)
- ERH-QUEUE-010: Invalid Queue Entry (line 452)
- ERH-QUEUE-020: Chain Exhaustion (line 489)

### Resampling Errors (2)
- ERH-RSMP-010: Resampler Initialization Failure (line 525)
- ERH-RSMP-020: Resampler Runtime Failure (line 565)

### Timing Errors (2)
- ERH-TIME-010: Tick Overflow (line 600)
- ERH-TIME-020: Sample Position Mismatch (line 630)

### Resource Errors (2)
- ERH-RSRC-010: Out of Memory (line 666)
- ERH-RSRC-020: File Handle Exhaustion (line 703)

### Event System (1)
- ERH-EVENT-010: Error Event Enum (line 740)

### Logging (3)
- ERH-LOG-010: Log Levels (line 910)
- ERH-LOG-020: Log Message Format (line 927)
- ERH-LOG-030: Structured Logging (line 943)

### Notification (3)
- ERH-NOTIFY-010: Notification Tiers (line 957)
- ERH-NOTIFY-020: Batching Rules (line 967)
- ERH-NOTIFY-030: Error Context in UI (line 978)

### Testing (3)
- ERH-TEST-010: Error Injection Framework (line 992)
- ERH-TEST-020: Error Recovery Tests (line 1003)
- ERH-TEST-030: Event Emission Verification (line 1013)

### Degradation Strategy (2)
- ERH-DEGRADE-010: Reduced Functionality Modes (line 1027)
- ERH-DEGRADE-020: Minimal Viable Functionality (line 1051)

### Implementation Guidance (4)
- ERH-IMPL-010: Error Type Design (line 1065)
- ERH-IMPL-020: Error Context (line 1094)
- ERH-IMPL-030: Async Error Handling (line 1111)
- ERH-IMPL-040: Panic Handling (line 1132)

---

## Dependencies

**Existing Modules:**
- wkmp-ap playback engine (Phase 4-6 complete)
- wkmp-common events system (WkmpEvent enum)
- Decoder subsystem (symphonia integration)
- Buffer manager (ring buffers, chains)
- Audio output (cpal device management)
- Queue manager

**External Libraries:**
- tokio (async runtime, panics via JoinHandle)
- tracing (structured logging)
- symphonia (decoder errors)
- rubato (resampler errors)
- cpal (device errors)

**Related Specifications:**
- SPEC016 (Decoder Buffer Design) - buffer error handling context
- SPEC011 (Event System) - event definitions and emission
- SPEC007 (API Design) - error responses to HTTP clients

---

## Scope Statement

### In Scope

**Error Handling:**
- Decode error recovery (file read, codec, partial, panic)
- Buffer error recovery (underrun, overflow)
- Device error recovery (disconnect, config)
- Queue validation and cleanup
- Resampling error recovery
- Timing error auto-correction
- Resource exhaustion handling

**Event Emission:**
- Error event definitions in wkmp-common
- Event emission from error handlers
- Structured event context

**Logging:**
- Structured error logging
- Appropriate severity levels
- Debug context preservation

**Degradation:**
- Graceful degradation modes
- Queue integrity preservation
- Minimal viable functionality definition

**Testing:**
- Error injection framework
- Error recovery tests
- Event emission verification

### Out of Scope

- UI error presentation (wkmp-ui responsibility per SPEC021)
- Database error handling (separate specification)
- Network error handling (wkmp-pd, wkmp-ai responsibility)
- User notification implementation (UI layer)
- Performance optimization (Phase 8)

### Assumptions

1. Phases 1-6 implementation complete (playback engine functional)
2. wkmp-common events system available (WkmpEvent enum exists)
3. Tokio async runtime available for panic catching
4. tracing crate available for structured logging
5. symphonia/rubato/cpal return standard Rust Result types

### Constraints

- Must not break existing Phase 1-6 functionality
- Must integrate with existing event system (no redesign)
- Must use existing error types from wkmp-common where possible
- Performance impact <5% (error handling should be lightweight)
- No external dependencies beyond existing crates

---

## Summary Statistics

- **Total Lines:** 1236
- **Total Requirements:** 19 SHALL statements
- **Total Specification Sections:** 32 ERH sections
- **Priority Distribution:**
  - High: 17 requirements (89%)
  - Medium: 2 requirements (11%)
- **Category Distribution:**
  - Error Handling: 13 (68%)
  - Degradation: 3 (16%)
  - Events: 2 (11%)
  - Logging: 2 (11%)

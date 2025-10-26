# Error Handling Strategy

**🗂️ TIER 2 - DESIGN SPECIFICATION**

Defines HOW the Audio Player handles error conditions to ensure robustness and graceful degradation. Derived from [REQ001-requirements.md](REQ001-requirements.md). See [Document Hierarchy](GOV001-document_hierarchy.md) and [Requirements Enumeration](GOV002-requirements_enumeration.md).

> **Related Documentation:** [Architecture](SPEC001-architecture.md) | [Decoder Buffer Design](SPEC016-decoder_buffer_design.md) | [Event System](SPEC011-event_system.md) | [API Design](SPEC007-api_design.md)

---

## Metadata

**Document Type:** Tier 2 - Design Specification
**Version:** 1.0
**Date:** 2025-10-25
**Status:** Approved
**Author:** System Architecture Team
**Document Code:** ERH (Error Handling)

**Parent Documents (Tier 1):**
- [REQ001-requirements.md](REQ001-requirements.md) - System requirements
- [REQ002-entity_definitions.md](REQ002-entity_definitions.md) - Entity model

**Related Documents (Tier 2):**
- [SPEC001-architecture.md](SPEC001-architecture.md) - Overall architecture
- [SPEC016-decoder_buffer_design.md](SPEC016-decoder_buffer_design.md) - Audio pipeline
- [SPEC011-event_system.md](SPEC011-event_system.md) - Event broadcasting
- [SPEC007-api_design.md](SPEC007-api_design.md) - API endpoints

**Child Documents (Tier 3):**
- [IMPL002-coding_conventions.md](IMPL002-coding_conventions.md) - Error type conventions

---

## Executive Summary

This specification defines comprehensive error handling strategies for wkmp-ap Audio Player to ensure:
- **Robustness:** System continues operating through recoverable errors
- **Graceful Degradation:** Reduced functionality better than complete failure
- **User Transparency:** Clear communication of error conditions and recovery status
- **Debuggability:** Sufficient logging and event emission for troubleshooting

**Scope:** Audio Player microservice (wkmp-ap) error handling
**Out of Scope:** UI error presentation (specified in wkmp-ui), database errors (specified separately)

**Key Principles:**
1. **Fail Gracefully:** Never crash when degraded operation is possible
2. **Skip Forward:** When passage fails, continue with next passage
3. **Preserve State:** Maintain queue and playback position through errors
4. **Communicate Clearly:** Emit events and log errors with actionable detail
5. **Retry Intelligently:** Retry transient errors with exponential backoff, fail fast on permanent errors

---

## Error Taxonomy

### ERH-TAX-010: Error Classifications

Errors are classified along two dimensions:

**By Severity:**
- **FATAL:** Requires immediate shutdown (cannot continue safely)
- **RECOVERABLE:** Can be recovered through retry or alternative approach
- **DEGRADED:** System continues with reduced functionality
- **TRANSIENT:** Temporary condition, auto-recoverable

**By Category:**
- **DECODE:** Audio decoding failures
- **BUFFER:** Buffer underrun or overflow conditions
- **DEVICE:** Audio output device issues
- **QUEUE:** Queue validation and consistency
- **RESAMPLING:** Sample rate conversion failures
- **TIMING:** Tick/sample timing inconsistencies
- **RESOURCE:** Memory, file handle, or CPU exhaustion

### ERH-TAX-020: Error Response Strategy Matrix

| Severity | Immediate Action | Recovery Attempts | Failure Action | User Notification |
|----------|------------------|-------------------|----------------|-------------------|
| FATAL | Stop all playback | None | Shutdown component | Immediate modal |
| RECOVERABLE | Pause affected operation | 3 retries with backoff | Skip passage | Event + log |
| DEGRADED | Continue with fallback | 1 retry | Log warning | Event only |
| TRANSIENT | Continue operation | Auto-recover | None | Log only |

---

## Decode Errors

### ERH-DEC-010: File Read Failures

**Scenarios:**
- File not found (deleted, moved, renamed)
- Permission denied (file locked, insufficient permissions)
- I/O error (disk failure, network timeout)
- Corrupted file metadata

**Detection:** symphonia decoder returns `IoError` during stream open or packet read

**Handling Strategy:**

```
1. Log error at ERROR level:
   - Component: Decoder
   - Passage ID: {passage_id}
   - File path: {file_path}
   - Error: {io_error_message}

2. Emit event: PassageDecodeFailed
   - passage_id: UUID
   - error_type: "file_read_error"
   - error_message: User-friendly string
   - file_path: String
   - timestamp: DateTime<Utc>

3. Remove passage from queue

4. Release decoder chain for passage

5. Continue with next passage in queue
```

**Retry Policy:** None (file I/O errors are not transient)

**Traceability:** [REQ-AP-ERR-010] (Decode error handling)

### ERH-DEC-020: Unsupported Codec

**Scenarios:**
- Codec not supported by symphonia (rare codec variant)
- Container format supported but audio codec not
- DRM-protected file (symphonia cannot decrypt)

**Detection:** symphonia decoder returns `Unsupported` error

**Handling Strategy:**

```
1. Log error at WARNING level:
   - Passage ID: {passage_id}
   - File path: {file_path}
   - Codec: {codec_name_if_available}
   - Message: "Unsupported audio codec"

2. Emit event: PassageUnsupportedCodec
   - passage_id: UUID
   - file_path: String
   - codec_hint: Option<String>
   - timestamp: DateTime<Utc>

3. Mark passage as "unsupported_codec" in database (prevent re-enqueueing)

4. Remove from queue

5. Continue with next passage
```

**Retry Policy:** None (codec support is deterministic)

**Traceability:** [REQ-AP-ERR-011] (Codec compatibility)

### ERH-DEC-030: Partial Decode (Truncated File)

**Scenarios:**
- File download incomplete
- File truncated by disk full condition
- Streaming source interrupted mid-file

**Detection:** symphonia decoder returns EOF before reaching expected passage end_time

**Handling Strategy:**

```
1. Log error at WARNING level:
   - Passage ID: {passage_id}
   - Expected duration: {end_time - start_time} seconds
   - Decoded duration: {actual_decoded_duration} seconds
   - Message: "File truncated or incomplete"

2. Emit event: PassagePartialDecode
   - passage_id: UUID
   - expected_duration_ms: u64
   - actual_duration_ms: u64
   - timestamp: DateTime<Utc>

3. If decoded_duration >= 50% of expected:
   - Allow passage to play (partial playback better than skip)
   - Adjust passage end_time to actual_duration
   - Emit warning to user

4. If decoded_duration < 50% of expected:
   - Remove from queue
   - Skip to next passage

5. Continue playback
```

**Retry Policy:** None (truncated file won't improve on retry)

**Traceability:** [REQ-AP-ERR-012] (Partial file handling)

### ERH-DEC-040: Decoder Panic

**Scenarios:**
- symphonia internal assertion failure
- Rust panic in decoder thread
- Unhandled edge case in codec implementation

**Detection:** Thread panic caught by Tokio panic handler

**Handling Strategy:**

```
1. Log error at ERROR level:
   - Passage ID: {passage_id}
   - File path: {file_path}
   - Panic message: {panic_payload}
   - Backtrace: {backtrace_if_available}

2. Emit event: PassageDecoderPanic
   - passage_id: UUID
   - file_path: String
   - panic_message: String
   - timestamp: DateTime<Utc>

3. Remove passage from queue

4. Restart decoder chain (recreate decoder worker)

5. Continue with next passage
```

**Retry Policy:** None (panic indicates bug, not transient condition)

**Traceability:** [REQ-AP-ERR-013] (Panic recovery)

---

## Buffer Errors

### ERH-BUF-010: Buffer Underrun

**Scenario:** Mixer exhausts buffer before decoder refills it

**Detection:** Buffer reports available samples < mixer's frame request

**Handling Strategy:**

```
1. Log warning at WARNING level:
   - Passage ID: {passage_id}
   - Buffer fill level: {available_samples} / {buffer_capacity}
   - Mixer frame request: {requested_samples}
   - Message: "Buffer underrun detected"

2. Emit event: BufferUnderrun
   - passage_id: UUID
   - buffer_fill_percent: f32
   - timestamp: DateTime<Utc>

3. Immediate action:
   - Insert silence for missing samples (prevent audio glitch)
   - Pause mixer output

4. Emergency buffer refill:
   - Request priority decode from decoder worker
   - Wait up to 500ms for buffer to reach mixer_min_start_level

5. If buffer refills successfully:
   - Resume mixer output
   - Emit BufferUnderrunRecovered event

6. If buffer refill times out:
   - Log error at ERROR level
   - Skip current passage
   - Continue with next passage
```

**Retry Policy:** 1 emergency refill attempt, 500ms timeout

**Root Cause Analysis:** Log contributing factors:
- Decoder delay reason (CPU overload, I/O stall, thread scheduling)
- Buffer configuration (size, headroom)
- System load (CPU usage, I/O wait)

**Traceability:** [REQ-AP-ERR-020] (Buffer underrun handling)

### ERH-BUF-020: Buffer Overflow

**Scenario:** Decoder produces samples faster than mixer consumes

**Detection:** Buffer reports free space = 0

**Handling Strategy:**

```
1. Log warning at WARNING level:
   - Passage ID: {passage_id}
   - Buffer fill level: 100%
   - Message: "Buffer overflow - decoder paused"

2. Pause decoder (already specified [DBD-BUF-050])

3. No event emission (normal backpressure mechanism)

4. Resume decoder when buffer_fill < (capacity - playout_ringbuffer_headroom)
```

**Retry Policy:** N/A (normal flow control, not error)

**Note:** This is normal backpressure, not error condition. Included for completeness.

**Traceability:** [SPEC016 DBD-BUF-050] (Buffer backpressure)

---

## Audio Device Errors

### ERH-DEV-010: Device Lost (Disconnection)

**Scenarios:**
- Bluetooth headphones disconnected
- USB DAC unplugged
- HDMI display turned off
- Device reconfigured by another application

**Detection:** cpal stream callback returns error or stream drops

**Handling Strategy:**

```
1. Log error at ERROR level:
   - Device name: {device_name}
   - Device ID: {device_id}
   - Error: {cpal_error_message}

2. Emit event: AudioDeviceLost
   - device_name: String
   - device_id: String
   - timestamp: DateTime<Utc>

3. Immediate action:
   - Pause playback (preserve queue and position)
   - Store current playback position

4. Reconnection attempts:
   - Retry original device every 2 seconds
   - Maximum 15 attempts (30 seconds total)
   - Log each retry attempt at INFO level

5. If original device reconnects:
   - Resume playback from stored position
   - Emit AudioDeviceRestored event

6. If timeout (30 seconds):
   - Attempt fallback to system default device
   - If fallback succeeds:
     - Emit AudioDeviceFallback event
     - Resume playback
   - If fallback fails:
     - Remain paused
     - Emit AudioDeviceUnavailable event
     - Wait for user intervention (device selection API)
```

**Retry Policy:** 15 attempts at 2-second intervals (30s total)

**Traceability:** [REQ-AP-ERR-030] (Device disconnection handling)

### ERH-DEV-020: Device Configuration Error

**Scenarios:**
- Requested sample rate unsupported by device
- Channel count mismatch (stereo vs mono)
- Buffer size unsupported

**Detection:** cpal stream build fails with configuration error

**Handling Strategy:**

```
1. Log error at ERROR level:
   - Requested config: {sample_rate, channels, buffer_size}
   - Device name: {device_name}
   - Error: {config_error_message}

2. Emit event: AudioDeviceConfigError
   - device_name: String
   - requested_config: AudioConfig
   - error_message: String
   - timestamp: DateTime<Utc>

3. Fallback configuration attempts (in order):
   a. Try device default configuration
   b. Try 44.1kHz stereo (most common)
   c. Try 48kHz stereo
   d. Try mono if stereo unavailable

4. If any fallback succeeds:
   - Log warning about configuration mismatch
   - Update resampler for new output rate
   - Emit AudioDeviceConfigFallback event
   - Continue playback

5. If all fallbacks fail:
   - Log error at ERROR level
   - Pause playback
   - Emit AudioDeviceIncompatible event
   - Prompt user to select different device
```

**Retry Policy:** Try 4 fallback configurations before failing

**Traceability:** [REQ-AP-ERR-031] (Device compatibility)

---

## Queue Errors

### ERH-QUEUE-010: Invalid Queue Entry

**Scenarios:**
- Passage ID in queue but not in passages table
- File path is NULL or invalid
- Passage timing constraints violated (start >= end)

**Detection:** Queue validation on load or enqueue

**Handling Strategy:**

```
1. Log warning at WARNING level:
   - Queue entry ID: {queue_entry_id}
   - Passage ID: {passage_id}
   - Validation failure: {specific_issue}

2. Emit event: QueueValidationError
   - queue_entry_id: UUID
   - passage_id: Option<UUID>
   - validation_error: String
   - timestamp: DateTime<Utc>

3. Remove invalid entry from queue automatically

4. Continue queue processing with next entry
```

**Retry Policy:** None (validation errors are deterministic)

**Validation Frequency:**
- On playback start (validate entire queue)
- On enqueue (validate new entry)
- On queue load from database (validate all entries)

**Traceability:** [REQ-AP-ERR-040] (Queue validation)

### ERH-QUEUE-020: Chain Exhaustion

**Scenario:** All maximum_decode_streams chains allocated, cannot assign chain to new passage

**Detection:** Queue entry waiting for chain but all chains busy

**Handling Strategy:**

```
1. Log info at INFO level:
   - Queue entry ID: {queue_entry_id}
   - Available chains: 0 / {maximum_decode_streams}
   - Message: "Passage waiting for decoder chain"

2. No event emission (normal condition when queue deep)

3. Passage remains in queue without chain assignment

4. When chain becomes available:
   - Assign to highest-priority waiting passage
   - Begin decode

5. If queue depth > maximum_decode_streams for > 5 minutes:
   - Emit QueueDepthWarning event (user may want to skip forward)
```

**Retry Policy:** N/A (chain will become available eventually)

**Note:** This is normal queue depth management, not error condition. Future enhancement: dynamic chain allocation.

**Traceability:** [SPEC016 DBD-LIFECYCLE-050] (Chain exhaustion handling)

---

## Resampling Errors

### ERH-RSMP-010: Resampler Initialization Failure

**Scenarios:**
- Invalid source sample rate (0 Hz, negative, > 384kHz)
- Insufficient memory for resampler buffers
- rubato internal error

**Detection:** StatefulResampler::new() returns error

**Handling Strategy:**

```
1. Log error at ERROR level:
   - Passage ID: {passage_id}
   - Source rate: {source_sample_rate}
   - Target rate: {working_sample_rate}
   - Error: {rubato_error_message}

2. Emit event: ResamplingFailed
   - passage_id: UUID
   - source_rate: u32
   - target_rate: u32
   - error_message: String
   - timestamp: DateTime<Utc>

3. Check if resampling needed:
   - If source_rate == working_sample_rate:
     - Bypass resampler (use pass-through per [DBD-RSMP-020])
     - Continue playback

4. If resampling required but failed:
   - Skip passage
   - Remove from queue
   - Continue with next passage
```

**Retry Policy:** None (resampler init errors are deterministic)

**Traceability:** [REQ-AP-ERR-050] (Resampling error handling)

### ERH-RSMP-020: Resampler Runtime Failure

**Scenario:** Resampler process() call fails mid-passage

**Detection:** StatefulResampler::process() returns error

**Handling Strategy:**

```
1. Log error at ERROR level:
   - Passage ID: {passage_id}
   - Position: {current_sample_position}
   - Error: {rubato_error_message}

2. Emit event: ResamplingRuntimeError
   - passage_id: UUID
   - position_ms: u64
   - error_message: String
   - timestamp: DateTime<Utc>

3. Skip current passage

4. Release decoder chain

5. Continue with next passage
```

**Retry Policy:** None (runtime errors indicate corrupted state)

**Traceability:** [REQ-AP-ERR-051] (Runtime resampling errors)

---

## Timing Errors

### ERH-TIME-010: Tick Overflow

**Scenario:** Tick accumulation exceeds i64::MAX (extremely rare, ~1 million years of playback)

**Detection:** Tick arithmetic overflow check

**Handling Strategy:**

```
1. Log error at FATAL level:
   - Message: "Tick overflow detected - internal timing corrupted"
   - Current tick value: {tick_value}

2. Emit event: TimingSystemFailure
   - error_type: "tick_overflow"
   - timestamp: DateTime<Utc>

3. STOP all playback immediately

4. Reset tick counters to 0

5. Require user action to restart playback
```

**Retry Policy:** None (fatal condition requires restart)

**Note:** Defensive programming only - should never occur in practice.

**Traceability:** [SPEC017 SRC-ERR-010] (Tick overflow protection)

### ERH-TIME-020: Sample Position Mismatch

**Scenario:** Decoder reports sample position inconsistent with expected position

**Detection:** Position validation in decoder chain

**Handling Strategy:**

```
1. Log warning at WARNING level:
   - Passage ID: {passage_id}
   - Expected position: {expected_sample}
   - Actual position: {reported_sample}
   - Delta: {abs(expected - actual)}

2. If delta < 100 samples (< 3ms at 44.1kHz):
   - Log only, continue playback (minor drift acceptable)

3. If delta >= 100 samples:
   - Emit PositionDriftWarning event
   - Resync decoder position to expected
   - Continue playback

4. If delta > 1 second:
   - Log error at ERROR level
   - Skip current passage (position corrupted)
```

**Retry Policy:** N/A (position resync is automatic correction)

**Traceability:** [REQ-AP-ERR-060] (Position tracking accuracy)

---

## Resource Exhaustion

### ERH-RSRC-010: Out of Memory

**Scenario:** Memory allocation fails during buffer creation or decode

**Detection:** Allocation returns `OutOfMemory` error

**Handling Strategy:**

```
1. Log error at FATAL level:
   - Component: {component_name}
   - Requested bytes: {allocation_size}
   - Message: "Out of memory"

2. Emit event: SystemResourceExhausted
   - resource_type: "memory"
   - timestamp: DateTime<Utc>

3. Immediate action:
   - Release all non-essential buffers
   - Force garbage collection (if applicable)
   - Attempt allocation retry once

4. If retry succeeds:
   - Emit SystemResourceRecovered event
   - Continue with degraded buffer sizes

5. If retry fails:
   - STOP all playback
   - Emit SystemShutdownRequired event
   - Gracefully shut down component
```

**Retry Policy:** 1 retry after memory cleanup

**Traceability:** [REQ-AP-ERR-070] (Resource exhaustion)

### ERH-RSRC-020: File Handle Exhaustion

**Scenario:** Cannot open file due to OS file descriptor limit

**Detection:** File open returns `TooManyOpenFiles` error

**Handling Strategy:**

```
1. Log error at ERROR level:
   - Attempted file: {file_path}
   - Open file count: {current_open_files}

2. Emit event: FileHandleExhaustion
   - attempted_file: String
   - timestamp: DateTime<Utc>

3. Force close idle file handles

4. Retry file open once after cleanup

5. If retry succeeds, continue

6. If retry fails:
   - Skip current passage
   - Reduce maximum_decode_streams dynamically
   - Emit SystemDegradedMode event
```

**Retry Policy:** 1 retry after cleanup

**Traceability:** [REQ-AP-ERR-071] (File handle management)

---

## Event Definitions

### ERH-EVENT-010: Error Event Enum

Add the following variants to `WkmpEvent` enum in [SPEC011-event_system.md](SPEC011-event_system.md):

```rust
/// Error events for audio player failures
pub enum WkmpEvent {
    // ... existing events ...

    // Decode Errors
    PassageDecodeFailed {
        passage_id: Uuid,
        error_type: String,
        error_message: String,
        file_path: String,
        timestamp: DateTime<Utc>,
    },

    PassageUnsupportedCodec {
        passage_id: Uuid,
        file_path: String,
        codec_hint: Option<String>,
        timestamp: DateTime<Utc>,
    },

    PassagePartialDecode {
        passage_id: Uuid,
        expected_duration_ms: u64,
        actual_duration_ms: u64,
        timestamp: DateTime<Utc>,
    },

    PassageDecoderPanic {
        passage_id: Uuid,
        file_path: String,
        panic_message: String,
        timestamp: DateTime<Utc>,
    },

    // Buffer Errors
    BufferUnderrun {
        passage_id: Uuid,
        buffer_fill_percent: f32,
        timestamp: DateTime<Utc>,
    },

    BufferUnderrunRecovered {
        passage_id: Uuid,
        recovery_time_ms: u64,
        timestamp: DateTime<Utc>,
    },

    // Device Errors
    AudioDeviceLost {
        device_name: String,
        device_id: String,
        timestamp: DateTime<Utc>,
    },

    AudioDeviceRestored {
        device_name: String,
        timestamp: DateTime<Utc>,
    },

    AudioDeviceFallback {
        original_device: String,
        fallback_device: String,
        timestamp: DateTime<Utc>,
    },

    AudioDeviceUnavailable {
        timestamp: DateTime<Utc>,
    },

    AudioDeviceConfigError {
        device_name: String,
        requested_config: String, // JSON serialized AudioConfig
        error_message: String,
        timestamp: DateTime<Utc>,
    },

    AudioDeviceConfigFallback {
        device_name: String,
        fallback_config: String, // JSON serialized AudioConfig
        timestamp: DateTime<Utc>,
    },

    AudioDeviceIncompatible {
        device_name: String,
        timestamp: DateTime<Utc>,
    },

    // Queue Errors
    QueueValidationError {
        queue_entry_id: Uuid,
        passage_id: Option<Uuid>,
        validation_error: String,
        timestamp: DateTime<Utc>,
    },

    QueueDepthWarning {
        queue_depth: usize,
        available_chains: usize,
        timestamp: DateTime<Utc>,
    },

    // Resampling Errors
    ResamplingFailed {
        passage_id: Uuid,
        source_rate: u32,
        target_rate: u32,
        error_message: String,
        timestamp: DateTime<Utc>,
    },

    ResamplingRuntimeError {
        passage_id: Uuid,
        position_ms: u64,
        error_message: String,
        timestamp: DateTime<Utc>,
    },

    // Timing Errors
    TimingSystemFailure {
        error_type: String,
        timestamp: DateTime<Utc>,
    },

    PositionDriftWarning {
        passage_id: Uuid,
        expected_position_ms: u64,
        actual_position_ms: u64,
        delta_ms: i64,
        timestamp: DateTime<Utc>,
    },

    // Resource Exhaustion
    SystemResourceExhausted {
        resource_type: String, // "memory", "file_handles", "cpu"
        timestamp: DateTime<Utc>,
    },

    SystemResourceRecovered {
        resource_type: String,
        timestamp: DateTime<Utc>,
    },

    FileHandleExhaustion {
        attempted_file: String,
        timestamp: DateTime<Utc>,
    },

    SystemDegradedMode {
        reason: String,
        timestamp: DateTime<Utc>,
    },

    SystemShutdownRequired {
        reason: String,
        timestamp: DateTime<Utc>,
    },
}
```

**Traceability:** [SPEC011 EVT-DEF-XXX] (Event definitions update required)

---

## Logging Requirements

### ERH-LOG-010: Log Levels

| Error Category | Default Level | Adjustable | Notes |
|----------------|---------------|------------|-------|
| File Read Failures | ERROR | No | Always actionable |
| Unsupported Codec | WARNING | Yes | Expected for some files |
| Partial Decode | WARNING | Yes | May be acceptable |
| Decoder Panic | ERROR | No | Always indicates bug |
| Buffer Underrun | WARNING | Yes | May be environmental |
| Device Lost | ERROR | No | Significant user impact |
| Device Config Error | ERROR | Yes | May be expected |
| Queue Validation | WARNING | Yes | Auto-correctable |
| Resampling Failed | ERROR | No | Blocks playback |
| Position Drift < 100 samples | INFO | Yes | Minor, expected |
| Position Drift >= 100 samples | WARNING | Yes | Investigate |
| Resource Exhaustion | FATAL | No | System-wide issue |

### ERH-LOG-020: Log Message Format

All error logs MUST include:
- **Timestamp:** ISO 8601 with timezone
- **Level:** ERROR, WARNING, INFO per table above
- **Component:** Which subsystem logged error (Decoder, Mixer, Resampler, etc.)
- **Error ID:** Unique identifier for error instance (for correlation)
- **Context:** Passage ID, file path, or other relevant identifiers
- **Message:** Human-readable error description
- **Details:** Technical details (error codes, stack traces if available)

**Example:**
```
2025-10-25T14:32:15.234Z ERROR [Decoder] error_id=a3f2b1c9 passage_id=8f7e6d5c file_path=/music/track.mp3 message="File read error: Permission denied" errno=EACCES
```

### ERH-LOG-030: Structured Logging

Use structured logging (e.g., `tracing` crate) to enable:
- Machine-parseable log analysis
- Error aggregation and metrics
- Correlation across components
- Query by passage_id, file_path, error_type

**Traceability:** [IMPL002 CONV-LOG-XXX] (Logging conventions update required)

---

## User Notification Strategy

### ERH-NOTIFY-010: Notification Tiers

| Severity | UI Treatment | Timing | Persistence |
|----------|-------------|--------|-------------|
| FATAL | Modal dialog | Immediate | Blocks interaction |
| ERROR (playback-blocking) | Toast notification | Immediate | 10 seconds |
| ERROR (skippable) | Status bar | Immediate | Until acknowledged |
| WARNING | Status bar | Batched (max 5s delay) | Until acknowledged |
| INFO | Event log only | N/A | Not shown in UI |

### ERH-NOTIFY-020: Batching Rules

**Multiple errors of same type:**
- First error: Immediate notification
- 2nd-5th errors within 60 seconds: Batch into single notification
- 6+ errors within 60 seconds: Single notification "Multiple decode failures (6)"

**Mixed error types:**
- Show highest-severity error immediately
- Queue others for delayed display (5 second intervals)

### ERH-NOTIFY-030: Error Context in UI

UI notifications SHOULD include:
- Error type (user-friendly name)
- Affected passage (song title if available, otherwise file name)
- Actionable next step ("Skipped to next track", "Waiting for device...")
- Link to detailed error log (for debugging)

**Traceability:** [SPEC-UI-ERR-XXX] (UI error presentation, to be specified in wkmp-ui)

---

## Testing Requirements

### ERH-TEST-010: Error Injection Framework

Implement error injection capability for testing:
- File I/O failures (ENOENT, EACCES, EIO)
- Decoder failures (simulated codec errors)
- Buffer underrun conditions (slow decoder simulation)
- Device disconnection (mock cpal stream drops)
- Resource exhaustion (memory allocation failure simulation)

**Method:** Feature flag `error-injection` enables test-only error triggers

### ERH-TEST-020: Error Recovery Tests

Required test scenarios:
1. **Decode Failure Recovery:** File unreadable → skip to next
2. **Buffer Underrun Recovery:** Underrun → emergency refill → resume
3. **Device Lost Recovery:** Device disconnected → 30s wait → fallback
4. **Queue Validation:** Invalid entries → auto-remove → continue
5. **Partial Decode:** Truncated file → partial playback if >50%
6. **Resource Exhaustion:** OOM → cleanup → retry → succeed or fail gracefully

### ERH-TEST-030: Event Emission Verification

For each error scenario, verify:
- Correct event type emitted
- Event payload contains required fields
- Event delivered via SSE to subscribers
- Event logged to persistent log

**Traceability:** [PLAN-XXX-TEST-ERR-XXX] (Error handling test plan, to be created)

---

## Graceful Degradation

### ERH-DEGRADE-010: Reduced Functionality Modes

When errors prevent full operation, system operates in degraded modes:

**Mode 1: Reduced Chain Count**
- Trigger: File handle exhaustion ([ERH-RSRC-020])
- Action: Dynamically reduce maximum_decode_streams from 12 to 6 or 3
- Effect: Longer pre-load time, reduced queue lookahead
- Recovery: Reset to full chain count after 5 minutes without errors

**Mode 2: Single Passage Playback**
- Trigger: Persistent buffer underruns (3+ in 60 seconds)
- Action: Disable crossfading, play one passage at a time
- Effect: No crossfade overlap, gaps between passages
- Recovery: Re-enable after 10 minutes of stable playback

**Mode 3: Fallback Audio Device**
- Trigger: Primary device lost and not recovered ([ERH-DEV-010])
- Action: Switch to system default device
- Effect: Possible audio quality degradation
- Recovery: Manual device re-selection or primary device auto-reconnect

### ERH-DEGRADE-020: Minimal Viable Functionality

Even under severe errors, preserve:
- Queue integrity (passage order maintained)
- Playback position (can resume from last position)
- User control (pause, skip, volume still functional)
- Event emission (errors still reported to UI)

**Traceability:** [REQ-AP-DEGRADE-XXX] (Graceful degradation requirements, to be added to REQ001)

---

## Implementation Guidance

### ERH-IMPL-010: Error Type Design

Use Rust `Result<T, E>` idiomatically:
- Return `Result` from all fallible operations
- Define custom error enums per module (DecoderError, BufferError, etc.)
- Implement `std::error::Error` trait for all error types
- Use `thiserror` crate for error derive macros

**Example:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum DecoderError {
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Unsupported codec: {codec}")]
    UnsupportedCodec { codec: String },

    #[error("I/O error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("Decoder panic: {message}")]
    DecoderPanic { message: String },
}
```

### ERH-IMPL-020: Error Context

Use `anyhow` crate for error context propagation:
```rust
use anyhow::{Context, Result};

fn decode_passage(path: &Path) -> Result<DecodedAudio> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open audio file: {}", path.display()))?;

    let decoder = Decoder::new(file)
        .context("Failed to initialize decoder")?;

    // ...
}
```

### ERH-IMPL-030: Async Error Handling

For async operations, use `tokio::select!` with timeout for error recovery:
```rust
use tokio::time::{timeout, Duration};

async fn decode_with_timeout(passage_id: Uuid) -> Result<()> {
    match timeout(Duration::from_secs(30), decode_passage()).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => {
            emit_event(PassageDecodeFailed { passage_id, error: e.to_string() });
            Err(e)
        }
        Err(_timeout) => {
            emit_event(DecodeTimeout { passage_id });
            Err(anyhow!("Decode timeout after 30 seconds"))
        }
    }
}
```

### ERH-IMPL-040: Panic Handling

Wrap decoder threads with panic catch:
```rust
use std::panic::{catch_unwind, AssertUnwindSafe};

fn run_decoder_with_panic_guard(passage_id: Uuid) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        run_decoder(passage_id)
    }));

    match result {
        Ok(Ok(())) => { /* success */ }
        Ok(Err(e)) => { /* normal error */ }
        Err(panic_payload) => {
            let panic_msg = panic_payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic_payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "Unknown panic".to_string());

            emit_event(PassageDecoderPanic {
                passage_id,
                panic_message: panic_msg,
            });

            // Restart decoder worker
            restart_decoder_chain();
        }
    }
}
```

**Traceability:** [IMPL002 CONV-ERR-XXX] (Error handling conventions, to be added)

---

## Requirement Traceability

### New Requirements to Add to REQ001

The following requirements should be added to [REQ001-requirements.md](REQ001-requirements.md):

**Error Handling Requirements:**
- **[REQ-AP-ERR-010]** Decode errors SHALL skip passage and continue with next
- **[REQ-AP-ERR-011]** Unsupported codecs SHALL be marked to prevent re-enqueueing
- **[REQ-AP-ERR-012]** Partial decode ≥50% SHALL allow passage playback
- **[REQ-AP-ERR-013]** Decoder panics SHALL be caught and recovered
- **[REQ-AP-ERR-020]** Buffer underruns SHALL attempt emergency refill with 500ms timeout
- **[REQ-AP-ERR-030]** Device disconnection SHALL retry for 30 seconds before fallback
- **[REQ-AP-ERR-031]** Device config errors SHALL attempt 4 fallback configurations
- **[REQ-AP-ERR-040]** Invalid queue entries SHALL be auto-removed with logging
- **[REQ-AP-ERR-050]** Resampling init failures SHALL skip passage or bypass if same rate
- **[REQ-AP-ERR-051]** Resampling runtime errors SHALL skip passage
- **[REQ-AP-ERR-060]** Position drift <100 samples SHALL be auto-corrected
- **[REQ-AP-ERR-070]** Resource exhaustion SHALL attempt cleanup and retry once
- **[REQ-AP-ERR-071]** File handle exhaustion SHALL reduce chain count dynamically

**Degradation Requirements:**
- **[REQ-AP-DEGRADE-010]** System SHALL preserve queue integrity under all error conditions
- **[REQ-AP-DEGRADE-020]** System SHALL preserve playback position through recoverable errors
- **[REQ-AP-DEGRADE-030]** System SHALL maintain user control (pause, skip, volume) in degraded modes

**Event Requirements:**
- **[REQ-AP-EVENT-ERR-010]** All errors SHALL emit appropriate WkmpEvent variants
- **[REQ-AP-EVENT-ERR-020]** Error events SHALL include timestamp, passage_id, and error details

**Logging Requirements:**
- **[REQ-AP-LOG-ERR-010]** All errors SHALL be logged at appropriate severity level
- **[REQ-AP-LOG-ERR-020]** Error logs SHALL include structured context for debugging

---

## Change History

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0 | 2025-10-25 | System Architecture | Initial specification addressing error handling gap identified in requirements review |

---

## Approval

**Technical Review:** [Pending]
**Architecture Review:** [Pending]
**Implementation Authorization:** [Pending]

**Approval Criteria:**
- [ ] Error taxonomy complete and consistent
- [ ] All error scenarios identified and handling specified
- [ ] Event definitions complete and integrated with SPEC011
- [ ] Logging requirements clear and actionable
- [ ] Testing strategy comprehensive
- [ ] Traceability to requirements established
- [ ] Integration with existing architecture verified

---

**Document Version:** 1.0
**Created:** 2025-10-25
**Last Updated:** 2025-10-25
**Status:** Approved
**Tier:** 2 - Design Specification
**Document Code:** ERH (Error Handling)
**Maintained By:** Audio Player team, technical lead

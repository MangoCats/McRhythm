# WKMP Timing Terminology

**📝 TIER 2 - DESIGN SPECIFICATION**

Defines timing types and terminology used throughout WKMP. Reference this document for correct timing terminology.

> **Related Documentation:** [Requirements](REQ001-requirements.md) | [Coding Conventions](IMPL002-coding_conventions.md)

---

## Overview

WKMP uses four distinct timing types. **Never use ambiguous terms like "wall clock time"** - always specify the timing type explicitly.

---

## Timing Types

### 1. Calendar Time (System Timestamps)

**Purpose:** Absolute timestamps for external coordination and audit trails

**Implementation:**
- Rust: `std::time::SystemTime::now()` or `chrono::Utc::now()`
- Database: Unix milliseconds (INTEGER)

**Use Cases:**
- API authentication timestamps (hash verification)
- Event timestamps for UI correlation (`WkmpEvent::timestamp`)
- Database audit columns (`start_timestamp_ms`, `stop_timestamp_ms`)
- SSE event timestamps

**Characteristics:**
- Can go backwards with NTP adjustments
- Non-monotonic
- Suitable for external coordination

**Examples:**
```rust
// API timestamp validation
let timestamp = chrono::Utc::now();

// Event emission
WkmpEvent::PassageStarted {
    passage_id,
    timestamp: chrono::Utc::now(),  // Calendar time for UI correlation
}
```

---

### 2. Monotonic Elapsed Time (Real-Time Intervals)

**Purpose:** Measuring durations and scheduling intervals

**Implementation:**
- Rust: `std::time::Instant` with `elapsed()` method
- Always advances forward, immune to clock adjustments

**Use Cases:**
- Performance measurements (decode latency, API response time)
- Scheduling intervals (`tokio::sleep`, `output_refill_period`, `decode_work_period`)
- Timeout calculations
- Non-callback timing instrumentation

**Characteristics:**
- Always monotonically increasing
- Unaffected by NTP/clock adjustments
- Measures real-time passage
- **Fast but NOT real-time safe** (see CO-257)

**Examples:**
```rust
// Performance measurement
let start = Instant::now();
decode_audio(&file);
let duration = start.elapsed();
info!("Decode took {:?}", duration);

// Interval scheduling
tokio::time::sleep(Duration::from_millis(90)).await;  // Uses monotonic time
```

**Important:** Even `Instant::elapsed()` is NOT allowed in audio callbacks per [IMPL002 CO-257](IMPL002-coding_conventions.md#co-256-real-time-audio-constraints).

---

### 3. Audio Timeline Time (Ticks)

**Purpose:** Sample-accurate audio positions and durations

**Implementation:**
- Type: `i64` (signed for potential negative offsets)
- Precision: 1 tick = 1/28,224,000 second ([SPEC017 SRC-TICK-020](SPEC017-sample_rate_conversion.md))
- Conversion: `ticks = samples * (28,224,000 / sample_rate)`

**Use Cases:**
- Passage start/end positions (`start_time_ticks`, `end_time_ticks`)
- Crossfade timing points
- Playback position tracking
- Audio duration calculations in database

**Characteristics:**
- Sample-accurate precision
- Independent of real-time drift
- Represents audio timeline, not calendar time
- Can drift from real-time during playback (jitter, underruns)

**Examples:**
```rust
// Convert milliseconds to ticks
let start_ticks = wkmp_common::timing::ms_to_ticks(start_ms as i64);

// Calculate audio duration
let duration_ticks = end_time_ticks - start_time_ticks;
```

---

### 4. Audio Pipeline Samples/Frames

**Purpose:** Internal audio processing units

**Implementation:**
- Type: `usize` for sample counts, `AudioFrame` structs
- Native processing unit (no conversion overhead)

**Use Cases:**
- Ring buffer positions
- Decoder output counting
- Mixer frame processing
- Internal pipeline state

**Characteristics:**
- Lowest-level audio representation
- Sample-rate specific (44.1kHz vs 48kHz have different sample counts for same duration)
- Convert to ticks for sample-rate-independent representation

---

## Deprecated/Ambiguous Terms

### ❌ "Wall Clock Time" - DO NOT USE

**Problem:** Ambiguous - could mean calendar time, monotonic time, or audio timeline

**Replacements:**
- "Calendar time" or "system timestamp" → Type 1
- "Monotonic elapsed time" or "real-time interval" → Type 2
- "Audio timeline" or "ticks" → Type 3

### ❌ "Real-Time" - CONTEXT DEPENDENT

**Problem:** "Real-time" has two meanings:
1. **Real-time audio constraints** (CO-257): Code that must complete within strict deadlines
2. **Real-time interval measurement**: Monotonic elapsed time (Type 2)

**Solution:** Be explicit:
- "Real-time safe code" → Refers to CO-257 constraints
- "Monotonic elapsed time" → Refers to Type 2 timing

---

## Terminology Usage Guidelines

### Performance Measurements
```rust
// ✅ CORRECT
let start = Instant::now();
let duration = start.elapsed();
info!("Operation took {:?}", duration);

// ❌ WRONG
let start = SystemTime::now();  // Can go backwards!
```

### Event Timestamps
```rust
// ✅ CORRECT - Calendar time for external correlation
WkmpEvent::BufferUnderrun {
    timestamp: chrono::Utc::now(),
}

// ❌ WRONG - Instant is meaningless across processes
timestamp: Instant::now(),  // Cannot serialize/compare across restarts
```

### Audio Positions
```rust
// ✅ CORRECT - Ticks for sample-accurate positions
passage.start_time_ticks = wkmp_common::timing::ms_to_ticks(start_ms);

// ❌ WRONG - Milliseconds lose precision
passage.start_time_ms = start_ms;  // Not sample-accurate
```

### Documentation Terminology
```markdown
<!-- ✅ CORRECT -->
The mixer checks the ring buffer every 90ms of monotonic elapsed time.

<!-- ❌ WRONG - Ambiguous -->
The mixer checks the ring buffer every 90ms of wall clock time.
```

---

## Reference Table

| Timing Type | Rust Type | Use Case | Can Go Backwards? | Real-Time Safe? |
|-------------|-----------|----------|-------------------|-----------------|
| Calendar Time | `SystemTime`, `chrono::Utc` | Timestamps, API auth | Yes (NTP) | No (system call) |
| Monotonic Elapsed | `Instant` | Intervals, performance | No | No (not in callback) |
| Audio Timeline | `i64` ticks | Positions, durations | No | Yes (just math) |
| Pipeline Samples | `usize`, `AudioFrame` | Internal processing | No | Yes (just pointers) |

---

## See Also

- **[IMPL002 CO-257](IMPL002-coding_conventions.md#co-256-real-time-audio-constraints)**: Real-time audio constraints (prohibited operations)
- **[SPEC017](SPEC017-sample_rate_conversion.md)**: Tick conversion and audio timeline
- **[SPEC016](SPEC016-decoder_buffer_design.md)**: Operating parameters using monotonic elapsed time
- **[IMPL001](IMPL001-database_schema.md)**: Database timing field types

---

## Change History

| Date | Version | Changes | Reason |
|------|---------|---------|--------|
| 2025-10-26 | 1.0 | Initial specification | Centralize timing terminology, eliminate "wall clock time" ambiguity, prevent future SystemTime misuse in audio pipeline |

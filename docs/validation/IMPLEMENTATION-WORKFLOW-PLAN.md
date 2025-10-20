# WKMP Audio Player Reimplementation Workflow

**Project:** Reimplement Audio Player per SPEC016/SPEC017 Design Principles
**Goal:** Fast, reliable playback with minimal startup latency
**Status:** Planning Phase
**Date:** 2025-10-19

---

## Executive Summary

### Objective

Reimplement the WKMP audio player (`wkmp-ap`) to align with the improved design principles documented in SPEC016-decoder_buffer_design.md and SPEC017-sample_rate_conversion.md, with particular emphasis on **fast playback startup** (audio begins playing as soon as possible after enqueue).

### Current Implementation Issues

Based on documentation analysis, the current implementation likely suffers from:

1. **Slow Playback Startup**
   - Decoder pool coordination overhead
   - Excessive buffer pre-filling requirements
   - Serial bottlenecks in initialization
   - Inefficient decode-and-skip for passage start points

2. **Timing Precision Issues**
   - REAL seconds (floating-point) timing in database
   - Accumulated rounding errors in long passages
   - Inconsistent crossfade timing

3. **Resource Inefficiency**
   - 2-thread parallel decode pool (cache thrashing)
   - Runtime fade curve application (per-sample overhead)
   - Excessive memory allocation

### Target Improvements

**Primary Goal:** Fast Playback Startup
- **Current:** Unknown baseline (to be measured)
- **Target:** <100ms from enqueue to first audio sample output
- **Stretch Goal:** <50ms for passages already decoded

**Secondary Goals:**
- Sample-accurate timing (zero rounding errors)
- Smooth crossfades (no clicks, pops, or gaps)
- Low CPU usage (<5% on modern CPU for stereo 44.1kHz)
- Predictable memory usage (~60MB for 12 buffers)

---

## Multi-Agent Workflow Overview

### Phase 1: Analysis and Baseline Measurement (3-5 days)

**Goal:** Understand current implementation gaps and establish performance baselines

**Agents:**
1. **Agent 1A: Code Inventory Agent** - Map existing codebase
2. **Agent 1B: Gap Analysis Agent** - Compare code vs SPEC016/SPEC017
3. **Agent 1C: Performance Profiling Agent** - Measure current performance
4. **Agent 1D: Test Coverage Agent** - Assess existing test quality

**Deliverables:**
- Code inventory with file structure and responsibilities
- Gap analysis report: current vs desired state
- Performance baseline: startup time, CPU usage, memory usage
- Test coverage report with identified gaps

---

### Phase 2: Test-Driven Design (5-7 days)

**Goal:** Design comprehensive test suite BEFORE implementation

**Agents:**
1. **Agent 2A: Unit Test Design Agent** - Design unit tests for core components
2. **Agent 2B: Integration Test Design Agent** - Design end-to-end scenarios
3. **Agent 2C: Performance Test Design Agent** - Design performance benchmarks
4. **Agent 2D: Property-Based Test Agent** - Design property tests for invariants

**Deliverables:**
- Unit test specifications for all core components
- Integration test scenarios (playback, crossfade, enqueue, skip)
- Performance benchmarks (startup time, CPU, memory)
- Property tests (timing precision, buffer safety, thread safety)

---

### Phase 3: Database Migration (2-3 days)

**Goal:** Migrate to INTEGER ticks timing system (T1-TIMING-001)

**Agents:**
1. **Agent 3A: Migration Script Generator** - Create migration SQL
2. **Agent 3B: Data Validator** - Verify migration accuracy
3. **Agent 3C: API Converter** - Implement ms ↔ ticks conversion layer

**Deliverables:**
- Migration script: `migrations/NNN-tick_based_timing.sql`
- Rollback script
- API conversion utilities: `wkmp-common/src/conversions.rs`
- Migration validation tests

---

### Phase 4: Core Playback Engine Reimplementation (10-15 days)

**Goal:** Implement decoder-buffer chain per SPEC016

**Sub-Phase 4A: Serial Decode Execution (3-4 days)**

**Agents:**
- **Agent 4A1: Decoder Pool Refactor** - Replace 2-thread pool with serial execution
- **Agent 4A2: Priority Queue Implementation** - Implement decode priority scheduling
- **Agent 4A3: Decode-and-Skip Optimizer** - Optimize skip performance for fast startup

**Deliverables:**
- `wkmp-ap/src/playback/decoder_pool.rs` (refactored)
- `wkmp-ap/src/playback/decode_queue.rs` (new)
- Unit tests for serial decode, priority scheduling, skip optimization

**Sub-Phase 4B: Pre-Buffer Fade Application (2-3 days)**

**Agents:**
- **Agent 4B1: Fade Handler Implementation** - Apply fades before buffering
- **Agent 4B2: Fade Curve Generator** - Implement 5 fade curve types
- **Agent 4B3: Fade Timing Validator** - Verify sample-accurate fade points

**Deliverables:**
- `wkmp-ap/src/playback/fade_handler.rs` (refactored)
- `wkmp-ap/src/playback/fade_curves.rs` (new)
- Unit tests for fade curves, timing, pre-buffer application

**Sub-Phase 4C: Buffer Management (3-4 days)**

**Agents:**
- **Agent 4C1: Ring Buffer Implementation** - Playout ring buffer per DBD-PARAM-070
- **Agent 4C2: Buffer State Machine** - Implement 5 buffer states [DBD-BUF-020 through DBD-BUF-060]
- **Agent 4C3: Buffer Event System** - Integrate with SPEC011 event system

**Deliverables:**
- `wkmp-ap/src/playback/ring_buffer.rs` (refactored)
- `wkmp-ap/src/playback/buffer_states.rs` (new)
- Unit tests for buffer lifecycle, state transitions, event emission

**Sub-Phase 4D: Mixer and Crossfade (2-4 days)**

**Agents:**
- **Agent 4D1: Crossfade Mixer** - Implement DBD-MIX-040 overlap behavior
- **Agent 4D2: Tick Timing Integration** - Use INTEGER ticks throughout mixer
- **Agent 4D3: Crossfade Validator** - Verify smooth transitions, no clicks/pops

**Deliverables:**
- `wkmp-ap/src/playback/mixer.rs` (refactored)
- Unit tests for crossfade timing, volume calculations, smooth transitions
- Integration tests for crossfade scenarios

---

### Phase 5: Fast Startup Optimization (3-5 days)

**Goal:** Achieve <100ms playback startup time

**Agents:**
1. **Agent 5A: Startup Path Profiler** - Identify bottlenecks in enqueue → audio out
2. **Agent 5B: Lazy Initialization Agent** - Defer non-critical work until after audio starts
3. **Agent 5C: Decode Prefetch Agent** - Predictive decode scheduling
4. **Agent 5D: Buffer Preload Agent** - Optimize initial buffer fill strategy

**Deliverables:**
- Startup path optimization recommendations
- Lazy initialization implementation
- Prefetch heuristics
- Performance benchmarks showing <100ms startup

**Key Optimizations:**
1. **Minimal Initial Buffer:** Start playback with small buffer (e.g., 0.5s instead of 15s)
2. **Background Buffer Fill:** Continue decoding while playing first samples
3. **Decode-and-Skip Optimization:** Fast seek to passage start_time using index frames
4. **Thread Coordination:** Minimize mutex contention on critical path
5. **Memory Preallocation:** Avoid allocations during enqueue → playback

---

### Phase 6: Integration and End-to-End Testing (5-7 days)

**Goal:** Validate complete system with real-world scenarios

**Agents:**
1. **Agent 6A: Integration Test Executor** - Run all integration tests
2. **Agent 6B: Real-World Scenario Validator** - Test with real music files
3. **Agent 6C: Performance Regression Detector** - Compare vs baseline
4. **Agent 6D: Stress Test Agent** - Test edge cases, error conditions

**Deliverables:**
- Integration test suite execution report
- Real-world validation report (100+ music files, various formats)
- Performance comparison: before vs after
- Stress test results (rapid enqueue, skip, queue manipulation)

**Test Scenarios:**
1. **Basic Playback:** Single passage plays smoothly
2. **Crossfade:** Smooth transition between two passages
3. **Rapid Enqueue:** Enqueue 10 passages in <1 second
4. **Skip During Crossfade:** Skip while crossfade is active
5. **Queue Manipulation:** Add/remove passages during playback
6. **Long Playback:** 1 hour continuous playback (memory leak detection)
7. **Error Recovery:** Corrupted file, missing file, decode error
8. **Sample Rate Variations:** 44.1kHz, 48kHz, 96kHz, 192kHz sources
9. **Format Variations:** MP3, FLAC, OGG, AAC, WAV
10. **Edge Cases:** 0-length passage, passage at file end, passage spanning entire file

---

### Phase 7: Performance Validation and Benchmarking (2-3 days)

**Goal:** Prove performance improvements with quantitative metrics

**Agents:**
1. **Agent 7A: Startup Time Benchmark** - Measure enqueue → audio latency
2. **Agent 7B: CPU Usage Profiler** - Measure CPU usage during playback
3. **Agent 7C: Memory Profiler** - Measure memory usage and allocation patterns
4. **Agent 7D: Timing Precision Validator** - Verify sample-accurate timing

**Deliverables:**
- Performance benchmark report
- CPU usage profile (flame graphs)
- Memory usage analysis
- Timing precision validation (tick-level accuracy confirmed)

**Success Criteria:**
- ✅ Startup time: <100ms (target), <50ms (stretch)
- ✅ CPU usage: <5% during stereo 44.1kHz playback
- ✅ Memory usage: ~60MB for 12 buffers (predictable)
- ✅ Timing precision: Zero rounding errors, sample-accurate
- ✅ Crossfade quality: No clicks, pops, or gaps detected
- ✅ Smooth playback: No underruns, no buffer starvation

---

## Detailed Agent Specifications

### Phase 1: Analysis and Baseline Measurement

#### Agent 1A: Code Inventory Agent

**Task:** Map existing wkmp-ap codebase structure

**Actions:**
1. Scan `wkmp-ap/src/` directory recursively
2. Identify all modules and their responsibilities
3. Map dependencies between modules
4. Identify external crate dependencies (symphonia, rubato, cpal)
5. Generate module dependency graph

**Output:** `IMPL-ANALYSIS-001-code-inventory.json`

```json
{
  "modules": [
    {
      "path": "wkmp-ap/src/playback/decoder_pool.rs",
      "responsibility": "Manages 2-thread decode pool",
      "key_types": ["DecoderPool", "DecoderThread"],
      "dependencies": ["symphonia", "tokio"],
      "lines_of_code": 450,
      "test_coverage": "65%"
    },
    // ... more modules
  ],
  "dependency_graph": { /* mermaid graph */ }
}
```

---

#### Agent 1B: Gap Analysis Agent

**Task:** Compare current implementation vs SPEC016/SPEC017 requirements

**Actions:**
1. Read SPEC016-decoder_buffer_design.md
2. Read SPEC017-sample_rate_conversion.md
3. For each requirement ID ([DBD-XXX-NNN], [SRC-XXX-NNN]):
   a. Search codebase for implementation
   b. Classify as: IMPLEMENTED, PARTIALLY_IMPLEMENTED, MISSING, INCORRECT
   c. Document gap with severity
4. Generate gap analysis report

**Output:** `IMPL-ANALYSIS-002-gap-analysis.json`

```json
{
  "total_requirements": 122,
  "implemented": 58,
  "partially_implemented": 34,
  "missing": 20,
  "incorrect": 10,
  "gaps": [
    {
      "requirement_id": "DBD-DEC-040",
      "requirement": "Serial decode execution",
      "current_state": "INCORRECT",
      "current_implementation": "2-thread parallel decode pool in decoder_pool.rs:45",
      "gap_description": "Current implementation uses parallel threads, violates serial execution requirement",
      "severity": "HIGH",
      "effort_estimate": "3-4 days",
      "affected_files": ["decoder_pool.rs", "playback_controller.rs"]
    },
    {
      "requirement_id": "SRC-DB-011",
      "requirement": "start_time as INTEGER ticks",
      "current_state": "INCORRECT",
      "current_implementation": "REAL seconds in passages table",
      "gap_description": "Database uses REAL, code uses f64, violates INTEGER ticks requirement",
      "severity": "CRITICAL",
      "effort_estimate": "2-3 days",
      "affected_files": ["models/passage.rs", "migrations/*"]
    },
    // ... 30 total gaps
  ]
}
```

---

#### Agent 1C: Performance Profiling Agent

**Task:** Establish performance baselines for current implementation

**Actions:**
1. Run `cargo run --package wkmp-ap --release` in test mode
2. Measure:
   a. **Startup time:** Time from enqueue API call to first audio sample output
   b. **CPU usage:** During steady-state playback
   c. **Memory usage:** Peak and average during playback
   d. **Crossfade quality:** Measure for clicks/pops (frequency domain analysis)
3. Run 10 iterations for statistical significance
4. Generate performance baseline report

**Test Setup:**
```rust
// Performance test harness
fn benchmark_startup_time() {
    let start = Instant::now();

    // Enqueue passage
    api::enqueue_passage(test_file_path, start_time, end_time);

    // Wait for first audio callback with samples
    let first_audio = wait_for_audio_output();

    let startup_time = start.elapsed();
    println!("Startup time: {:?}", startup_time);
}
```

**Output:** `IMPL-ANALYSIS-003-performance-baseline.json`

```json
{
  "startup_time": {
    "mean_ms": 450,
    "std_dev_ms": 120,
    "min_ms": 280,
    "max_ms": 680,
    "samples": 10
  },
  "cpu_usage": {
    "mean_percent": 8.5,
    "peak_percent": 15.2
  },
  "memory_usage": {
    "peak_mb": 120,
    "average_mb": 85
  },
  "crossfade_quality": {
    "clicks_detected": 3,
    "pops_detected": 1,
    "rms_error": 0.002
  }
}
```

---

#### Agent 1D: Test Coverage Agent

**Task:** Assess existing test quality and coverage

**Actions:**
1. Run `cargo tarpaulin --workspace` for code coverage
2. Analyze existing tests in `wkmp-ap/tests/`
3. Identify untested code paths
4. Classify test types: unit, integration, performance
5. Generate test coverage report

**Output:** `IMPL-ANALYSIS-004-test-coverage.json`

```json
{
  "overall_coverage": "62%",
  "uncovered_modules": [
    {
      "module": "playback/decoder_pool.rs",
      "coverage": "45%",
      "critical_uncovered": [
        "decoder thread panic recovery",
        "decode-and-skip error handling",
        "buffer overflow conditions"
      ]
    }
  ],
  "test_types": {
    "unit_tests": 45,
    "integration_tests": 12,
    "performance_tests": 0
  },
  "gaps": [
    "No performance benchmarks",
    "No crossfade integration tests",
    "No stress tests for rapid enqueue",
    "No error recovery tests"
  ]
}
```

---

### Phase 2: Test-Driven Design

#### Agent 2A: Unit Test Design Agent

**Task:** Design comprehensive unit test suite

**Actions:**
1. For each module in gap analysis with gaps:
   a. Design unit tests for all public functions
   b. Design unit tests for error conditions
   c. Design unit tests for edge cases
2. Generate test specifications in BDD format (Given-When-Then)
3. Create test skeleton files

**Output:** `IMPL-TESTS-001-unit-test-specs.md`

**Example:**

```markdown
## Module: decoder_pool.rs (refactored to serial_decoder.rs)

### Test: Serial decode execution

**Requirement:** [DBD-DEC-040] Only one decoder runs at a time

**Given:** Two passages enqueued simultaneously
**When:** Decoding begins
**Then:** Only one decoder thread is active (verified via thread count)
**And:** Second passage waits in priority queue

**Test Code Skeleton:**
```rust
#[test]
fn test_serial_decode_execution() {
    let decoder = SerialDecoder::new();
    decoder.enqueue(passage_1);
    decoder.enqueue(passage_2);

    // Start decoding
    decoder.start();

    // Verify only one thread active
    assert_eq!(decoder.active_threads(), 1);

    // Verify second passage in queue
    assert!(decoder.queue_contains(passage_2.id));
}
```

### Test: Decode-and-skip performance

**Requirement:** [DBD-DEC-050] Decode from file start, skip to passage start

**Given:** Passage with start_time = 60 seconds into file
**When:** Decode begins
**Then:** Decoder completes skip in <100ms
**And:** First sample output is at exactly start_time (tick-accurate)

**Test Code Skeleton:**
```rust
#[test]
fn test_decode_and_skip_performance() {
    let passage = Passage {
        start_time_ticks: 60 * 28_224_000, // 60 seconds in ticks
        // ...
    };

    let start = Instant::now();
    let first_sample = decoder.decode_passage(&passage).next().unwrap();
    let skip_time = start.elapsed();

    assert!(skip_time < Duration::from_millis(100));
    assert_eq!(first_sample.position_ticks, passage.start_time_ticks);
}
```
```

---

#### Agent 2B: Integration Test Design Agent

**Task:** Design end-to-end integration tests

**Actions:**
1. Design test scenarios based on real-world usage
2. Include error conditions and edge cases
3. Design tests for all crossfade scenarios
4. Design tests for queue manipulation during playback

**Output:** `IMPL-TESTS-002-integration-test-specs.md`

**Example:**

```markdown
## Integration Test: Basic Playback

**Scenario:** User enqueues single passage and listens to completion

**Steps:**
1. Start wkmp-ap server
2. Enqueue passage via POST /playback/enqueue
3. Monitor SSE for playback events
4. Verify audio output contains passage content
5. Verify playback completes at passage end_time

**Expected Events:**
- `PassageEnqueued` (immediate)
- `DecodingStarted` (within 50ms)
- `PlaybackStarted` (within 100ms)
- `PositionUpdate` (every 100ms)
- `PlaybackCompleted` (at passage end)

**Test Code:**
```rust
#[tokio::test]
async fn test_basic_playback() {
    let server = start_test_server().await;

    // Enqueue passage
    let response = server.enqueue_passage(test_passage()).await;
    assert_eq!(response.status(), 200);

    // Monitor events
    let mut events = server.subscribe_events().await;

    // Verify enqueued
    assert_eq!(events.next().await.event_type, "PassageEnqueued");

    // Verify decoding starts quickly
    let decode_start = events.next_timeout(Duration::from_millis(50)).await;
    assert_eq!(decode_start.event_type, "DecodingStarted");

    // Verify playback starts quickly
    let playback_start = events.next_timeout(Duration::from_millis(100)).await;
    assert_eq!(playback_start.event_type, "PlaybackStarted");

    // Wait for completion
    let completion = events.wait_for("PlaybackCompleted", Duration::from_secs(300)).await;
    assert!(completion.is_some());
}
```

## Integration Test: Crossfade Transition

**Scenario:** Two passages crossfade smoothly

**Steps:**
1. Enqueue passage A (30 seconds, fade_out at 25s)
2. Enqueue passage B (30 seconds, fade_in for 5s)
3. Monitor audio output during crossfade region
4. Verify no clicks, pops, or gaps
5. Verify volume curves match fade curve specification

**Audio Quality Checks:**
- FFT analysis: No frequency spikes above -60dB outside passage content
- RMS continuity: No sudden jumps >1dB during crossfade
- Phase continuity: No phase inversions

**Test Code:**
```rust
#[tokio::test]
async fn test_crossfade_quality() {
    let server = start_test_server().await;

    // Enqueue two passages with crossfade
    server.enqueue_passage(passage_a_with_fadeout()).await;
    server.enqueue_passage(passage_b_with_fadein()).await;

    // Capture audio output
    let audio_capture = server.start_audio_capture().await;

    // Wait for crossfade region
    tokio::time::sleep(Duration::from_secs(25)).await;

    // Capture 10 seconds covering crossfade
    let crossfade_audio = audio_capture.capture_duration(Duration::from_secs(10)).await;

    // Analyze audio quality
    let analysis = analyze_audio_quality(&crossfade_audio);

    assert_eq!(analysis.clicks_detected, 0);
    assert_eq!(analysis.pops_detected, 0);
    assert!(analysis.max_rms_jump < 1.0); // dB
    assert!(analysis.max_freq_spike < -60.0); // dB
}
```
```

---

#### Agent 2C: Performance Test Design Agent

**Task:** Design performance benchmarks

**Actions:**
1. Design startup time benchmark (critical)
2. Design CPU usage benchmark
3. Design memory usage benchmark
4. Design throughput benchmark (passages/second)

**Output:** `IMPL-TESTS-003-performance-benchmarks.rs`

**Example:**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_startup_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("startup_time");

    // Benchmark with different file formats
    for format in &["mp3", "flac", "ogg"] {
        group.bench_with_input(BenchmarkId::from_parameter(format), format, |b, &format| {
            b.iter(|| {
                let passage = create_test_passage(format);
                let start = Instant::now();

                enqueue_passage(black_box(&passage));
                wait_for_first_audio_sample();

                start.elapsed()
            });
        });
    }

    group.finish();
}

fn benchmark_decode_throughput(c: &mut Criterion) {
    c.bench_function("decode_throughput", |b| {
        b.iter(|| {
            let passage = create_test_passage("flac");
            let start = Instant::now();

            decode_entire_passage(black_box(&passage));

            let duration = start.elapsed();
            let realtime_factor = passage.duration().as_secs_f64() / duration.as_secs_f64();

            realtime_factor // Should be >10x realtime
        });
    });
}

criterion_group!(benches, benchmark_startup_time, benchmark_decode_throughput);
criterion_main!(benches);
```

---

#### Agent 2D: Property-Based Test Agent

**Task:** Design property tests for invariants

**Actions:**
1. Identify system invariants from SPEC016/SPEC017
2. Design property tests using proptest
3. Focus on timing precision, buffer safety, thread safety

**Output:** `IMPL-TESTS-004-property-tests.rs`

**Example:**

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn property_tick_conversion_roundtrip(seconds in 0.0f64..1000.0f64) {
        // Property: Converting seconds → ticks → seconds should be lossless (within tick precision)
        let ticks = seconds_to_ticks(seconds);
        let roundtrip_seconds = ticks_to_seconds(ticks);

        let error = (seconds - roundtrip_seconds).abs();
        let max_error = 1.0 / 28_224_000.0; // One tick

        prop_assert!(error <= max_error);
    }

    #[test]
    fn property_buffer_never_overflows(
        write_size in 1usize..10000,
        read_size in 1usize..10000
    ) {
        // Property: Ring buffer never overflows regardless of write/read patterns
        let mut buffer = RingBuffer::new(661_941); // playout_ringbuffer_size

        for _ in 0..1000 {
            if buffer.available_write() >= write_size {
                buffer.write(&vec![0.0; write_size]);
            }
            if buffer.available_read() >= read_size {
                buffer.read(read_size);
            }
        }

        // Buffer should never exceed capacity
        prop_assert!(buffer.len() <= 661_941);
    }

    #[test]
    fn property_crossfade_sum_equals_one(
        position in 0.0f64..1.0f64,
        curve_type in 0usize..5
    ) {
        // Property: At any crossfade position, fade_out + fade_in = 1.0
        let fade_out = calculate_fade_out(position, curve_type);
        let fade_in = calculate_fade_in(position, curve_type);

        prop_assert!((fade_out + fade_in - 1.0).abs() < 0.0001);
    }
}
```

---

### Phase 3: Database Migration

#### Agent 3A: Migration Script Generator

**Task:** Create SQL migration for INTEGER ticks

**Output:** `wkmp-ap/migrations/008_tick_based_timing.sql`

```sql
-- Migration: Tick-Based Timing System
-- Requirement: [SRC-DB-010] through [SRC-DB-016]
-- Date: 2025-10-19

BEGIN TRANSACTION;

-- Step 1: Add new INTEGER tick columns
ALTER TABLE passages ADD COLUMN start_time_ticks INTEGER;
ALTER TABLE passages ADD COLUMN end_time_ticks INTEGER;
ALTER TABLE passages ADD COLUMN fade_in_point_ticks INTEGER;
ALTER TABLE passages ADD COLUMN fade_out_point_ticks INTEGER;
ALTER TABLE passages ADD COLUMN lead_in_point_ticks INTEGER;
ALTER TABLE passages ADD COLUMN lead_out_point_ticks INTEGER;

-- Step 2: Migrate existing data (seconds to ticks)
-- Conversion: ticks = seconds × 28,224,000
UPDATE passages
SET start_time_ticks = CAST(start_time * 28224000 AS INTEGER)
WHERE start_time IS NOT NULL;

UPDATE passages
SET end_time_ticks = CAST(end_time * 28224000 AS INTEGER)
WHERE end_time IS NOT NULL;

UPDATE passages
SET fade_in_point_ticks = CAST(fade_in_point * 28224000 AS INTEGER)
WHERE fade_in_point IS NOT NULL;

UPDATE passages
SET fade_out_point_ticks = CAST(fade_out_point * 28224000 AS INTEGER)
WHERE fade_out_point IS NOT NULL;

UPDATE passages
SET lead_in_point_ticks = CAST(lead_in_point * 28224000 AS INTEGER)
WHERE lead_in_point IS NOT NULL;

UPDATE passages
SET lead_out_point_ticks = CAST(lead_out_point * 28224000 AS INTEGER)
WHERE lead_out_point IS NOT NULL;

-- Step 3: Verify conversion accuracy
-- This check ensures no data loss during migration
CREATE TEMP TABLE migration_verification AS
SELECT
    guid,
    start_time,
    start_time_ticks,
    ABS(start_time - (CAST(start_time_ticks AS REAL) / 28224000.0)) AS error
FROM passages
WHERE start_time IS NOT NULL;

-- Maximum error should be less than one tick (1/28224000 ≈ 35.4 nanoseconds)
-- Fail migration if error exceeds this
SELECT CASE
    WHEN MAX(error) > (1.0 / 28224000.0)
    THEN RAISE(ABORT, 'Migration conversion error exceeds one tick')
END
FROM migration_verification;

DROP TABLE migration_verification;

-- Step 4: Drop old REAL columns
ALTER TABLE passages DROP COLUMN start_time;
ALTER TABLE passages DROP COLUMN end_time;
ALTER TABLE passages DROP COLUMN fade_in_point;
ALTER TABLE passages DROP COLUMN fade_out_point;
ALTER TABLE passages DROP COLUMN lead_in_point;
ALTER TABLE passages DROP COLUMN lead_out_point;

-- Step 5: Rename tick columns to original names
ALTER TABLE passages RENAME COLUMN start_time_ticks TO start_time;
ALTER TABLE passages RENAME COLUMN end_time_ticks TO end_time;
ALTER TABLE passages RENAME COLUMN fade_in_point_ticks TO fade_in_point;
ALTER TABLE passages RENAME COLUMN fade_out_point_ticks TO fade_out_point;
ALTER TABLE passages RENAME COLUMN lead_in_point_ticks TO lead_in_point;
ALTER TABLE passages RENAME COLUMN lead_out_point_ticks TO lead_out_point;

-- Step 6: Update schema_version
UPDATE schema_version SET version = 8;

COMMIT;
```

**Rollback Script:** `wkmp-ap/migrations/008_tick_based_timing_rollback.sql`

```sql
BEGIN TRANSACTION;

-- Reverse migration: INTEGER ticks back to REAL seconds

ALTER TABLE passages ADD COLUMN start_time_seconds REAL;
ALTER TABLE passages ADD COLUMN end_time_seconds REAL;
-- ... (reverse process)

COMMIT;
```

---

#### Agent 3C: API Converter

**Task:** Implement milliseconds ↔ ticks conversion layer

**Output:** `wkmp-common/src/conversions.rs`

```rust
//! Time conversion utilities
//!
//! Converts between different time representations:
//! - Milliseconds (u64) - API representation
//! - Ticks (i64) - Internal/database representation
//! - Samples (usize) - Audio processing representation
//!
//! Tick rate: 28,224,000 Hz per [SRC-TICK-020]

/// Ticks per second
pub const TICK_RATE: i64 = 28_224_000;

/// Ticks per millisecond (28,224,000 / 1,000)
pub const TICKS_PER_MS: i64 = 28_224;

/// [SRC-API-020] Convert milliseconds to ticks
///
/// # Examples
/// ```
/// let ms = 1000u64; // 1 second
/// let ticks = ms_to_ticks(ms);
/// assert_eq!(ticks, 28_224_000);
/// ```
pub fn ms_to_ticks(milliseconds: u64) -> i64 {
    (milliseconds as i64) * TICKS_PER_MS
}

/// [SRC-API-030] Convert ticks to milliseconds (rounded)
///
/// # Examples
/// ```
/// let ticks = 28_224_000i64; // 1 second in ticks
/// let ms = ticks_to_ms(ticks);
/// assert_eq!(ms, 1000);
/// ```
pub fn ticks_to_ms(ticks: i64) -> u64 {
    // Round to nearest millisecond
    ((ticks + (TICKS_PER_MS / 2)) / TICKS_PER_MS) as u64
}

/// [SRC-CONV-030] Convert ticks to samples at given sample rate
///
/// # Examples
/// ```
/// let ticks = 28_224_000i64; // 1 second
/// let samples = ticks_to_samples(ticks, 44100);
/// assert_eq!(samples, 44100);
/// ```
pub fn ticks_to_samples(ticks: i64, sample_rate: u32) -> usize {
    ((ticks as i128 * sample_rate as i128) / TICK_RATE as i128) as usize
}

/// [SRC-CONV-020] Convert samples to ticks at given sample rate
pub fn samples_to_ticks(samples: usize, sample_rate: u32) -> i64 {
    ((samples as i128 * TICK_RATE as i128) / sample_rate as i128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ms_roundtrip() {
        for ms in 0..10000 {
            let ticks = ms_to_ticks(ms);
            let roundtrip = ticks_to_ms(ticks);
            assert_eq!(roundtrip, ms);
        }
    }

    #[test]
    fn test_sample_conversion_44100() {
        let one_second_ticks = 28_224_000i64;
        let samples = ticks_to_samples(one_second_ticks, 44100);
        assert_eq!(samples, 44100);

        let roundtrip_ticks = samples_to_ticks(samples, 44100);
        assert_eq!(roundtrip_ticks, one_second_ticks);
    }

    #[test]
    fn test_sample_conversion_48000() {
        let one_second_ticks = 28_224_000i64;
        let samples = ticks_to_samples(one_second_ticks, 48000);
        assert_eq!(samples, 48000);
    }
}
```

---

### Phase 4: Core Playback Engine Reimplementation

#### Sub-Phase 4A: Serial Decode Execution

**Agent 4A1: Decoder Pool Refactor**

**Task:** Replace 2-thread parallel pool with serial execution

**Output:** `wkmp-ap/src/playback/serial_decoder.rs`

```rust
//! Serial Decoder
//!
//! Implements [DBD-DEC-040] serial decode execution strategy.
//! Only one decoder runs at a time for improved cache coherency
//! and reduced CPU load.

use std::sync::Arc;
use tokio::sync::Mutex;
use symphonia::core::formats::FormatReader;

/// Serial decoder with priority-based scheduling
pub struct SerialDecoder {
    /// Priority queue of decode requests
    queue: Arc<Mutex<DecodeQueue>>,

    /// Currently active decoder (if any)
    active_decoder: Arc<Mutex<Option<DecoderHandle>>>,

    /// Decoder thread handle
    thread: Option<JoinHandle<()>>,
}

impl SerialDecoder {
    pub fn new() -> Self {
        let queue = Arc::new(Mutex::new(DecodeQueue::new()));
        let active_decoder = Arc::new(Mutex::new(None));

        // Spawn decoder thread
        let thread = Self::spawn_decoder_thread(
            Arc::clone(&queue),
            Arc::clone(&active_decoder),
        );

        Self {
            queue,
            active_decoder,
            thread: Some(thread),
        }
    }

    /// [DBD-DEC-030] Enqueue decode request with priority
    pub async fn enqueue(&self, passage: Passage, priority: DecodePriority) {
        let mut queue = self.queue.lock().await;
        queue.push(DecodeRequest { passage, priority });

        // Wake decoder thread if idle
        queue.notify_one();
    }

    /// Decoder thread main loop
    fn spawn_decoder_thread(
        queue: Arc<Mutex<DecodeQueue>>,
        active_decoder: Arc<Mutex<Option<DecoderHandle>>>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                // Wait for decode request
                let request = {
                    let mut q = queue.lock().await;
                    q.wait_for_request().await
                };

                // Decode passage (this is the blocking work)
                let decoder_handle = Self::decode_passage_serial(&request.passage).await;

                // Store active decoder
                {
                    let mut active = active_decoder.lock().await;
                    *active = Some(decoder_handle);
                }

                // When decoding completes, active_decoder is cleared
                // Loop continues to next request
            }
        })
    }

    /// [DBD-DEC-050] Decode passage using decode-and-skip
    async fn decode_passage_serial(passage: &Passage) -> DecoderHandle {
        // Implementation: symphonia decode from file start, skip to start_time
        // ...
    }
}

/// Decode priority levels per [DBD-DEC-030]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecodePriority {
    /// Current passage (highest priority)
    Current = 0,

    /// Next passage in queue
    Next = 1,

    /// Prefetch for future passages
    Prefetch = 2,
}

struct DecodeQueue {
    requests: BinaryHeap<DecodeRequest>,
    notify: tokio::sync::Notify,
}

impl DecodeQueue {
    fn push(&mut self, request: DecodeRequest) {
        self.requests.push(request);
        self.notify.notify_one();
    }

    async fn wait_for_request(&mut self) -> DecodeRequest {
        while self.requests.is_empty() {
            self.notify.notified().await;
        }
        self.requests.pop().unwrap()
    }
}
```

**Unit Tests:** `wkmp-ap/src/playback/serial_decoder_tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_serial_execution() {
        let decoder = SerialDecoder::new();

        // Enqueue two passages
        decoder.enqueue(passage_1(), DecodePriority::Current).await;
        decoder.enqueue(passage_2(), DecodePriority::Next).await;

        // Give decoder thread time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Verify only one decoder active
        let active = decoder.active_decoder.lock().await;
        assert!(active.is_some());

        // Verify queue has one waiting
        let queue = decoder.queue.lock().await;
        assert_eq!(queue.len(), 1);
    }

    #[tokio::test]
    async fn test_priority_scheduling() {
        let decoder = SerialDecoder::new();

        // Enqueue in reverse priority order
        decoder.enqueue(passage_prefetch(), DecodePriority::Prefetch).await;
        decoder.enqueue(passage_next(), DecodePriority::Next).await;
        decoder.enqueue(passage_current(), DecodePriority::Current).await;

        // Verify current passage decoded first (despite being enqueued last)
        tokio::time::sleep(Duration::from_millis(100)).await;

        let active = decoder.active_decoder.lock().await;
        assert_eq!(active.as_ref().unwrap().passage_id, passage_current().id);
    }

    #[tokio::test]
    async fn test_decode_and_skip_fast() {
        let passage = Passage {
            start_time_ticks: 60 * 28_224_000, // 60 seconds
            // ...
        };

        let start = Instant::now();
        let decoder_handle = SerialDecoder::decode_passage_serial(&passage).await;
        let skip_time = start.elapsed();

        // Skip should complete in <100ms per performance requirement
        assert!(skip_time < Duration::from_millis(100));

        // First sample should be at exact start_time
        assert_eq!(decoder_handle.current_position_ticks(), passage.start_time_ticks);
    }
}
```

---

### Fast Startup Optimization Strategy

**Key Insight:** The critical path from enqueue → audio output must be minimized.

**Startup Path Analysis:**

```
User calls enqueue API
  ↓ (API handler overhead)
Parse request, validate passage
  ↓ (Database query)
Load passage metadata from DB
  ↓ (Decoder initialization)
Open audio file, read headers
  ↓ (Decode-and-skip) ← BOTTLENECK #1
Seek to start_time, skip samples
  ↓ (Resampling)
Convert to working_sample_rate (44.1kHz)
  ↓ (Fade application)
Apply fade-in curve (pre-buffer)
  ↓ (Buffer fill) ← BOTTLENECK #2
Fill playout_ringbuffer to minimum threshold
  ↓ (Mixer activation)
Activate mixer, start reading from buffer
  ↓ (Audio output)
First audio sample sent to speakers
```

**Optimization Targets:**

1. **Decode-and-Skip (BOTTLENECK #1):**
   - **Current:** Linear decode from file start (~450ms for 60s passage)
   - **Optimized:** Use codec seek tables where available (<50ms)
   - **Fallback:** Fast-forward decode (decode at 10x realtime, skip output)

2. **Buffer Fill (BOTTLENECK #2):**
   - **Current:** Fill 15 seconds (661,941 samples) before playback
   - **Optimized:** Start playback with 0.5 seconds (22,050 samples)
   - **Background:** Continue filling buffer during playback

3. **API/DB Overhead:**
   - **Current:** Synchronous database query blocks API handler
   - **Optimized:** Async DB query, pipeline with decoder initialization

**Implementation:**

```rust
/// Fast startup path for passage enqueue
pub async fn enqueue_passage_fast(passage: Passage) -> Result<()> {
    // Parallel initialization (spawn multiple tasks)
    let (db_handle, decoder_handle, buffer_handle) = tokio::join!(
        async { load_passage_metadata(&passage) },  // Database query
        async { initialize_decoder(&passage) },      // Open audio file
        async { allocate_buffer() },                 // Allocate ring buffer
    );

    let metadata = db_handle?;
    let mut decoder = decoder_handle?;
    let mut buffer = buffer_handle?;

    // Fast seek using codec tables (if available)
    let seek_result = decoder.seek_to_ticks(metadata.start_time_ticks);

    if seek_result.is_err() {
        // Fallback: fast-forward decode
        decoder.fast_forward_to(metadata.start_time_ticks)?;
    }

    // Decode minimal initial buffer (0.5 seconds = 22,050 samples @ 44.1kHz)
    const MIN_STARTUP_SAMPLES: usize = 22_050;
    let initial_samples = decoder.decode_samples(MIN_STARTUP_SAMPLES)?;

    // Apply fade-in (if needed)
    let faded_samples = apply_fade_in(&initial_samples, &metadata);

    // Write to buffer
    buffer.write(&faded_samples)?;

    // START PLAYBACK NOW (don't wait for full buffer)
    mixer.activate_buffer(buffer.clone())?;

    // Background: Continue decoding rest of passage
    tokio::spawn(async move {
        decode_remaining_passage(decoder, buffer, metadata).await
    });

    Ok(())
}

/// Background task: decode rest of passage while playback continues
async fn decode_remaining_passage(
    mut decoder: Decoder,
    mut buffer: RingBuffer,
    metadata: PassageMetadata,
) {
    let remaining_samples = metadata.duration_samples - 22_050;

    // Decode in chunks, yielding between chunks to avoid blocking
    const CHUNK_SIZE: usize = 8192;
    let mut decoded = 0;

    while decoded < remaining_samples {
        let chunk = decoder.decode_samples(CHUNK_SIZE).await?;
        buffer.write(&chunk)?;

        decoded += chunk.len();

        // Yield to allow other tasks to run
        tokio::task::yield_now().await;
    }
}
```

**Expected Performance:**

- **Decode-and-skip with seek tables:** <10ms
- **Decode-and-skip fallback:** 50-100ms
- **Initial buffer fill:** 20-30ms
- **API/DB/initialization:** 10-20ms
- **Total startup time:** <100ms (target), <50ms (with seek tables)

---

## Success Criteria Summary

### Functional Requirements

- ✅ Smooth playback (no clicks, pops, gaps, underruns)
- ✅ Accurate crossfades per SPEC002
- ✅ Sample-accurate timing per SPEC017
- ✅ Support all formats (MP3, FLAC, OGG, AAC, WAV)
- ✅ Support all sample rates (8kHz - 192kHz)
- ✅ Queue manipulation during playback
- ✅ Error recovery (corrupted files, decode errors)

### Performance Requirements

- ✅ **Startup time:** <100ms (target), <50ms (stretch)
- ✅ **CPU usage:** <5% during stereo 44.1kHz playback
- ✅ **Memory usage:** ~60MB for 12 buffers (predictable)
- ✅ **Timing precision:** Zero rounding errors, tick-accurate
- ✅ **Crossfade quality:** No artifacts detectable

### Test Coverage Requirements

- ✅ **Unit test coverage:** >80%
- ✅ **Integration tests:** All critical paths covered
- ✅ **Performance benchmarks:** Startup, CPU, memory measured
- ✅ **Property tests:** Invariants verified (timing, buffer safety)
- ✅ **Real-world validation:** 100+ diverse music files tested

---

## Timeline and Effort Estimate

| Phase | Duration | Agents | Effort (days) |
|-------|----------|--------|---------------|
| **Phase 1: Analysis** | 3-5 days | 4 | 3-5 |
| **Phase 2: Test Design** | 5-7 days | 4 | 5-7 |
| **Phase 3: DB Migration** | 2-3 days | 3 | 2-3 |
| **Phase 4A: Serial Decode** | 3-4 days | 3 | 3-4 |
| **Phase 4B: Pre-Buffer Fades** | 2-3 days | 3 | 2-3 |
| **Phase 4C: Buffer Management** | 3-4 days | 3 | 3-4 |
| **Phase 4D: Mixer/Crossfade** | 2-4 days | 3 | 2-4 |
| **Phase 5: Fast Startup** | 3-5 days | 4 | 3-5 |
| **Phase 6: Integration Testing** | 5-7 days | 4 | 5-7 |
| **Phase 7: Performance Validation** | 2-3 days | 4 | 2-3 |
| **Total** | **30-45 days** | **35 agents** | **30-45** |

**Note:** Phases can overlap. Test-driven design (Phase 2) informs implementation (Phase 4). Integration testing (Phase 6) can begin as Phase 4 sub-phases complete.

---

## Risk Mitigation

### Risk 1: Startup time target missed

**Mitigation:**
- Profile early and often (Phase 1C baseline, Phase 5 optimization)
- Identify bottlenecks with flame graphs
- Have fallback: Progressive optimization until target met

### Risk 2: Crossfade quality regression

**Mitigation:**
- Audio quality tests in Phase 2B (FFT analysis, RMS continuity)
- Reference recordings for comparison
- Property tests for fade curve sum = 1.0

### Risk 3: Database migration data loss

**Mitigation:**
- Migration validation (Phase 3B)
- Rollback script tested
- Backup before migration (documented in T1-TIMING-001-APPROVED.md)

### Risk 4: Test suite maintenance burden

**Mitigation:**
- Test code quality = production code quality
- DRY principle for test utilities
- Continuous refactoring as implementation evolves

---

## Next Steps

1. **Review this workflow plan**
2. **Approve/modify agent specifications**
3. **Execute Phase 1 (Analysis)** to establish baseline
4. **Begin Phase 2 (Test Design)** in parallel with Phase 1
5. **Iterate** through phases with regular checkpoints

---

**Prepared By:** Documentation Integration Team
**Date:** 2025-10-19
**Status:** READY FOR EXECUTION

**End of Implementation Workflow Plan**

# Integration Test Specifications

**Document Type:** Test Specifications
**Created:** 2025-10-19
**Status:** Draft
**Purpose:** Define comprehensive end-to-end integration test scenarios for WKMP Audio Player

> **Related Documentation:** [SPEC016 Decoder Buffer Design](/home/sw/Dev/McRhythm/docs/SPEC016-decoder_buffer_design.md) | [SPEC017 Sample Rate Conversion](/home/sw/Dev/McRhythm/docs/SPEC017-sample_rate_conversion.md) | [SPEC002 Crossfade Design](/home/sw/Dev/McRhythm/docs/SPEC002-crossfade.md)

---

## Overview

This document specifies 10 critical integration test scenarios that validate the complete playback pipeline from API request to audio output. Each test includes:

- **Requirements traceability** to SPEC016/SPEC017
- **Setup procedures** for database and audio fixtures
- **Step-by-step test execution**
- **Expected SSE events**
- **Audio quality assertions** (FFT, RMS, phase analysis)
- **Rust test implementation** with tokio::test

---

## Test Infrastructure Components

### Required Test Helpers

1. **TestServer** (`/home/sw/Dev/McRhythm/wkmp-ap/tests/helpers/test_server.rs`)
   - Start/stop wkmp-ap programmatically
   - In-memory SQLite database
   - SSE event subscription
   - API request wrapper

2. **AudioCapture** (`/home/sw/Dev/McRhythm/wkmp-ap/tests/helpers/audio_capture.rs`)
   - Capture audio output stream
   - Record samples that would go to speakers
   - Provide raw PCM data for analysis

3. **AudioAnalysis** (`/home/sw/Dev/McRhythm/wkmp-ap/tests/helpers/audio_analysis.rs`)
   - FFT analysis for frequency spikes
   - RMS level tracking
   - Phase continuity verification
   - Click/pop detection algorithms

### Audio Quality Analysis Functions

#### `detect_clicks(samples: &[f32]) -> Vec<ClickEvent>`

**Algorithm:**
- Perform FFT in 2048-sample windows (46ms at 44.1kHz)
- Scan for frequency spikes >-60dB above baseline
- Clicks appear as wideband frequency spikes
- Return timestamps and severity

**Implementation:**
```rust
pub struct ClickEvent {
    pub sample_index: usize,
    pub timestamp_ms: u64,
    pub peak_db: f32,
    pub frequency_hz: f32,
}

pub fn detect_clicks(samples: &[f32], sample_rate: u32) -> Vec<ClickEvent> {
    // Use realfft for efficient FFT
    // Window size: 2048 samples
    // Hop size: 512 samples (75% overlap)
    // Threshold: -60dB above rolling average
}
```

#### `detect_pops(samples: &[f32]) -> Vec<PopEvent>`

**Algorithm:**
- Calculate frame-to-frame amplitude change
- Pop = sudden change >6dB in <10ms (441 samples at 44.1kHz)
- Use sliding window to detect rapid transients
- Distinguish from legitimate music transients via frequency analysis

**Implementation:**
```rust
pub struct PopEvent {
    pub sample_index: usize,
    pub timestamp_ms: u64,
    pub amplitude_change_db: f32,
}

pub fn detect_pops(samples: &[f32], sample_rate: u32) -> Vec<PopEvent> {
    // Calculate RMS in 10ms windows
    // Detect changes >6dB between consecutive windows
    // Filter out legitimate transients (bass hits, etc.)
}
```

#### `verify_rms_continuity(samples: &[f32], crossfade_region: (usize, usize)) -> RmsContinuityReport`

**Algorithm:**
- Calculate RMS in 100ms windows throughout crossfade
- Verify no jumps >1dB between consecutive windows
- Expected: Smooth RMS transition during crossfade
- Return report with jump locations and magnitudes

**Implementation:**
```rust
pub struct RmsContinuityReport {
    pub passed: bool,
    pub max_jump_db: f32,
    pub jump_locations: Vec<(usize, f32)>, // (sample_index, jump_db)
    pub rms_timeline: Vec<(u64, f32)>,     // (timestamp_ms, rms)
}

pub fn verify_rms_continuity(
    samples: &[f32],
    crossfade_region: (usize, usize),
    sample_rate: u32
) -> RmsContinuityReport {
    // 100ms window = 4410 samples at 44.1kHz
    // Calculate RMS for each window
    // Check consecutive window differences
}
```

#### `verify_phase_continuity(samples: &[f32]) -> PhaseContinuityReport`

**Algorithm:**
- Detect phase inversions (sudden 180° phase shifts)
- Check stereo coherence (L/R phase relationship)
- Flag discontinuities that indicate mixing errors

**Implementation:**
```rust
pub struct PhaseContinuityReport {
    pub passed: bool,
    pub inversions_detected: Vec<usize>, // sample indices
    pub stereo_coherence: f32,           // 0.0-1.0
}

pub fn verify_phase_continuity(samples: &[f32], sample_rate: u32) -> PhaseContinuityReport {
    // Analyze L/R channel correlation
    // Detect sudden phase inversions
    // Check for polarity flips
}
```

#### `measure_startup_latency(api_call_time: Instant, audio_capture: &AudioCapture) -> Duration`

**Algorithm:**
- Record timestamp of API call (passage enqueue)
- Wait for first non-zero audio sample in capture buffer
- Calculate elapsed time
- CRITICAL: Must be <100ms for Phase 5 goal

**Implementation:**
```rust
pub fn measure_startup_latency(
    start_time: Instant,
    audio_capture: &AudioCapture,
    threshold: f32 // e.g., 0.001
) -> Duration {
    // Poll audio capture buffer
    // Find first sample with abs(value) > threshold
    // Return elapsed time since start_time
}
```

---

## Test Scenarios

### Scenario 1: Basic Playback with Fast Startup

**Requirements:** [DBD-OV-010], [DBD-DEC-050], [DBD-MIX-010]

**Goal:** Verify passage plays from start to end with <100ms startup latency

**Priority:** CRITICAL (Phase 1 target: <100ms startup)

#### Setup

1. Start wkmp-ap test server with in-memory database
2. Create test passage metadata:
   - `file_path`: `/home/sw/Music/Bigger,_Better,_Faster,_More/01-Train.mp3`
   - `start_time`: 0 ticks
   - `end_time`: 30 seconds (846,720,000 ticks)
   - `fade_in_point`: 0 (no fade-in for latency test)
   - `fade_out_point`: 30s (no fade-out)
3. Start SSE event monitor
4. Start audio output capture

#### Steps

1. Record timestamp T0 (Instant::now())
2. POST `/playback/enqueue` with test passage
3. Wait for first audio sample with amplitude >0.001
4. Record timestamp T1 when first audio detected
5. Calculate startup latency: `T1 - T0`
6. Monitor playback until completion
7. Record timestamp T2 when PlaybackCompleted event received

#### Expected Events

- `PassageEnqueued` (immediate, <10ms from T0)
- `DecodingStarted` (within 50ms from T0)
- `PlaybackStarted` (within 100ms from T0) **← CRITICAL REQUIREMENT**
- `PositionUpdate` (every 100ms during playback)
- `PlaybackCompleted` (at ~30 seconds)

#### Assertions

**Timing:**
```rust
assert!(startup_latency < Duration::from_millis(100),
        "Startup took {:?}, expected <100ms", startup_latency);
```

**Audio Quality:**
```rust
let analysis = analyze_audio(&audio_capture.samples());
assert_eq!(analysis.clicks_detected, 0, "No clicks allowed");
assert_eq!(analysis.pops_detected, 0, "No pops allowed");
```

**Playback Accuracy:**
```rust
// Verify exact timing: 30s at 44.1kHz = 1,323,000 samples
let expected_duration = Duration::from_secs(30);
let actual_duration = T2 - T1;
let timing_error = (actual_duration.as_secs_f32() - expected_duration.as_secs_f32()).abs();
assert!(timing_error < 0.1, "Timing error: {:.2}s", timing_error);
```

#### Test Code

```rust
#[tokio::test]
async fn test_basic_playback_with_fast_startup() {
    // Setup
    let server = TestServer::start().await.expect("Server start failed");
    let mut events = server.subscribe_events().await;
    let mut audio_capture = server.start_audio_capture().await.expect("Capture start failed");

    // Create test passage
    let passage = create_test_passage(
        "/home/sw/Music/Bigger,_Better,_Faster,_More/01-Train.mp3",
        0,          // start_time (ticks)
        846_720_000, // end_time (30s in ticks)
    );

    // Record start time
    let t0 = Instant::now();

    // Enqueue passage
    server.enqueue_passage(passage).await.expect("Enqueue failed");

    // Wait for first audio sample
    let first_audio_time = audio_capture
        .wait_for_audio(Duration::from_secs(1), 0.001)
        .await
        .expect("No audio detected within 1 second");

    let t1 = first_audio_time;
    let startup_latency = t1.duration_since(t0);

    // CRITICAL: Verify startup < 100ms
    assert!(
        startup_latency < Duration::from_millis(100),
        "Startup took {:?}, expected <100ms (Phase 1 goal)",
        startup_latency
    );

    println!("✅ Startup latency: {:?}", startup_latency);

    // Verify events in correct order
    let passage_enqueued = events.next_timeout(Duration::from_millis(100)).await
        .expect("PassageEnqueued timeout");
    assert_eq!(passage_enqueued.event_type(), "PassageEnqueued");

    let decoding_started = events.next_timeout(Duration::from_millis(100)).await
        .expect("DecodingStarted timeout");
    assert_eq!(decoding_started.event_type(), "DecodingStarted");

    let playback_started = events.next_timeout(Duration::from_millis(100)).await
        .expect("PlaybackStarted timeout");
    assert_eq!(playback_started.event_type(), "PlaybackStarted");

    // Wait for completion
    let completion = events
        .wait_for("PlaybackCompleted", Duration::from_secs(35))
        .await
        .expect("PlaybackCompleted timeout");

    let t2 = Instant::now();

    // Verify playback duration
    let playback_duration = t2.duration_since(t1);
    let expected_duration = Duration::from_secs(30);
    let timing_error = (playback_duration.as_secs_f32() - expected_duration.as_secs_f32()).abs();

    assert!(
        timing_error < 0.1,
        "Timing error: {:.2}s (expected ~30s, got {:.2}s)",
        timing_error,
        playback_duration.as_secs_f32()
    );

    // Analyze audio quality
    let samples = audio_capture.get_samples();
    let analysis = AudioAnalysis::analyze(&samples, 44100);

    assert_eq!(analysis.clicks_detected, 0, "Clicks detected: {:?}", analysis.click_events);
    assert_eq!(analysis.pops_detected, 0, "Pops detected: {:?}", analysis.pop_events);

    println!("✅ Audio quality verified (no clicks/pops)");
    println!("✅ Basic playback test PASSED");
}
```

---

### Scenario 2: Smooth Crossfade Quality

**Requirements:** [DBD-MIX-040], [DBD-FADE-030], [DBD-FADE-050]

**Goal:** Verify crossfades have no clicks, pops, gaps, or RMS discontinuities

**Priority:** HIGH (Core feature quality)

#### Setup

1. Start test server
2. Create two test passages:
   - **Passage A:**
     - File: `/home/sw/Music/Bigger,_Better,_Faster,_More/01-Train.mp3`
     - Duration: 30s
     - `lead_out_point`: 25s (5s crossfade region)
     - `fade_out_point`: 25s
     - `fade_out_curve`: EqualPower
   - **Passage B:**
     - File: `/home/sw/Music/Bigger,_Better,_Faster,_More/02-Superfly.mp3`
     - Duration: 30s
     - `lead_in_point`: 5s (5s crossfade region)
     - `fade_in_point`: 0s
     - `fade_in_curve`: EqualPower
3. Start audio capture

#### Steps

1. Enqueue Passage A
2. Wait 20 seconds
3. Enqueue Passage B
4. Monitor audio during crossfade (seconds 25-30 of Passage A)
5. Record crossfade audio samples
6. Continue until Passage B completes

#### Expected Events

- `PassageEnqueued` (A)
- `PlaybackStarted` (A)
- `PassageEnqueued` (B)
- `CrossfadeStarted` (at ~25s)
- `CrossfadeCompleted` (at ~30s)
- `PlaybackCompleted` (A)
- `PlaybackStarted` (B)
- `PlaybackCompleted` (B)

#### Assertions

**Audio Quality during Crossfade:**

```rust
// Extract crossfade region (25s-30s = samples 1,102,500-1,323,000 at 44.1kHz)
let crossfade_start = 25 * 44100 * 2; // stereo interleaved
let crossfade_end = 30 * 44100 * 2;
let crossfade_samples = &samples[crossfade_start..crossfade_end];

// 1. No clicks
let clicks = detect_clicks(crossfade_samples, 44100);
assert_eq!(clicks.len(), 0, "Clicks during crossfade: {:?}", clicks);

// 2. No pops
let pops = detect_pops(crossfade_samples, 44100);
assert_eq!(pops.len(), 0, "Pops during crossfade: {:?}", pops);

// 3. RMS continuity
let rms_report = verify_rms_continuity(crossfade_samples, (0, crossfade_samples.len()), 44100);
assert!(rms_report.passed, "RMS discontinuity: max jump {:.2}dB", rms_report.max_jump_db);
assert!(rms_report.max_jump_db < 1.0, "RMS jump too large: {:.2}dB", rms_report.max_jump_db);

// 4. Phase continuity
let phase_report = verify_phase_continuity(crossfade_samples, 44100);
assert!(phase_report.passed, "Phase inversions detected: {:?}", phase_report.inversions_detected);

// 5. Verify Equal-Power sum (constant energy)
// For Equal-Power crossfade: fade_out^2 + fade_in^2 = 1.0 (constant power)
// This means RMS should be relatively constant
let rms_variance = calculate_variance(&rms_report.rms_timeline);
assert!(rms_variance < 0.01, "RMS variance too high: {:.4}", rms_variance);
```

**Fade Curve Accuracy:**

```rust
// Verify fade curves match EqualPower formula
// fade_out(t) = cos(t * π/2)
// fade_in(t) = sin(t * π/2)
// where t goes from 0.0 to 1.0 over the crossfade duration

let fade_accuracy = verify_fade_curve(
    crossfade_samples,
    FadeCurve::EqualPower,
    5.0, // duration seconds
    44100
);
assert!(fade_accuracy.max_error < 0.01, "Fade curve error: {:.4}", fade_accuracy.max_error);
```

#### Test Code

```rust
#[tokio::test]
async fn test_smooth_crossfade_quality() {
    let server = TestServer::start().await.expect("Server start");
    let mut events = server.subscribe_events().await;
    let mut audio_capture = server.start_audio_capture().await.expect("Capture start");

    // Create passages with crossfade configuration
    let passage_a = PassageBuilder::new()
        .file("/home/sw/Music/Bigger,_Better,_Faster,_More/01-Train.mp3")
        .duration_seconds(30.0)
        .lead_out_point_seconds(25.0)
        .fade_out_point_seconds(25.0)
        .fade_out_curve(FadeCurve::EqualPower)
        .build();

    let passage_b = PassageBuilder::new()
        .file("/home/sw/Music/Bigger,_Better,_Faster,_More/02-Superfly.mp3")
        .duration_seconds(30.0)
        .lead_in_point_seconds(5.0)
        .fade_in_point_seconds(0.0)
        .fade_in_curve(FadeCurve::EqualPower)
        .build();

    // Enqueue passage A
    server.enqueue_passage(passage_a).await.expect("Enqueue A");

    // Wait until near end of passage A
    tokio::time::sleep(Duration::from_secs(20)).await;

    // Enqueue passage B (will trigger crossfade)
    server.enqueue_passage(passage_b).await.expect("Enqueue B");

    // Wait for crossfade to start
    let crossfade_started = events
        .wait_for("CrossfadeStarted", Duration::from_secs(10))
        .await
        .expect("CrossfadeStarted timeout");

    println!("✅ Crossfade started at {:?}", crossfade_started.timestamp);

    // Wait for crossfade to complete
    let crossfade_completed = events
        .wait_for("CrossfadeCompleted", Duration::from_secs(10))
        .await
        .expect("CrossfadeCompleted timeout");

    println!("✅ Crossfade completed at {:?}", crossfade_completed.timestamp);

    // Wait for passage B to complete
    events.wait_for("PlaybackCompleted", Duration::from_secs(40)).await
        .expect("PlaybackCompleted timeout");

    // Extract crossfade region for analysis
    // Crossfade occurs at 25s-30s of Passage A
    let samples = audio_capture.get_samples();
    let crossfade_start_sample = 25 * 44100 * 2; // 25s, stereo
    let crossfade_end_sample = 30 * 44100 * 2;   // 30s, stereo

    let crossfade_samples = &samples[crossfade_start_sample..crossfade_end_sample];

    // Audio quality checks
    let clicks = detect_clicks(crossfade_samples, 44100);
    assert_eq!(clicks.len(), 0, "Clicks detected during crossfade: {:?}", clicks);

    let pops = detect_pops(crossfade_samples, 44100);
    assert_eq!(pops.len(), 0, "Pops detected during crossfade: {:?}", pops);

    let rms_report = verify_rms_continuity(crossfade_samples, (0, crossfade_samples.len()), 44100);
    assert!(rms_report.passed, "RMS discontinuity detected");
    assert!(rms_report.max_jump_db < 1.0,
            "RMS jump too large: {:.2}dB (expected <1dB)", rms_report.max_jump_db);

    let phase_report = verify_phase_continuity(crossfade_samples, 44100);
    assert!(phase_report.passed,
            "Phase inversions detected: {:?}", phase_report.inversions_detected);

    // Verify Equal-Power crossfade maintains constant energy
    let rms_variance = calculate_variance(&rms_report.rms_timeline);
    assert!(rms_variance < 0.01,
            "RMS variance too high: {:.4} (expected <0.01)", rms_variance);

    println!("✅ Crossfade quality verified:");
    println!("   - No clicks/pops");
    println!("   - RMS continuity maintained");
    println!("   - Phase continuity verified");
    println!("   - Equal-Power sum constant");
    println!("✅ Crossfade test PASSED");
}
```

---

### Scenario 3: Rapid Enqueue

**Requirements:** [DBD-FLOW-100], [DBD-FLOW-110], [DBD-OV-050]

**Goal:** Verify system handles rapid passage enqueuing without errors or race conditions

**Priority:** HIGH (Stress test for queue management)

#### Setup

1. Start test server
2. Prepare 10 test passage definitions (different files)
3. Start event monitor

#### Steps

1. Spawn 10 concurrent tasks
2. Each task enqueues one passage immediately
3. All 10 passages enqueued within <1 second
4. Verify queue order maintained
5. Monitor playback through all 10 passages

#### Expected Events

- 10× `PassageEnqueued` (in rapid succession)
- `PlaybackStarted` (for first passage)
- 9× `CrossfadeStarted` / `CrossfadeCompleted`
- 10× `PlaybackCompleted`

#### Assertions

```rust
// 1. All passages successfully enqueued
assert_eq!(enqueue_results.len(), 10);
for result in &enqueue_results {
    assert!(result.is_ok(), "Enqueue failed: {:?}", result);
}

// 2. Queue order maintained (play_order is sequential)
let queue = server.get_queue().await.expect("Get queue failed");
assert_eq!(queue.len(), 10);

let mut prev_order = 0;
for entry in queue {
    assert!(entry.play_order > prev_order, "Queue order not maintained");
    prev_order = entry.play_order;
}

// 3. No events lost during rapid enqueue
let enqueued_events: Vec<_> = events
    .filter(|e| e.event_type() == "PassageEnqueued")
    .take(10)
    .collect();
assert_eq!(enqueued_events.len(), 10);

// 4. Playback progresses through all passages
for i in 1..=10 {
    let completed = events
        .wait_for("PlaybackCompleted", Duration::from_secs(60))
        .await
        .expect(&format!("Passage {} completion timeout", i));
}
```

#### Test Code

```rust
#[tokio::test]
async fn test_rapid_enqueue() {
    let server = TestServer::start().await.expect("Server start");
    let mut events = server.subscribe_events().await;

    // Prepare 10 test passages
    let passages: Vec<_> = (0..10)
        .map(|i| {
            PassageBuilder::new()
                .file(format!("/home/sw/Music/Bigger,_Better,_Faster,_More/0{}-*.mp3", i+1))
                .duration_seconds(10.0)
                .build()
        })
        .collect();

    // Rapid concurrent enqueue
    let start = Instant::now();

    let mut tasks = Vec::new();
    for passage in passages {
        let server_clone = server.clone();
        let task = tokio::spawn(async move {
            server_clone.enqueue_passage(passage).await
        });
        tasks.push(task);
    }

    let results: Vec<_> = futures::future::join_all(tasks).await;
    let enqueue_duration = start.elapsed();

    // Verify all enqueued successfully within 1 second
    assert!(enqueue_duration < Duration::from_secs(1),
            "Enqueue took {:?}, expected <1s", enqueue_duration);

    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Task {} failed: {:?}", i, result);
        assert!(result.as_ref().unwrap().is_ok(), "Enqueue {} failed", i);
    }

    println!("✅ Enqueued 10 passages in {:?}", enqueue_duration);

    // Verify queue order
    let queue = server.get_queue().await.expect("Get queue");
    assert_eq!(queue.len(), 10);

    let mut prev_order = 0;
    for (i, entry) in queue.iter().enumerate() {
        assert!(entry.play_order > prev_order,
                "Queue order violation at index {}", i);
        prev_order = entry.play_order;
    }

    println!("✅ Queue order maintained");

    // Verify all PassageEnqueued events received
    let enqueued_events: Vec<_> = events
        .take_matching("PassageEnqueued", 10, Duration::from_secs(5))
        .await
        .expect("Missing PassageEnqueued events");

    assert_eq!(enqueued_events.len(), 10);
    println!("✅ All 10 PassageEnqueued events received");

    // Monitor playback through all passages
    for i in 1..=10 {
        let completed = events
            .wait_for("PlaybackCompleted", Duration::from_secs(30))
            .await
            .expect(&format!("Passage {} completion timeout", i));

        println!("✅ Passage {} completed", i);
    }

    println!("✅ Rapid enqueue test PASSED");
}
```

---

### Scenario 4: Skip During Crossfade

**Requirements:** [DBD-MIX-040], [DBD-BUF-060]

**Goal:** Verify skip command during crossfade cleanly terminates fade and starts new passage

**Priority:** HIGH (Edge case for resource cleanup)

#### Setup

1. Start test server
2. Create 3 test passages (A, B, C)
3. Configure passages A→B crossfade (5s)
4. Start event monitor and audio capture

#### Steps

1. Enqueue all 3 passages
2. Wait for crossfade A→B to start
3. During crossfade (at 2.5s into it), issue skip command
4. Verify immediate transition to passage C
5. Monitor audio for artifacts

#### Expected Events

- `CrossfadeStarted` (A→B)
- `SkipRequested` (user action)
- `CrossfadeAborted` (A→B)
- `PlaybackCompleted` (A, early)
- `PlaybackCompleted` (B, never played)
- `PlaybackStarted` (C)
- `PlaybackCompleted` (C)

#### Assertions

```rust
// 1. Skip occurs during crossfade
let crossfade_start_time = /* from event */;
let skip_time = /* from event */;
let time_into_crossfade = skip_time.duration_since(crossfade_start_time);
assert!(time_into_crossfade > Duration::from_millis(2000));
assert!(time_into_crossfade < Duration::from_millis(3000));

// 2. Passage B resources cleaned up (never fully played)
let queue_after_skip = server.get_queue().await;
assert!(!queue_after_skip.iter().any(|e| e.passage_id == passage_b_id));

// 3. No audio artifacts during skip
let skip_sample_index = /* calculate from skip_time */;
let skip_region = &samples[skip_sample_index..skip_sample_index + 4410]; // 100ms
let clicks = detect_clicks(skip_region, 44100);
let pops = detect_pops(skip_region, 44100);
assert_eq!(clicks.len(), 0);
assert_eq!(pops.len(), 0);

// 4. Passage C starts immediately
let c_started = events.wait_for("PlaybackStarted", Duration::from_millis(200)).await;
assert!(c_started.is_some());
```

#### Test Code

```rust
#[tokio::test]
async fn test_skip_during_crossfade() {
    let server = TestServer::start().await.expect("Server start");
    let mut events = server.subscribe_events().await;
    let mut audio_capture = server.start_audio_capture().await.expect("Capture start");

    // Create 3 passages
    let passage_a = PassageBuilder::new()
        .file("/home/sw/Music/Bigger,_Better,_Faster,_More/01-Train.mp3")
        .duration_seconds(20.0)
        .lead_out_point_seconds(15.0)
        .build();

    let passage_b = PassageBuilder::new()
        .file("/home/sw/Music/Bigger,_Better,_Faster,_More/02-Superfly.mp3")
        .duration_seconds(20.0)
        .lead_in_point_seconds(5.0)
        .build();

    let passage_c = PassageBuilder::new()
        .file("/home/sw/Music/Bigger,_Better,_Faster,_More/03-Whats_Up.mp3")
        .duration_seconds(20.0)
        .build();

    // Enqueue all passages
    server.enqueue_passage(passage_a).await.expect("Enqueue A");
    server.enqueue_passage(passage_b).await.expect("Enqueue B");
    let passage_c_id = server.enqueue_passage(passage_c).await.expect("Enqueue C");

    // Wait for crossfade to start
    let crossfade_started = events
        .wait_for("CrossfadeStarted", Duration::from_secs(20))
        .await
        .expect("CrossfadeStarted timeout");

    let crossfade_start_time = crossfade_started.timestamp;
    println!("✅ Crossfade started at {:?}", crossfade_start_time);

    // Wait 2.5 seconds into crossfade
    tokio::time::sleep(Duration::from_millis(2500)).await;

    // Issue skip command
    let skip_time = Instant::now();
    server.skip_next().await.expect("Skip failed");

    println!("✅ Skip issued at 2.5s into crossfade");

    // Verify CrossfadeAborted event
    let crossfade_aborted = events
        .wait_for("CrossfadeAborted", Duration::from_millis(500))
        .await
        .expect("CrossfadeAborted timeout");

    assert!(crossfade_aborted.is_some());
    println!("✅ CrossfadeAborted event received");

    // Verify passage C starts immediately
    let c_started = events
        .wait_for("PlaybackStarted", Duration::from_millis(200))
        .await
        .expect("Passage C PlaybackStarted timeout");

    // Verify passage C is actually the one playing
    let c_started_data = c_started.unwrap();
    assert_eq!(c_started_data.passage_id, passage_c_id);

    println!("✅ Passage C started immediately after skip");

    // Verify passage B cleaned up (not in queue)
    tokio::time::sleep(Duration::from_millis(100)).await;
    let queue = server.get_queue().await.expect("Get queue");
    assert!(!queue.iter().any(|e| e.passage_id == passage_b.id));

    println!("✅ Passage B resources cleaned up");

    // Analyze audio at skip point
    let samples = audio_capture.get_samples();
    let skip_sample_index = audio_capture.timestamp_to_sample_index(skip_time);
    let skip_region = &samples[skip_sample_index..skip_sample_index.min(samples.len()).min(skip_sample_index + 4410)];

    let clicks = detect_clicks(skip_region, 44100);
    let pops = detect_pops(skip_region, 44100);

    assert_eq!(clicks.len(), 0, "Clicks detected during skip: {:?}", clicks);
    assert_eq!(pops.len(), 0, "Pops detected during skip: {:?}", pops);

    println!("✅ No audio artifacts during skip");

    // Wait for passage C to complete
    events.wait_for("PlaybackCompleted", Duration::from_secs(25)).await
        .expect("Passage C completion timeout");

    println!("✅ Skip during crossfade test PASSED");
}
```

---

### Scenario 5: Queue Manipulation During Playback

**Requirements:** [DBD-FLOW-110], [DBD-OV-050]

**Goal:** Verify queue add/remove operations during active playback don't disrupt audio

**Priority:** MEDIUM (Common user interaction pattern)

#### Setup

1. Start test server
2. Create 7 test passages (initially enqueue 5)
3. Start audio capture

#### Steps

1. Enqueue passages 1-5
2. Start playback
3. Wait 10 seconds
4. Add passage 6 to queue (mid-playback)
5. Wait 5 seconds
6. Add passage 7 to queue
7. Wait 5 seconds
8. Remove passage 4 from queue
9. Monitor playback continues smoothly
10. Verify final queue state

#### Expected Events

- 5× `PassageEnqueued` (initial)
- `PlaybackStarted` (passage 1)
- `PassageEnqueued` (passage 6, mid-playback)
- `PassageEnqueued` (passage 7)
- `PassageRemoved` (passage 4)
- `PlaybackCompleted` (passages 1, 2, 3, 5, 6, 7 in order)

#### Assertions

```rust
// 1. Queue operations succeed during playback
assert!(add_passage_6_result.is_ok());
assert!(add_passage_7_result.is_ok());
assert!(remove_passage_4_result.is_ok());

// 2. No audio disruption during queue operations
let add_6_sample = /* sample index when passage 6 added */;
let add_7_sample = /* sample index when passage 7 added */;
let remove_4_sample = /* sample index when passage 4 removed */;

for &sample_idx in &[add_6_sample, add_7_sample, remove_4_sample] {
    let region = &samples[sample_idx..sample_idx + 4410];
    assert_eq!(detect_clicks(region, 44100).len(), 0);
    assert_eq!(detect_pops(region, 44100).len(), 0);
}

// 3. Final queue contains: 1, 2, 3, 5, 6, 7 (passage 4 removed)
let final_queue = server.get_queue().await;
let expected_ids = vec![passage_1_id, passage_2_id, passage_3_id,
                        passage_5_id, passage_6_id, passage_7_id];
assert_eq!(final_queue.iter().map(|e| e.passage_id).collect::<Vec<_>>(),
           expected_ids);

// 4. Playback completes all passages except 4
let completed_passages = events
    .filter(|e| e.event_type() == "PlaybackCompleted")
    .take(6)
    .collect();
assert_eq!(completed_passages.len(), 6);
```

---

### Scenario 6: Long Playback (Memory Leak Detection)

**Requirements:** [DBD-BUF-010], [DBD-MIX-010]

**Goal:** Verify no memory leaks during extended playback

**Priority:** MEDIUM (Long-running stability)

#### Setup

1. Start test server with memory monitoring
2. Enqueue 100 short passages (10 seconds each, ~16 minutes total)
3. Record initial memory usage

#### Steps

1. Start playback
2. Every 60 seconds, record:
   - Memory usage (resident set size)
   - CPU usage
   - Queue length
   - Active buffer count
3. Continue until all 100 passages played
4. Analyze memory trend

#### Expected Events

- 100× `PassageEnqueued`
- 100× `PlaybackCompleted`
- No errors or warnings

#### Assertions

```rust
// 1. Memory growth bounded
let initial_memory = memory_samples[0];
let final_memory = memory_samples.last().unwrap();
let memory_growth = final_memory - initial_memory;
let memory_growth_percent = (memory_growth as f64 / initial_memory as f64) * 100.0;

assert!(memory_growth_percent < 5.0,
        "Memory grew {:.1}% (expected <5%)", memory_growth_percent);

// 2. No unbounded growth trend (linear regression)
let slope = calculate_linear_regression_slope(&memory_samples);
assert!(slope < 0.1, // Max 0.1MB/minute growth
        "Memory growth trend too high: {:.2} MB/min", slope);

// 3. All passages completed
assert_eq!(completed_count, 100);

// 4. No buffer leaks (all buffers released)
let final_buffer_count = server.get_active_buffer_count().await;
assert_eq!(final_buffer_count, 0,
           "Active buffers remaining: {}", final_buffer_count);
```

#### Test Code

```rust
#[tokio::test]
#[ignore] // Long-running test (16+ minutes)
async fn test_long_playback_memory_leak() {
    let server = TestServer::start().await.expect("Server start");
    let mut events = server.subscribe_events().await;

    // Enqueue 100 short passages
    for i in 0..100 {
        let passage = PassageBuilder::new()
            .file(test_file_for_index(i))
            .duration_seconds(10.0)
            .build();

        server.enqueue_passage(passage).await.expect("Enqueue failed");
    }

    println!("✅ Enqueued 100 passages (~16 minutes total)");

    // Start memory monitoring
    let mut memory_samples = Vec::new();
    let mut cpu_samples = Vec::new();
    let initial_memory = get_process_memory();
    memory_samples.push(initial_memory);

    println!("Initial memory: {:.2} MB", initial_memory as f64 / 1024.0 / 1024.0);

    // Monitor playback
    let mut completed_count = 0;
    let start_time = Instant::now();

    loop {
        // Wait for next event or timeout
        match tokio::time::timeout(Duration::from_secs(60), events.next()).await {
            Ok(Some(event)) => {
                if event.event_type() == "PlaybackCompleted" {
                    completed_count += 1;
                    println!("✅ Passage {} completed ({:.1}% done)",
                             completed_count,
                             (completed_count as f64 / 100.0) * 100.0);
                }
            }
            Ok(None) => break, // Event stream closed
            Err(_) => {
                // 60-second timeout - record metrics
                let memory = get_process_memory();
                let cpu = get_process_cpu_usage();

                memory_samples.push(memory);
                cpu_samples.push(cpu);

                let elapsed = start_time.elapsed();
                println!("⏱ {} min - Memory: {:.2} MB, CPU: {:.1}%, Queue: {}",
                         elapsed.as_secs() / 60,
                         memory as f64 / 1024.0 / 1024.0,
                         cpu,
                         server.get_queue().await.unwrap().len());
            }
        }

        if completed_count >= 100 {
            break;
        }
    }

    // Verify all passages completed
    assert_eq!(completed_count, 100, "Not all passages completed");
    println!("✅ All 100 passages completed");

    // Analyze memory trend
    let initial_memory = memory_samples[0];
    let final_memory = *memory_samples.last().unwrap();
    let memory_growth = final_memory.saturating_sub(initial_memory);
    let memory_growth_percent = (memory_growth as f64 / initial_memory as f64) * 100.0;

    println!("Memory: {:.2} MB → {:.2} MB ({:+.1}%)",
             initial_memory as f64 / 1024.0 / 1024.0,
             final_memory as f64 / 1024.0 / 1024.0,
             memory_growth_percent);

    assert!(memory_growth_percent < 5.0,
            "Memory grew {:.1}% (expected <5%)", memory_growth_percent);

    // Check for unbounded growth trend
    let slope = calculate_linear_regression_slope(&memory_samples);
    println!("Memory growth rate: {:.2} MB/min", slope);
    assert!(slope < 0.1, "Memory growth trend too high: {:.2} MB/min", slope);

    // Verify all buffers released
    tokio::time::sleep(Duration::from_secs(2)).await; // Allow cleanup
    let active_buffers = server.get_active_buffer_count().await.unwrap();
    assert_eq!(active_buffers, 0, "Buffers not released: {}", active_buffers);

    println!("✅ Memory leak test PASSED");
}
```

---

### Scenario 7: Error Recovery - Corrupted File

**Requirements:** [DBD-DEC-050], [DBD-FLOW-060]

**Goal:** Verify system gracefully handles corrupted audio files without crashing

**Priority:** HIGH (Robustness requirement)

#### Setup

1. Start test server
2. Create 3 passages:
   - Passage A: Valid file
   - Passage B: Corrupted file (truncated MP3)
   - Passage C: Valid file
3. Start event monitor

#### Steps

1. Enqueue all 3 passages
2. Monitor playback of passage A
3. Observe error when passage B decode fails
4. Verify passage B skipped
5. Verify passage C plays successfully

#### Expected Events

- `PlaybackStarted` (A)
- `PlaybackCompleted` (A)
- `DecodingStarted` (B)
- `DecodingError` (B) **← Error event**
- `PassageSkipped` (B)
- `PlaybackStarted` (C)
- `PlaybackCompleted` (C)

#### Assertions

```rust
// 1. Passage A plays successfully
let a_completed = events.wait_for("PlaybackCompleted", Duration::from_secs(15)).await;
assert!(a_completed.is_some());

// 2. Error event emitted for passage B
let b_error = events.wait_for("DecodingError", Duration::from_secs(5)).await;
assert!(b_error.is_some());
assert!(b_error.unwrap().error_message.contains("decode failed"));

// 3. Passage B skipped
let b_skipped = events.wait_for("PassageSkipped", Duration::from_secs(2)).await;
assert!(b_skipped.is_some());

// 4. Passage C plays successfully (recovery)
let c_started = events.wait_for("PlaybackStarted", Duration::from_secs(2)).await;
assert!(c_started.is_some());

let c_completed = events.wait_for("PlaybackCompleted", Duration::from_secs(15)).await;
assert!(c_completed.is_some());

// 5. No audio disruption (no clicks/pops at error point)
let error_sample_index = /* from error timestamp */;
let error_region = &samples[error_sample_index..error_sample_index + 4410];
assert_eq!(detect_clicks(error_region, 44100).len(), 0);
assert_eq!(detect_pops(error_region, 44100).len(), 0);

// 6. Server still running (no crash)
let health = server.check_health().await;
assert!(health.is_ok());
```

---

### Scenario 8: Sample Rate Variations

**Requirements:** [SRC-CONV-010], [DBD-RSMP-010], [DBD-RSMP-020]

**Goal:** Verify accurate resampling across different source sample rates

**Priority:** HIGH (Core format support)

#### Setup

1. Start test server
2. Prepare test files at different sample rates:
   - 44.1kHz (CD quality)
   - 48kHz (DAT/DVD)
   - 96kHz (high-res)
   - 22.05kHz (half CD)
3. Create passages from each file

#### Steps

1. Enqueue passages with different sample rates in sequence
2. Monitor crossfades between different sample rate passages
3. Verify no clicks at sample rate transitions
4. Verify timing accuracy maintained

#### Expected Events

- `PlaybackStarted` (for each passage)
- `SampleRateConverted` (when source rate ≠ working rate)
- `CrossfadeStarted` (between passages)
- `CrossfadeCompleted`
- `PlaybackCompleted` (for each passage)

#### Assertions

```rust
// 1. All passages decode successfully
for passage in &passages {
    let completed = events.wait_for("PlaybackCompleted", Duration::from_secs(20)).await;
    assert!(completed.is_some(), "Passage {:?} failed to complete", passage.file);
}

// 2. No clicks at sample rate transitions
// Transition points are at crossfades between different-rate files
for (i, transition_point) in transition_sample_indices.iter().enumerate() {
    let region = &samples[*transition_point..*transition_point + 8820]; // 200ms
    let clicks = detect_clicks(region, 44100);
    assert_eq!(clicks.len(), 0, "Clicks at transition {}: {:?}", i, clicks);
}

// 3. Timing accuracy maintained across sample rates
// All passages are 10 seconds, should take exactly 10s each regardless of source rate
for (i, passage_duration) in passage_durations.iter().enumerate() {
    let timing_error = (*passage_duration - 10.0).abs();
    assert!(timing_error < 0.05,
            "Passage {} timing error: {:.3}s", i, timing_error);
}

// 4. Audio quality preserved (no resampling artifacts)
let full_analysis = AudioAnalysis::analyze(&samples, 44100);
assert_eq!(full_analysis.clicks_detected, 0);
assert_eq!(full_analysis.pops_detected, 0);
```

---

### Scenario 9: Format Variations

**Requirements:** [DBD-DEC-010], [DBD-FMT-010]

**Goal:** Verify decoder handles multiple audio formats correctly

**Priority:** HIGH (Format compatibility)

#### Setup

1. Start test server
2. Prepare test files in different formats:
   - MP3 (lossy, most common)
   - FLAC (lossless)
   - OGG Vorbis (lossy)
   - AAC (lossy)
   - WAV (uncompressed)
3. Same musical content in each format

#### Steps

1. Enqueue passages of different formats in sequence
2. Monitor playback and crossfades
3. Verify all formats decode correctly
4. Verify crossfades work across format boundaries

#### Expected Events

- `PlaybackStarted` (for each format)
- `FormatDetected` (format type)
- `CrossfadeStarted` (between formats)
- `PlaybackCompleted` (for each format)

#### Assertions

```rust
// 1. All formats decode successfully
let format_results: HashMap<String, bool> = HashMap::new();
for (format, passage) in &format_passages {
    let completed = events.wait_for("PlaybackCompleted", Duration::from_secs(20)).await;
    format_results.insert(format.clone(), completed.is_some());
}

assert!(format_results.values().all(|&v| v),
        "Not all formats decoded: {:?}", format_results);

// 2. Crossfades work across format boundaries
// E.g., MP3 → FLAC crossfade should be smooth
for crossfade_region in &format_crossfade_regions {
    let clicks = detect_clicks(crossfade_region, 44100);
    let pops = detect_pops(crossfade_region, 44100);
    assert_eq!(clicks.len(), 0);
    assert_eq!(pops.len(), 0);
}

// 3. Audio quality maintained for all formats
// FLAC and WAV should have highest quality, lossy formats acceptable
for (format, passage_samples) in &format_sample_regions {
    let snr = calculate_signal_to_noise_ratio(passage_samples);

    if format == "FLAC" || format == "WAV" {
        assert!(snr > 80.0, "Lossless format {} has low SNR: {:.1}dB", format, snr);
    } else {
        assert!(snr > 40.0, "Lossy format {} has low SNR: {:.1}dB", format, snr);
    }
}

// 4. Timing accuracy consistent across formats
for (format, duration) in &format_durations {
    let timing_error = (*duration - 10.0).abs();
    assert!(timing_error < 0.05,
            "Format {} timing error: {:.3}s", format, timing_error);
}
```

---

### Scenario 10: Edge Cases

**Requirements:** [DBD-FADE-020], [DBD-FADE-060], [DBD-BUF-060]

**Goal:** Verify graceful handling of edge case passage configurations

**Priority:** MEDIUM (Robustness against unusual inputs)

#### Setup

1. Start test server
2. Create edge case passages:
   - **Zero-length passage:** start_time == end_time
   - **Passage at file end:** last 5 seconds of file
   - **Full-file passage:** entire file (start=0, end=file_duration)
   - **Overlapping fades:** fade_out_point < fade_in_point (crossfade)
   - **Maximum duration passage:** Very long passage (>1 hour)

#### Steps

1. Attempt to enqueue each edge case passage
2. Monitor behavior for each case
3. Verify appropriate handling (skip, error, or success)

#### Expected Behavior

| Edge Case | Expected Behavior | Assertion |
|-----------|-------------------|-----------|
| Zero-length | Rejected with validation error | `enqueue_result.is_err()` |
| At file end | Plays successfully | `PlaybackCompleted` event |
| Full file | Plays successfully | Duration matches file |
| Overlapping fades | Rejected with validation error | `enqueue_result.is_err()` |
| Max duration | Plays successfully (may take time) | `PlaybackCompleted` eventually |

#### Assertions

```rust
// 1. Zero-length passage rejected
let zero_length_result = server.enqueue_passage(zero_length_passage).await;
assert!(zero_length_result.is_err());
assert!(zero_length_result.unwrap_err().to_string().contains("start_time >= end_time"));

// 2. Passage at file end plays correctly
let end_passage_result = server.enqueue_passage(end_passage).await;
assert!(end_passage_result.is_ok());

let completed = events.wait_for("PlaybackCompleted", Duration::from_secs(10)).await;
assert!(completed.is_some());

// Verify audio output (last 5 seconds of file)
let samples = audio_capture.get_samples();
let expected_sample_count = 5 * 44100 * 2; // 5s stereo
assert!((samples.len() as i32 - expected_sample_count as i32).abs() < 100);

// 3. Full-file passage plays entire file
let full_file_result = server.enqueue_passage(full_file_passage).await;
assert!(full_file_result.is_ok());

let completed = events.wait_for("PlaybackCompleted", Duration::from_secs(180)).await;
assert!(completed.is_some());

let actual_duration = audio_capture.get_duration();
let expected_duration = get_file_duration(&full_file_passage.file_path);
let timing_error = (actual_duration - expected_duration).abs();
assert!(timing_error < 0.1);

// 4. Overlapping fades rejected
let overlapping_result = server.enqueue_passage(overlapping_fades_passage).await;
assert!(overlapping_result.is_err());

// 5. Maximum duration passage eventually completes
let max_duration_result = server.enqueue_passage(max_duration_passage).await;
assert!(max_duration_result.is_ok());

// Don't wait for full completion (would take >1 hour), just verify it starts
let started = events.wait_for("PlaybackStarted", Duration::from_secs(5)).await;
assert!(started.is_some());

// Skip to avoid waiting full duration
server.skip_next().await.expect("Skip failed");
```

---

## Success Criteria Summary

### Performance Benchmarks

| Metric | Target | Critical |
|--------|--------|----------|
| Startup latency | <100ms | YES (Phase 1 goal) |
| Crossfade quality | 0 clicks/pops | YES |
| RMS continuity | <1dB jumps | YES |
| Memory growth | <5% over 1 hour | NO |
| Queue operations | <10ms latency | NO |

### Audio Quality Standards

**Zero Tolerance:**
- Clicks (frequency spikes >-60dB)
- Pops (amplitude jumps >6dB in <10ms)
- Phase inversions during crossfades

**Acceptable Ranges:**
- RMS variance during Equal-Power crossfade: <0.01
- Timing accuracy: ±100ms over 30-second passage
- Resampling artifacts: SNR >80dB for lossless, >40dB for lossy

### Robustness Requirements

- All tests pass with 0 failures
- No panics or crashes under any scenario
- Graceful error handling for corrupted files
- Stable memory usage over extended playback
- Race condition-free queue operations

---

## Test Execution Plan

### Phase 1: Critical Path (Week 1)
- Scenario 1: Basic Playback (startup latency)
- Scenario 2: Smooth Crossfade
- Scenario 7: Error Recovery

### Phase 2: Stress Testing (Week 2)
- Scenario 3: Rapid Enqueue
- Scenario 4: Skip During Crossfade
- Scenario 6: Long Playback

### Phase 3: Format/Rate Support (Week 3)
- Scenario 8: Sample Rate Variations
- Scenario 9: Format Variations

### Phase 4: Edge Cases (Week 4)
- Scenario 5: Queue Manipulation
- Scenario 10: Edge Cases

---

## Test Infrastructure Implementation

### File Locations

- **Test Server:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/helpers/test_server.rs`
- **Audio Capture:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/helpers/audio_capture.rs`
- **Audio Analysis:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/helpers/audio_analysis.rs`
- **Scenario 1:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/integration_basic_playback.rs`
- **Scenario 2:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/integration_crossfade.rs`
- **Scenario 3:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/integration_rapid_enqueue.rs`
- **Scenario 4:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/integration_skip_during_crossfade.rs`
- **Scenario 5:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/integration_queue_manipulation.rs`
- **Scenario 6:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/integration_memory_leak.rs`
- **Scenario 7:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/integration_error_recovery.rs`
- **Scenario 8:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/integration_sample_rate.rs`
- **Scenario 9:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/integration_format_variations.rs`
- **Scenario 10:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/integration_edge_cases.rs`

### Dependencies

Add to `wkmp-ap/Cargo.toml`:

```toml
[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
axum = "0.7"
tower = "0.4"
serde_json = "1.0"
uuid = { version = "1", features = ["v4"] }
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio"] }
futures = "0.3"
realfft = "3.3"  # For FFT analysis
hound = "3.5"    # For WAV file testing
```

---

**Document Version:** 1.0
**Created:** 2025-10-19
**Status:** Draft
**Next Review:** After Phase 1 implementation

**Maintained By:** Agent 2B - Integration Test Design

---

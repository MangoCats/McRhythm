# TC-S-XFD-TIME-01: Sample-Accurate Crossfade Timing

**Test ID:** TC-S-XFD-TIME-01
**Requirement:** XFD-TIME-010 - Trigger crossfade at exactly fade_out_start_time
**Test Type:** System/End-to-End Test
**Priority:** High
**Est. Effort:** 4 hours (2h test implementation + 2h verification tooling)
**Last Reviewed:** 2025-10-26 (Tick consistency - updated all timing fields to use ticks per SPEC017)

---

## Objective

Verify that crossfade is triggered at exactly the sample-accurate fade_out_start_time specified for a passage, with timing accuracy within ±1ms (±44.1 samples at 44100 Hz).

---

## Scope

- **Components Under Test:** PlaybackEngine, Mixer, DecoderChain, Queue
- **Dependencies:** SPEC002 (Crossfade), SPEC016 (Decoder Buffer), Database (queue table)
- **Environment:** Development system with audio output device

---

## Test Specification

### Given (Initial Conditions)

1. **Database Setup:**
   - Queue table contains 2 passages (Passage A and Passage B)
   - Passage A (all timing in ticks per SPEC017):
     - `start_time_ticks`: 0
     - `end_time_ticks`: 1693440000 (60 seconds × 28,224,000 ticks/s)
     - `fade_out_start_time_ticks`: 1552320000 (55 seconds)
     - `fade_out_duration_ticks`: 141120000 (5 seconds)
     - `lead_out_point_ticks`: 1693440000
   - Passage B (all timing in ticks):
     - `start_time_ticks`: 0
     - `end_time_ticks`: 1693440000
     - `fade_in_start_time_ticks`: 0
     - `fade_in_duration_ticks`: 141120000
     - `lead_in_point_ticks`: 0
   - Queue entries:
     - Entry 1 → Passage A, play_order: 10
     - Entry 2 → Passage B, play_order: 20

2. **System State:**
   - wkmp-ap service started
   - Queue restored from database
   - Audio output device open and functional
   - EventBus initialized and subscribed to by test harness

3. **Test Files:**
   - Audio files: silence_60s_44100hz_stereo.flac (known test file)
   - Duration: exactly 60.0 seconds
   - Sample rate: 44100 Hz
   - Channels: 2 (stereo)

---

### When (Action/Input)

1. **API Call:** POST /playback/play
   - Triggers playback of queue
2. **Monitoring:** Test harness subscribes to SSE event stream
3. **Timing Capture:** Record timestamps for:
   - PassageStarted event (Passage A)
   - PlaybackProgress events (500ms intervals)
   - CrossfadeStarted event (if emitted)
   - PassageStarted event (Passage B)
   - PassageCompleted event (Passage A)

---

### Then (Expected Result)

1. **Crossfade Start Time (Sample-Accurate):**
   - Crossfade triggered when Passage A playback position reaches 55.0 seconds
   - Calculation: 55.0s × 44100 samples/s = 2425500 samples
   - Tolerance: ±44 samples (±1ms at 44100 Hz sample rate)

2. **Event Sequence:**
   ```
   T+0.0s:   PassageStarted(passage_a)
   T+54.5s:  PlaybackProgress(passage_a, position_ticks=1538208000) [last before crossfade]
   T+55.0s:  CrossfadeStarted(outgoing=passage_a, incoming=passage_b) [TIMING CRITICAL]
   T+55.0s:  PassageStarted(passage_b) [overlaps during crossfade]
   T+60.0s:  PassageCompleted(passage_a)
   T+60.0s:  Crossfade continues (5s duration)
   ```

3. **Audio Verification:**
   - Capture audio output during test
   - Analyze captured audio:
     - Passage A volume begins reducing at T+55.0s
     - Passage B volume begins increasing at T+55.0s
     - Crossfade duration: 5.0 seconds (within ±10ms)

---

## Verification Steps

### Automated Verification (Rust Test)

```rust
#[tokio::test]
async fn test_sample_accurate_crossfade_timing() {
    // Setup
    let db = setup_test_database().await;
    insert_test_passages(&db, passage_a_config(), passage_b_config()).await;
    let (engine, event_rx) = start_playback_engine(&db).await;

    // Act
    engine.play().await.unwrap();

    // Monitor events
    let mut events = Vec::new();
    let timeout = Duration::from_secs(70); // 60s + 10s margin
    let start_time = Instant::now();

    while start_time.elapsed() < timeout {
        if let Ok(event) = tokio::time::timeout(
            Duration::from_millis(100),
            event_rx.recv()
        ).await {
            events.push((Instant::now(), event));
        }
    }

    // Verify timing
    let passage_a_started = find_event(&events, |e| matches!(e, PassageStarted { id } if id == passage_a_id));
    let crossfade_started = find_event(&events, |e| matches!(e, CrossfadeStarted { .. }));

    let crossfade_offset = crossfade_started.0 - passage_a_started.0;
    let expected_offset_ticks = 1552320000; // 55 seconds
    let tolerance_ms = 1; // ±1ms

    assert!(
        (crossfade_offset.as_millis() as i64 - expected_offset_ms).abs() <= tolerance_ms,
        "Crossfade timing outside tolerance: expected {}ms, got {}ms (delta: {}ms)",
        expected_offset_ms,
        crossfade_offset.as_millis(),
        (crossfade_offset.as_millis() as i64 - expected_offset_ms).abs()
    );
}
```

### Manual Verification (Audio Analysis)

1. **Capture Audio Output:**
   ```bash
   # Record audio during test
   arecord -D loopback -f cd -d 70 test_output.wav
   ```

2. **Analyze with Audacity:**
   - Open test_output.wav
   - Locate crossfade region (around 55-60 seconds)
   - Measure exact start of volume reduction (Passage A fade-out)
   - Expected: 55.000s ± 0.001s

3. **Automated Audio Analysis:**
   ```rust
   // Use hound crate to analyze captured audio
   let reader = hound::WavReader::open("test_output.wav")?;
   let samples: Vec<i16> = reader.samples().collect()?;

   // Find crossfade start by detecting volume decrease in Passage A
   let crossfade_start_sample = detect_volume_decrease(&samples, threshold=0.95);
   let crossfade_start_time_s = crossfade_start_sample as f64 / 44100.0;

   assert!(
       (crossfade_start_time_s - 55.0).abs() < 0.001,
       "Audio analysis: crossfade started at {}s (expected 55.000s)",
       crossfade_start_time_s
   );
   ```

---

## Pass Criteria

**Test PASSES if ALL of the following are true:**

1. ✅ CrossfadeStarted event emitted at passage position 55.0s ± 1ms
2. ✅ Audio analysis confirms volume reduction starts at 55.0s ± 1ms
3. ✅ Crossfade duration is 5.0s ± 10ms (measured from audio or events)
4. ✅ No audio glitches or dropouts during crossfade
5. ✅ PassageCompleted(passage_a) emitted after crossfade completes

---

## Fail Criteria

**Test FAILS if ANY of the following are true:**

1. ❌ CrossfadeStarted timing outside ±1ms tolerance
2. ❌ Audio analysis shows volume change at incorrect time (>1ms error)
3. ❌ Audio glitches or dropouts during crossfade
4. ❌ Crossfade duration incorrect (>10ms error)
5. ❌ Events emitted in wrong order
6. ❌ Test crashes or panics

---

## Test Data

### Passage A Configuration
```json
{
  "passage_id": "550e8400-e29b-41d4-a716-446655440001",
  "recording_id": "550e8400-e29b-41d4-a716-446655440010",
  "file_path": "/path/to/test/silence_60s_44100hz_stereo.flac",
  "start_time_ms": 0,
  "end_time_ms": 60000,
  "fade_in_start_time_ms": 0,
  "fade_in_point_ms": 2000,
  "fade_in_duration_ms": 2000,
  "fade_out_start_time_ms": 55000,
  "fade_out_point_ms": 58000,
  "fade_out_duration_ms": 5000,
  "lead_in_point_ms": 0,
  "lead_out_point_ms": 60000,
  "fade_in_curve_type": "equal_power",
  "fade_out_curve_type": "equal_power"
}
```

### Passage B Configuration
```json
{
  "passage_id": "550e8400-e29b-41d4-a716-446655440002",
  "recording_id": "550e8400-e29b-41d4-a716-446655440020",
  "file_path": "/path/to/test/silence_60s_44100hz_stereo.flac",
  "start_time_ms": 0,
  "end_time_ms": 60000,
  "fade_in_start_time_ms": 0,
  "fade_in_point_ms": 2000,
  "fade_in_duration_ms": 5000,
  "lead_in_point_ms": 0,
  "lead_out_point_ms": 60000,
  "fade_in_curve_type": "equal_power",
  "fade_out_curve_type": "equal_power"
}
```

---

## Dependencies

- Test audio file: `tests/data/silence_60s_44100hz_stereo.flac`
- Audio analysis tool: Audacity or programmatic analysis (hound crate)
- Event capture: SSE client or EventBus subscription
- Timing analysis: Rust std::time::Instant for high-precision timing

---

## Notes

- This test verifies the MOST CRITICAL requirement for WKMP: sample-accurate crossfading
- Timing tolerance (±1ms) derived from SPEC022 PERF-XFD-010
- Test can be run on development system, but should also be validated on Pi Zero 2W
- Audio capture may require loopback device setup on Linux (snd-aloop module)

---

**Status:** Ready for implementation
**Last Updated:** 2025-10-26

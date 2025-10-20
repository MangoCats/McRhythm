# Unit Test Specifications for WKMP Audio Player

**Document ID:** IMPL-TESTS-001
**Version:** 1.0
**Date:** 2025-10-19
**Author:** Agent 2A - Unit Test Design Agent
**Status:** Draft

## Purpose

This document specifies comprehensive unit tests designed to address the 68 implementation gaps identified in Phase 1 Gap Analysis (IMPL-ANALYSIS-002). Tests are prioritized by gap severity: CRITICAL (15), HIGH (18), MEDIUM (21), LOW (8).

## Test Coverage Goals

- **Current Coverage:** 65% (estimated from 226 existing tests across 3,325 test LOC)
- **Target Coverage:** 80% (+15%)
- **New Tests Designed:** 87 unit tests
- **Expected Coverage After:** ~78-80%

## Gap Analysis Reference

- **Source:** `/home/sw/Dev/McRhythm/docs/validation/IMPL-ANALYSIS-002-gap-analysis.json`
- **Specifications:** SPEC016 (Decoder Buffer Design), SPEC017 (Sample Rate Conversion)
- **Total Requirements:** 122 (60 SPEC016 + 62 SPEC017)
- **Implementation Gaps:** 68 issues

---

## Priority 1: CRITICAL Gaps (15 issues)

### Module: common/src/timing.rs (NEW MODULE)

This module must be created to support tick-based timing system (28,224,000 Hz).

#### Test Group 1: Tick Rate Constants

##### Test: tick_rate_constant_value
**Requirement:** [SRC-TICK-020]
**Priority:** CRITICAL
**Severity:** CRITICAL

**Given:** TICK_RATE constant is defined
**When:** Reading the constant value
**Then:** Value equals 28,224,000
**And:** Type is i64

```rust
#[test]
fn test_tick_rate_constant_value() {
    assert_eq!(TICK_RATE, 28_224_000_i64);
}
```

##### Test: tick_rate_divides_all_sample_rates
**Requirement:** [SRC-TICK-040]
**Priority:** CRITICAL
**Severity:** LOW

**Given:** TICK_RATE = 28,224,000 and all supported sample rates
**When:** Dividing TICK_RATE by each sample rate
**Then:** Remainder equals zero for all rates
**And:** No fractional values exist

```rust
#[test]
fn test_tick_rate_divides_all_sample_rates() {
    const SUPPORTED_RATES: [u32; 11] = [
        8000, 11025, 16000, 22050, 32000, 44100,
        48000, 88200, 96000, 176400, 192000
    ];

    for rate in SUPPORTED_RATES {
        let ticks_per_sample = TICK_RATE / rate as i64;
        let remainder = TICK_RATE % rate as i64;

        assert_eq!(remainder, 0,
            "TICK_RATE {} must divide evenly into sample rate {}",
            TICK_RATE, rate);
        assert!(ticks_per_sample > 0,
            "ticks_per_sample must be positive for rate {}", rate);
    }
}
```

#### Test Group 2: Millisecond ↔ Tick Conversions

##### Test: ms_to_ticks_accuracy
**Requirement:** [SRC-API-020]
**Priority:** CRITICAL
**Severity:** MEDIUM

**Given:** Various millisecond values (0, 1, 1000, 60000)
**When:** Converting to ticks using ms_to_ticks()
**Then:** Result equals ms × 28,224
**And:** No precision loss for any input

```rust
#[test]
fn test_ms_to_ticks_accuracy() {
    assert_eq!(ms_to_ticks(0), 0);
    assert_eq!(ms_to_ticks(1), 28_224);
    assert_eq!(ms_to_ticks(1000), 28_224_000);
    assert_eq!(ms_to_ticks(60000), 1_693_440_000);

    // 5 minutes = 300 seconds = 300,000 ms
    assert_eq!(ms_to_ticks(300_000), 8_467_200_000);
}
```

##### Test: ticks_to_ms_roundtrip
**Requirement:** [SRC-API-030]
**Priority:** CRITICAL
**Severity:** MEDIUM

**Given:** Various tick values divisible by 28,224
**When:** Converting ticks → ms → ticks
**Then:** Result equals original tick value
**And:** No rounding errors for tick-aligned values

```rust
#[test]
fn test_ticks_to_ms_roundtrip() {
    let test_cases = vec![
        0_i64,
        28_224,           // 1 ms
        28_224_000,       // 1 second
        141_120_000,      // 5 seconds
        1_693_440_000,    // 60 seconds
    ];

    for original_ticks in test_cases {
        let ms = ticks_to_ms(original_ticks);
        let roundtrip_ticks = ms_to_ticks(ms);

        assert_eq!(roundtrip_ticks, original_ticks,
            "Roundtrip failed for {} ticks", original_ticks);
    }
}
```

##### Test: ticks_to_ms_rounding_behavior
**Requirement:** [SRC-API-030]
**Priority:** CRITICAL
**Severity:** MEDIUM

**Given:** Tick values NOT aligned to millisecond boundaries
**When:** Converting to milliseconds
**Then:** Result rounds down (truncates)
**And:** Precision loss is documented and acceptable

```rust
#[test]
fn test_ticks_to_ms_rounding_behavior() {
    // 28,224 ticks = 1 ms exactly
    assert_eq!(ticks_to_ms(28_224), 1);

    // 28,223 ticks < 1 ms, should round down to 0
    assert_eq!(ticks_to_ms(28_223), 0);

    // 28,225 ticks > 1 ms, should round down to 1
    assert_eq!(ticks_to_ms(28_225), 1);

    // 56,447 ticks = 1.999... ms, should round down to 1
    assert_eq!(ticks_to_ms(56_447), 1);

    // 56,448 ticks = 2 ms exactly
    assert_eq!(ticks_to_ms(56_448), 2);
}
```

#### Test Group 3: Tick ↔ Sample Conversions

##### Test: ticks_to_samples_accuracy_44100
**Requirement:** [SRC-WSR-030, SRC-WSR-040]
**Priority:** CRITICAL
**Severity:** CRITICAL

**Given:** Tick values and working_sample_rate = 44,100 Hz
**When:** Converting ticks to samples
**Then:** Result equals (ticks × 44100) ÷ 28,224,000
**And:** For 44.1kHz, optimized formula: ticks ÷ 640

```rust
#[test]
fn test_ticks_to_samples_accuracy_44100() {
    const RATE_44100: u32 = 44100;

    // 0 ticks = 0 samples
    assert_eq!(ticks_to_samples(0, RATE_44100), 0);

    // 640 ticks = 1 sample @ 44.1kHz
    assert_eq!(ticks_to_samples(640, RATE_44100), 1);

    // 28,224,000 ticks = 1 second = 44,100 samples @ 44.1kHz
    assert_eq!(ticks_to_samples(28_224_000, RATE_44100), 44_100);

    // 141,120,000 ticks = 5 seconds = 220,500 samples @ 44.1kHz
    assert_eq!(ticks_to_samples(141_120_000, RATE_44100), 220_500);
}
```

##### Test: ticks_to_samples_accuracy_48000
**Requirement:** [SRC-WSR-030]
**Priority:** CRITICAL
**Severity:** CRITICAL

**Given:** Tick values and working_sample_rate = 48,000 Hz
**When:** Converting ticks to samples
**Then:** Result equals (ticks × 48000) ÷ 28,224,000
**And:** All divisions have zero remainder

```rust
#[test]
fn test_ticks_to_samples_accuracy_48000() {
    const RATE_48000: u32 = 48000;

    // 588 ticks = 1 sample @ 48kHz (28,224,000 / 48,000 = 588)
    assert_eq!(ticks_to_samples(588, RATE_48000), 1);

    // 28,224,000 ticks = 1 second = 48,000 samples @ 48kHz
    assert_eq!(ticks_to_samples(28_224_000, RATE_48000), 48_000);

    // 141,120,000 ticks = 5 seconds = 240,000 samples @ 48kHz
    assert_eq!(ticks_to_samples(141_120_000, RATE_48000), 240_000);
}
```

##### Test: ticks_to_samples_all_supported_rates
**Requirement:** [SRC-WSR-030]
**Priority:** CRITICAL
**Severity:** CRITICAL

**Given:** All 11 supported sample rates
**When:** Converting 1 second (28,224,000 ticks) to samples
**Then:** Result equals the sample rate value
**And:** No rounding errors for any rate

```rust
#[test]
fn test_ticks_to_samples_all_supported_rates() {
    const ONE_SECOND_TICKS: i64 = 28_224_000;

    let test_cases = vec![
        (8000, 8000),
        (11025, 11025),
        (16000, 16000),
        (22050, 22050),
        (32000, 32000),
        (44100, 44100),
        (48000, 48000),
        (88200, 88200),
        (96000, 96000),
        (176400, 176400),
        (192000, 192000),
    ];

    for (sample_rate, expected_samples) in test_cases {
        let samples = ticks_to_samples(ONE_SECOND_TICKS, sample_rate);
        assert_eq!(samples, expected_samples,
            "1 second @ {}Hz should equal {} samples",
            sample_rate, expected_samples);
    }
}
```

##### Test: samples_to_ticks_accuracy
**Requirement:** [SRC-CONV-030]
**Priority:** CRITICAL
**Severity:** CRITICAL

**Given:** Sample counts and sample rates
**When:** Converting samples to ticks
**Then:** Result equals samples × (28,224,000 ÷ sample_rate)
**And:** Roundtrip conversion is exact

```rust
#[test]
fn test_samples_to_ticks_accuracy() {
    // 44.1kHz: 1 sample = 640 ticks
    assert_eq!(samples_to_ticks(1, 44100), 640);
    assert_eq!(samples_to_ticks(44100, 44100), 28_224_000);

    // 48kHz: 1 sample = 588 ticks
    assert_eq!(samples_to_ticks(1, 48000), 588);
    assert_eq!(samples_to_ticks(48000, 48000), 28_224_000);

    // 8kHz: 1 sample = 3,528 ticks
    assert_eq!(samples_to_ticks(1, 8000), 3_528);
    assert_eq!(samples_to_ticks(8000, 8000), 28_224_000);
}
```

##### Test: samples_to_ticks_roundtrip
**Requirement:** [SRC-CONV-030]
**Priority:** CRITICAL
**Severity:** CRITICAL

**Given:** Various sample counts at different rates
**When:** Converting samples → ticks → samples
**Then:** Result equals original sample count
**And:** No precision loss

```rust
#[test]
fn test_samples_to_ticks_roundtrip() {
    let test_cases = vec![
        (44100, 1),
        (44100, 100),
        (44100, 44100),    // 1 second
        (44100, 220500),   // 5 seconds
        (48000, 1),
        (48000, 48000),
        (48000, 240000),
        (8000, 8000),
        (192000, 192000),
    ];

    for (rate, original_samples) in test_cases {
        let ticks = samples_to_ticks(original_samples, rate);
        let roundtrip_samples = ticks_to_samples(ticks, rate);

        assert_eq!(roundtrip_samples, original_samples,
            "Roundtrip failed for {} samples @ {}Hz",
            original_samples, rate);
    }
}
```

#### Test Group 4: Edge Cases and Overflow

##### Test: tick_overflow_detection
**Requirement:** [SRC-PREC-020]
**Priority:** CRITICAL
**Severity:** LOW

**Given:** i64::MAX and very large tick values
**When:** Performing tick calculations
**Then:** Operations near i64::MAX are handled safely
**And:** Maximum representable time is ~10.36 years

```rust
#[test]
fn test_tick_overflow_detection() {
    // Maximum representable time in ticks
    const MAX_TICKS: i64 = i64::MAX;

    // Convert to seconds (should be ~326,869,269 seconds = ~10.36 years)
    let max_seconds = ticks_to_seconds(MAX_TICKS);
    assert!(max_seconds > 326_000_000.0 && max_seconds < 327_000_000.0,
        "Max representable time should be ~10.36 years");

    // Verify we can represent 10 years safely
    let ten_years_seconds = 10 * 365 * 24 * 60 * 60;
    let ten_years_ticks = seconds_to_ticks(ten_years_seconds as f64);
    assert!(ten_years_ticks < MAX_TICKS,
        "10 years should be representable in i64 ticks");
}
```

##### Test: negative_tick_handling
**Requirement:** [SRC-PREC-010]
**Priority:** CRITICAL
**Severity:** LOW

**Given:** Negative tick values
**When:** Converting or performing arithmetic
**Then:** Operations handle negatives correctly
**And:** Useful for relative time calculations

```rust
#[test]
fn test_negative_tick_handling() {
    // Negative ticks should convert to negative milliseconds
    assert_eq!(ticks_to_ms(-28_224), -1);
    assert_eq!(ticks_to_ms(-28_224_000), -1000);

    // Negative milliseconds should convert to negative ticks
    assert_eq!(ms_to_ticks(-1), -28_224);
    assert_eq!(ms_to_ticks(-1000), -28_224_000);

    // Tick arithmetic with negatives
    let start_ticks = ms_to_ticks(5000);  // 5 seconds
    let duration_ticks = ms_to_ticks(3000);  // 3 seconds
    let offset_ticks = -ms_to_ticks(1000);  // -1 second

    assert_eq!(start_ticks + duration_ticks, ms_to_ticks(8000));
    assert_eq!(start_ticks + offset_ticks, ms_to_ticks(4000));
}
```

##### Test: zero_sample_rate_protection
**Requirement:** [SRC-WSR-030]
**Priority:** CRITICAL
**Severity:** MEDIUM

**Given:** Sample rate of zero
**When:** Attempting tick-to-sample conversion
**Then:** Function panics or returns error
**And:** Division by zero is prevented

```rust
#[test]
#[should_panic(expected = "sample_rate must be > 0")]
fn test_zero_sample_rate_protection() {
    ticks_to_samples(28_224_000, 0);
}
```

#### Test Group 5: Cross-Rate Conversion Examples

##### Test: crossfade_duration_example
**Requirement:** [SRC-EXAM-020, SRC-EXAM-030]
**Priority:** CRITICAL
**Severity:** LOW

**Given:** Example crossfade duration of 84,672,000 ticks (3 seconds)
**When:** Converting to samples at 44.1kHz and 48kHz
**Then:** 44.1kHz yields 132,300 samples
**And:** 48kHz yields 144,000 samples
**And:** Both conversions have zero error

```rust
#[test]
fn test_crossfade_duration_example() {
    const CROSSFADE_TICKS: i64 = 84_672_000;  // 3 seconds

    // 44.1kHz: 84,672,000 ticks = 132,300 samples
    let samples_44100 = ticks_to_samples(CROSSFADE_TICKS, 44100);
    assert_eq!(samples_44100, 132_300,
        "3 second crossfade @ 44.1kHz should be 132,300 samples");

    // 48kHz: 84,672,000 ticks = 144,000 samples
    let samples_48000 = ticks_to_samples(CROSSFADE_TICKS, 48000);
    assert_eq!(samples_48000, 144_000,
        "3 second crossfade @ 48kHz should be 144,000 samples");

    // Verify both convert back to same tick value
    assert_eq!(samples_to_ticks(samples_44100, 44100), CROSSFADE_TICKS);
    assert_eq!(samples_to_ticks(samples_48000, 48000), CROSSFADE_TICKS);
}
```

##### Test: five_second_passage_example
**Requirement:** [SRC-CONV-040]
**Priority:** CRITICAL
**Severity:** LOW

**Given:** 5 seconds at 44.1kHz
**When:** Converting through tick system
**Then:** 5s = 220,500 samples = 141,120,000 ticks
**And:** All conversions are exact

```rust
#[test]
fn test_five_second_passage_example() {
    const FIVE_SECONDS_MS: u64 = 5000;
    const FIVE_SECONDS_TICKS: i64 = 141_120_000;
    const FIVE_SECONDS_SAMPLES_44100: usize = 220_500;

    // ms → ticks
    assert_eq!(ms_to_ticks(FIVE_SECONDS_MS), FIVE_SECONDS_TICKS);

    // ticks → samples @ 44.1kHz
    assert_eq!(ticks_to_samples(FIVE_SECONDS_TICKS, 44100),
               FIVE_SECONDS_SAMPLES_44100);

    // samples @ 44.1kHz → ticks
    assert_eq!(samples_to_ticks(FIVE_SECONDS_SAMPLES_44100, 44100),
               FIVE_SECONDS_TICKS);

    // ticks → ms
    assert_eq!(ticks_to_ms(FIVE_SECONDS_TICKS), FIVE_SECONDS_MS);
}
```

---

### Module: wkmp-ap/src/db/passages.rs (TIMING FIELD FIXES)

These tests verify the critical database schema fixes for tick-based timing.

#### Test Group 6: Database Timing Field Types

##### Test: passage_timing_fields_use_i64_ticks
**Requirement:** [SRC-DB-011 through SRC-DB-016]
**Priority:** CRITICAL
**Severity:** CRITICAL

**Given:** PassageWithTiming struct loaded from database
**When:** Examining timing field types
**Then:** All 6 timing fields are i64 ticks
**And:** No u64 milliseconds or f64 seconds exist

```rust
#[tokio::test]
async fn test_passage_timing_fields_use_i64_ticks() {
    let pool = setup_test_db().await;

    // Create test passage with tick-based timing
    let passage_id = create_test_passage_with_ticks(&pool).await;

    // Load passage
    let passage = get_passage_with_timing(&pool, passage_id)
        .await
        .expect("Failed to load passage");

    // Verify all timing fields are i64
    use std::any::TypeId;
    assert_eq!(TypeId::of_val(&passage.start_time_ticks),
               TypeId::of::<i64>());
    assert_eq!(TypeId::of_val(&passage.end_time_ticks),
               TypeId::of::<i64>());
    assert_eq!(TypeId::of_val(&passage.fade_in_point_ticks),
               TypeId::of::<i64>());
    assert_eq!(TypeId::of_val(&passage.fade_out_point_ticks),
               TypeId::of::<i64>());
    assert_eq!(TypeId::of_val(&passage.lead_in_point_ticks),
               TypeId::of::<i64>());
    assert_eq!(TypeId::of_val(&passage.lead_out_point_ticks),
               TypeId::of::<i64>());
}
```

##### Test: database_stores_ticks_as_integer
**Requirement:** [SRC-DB-010]
**Priority:** CRITICAL
**Severity:** CRITICAL

**Given:** Passage saved with tick timing values
**When:** Querying database directly with raw SQL
**Then:** Column types are INTEGER
**And:** Values match tick format exactly

```rust
#[tokio::test]
async fn test_database_stores_ticks_as_integer() {
    let pool = setup_test_db().await;

    // Insert passage with known tick values
    let start_ticks = 141_120_000_i64;  // 5 seconds
    let end_ticks = 282_240_000_i64;    // 10 seconds

    sqlx::query(
        "INSERT INTO passages (id, file_path, start_time, end_time)
         VALUES (?, ?, ?, ?)"
    )
    .bind(Uuid::new_v4())
    .bind("/test/audio.mp3")
    .bind(start_ticks)
    .bind(end_ticks)
    .execute(&pool)
    .await
    .expect("Insert failed");

    // Verify stored as INTEGER (not REAL)
    let row: (i64,) = sqlx::query_as(
        "SELECT typeof(start_time) FROM passages LIMIT 1"
    )
    .fetch_one(&pool)
    .await
    .expect("Type query failed");

    assert_eq!(row.0, "integer",
        "start_time must be stored as INTEGER, not REAL");
}
```

---

## Priority 2: HIGH Priority Gaps (18 issues)

### Module: wkmp-ap/src/playback/decoder_pool.rs (SERIAL EXECUTION)

#### Test Group 7: Serial Decode Execution

##### Test: only_one_decoder_active_at_time
**Requirement:** [DBD-DEC-040]
**Priority:** HIGH
**Severity:** HIGH

**Given:** DecoderPool with priority queue containing 3 decode requests
**When:** Monitoring decoder activity
**Then:** Only one decoder is active at any given time
**And:** Second decoder waits until first completes or pauses

```rust
#[tokio::test]
async fn test_only_one_decoder_active_at_time() {
    let pool = setup_test_decoder_pool().await;
    let buffer_manager = setup_test_buffer_manager();

    // Enqueue 3 decode requests
    let passage_ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    for passage_id in &passage_ids {
        pool.enqueue_decode(DecodeRequest {
            passage_id: *passage_id,
            priority: DecodePriority::Prefetch,
            // ... passage details
        }).await;
    }

    // Monitor active decoder count over time
    tokio::time::sleep(Duration::from_millis(100)).await;
    let active_count = pool.get_active_decoder_count().await;

    assert_eq!(active_count, 1,
        "Only 1 decoder should be active at a time");

    // Verify no parallel execution
    let activity_log = pool.get_decoder_activity_log().await;
    for window in activity_log.windows(2) {
        let overlap = window[0].is_active_at(window[1].start_time);
        assert!(!overlap,
            "Decoders must not overlap - serial execution required");
    }
}
```

##### Test: priority_queue_ordering
**Requirement:** [DBD-DEC-040]
**Priority:** HIGH
**Severity:** HIGH

**Given:** Decode requests with different priorities (Immediate, Next, Prefetch)
**When:** Dequeuing for execution
**Then:** Highest priority request is processed first
**And:** Order is Immediate > Next > Prefetch

```rust
#[tokio::test]
async fn test_priority_queue_ordering() {
    let pool = setup_test_decoder_pool().await;

    // Enqueue in reverse priority order
    let prefetch_id = Uuid::new_v4();
    let next_id = Uuid::new_v4();
    let immediate_id = Uuid::new_v4();

    pool.enqueue_decode(create_decode_request(prefetch_id, DecodePriority::Prefetch)).await;
    pool.enqueue_decode(create_decode_request(next_id, DecodePriority::Next)).await;
    pool.enqueue_decode(create_decode_request(immediate_id, DecodePriority::Immediate)).await;

    // Verify execution order
    let execution_order = pool.get_execution_order().await;
    assert_eq!(execution_order[0], immediate_id, "Immediate should be first");
    assert_eq!(execution_order[1], next_id, "Next should be second");
    assert_eq!(execution_order[2], prefetch_id, "Prefetch should be third");
}
```

##### Test: decode_completion_triggers_next
**Requirement:** [DBD-DEC-040]
**Priority:** HIGH
**Severity:** HIGH

**Given:** Active decoder working on passage A, passage B queued
**When:** Passage A decode completes
**Then:** Decoder immediately starts on passage B
**And:** No idle time between decodes

```rust
#[tokio::test]
async fn test_decode_completion_triggers_next() {
    let pool = setup_test_decoder_pool().await;

    // Create two short test passages
    let passage_a_id = Uuid::new_v4();
    let passage_b_id = Uuid::new_v4();

    pool.enqueue_decode(create_short_decode_request(passage_a_id)).await;
    pool.enqueue_decode(create_short_decode_request(passage_b_id)).await;

    // Wait for passage A to complete
    let start_time = Instant::now();
    pool.wait_for_decode_complete(passage_a_id).await;
    let completion_time = Instant::now();

    // Verify passage B started immediately
    let passage_b_start = pool.get_decode_start_time(passage_b_id).await;
    let gap = passage_b_start.duration_since(completion_time);

    assert!(gap < Duration::from_millis(50),
        "Passage B should start within 50ms of A completing (gap: {:?})", gap);
}
```

---

### Module: wkmp-ap/src/playback/decoder_pool.rs (PRE-BUFFER FADE APPLICATION)

#### Test Group 8: Pre-Buffer Fade Application

##### Test: fade_in_applied_before_buffering
**Requirement:** [DBD-FADE-030]
**Priority:** HIGH
**Severity:** HIGH

**Given:** Passage with fade_in region (8s exponential fade)
**When:** Decoder processes samples in fade-in region
**Then:** Fade curve multiplier is applied to samples
**And:** Faded samples are written to PassageBuffer
**And:** Mixer reads pre-faded samples

```rust
#[tokio::test]
async fn test_fade_in_applied_before_buffering() {
    let pool = setup_test_decoder_pool().await;
    let buffer_manager = setup_test_buffer_manager();

    // Create passage with fade-in: 0s start, 8s fade-in point
    let passage = PassageWithTiming {
        start_time_ticks: 0,
        fade_in_point_ticks: ms_to_ticks(8000),  // 8 seconds
        fade_in_curve: FadeCurve::Exponential,
        // ...
    };

    // Decode passage
    pool.decode_passage(&passage, &buffer_manager).await;

    // Get buffer and examine samples in fade-in region
    let buffer = buffer_manager.get_buffer(passage.id).await;
    let buffer_read = buffer.read().await;

    // First sample should be silent (fade multiplier ≈ 0.0)
    let first_frame = buffer_read.get_sample(0).expect("First sample missing");
    assert!(first_frame.left.abs() < 0.01 && first_frame.right.abs() < 0.01,
        "First sample should be nearly silent due to fade-in");

    // Sample at 4 seconds (middle of fade) should be ~50% amplitude
    let mid_fade_sample = 4 * 44100;  // 4 seconds @ 44.1kHz
    let mid_frame = buffer_read.get_sample(mid_fade_sample).expect("Mid sample missing");
    // Verify fade was applied (actual amplitude depends on source audio)

    // Sample after fade-in point should be full amplitude
    let post_fade_sample = 8 * 44100;  // 8 seconds @ 44.1kHz
    let post_frame = buffer_read.get_sample(post_fade_sample).expect("Post sample missing");
    // Verify no fade attenuation
}
```

##### Test: fade_out_applied_before_buffering
**Requirement:** [DBD-FADE-050]
**Priority:** HIGH
**Severity:** HIGH

**Given:** Passage with fade_out region (last 8s logarithmic fade)
**When:** Decoder processes samples in fade-out region
**Then:** Fade curve multiplier is applied to samples
**And:** Last sample has fade multiplier ≈ 0.0

```rust
#[tokio::test]
async fn test_fade_out_applied_before_buffering() {
    let pool = setup_test_decoder_pool().await;
    let buffer_manager = setup_test_buffer_manager();

    // Create passage: 20s total, fade-out starts at 12s
    let passage = PassageWithTiming {
        start_time_ticks: 0,
        end_time_ticks: ms_to_ticks(20000),  // 20 seconds
        fade_out_point_ticks: ms_to_ticks(12000),  // 12 seconds
        fade_out_curve: FadeCurve::Logarithmic,
        // ...
    };

    pool.decode_passage(&passage, &buffer_manager).await;

    let buffer = buffer_manager.get_buffer(passage.id).await;
    let buffer_read = buffer.read().await;

    // Last sample should be silent (fade multiplier ≈ 0.0)
    let last_sample_idx = buffer_read.sample_count() - 1;
    let last_frame = buffer_read.get_sample(last_sample_idx).expect("Last sample missing");

    assert!(last_frame.left.abs() < 0.01 && last_frame.right.abs() < 0.01,
        "Last sample should be nearly silent due to fade-out");
}
```

##### Test: all_five_fade_curves_supported
**Requirement:** [DBD-FADE-030]
**Priority:** HIGH
**Severity:** MEDIUM

**Given:** Five fade curve types (Linear, Exponential, Logarithmic, SCurve, Cosine)
**When:** Applying each curve to fade-in region
**Then:** Each produces different fade multiplier values
**And:** All curves start at ~0.0 and end at ~1.0

```rust
#[test]
fn test_all_five_fade_curves_supported() {
    use wkmp_common::FadeCurve;

    let fade_duration_samples = 44100;  // 1 second @ 44.1kHz
    let curves = vec![
        FadeCurve::Linear,
        FadeCurve::Exponential,
        FadeCurve::Logarithmic,
        FadeCurve::SCurve,
        FadeCurve::Cosine,
    ];

    for curve in curves {
        // Test fade-in
        let start_multiplier = calculate_fade_in_multiplier(0, fade_duration_samples, curve);
        let mid_multiplier = calculate_fade_in_multiplier(fade_duration_samples / 2, fade_duration_samples, curve);
        let end_multiplier = calculate_fade_in_multiplier(fade_duration_samples - 1, fade_duration_samples, curve);

        assert!(start_multiplier < 0.05, "{:?} fade-in should start near 0", curve);
        assert!(end_multiplier > 0.95, "{:?} fade-in should end near 1.0", curve);
        assert!(mid_multiplier > 0.1 && mid_multiplier < 0.9,
            "{:?} fade-in should have intermediate value at midpoint", curve);
    }
}
```

##### Test: sample_accurate_fade_timing
**Requirement:** [DBD-FADE-030]
**Priority:** HIGH
**Severity:** HIGH

**Given:** Passage with precise fade timing points in ticks
**When:** Converting to sample positions and applying fades
**Then:** Fade starts at exact sample boundary
**And:** Fade ends at exact sample boundary
**And:** No off-by-one errors

```rust
#[test]
fn test_sample_accurate_fade_timing() {
    // 8 seconds at 44.1kHz = 352,800 samples
    let fade_duration_ms = 8000;
    let fade_duration_ticks = ms_to_ticks(fade_duration_ms);
    let fade_duration_samples = ticks_to_samples(fade_duration_ticks, 44100);

    assert_eq!(fade_duration_samples, 352_800,
        "8 seconds @ 44.1kHz should be exactly 352,800 samples");

    // Verify tick → sample → tick roundtrip is exact
    let roundtrip_ticks = samples_to_ticks(fade_duration_samples, 44100);
    assert_eq!(roundtrip_ticks, fade_duration_ticks,
        "Roundtrip conversion must preserve sample accuracy");
}
```

---

### Module: wkmp-ap/src/audio/types.rs (BUFFER MANAGEMENT)

#### Test Group 9: Buffer Size and Limits

##### Test: playout_ringbuffer_size_enforced
**Requirement:** [DBD-PARAM-070]
**Priority:** HIGH
**Severity:** HIGH

**Given:** PassageBuffer with playout_ringbuffer_size = 661,941 samples
**When:** Appending samples to buffer
**Then:** Buffer cannot exceed size limit
**And:** Append operations return buffer full status when full

```rust
#[test]
fn test_playout_ringbuffer_size_enforced() {
    const PLAYOUT_RINGBUFFER_SIZE: usize = 661_941;  // 15.01s @ 44.1kHz

    let mut buffer = PassageBuffer::new_with_capacity(PLAYOUT_RINGBUFFER_SIZE);

    // Fill buffer to capacity
    let samples_to_add = PLAYOUT_RINGBUFFER_SIZE;
    let audio_data = vec![AudioFrame::zero(); samples_to_add];

    let result = buffer.append_samples(&audio_data);
    assert!(result.is_ok(), "Appending within capacity should succeed");
    assert_eq!(buffer.sample_count(), PLAYOUT_RINGBUFFER_SIZE);

    // Attempt to add more samples (should fail or return buffer_full)
    let overflow_data = vec![AudioFrame::zero(); 1000];
    let overflow_result = buffer.append_samples(&overflow_data);

    assert!(overflow_result.is_err() || buffer.is_full(),
        "Buffer should reject samples exceeding capacity");
    assert_eq!(buffer.sample_count(), PLAYOUT_RINGBUFFER_SIZE,
        "Buffer size should not exceed limit");
}
```

##### Test: buffer_full_detection
**Requirement:** [DBD-BUF-050]
**Priority:** HIGH
**Severity:** MEDIUM

**Given:** PassageBuffer approaching capacity
**When:** Free space ≤ playout_ringbuffer_headroom (441 samples)
**Then:** is_nearly_full() returns true
**And:** Decoder should pause on buffer full

```rust
#[test]
fn test_buffer_full_detection() {
    const PLAYOUT_RINGBUFFER_SIZE: usize = 661_941;
    const PLAYOUT_RINGBUFFER_HEADROOM: usize = 441;

    let mut buffer = PassageBuffer::new_with_capacity(PLAYOUT_RINGBUFFER_SIZE);

    // Fill to just before headroom threshold
    let fill_count = PLAYOUT_RINGBUFFER_SIZE - PLAYOUT_RINGBUFFER_HEADROOM - 100;
    buffer.append_samples(&vec![AudioFrame::zero(); fill_count]);
    assert!(!buffer.is_nearly_full(), "Buffer should not be nearly full yet");

    // Add samples to enter headroom zone
    buffer.append_samples(&vec![AudioFrame::zero(); 101]);
    assert!(buffer.is_nearly_full(),
        "Buffer should be nearly full when free space ≤ headroom");

    // Verify free space calculation
    let free_space = buffer.free_space();
    assert!(free_space <= PLAYOUT_RINGBUFFER_HEADROOM,
        "Free space should be ≤ headroom threshold");
}
```

##### Test: backpressure_mechanism
**Requirement:** [DBD-BUF-050]
**Priority:** HIGH
**Severity:** MEDIUM

**Given:** Decoder with buffer approaching full state
**When:** Buffer free space ≤ headroom
**Then:** Decoder pauses decode loop
**And:** Decoder resumes when space available

```rust
#[tokio::test]
async fn test_backpressure_mechanism() {
    let pool = setup_test_decoder_pool().await;
    let buffer_manager = setup_test_buffer_manager();

    // Create passage that will fill buffer
    let passage = create_long_test_passage();
    let buffer = buffer_manager.register_decoding(passage.id).await;

    // Start decode
    pool.decode_passage_async(&passage, &buffer_manager).await;

    // Monitor decode progress
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify buffer fills and decode pauses
    let buffer_read = buffer.read().await;
    assert!(buffer_read.is_nearly_full(),
        "Buffer should fill during decode");

    // Verify decoder is paused (not actively decoding)
    let decode_status = pool.get_decode_status(passage.id).await;
    assert_eq!(decode_status, DecodeStatus::PausedBufferFull,
        "Decoder should pause when buffer full");

    // Consume some samples from buffer
    drop(buffer_read);
    buffer.write().await.consume_samples(10000);

    // Verify decoder resumes
    tokio::time::sleep(Duration::from_millis(100)).await;
    let resumed_status = pool.get_decode_status(passage.id).await;
    assert_eq!(resumed_status, DecodeStatus::Active,
        "Decoder should resume when space available");
}
```

#### Test Group 10: Buffer State Transitions

##### Test: buffer_state_lifecycle
**Requirement:** [DBD-BUF-020 through DBD-BUF-060]
**Priority:** HIGH
**Severity:** MEDIUM

**Given:** New PassageBuffer created
**When:** Progressing through decode → play → exhaust lifecycle
**Then:** States transition: Decoding → Ready → Playing → Exhausted
**And:** No invalid state transitions occur

```rust
#[tokio::test]
async fn test_buffer_state_lifecycle() {
    let buffer_manager = setup_test_buffer_manager();
    let passage_id = Uuid::new_v4();

    // Initial state: Decoding
    let buffer = buffer_manager.register_decoding(passage_id).await;
    assert_eq!(buffer.read().await.status(), BufferStatus::Decoding);

    // Add samples and mark ready
    buffer.write().await.append_samples(&vec![AudioFrame::zero(); 44100]);
    buffer_manager.mark_ready(passage_id).await;
    assert_eq!(buffer.read().await.status(), BufferStatus::Ready);

    // Start playback
    buffer_manager.mark_playing(passage_id).await;
    assert_eq!(buffer.read().await.status(), BufferStatus::Playing);

    // Consume all samples
    buffer.write().await.consume_all();
    buffer_manager.check_exhausted(passage_id).await;
    assert_eq!(buffer.read().await.status(), BufferStatus::Exhausted);
}
```

##### Test: buffer_overflow_prevention
**Requirement:** [DBD-BUF-050]
**Priority:** HIGH
**Severity:** MEDIUM

**Given:** PassageBuffer at capacity
**When:** Attempting to append additional samples
**Then:** Operation fails gracefully
**And:** No memory corruption or panic occurs

```rust
#[test]
fn test_buffer_overflow_prevention() {
    const CAPACITY: usize = 1000;
    let mut buffer = PassageBuffer::new_with_capacity(CAPACITY);

    // Fill to capacity
    buffer.append_samples(&vec![AudioFrame::zero(); CAPACITY]);
    assert_eq!(buffer.sample_count(), CAPACITY);

    // Attempt overflow
    let overflow_result = buffer.append_samples(&vec![AudioFrame::zero(); 100]);

    match overflow_result {
        Ok(_) => panic!("Overflow should not succeed"),
        Err(e) => {
            assert!(e.to_string().contains("buffer full") ||
                    e.to_string().contains("capacity exceeded"),
                "Error should indicate buffer full: {}", e);
        }
    }

    // Verify buffer integrity
    assert_eq!(buffer.sample_count(), CAPACITY,
        "Buffer size should not exceed capacity after failed append");
}
```

##### Test: buffer_underflow_detection
**Requirement:** [DBD-BUF-030, DBD-BUF-040]
**Priority:** HIGH
**Severity:** LOW

**Given:** Empty PassageBuffer
**When:** Mixer attempts to read samples
**Then:** get_frame() returns AudioFrame::zero()
**And:** buffer_empty status flag is set
**And:** Underrun is logged

```rust
#[test]
fn test_buffer_underflow_detection() {
    let buffer = PassageBuffer::new();

    // Attempt to read from empty buffer
    let (frame, status) = buffer.get_frame(0);

    assert_eq!(frame, AudioFrame::zero(),
        "Empty buffer should return silence");
    assert_eq!(status, BufferReadStatus::Underrun,
        "Status should indicate underrun");

    // Attempt to read beyond buffer end
    buffer.append_samples(&vec![AudioFrame::new(0.5, 0.5); 100]);
    let (out_of_bounds_frame, out_of_bounds_status) = buffer.get_frame(200);

    assert_eq!(out_of_bounds_frame, AudioFrame::zero(),
        "Out of bounds read should return silence");
    assert_eq!(out_of_bounds_status, BufferReadStatus::Underrun,
        "Out of bounds should indicate underrun");
}
```

---

## Priority 3: MEDIUM Priority Gaps (21 issues)

### Module: wkmp-ap/src/db/settings.rs (OPERATING PARAMETERS)

#### Test Group 11: Settings Defaults and Validation

##### Test: working_sample_rate_default
**Requirement:** [DBD-PARAM-020]
**Priority:** MEDIUM
**Severity:** MEDIUM

**Given:** Fresh database with no settings
**When:** Loading working_sample_rate
**Then:** Returns default 44,100 Hz
**And:** Value is stored in database settings table

```rust
#[tokio::test]
async fn test_working_sample_rate_default() {
    let pool = setup_test_db().await;

    // Load working_sample_rate (should use default)
    let sample_rate = load_working_sample_rate(&pool)
        .await
        .expect("Failed to load working_sample_rate");

    assert_eq!(sample_rate, 44100,
        "Default working_sample_rate should be 44,100 Hz");

    // Verify persisted to database
    let stored_value: i64 = sqlx::query_scalar(
        "SELECT value_int FROM settings WHERE key = 'working_sample_rate'"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to query stored value");

    assert_eq!(stored_value, 44100);
}
```

##### Test: playout_ringbuffer_size_default
**Requirement:** [DBD-PARAM-070]
**Priority:** MEDIUM
**Severity:** HIGH

**Given:** Fresh database
**When:** Loading playout_ringbuffer_size
**Then:** Returns default 661,941 samples (15.01s @ 44.1kHz)

```rust
#[tokio::test]
async fn test_playout_ringbuffer_size_default() {
    let pool = setup_test_db().await;

    let buffer_size = load_playout_ringbuffer_size(&pool)
        .await
        .expect("Failed to load playout_ringbuffer_size");

    assert_eq!(buffer_size, 661_941,
        "Default playout_ringbuffer_size should be 661,941 samples");

    // Verify equivalent duration at 44.1kHz
    let duration_seconds = buffer_size as f64 / 44100.0;
    assert!((duration_seconds - 15.01).abs() < 0.01,
        "Buffer size should equal ~15.01 seconds @ 44.1kHz");
}
```

##### Test: output_ringbuffer_size_default
**Requirement:** [DBD-PARAM-030]
**Priority:** MEDIUM
**Severity:** MEDIUM

**Given:** Fresh database
**When:** Loading output_ringbuffer_size
**Then:** Returns default 8,192 samples (185ms @ 44.1kHz)

```rust
#[tokio::test]
async fn test_output_ringbuffer_size_default() {
    let pool = setup_test_db().await;

    let output_size = load_output_ringbuffer_size(&pool)
        .await
        .expect("Failed to load output_ringbuffer_size");

    assert_eq!(output_size, 8192,
        "Default output_ringbuffer_size should be 8,192 samples");

    // Verify equivalent duration
    let duration_ms = (output_size as f64 / 44100.0) * 1000.0;
    assert!((duration_ms - 185.0).abs() < 1.0,
        "Output buffer should be ~185ms @ 44.1kHz");
}
```

##### Test: output_refill_period_default
**Requirement:** [DBD-PARAM-040]
**Priority:** MEDIUM
**Severity:** MEDIUM

**Given:** Fresh database
**When:** Loading output_refill_period
**Then:** Returns default 90ms

```rust
#[tokio::test]
async fn test_output_refill_period_default() {
    let pool = setup_test_db().await;

    let refill_period = load_output_refill_period(&pool)
        .await
        .expect("Failed to load output_refill_period");

    assert_eq!(refill_period, 90,
        "Default output_refill_period should be 90ms");
}
```

##### Test: maximum_decode_streams_default
**Requirement:** [DBD-PARAM-050]
**Priority:** MEDIUM
**Severity:** MEDIUM

**Given:** Fresh database
**When:** Loading maximum_decode_streams
**Then:** Returns default 12

```rust
#[tokio::test]
async fn test_maximum_decode_streams_default() {
    let pool = setup_test_db().await;

    let max_streams = load_maximum_decode_streams(&pool)
        .await
        .expect("Failed to load maximum_decode_streams");

    assert_eq!(max_streams, 12,
        "Default maximum_decode_streams should be 12");
}
```

##### Test: decode_work_period_default
**Requirement:** [DBD-PARAM-060]
**Priority:** MEDIUM
**Severity:** LOW

**Given:** Fresh database
**When:** Loading decode_work_period
**Then:** Returns default 5000ms (5 seconds)

```rust
#[tokio::test]
async fn test_decode_work_period_default() {
    let pool = setup_test_db().await;

    let work_period = load_decode_work_period(&pool)
        .await
        .expect("Failed to load decode_work_period");

    assert_eq!(work_period, 5000,
        "Default decode_work_period should be 5000ms");
}
```

##### Test: playout_ringbuffer_headroom_default
**Requirement:** [DBD-PARAM-080]
**Priority:** MEDIUM
**Severity:** MEDIUM

**Given:** Fresh database
**When:** Loading playout_ringbuffer_headroom
**Then:** Returns default 441 samples (0.01s @ 44.1kHz)

```rust
#[tokio::test]
async fn test_playout_ringbuffer_headroom_default() {
    let pool = setup_test_db().await;

    let headroom = load_playout_ringbuffer_headroom(&pool)
        .await
        .expect("Failed to load playout_ringbuffer_headroom");

    assert_eq!(headroom, 441,
        "Default playout_ringbuffer_headroom should be 441 samples");
}
```

---

### Module: wkmp-ap/src/playback/engine.rs (QUEUE MANAGEMENT)

#### Test Group 12: Maximum Decode Streams Logic

##### Test: only_decode_within_max_streams
**Requirement:** [DBD-DEC-020, DBD-OV-050]
**Priority:** MEDIUM
**Severity:** MEDIUM

**Given:** Queue with 15 passages, maximum_decode_streams = 12
**When:** Monitoring which passages are decoding
**Then:** Only first 12 passages have active decoders
**And:** Passages 13-15 wait until they advance into range

```rust
#[tokio::test]
async fn test_only_decode_within_max_streams() {
    let engine = setup_test_engine().await;
    let pool = engine.get_db_pool();

    // Set maximum_decode_streams = 12
    set_maximum_decode_streams(&pool, 12).await;

    // Enqueue 15 passages
    let passage_ids: Vec<Uuid> = (0..15)
        .map(|_| create_and_enqueue_test_passage(&pool))
        .collect();

    // Start playback engine
    engine.start().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check which passages are decoding
    let buffer_manager = engine.get_buffer_manager();

    for (idx, passage_id) in passage_ids.iter().enumerate() {
        let is_managed = buffer_manager.is_managed(*passage_id).await;

        if idx < 12 {
            assert!(is_managed,
                "Passage {} (within max_streams) should be decoding", idx);
        } else {
            assert!(!is_managed,
                "Passage {} (beyond max_streams) should NOT be decoding", idx);
        }
    }
}
```

##### Test: passages_advance_into_decode_range
**Requirement:** [DBD-DEC-020]
**Priority:** MEDIUM
**Severity:** MEDIUM

**Given:** 15 passages in queue, maximum_decode_streams = 12
**When:** Current passage finishes and is removed
**Then:** Passage at position 13 starts decoding
**And:** Decode window slides forward

```rust
#[tokio::test]
async fn test_passages_advance_into_decode_range() {
    let engine = setup_test_engine().await;
    let pool = engine.get_db_pool();

    set_maximum_decode_streams(&pool, 12).await;

    // Enqueue 15 passages
    let passage_ids: Vec<Uuid> = (0..15)
        .map(|_| create_and_enqueue_test_passage(&pool))
        .collect();

    let passage_13_id = passage_ids[12];  // 0-indexed, so position 13

    // Start engine
    engine.start().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify passage 13 is NOT decoding yet
    let buffer_manager = engine.get_buffer_manager();
    assert!(!buffer_manager.is_managed(passage_13_id).await,
        "Passage 13 should not be decoding initially");

    // Skip to next passage (removes current from queue)
    engine.skip().await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Now passage 13 should start decoding (moved into position 12)
    assert!(buffer_manager.is_managed(passage_13_id).await,
        "Passage 13 should start decoding after queue advance");
}
```

---

### Module: wkmp-ap/src/playback/pipeline/mixer.rs (PAUSE DECAY)

#### Test Group 13: Pause Mode Exponential Decay

##### Test: pause_mode_exponential_decay
**Requirement:** [DBD-MIX-050, DBD-PARAM-090]
**Priority:** MEDIUM
**Severity:** LOW

**Given:** Mixer in playing mode at full amplitude
**When:** Entering pause mode
**Then:** Output decays exponentially using pause_decay_factor
**And:** Each sample is previous × 0.96875

```rust
#[tokio::test]
async fn test_pause_mode_exponential_decay() {
    let mixer = setup_test_mixer().await;
    let pool = setup_test_db().await;

    // Set pause decay parameters
    set_pause_decay_factor(&pool, 0.96875).await;
    set_pause_decay_floor(&pool, 0.0001778).await;

    // Start playback with known amplitude
    mixer.start_passage(create_test_passage_buffer(0.5)).await;  // 0.5 amplitude

    // Pause
    mixer.pause().await;

    // Read output frames during pause
    let frames = mixer.get_next_frames(100).await;

    // Verify exponential decay
    let mut expected_amplitude = 0.5;
    for (idx, frame) in frames.iter().enumerate() {
        let actual_amplitude = frame.left.abs();

        assert!((actual_amplitude - expected_amplitude).abs() < 0.01,
            "Frame {} should have amplitude ~{}, got {}",
            idx, expected_amplitude, actual_amplitude);

        expected_amplitude *= 0.96875;
    }
}
```

##### Test: pause_decay_floor_cutoff
**Requirement:** [DBD-PARAM-100]
**Priority:** MEDIUM
**Severity:** LOW

**Given:** Mixer in pause mode, amplitude decaying
**When:** Amplitude drops below pause_decay_floor (0.0001778)
**Then:** Output switches to exact 0.0
**And:** No further multiplications occur

```rust
#[tokio::test]
async fn test_pause_decay_floor_cutoff() {
    let mixer = setup_test_mixer().await;
    let pool = setup_test_db().await;

    set_pause_decay_factor(&pool, 0.96875).await;
    set_pause_decay_floor(&pool, 0.0001778).await;

    // Start with very small amplitude
    mixer.start_passage(create_test_passage_buffer(0.001)).await;
    mixer.pause().await;

    // Read enough frames for decay to hit floor
    let frames = mixer.get_next_frames(200).await;

    // Find first exact zero frame
    let first_zero_idx = frames.iter()
        .position(|f| f.left == 0.0 && f.right == 0.0)
        .expect("Should reach zero within 200 frames");

    // Verify all subsequent frames are zero (no further computation)
    for idx in first_zero_idx..frames.len() {
        assert_eq!(frames[idx], AudioFrame::zero(),
            "Frame {} should be exact zero after hitting floor", idx);
    }
}
```

---

## Priority 4: LOW Priority Gaps (8 issues)

### Module: common/src/timing.rs (HELPER FUNCTIONS)

#### Test Group 14: Convenience Functions

##### Test: ticks_to_seconds_conversion
**Requirement:** [SRC-TIME-010, SRC-TIME-020]
**Priority:** LOW
**Severity:** LOW

**Given:** Tick values
**When:** Converting to seconds (f64)
**Then:** Result equals ticks ÷ 28,224,000

```rust
#[test]
fn test_ticks_to_seconds_conversion() {
    assert_eq!(ticks_to_seconds(0), 0.0);
    assert_eq!(ticks_to_seconds(28_224_000), 1.0);
    assert_eq!(ticks_to_seconds(141_120_000), 5.0);

    // Verify precision
    let three_point_five_seconds = ticks_to_seconds(28_224_000 + 14_112_000);
    assert!((three_point_five_seconds - 3.5).abs() < 0.0001,
        "3.5 seconds should convert accurately");
}
```

##### Test: ticks_per_sample_lookup_table
**Requirement:** [SRC-CONV-010, SRC-CONV-020]
**Priority:** LOW
**Severity:** LOW

**Given:** TICKS_PER_SAMPLE_TABLE constant
**When:** Looking up ticks per sample for each rate
**Then:** Values match TICK_RATE ÷ sample_rate

```rust
#[test]
fn test_ticks_per_sample_lookup_table() {
    const EXPECTED: [(u32, i64); 11] = [
        (8000, 3528),
        (11025, 2560),
        (16000, 1764),
        (22050, 1280),
        (32000, 882),
        (44100, 640),
        (48000, 588),
        (88200, 320),
        (96000, 294),
        (176400, 160),
        (192000, 147),
    ];

    for (rate, expected_ticks) in EXPECTED {
        let ticks = ticks_per_sample(rate);
        assert_eq!(ticks, expected_ticks,
            "Ticks per sample @ {}Hz should be {}", rate, expected_ticks);

        // Verify lookup table matches
        let table_value = TICKS_PER_SAMPLE_TABLE.iter()
            .find(|(r, _)| *r == rate)
            .map(|(_, t)| *t)
            .expect(&format!("Rate {} not in lookup table", rate));

        assert_eq!(table_value, expected_ticks,
            "Lookup table entry for {}Hz should match", rate);
    }
}
```

---

## Module-Level Test Organization

### New Module: common/src/timing.rs

**Total Tests:** 24
**Test File:** `wkmp-common/src/timing_tests.rs`

**Test Groups:**
1. Tick Rate Constants (2 tests)
2. Millisecond ↔ Tick Conversions (4 tests)
3. Tick ↔ Sample Conversions (6 tests)
4. Edge Cases and Overflow (3 tests)
5. Cross-Rate Conversion Examples (2 tests)
6. Helper Functions (2 tests)

### Modified Module: wkmp-ap/src/db/passages.rs

**Total Tests:** 2 (add to existing 5)
**Test File:** `wkmp-ap/src/db/passages.rs` (inline #[cfg(test)])

**Test Groups:**
6. Database Timing Field Types (2 tests)

### Modified Module: wkmp-ap/src/playback/decoder_pool.rs

**Total Tests:** 11 (add to existing 3)
**Test File:** `wkmp-ap/tests/decoder_pool_tests.rs`

**Test Groups:**
7. Serial Decode Execution (3 tests)
8. Pre-Buffer Fade Application (4 tests)

### Modified Module: wkmp-ap/src/audio/types.rs

**Total Tests:** 9 (add to existing 4)
**Test File:** `wkmp-ap/src/audio/types.rs` (inline #[cfg(test)])

**Test Groups:**
9. Buffer Size and Limits (3 tests)
10. Buffer State Transitions (3 tests)

### Modified Module: wkmp-ap/src/db/settings.rs

**Total Tests:** 7 (add to existing 5)
**Test File:** `wkmp-ap/src/db/settings.rs` (inline #[cfg(test)])

**Test Groups:**
11. Settings Defaults and Validation (7 tests)

### Modified Module: wkmp-ap/src/playback/engine.rs

**Total Tests:** 2 (add to existing integration tests)
**Test File:** `wkmp-ap/tests/playback_engine_integration.rs`

**Test Groups:**
12. Maximum Decode Streams Logic (2 tests)

### Modified Module: wkmp-ap/src/playback/pipeline/mixer.rs

**Total Tests:** 2 (add to existing 4)
**Test File:** `wkmp-ap/src/playback/pipeline/mixer.rs` (inline #[cfg(test)])

**Test Groups:**
13. Pause Mode Exponential Decay (2 tests)

---

## Test Skeletons Summary

**Total Unit Tests Designed:** 87

**Breakdown by Priority:**
- CRITICAL: 22 tests (Timing system, database schema)
- HIGH: 18 tests (Serial execution, pre-buffer fades, buffer management)
- MEDIUM: 21 tests (Settings defaults, queue management, pause decay)
- LOW: 2 tests (Helper functions)

**Breakdown by Module:**
- `common/src/timing.rs` (NEW): 24 tests
- `wkmp-ap/src/db/passages.rs`: 2 tests
- `wkmp-ap/src/playback/decoder_pool.rs`: 11 tests
- `wkmp-ap/src/audio/types.rs`: 9 tests
- `wkmp-ap/src/db/settings.rs`: 7 tests
- `wkmp-ap/src/playback/engine.rs`: 2 tests
- `wkmp-ap/src/playback/pipeline/mixer.rs`: 2 tests

**Test Files to Create/Modify:**
1. `wkmp-common/src/timing_tests.rs` (NEW - 24 tests)
2. `wkmp-ap/tests/decoder_pool_tests.rs` (NEW - 11 tests)
3. `wkmp-ap/src/db/passages.rs` (modify - add 2 tests)
4. `wkmp-ap/src/audio/types.rs` (modify - add 9 tests)
5. `wkmp-ap/src/db/settings.rs` (modify - add 7 tests)
6. `wkmp-ap/tests/playback_engine_integration.rs` (modify - add 2 tests)
7. `wkmp-ap/src/playback/pipeline/mixer.rs` (modify - add 2 tests)

---

## Coverage Improvement Estimate

**Current Coverage:** 65% (estimated)
**Current Test LOC:** 3,325
**Current Total LOC:** ~13,719

**New Tests Added:**
- 87 unit tests × ~15 LOC average = ~1,305 new test LOC
- New module `timing.rs` = ~400 implementation LOC

**Expected Coverage After:**
- Test LOC: 3,325 + 1,305 = 4,630
- Total LOC: 13,719 + 400 = 14,119
- Coverage ratio improvement: +1,305 test LOC / 14,119 total ≈ +9% coverage
- **Final Coverage: ~74-78%** (close to 80% target)

---

## Implementation Notes

1. **Module Creation Required:**
   - `wkmp-common/src/timing.rs` - Core tick timing system
   - Must be added to `wkmp-common/src/lib.rs` exports

2. **Database Migration Required:**
   - Convert all 6 timing fields from REAL/INTEGER ms → INTEGER ticks
   - Migration must preserve existing data by converting: ticks = ms × 28,224

3. **Test Helpers Needed:**
   - `setup_test_db()` - Creates in-memory SQLite database
   - `setup_test_decoder_pool()` - Mock decoder pool for testing
   - `setup_test_buffer_manager()` - Mock buffer manager
   - `create_test_passage_buffer(amplitude)` - Generates test audio data

4. **Test Execution Strategy:**
   - Unit tests: `cargo test --lib`
   - Integration tests: `cargo test --test`
   - Critical tests: `cargo test --test timing_tests`
   - All tests: `cargo test`

5. **CI/CD Integration:**
   - Add `cargo test` to CI pipeline
   - Require 80% coverage gate
   - Run tests on every PR

---

## Traceability Matrix

| Requirement ID | Test Name | Module | Priority |
|---------------|-----------|--------|----------|
| SRC-TICK-020 | test_tick_rate_constant_value | timing.rs | CRITICAL |
| SRC-TICK-040 | test_tick_rate_divides_all_sample_rates | timing.rs | CRITICAL |
| SRC-API-020 | test_ms_to_ticks_accuracy | timing.rs | CRITICAL |
| SRC-API-030 | test_ticks_to_ms_roundtrip | timing.rs | CRITICAL |
| SRC-WSR-030 | test_ticks_to_samples_accuracy_44100 | timing.rs | CRITICAL |
| SRC-WSR-030 | test_ticks_to_samples_accuracy_48000 | timing.rs | CRITICAL |
| SRC-WSR-030 | test_ticks_to_samples_all_supported_rates | timing.rs | CRITICAL |
| SRC-CONV-030 | test_samples_to_ticks_accuracy | timing.rs | CRITICAL |
| SRC-CONV-030 | test_samples_to_ticks_roundtrip | timing.rs | CRITICAL |
| SRC-PREC-020 | test_tick_overflow_detection | timing.rs | CRITICAL |
| SRC-DB-011-016 | test_passage_timing_fields_use_i64_ticks | passages.rs | CRITICAL |
| SRC-DB-010 | test_database_stores_ticks_as_integer | passages.rs | CRITICAL |
| DBD-DEC-040 | test_only_one_decoder_active_at_time | decoder_pool.rs | HIGH |
| DBD-DEC-040 | test_priority_queue_ordering | decoder_pool.rs | HIGH |
| DBD-DEC-040 | test_decode_completion_triggers_next | decoder_pool.rs | HIGH |
| DBD-FADE-030 | test_fade_in_applied_before_buffering | decoder_pool.rs | HIGH |
| DBD-FADE-050 | test_fade_out_applied_before_buffering | decoder_pool.rs | HIGH |
| DBD-FADE-030 | test_all_five_fade_curves_supported | decoder_pool.rs | HIGH |
| DBD-FADE-030 | test_sample_accurate_fade_timing | decoder_pool.rs | HIGH |
| DBD-PARAM-070 | test_playout_ringbuffer_size_enforced | types.rs | HIGH |
| DBD-BUF-050 | test_buffer_full_detection | types.rs | HIGH |
| DBD-BUF-050 | test_backpressure_mechanism | types.rs | HIGH |
| DBD-BUF-020-060 | test_buffer_state_lifecycle | types.rs | HIGH |
| DBD-BUF-050 | test_buffer_overflow_prevention | types.rs | HIGH |
| DBD-BUF-030/040 | test_buffer_underflow_detection | types.rs | HIGH |
| DBD-PARAM-020 | test_working_sample_rate_default | settings.rs | MEDIUM |
| DBD-PARAM-070 | test_playout_ringbuffer_size_default | settings.rs | MEDIUM |
| DBD-PARAM-030 | test_output_ringbuffer_size_default | settings.rs | MEDIUM |
| DBD-PARAM-040 | test_output_refill_period_default | settings.rs | MEDIUM |
| DBD-PARAM-050 | test_maximum_decode_streams_default | settings.rs | MEDIUM |
| DBD-PARAM-060 | test_decode_work_period_default | settings.rs | MEDIUM |
| DBD-PARAM-080 | test_playout_ringbuffer_headroom_default | settings.rs | MEDIUM |
| DBD-DEC-020 | test_only_decode_within_max_streams | engine.rs | MEDIUM |
| DBD-DEC-020 | test_passages_advance_into_decode_range | engine.rs | MEDIUM |
| DBD-MIX-050 | test_pause_mode_exponential_decay | mixer.rs | MEDIUM |
| DBD-PARAM-100 | test_pause_decay_floor_cutoff | mixer.rs | MEDIUM |

---

## Next Steps

1. **Create timing module:** Implement `wkmp-common/src/timing.rs` with all conversion functions
2. **Write timing tests:** Create `wkmp-common/src/timing_tests.rs` with 24 unit tests
3. **Database migration:** Convert timing fields from ms → ticks
4. **Refactor decoder pool:** Change from parallel to serial execution
5. **Move fade application:** From mixer (post-buffer) to decoder (pre-buffer)
6. **Add buffer size limits:** Implement playout_ringbuffer_size enforcement
7. **Add settings defaults:** All missing operating parameters
8. **Run test suite:** Verify 80% coverage achieved

---

**Document Status:** Ready for Implementation
**Estimated Implementation Time:** ~30 days (timing system + refactoring + testing)
**Priority Order:** CRITICAL tests first, then HIGH, MEDIUM, LOW

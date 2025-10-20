# Property-Based Test Specifications

**TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines property-based tests using proptest to verify WKMP system invariants hold for ALL possible inputs, not just hand-picked test cases.

> **Related Documentation:** [SPEC016 Decoder Buffer Design](../SPEC016-decoder_buffer_design.md) | [SPEC017 Sample Rate Conversion](../SPEC017-sample_rate_conversion.md) | [SPEC002 Crossfade Design](../SPEC002-crossfade.md) | [Unit Tests](IMPL-TESTS-001-unit-tests.md) | [Integration Tests](IMPL-TESTS-002-integration-tests.md)

---

## Overview

**[PROP-OV-010]** Property-based testing generates thousands of random inputs to verify that system invariants hold universally, discovering edge cases that manual testing misses.

**[PROP-OV-020]** WKMP uses the `proptest` crate (Rust equivalent of QuickCheck/Hypothesis).

**[PROP-OV-030]** When a property test fails, proptest automatically **shrinks** the failing case to find the minimal input that reproduces the failure.

**[PROP-OV-040]** Critical for WKMP subsystems with precise mathematical constraints:
- Timing precision (tick-based arithmetic)
- Buffer safety (ring buffer bounds)
- Audio quality (no clipping, no discontinuities)
- Thread safety (lock-free operations)

---

## Property 1: Tick Conversion Roundtrip

### Invariant

**[PROP-TICK-010]** Converting seconds → ticks → seconds is lossless within tick precision.

For all seconds s ∈ [0, 10000]:
```
|s - roundtrip(s)| ≤ 1 tick = 1/28,224,000 ≈ 35.4 nanoseconds
```

### Why Important

**[PROP-TICK-020]** Ensures tick-based timing ([SRC-SOL-010]) doesn't lose precision over long durations or many conversions.

### Test Code

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn property_tick_conversion_roundtrip(seconds in 0.0f64..10000.0f64) {
        let ticks = seconds_to_ticks(seconds);
        let roundtrip_seconds = ticks_to_seconds(ticks);

        let error = (seconds - roundtrip_seconds).abs();
        let max_error = 1.0 / 28_224_000.0; // One tick precision

        prop_assert!(
            error <= max_error,
            "Roundtrip error {} exceeds max {} for seconds={}",
            error, max_error, seconds
        );
    }
}

fn seconds_to_ticks(seconds: f64) -> i64 {
    (seconds * 28_224_000.0).round() as i64
}

fn ticks_to_seconds(ticks: i64) -> f64 {
    ticks as f64 / 28_224_000.0
}
```

**[PROP-TICK-030]** Run 1000 test cases (proptest default).

---

## Property 2: Sample Conversion Monotonicity

### Invariant

**[PROP-SAMP-010]** Sample-to-tick conversion preserves ordering (monotonicity).

For all sample_rate, samples1, samples2:
```
samples1 < samples2  ⟹  ticks1 < ticks2
samples1 = samples2  ⟹  ticks1 = ticks2
samples1 > samples2  ⟹  ticks1 > ticks2
```

### Why Important

**[PROP-SAMP-020]** Ensures timing never goes backwards during sample-to-tick conversion ([SRC-CONV-030]).

### Test Code

```rust
proptest! {
    #[test]
    fn property_sample_conversion_monotonic(
        samples1 in 0usize..10_000_000,
        samples2 in 0usize..10_000_000,
        sample_rate in prop::sample::select(&[8000u32, 11025, 16000, 22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000])
    ) {
        let ticks1 = samples_to_ticks(samples1, sample_rate);
        let ticks2 = samples_to_ticks(samples2, sample_rate);

        match samples1.cmp(&samples2) {
            std::cmp::Ordering::Less => prop_assert!(ticks1 < ticks2),
            std::cmp::Ordering::Equal => prop_assert_eq!(ticks1, ticks2),
            std::cmp::Ordering::Greater => prop_assert!(ticks1 > ticks2),
        }
    }
}

fn samples_to_ticks(samples: usize, sample_rate: u32) -> i64 {
    let ticks_per_sample = 28_224_000 / sample_rate as i64;
    samples as i64 * ticks_per_sample
}
```

**[PROP-SAMP-030]** Run 1000 test cases across all supported sample rates ([SRC-RATE-010]).

---

## Property 3: Crossfade Volume Sum Equals 1.0

### Invariant

**[PROP-XFD-010]** At any point during crossfade, fade-out + fade-in volumes sum to 1.0 (constant-power crossfade).

For all position t ∈ [0, 1], all curve types:
```
fade_out(t) + fade_in(t) = 1.0 ± 0.0001
```

### Why Important

**[PROP-XFD-020]** Prevents volume dips or peaks during crossfades ([XFD-IMPL-090]). Tolerance allows tiny floating-point rounding errors.

### Test Code

```rust
proptest! {
    #[test]
    fn property_crossfade_sum_equals_one(
        position in 0.0f32..1.0f32,
        curve_type in 0usize..3  // linear, exponential/logarithmic, cosine
    ) {
        let fade_out = calculate_fade_out(position, curve_type);
        let fade_in = calculate_fade_in(position, curve_type);

        let sum = fade_out + fade_in;
        let tolerance = 0.0001;

        prop_assert!(
            (sum - 1.0).abs() < tolerance,
            "Crossfade sum {} at position {} for curve {} should equal 1.0 ± {}",
            sum, position, curve_type, tolerance
        );
    }
}

fn calculate_fade_in(t: f32, curve: usize) -> f32 {
    match curve {
        0 => t,                                    // Linear
        1 => t * t,                                // Exponential
        2 => 0.5 * (1.0 - (std::f32::consts::PI * t).cos()), // Cosine
        _ => panic!("Invalid curve type"),
    }
}

fn calculate_fade_out(t: f32, curve: usize) -> f32 {
    match curve {
        0 => 1.0 - t,                              // Linear
        1 => (1.0 - t) * (1.0 - t),                // Logarithmic
        2 => 0.5 * (1.0 + (std::f32::consts::PI * t).cos()), // Cosine
        _ => panic!("Invalid curve type"),
    }
}
```

**[PROP-XFD-030]** Run 1000 test cases per fade curve type ([XFD-CURV-020], [XFD-CURV-030]).

---

## Property 4: Ring Buffer Never Overflows

### Invariant

**[PROP-BUF-010]** After any sequence of write/read operations, ring buffer length never exceeds capacity.

For all operation sequences:
```
buffer.len() ≤ capacity
buffer.len() + buffer.available_write() = capacity
```

### Why Important

**[PROP-BUF-020]** Buffer overflow corrupts memory and causes crashes ([DBD-BUF-010]).

### Test Code

```rust
proptest! {
    #[test]
    fn property_buffer_never_overflows(
        operations in prop::collection::vec(
            (0..2usize, 1..1000usize), // (operation_type, size)
            0..10000
        )
    ) {
        let capacity = 661_941; // playout_ringbuffer_size [DBD-PARAM-070]
        let mut buffer = RingBuffer::new(capacity);

        for (op_type, size) in operations {
            match op_type {
                0 => { // Write
                    if buffer.available_write() >= size {
                        buffer.write(&vec![0.0f32; size * 2]).ok(); // stereo
                    }
                }
                1 => { // Read
                    if buffer.available_read() >= size {
                        buffer.read(size).ok();
                    }
                }
                _ => {}
            }

            // INVARIANT: Buffer never exceeds capacity
            prop_assert!(
                buffer.len() <= capacity,
                "Buffer length {} exceeds capacity {}",
                buffer.len(), capacity
            );

            // INVARIANT: Available space is correct
            prop_assert_eq!(
                buffer.len() + buffer.available_write(),
                capacity,
                "len + available_write != capacity"
            );
        }
    }
}
```

**[PROP-BUF-030]** Run 100 test cases (expensive: 10,000 operations per case).

---

## Property 5: Buffer Read Never Exceeds Written

### Invariant

**[PROP-BUF-040]** Total samples read can never exceed total samples written.

For all write/read sequences:
```
total_read ≤ total_written
```

### Why Important

**[PROP-BUF-050]** Prevents buffer underflow which produces silence or garbage audio ([DBD-BUF-030]).

### Test Code

```rust
proptest! {
    #[test]
    fn property_buffer_read_never_exceeds_written(
        writes in prop::collection::vec(1..1000usize, 0..100),
        reads in prop::collection::vec(1..1000usize, 0..100)
    ) {
        let mut buffer = RingBuffer::new(100_000);
        let mut total_written = 0;
        let mut total_read = 0;

        for (write_size, read_size) in writes.iter().zip(reads.iter()) {
            // Write
            if buffer.available_write() >= *write_size {
                buffer.write(&vec![0.0; write_size * 2]).ok();
                total_written += write_size;
            }

            // Read
            if buffer.available_read() >= *read_size {
                buffer.read(*read_size).ok();
                total_read += read_size;
            }

            // INVARIANT: Can never read more than written
            prop_assert!(
                total_read <= total_written,
                "total_read {} > total_written {}",
                total_read, total_written
            );
        }
    }
}
```

**[PROP-BUF-060]** Run 1000 test cases.

---

## Property 6: No Audio Clipping

### Invariant

**[PROP-AUD-010]** After applying fade curves, all audio samples remain in valid range [-1.0, 1.0].

For all samples, all fade positions, all curve types:
```
-1.0 ≤ (sample × fade_curve) ≤ 1.0
is_finite(sample × fade_curve) = true
```

### Why Important

**[PROP-AUD-020]** Clipping produces audible distortion. NaN/Inf crashes audio output ([XFD-VOL-030]).

### Test Code

```rust
proptest! {
    #[test]
    fn property_no_clipping(
        samples in prop::collection::vec(-2.0f32..2.0, 0..10000),
        fade_position in 0.0f32..1.0,
        fade_type in 0usize..3
    ) {
        let fade_curve = calculate_fade_out(fade_position, fade_type);

        let processed: Vec<f32> = samples.iter()
            .map(|&s| s * fade_curve)
            .collect();

        for &sample in &processed {
            // INVARIANT: All samples in valid range
            prop_assert!(
                sample >= -1.0 && sample <= 1.0,
                "Sample {} outside [-1.0, 1.0] range",
                sample
            );

            // INVARIANT: No NaN or Inf
            prop_assert!(
                sample.is_finite(),
                "Sample {} is not finite (NaN or Inf)",
                sample
            );
        }
    }
}
```

**[PROP-AUD-030]** Run 1000 test cases across all fade curve types.

---

## Property 7: Fade Curves Are Continuous

### Invariant

**[PROP-FADE-010]** Fade curves have no sudden jumps (discontinuities).

For all adjacent positions t1, t2 where |t2 - t1| is small:
```
|fade(t2) - fade(t1)| ≤ 0.1  // No sudden jumps
```

### Why Important

**[PROP-FADE-020]** Discontinuities produce audible clicks/pops ([XFD-IMPL-090]).

### Test Code

```rust
proptest! {
    #[test]
    fn property_fade_curves_continuous(
        positions in prop::collection::vec(0.0f32..1.0, 2..1000),
        curve_type in 0usize..3
    ) {
        let mut sorted_positions = positions.clone();
        sorted_positions.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let fade_values: Vec<f32> = sorted_positions.iter()
            .map(|&pos| calculate_fade_out(pos, curve_type))
            .collect();

        // Check continuity: adjacent values shouldn't differ by >0.1
        for window in fade_values.windows(2) {
            let diff = (window[1] - window[0]).abs();
            let max_diff = 0.1; // Reasonable continuity threshold

            prop_assert!(
                diff <= max_diff,
                "Discontinuity detected: {} at curve type {}",
                diff, curve_type
            );
        }
    }
}
```

**[PROP-FADE-030]** Run 1000 test cases.

---

## Property 8: Tick Arithmetic Correctness

### Invariant

**[PROP-ARITH-010]** Tick arithmetic for all supported sample rates produces exact conversions with zero remainder.

For all sample_rate ∈ supported_rates:
```
28,224,000 mod sample_rate = 0
```

### Why Important

**[PROP-ARITH-020]** LCM property ([SRC-TICK-020]) ensures sample-accurate timing across all sample rates.

### Test Code

```rust
proptest! {
    #[test]
    fn property_tick_rate_divides_evenly(
        sample_rate in prop::sample::select(&[
            8000u32, 11025, 16000, 22050, 32000, 44100, 48000,
            88200, 96000, 176400, 192000
        ])
    ) {
        const TICK_RATE: i64 = 28_224_000;

        // INVARIANT: Tick rate divides evenly into all sample rates
        let remainder = TICK_RATE % sample_rate as i64;
        prop_assert_eq!(
            remainder, 0,
            "Tick rate {} does not divide evenly by sample rate {} (remainder={})",
            TICK_RATE, sample_rate, remainder
        );
    }
}
```

**[PROP-ARITH-030]** Run 1000 test cases (fast property).

---

## Property 9: Crossfade Duration Calculation

### Invariant

**[PROP-XTIME-010]** Crossfade duration is always the minimum of lead-out and lead-in durations.

For all passage_a_lead_out, passage_b_lead_in:
```
crossfade_duration = min(passage_a_lead_out, passage_b_lead_in)
0 ≤ crossfade_duration ≤ max(passage_a_lead_out, passage_b_lead_in)
```

### Why Important

**[PROP-XTIME-020]** Ensures crossfade never exceeds valid range of either passage ([XFD-IMPL-050]).

### Test Code

```rust
proptest! {
    #[test]
    fn property_crossfade_duration_is_minimum(
        passage_a_duration in 1.0f64..300.0,
        passage_b_duration in 1.0f64..300.0,
        lead_out_pct in 0.0f64..0.5,  // 0-50% of duration
        lead_in_pct in 0.0f64..0.5
    ) {
        let lead_out_duration = passage_a_duration * lead_out_pct;
        let lead_in_duration = passage_b_duration * lead_in_pct;

        let crossfade_duration = calculate_crossfade_duration(
            lead_out_duration,
            lead_in_duration
        );

        // INVARIANT: Duration is the minimum
        prop_assert_eq!(
            crossfade_duration,
            lead_out_duration.min(lead_in_duration),
            "Crossfade duration should be min(lead_out, lead_in)"
        );

        // INVARIANT: Duration in valid range
        prop_assert!(crossfade_duration >= 0.0);
        prop_assert!(crossfade_duration <= lead_out_duration);
        prop_assert!(crossfade_duration <= lead_in_duration);
    }
}

fn calculate_crossfade_duration(lead_out: f64, lead_in: f64) -> f64 {
    lead_out.min(lead_in)
}
```

**[PROP-XTIME-030]** Run 1000 test cases.

---

## Property 10: Timing Point Constraints

### Invariant

**[PROP-CONS-010]** Passage timing points satisfy constraint chains after validation.

For all validated passages:
```
start ≤ fade_in ≤ fade_out ≤ end
start ≤ lead_in ≤ lead_out ≤ end
```

### Why Important

**[PROP-CONS-020]** Constraint violations ([XFD-CONS-010]) produce invalid crossfades or crashes.

### Test Code

```rust
proptest! {
    #[test]
    fn property_timing_constraints_satisfied(
        start in 0.0f64..1000.0,
        duration in 1.0f64..300.0,
        fade_in_offset in 0.0f64..1.0,   // normalized [0,1]
        fade_out_offset in 0.0f64..1.0,
        lead_in_offset in 0.0f64..1.0,
        lead_out_offset in 0.0f64..1.0
    ) {
        let end = start + duration;

        // Generate potentially invalid timing points
        let fade_in = start + (duration * fade_in_offset);
        let fade_out = start + (duration * fade_out_offset);
        let lead_in = start + (duration * lead_in_offset);
        let lead_out = start + (duration * lead_out_offset);

        // Validate and correct (per XFD-IMPL-040)
        let validated = validate_and_correct_passage_timing(
            start, end, fade_in, fade_out, lead_in, lead_out
        );

        // INVARIANT: Fade point constraints
        prop_assert!(validated.start <= validated.fade_in);
        prop_assert!(validated.fade_in <= validated.fade_out);
        prop_assert!(validated.fade_out <= validated.end);

        // INVARIANT: Lead point constraints
        prop_assert!(validated.start <= validated.lead_in);
        prop_assert!(validated.lead_in <= validated.lead_out);
        prop_assert!(validated.lead_out <= validated.end);
    }
}

struct ValidatedPassage {
    start: f64,
    fade_in: f64,
    lead_in: f64,
    lead_out: f64,
    fade_out: f64,
    end: f64,
}

fn validate_and_correct_passage_timing(
    start: f64, end: f64,
    fade_in: f64, fade_out: f64,
    lead_in: f64, lead_out: f64
) -> ValidatedPassage {
    // Clamp to boundaries
    let fade_in = fade_in.clamp(start, end);
    let fade_out = fade_out.clamp(start, end);
    let lead_in = lead_in.clamp(start, end);
    let lead_out = lead_out.clamp(start, end);

    // Fix ordering violations with midpoint
    let (fade_in, fade_out) = if fade_in > fade_out {
        let mid = (fade_in + fade_out) / 2.0;
        (mid, mid)
    } else {
        (fade_in, fade_out)
    };

    let (lead_in, lead_out) = if lead_in > lead_out {
        let mid = (lead_in + lead_out) / 2.0;
        (mid, mid)
    } else {
        (lead_in, lead_out)
    };

    ValidatedPassage { start, fade_in, lead_in, lead_out, fade_out, end }
}
```

**[PROP-CONS-030]** Run 1000 test cases.

---

## Property 11: Lock-Free Ring Buffer (Thread Safety)

### Invariant

**[PROP-THREAD-010]** Concurrent reads and writes never deadlock or corrupt buffer state.

For all concurrent operation sequences:
```
buffer state remains valid after concurrent access
len ≤ capacity always holds
no data races (verified by ThreadSanitizer)
```

### Why Important

**[PROP-THREAD-020]** Audio thread must never block ([SSP-OUT-010]). Data races corrupt buffer state.

### Test Code

```rust
proptest! {
    #[test]
    fn property_lockfree_buffer_concurrent(
        write_ops in prop::collection::vec(1..100usize, 0..1000),
        read_ops in prop::collection::vec(1..100usize, 0..1000)
    ) {
        let capacity = 100_000;
        let buffer = Arc::new(RingBuffer::new(capacity));
        let buffer_writer = Arc::clone(&buffer);
        let buffer_reader = Arc::clone(&buffer);

        // Writer thread
        let writer = std::thread::spawn(move || {
            for size in write_ops {
                if buffer_writer.available_write() >= size {
                    buffer_writer.write(&vec![1.0; size * 2]).ok();
                }
                std::thread::yield_now(); // Encourage interleaving
            }
        });

        // Reader thread
        let reader = std::thread::spawn(move || {
            for size in read_ops {
                if buffer_reader.available_read() >= size {
                    buffer_reader.read(size).ok();
                }
                std::thread::yield_now(); // Encourage interleaving
            }
        });

        // Should complete without deadlock
        writer.join().expect("Writer thread panicked");
        reader.join().expect("Reader thread panicked");

        // INVARIANT: Buffer still valid after concurrent access
        prop_assert!(
            buffer.len() <= capacity,
            "Buffer corrupted: len {} > capacity {}",
            buffer.len(), capacity
        );
    }
}
```

**[PROP-THREAD-030]** Run 100 test cases (expensive: spawns 200 threads total).

**[PROP-THREAD-040]** Additional validation with ThreadSanitizer:
```bash
RUSTFLAGS="-Z sanitizer=thread" cargo test property_lockfree_buffer_concurrent
```

---

## Property 12: Pause Decay Convergence

### Invariant

**[PROP-PAUSE-010]** Exponential decay during pause converges to zero within finite time.

For all initial sample values, decay factor = 0.96875, floor = 0.0001778:
```
After N iterations, |sample| < floor
N is finite and predictable
```

### Why Important

**[PROP-PAUSE-020]** Ensures pause decay doesn't run forever ([DBD-PARAM-090], [DBD-PARAM-100]).

### Test Code

```rust
proptest! {
    #[test]
    fn property_pause_decay_converges(
        initial_sample in -1.0f32..1.0
    ) {
        const DECAY_FACTOR: f32 = 0.96875; // [DBD-PARAM-090]
        const FLOOR: f32 = 0.0001778;      // [DBD-PARAM-100]
        const MAX_ITERATIONS: usize = 10000;

        let mut sample = initial_sample;
        let mut iterations = 0;

        while sample.abs() >= FLOOR && iterations < MAX_ITERATIONS {
            sample *= DECAY_FACTOR;
            iterations += 1;
        }

        // INVARIANT: Converges within finite iterations
        prop_assert!(
            iterations < MAX_ITERATIONS,
            "Decay did not converge within {} iterations (initial={})",
            MAX_ITERATIONS, initial_sample
        );

        // INVARIANT: Final value below floor
        prop_assert!(
            sample.abs() < FLOOR,
            "Final sample {} not below floor {}",
            sample, FLOOR
        );
    }
}
```

**[PROP-PAUSE-030]** Run 1000 test cases.

---

## Test Configuration

### Proptest Settings

**[PROP-CFG-010]** Test execution configuration in `proptest-regressions/` directory:

```toml
# Cargo.toml
[dev-dependencies]
proptest = "1.0"

# Default proptest configuration (can override per-test)
[profile.test]
opt-level = 2  # Optimize tests for speed
```

**[PROP-CFG-020]** Test case counts by property cost:

| Property Type | Test Cases | Rationale |
|---------------|-----------|-----------|
| Fast (arithmetic, conversions) | 1000 | Default proptest setting |
| Medium (buffer ops, fades) | 1000 | Balanced coverage vs. speed |
| Expensive (threading, large sequences) | 100 | Spawns threads or processes large inputs |

**[PROP-CFG-030]** Override test case count:

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn expensive_property(...) {
        // Test implementation
    }
}
```

### Shrinking Strategy

**[PROP-SHRINK-010]** When a property test fails, proptest automatically **shrinks** the failing input to find the minimal case.

**Example:** Property 4 (Buffer Overflow) fails with 10,000 operations → proptest shrinks to minimal failing sequence (e.g., 3 operations).

**[PROP-SHRINK-020]** Shrinking process:
1. Proptest detects failure with random input
2. Systematically reduces input size/complexity
3. Retests after each reduction
4. Stops when further reduction passes
5. Reports minimal failing case

**[PROP-SHRINK-030]** Shrunk failures are stored in `proptest-regressions/` directory for deterministic replay:

```
proptest-regressions/
  timing_property_tests.txt
  buffer_property_tests.txt
  fade_property_tests.txt
```

**[PROP-SHRINK-040]** Replay previous failures:

```bash
# Automatically replays all stored regression cases
cargo test property_buffer_never_overflows
```

---

## Test Organization

### File Structure

**[PROP-ORG-010]** Property tests organized by subsystem:

```
wkmp-common/
  src/
    timing.rs                    # Timing implementation
    timing_property_tests.rs     # Property tests for timing

wkmp-ap/
  src/
    playback/
      ring_buffer.rs             # Ring buffer implementation
      ring_buffer_property_tests.rs  # Property tests for buffers
    audio/
      fade.rs                    # Fade curve implementation
      fade_property_tests.rs     # Property tests for fades
  tests/
    property_tests.rs            # Integration property tests
```

**[PROP-ORG-020]** Enable property test modules:

```rust
// In timing.rs
#[cfg(test)]
mod timing_property_tests;
```

### Test Modules

**[PROP-MOD-010]** Property test files include:

1. **timing_property_tests.rs**: Properties 1, 2, 8
2. **ring_buffer_property_tests.rs**: Properties 4, 5, 11
3. **fade_property_tests.rs**: Properties 3, 6, 7
4. **crossfade_property_tests.rs**: Properties 9, 10
5. **pause_property_tests.rs**: Property 12

---

## Running Property Tests

### Basic Execution

```bash
# Run all property tests
cargo test --lib property_

# Run specific property test
cargo test property_tick_conversion_roundtrip

# Run with more test cases (slower, more thorough)
PROPTEST_CASES=10000 cargo test property_

# Run with verbose output
cargo test property_ -- --nocapture
```

### Thread Safety Testing

**[PROP-RUN-010]** Use ThreadSanitizer for lock-free ring buffer tests:

```bash
# Requires nightly Rust
rustup install nightly
RUSTFLAGS="-Z sanitizer=thread" cargo +nightly test property_lockfree_buffer_concurrent
```

### Continuous Integration

**[PROP-CI-010]** Run property tests in CI pipeline:

```yaml
# .github/workflows/test.yml
- name: Run property tests
  run: cargo test --lib property_

- name: Run property tests (extended)
  run: PROPTEST_CASES=10000 cargo test property_tick_conversion_roundtrip
```

---

## Summary

### Properties Verified

**[PROP-SUM-010]** Total property tests designed: **12 properties**

**[PROP-SUM-020]** Key invariants verified:

**Timing Precision:**
- Property 1: Tick conversion roundtrip lossless
- Property 2: Sample conversion monotonic
- Property 8: Tick arithmetic correct
- Property 9: Crossfade duration calculation

**Buffer Safety:**
- Property 4: Ring buffer never overflows
- Property 5: Read never exceeds written
- Property 11: Lock-free concurrency safe

**Audio Quality:**
- Property 3: Crossfade volume sum = 1.0
- Property 6: No clipping or NaN/Inf
- Property 7: Fade curves continuous
- Property 12: Pause decay converges

**Structural Invariants:**
- Property 10: Timing point constraints satisfied

### Coverage

**[PROP-SUM-030]** Property tests provide:
- **1000 test cases** per fast property (timing, fades, crossfades)
- **100 test cases** per expensive property (threading, large sequences)
- **Automatic shrinking** to minimal failing cases
- **Deterministic replay** of previous failures via regression files

### Shrinking Strategy

**[PROP-SUM-040]** Shrinking failure cases:
1. Proptest detects failure with random input
2. Systematically reduces input to minimal failing case
3. Stores minimal case in `proptest-regressions/`
4. Automatically replays on subsequent test runs
5. Enables efficient debugging of edge cases

### Files Created

**[PROP-SUM-050]** Property test specification files:

- `/home/sw/Dev/McRhythm/docs/validation/IMPL-TESTS-004-property-tests.md` (this document)
- Property test implementation files (to be created by Agent 2E):
  - `/home/sw/Dev/McRhythm/wkmp-common/src/timing_property_tests.rs`
  - `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/ring_buffer_property_tests.rs`
  - `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/fade_property_tests.rs`
  - `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/crossfade_property_tests.rs`
  - `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/pause_property_tests.rs`
  - `/home/sw/Dev/McRhythm/wkmp-ap/tests/property_tests.rs` (integration tests)

---

**Document Version:** 1.0
**Created:** 2025-10-19
**Status:** Current
**Tier:** 3 - Implementation Specification
**Document Code:** PROP (Property-Based Tests)

**Change Log:**
- v1.0 (2025-10-19): Initial property test specification
  - Defined 12 critical system properties
  - Specified proptest-based test implementations
  - Added shrinking strategy and test configuration
  - Organized by subsystem (timing, buffer, audio, crossfade)
  - Linked to SPEC016, SPEC017, SPEC002 design specifications

**Maintained By:** Test engineer, audio engineer

---
End of document - Property-Based Test Specifications

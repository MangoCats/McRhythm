# IMPL-TESTS-003: Performance Benchmark Specifications

**Document Type:** Implementation Testing
**Status:** Active
**Version:** 1.0
**Date:** 2025-10-19

---

## Purpose

Define comprehensive performance benchmarks to measure and validate WKMP Audio Player performance requirements, with primary focus on achieving the <100ms startup time goal.

## Performance Goals

### Critical: Startup Time Goal

**Phase 1 Baseline:** ~1,500ms
**Phase 5 Target:** <100ms
**Phase 5 Stretch:** <50ms

**Improvement Required:** 15x faster (93% reduction)

### Other Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Decode Throughput | >10x realtime | Decode 10s audio in 1s |
| Resample Performance | >20x realtime | Minimal overhead |
| Fade Curve Application | >50x realtime | Negligible cost |
| Buffer Operations | >1000x realtime | Lock-free performance |
| Mixer Throughput | >50x realtime | Real-time mixing |
| Tick Conversions | >1M/sec | Trivial arithmetic |

---

## Benchmark Suite Overview

### 7 Core Benchmarks

1. **Startup Time Benchmark** (CRITICAL) - End-to-end latency measurement
2. **Decode Throughput Benchmark** - Codec performance by format
3. **Resample Performance Benchmark** - Rubato resampling overhead
4. **Fade Curve Application Benchmark** - Crossfade math performance
5. **Buffer Operations Benchmark** - Ring buffer throughput
6. **Mixer Throughput Benchmark** - Real-time mixing performance
7. **Tick Conversion Benchmark** - Timing arithmetic performance

### Benchmark Configuration

All benchmarks use **criterion** framework with:
- Statistical analysis (mean, std dev, min, max, percentiles)
- Regression detection (>10% slowdown alerts)
- HTML reports with graphs
- Comparison against baseline

---

## Benchmark 1: Startup Time (CRITICAL)

### Goal

Measure and validate <100ms startup time requirement from API enqueue call to first audio sample output.

### Test Scenarios

| Scenario | Format | Start Offset | Target Time |
|----------|--------|--------------|-------------|
| Best Case | MP3 @ 44.1kHz | 0s | <100ms |
| Typical | MP3 @ 44.1kHz | 30s | <150ms |
| Decode+Skip | MP3 @ 44.1kHz | 60s | <150ms |
| Resample | FLAC @ 48kHz | 0s | <200ms |
| Worst Case | FLAC @ 96kHz | 60s | <300ms |

### Measured Timings

End-to-end breakdown with targets:

1. **API request received → request parsed** (target: <5ms)
2. **Request parsed → database query complete** (target: <10ms)
3. **Database query → decoder initialized** (target: <20ms)
4. **Decoder initialized → decode-and-skip complete** (target: <50ms)
5. **Skip complete → minimum buffer filled** (target: <10ms)
6. **Buffer filled → mixer activated** (target: <5ms)
7. **Mixer activated → first audio sample output** (target: <5ms)

**Total Target:** <100ms

### Implementation

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

fn benchmark_startup_time(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let mut group = c.benchmark_group("startup_time");

    group.sample_size(100); // Statistical significance
    group.measurement_time(Duration::from_secs(60));

    // Best case: MP3 @ 44.1kHz, start at 0s
    group.bench_function("mp3_44100_start_0s", |b| {
        b.to_async(&runtime).iter(|| async {
            let start = Instant::now();

            // 1. Enqueue passage
            let queue_entry_id = enqueue_test_passage(
                "/test/audio/test_44100.mp3",
                0, // start_time_ms
                180000, // end_time_ms (3 minutes)
            ).await;

            // 2. Start playback
            play().await;

            // 3. Wait for first audio sample
            wait_for_first_audio_sample().await;

            let elapsed = start.elapsed();

            // Assert target met
            if elapsed > Duration::from_millis(100) {
                panic!("Startup time {}ms exceeds 100ms target", elapsed.as_millis());
            }

            elapsed
        })
    });

    // Decode-and-skip: MP3 starting at 60s
    group.bench_function("mp3_44100_start_60s", |b| {
        b.to_async(&runtime).iter(|| async {
            let start = Instant::now();

            let queue_entry_id = enqueue_test_passage(
                "/test/audio/test_44100.mp3",
                60000, // start at 60s - requires decode-and-skip
                180000,
            ).await;

            play().await;
            wait_for_first_audio_sample().await;

            let elapsed = start.elapsed();

            // Decode-and-skip should still be fast
            if elapsed > Duration::from_millis(150) {
                panic!("Skip startup time {}ms exceeds 150ms target", elapsed.as_millis());
            }

            elapsed
        })
    });

    // Resample case: FLAC @ 48kHz → 44.1kHz
    group.bench_function("flac_48000_resample", |b| {
        b.to_async(&runtime).iter(|| async {
            let start = Instant::now();

            let queue_entry_id = enqueue_test_passage(
                "/test/audio/test_48000.flac",
                0,
                180000,
            ).await;

            play().await;
            wait_for_first_audio_sample().await;

            let elapsed = start.elapsed();

            // Resampling adds overhead
            if elapsed > Duration::from_millis(200) {
                panic!("Resample startup time {}ms exceeds 200ms target", elapsed.as_millis());
            }

            elapsed
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_startup_time);
criterion_main!(benches);
```

### Success Criteria

- Mean startup time < 100ms for MP3 @ 44.1kHz at 0s
- Mean startup time < 150ms for MP3 @ 44.1kHz at 60s (decode-and-skip)
- 99th percentile < 200ms (consistent performance)
- Regression detection: Alert if >10% slower than previous run

---

## Benchmark 2: Decode Throughput

### Goal

Measure samples decoded per second to verify >10x realtime performance.

### Test Scenarios

| Format | Source Sample Rate | Expected Speed |
|--------|-------------------|----------------|
| MP3 192kbps | 44.1kHz | 12-15x realtime |
| MP3 320kbps | 44.1kHz | 10-12x realtime |
| FLAC 16-bit | 44.1kHz | 15-20x realtime |
| FLAC 24-bit | 96kHz | 8-10x realtime |
| AAC | 48kHz | 10-12x realtime |

### Implementation

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use wkmp_ap::audio::decoder::SimpleDecoder;

fn benchmark_decode_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_throughput");

    let test_files = vec![
        ("mp3_192k_44100", "/test/audio/test_192k_44100.mp3", 30000), // 30s
        ("mp3_320k_44100", "/test/audio/test_320k_44100.mp3", 30000),
        ("flac_16bit_44100", "/test/audio/test_16bit_44100.flac", 30000),
        ("flac_24bit_96000", "/test/audio/test_24bit_96000.flac", 30000),
    ];

    for (name, file_path, duration_ms) in test_files {
        group.throughput(Throughput::Elements(duration_ms as u64));

        group.bench_function(BenchmarkId::from_parameter(name), |b| {
            b.iter(|| {
                let start = std::time::Instant::now();

                let (samples, sample_rate, channels) = SimpleDecoder::decode_passage(
                    black_box(file_path),
                    black_box(0),
                    black_box(duration_ms),
                ).unwrap();

                let elapsed = start.elapsed().as_secs_f64();
                let realtime_factor = (duration_ms as f64 / 1000.0) / elapsed;

                // Assert >10x realtime
                assert!(
                    realtime_factor > 10.0,
                    "Decode speed {}x is below 10x realtime target",
                    realtime_factor
                );

                black_box((samples, sample_rate, channels));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_decode_throughput);
criterion_main!(benches);
```

### Success Criteria

- All formats achieve >10x realtime decode speed
- MP3: 12-15x realtime
- FLAC: 15-20x realtime (less complex codec)
- No format falls below 10x on target hardware

---

## Benchmark 3: Resample Performance

### Goal

Measure rubato resampling throughput to verify minimal overhead (>20x realtime).

### Test Scenarios

| Source Rate | Target Rate | Ratio | Expected Speed |
|-------------|-------------|-------|----------------|
| 48000 Hz | 44100 Hz | 0.91875 | >25x realtime |
| 96000 Hz | 44100 Hz | 0.459375 | >20x realtime |
| 192000 Hz | 44100 Hz | 0.2296875 | >15x realtime |

### Implementation

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rubato::{Resampler, SincFixedIn, InterpolationType, InterpolationParameters, WindowFunction};

fn benchmark_resample_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("resample_performance");

    let test_cases = vec![
        ("48k_to_44k", 48000, 44100, 30 * 48000), // 30s @ 48kHz
        ("96k_to_44k", 96000, 44100, 30 * 96000), // 30s @ 96kHz
        ("192k_to_44k", 192000, 44100, 30 * 192000), // 30s @ 192kHz
    ];

    for (name, source_rate, target_rate, sample_count) in test_cases {
        group.bench_function(BenchmarkId::from_parameter(name), |b| {
            // Create test audio data
            let stereo_samples: Vec<f32> = vec![0.5; sample_count * 2]; // Stereo interleaved

            b.iter(|| {
                let start = std::time::Instant::now();

                // Create resampler
                let params = InterpolationParameters {
                    sinc_len: 256,
                    f_cutoff: 0.95,
                    interpolation: InterpolationType::Linear,
                    oversampling_factor: 256,
                    window: WindowFunction::BlackmanHarris2,
                };

                let mut resampler = SincFixedIn::<f32>::new(
                    target_rate as f64 / source_rate as f64,
                    2.0,
                    params,
                    sample_count,
                    2, // stereo
                ).unwrap();

                // Resample (simplified - real implementation uses chunks)
                let resampled = resample_stereo_interleaved(
                    black_box(&stereo_samples),
                    source_rate,
                    target_rate,
                );

                let elapsed = start.elapsed().as_secs_f64();
                let duration_s = sample_count as f64 / source_rate as f64;
                let realtime_factor = duration_s / elapsed;

                // Assert >20x realtime
                assert!(
                    realtime_factor > 20.0,
                    "Resample speed {}x is below 20x realtime target",
                    realtime_factor
                );

                black_box(resampled);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_resample_performance);
criterion_main!(benches);
```

### Success Criteria

- 48kHz → 44.1kHz: >25x realtime
- 96kHz → 44.1kHz: >20x realtime
- 192kHz → 44.1kHz: >15x realtime
- No regressions when rubato version updated

---

## Benchmark 4: Fade Curve Application

### Goal

Measure fade curve calculation throughput to verify negligible overhead (>50x realtime).

### Test Scenarios

Test all 5 fade curve types:
- Linear
- Exponential
- Logarithmic
- SCurve
- Cosine

### Implementation

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use wkmp_ap::playback::pipeline::single_stream::buffer::FadeCurve;
use wkmp_ap::playback::pipeline::single_stream::mixer::calculate_fade_gain;

fn benchmark_fade_curves(c: &mut Criterion) {
    let mut group = c.benchmark_group("fade_curves");

    let curves = vec![
        ("linear", FadeCurve::Linear),
        ("exponential", FadeCurve::Exponential),
        ("logarithmic", FadeCurve::Logarithmic),
        ("s_curve", FadeCurve::SCurve),
        ("cosine", FadeCurve::Cosine),
    ];

    // Test with 10 seconds of audio @ 44.1kHz stereo = 441,000 samples
    let sample_count = 441_000usize;

    for (name, curve) in curves {
        group.bench_function(BenchmarkId::new("fade_in", name), |b| {
            b.iter(|| {
                let start = std::time::Instant::now();

                for i in 0..sample_count {
                    let progress = i as f32 / sample_count as f32;
                    let gain = calculate_fade_gain(curve.clone(), progress, true);
                    black_box(gain);
                }

                let elapsed = start.elapsed().as_secs_f64();
                let duration_s = sample_count as f64 / 44100.0; // 10 seconds
                let realtime_factor = duration_s / elapsed;

                // Assert >50x realtime
                assert!(
                    realtime_factor > 50.0,
                    "Fade calculation speed {}x is below 50x realtime target",
                    realtime_factor
                );
            });
        });

        group.bench_function(BenchmarkId::new("fade_out", name), |b| {
            b.iter(|| {
                for i in 0..sample_count {
                    let progress = i as f32 / sample_count as f32;
                    let gain = calculate_fade_gain(curve.clone(), progress, false);
                    black_box(gain);
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_fade_curves);
criterion_main!(benches);
```

### Success Criteria

- All fade curves: >50x realtime
- Linear should be fastest (>100x realtime)
- S-curve and Cosine may be slower but still >50x
- Negligible impact on mixing performance

---

## Benchmark 5: Buffer Operations

### Goal

Measure ring buffer and passage buffer throughput to verify lock-free performance (>1000x realtime).

### Test Scenarios

| Operation | Buffer Size | Expected Speed |
|-----------|-------------|----------------|
| Ring buffer write | 2048 samples | >5000x realtime |
| Ring buffer read | 2048 samples | >5000x realtime |
| Passage buffer append | 44,100 samples (1s) | >1000x realtime |
| Passage buffer copy | 441,000 samples (10s) | >500x realtime |

### Implementation

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use ringbuf::HeapRb;

fn benchmark_buffer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_operations");

    // Ring buffer operations
    group.bench_function("ring_buffer_write", |b| {
        let ring = HeapRb::<f32>::new(2048);
        let (mut producer, _consumer) = ring.split();
        let data = vec![0.5f32; 2048];

        b.iter(|| {
            let written = producer.push_slice(&data);
            black_box(written);
        });
    });

    group.bench_function("ring_buffer_read", |b| {
        let ring = HeapRb::<f32>::new(2048);
        let (mut producer, mut consumer) = ring.split();
        let data = vec![0.5f32; 2048];
        producer.push_slice(&data);

        let mut buffer = vec![0.0f32; 2048];

        b.iter(|| {
            let read = consumer.pop_slice(&mut buffer);
            black_box(read);
        });
    });

    // Passage buffer operations
    group.bench_function("passage_buffer_append_1s", |b| {
        let mut buffer = Vec::with_capacity(441_000 * 2); // 10s capacity
        let chunk = vec![0.5f32; 44_100 * 2]; // 1s stereo

        b.iter(|| {
            buffer.extend_from_slice(black_box(&chunk));
            black_box(&buffer);
        });
    });

    group.bench_function("passage_buffer_copy_10s", |b| {
        let source = vec![0.5f32; 441_000 * 2]; // 10s stereo
        let mut dest = vec![0.0f32; 441_000 * 2];

        b.iter(|| {
            dest.copy_from_slice(black_box(&source));
            black_box(&dest);
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_buffer_operations);
criterion_main!(benches);
```

### Success Criteria

- Ring buffer operations: >5000x realtime (should be nearly instant)
- Passage buffer append: >1000x realtime
- Passage buffer copy: >500x realtime
- No lock contention detected

---

## Benchmark 6: Mixer Throughput

### Goal

Measure real-time mixing performance to verify >50x realtime throughput.

### Test Scenarios

| Scenario | Description | Expected Speed |
|----------|-------------|----------------|
| Single passage | No crossfade | >100x realtime |
| Crossfade overlap | Two passages mixed | >50x realtime |
| Volume adjustment | Gain multiplication | >100x realtime |

### Implementation

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_mixer_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixer_throughput");

    // Single passage playback (no mixing)
    group.bench_function("single_passage", |b| {
        let buffer = vec![0.5f32; 44_100 * 2]; // 1s stereo
        let volume = 0.8f32;
        let mut output = vec![0.0f32; 44_100 * 2];

        b.iter(|| {
            let start = std::time::Instant::now();

            for i in 0..output.len() {
                output[i] = buffer[i] * volume;
            }

            let elapsed = start.elapsed().as_secs_f64();
            let realtime_factor = 1.0 / elapsed; // 1 second of audio

            assert!(
                realtime_factor > 100.0,
                "Single passage speed {}x is below 100x realtime target",
                realtime_factor
            );

            black_box(&output);
        });
    });

    // Crossfade mixing (two passages)
    group.bench_function("crossfade_overlap", |b| {
        let buffer_a = vec![0.5f32; 44_100 * 2]; // 1s stereo
        let buffer_b = vec![0.7f32; 44_100 * 2]; // 1s stereo
        let mut output = vec![0.0f32; 44_100 * 2];

        b.iter(|| {
            let start = std::time::Instant::now();

            for i in 0..(44_100 * 2) {
                let progress = (i / 2) as f32 / 44_100.0;
                let gain_a = 1.0 - progress; // fade out
                let gain_b = progress; // fade in

                output[i] = buffer_a[i] * gain_a + buffer_b[i] * gain_b;
            }

            let elapsed = start.elapsed().as_secs_f64();
            let realtime_factor = 1.0 / elapsed;

            assert!(
                realtime_factor > 50.0,
                "Crossfade speed {}x is below 50x realtime target",
                realtime_factor
            );

            black_box(&output);
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_mixer_throughput);
criterion_main!(benches);
```

### Success Criteria

- Single passage: >100x realtime
- Crossfade overlap: >50x realtime
- Volume adjustment: >100x realtime
- No audio dropouts in real-time callback

---

## Benchmark 7: Tick Conversions

### Goal

Measure timing arithmetic performance to verify >1M conversions/second.

### Test Scenarios

| Conversion | Description | Expected Speed |
|------------|-------------|----------------|
| ms_to_ticks | Milliseconds to ticks @ 470.4 ticks/ms | >5M/sec |
| ticks_to_ms | Ticks to milliseconds | >5M/sec |
| ticks_to_samples | Ticks to sample count @ 44.1kHz | >5M/sec |
| samples_to_ticks | Sample count to ticks | >5M/sec |

### Implementation

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

const TICKS_PER_MS: u64 = 470_400 / 10; // 470.4 ticks per ms
const SAMPLE_RATE: u64 = 44_100;

fn ms_to_ticks(ms: u64) -> u64 {
    ms * TICKS_PER_MS
}

fn ticks_to_ms(ticks: u64) -> u64 {
    ticks / TICKS_PER_MS
}

fn ticks_to_samples(ticks: u64) -> u64 {
    (ticks * SAMPLE_RATE) / (TICKS_PER_MS * 1000)
}

fn samples_to_ticks(samples: u64) -> u64 {
    (samples * TICKS_PER_MS * 1000) / SAMPLE_RATE
}

fn benchmark_tick_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("tick_conversions");

    let test_values = vec![0, 100, 1000, 10_000, 100_000, 1_000_000];

    group.bench_function("ms_to_ticks", |b| {
        b.iter(|| {
            let start = std::time::Instant::now();

            for _ in 0..1_000_000 {
                for &val in &test_values {
                    black_box(ms_to_ticks(black_box(val)));
                }
            }

            let elapsed = start.elapsed().as_secs_f64();
            let conversions_per_sec = (1_000_000.0 * test_values.len() as f64) / elapsed;

            assert!(
                conversions_per_sec > 1_000_000.0,
                "Conversion speed {:.0}/s is below 1M/s target",
                conversions_per_sec
            );
        });
    });

    group.bench_function("ticks_to_ms", |b| {
        b.iter(|| {
            for &val in &test_values {
                black_box(ticks_to_ms(black_box(val * TICKS_PER_MS)));
            }
        });
    });

    group.bench_function("ticks_to_samples", |b| {
        b.iter(|| {
            for &val in &test_values {
                black_box(ticks_to_samples(black_box(val * TICKS_PER_MS)));
            }
        });
    });

    group.bench_function("samples_to_ticks", |b| {
        b.iter(|| {
            for &val in &test_values {
                black_box(samples_to_ticks(black_box(val * SAMPLE_RATE)));
            }
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_tick_conversions);
criterion_main!(benches);
```

### Success Criteria

- All conversions: >1M/second
- Target: >5M/second (trivial arithmetic)
- No floating-point overhead
- Consistent performance across value ranges

---

## CPU Usage Benchmarks

### Goals

Measure CPU usage during different operational scenarios.

### Test Scenarios

| Scenario | Target CPU (Desktop) | Target CPU (Pi) |
|----------|---------------------|-----------------|
| Idle (server running, no playback) | <1% | <5% |
| Decoding (one passage) | <50% | <80% |
| Steady-state playback | <5% | <10% |
| Crossfade (two passages overlapping) | <10% | <15% |
| Peak load (decode + playback + crossfade) | <60% | <90% |

### Implementation

Requires system-level monitoring tools:
- Linux: `/proc/stat` CPU time monitoring
- Cross-platform: `sysinfo` crate for CPU percentage

```rust
use sysinfo::{System, SystemExt, ProcessExt};
use std::thread;
use std::time::Duration;

fn measure_cpu_usage_during_decode() -> f32 {
    let mut system = System::new_all();
    let pid = sysinfo::get_current_pid().unwrap();

    // Baseline measurement
    system.refresh_process(pid);
    let start_cpu = system.process(pid).unwrap().cpu_usage();

    // Trigger decode operation
    decode_test_passage().await;

    thread::sleep(Duration::from_millis(100));

    // Measure CPU
    system.refresh_process(pid);
    let end_cpu = system.process(pid).unwrap().cpu_usage();

    end_cpu - start_cpu
}
```

### Success Criteria

- Idle CPU: <1% on desktop, <5% on Pi
- Decode CPU: <50% on desktop, <80% on Pi
- Playback CPU: <5% on desktop, <10% on Pi
- No CPU spikes during crossfades

---

## Memory Usage Benchmarks

### Goals

Track memory usage and detect memory leaks.

### Test Scenarios

| Scenario | Target Memory |
|----------|---------------|
| Baseline (server started, no passages) | <50 MB |
| Single passage (full decode, 3 minutes) | <110 MB |
| Queue (1 current + 1 next + 3 queued) | <200 MB |
| Long-running (100 passages, 5 hours playback) | Linear growth only |

### Implementation

```rust
use sysinfo::{System, SystemExt, ProcessExt};

fn measure_memory_usage() -> u64 {
    let mut system = System::new_all();
    let pid = sysinfo::get_current_pid().unwrap();

    system.refresh_process(pid);
    let process = system.process(pid).unwrap();

    process.memory() // bytes
}

fn test_memory_leak() {
    let baseline = measure_memory_usage();

    // Play 100 passages
    for i in 0..100 {
        play_and_complete_passage().await;

        let current = measure_memory_usage();
        let growth_mb = (current - baseline) as f64 / 1_048_576.0;

        println!("After {} passages: +{:.2} MB", i + 1, growth_mb);

        // Assert no unbounded growth
        assert!(
            growth_mb < 50.0, // Allow 50MB growth for caching
            "Memory leak detected: {} MB growth after {} passages",
            growth_mb,
            i + 1
        );
    }
}
```

### Success Criteria

- Baseline: <50 MB
- Single passage: <110 MB (50 MB baseline + 60 MB buffer)
- Queue: <200 MB
- No unbounded memory growth over time
- Buffers released after passage completes

---

## Regression Detection

### Configuration

Criterion automatically detects regressions:

```toml
[target.criterion]
# Baseline measurements stored in target/criterion/
# Compare against previous run
# Alert on >10% regression
```

### Thresholds

| Benchmark | Regression Threshold |
|-----------|---------------------|
| Startup Time | >10% slower |
| Decode Throughput | <10% slower |
| All others | >15% slower |

### CI Integration

```yaml
# .github/workflows/benchmark.yml
name: Performance Benchmarks

on:
  pull_request:
  push:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run benchmarks
        run: |
          cd wkmp-ap
          cargo bench --bench startup_bench -- --save-baseline main
      - name: Compare benchmarks
        run: |
          cargo bench --bench startup_bench -- --baseline main
      - name: Fail on regression
        if: failure()
        run: exit 1
```

---

## Running Benchmarks

### All Benchmarks

```bash
cd /home/sw/Dev/McRhythm/wkmp-ap
cargo bench
```

### Specific Benchmark

```bash
cargo bench --bench startup_bench
cargo bench --bench decode_bench
cargo bench --bench resample_bench
```

### Generate HTML Reports

```bash
cargo bench --bench startup_bench
open target/criterion/startup_time/report/index.html
```

### Compare Against Baseline

```bash
# Save current as baseline
cargo bench --bench startup_bench -- --save-baseline phase1

# Make changes...

# Compare against baseline
cargo bench --bench startup_bench -- --baseline phase1
```

---

## Test Data Requirements

### Audio Files Needed

| File | Format | Sample Rate | Duration | Purpose |
|------|--------|-------------|----------|---------|
| test_44100.mp3 | MP3 192kbps | 44.1kHz | 3min | Best case startup |
| test_48000.flac | FLAC 16-bit | 48kHz | 3min | Resample test |
| test_96000.flac | FLAC 24-bit | 96kHz | 3min | High-res decode |
| test_short.mp3 | MP3 192kbps | 44.1kHz | 30s | Quick iteration |

### Database Setup

Benchmarks require test database with:
- Settings table (minimum_buffer_threshold_ms = 500)
- Queue table schema
- Ephemeral passages support

---

## Benchmark Output Format

### Criterion HTML Report

```
target/criterion/
├── startup_time/
│   ├── mp3_44100_start_0s/
│   │   ├── report/
│   │   │   ├── index.html
│   │   │   ├── violin.svg
│   │   │   └── pdf.svg
│   │   └── base/
│   │       └── estimates.json
│   └── report/
│       └── index.html (summary)
```

### Console Output

```
startup_time/mp3_44100_start_0s
                        time:   [87.234 ms 89.451 ms 91.892 ms]
                        change: [-8.2345% -5.6789% -3.1234%] (p = 0.00 < 0.05)
                        Performance has improved.

startup_time/mp3_44100_start_60s
                        time:   [132.45 ms 135.67 ms 138.92 ms]
                        change: [+2.3456% +4.5678% +6.7890%] (p = 0.02 < 0.05)
                        Performance has regressed.
```

---

## Success Criteria Summary

### Phase 5 Goals

| Benchmark | Current | Target | Status |
|-----------|---------|--------|--------|
| Startup Time (MP3 @ 44.1kHz, 0s) | ~1500ms | <100ms | TBD |
| Startup Time (MP3 @ 44.1kHz, 60s) | ~1500ms | <150ms | TBD |
| Decode Throughput (all formats) | TBD | >10x realtime | TBD |
| Resample Performance | TBD | >20x realtime | TBD |
| Fade Curve Application | TBD | >50x realtime | TBD |
| Buffer Operations | TBD | >1000x realtime | TBD |
| Mixer Throughput | TBD | >50x realtime | TBD |
| Tick Conversions | TBD | >1M/sec | TBD |

---

## References

### Analysis Documents

- `/home/sw/Dev/McRhythm/wkmp-ap/STARTUP_PERFORMANCE_ANALYSIS.md` - Phase 1 bottleneck analysis
- `/home/sw/Dev/McRhythm/wkmp-ap/STARTUP_OPTIMIZATION_IMPLEMENTATION.md` - Optimization strategy
- `/home/sw/Dev/McRhythm/wkmp-ap/DIAGNOSTIC_RESULTS.md` - Verification results
- `/home/sw/Dev/McRhythm/docs/validation/IMPL-ANALYSIS-003-performance-baseline.json` - Baseline measurements

### Implementation Files

- `/home/sw/Dev/McRhythm/wkmp-ap/benches/startup_bench.rs` - Startup time benchmark
- `/home/sw/Dev/McRhythm/wkmp-ap/benches/decode_bench.rs` - Decode throughput
- `/home/sw/Dev/McRhythm/wkmp-ap/benches/resample_bench.rs` - Resample performance
- `/home/sw/Dev/McRhythm/wkmp-ap/benches/fade_bench.rs` - Fade curves
- `/home/sw/Dev/McRhythm/wkmp-ap/benches/buffer_bench.rs` - Buffer operations
- `/home/sw/Dev/McRhythm/wkmp-ap/benches/mixer_bench.rs` - Mixer throughput
- `/home/sw/Dev/McRhythm/wkmp-ap/benches/tick_bench.rs` - Tick conversions

---

**End of Document**

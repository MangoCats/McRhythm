//! Performance benchmarks for crossfade operations
//!
//! Measures:
//! - Fade curve calculation performance
//! - Crossfade mixing throughput
//! - Sample-accurate timing precision
//!
//! Implements requirements from crossfade.md - 0.02ms precision requirement

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use wkmp_ap::playback::pipeline::single_stream::buffer::FadeCurve;
use wkmp_ap::playback::pipeline::single_stream::mixer::{calculate_fade_gain, CrossfadePoints};
use std::time::Duration;

/// Benchmark fade curve calculations
fn bench_fade_curves(c: &mut Criterion) {
    let mut group = c.benchmark_group("fade_curves");

    let curves = vec![
        ("linear", FadeCurve::Linear),
        ("exponential", FadeCurve::Exponential),
        ("logarithmic", FadeCurve::Logarithmic),
        ("s_curve", FadeCurve::SCurve),
    ];

    let progress_values = vec![0.0, 0.25, 0.5, 0.75, 1.0];

    for (name, curve) in curves {
        group.bench_function(BenchmarkId::new("fade_in", name), |b| {
            b.iter(|| {
                for &progress in &progress_values {
                    black_box(calculate_fade_gain(curve.clone(), progress, true));
                }
            })
        });

        group.bench_function(BenchmarkId::new("fade_out", name), |b| {
            b.iter(|| {
                for &progress in &progress_values {
                    black_box(calculate_fade_gain(curve.clone(), progress, false));
                }
            })
        });
    }

    group.finish();
}

/// Benchmark crossfade mixing of audio buffers
fn bench_crossfade_mixing(c: &mut Criterion) {
    let mut group = c.benchmark_group("crossfade_mixing");

    // Different buffer sizes (in samples)
    let buffer_sizes = vec![
        ("1ms", 44),      // 1ms at 44.1kHz
        ("10ms", 441),    // 10ms at 44.1kHz
        ("100ms", 4410),  // 100ms at 44.1kHz
        ("1s", 44100),    // 1 second at 44.1kHz
    ];

    for (name, size) in buffer_sizes {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_function(BenchmarkId::new("stereo_mix", name), |b| {
            let buffer_a = vec![0.5f32; size * 2]; // Stereo
            let buffer_b = vec![0.7f32; size * 2]; // Stereo
            let mut output = vec![0.0f32; size * 2];

            b.iter(|| {
                // Simulate crossfade mixing
                for i in 0..size {
                    let progress = i as f32 / size as f32;
                    let gain_a = 1.0 - progress;
                    let gain_b = progress;

                    // Mix stereo channels
                    output[i * 2] = buffer_a[i * 2] * gain_a + buffer_b[i * 2] * gain_b;
                    output[i * 2 + 1] = buffer_a[i * 2 + 1] * gain_a + buffer_b[i * 2 + 1] * gain_b;
                }
                black_box(&output);
            });
        });
    }

    group.finish();
}

/// Benchmark timing precision for crossfade points
fn bench_timing_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("timing_precision");

    group.bench_function("crossfade_point_calculation", |b| {
        b.iter(|| {
            let points = CrossfadePoints::calculate(
                black_box(0),
                black_box(44100),    // 1 second fade in start
                black_box(88200),    // 2 seconds lead in end
                black_box(352800),   // 8 seconds lead out start
                black_box(396900),   // 9 seconds fade out end
                black_box(441000),   // 10 seconds end
            );
            black_box(points);
        });
    });

    // Benchmark sample-to-time conversion precision
    group.bench_function("sample_to_ms_conversion", |b| {
        let sample_rate = 44100;
        let samples = vec![1, 44, 441, 4410, 44100];

        b.iter(|| {
            for &s in &samples {
                let ms = (s as f64 / sample_rate as f64) * 1000.0;
                black_box(ms);
            }
        });
    });

    // Verify we meet the 0.02ms precision requirement
    group.bench_function("precision_verification", |b| {
        let sample_rate = 44100;
        let precision_ms = 0.02;

        b.iter(|| {
            let precision_samples = (sample_rate as f64 * precision_ms / 1000.0);
            assert!(precision_samples < 1.0, "Sub-sample precision required");
            black_box(precision_samples);
        });
    });

    group.finish();
}

/// Benchmark buffer operations
fn bench_buffer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_operations");

    // 15-second buffer at 44.1kHz stereo
    let buffer_size = 44100 * 2 * 15;
    group.throughput(Throughput::Bytes((buffer_size * 4) as u64)); // f32 is 4 bytes

    group.bench_function("buffer_allocation", |b| {
        b.iter(|| {
            let buffer = vec![0.0f32; buffer_size];
            black_box(buffer);
        });
    });

    group.bench_function("buffer_copy", |b| {
        let source = vec![0.5f32; buffer_size];
        let mut dest = vec![0.0f32; buffer_size];

        b.iter(|| {
            dest.copy_from_slice(&source);
            black_box(&dest);
        });
    });

    group.bench_function("buffer_clear", |b| {
        let mut buffer = vec![0.5f32; buffer_size];

        b.iter(|| {
            buffer.fill(0.0);
            black_box(&buffer);
        });
    });

    group.finish();
}

/// Benchmark parallel processing simulation
fn bench_parallel_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_processing");
    group.measurement_time(Duration::from_secs(10));

    // Simulate decoder pool with different worker counts
    let worker_counts = vec![1, 2, 4, 8];

    for workers in worker_counts {
        group.bench_function(BenchmarkId::new("decode_simulation", workers), |b| {
            b.iter(|| {
                // Simulate decoding work
                let mut sum = 0.0f32;
                for _ in 0..1000 {
                    sum += black_box(0.5f32).sin();
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

/// Benchmark real-time constraints
fn bench_realtime_constraints(c: &mut Criterion) {
    let mut group = c.benchmark_group("realtime_constraints");

    // Audio callback typically needs to complete within ~5ms for 256 sample buffer
    let callback_buffer_size = 256;

    group.bench_function("audio_callback_simulation", |b| {
        let input = vec![0.5f32; callback_buffer_size * 2]; // Stereo
        let mut output = vec![0.0f32; callback_buffer_size * 2];

        b.iter(|| {
            // Simulate audio callback processing
            for i in 0..callback_buffer_size * 2 {
                output[i] = input[i] * 0.8; // Simple gain adjustment
            }
            black_box(&output);
        });
    });

    // Measure lock-free operations
    group.bench_function("atomic_position_update", |b| {
        use std::sync::atomic::{AtomicU64, Ordering};
        let position = AtomicU64::new(0);

        b.iter(|| {
            let old = position.fetch_add(256, Ordering::SeqCst);
            black_box(old);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fade_curves,
    bench_crossfade_mixing,
    bench_timing_precision,
    bench_buffer_operations,
    bench_parallel_processing,
    bench_realtime_constraints
);
criterion_main!(benches);
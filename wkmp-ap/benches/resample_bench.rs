//! Resample Performance Benchmark
//!
//! Measures rubato resampling throughput to verify minimal overhead.
//!
//! **Goal:** Resampling should add minimal overhead to decode process
//! **Target:** >20x realtime
//! **Stretch:** >30x realtime
//!
//! ## Test Scenarios
//!
//! - 48000 Hz → 44100 Hz (ratio: 0.91875) → Expected: >25x realtime
//! - 96000 Hz → 44100 Hz (ratio: 0.459375) → Expected: >20x realtime
//! - 192000 Hz → 44100 Hz (ratio: 0.2296875) → Expected: >15x realtime
//!
//! ## Requirements Traceability
//!
//! - [SSD-DEC-040] Resample all audio to 44.1kHz for mixer
//! - [SSD-PBUF-028] Enable instant play start with background decode

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rubato::{Resampler, SincFixedIn, InterpolationType, InterpolationParameters, WindowFunction};
use std::time::Instant;

/// Helper: Resample stereo interleaved samples
fn resample_stereo_interleaved(
    input: &[f32],
    source_rate: u32,
    target_rate: u32,
) -> Vec<f32> {
    if source_rate == target_rate {
        return input.to_vec();
    }

    let channels = 2;
    let chunk_size = input.len() / channels;

    // Configure rubato resampler
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
        chunk_size,
        channels,
    )
    .expect("Failed to create resampler");

    // De-interleave for rubato
    let mut left = Vec::with_capacity(chunk_size);
    let mut right = Vec::with_capacity(chunk_size);
    for i in 0..chunk_size {
        left.push(input[i * 2]);
        right.push(input[i * 2 + 1]);
    }

    // Resample
    let input_frames = vec![left, right];
    let output_frames = resampler
        .process(&input_frames, None)
        .expect("Failed to resample");

    // Re-interleave
    let mut output = Vec::with_capacity(output_frames[0].len() * 2);
    for i in 0..output_frames[0].len() {
        output.push(output_frames[0][i]);
        output.push(output_frames[1][i]);
    }

    output
}

/// Benchmark: Resample 48kHz → 44.1kHz
///
/// **Target:** >25x realtime
fn bench_resample_48k_to_44k(c: &mut Criterion) {
    let mut group = c.benchmark_group("resample_performance");

    let source_rate = 48_000u32;
    let target_rate = 44_100u32;
    let duration_s = 30.0; // 30 seconds
    let sample_count = (duration_s * source_rate as f64) as usize;

    group.throughput(Throughput::Elements((duration_s * 1000.0) as u64)); // ms

    group.bench_function("48k_to_44k", |b| {
        // Create test audio data (stereo interleaved)
        let stereo_samples: Vec<f32> = vec![0.5; sample_count * 2];

        b.iter(|| {
            let start = Instant::now();

            let resampled = resample_stereo_interleaved(
                black_box(&stereo_samples),
                source_rate,
                target_rate,
            );

            let elapsed = start.elapsed().as_secs_f64();
            let realtime_factor = duration_s / elapsed;

            println!(
                "48kHz→44.1kHz: {:.2}x realtime ({:.2}s audio in {:.3}s)",
                realtime_factor, duration_s, elapsed
            );

            // Assert >20x realtime (target is >25x)
            assert!(
                realtime_factor > 20.0,
                "Resample speed {:.2}x is below 20x realtime target",
                realtime_factor
            );

            black_box(resampled);
        });
    });

    group.finish();
}

/// Benchmark: Resample 96kHz → 44.1kHz
///
/// **Target:** >20x realtime
fn bench_resample_96k_to_44k(c: &mut Criterion) {
    let mut group = c.benchmark_group("resample_performance");

    let source_rate = 96_000u32;
    let target_rate = 44_100u32;
    let duration_s = 30.0;
    let sample_count = (duration_s * source_rate as f64) as usize;

    group.throughput(Throughput::Elements((duration_s * 1000.0) as u64));

    group.bench_function("96k_to_44k", |b| {
        let stereo_samples: Vec<f32> = vec![0.5; sample_count * 2];

        b.iter(|| {
            let start = Instant::now();

            let resampled = resample_stereo_interleaved(
                black_box(&stereo_samples),
                source_rate,
                target_rate,
            );

            let elapsed = start.elapsed().as_secs_f64();
            let realtime_factor = duration_s / elapsed;

            println!(
                "96kHz→44.1kHz: {:.2}x realtime",
                realtime_factor
            );

            assert!(
                realtime_factor > 20.0,
                "Resample speed {:.2}x is below 20x realtime target",
                realtime_factor
            );

            black_box(resampled);
        });
    });

    group.finish();
}

/// Benchmark: Resample 192kHz → 44.1kHz
///
/// **Target:** >15x realtime (high sample rate = more work)
fn bench_resample_192k_to_44k(c: &mut Criterion) {
    let mut group = c.benchmark_group("resample_performance");

    let source_rate = 192_000u32;
    let target_rate = 44_100u32;
    let duration_s = 30.0;
    let sample_count = (duration_s * source_rate as f64) as usize;

    group.throughput(Throughput::Elements((duration_s * 1000.0) as u64));

    group.bench_function("192k_to_44k", |b| {
        let stereo_samples: Vec<f32> = vec![0.5; sample_count * 2];

        b.iter(|| {
            let start = Instant::now();

            let resampled = resample_stereo_interleaved(
                black_box(&stereo_samples),
                source_rate,
                target_rate,
            );

            let elapsed = start.elapsed().as_secs_f64();
            let realtime_factor = duration_s / elapsed;

            println!(
                "192kHz→44.1kHz: {:.2}x realtime",
                realtime_factor
            );

            assert!(
                realtime_factor > 15.0,
                "Resample speed {:.2}x is below 15x realtime target",
                realtime_factor
            );

            black_box(resampled);
        });
    });

    group.finish();
}

/// Benchmark: Resample various chunk sizes
///
/// Tests impact of chunk size on resampling performance
fn bench_resample_chunk_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("resample_chunk_sizes");

    let source_rate = 48_000u32;
    let target_rate = 44_100u32;

    let chunk_durations = vec![
        ("100ms", 0.1),
        ("500ms", 0.5),
        ("1s", 1.0),
        ("5s", 5.0),
    ];

    for (name, duration_s) in chunk_durations {
        let sample_count = (duration_s * source_rate as f64) as usize;

        group.bench_function(BenchmarkId::from_parameter(name), |b| {
            let stereo_samples: Vec<f32> = vec![0.5; sample_count * 2];

            b.iter(|| {
                let resampled = resample_stereo_interleaved(
                    black_box(&stereo_samples),
                    source_rate,
                    target_rate,
                );

                black_box(resampled);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_resample_48k_to_44k,
    bench_resample_96k_to_44k,
    bench_resample_192k_to_44k,
    bench_resample_chunk_sizes,
);
criterion_main!(benches);

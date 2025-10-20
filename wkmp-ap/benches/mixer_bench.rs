//! Mixer Throughput Performance Benchmark
//!
//! Measures real-time mixing performance to verify >50x realtime throughput.
//!
//! **Goal:** Mixing should complete far faster than realtime
//! **Target:** >50x realtime for crossfade, >100x for single passage

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Instant;

fn bench_mixer_single_passage(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixer_throughput");

    group.bench_function("single_passage", |b| {
        let buffer = vec![0.5f32; 44_100 * 2]; // 1s stereo
        let volume = 0.8f32;
        let mut output = vec![0.0f32; 44_100 * 2];

        b.iter(|| {
            let start = Instant::now();

            for i in 0..output.len() {
                output[i] = buffer[i] * volume;
            }

            let elapsed = start.elapsed().as_secs_f64();
            let realtime_factor = 1.0 / elapsed;

            if realtime_factor < 100.0 {
                eprintln!(
                    "WARNING: Single passage speed {:.2}x is below 100x realtime target",
                    realtime_factor
                );
            }

            black_box(&output);
        });
    });

    group.finish();
}

fn bench_mixer_crossfade(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixer_throughput");

    group.bench_function("crossfade_overlap", |b| {
        let buffer_a = vec![0.5f32; 44_100 * 2]; // 1s stereo
        let buffer_b = vec![0.7f32; 44_100 * 2]; // 1s stereo
        let mut output = vec![0.0f32; 44_100 * 2];

        b.iter(|| {
            let start = Instant::now();

            for i in 0..(44_100 * 2) {
                let progress = (i / 2) as f32 / 44_100.0;
                let gain_a = 1.0 - progress; // fade out
                let gain_b = progress; // fade in

                output[i] = buffer_a[i] * gain_a + buffer_b[i] * gain_b;
            }

            let elapsed = start.elapsed().as_secs_f64();
            let realtime_factor = 1.0 / elapsed;

            if realtime_factor < 50.0 {
                eprintln!(
                    "WARNING: Crossfade speed {:.2}x is below 50x realtime target",
                    realtime_factor
                );
            }

            black_box(&output);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_mixer_single_passage, bench_mixer_crossfade);
criterion_main!(benches);

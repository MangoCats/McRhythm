//! Tick Conversion Performance Benchmark
//!
//! Measures timing arithmetic performance to verify >1M conversions/second.
//!
//! **Goal:** Tick conversions should be trivial arithmetic
//! **Target:** >1M conversions/second
//! **Stretch:** >5M conversions/second

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Instant;

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

fn bench_tick_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("tick_conversions");

    let test_values = vec![0, 100, 1000, 10_000, 100_000, 1_000_000];

    group.bench_function("ms_to_ticks", |b| {
        b.iter(|| {
            let start = Instant::now();

            for _ in 0..1_000_000 {
                for &val in &test_values {
                    black_box(ms_to_ticks(black_box(val)));
                }
            }

            let elapsed = start.elapsed().as_secs_f64();
            let conversions_per_sec = (1_000_000.0 * test_values.len() as f64) / elapsed;

            if conversions_per_sec < 1_000_000.0 {
                eprintln!(
                    "WARNING: Conversion speed {:.0}/s is below 1M/s target",
                    conversions_per_sec
                );
            }
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

criterion_group!(benches, bench_tick_conversions);
criterion_main!(benches);

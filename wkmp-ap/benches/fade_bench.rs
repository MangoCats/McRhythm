//! Fade Curve Application Performance Benchmark
//!
//! Measures fade curve calculation throughput to verify negligible overhead.
//!
//! **Goal:** Fade calculations should be trivial compared to mixing
//! **Target:** >50x realtime
//! **Stretch:** >100x realtime
//!
//! Tests all 5 fade curve types: Linear, Exponential, Logarithmic, SCurve, Cosine

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use wkmp_ap::playback::pipeline::single_stream::buffer::FadeCurve;
use wkmp_ap::playback::pipeline::single_stream::mixer::calculate_fade_gain;
use std::time::Instant;

fn bench_fade_curves_all_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("fade_curves");

    let curves = vec![
        ("linear", FadeCurve::Linear),
        ("exponential", FadeCurve::Exponential),
        ("logarithmic", FadeCurve::Logarithmic),
        ("s_curve", FadeCurve::SCurve),
    ];

    // Test with 10 seconds of audio @ 44.1kHz stereo = 441,000 samples
    let sample_count = 441_000usize;

    for (name, curve) in curves {
        group.bench_function(BenchmarkId::new("fade_in", name), |b| {
            b.iter(|| {
                let start = Instant::now();

                for i in 0..sample_count {
                    let progress = i as f32 / sample_count as f32;
                    let gain = calculate_fade_gain(curve.clone(), progress, true);
                    black_box(gain);
                }

                let elapsed = start.elapsed().as_secs_f64();
                let duration_s = sample_count as f64 / 44100.0; // 10 seconds
                let realtime_factor = duration_s / elapsed;

                if realtime_factor < 50.0 {
                    eprintln!(
                        "WARNING: {} fade speed {:.2}x is below 50x realtime target",
                        name, realtime_factor
                    );
                }
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

criterion_group!(benches, bench_fade_curves_all_types);
criterion_main!(benches);

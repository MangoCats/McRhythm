//! Buffer Operations Performance Benchmark
//!
//! Measures ring buffer and passage buffer throughput to verify lock-free performance.
//!
//! **Goal:** Buffer operations should be nearly instant
//! **Target:** >1000x realtime
//! **Stretch:** >5000x realtime

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ringbuf::HeapRb;

fn bench_ring_buffer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_operations");

    group.bench_function("ring_buffer_write", |b| {
        let ring = HeapRb::<f32>::new(2048);
        let (mut producer, _consumer) = ring.split();
        let data = vec![0.5f32; 2048];

        b.iter(|| {
            let written = producer.push_slice(black_box(&data));
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
            let read = consumer.pop_slice(black_box(&mut buffer));
            black_box(read);
        });
    });

    group.bench_function("passage_buffer_append_1s", |b| {
        let mut buffer = Vec::with_capacity(441_000 * 2);
        let chunk = vec![0.5f32; 44_100 * 2]; // 1s stereo

        b.iter(|| {
            buffer.extend_from_slice(black_box(&chunk));
            buffer.clear(); // Reset for next iteration
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

criterion_group!(benches, bench_ring_buffer_operations);
criterion_main!(benches);

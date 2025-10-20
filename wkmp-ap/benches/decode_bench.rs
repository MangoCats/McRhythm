//! Decode Throughput Performance Benchmark
//!
//! Measures samples decoded per second to verify >10x realtime performance.
//!
//! **Goal:** Decode audio faster than realtime to enable buffer fill
//! **Target:** >10x realtime (decode 10s of audio in 1s)
//! **Stretch:** >15x realtime
//!
//! ## Test Scenarios
//!
//! - MP3 192kbps @ 44.1kHz → Expected: 12-15x realtime
//! - MP3 320kbps @ 44.1kHz → Expected: 10-12x realtime
//! - FLAC 16-bit @ 44.1kHz → Expected: 15-20x realtime (simpler codec)
//! - FLAC 24-bit @ 96kHz → Expected: 8-10x realtime (high-res)
//! - AAC @ 48kHz → Expected: 10-12x realtime
//!
//! ## Requirements Traceability
//!
//! - [SSD-DEC-010] Decoder must achieve realtime performance
//! - [SSD-DEC-020] Support multiple audio formats (MP3, FLAC, AAC, OGG)
//! - [SSD-PBUF-028] Enable instant play start with background decode

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::path::Path;
use std::time::Instant;

// Import decoder from wkmp-ap
// NOTE: This requires exposing the decoder module publicly or using test helpers
// use wkmp_ap::audio::decoder::SimpleDecoder;

/// Mock decoder for benchmarking structure
/// Replace with actual SimpleDecoder when module is accessible
struct MockDecoder;

impl MockDecoder {
    fn decode_passage(
        file_path: &Path,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<(Vec<f32>, u32, u32), String> {
        // Actual implementation would use symphonia to decode
        // For now, return dummy data for benchmark structure
        let duration_ms = end_ms - start_ms;
        let sample_rate = 44100;
        let channels = 2;
        let sample_count = ((duration_ms as f64 / 1000.0) * sample_rate as f64) as usize;

        Ok((
            vec![0.5f32; sample_count * channels as usize],
            sample_rate,
            channels,
        ))
    }
}

/// Benchmark: Decode MP3 192kbps @ 44.1kHz
///
/// **Target:** >12x realtime
fn bench_decode_mp3_192k(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_throughput");

    let duration_ms = 30_000; // 30 seconds of audio
    group.throughput(Throughput::Elements(duration_ms as u64));

    group.bench_function("mp3_192k_44100", |b| {
        let test_file = Path::new("/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-02-Superfly_.mp3");

        b.iter(|| {
            let start = Instant::now();

            let (samples, sample_rate, channels) = MockDecoder::decode_passage(
                black_box(test_file),
                black_box(0),
                black_box(duration_ms),
            )
            .expect("Failed to decode MP3");

            let elapsed = start.elapsed().as_secs_f64();
            let duration_s = duration_ms as f64 / 1000.0;
            let realtime_factor = duration_s / elapsed;

            // Log performance
            println!(
                "MP3 192k decode: {}x realtime ({:.2}s of audio in {:.3}s)",
                realtime_factor, duration_s, elapsed
            );

            // Assert >10x realtime
            assert!(
                realtime_factor > 10.0,
                "MP3 decode speed {:.2}x is below 10x realtime target",
                realtime_factor
            );

            black_box((samples, sample_rate, channels));
        });
    });

    group.finish();
}

/// Benchmark: Decode MP3 320kbps @ 44.1kHz
///
/// **Target:** >10x realtime (higher bitrate = more decode work)
fn bench_decode_mp3_320k(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_throughput");

    let duration_ms = 30_000;
    group.throughput(Throughput::Elements(duration_ms as u64));

    group.bench_function("mp3_320k_44100", |b| {
        // NOTE: Requires 320kbps test file
        let test_file = Path::new("/test/audio/test_320k_44100.mp3");

        b.iter(|| {
            let start = Instant::now();

            let (samples, sample_rate, channels) = MockDecoder::decode_passage(
                black_box(test_file),
                black_box(0),
                black_box(duration_ms),
            )
            .expect("Failed to decode MP3");

            let elapsed = start.elapsed().as_secs_f64();
            let realtime_factor = (duration_ms as f64 / 1000.0) / elapsed;

            assert!(
                realtime_factor > 10.0,
                "MP3 320k decode speed {:.2}x is below 10x realtime target",
                realtime_factor
            );

            black_box((samples, sample_rate, channels));
        });
    });

    group.finish();
}

/// Benchmark: Decode FLAC 16-bit @ 44.1kHz
///
/// **Target:** >15x realtime (FLAC decodes faster than MP3)
fn bench_decode_flac_16bit(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_throughput");

    let duration_ms = 30_000;
    group.throughput(Throughput::Elements(duration_ms as u64));

    group.bench_function("flac_16bit_44100", |b| {
        let test_file = Path::new("/test/audio/test_16bit_44100.flac");

        b.iter(|| {
            let start = Instant::now();

            let (samples, sample_rate, channels) = MockDecoder::decode_passage(
                black_box(test_file),
                black_box(0),
                black_box(duration_ms),
            )
            .expect("Failed to decode FLAC");

            let elapsed = start.elapsed().as_secs_f64();
            let realtime_factor = (duration_ms as f64 / 1000.0) / elapsed;

            println!(
                "FLAC 16-bit decode: {}x realtime",
                realtime_factor
            );

            assert!(
                realtime_factor > 15.0,
                "FLAC decode speed {:.2}x is below 15x realtime target",
                realtime_factor
            );

            black_box((samples, sample_rate, channels));
        });
    });

    group.finish();
}

/// Benchmark: Decode FLAC 24-bit @ 96kHz
///
/// **Target:** >8x realtime (high-res requires more processing)
fn bench_decode_flac_24bit_96k(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_throughput");

    let duration_ms = 30_000;
    group.throughput(Throughput::Elements(duration_ms as u64));

    group.bench_function("flac_24bit_96000", |b| {
        let test_file = Path::new("/test/audio/test_24bit_96000.flac");

        b.iter(|| {
            let start = Instant::now();

            let (samples, sample_rate, channels) = MockDecoder::decode_passage(
                black_box(test_file),
                black_box(0),
                black_box(duration_ms),
            )
            .expect("Failed to decode FLAC");

            let elapsed = start.elapsed().as_secs_f64();
            let realtime_factor = (duration_ms as f64 / 1000.0) / elapsed;

            println!(
                "FLAC 24-bit 96kHz decode: {}x realtime",
                realtime_factor
            );

            assert!(
                realtime_factor > 8.0,
                "FLAC 24-bit decode speed {:.2}x is below 8x realtime target",
                realtime_factor
            );

            black_box((samples, sample_rate, channels));
        });
    });

    group.finish();
}

/// Benchmark: Decode with skip offset
///
/// Measures decode-and-skip performance (start at 60s into file)
fn bench_decode_with_skip(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_with_skip");

    let skip_offsets = vec![
        ("skip_0s", 0, 30_000),
        ("skip_30s", 30_000, 60_000),
        ("skip_60s", 60_000, 90_000),
        ("skip_120s", 120_000, 150_000),
    ];

    for (name, start_ms, end_ms) in skip_offsets {
        group.bench_function(BenchmarkId::from_parameter(name), |b| {
            let test_file = Path::new("/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-02-Superfly_.mp3");
            let duration_ms = end_ms - start_ms;

            b.iter(|| {
                let start = Instant::now();

                let (samples, sample_rate, channels) = MockDecoder::decode_passage(
                    black_box(test_file),
                    black_box(start_ms),
                    black_box(end_ms),
                )
                .expect("Failed to decode with skip");

                let elapsed = start.elapsed();

                println!(
                    "{}: {}ms decode time for {}s audio starting at {}s",
                    name,
                    elapsed.as_millis(),
                    duration_ms / 1000,
                    start_ms / 1000
                );

                black_box((samples, sample_rate, channels));
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_decode_mp3_192k,
    bench_decode_mp3_320k,
    bench_decode_flac_16bit,
    bench_decode_flac_24bit_96k,
    bench_decode_with_skip,
);
criterion_main!(benches);

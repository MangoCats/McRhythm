use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Expected track durations from MusicBrainz (in seconds)
/// Michael Jackson - Thriller (1982 original release)
const EXPECTED_DURATIONS: &[u32] = &[
    // Track durations from MusicBrainz (release MBID: 95a7c47a-7d2d-4992-939c-689b33e49019)
    353, // 1. Wanna Be Startin' Somethin' (5:53)
    221, // 2. Baby Be Mine (3:41)
    239, // 3. The Girl Is Mine (3:59)
    358, // 4. Thriller (5:58)
    288, // 5. Beat It (4:48)
    302, // 6. Billie Jean (5:02)
    257, // 7. Human Nature (4:17)
    260, // 8. P.Y.T. (Pretty Young Thing) (4:20)
    297, // 9. The Lady in My Life (4:57)
];

#[derive(Debug, Clone)]
struct SilenceParams {
    index: usize,
    threshold_db: f32,
    min_duration_secs: f32,
    window_size_ms: f32,
    window_step_ms: f32,
}

#[derive(Debug)]
struct Segment {
    start_sample: u64,
    end_sample: u64,
    duration_secs: f64,
}

/// Calculate RMS (root mean square) amplitude and convert to dB
fn calculate_db(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return -100.0;
    }

    let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

    if rms > 0.0 {
        20.0 * rms.log10()
    } else {
        -100.0
    }
}

/// Decode MP3 file into mono samples
fn decode_mp3(file_path: &Path) -> Result<(Vec<f32>, u32), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(file_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    hint.with_extension("mp3");

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
    let mut format = probed.format;

    let track = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("No audio track found")?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.ok_or("No sample rate")?;

    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet)? {
            AudioBufferRef::F32(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx];
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::U8(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        let sample = (buf.chan(ch)[frame_idx] as f32 - 128.0) / 128.0;
                        sum += sample;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::U16(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        let sample = (buf.chan(ch)[frame_idx] as f32 - 32768.0) / 32768.0;
                        sum += sample;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::U24(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        let sample = (buf.chan(ch)[frame_idx].inner() as f32 - 8388608.0) / 8388608.0;
                        sum += sample;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::U32(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        let sample = (buf.chan(ch)[frame_idx] as f32 - 2147483648.0) / 2147483648.0;
                        sum += sample;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::S8(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx] as f32 / 128.0;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::S16(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx] as f32 / 32768.0;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::S24(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx].inner() as f32 / 8388608.0;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::S32(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx] as f32 / 2147483648.0;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::F64(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx] as f32;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
        }
    }

    Ok((all_samples, sample_rate))
}

/// Analyze pre-decoded samples for silence gaps with given parameters
fn analyze_silence(
    samples: &[f32],
    sample_rate: u32,
    params: &SilenceParams,
) -> Result<Vec<Segment>, Box<dyn std::error::Error>> {
    // Calculate window sizes based on parameters
    let window_size = (sample_rate as f32 * params.window_size_ms / 1000.0) as usize;
    let window_step = (sample_rate as f32 * params.window_step_ms / 1000.0) as usize;
    let min_silence_samples = (sample_rate as f32 * params.min_duration_secs) as usize;

    let mut is_silent = Vec::new();

    for window_start in (0..samples.len()).step_by(window_step) {
        let window_end = (window_start + window_size).min(samples.len());
        let db = calculate_db(&samples[window_start..window_end]);
        is_silent.push(db < params.threshold_db);
    }

    // Find continuous silence regions
    let mut silence_regions: Vec<(usize, usize)> = Vec::new();
    let mut silence_start = None;

    for (idx, &silent) in is_silent.iter().enumerate() {
        match (silent, silence_start) {
            (true, None) => silence_start = Some(idx),
            (false, Some(start)) => {
                let duration_windows = idx - start;
                let duration_samples = duration_windows * window_step;
                if duration_samples >= min_silence_samples {
                    silence_regions.push((start * window_step, idx * window_step));
                }
                silence_start = None;
            }
            _ => {}
        }
    }

    // Handle case where silence extends to end
    if let Some(start) = silence_start {
        let idx = is_silent.len();
        let duration_windows = idx - start;
        let duration_samples = duration_windows * window_step;
        if duration_samples >= min_silence_samples {
            silence_regions.push((start * window_step, idx * window_step));
        }
    }

    // Convert silence regions to audio segments
    let mut segments = Vec::new();
    let mut current_start = 0u64;

    for (silence_start, silence_end) in silence_regions {
        if silence_start > current_start as usize {
            segments.push(Segment {
                start_sample: current_start,
                end_sample: silence_start as u64,
                duration_secs: (silence_start as u64 - current_start) as f64 / sample_rate as f64,
            });
        }
        current_start = silence_end as u64;
    }

    // Add final segment if exists
    if (current_start as usize) < samples.len() {
        segments.push(Segment {
            start_sample: current_start,
            end_sample: samples.len() as u64,
            duration_secs: (samples.len() as u64 - current_start) as f64 / sample_rate as f64,
        });
    }

    Ok(segments)
}

/// Calculate quality score comparing detected segments to expected durations
fn calculate_score(segments: &[Segment]) -> f64 {
    if segments.len() != EXPECTED_DURATIONS.len() {
        // Heavy penalty for wrong track count
        let count_diff = (segments.len() as i32 - EXPECTED_DURATIONS.len() as i32).abs();
        return 1000.0 * count_diff as f64;
    }

    // Calculate mean absolute error in duration
    let mut total_error = 0.0;
    for (seg, &expected) in segments.iter().zip(EXPECTED_DURATIONS.iter()) {
        let error = (seg.duration_secs - expected as f64).abs();
        total_error += error;
    }

    total_error / segments.len() as f64
}

/// Read parameters from CSV file
fn read_params_file(path: &Path) -> Result<Vec<SilenceParams>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut params = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if line_num == 0 {
            continue; // Skip header
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != 5 {
            continue;
        }

        params.push(SilenceParams {
            index: parts[0].parse()?,
            threshold_db: parts[1].parse()?,
            min_duration_secs: parts[2].parse()?,
            window_size_ms: parts[3].parse()?,
            window_step_ms: parts[4].parse()?,
        });
    }

    Ok(params)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = Path::new(r"C:\Users\Mango Cat\Music\Jackson, Michael\Thriller.mp3");
    let params_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\wkmp-ai\silence_params_thriller.csv");

    if !file_path.exists() {
        eprintln!("Audio file not found: {}", file_path.display());
        return Ok(());
    }

    println!("Analyzing: {}", file_path.display());
    println!("Expected: {} tracks", EXPECTED_DURATIONS.len());
    println!();

    // Decode MP3 once
    print!("Decoding MP3... ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let (samples, sample_rate) = decode_mp3(file_path)?;
    println!("Done! {} samples at {} Hz ({:.2} seconds total)",
        samples.len(), sample_rate, samples.len() as f64 / sample_rate as f64);
    println!();

    let mut results = Vec::new();

    loop {
        // Re-read parameters file before each test
        let all_params = read_params_file(params_path)?;

        // Find next untested parameter set
        let next_index = results.len() + 1;
        let params = match all_params.iter().find(|p| p.index == next_index) {
            Some(p) => p.clone(),
            None => {
                println!("No more parameters to test (index {} not found)", next_index);
                break;
            }
        };

        println!("Test #{}: threshold={} dB, min_duration={:.1}s, window={:.0}ms, step={:.0}ms",
            params.index, params.threshold_db, params.min_duration_secs,
            params.window_size_ms, params.window_step_ms);

        let segments = analyze_silence(&samples, sample_rate, &params)?;
        let score = calculate_score(&segments);

        println!("  → Found {} segments, score={:.2}", segments.len(), score);

        if segments.len() == EXPECTED_DURATIONS.len() {
            println!("  → EXACT MATCH! Showing duration comparison:");
            for (j, (seg, &expected)) in segments.iter().zip(EXPECTED_DURATIONS.iter()).enumerate() {
                let diff = seg.duration_secs - expected as f64;
                println!("     Track {:2}: {:6.1}s vs {:3}s (diff: {:+6.1}s)",
                    j + 1, seg.duration_secs, expected, diff);
            }
        }
        println!();

        results.push((params, segments, score));
    }

    // Sort by score (lower is better)
    results.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

    println!("\n=== TOP 5 RESULTS ===\n");

    for (i, (params, segments, score)) in results.iter().take(5).enumerate() {
        println!("{}. Test #{}: Threshold={} dB, Min Duration={:.1}s, Window={:.0}ms, Step={:.0}ms",
            i + 1, params.index, params.threshold_db, params.min_duration_secs,
            params.window_size_ms, params.window_step_ms);
        println!("   Segments: {}, Score: {:.2}", segments.len(), score);
    }

    if let Some((best_params, _, _)) = results.first() {
        println!("\n=== RECOMMENDATION ===");
        println!("Test #{}: Silence threshold: {} dB", best_params.index, best_params.threshold_db);
        println!("Minimum duration: {:.1} seconds", best_params.min_duration_secs);
        println!("Window size: {:.0} ms", best_params.window_size_ms);
        println!("Window step: {:.0} ms", best_params.window_step_ms);
    }

    Ok(())
}

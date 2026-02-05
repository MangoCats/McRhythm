//! Passage boundary detection using silence analysis
//!
//! Detects passage boundaries within audio files by analyzing RMS energy levels.
//! Identifies silent regions that likely represent boundaries between passages.
//!
//! **[PLAN023]** Phase 0 - Simple silence-based boundary detection
//!
//! # Algorithm
//!
//! 1. Calculate RMS energy for each audio frame
//! 2. Identify regions below silence threshold
//! 3. Find silence regions longer than minimum duration
//! 4. Mark boundaries at silence midpoints
//! 5. Filter out passages shorter than minimum duration
//!
//! # SPEC017 Compliance
//!
//! Converts sample counts to ticks (1 tick = 1/28,224,000 second) for
//! sample-accurate precision as required by [REQ-AI-088-04].

use super::{memory_tracker, FileAudioData, PassageBoundary, TICK_RATE};
use anyhow::{Context, Result};
use std::path::Path;
use tracing::{debug, info, warn};

/// Silence detection threshold (RMS energy below this is considered silence)
const SILENCE_THRESHOLD: f32 = 0.01;

/// Minimum silence duration to consider a boundary (seconds)
const MIN_SILENCE_DURATION: f64 = 2.0;

/// Minimum passage duration (seconds)
const MIN_PASSAGE_DURATION: f64 = 30.0;

/// Convert sample count to SPEC017 ticks
///
/// Per REQ-AI-088-04: ticks = samples × (28,224,000 ÷ sample_rate)
fn samples_to_ticks(samples: usize, sample_rate: u32) -> i64 {
    let ticks_per_sample = TICK_RATE / sample_rate as i64;
    (samples as i64) * ticks_per_sample
}

/// Convert seconds to sample count for boundary detection
fn seconds_to_samples(seconds: f64, sample_rate: u32) -> usize {
    (seconds * sample_rate as f64) as usize
}

/// Convert SPEC017 ticks to sample index
///
/// Per REQ-AI-088-04: samples = ticks × (sample_rate ÷ 28,224,000)
fn ticks_to_samples(ticks: i64, sample_rate: u32) -> usize {
    let samples_per_tick = sample_rate as f64 / TICK_RATE as f64;
    (ticks as f64 * samples_per_tick) as usize
}

/// Extract passage audio samples from full file audio buffer
///
/// **[AIA-PERF-046]** Slices cached audio buffer for passage boundaries
///
/// # Arguments
/// * `file_audio` - Full decoded audio from detect_boundaries_with_audio()
/// * `boundary` - Passage boundary (start/end in ticks)
///
/// # Returns
/// * Vec of audio samples for this passage (interleaved if stereo)
pub fn extract_passage_samples(file_audio: &FileAudioData, boundary: &PassageBoundary) -> Vec<f32> {
    let start_sample = ticks_to_samples(boundary.start_time, file_audio.sample_rate);
    let end_sample = ticks_to_samples(boundary.end_time, file_audio.sample_rate);

    // Account for multichannel interleaving
    let start_idx = start_sample * file_audio.num_channels as usize;
    let end_idx = end_sample * file_audio.num_channels as usize;

    // Clamp to valid range
    let start_idx = start_idx.min(file_audio.samples.len());
    let end_idx = end_idx.min(file_audio.samples.len());

    // Warn if passage extends beyond available audio (indicates incomplete decode)
    if end_idx > file_audio.samples.len() || start_idx >= end_idx {
        warn!(
            "Passage boundary ({}-{} ticks) extends beyond available audio ({} samples). \
            File may have had decode errors. Returning empty buffer.",
            boundary.start_time,
            boundary.end_time,
            file_audio.samples.len()
        );
        return Vec::new();
    }

    file_audio.samples[start_idx..end_idx].to_vec()
}

/// Detect passage boundaries using silence detection
///
/// **[AIA-PERF-045]** Runs on blocking thread pool to avoid blocking async runtime
///
/// # Arguments
/// * `file_path` - Path to audio file
///
/// # Returns
/// * Vec of passage boundaries (start/end times)
#[deprecated(note = "Use detect_boundaries_with_audio() to avoid re-decoding audio for extractors")]
pub async fn detect_boundaries(file_path: &Path) -> Result<Vec<PassageBoundary>> {
    let file_path = file_path.to_path_buf();

    // **[AIA-PERF-045]** Move CPU-bound sync work to blocking thread pool
    // This prevents blocking Tokio async threads and allows other files to run
    tokio::task::spawn_blocking(move || detect_boundaries_sync(&file_path))
        .await
        .context("Boundary detection task panicked")?
}

/// Detect passage boundaries and return decoded audio for extractors
///
/// **[AIA-PERF-046]** Returns decoded audio to avoid re-decoding for Chromaprint/AudioDerived
///
/// # Arguments
/// * `file_path` - Path to audio file
///
/// # Returns
/// * FileAudioData with boundaries and decoded samples
pub async fn detect_boundaries_with_audio(file_path: &Path) -> Result<FileAudioData> {
    let file_path = file_path.to_path_buf();

    // **[AIA-PERF-045]** Move CPU-bound sync work to blocking thread pool
    tokio::task::spawn_blocking(move || detect_boundaries_with_audio_sync(&file_path))
        .await
        .context("Boundary detection task panicked")?
}

/// Synchronous boundary detection implementation (runs on blocking thread pool)
fn detect_boundaries_sync(file_path: &Path) -> Result<Vec<PassageBoundary>> {
    use std::fs::File;
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    info!("Detecting passage boundaries in {:?}", file_path);

    // Open audio file with symphonia
    let file = File::open(file_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = file_path.extension() {
        hint.with_extension(ext.to_str().unwrap_or(""));
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("No valid audio track found"))?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);

    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    // Decode all samples and calculate energy
    let mut all_samples = Vec::new();
    loop {
        match format.next_packet() {
            Ok(packet) if packet.track_id() == track_id => match decoder.decode(&packet) {
                Ok(decoded) => {
                    let samples = extract_samples_f32(&decoded)?;
                    all_samples.extend(samples);
                }
                Err(e) => {
                    debug!("Decode error (continuing): {}", e);
                    continue;
                }
            },
            Ok(_) => continue,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => {
                debug!("Format error (continuing): {}", e);
                continue;
            }
        }
    }

    debug!("Decoded {} samples at {}Hz", all_samples.len(), sample_rate);

    // Calculate RMS energy in windows
    let window_size = (sample_rate as f64 * 0.1) as usize; // 100ms windows
    let mut energy_windows = Vec::new();

    for chunk in all_samples.chunks(window_size) {
        let rms = calculate_rms_energy(chunk);
        energy_windows.push(rms);
    }

    // Detect silence regions (track as sample indices)
    let mut silence_regions = Vec::new();
    let mut silence_start: Option<usize> = None;

    for (i, &energy) in energy_windows.iter().enumerate() {
        let is_silent = energy < SILENCE_THRESHOLD;

        match (silence_start, is_silent) {
            (None, true) => {
                // Start of silence
                silence_start = Some(i);
            }
            (Some(start), false) => {
                // End of silence
                let duration = (i - start) as f64 * 0.1; // windows are 100ms
                if duration >= MIN_SILENCE_DURATION {
                    // Convert window indices to sample indices
                    let start_sample = start * window_size;
                    let end_sample = i * window_size;
                    silence_regions.push((start_sample, end_sample));
                }
                silence_start = None;
            }
            _ => {}
        }
    }

    debug!("Found {} silence regions", silence_regions.len());

    // Convert silence regions to passage boundaries (SPEC017 ticks)
    let mut boundaries = Vec::new();
    let total_samples = all_samples.len();
    let min_passage_samples = seconds_to_samples(MIN_PASSAGE_DURATION, sample_rate);

    if silence_regions.is_empty() {
        // No silence detected - treat entire file as one passage
        boundaries.push(PassageBoundary {
            start_time: 0,
            end_time: samples_to_ticks(total_samples, sample_rate),
            confidence: 0.5, // Low confidence (no clear boundary)
        });
    } else {
        // Create passages between silence regions
        let mut current_start_sample = 0;

        for (silence_start_sample, silence_end_sample) in silence_regions {
            if silence_start_sample - current_start_sample >= min_passage_samples {
                boundaries.push(PassageBoundary {
                    start_time: samples_to_ticks(current_start_sample, sample_rate),
                    end_time: samples_to_ticks(silence_start_sample, sample_rate),
                    confidence: 0.8, // High confidence (clear silence boundary)
                });
                current_start_sample = silence_end_sample;
            }
        }

        // Final passage
        if total_samples - current_start_sample >= min_passage_samples {
            boundaries.push(PassageBoundary {
                start_time: samples_to_ticks(current_start_sample, sample_rate),
                end_time: samples_to_ticks(total_samples, sample_rate),
                confidence: 0.8,
            });
        }
    }

    info!("Detected {} passage boundaries", boundaries.len());

    Ok(boundaries)
}

/// Synchronous boundary detection with audio caching (runs on blocking thread pool)
///
/// **[AIA-PERF-046]** Returns decoded audio to eliminate re-decoding for extractors
fn detect_boundaries_with_audio_sync(file_path: &Path) -> Result<FileAudioData> {
    use std::fs::File;
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    info!(
        "Detecting passage boundaries with audio caching in {:?}",
        file_path
    );

    // Open audio file with symphonia
    let file = File::open(file_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = file_path.extension() {
        hint.with_extension(ext.to_str().unwrap_or(""));
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("No valid audio track found"))?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let num_channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u8)
        .unwrap_or(2);

    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    // **[PHASE2-HYBRID]** Hybrid boundary detection approach (Solution 1C)
    // - Cache audio for files with <10 passages (common case, ~97% of files)
    // - Stream without caching for files with >=10 passages (long albums)
    // This gives best performance for typical files while fixing memory issues for long albums
    let window_size = (sample_rate as f64 * 0.1) as usize; // 100ms windows
    let min_silence_samples = seconds_to_samples(MIN_SILENCE_DURATION, sample_rate);
    let min_passage_samples = seconds_to_samples(MIN_PASSAGE_DURATION, sample_rate);
    let log_interval = sample_rate as usize * 10; // Log every 10 seconds

    let mut all_samples = Vec::new(); // Cache samples during detection
    let mut total_samples = 0usize;
    let mut window_buffer = Vec::new();
    let mut window_count = 0usize;
    let mut silence_regions: Vec<(usize, usize)> = Vec::new();
    let mut silence_start: Option<usize> = None;

    loop {
        match format.next_packet() {
            Ok(packet) if packet.track_id() == track_id => match decoder.decode(&packet) {
                Ok(decoded) => {
                    let samples = extract_samples_f32(&decoded)?;
                    total_samples += samples.len();

                    // Cache samples for potential reuse (if <10 passages)
                    all_samples.extend_from_slice(&samples);

                    // Append to window buffer for boundary detection
                    window_buffer.extend_from_slice(&samples);

                    // Process complete windows
                    while window_buffer.len() >= window_size {
                        // Calculate RMS for this window
                        let rms = calculate_rms_energy(&window_buffer[..window_size]);
                        let is_silent = rms < SILENCE_THRESHOLD;

                        match (silence_start, is_silent) {
                            (None, true) => {
                                // Start of silence
                                silence_start = Some(window_count);
                            }
                            (Some(start), false) => {
                                // End of silence
                                let duration_windows = window_count - start;
                                let duration_samples = duration_windows * window_size;
                                if duration_samples >= min_silence_samples {
                                    let start_sample = start * window_size;
                                    let end_sample = window_count * window_size;
                                    silence_regions.push((start_sample, end_sample));
                                }
                                silence_start = None;
                            }
                            _ => {}
                        }

                        // Remove processed window from buffer
                        window_buffer.drain(..window_size);
                        window_count += 1;

                        // Log progress every 10 seconds
                        let samples_processed = window_count * window_size;
                        if samples_processed / log_interval != (samples_processed - window_size) / log_interval {
                            let duration_sec = samples_processed / sample_rate as usize;
                            debug!("Decoded {} seconds of audio ({} samples, {} silence regions so far)",
                                   duration_sec, samples_processed, silence_regions.len());
                        }
                    }
                }
                Err(e) => {
                    debug!("Decode error (continuing): {}", e);
                    continue;
                }
            },
            Ok(_) => continue,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => {
                debug!("Format error (continuing): {}", e);
                continue;
            }
        }
    }

    // Process final partial window if any
    if !window_buffer.is_empty() {
        let rms = calculate_rms_energy(&window_buffer);
        let is_silent = rms < SILENCE_THRESHOLD;
        if is_silent && silence_start.is_none() {
            let _ = silence_start.insert(window_count);
        } else if !is_silent && silence_start.is_some() {
            // Silence ends in final partial window
            let start = silence_start.unwrap();
            let duration_samples = (window_count - start) * window_size + window_buffer.len();
            if duration_samples >= min_silence_samples {
                let start_sample = start * window_size;
                let end_sample = window_count * window_size + window_buffer.len();
                silence_regions.push((start_sample, end_sample));
            }
        }
        // window_count not used after this point
    }

    // Convert silence regions to passage boundaries (SPEC017 ticks)
    let mut boundaries = Vec::new();

    if silence_regions.is_empty() {
        // No silence detected - treat entire file as one passage
        boundaries.push(PassageBoundary {
            start_time: 0,
            end_time: samples_to_ticks(total_samples, sample_rate),
            confidence: 0.5, // Low confidence (no clear boundary)
        });
    } else {
        // Create passages between silence regions
        let mut current_start_sample = 0;

        for (silence_start_sample, silence_end_sample) in silence_regions {
            if silence_start_sample - current_start_sample >= min_passage_samples {
                boundaries.push(PassageBoundary {
                    start_time: samples_to_ticks(current_start_sample, sample_rate),
                    end_time: samples_to_ticks(silence_start_sample, sample_rate),
                    confidence: 0.8, // High confidence (clear silence boundary)
                });
                current_start_sample = silence_end_sample;
            }
        }

        // Final passage
        if total_samples - current_start_sample >= min_passage_samples {
            boundaries.push(PassageBoundary {
                start_time: samples_to_ticks(current_start_sample, sample_rate),
                end_time: samples_to_ticks(total_samples, sample_rate),
                confidence: 0.8,
            });
        }
    }

    // **[PHASE2-HYBRID + IMPROVEMENT#3]** Memory-based hybrid threshold
    // Cache samples up to 500MB limit, then stream to avoid memory issues
    const MAX_CACHE_BYTES: usize = 500_000_000; // 500MB limit (~5 mins stereo @ 44.1kHz)
    const BYTES_PER_SAMPLE: usize = 4; // f32

    let sample_bytes = all_samples.len() * BYTES_PER_SAMPLE;
    let (final_samples, memory_guard, cache_mode) = if sample_bytes > MAX_CACHE_BYTES {
        // Large file (>500MB samples): Discard cached samples to save memory
        // Extractors that need audio will have to decode on-demand (2-3% slower overall)
        (Vec::new(), None, "streaming (memory limit)")
    } else {
        // Small file (<=500MB samples): Keep cached samples for extractors
        // This is the common case and avoids re-decoding
        let guard = memory_tracker::MemoryGuard::new(all_samples.len(), "boundary_detector cache");
        (all_samples, Some(guard), "cached")
    };

    info!(
        "Detected {} passage boundaries - mode: {} - {} samples ({:.1} minutes) at {}Hz",
        boundaries.len(),
        cache_mode,
        if cache_mode == "cached" { total_samples } else { 0 },
        total_samples as f64 / sample_rate as f64 / 60.0,
        sample_rate
    );

    Ok(FileAudioData {
        boundaries,
        samples: final_samples,
        sample_rate,
        num_channels,
        _memory_guard: memory_guard,
    })
}

/// **[IMPROVEMENT#1]** Detect boundaries from pre-decoded audio samples
///
/// Reuses samples from album matcher to avoid re-decoding. Assumes mono audio.
///
/// # Arguments
/// * `samples` - Pre-decoded audio samples (mono, f32)
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// * FileAudioData with boundaries and samples
pub fn detect_boundaries_from_samples(samples: Vec<f32>, sample_rate: u32) -> Result<FileAudioData> {
    let memory_guard = memory_tracker::MemoryGuard::new(samples.len(), "album_match_fallback reuse");

    info!(
        "Detecting passage boundaries from {} pre-decoded samples at {}Hz",
        samples.len(),
        sample_rate
    );

    let window_size = (sample_rate as f64 * 0.1) as usize; // 100ms windows
    let min_silence_samples = seconds_to_samples(MIN_SILENCE_DURATION, sample_rate);
    let min_passage_samples = seconds_to_samples(MIN_PASSAGE_DURATION, sample_rate);

    let total_samples = samples.len();
    let mut window_buffer = Vec::new();
    let mut window_count = 0usize;
    let mut silence_regions: Vec<(usize, usize)> = Vec::new();
    let mut silence_start: Option<usize> = None;

    // Process samples in windows
    for chunk_start in (0..samples.len()).step_by(window_size) {
        let chunk_end = (chunk_start + window_size).min(samples.len());
        let chunk = &samples[chunk_start..chunk_end];

        if chunk.len() < window_size && chunk_start + chunk.len() < samples.len() {
            // Partial window in the middle - accumulate
            window_buffer.extend_from_slice(chunk);
            continue;
        }

        let window_to_process = if window_buffer.is_empty() {
            chunk
        } else {
            window_buffer.extend_from_slice(chunk);
            &window_buffer[..]
        };

        if window_to_process.len() >= window_size || chunk_end == samples.len() {
            let rms = calculate_rms_energy(window_to_process);
            let is_silent = rms < SILENCE_THRESHOLD;

            match (silence_start, is_silent) {
                (None, true) => {
                    silence_start = Some(window_count);
                }
                (Some(start), false) => {
                    let duration_windows = window_count - start;
                    let duration_samples = duration_windows * window_size;
                    if duration_samples >= min_silence_samples {
                        let start_sample = start * window_size;
                        let end_sample = window_count * window_size;
                        silence_regions.push((start_sample, end_sample));
                    }
                    silence_start = None;
                }
                _ => {}
            }

            window_buffer.clear();
            window_count += 1;
        }
    }

    // Convert silence regions to passage boundaries (SPEC017 ticks)
    let mut boundaries = Vec::new();

    if silence_regions.is_empty() {
        boundaries.push(PassageBoundary {
            start_time: 0,
            end_time: samples_to_ticks(total_samples, sample_rate),
            confidence: 0.5,
        });
    } else {
        let mut current_start_sample = 0;

        for (silence_start_sample, silence_end_sample) in silence_regions {
            if silence_start_sample - current_start_sample >= min_passage_samples {
                boundaries.push(PassageBoundary {
                    start_time: samples_to_ticks(current_start_sample, sample_rate),
                    end_time: samples_to_ticks(silence_start_sample, sample_rate),
                    confidence: 0.8,
                });
                current_start_sample = silence_end_sample;
            }
        }

        if total_samples - current_start_sample >= min_passage_samples {
            boundaries.push(PassageBoundary {
                start_time: samples_to_ticks(current_start_sample, sample_rate),
                end_time: samples_to_ticks(total_samples, sample_rate),
                confidence: 0.8,
            });
        }
    }

    // **[IMPROVEMENT#1+3]** Always keep samples - they're already in RAM from album matcher
    // Memory was already allocated, discarding them gains nothing and would force re-decode
    info!(
        "Detected {} passage boundaries from pre-decoded audio - cached (reused from album matcher) - {} samples ({:.1} minutes) at {}Hz",
        boundaries.len(),
        total_samples,
        total_samples as f64 / sample_rate as f64 / 60.0,
        sample_rate
    );

    Ok(FileAudioData {
        boundaries,
        samples, // Keep all samples - memory cost already paid
        sample_rate,
        num_channels: 1, // Album matcher converts to mono
        _memory_guard: Some(memory_guard),
    })
}

/// Extract samples as f32 from decoded audio buffer
fn extract_samples_f32(buffer: &symphonia::core::audio::AudioBufferRef<'_>) -> Result<Vec<f32>> {
    use symphonia::core::audio::AudioBufferRef;

    match buffer {
        AudioBufferRef::F32(buf) => {
            // Mix all channels to mono
            Ok(mix_to_mono_f32(buf))
        }
        AudioBufferRef::F64(buf) => {
            // Convert f64 to f32 and mix to mono
            Ok(mix_to_mono_f64(buf))
        }
        AudioBufferRef::S16(buf) => {
            // Convert i16 to f32 and mix to mono
            Ok(mix_to_mono_i16(buf))
        }
        AudioBufferRef::S32(buf) => {
            // Convert i32 to f32 and mix to mono
            Ok(mix_to_mono_i32(buf))
        }
        _ => anyhow::bail!("Unsupported audio buffer format"),
    }
}

fn mix_to_mono_f32(buf: &symphonia::core::audio::AudioBuffer<f32>) -> Vec<f32> {
    use symphonia::core::audio::Signal;
    let num_channels = buf.spec().channels.count();
    let num_frames = buf.frames();
    let mut samples = Vec::with_capacity(num_frames);

    for frame in 0..num_frames {
        let mut sum = 0.0f32;
        for ch in 0..num_channels {
            sum += buf.chan(ch)[frame];
        }
        samples.push(sum / num_channels as f32);
    }
    samples
}

fn mix_to_mono_f64(buf: &symphonia::core::audio::AudioBuffer<f64>) -> Vec<f32> {
    use symphonia::core::audio::Signal;
    let num_channels = buf.spec().channels.count();
    let num_frames = buf.frames();
    let mut samples = Vec::with_capacity(num_frames);

    for frame in 0..num_frames {
        let mut sum = 0.0f32;
        for ch in 0..num_channels {
            sum += buf.chan(ch)[frame] as f32;
        }
        samples.push(sum / num_channels as f32);
    }
    samples
}

fn mix_to_mono_i16(buf: &symphonia::core::audio::AudioBuffer<i16>) -> Vec<f32> {
    use symphonia::core::audio::Signal;
    let num_channels = buf.spec().channels.count();
    let num_frames = buf.frames();
    let mut samples = Vec::with_capacity(num_frames);

    for frame in 0..num_frames {
        let mut sum = 0.0f32;
        for ch in 0..num_channels {
            sum += buf.chan(ch)[frame] as f32 / 32768.0;
        }
        samples.push(sum / num_channels as f32);
    }
    samples
}

fn mix_to_mono_i32(buf: &symphonia::core::audio::AudioBuffer<i32>) -> Vec<f32> {
    use symphonia::core::audio::Signal;
    let num_channels = buf.spec().channels.count();
    let num_frames = buf.frames();
    let mut samples = Vec::with_capacity(num_frames);

    for frame in 0..num_frames {
        let mut sum = 0.0f32;
        for ch in 0..num_channels {
            sum += buf.chan(ch)[frame] as f32 / 2147483648.0;
        }
        samples.push(sum / num_channels as f32);
    }
    samples
}

/// Calculate RMS energy
fn calculate_rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_energy() {
        let samples = vec![0.0; 1000];
        assert_eq!(calculate_rms_energy(&samples), 0.0);

        let samples = vec![1.0; 1000];
        assert_eq!(calculate_rms_energy(&samples), 1.0);
    }

    #[test]
    fn test_constants() {
        assert_eq!(SILENCE_THRESHOLD, 0.01);
        assert_eq!(MIN_SILENCE_DURATION, 2.0);
        assert_eq!(MIN_PASSAGE_DURATION, 30.0);
    }
}

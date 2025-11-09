// Passage Boundary Detector
//
// PLAN023: Phase 0 - Simple silence-based boundary detection
// Uses RMS energy analysis to find passage boundaries
//
// SPEC017 Compliance: Converts sample counts to ticks for sample-accurate precision

use super::{PassageBoundary, TICK_RATE};
use anyhow::Result;
use std::path::Path;
use tracing::{debug, info};

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

/// Detect passage boundaries using silence detection
///
/// # Arguments
/// * `file_path` - Path to audio file
///
/// # Returns
/// * Vec of passage boundaries (start/end times)
pub async fn detect_boundaries(file_path: &Path) -> Result<Vec<PassageBoundary>> {
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    use std::fs::File;

    info!("Detecting passage boundaries in {:?}", file_path);

    // Open audio file with symphonia
    let file = File::open(file_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = file_path.extension() {
        hint.with_extension(ext.to_str().unwrap_or(""));
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("No valid audio track found"))?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())?;

    // Decode all samples and calculate energy
    let mut all_samples = Vec::new();
    loop {
        match format.next_packet() {
            Ok(packet) if packet.track_id() == track_id => {
                match decoder.decode(&packet) {
                    Ok(decoded) => {
                        let samples = extract_samples_f32(&decoded)?;
                        all_samples.extend(samples);
                    }
                    Err(e) => {
                        debug!("Decode error (continuing): {}", e);
                        continue;
                    }
                }
            }
            Ok(_) => continue,
            Err(symphonia::core::errors::Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(e) => {
                debug!("Format error (stopping): {}", e);
                break;
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

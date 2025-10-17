//! Audio decoding pipeline using symphonia
//!
//! This module implements parallel audio decoding using symphonia for format support
//! and rubato for sample rate conversion. Decoded PCM data is written to passage buffers.
//!
//! Implements requirements from single-stream-design.md - Phase 1: Core Infrastructure

use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use anyhow::{Context, Result};
use tracing::{debug, info, warn, error, trace};

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;

use rubato::{
    FftFixedInOut, Resampler,
};

use super::buffer::PassageBufferManager;

/// Number of decoder worker threads
/// CO-251: Optimize for resource-constrained devices (Raspberry Pi)
const DEFAULT_DECODER_THREADS: usize = 2;

/// Standard output sample rate - all audio is resampled to this rate
const STANDARD_SAMPLE_RATE: u32 = 44100;

/// Decode priority levels for passages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecodePriority {
    /// Passage needs to be decoded immediately (currently playing)
    Immediate,
    /// Passage is next in queue
    Next,
    /// Passage is being pre-fetched for future playback
    Prefetch,
}

/// Request to decode an audio passage
#[derive(Debug, Clone)]
pub struct DecodeRequest {
    /// Unique identifier for this passage
    pub passage_id: Uuid,
    /// Path to the audio file
    pub file_path: PathBuf,
    /// Start position in samples (0 = beginning of file)
    pub start_sample: u64,
    /// End position in samples (u64::MAX = end of file)
    pub end_sample: u64,
    /// Decode priority
    pub priority: DecodePriority,
}

/// Pool of decoder workers for parallel audio decoding
pub struct DecoderPool {
    /// Channel for sending decode requests to workers
    request_sender: mpsc::Sender<DecodeRequest>,
    /// Handle to join workers on shutdown
    worker_handles: Vec<tokio::task::JoinHandle<()>>,
    /// Reference to the buffer manager
    buffer_manager: Arc<PassageBufferManager>,
}

impl DecoderPool {
    /// Create a new decoder pool with the specified number of workers
    pub fn new(
        buffer_manager: Arc<PassageBufferManager>,
        num_workers: Option<usize>,
    ) -> Self {
        let num_workers = num_workers.unwrap_or(DEFAULT_DECODER_THREADS);
        info!(num_workers, "Initializing decoder pool");

        // Create work queue channel
        let (request_sender, request_receiver) = mpsc::channel::<DecodeRequest>(32);
        let request_receiver = Arc::new(RwLock::new(request_receiver));

        // Spawn worker threads
        let mut worker_handles = Vec::with_capacity(num_workers);
        for worker_id in 0..num_workers {
            let receiver = Arc::clone(&request_receiver);
            let manager = Arc::clone(&buffer_manager);

            let handle = tokio::spawn(async move {
                decoder_worker(worker_id, receiver, manager).await;
            });

            worker_handles.push(handle);
        }

        Self {
            request_sender,
            worker_handles,
            buffer_manager,
        }
    }

    /// Submit a decode request to the pool
    ///
    /// Implements requirement from single-stream-design.md: Background decoding for next 2-3 passages
    pub async fn decode_passage(&self, request: DecodeRequest) -> Result<()> {
        info!(
            passage_id = %request.passage_id,
            file_path = %request.file_path.display(),
            priority = ?request.priority,
            "Submitting decode request"
        );

        // Allocate buffer before sending to decoder
        self.buffer_manager.allocate_buffer(
            request.passage_id,
            request.file_path.clone(),
            request.start_sample,
            request.end_sample,
        ).await?;

        // Send to work queue
        self.request_sender.send(request).await
            .context("Failed to send decode request")?;

        Ok(())
    }

    /// Shutdown the decoder pool
    pub async fn shutdown(self) {
        // Close the channel to signal workers to exit
        drop(self.request_sender);

        // Wait for all workers to finish
        for handle in self.worker_handles {
            let _ = handle.await;
        }

        info!("Decoder pool shut down");
    }
}

/// Worker task that processes decode requests
async fn decoder_worker(
    worker_id: usize,
    receiver: Arc<RwLock<mpsc::Receiver<DecodeRequest>>>,
    buffer_manager: Arc<PassageBufferManager>,
) {
    info!(worker_id, "Decoder worker started");

    loop {
        // Get next request from queue
        let request = {
            let mut rx = receiver.write().await;
            rx.recv().await
        };

        let Some(request) = request else {
            info!(worker_id, "Decoder worker shutting down");
            break;
        };

        debug!(
            worker_id,
            passage_id = %request.passage_id,
            "Processing decode request"
        );

        // Perform the decode operation
        match decode_file(&request, &buffer_manager).await {
            Ok(samples_decoded) => {
                info!(
                    worker_id,
                    passage_id = %request.passage_id,
                    samples_decoded,
                    "Successfully decoded passage"
                );

                // Mark buffer as ready
                if let Err(e) = buffer_manager.mark_ready(&request.passage_id).await {
                    error!(
                        worker_id,
                        passage_id = %request.passage_id,
                        error = %e,
                        "Failed to mark buffer as ready"
                    );
                }
            }
            Err(e) => {
                error!(
                    worker_id,
                    passage_id = %request.passage_id,
                    file_path = %request.file_path.display(),
                    error = %e,
                    "Failed to decode passage"
                );

                // Remove the failed buffer
                buffer_manager.remove_buffer(&request.passage_id).await;
            }
        }
    }
}

/// Decode an audio file and write PCM data to the passage buffer
///
/// Implements requirements:
/// - Uses symphonia for audio decoding (MP3, FLAC, AAC, Vorbis support)
/// - Uses rubato for resampling to standard rate (44.1kHz)
async fn decode_file(
    request: &DecodeRequest,
    buffer_manager: &PassageBufferManager,
) -> Result<u64> {
    // Open the audio file
    let file = File::open(&request.file_path)
        .with_context(|| format!("Failed to open file: {}", request.file_path.display()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Probe the media format
    let mut hint = Hint::new();
    if let Some(ext) = request.file_path.extension() {
        hint.with_extension(ext.to_string_lossy().as_ref());
    }

    let format_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };

    let metadata_opts = MetadataOptions::default();

    let probed = get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .context("Failed to probe audio format")?;

    let mut format_reader = probed.format;

    // Find the first audio track
    let track = format_reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("No audio tracks found"))?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    // Create decoder
    let decoder_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &decoder_opts)
        .context("Failed to create decoder")?;

    // Get audio parameters
    let sample_rate = codec_params.sample_rate
        .ok_or_else(|| anyhow::anyhow!("Unknown sample rate"))?;
    let channels = codec_params.channels
        .ok_or_else(|| anyhow::anyhow!("Unknown channel count"))?
        .count() as u16;

    debug!(
        passage_id = %request.passage_id,
        sample_rate,
        channels,
        "Audio format detected"
    );

    // Set up resampler if needed
    let needs_resampling = sample_rate != STANDARD_SAMPLE_RATE;
    let mut resampler = if needs_resampling {
        info!(
            passage_id = %request.passage_id,
            from_rate = sample_rate,
            to_rate = STANDARD_SAMPLE_RATE,
            "Setting up resampler"
        );

        // Calculate resampling parameters
        let chunk_size = 1024; // Input chunk size

        // Create resampler with proper constructor parameters
        // FftFixedInOut::new(input_rate, output_rate, chunk_size, channels)
        Some(FftFixedInOut::<f32>::new(
            sample_rate as usize,
            STANDARD_SAMPLE_RATE as usize,
            chunk_size,
            channels as usize,
        )?)
    } else {
        None
    };

    // Seek to start position if needed
    if request.start_sample > 0 {
        let start_time_ns = (request.start_sample as f64 / sample_rate as f64 * 1_000_000_000.0) as u64;
        let seek_to = SeekTo::Time {
            time: symphonia::core::units::Time::from(start_time_ns),
            track_id: Some(track_id),
        };

        format_reader.seek(SeekMode::Coarse, seek_to)
            .context("Failed to seek to start position")?;
    }

    // Decode audio packets
    let mut total_samples = 0u64;
    let mut pcm_buffer = Vec::new();

    loop {
        // Check if we've reached the end position
        if request.end_sample != u64::MAX && total_samples >= request.end_sample - request.start_sample {
            debug!(
                passage_id = %request.passage_id,
                total_samples,
                "Reached end position"
            );
            break;
        }

        // Read next packet
        let packet = match format_reader.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(_)) => {
                debug!(
                    passage_id = %request.passage_id,
                    "Reached end of stream"
                );
                break;
            }
            Err(e) => return Err(e.into()),
        };

        // Skip packets from other tracks
        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) => {
                warn!("Decode error in packet, skipping");
                continue;
            }
            Err(e) => return Err(e.into()),
        };

        // Convert to f32 samples
        let samples = convert_audio_buffer_to_f32(decoded)?;

        // Apply resampling if needed
        let output_samples = if let Some(ref mut resampler) = resampler {
            resample_audio(resampler, samples, channels)?
        } else {
            samples
        };

        // Add to PCM buffer
        pcm_buffer.extend_from_slice(&output_samples);
        total_samples += (output_samples.len() / channels as usize) as u64;

        // Periodically write to passage buffer to avoid large memory allocations
        if pcm_buffer.len() >= 44100 * channels as usize * 2 { // ~2 seconds of audio
            write_to_buffer(buffer_manager, &request.passage_id, &pcm_buffer).await?;
            pcm_buffer.clear();
        }
    }

    // Write any remaining samples
    if !pcm_buffer.is_empty() {
        write_to_buffer(buffer_manager, &request.passage_id, &pcm_buffer).await?;
    }

    Ok(total_samples)
}

/// Convert symphonia audio buffer to f32 samples
fn convert_audio_buffer_to_f32(buffer: AudioBufferRef) -> Result<Vec<f32>> {
    match buffer {
        AudioBufferRef::F32(buf) => {
            // Already f32, just copy - interleave channels
            let mut samples = Vec::with_capacity(buf.frames() * buf.spec().channels.count());
            for frame in 0..buf.frames() {
                for ch in 0..buf.spec().channels.count() {
                    samples.push(buf.chan(ch)[frame]);
                }
            }
            Ok(samples)
        }
        AudioBufferRef::S16(buf) => {
            // Convert i16 to f32 [-1.0, 1.0] - interleave channels
            let mut samples = Vec::with_capacity(buf.frames() * buf.spec().channels.count());
            for frame in 0..buf.frames() {
                for ch in 0..buf.spec().channels.count() {
                    let s = buf.chan(ch)[frame];
                    samples.push(s as f32 / i16::MAX as f32);
                }
            }
            Ok(samples)
        }
        AudioBufferRef::S32(buf) => {
            // Convert i32 to f32 [-1.0, 1.0] - interleave channels
            let mut samples = Vec::with_capacity(buf.frames() * buf.spec().channels.count());
            for frame in 0..buf.frames() {
                for ch in 0..buf.spec().channels.count() {
                    let s = buf.chan(ch)[frame];
                    samples.push(s as f32 / i32::MAX as f32);
                }
            }
            Ok(samples)
        }
        _ => Err(anyhow::anyhow!("Unsupported audio format")),
    }
}

/// Resample audio using rubato
fn resample_audio(
    resampler: &mut FftFixedInOut<f32>,
    input: Vec<f32>,
    channels: u16,
) -> Result<Vec<f32>> {
    // Deinterleave samples for rubato
    let mut channel_data: Vec<Vec<f32>> = vec![Vec::new(); channels as usize];
    for (i, sample) in input.iter().enumerate() {
        channel_data[i % channels as usize].push(*sample);
    }

    // Process resampling
    let output = resampler.process(&channel_data, None)?;

    // Interleave back to stereo
    let mut result = Vec::with_capacity(output[0].len() * channels as usize);
    for i in 0..output[0].len() {
        for ch in 0..channels as usize {
            result.push(output[ch][i]);
        }
    }

    Ok(result)
}

/// Write PCM data to the passage buffer
async fn write_to_buffer(
    buffer_manager: &PassageBufferManager,
    passage_id: &Uuid,
    pcm_data: &[f32],
) -> Result<()> {
    if let Some(mut buffers) = buffer_manager.get_buffer_mut(passage_id).await {
        if let Some(buffer) = buffers.get_mut(passage_id) {
            buffer.pcm_data.extend_from_slice(pcm_data);
            trace!(
                passage_id = %passage_id,
                samples_added = pcm_data.len() / 2,
                total_samples = buffer.pcm_data.len() / 2,
                "Added PCM data to buffer"
            );
            Ok(())
        } else {
            Err(anyhow::anyhow!("Buffer not found for passage {}", passage_id))
        }
    } else {
        Err(anyhow::anyhow!("Buffer not found for passage {}", passage_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_s16_to_f32() {
        let samples = vec![i16::MAX, 0, i16::MIN];
        let converted = samples
            .iter()
            .map(|&s| s as f32 / i16::MAX as f32)
            .collect::<Vec<_>>();

        assert!((converted[0] - 1.0).abs() < 0.001);
        assert_eq!(converted[1], 0.0);
        assert!((converted[2] + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_decode_priority_ordering() {
        assert!(DecodePriority::Immediate > DecodePriority::Next);
        assert!(DecodePriority::Next > DecodePriority::Prefetch);
    }
}
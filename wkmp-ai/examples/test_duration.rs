//! Test symphonia duration calculation for VBR MP3s
//!
//! Usage: cargo run --example test_duration -- "path/to/file.mp3"

use anyhow::{Context, Result};
use std::path::Path;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::audio::{AudioBufferRef, Signal};
use std::time::Instant;

/// Get duration using symphonia's metadata (without full decode)
fn get_metadata_duration(file_path: &Path) -> Result<Option<f64>> {
    let file = std::fs::File::open(file_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())?;

    let format = probed.format;

    // Find first audio track
    if let Some(track) = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL) {
        let sample_rate = track.codec_params.sample_rate;
        let n_frames = track.codec_params.n_frames;
        let time_base = track.codec_params.time_base;

        println!("Track info:");
        println!("  Sample rate: {:?}", sample_rate);
        println!("  N frames: {:?}", n_frames);
        println!("  Time base: {:?}", time_base);

        // Try to calculate duration from metadata
        if let (Some(sr), Some(frames)) = (sample_rate, n_frames) {
            return Ok(Some(frames as f64 / sr as f64));
        }
    }

    Ok(None)
}

/// Get duration by full decode (accurate but slow)
fn get_full_decode_duration(file_path: &Path) -> Result<f64> {
    let file = std::fs::File::open(file_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())?;

    let mut format = probed.format;

    let track = format.tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("No audio track")?
        .clone();

    let sample_rate = track.codec_params.sample_rate.context("No sample rate")?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())?;

    let mut total_samples: u64 = 0;
    let mut packets_decoded: u64 = 0;
    let mut errors: u64 = 0;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    println!("  EOF reached after {} packets, {} errors", packets_decoded, errors);
                    break;
                },
            Err(e) => {
                errors += 1;
                if errors < 5 {
                    println!("  Packet error {}: {}", errors, e);
                }
                continue; // Try next packet instead of breaking
            },
        };

        if packet.track_id() != track.id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                packets_decoded += 1;
                total_samples += match &decoded {
                    AudioBufferRef::F32(buf) => buf.frames() as u64,
                    AudioBufferRef::S16(buf) => buf.frames() as u64,
                    AudioBufferRef::S32(buf) => buf.frames() as u64,
                    _ => 0,
                };
            }
            Err(e) => {
                errors += 1;
                if errors < 5 {
                    println!("  Decode error {}: {}", errors, e);
                }
            }
        }
    }

    println!("  Total samples: {}, packets: {}, errors: {}", total_samples, packets_decoded, errors);
    Ok(total_samples as f64 / sample_rate as f64)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let file_path = args.get(1)
        .map(|s| s.as_str())
        .unwrap_or(r"C:\Users\Mango Cat\Music\AC-DC\Back in Black\Disc 1 - 1 - Hells Bells.mp3");

    let path = Path::new(file_path);
    println!("Testing: {}", path.display());
    println!();

    // File info
    let file_size = std::fs::metadata(path)?.len();
    println!("File size: {:.2} MB", file_size as f64 / 1024.0 / 1024.0);
    println!();

    // Method 1: Metadata duration
    println!("Method 1: Symphonia metadata");
    let start = Instant::now();
    match get_metadata_duration(path) {
        Ok(Some(dur)) => println!("  Duration: {:.1}s ({:.1} min)", dur, dur / 60.0),
        Ok(None) => println!("  Duration: NOT AVAILABLE in metadata"),
        Err(e) => println!("  Error: {}", e),
    }
    println!("  Time: {:?}", start.elapsed());
    println!();

    // Method 2: Full decode
    println!("Method 2: Full decode (accurate)");
    let start = Instant::now();
    match get_full_decode_duration(path) {
        Ok(dur) => println!("  Duration: {:.1}s ({:.1} min)", dur, dur / 60.0),
        Err(e) => println!("  Error: {}", e),
    }
    println!("  Time: {:?}", start.elapsed());

    Ok(())
}

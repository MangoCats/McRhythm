//! Audio subsystem for wkmp-ap
//!
//! This module provides audio decoding, resampling, and output functionality
//! for the single-stream audio pipeline.
//!
//! **Traceability:**
//! - [SSD-DEC-010] Decoder with decode-and-skip approach
//! - [SSD-FBUF-020] Resampling to 44.1kHz standard rate
//! - [SSD-OUT-010] Audio device interface using cpal
//!
//! **Architecture:** Single-stream design (see single-stream-design.md)
//! - Decode audio files to PCM buffers
//! - Resample to standard 44.1kHz sample rate
//! - Output through cpal cross-platform audio

pub mod types;
pub mod decoder;
pub mod resampler;
pub mod output;

// Re-exports for external use (tests, other modules)
pub use decoder::SimpleDecoder;
pub use output::AudioOutput;
pub use resampler::Resampler;
pub use types::{AudioFrame, PassageBuffer};

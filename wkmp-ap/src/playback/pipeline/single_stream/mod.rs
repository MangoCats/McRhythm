//! Single-stream audio playback pipeline
//!
//! This module implements the single-stream architecture for audio playback
//! with sample-accurate crossfading, as specified in single-stream-design.md.
//!
//! The architecture consists of:
//! - PassageBuffer: PCM buffer management
//! - DecoderPool: Parallel audio decoding with symphonia
//! - CrossfadeMixer: Sample-accurate mixing with fade curves
//! - AudioOutput: Cross-platform audio output using cpal

pub mod buffer;
pub mod decoder;
pub mod mixer;
pub mod output;
pub mod output_simple;

// Re-export key types
pub use buffer::{
    PassageBuffer,
    PassageBufferManager,
    BufferStatus,
    FadeCurve,
    MemoryStats,
};

pub use decoder::{
    DecoderPool,
    DecodeRequest,
    DecodePriority,
};

pub use mixer::{
    CrossfadeMixer,
    CrossfadeState,
    CrossfadePoints,
    calculate_fade_gain,
};
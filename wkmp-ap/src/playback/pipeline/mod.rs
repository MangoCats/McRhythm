//! Audio playback pipeline implementations
//!
//! This module contains different pipeline architectures for audio playback.
//! Currently implementing the single-stream architecture for sample-accurate crossfading.

pub mod single_stream;

// Re-export the current implementation
pub use single_stream::{
    PassageBufferManager,
    DecoderPool,
    DecodeRequest,
    DecodePriority,
    CrossfadeMixer,
};
//! Playback-related type definitions
//!
//! Supporting types for playback state and decoder/buffer lifecycle.

use serde::{Deserialize, Serialize};

/// Decoder state enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum DecoderState {
    /// Waiting for work
    Idle,
    /// Actively decoding audio
    Decoding,
    /// Paused (buffer full or lower priority)
    Paused,
}

impl std::fmt::Display for DecoderState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecoderState::Idle => write!(f, "Idle"),
            DecoderState::Decoding => write!(f, "Decoding"),
            DecoderState::Paused => write!(f, "Paused"),
        }
    }
}

/// Fade processing stage enumeration
/// **[DBD-FADE-010]** through **[DBD-FADE-060]**
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum FadeStage {
    /// Before passage start (discarding samples) **[DBD-FADE-020]**
    PreStart,
    /// Applying fade-in curve **[DBD-FADE-030]**
    FadeIn,
    /// No fade applied (passthrough) **[DBD-FADE-040]**
    Body,
    /// Applying fade-out curve **[DBD-FADE-050]**
    FadeOut,
    /// After passage end (decode complete) **[DBD-FADE-060]**
    PostEnd,
}

impl std::fmt::Display for FadeStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FadeStage::PreStart => write!(f, "PreStart"),
            FadeStage::FadeIn => write!(f, "FadeIn"),
            FadeStage::Body => write!(f, "Body"),
            FadeStage::FadeOut => write!(f, "FadeOut"),
            FadeStage::PostEnd => write!(f, "PostEnd"),
        }
    }
}

/// Playback state enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Playing,
    Paused,
}

impl std::fmt::Display for PlaybackState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaybackState::Playing => write!(f, "playing"),
            PlaybackState::Paused => write!(f, "paused"),
        }
    }
}

/// Buffer status for passage decode/playback lifecycle
///
/// Per SPEC016 Buffers:
/// - DBD-BUF-020: Empty on start
/// - DBD-BUF-030: Mixer can't read empty buffer
/// - DBD-BUF-040: Returns last sample if empty
/// - DBD-BUF-050: Decoder pauses when nearly full
/// - DBD-BUF-060: Informs queue on completion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum BufferStatus {
    /// Buffer currently being populated from audio file
    Decoding,
    /// Buffer fully decoded and ready for playback
    Ready,
    /// Buffer currently being read for audio output
    Playing,
    /// Buffer playback completed
    Exhausted,
}

impl std::fmt::Display for BufferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferStatus::Decoding => write!(f, "Decoding"),
            BufferStatus::Ready => write!(f, "Ready"),
            BufferStatus::Playing => write!(f, "Playing"),
            BufferStatus::Exhausted => write!(f, "Exhausted"),
        }
    }
}

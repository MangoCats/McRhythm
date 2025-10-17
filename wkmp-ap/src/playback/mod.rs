//! Audio playback subsystem
//!
//! This module implements the core playback functionality for the audio player,
//! including the playback engine, pipeline management, and queue management.

pub mod pipeline;
pub mod engine;

// Re-export key types

pub use engine::PlaybackEngine;
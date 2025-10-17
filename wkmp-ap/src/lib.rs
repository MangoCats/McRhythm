//! WKMP Audio Player Library
//!
//! Exposes public API for integration testing and external use

pub mod api;
pub mod playback;

// Re-export commonly used types
pub use playback::engine::{
    PlaybackEngine,
    PlaybackState,
    EnqueueRequest,
};
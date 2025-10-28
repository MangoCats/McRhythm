//! # WKMP Audio Player Library (wkmp-ap)
//!
//! Core playback engine with sample-accurate crossfading.
//!
//! **Purpose:** Decode audio files, manage playback queue, perform sample-accurate
//! crossfading, and provide HTTP/SSE control interface.
//!
//! **Architecture:** Single-stream audio pipeline using symphonia + rubato + cpal
//!
//! **Traceability:** Implements requirements from single-stream-design.md,
//! api_design.md, and crossfade.md

pub mod api;
pub mod audio;
pub mod config;
pub mod db;
pub mod error;
pub mod playback;
pub mod state;
pub mod tuning;

pub use error::{Error, Result};
pub use state::SharedState;

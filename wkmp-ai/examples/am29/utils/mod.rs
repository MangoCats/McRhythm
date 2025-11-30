//! # Utilities Module
//!
//! Utility functions for audio, fingerprinting, timing, statistics, and early-exit coordination.
//!
//! ## Submodules
//! - `audio`: Audio decoding (symphonia integration)
//! - `fingerprint`: AcoustID fingerprinting
//! - `timing`: Timing, heartbeat, stagger logic
//! - `query_stats`: Query statistics tracking and heartbeat logging
//! - `early_exit`: Early-exit coordination for parallel processing

pub(crate) mod audio;
pub(crate) mod early_exit;
pub(crate) mod fingerprint;
pub(crate) mod query_stats;
pub(crate) mod timing;

// Re-export key items
pub use audio::*;
pub use early_exit::*;
pub use fingerprint::*;
pub use query_stats::*;
pub use timing::*;

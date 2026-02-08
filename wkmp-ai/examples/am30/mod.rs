//! # Album Matcher 28 - Modular Implementation
//!
//! Refactored version of album_matcher_28.rs with clear module boundaries.
//!
//! This module provides the same functionality as the monolithic album_matcher_28.rs
//! but organized into logical components for better maintainability and testability.

pub mod main;

pub(crate) mod types;
pub(crate) mod constants;
pub(crate) mod silence_detection;
pub(crate) mod musicbrainz;
pub(crate) mod metadata;
pub(crate) mod helpers;
pub(crate) mod stages;
pub(crate) mod matching;
pub(crate) mod utils;
pub(crate) mod orchestration;
pub(crate) mod single_track_discriminator;

// Re-export main for convenience
pub use main::main;

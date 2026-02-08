//! # MusicBrainz Module
//!
//! MusicBrainz API client with rate limiting and file-based caching.
//!
//! ## Submodules
//! - `api`: HTTP client with rate limiting
//! - `cache`: File-based cache (3 modes: Disabled, ReadWrite, ReadOnly)
//! - `types`: MusicBrainz-specific types

pub(crate) mod api;
pub(crate) mod cache;
pub(crate) mod types;

// Re-export key items
pub use api::*;
pub use cache::*;
pub use types::*;

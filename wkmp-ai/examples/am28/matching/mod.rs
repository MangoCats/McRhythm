//! # Matching Module
//!
//! Candidate testing, edition selection, validation, and MBID selection logic.
//!
//! ## Submodules
//! - `candidate`: Match quality calculation
//! - `edition`: Edition processing and winner selection
//! - `validation`: Artist/album name validation
//! - `mbid`: MBID selection using metadata prioritization

pub(crate) mod candidate;
pub(crate) mod edition;
pub(crate) mod validation;
pub(crate) mod mbid;

// Re-export key items
pub use candidate::*;
pub use edition::*;
pub use validation::*;
pub use mbid::*;

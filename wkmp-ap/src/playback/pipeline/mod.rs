//! Audio pipeline components
//!
//! Single-stream pipeline implementation.
//!
//! **Traceability:** Single-Stream Design - Component Structure

pub mod mixer;
pub mod timing;

pub use mixer::CrossfadeMixer;
pub use timing::{CrossfadeTiming, PassageTiming};

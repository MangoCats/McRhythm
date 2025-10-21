//! Audio pipeline components
//!
//! Single-stream pipeline implementation.
//!
//! **Traceability:** Single-Stream Design - Component Structure

pub mod mixer;
pub mod timing;

// Re-exports for external use (tests, other modules)
pub use mixer::CrossfadeMixer;


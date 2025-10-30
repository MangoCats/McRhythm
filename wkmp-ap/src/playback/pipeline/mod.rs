//! Audio pipeline components
//!
//! Single-stream pipeline implementation.
//!
//! **Traceability:** Single-Stream Design - Component Structure

pub mod decoder_chain;
pub mod fader;
pub mod mixer;
pub mod timing;

// Re-exports for external use (tests, other modules)
pub use decoder_chain::{ChunkProcessResult, DecoderChain};
pub use fader::Fader;


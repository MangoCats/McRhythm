//! Playback engine and audio pipeline
//!
//! Core playback logic including buffer management, decoding, mixing, and output.
//!
//! **Traceability:** Single-Stream Design - Complete architecture

pub mod buffer_events;
pub mod buffer_manager;
pub mod decoder_pool;
pub mod serial_decoder;
pub mod engine;
pub mod events;
pub mod pipeline;
pub mod playout_ring_buffer;
pub mod queue_manager;
pub mod ring_buffer;
pub mod song_timeline;
pub mod types;

// Re-exports for external use (tests, other modules)
pub use buffer_manager::BufferManager;
pub use serial_decoder::SerialDecoder;
pub use types::DecodePriority;

// Export from pipeline submodule
pub use pipeline::CrossfadeMixer;


//! Playback engine and audio pipeline
//!
//! Core playback logic including buffer management, decoding, mixing, and output.
//!
//! **Traceability:** Single-Stream Design - Complete architecture

pub mod buffer_manager;
pub mod decoder_pool;
pub mod engine;
pub mod pipeline;
pub mod queue_manager;
pub mod ring_buffer;
pub mod types;

pub use buffer_manager::BufferManager;
pub use decoder_pool::DecoderPool;
pub use engine::PlaybackEngine;
pub use queue_manager::{QueueEntry, QueueManager};
pub use types::DecodePriority;

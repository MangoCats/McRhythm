//! Playback engine and audio pipeline
//!
//! Core playback logic including buffer management, decoding, mixing, and output.
//!
//! **Traceability:** Single-Stream Design - Complete architecture

pub mod buffer_events;
pub mod buffer_manager;
pub mod callback_monitor; // Audio callback timing instrumentation
// Removed: decoder_pool (obsolete - replaced by DecoderWorker)
pub mod decoder_worker; // New single-threaded worker
// Removed: serial_decoder (obsolete - replaced by DecoderWorker)
pub mod diagnostics; // [PHASE1-INTEGRITY] Pipeline validation
pub mod validation_service; // [ARCH-AUTO-VAL-001] Automatic validation service
pub mod engine;
pub mod events;
pub mod mixer; // SPEC016-compliant mixer (reads pre-faded samples)
pub mod pipeline;
pub mod playout_ring_buffer;
pub mod queue_manager;
pub mod ring_buffer;
pub mod song_timeline;
pub mod types;

// Re-exports for external use (tests, other modules)
#[allow(unused_imports)] // Used by tests
pub use buffer_manager::BufferManager;
#[allow(unused_imports)] // Used by tests
pub use decoder_worker::DecoderWorker;
pub use diagnostics::{PassageMetrics, PipelineMetrics};

// Export from pipeline submodule


//! Utility modules for wkmp-ai

pub mod audio_decoder;
pub mod db_retry;
pub mod memory_monitor; // PLAN029 Task 2.2: Memory monitoring
pub mod pool_monitor;
pub mod string_similarity; // PLAN026: String similarity for single-song matching

pub use audio_decoder::{decode_audio_file, DecodedAudio};
pub use db_retry::retry_on_lock;
pub use memory_monitor::{MemoryMonitor, MemoryStatus};
pub use pool_monitor::{begin_monitored, MonitoredTransaction};
pub use string_similarity::{
    best_similarity, jaro_winkler_similarity, normalize_for_comparison, verify_match,
    ARTIST_SIMILARITY_THRESHOLD, TITLE_SIMILARITY_THRESHOLD,
};

//! Test Helper Utilities
//!
//! Shared utilities for testing wkmp-ai

pub mod audio_generator;
pub mod db_utils;
pub mod log_capture;

// Re-export commonly used items
pub use audio_generator::{generate_test_library, generate_test_wav, AudioConfig};
pub use db_utils::{
    assert_has_column, assert_no_column, create_test_db, create_test_orchestrator,
    get_table_columns, has_column, seed_test_files,
};
pub use log_capture::{init_test_logging, LogCapture};

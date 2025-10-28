//! Test helper modules for WKMP Audio Player integration tests
//!
//! Provides reusable test infrastructure components:
//! - TestServer: Start/stop wkmp-ap programmatically
//! - AudioCapture: Record audio output for analysis
//! - AudioAnalysis: FFT, RMS, phase analysis functions
//! - AudioGenerator: Generate deterministic test audio files
//! - ErrorInjection: Error injection utilities for Phase 7 error handling tests

pub mod test_server;
pub mod audio_capture;
pub mod audio_analysis;
pub mod audio_generator;
pub mod error_injection;

// Re-export commonly used types
pub use test_server::{TestServer, PassageBuilder, PassageRequest};
pub use audio_capture::AudioCapture;
pub use audio_analysis::{
    AudioAnalysisReport, ClickEvent, PopEvent, RmsContinuityReport,
    PhaseContinuityReport, detect_clicks, detect_pops,
    verify_rms_continuity, verify_phase_continuity, measure_startup_latency,
};
pub use audio_generator::{
    generate_silent_wav, generate_sine_wav, generate_chirp_wav,
    calculate_sample_count, calculate_frame_count,
};
pub use error_injection::{
    ErrorInjectionBuilder, panic_injection, event_verification, logging_verification,
};

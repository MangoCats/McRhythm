//! Test helper modules for WKMP Audio Player integration tests
//!
//! Provides reusable test infrastructure components:
//! - TestServer: Start/stop wkmp-ap programmatically
//! - AudioCapture: Record audio output for analysis
//! - AudioAnalysis: FFT, RMS, phase analysis functions

pub mod test_server;
pub mod audio_capture;
pub mod audio_analysis;

// Re-export commonly used types
pub use test_server::{TestServer, PassageBuilder, PassageRequest};
pub use audio_capture::AudioCapture;
pub use audio_analysis::{
    AudioAnalysisReport, ClickEvent, PopEvent, RmsContinuityReport,
    PhaseContinuityReport, detect_clicks, detect_pops,
    verify_rms_continuity, verify_phase_continuity, measure_startup_latency,
};

//! Import and amplitude analysis parameters
//!
//! **[AIA-ASYNC-020]** Concurrent file processing configuration
//! **[AIA-PERF-010]** Performance tuning parameters

use serde::{Deserialize, Serialize};

/// **[AIA-ASYNC-020]** Import workflow parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportParameters {
    /// Scan subdirectories recursively (default: true)
    #[serde(default = "default_scan_subdirectories")]
    pub scan_subdirectories: bool,

    /// File extensions to import (default: common audio formats)
    #[serde(default = "default_file_extensions")]
    pub file_extensions: Vec<String>,

    /// Skip hidden files (starting with .) (default: true)
    #[serde(default = "default_skip_hidden")]
    pub skip_hidden_files: bool,

    /// Number of parallel workers (default: 4)
    #[serde(default = "default_parallelism")]
    pub parallelism: usize,

    /// Amplitude analysis parameters
    #[serde(default)]
    pub amplitude: AmplitudeParameters,
}

/// Amplitude analysis parameters (SPEC025)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmplitudeParameters {
    /// RMS window size in milliseconds (default: 100ms)
    #[serde(default = "default_rms_window_ms")]
    pub rms_window_ms: u32,

    /// Lead-in threshold in dB (default: -12.0 dB)
    #[serde(default = "default_lead_in_threshold_db")]
    pub lead_in_threshold_db: f64,

    /// Lead-out threshold in dB (default: -12.0 dB)
    #[serde(default = "default_lead_out_threshold_db")]
    pub lead_out_threshold_db: f64,

    /// Quick ramp threshold (0.0 - 1.0) (default: 0.75)
    #[serde(default = "default_quick_ramp_threshold")]
    pub quick_ramp_threshold: f64,

    /// Quick ramp duration in seconds (default: 1.0s)
    #[serde(default = "default_quick_ramp_duration_s")]
    pub quick_ramp_duration_s: f64,

    /// Maximum lead-in duration in seconds (default: 5.0s)
    #[serde(default = "default_max_lead_in_s")]
    pub max_lead_in_duration_s: f64,

    /// Maximum lead-out duration in seconds (default: 5.0s)
    #[serde(default = "default_max_lead_out_s")]
    pub max_lead_out_duration_s: f64,

    /// Apply A-weighting to RMS calculations (default: true)
    #[serde(default = "default_apply_a_weighting")]
    pub apply_a_weighting: bool,
}

// Default value functions
fn default_scan_subdirectories() -> bool {
    true
}

fn default_file_extensions() -> Vec<String> {
    vec![
        ".mp3".to_string(),
        ".flac".to_string(),
        ".ogg".to_string(),
        ".m4a".to_string(),
        ".wav".to_string(),
        ".opus".to_string(),
        ".aac".to_string(),
    ]
}

fn default_skip_hidden() -> bool {
    true
}

fn default_parallelism() -> usize {
    4
}

fn default_rms_window_ms() -> u32 {
    100
}

fn default_lead_in_threshold_db() -> f64 {
    -12.0
}

fn default_lead_out_threshold_db() -> f64 {
    -12.0
}

fn default_quick_ramp_threshold() -> f64 {
    0.75
}

fn default_quick_ramp_duration_s() -> f64 {
    1.0
}

fn default_max_lead_in_s() -> f64 {
    5.0
}

fn default_max_lead_out_s() -> f64 {
    5.0
}

fn default_apply_a_weighting() -> bool {
    true
}

impl Default for ImportParameters {
    fn default() -> Self {
        Self {
            scan_subdirectories: default_scan_subdirectories(),
            file_extensions: default_file_extensions(),
            skip_hidden_files: default_skip_hidden(),
            parallelism: default_parallelism(),
            amplitude: AmplitudeParameters::default(),
        }
    }
}

impl Default for AmplitudeParameters {
    fn default() -> Self {
        Self {
            rms_window_ms: default_rms_window_ms(),
            lead_in_threshold_db: default_lead_in_threshold_db(),
            lead_out_threshold_db: default_lead_out_threshold_db(),
            quick_ramp_threshold: default_quick_ramp_threshold(),
            quick_ramp_duration_s: default_quick_ramp_duration_s(),
            max_lead_in_duration_s: default_max_lead_in_s(),
            max_lead_out_duration_s: default_max_lead_out_s(),
            apply_a_weighting: default_apply_a_weighting(),
        }
    }
}

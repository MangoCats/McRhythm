//! Amplitude analysis data structures (SPEC025)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::parameters::AmplitudeParameters;

/// Amplitude analysis request (POST /analyze/amplitude)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmplitudeAnalysisRequest {
    /// File path to analyze
    pub file_path: String,

    /// Start time in seconds (default: 0.0 = beginning)
    #[serde(default)]
    pub start_time: f64,

    /// End time in seconds (default: None = end of file)
    pub end_time: Option<f64>,

    /// Amplitude analysis parameters (optional, uses global defaults if omitted)
    #[serde(default)]
    pub parameters: Option<AmplitudeParameters>,
}

/// Amplitude analysis response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmplitudeAnalysisResponse {
    /// File path analyzed
    pub file_path: String,

    /// Peak RMS value (0.0 - 1.0+)
    pub peak_rms: f64,

    /// Lead-in duration in seconds
    pub lead_in_duration: f64,

    /// Lead-out duration in seconds
    pub lead_out_duration: f64,

    /// Quick ramp up detected at start
    pub quick_ramp_up: bool,

    /// Quick ramp down detected at end
    pub quick_ramp_down: bool,

    /// RMS profile (array of RMS values at window intervals)
    pub rms_profile: Vec<f64>,

    /// Analysis timestamp
    pub analyzed_at: DateTime<Utc>,
}

/// Amplitude envelope data structure (internal use)
#[derive(Debug, Clone)]
pub struct AmplitudeProfile {
    /// RMS values at window intervals
    pub rms_values: Vec<f64>,

    /// Window size in samples
    pub window_size_samples: usize,

    /// Sample rate
    pub sample_rate: u32,

    /// Peak RMS value
    pub peak_rms: f64,

    /// Lead-in duration in seconds
    pub lead_in_duration: f64,

    /// Lead-out duration in seconds
    pub lead_out_duration: f64,

    /// Quick ramp flags
    pub quick_ramp_up: bool,
    pub quick_ramp_down: bool,
}

impl AmplitudeProfile {
    /// Convert to API response
    pub fn to_response(&self, file_path: String) -> AmplitudeAnalysisResponse {
        AmplitudeAnalysisResponse {
            file_path,
            peak_rms: self.peak_rms,
            lead_in_duration: self.lead_in_duration,
            lead_out_duration: self.lead_out_duration,
            quick_ramp_up: self.quick_ramp_up,
            quick_ramp_down: self.quick_ramp_down,
            rms_profile: self.rms_values.clone(),
            analyzed_at: Utc::now(),
        }
    }
}

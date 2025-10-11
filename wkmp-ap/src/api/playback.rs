//! Playback API request/response types

use serde::{Deserialize, Serialize};

/// Enqueue request body
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnqueueRequest {
    /// File path relative to root folder (required)
    pub file_path: String,

    /// Passage GUID for defaults and song identification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passage_guid: Option<String>,

    /// Timing overrides (all optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time_ms: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time_ms: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lead_in_point_ms: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lead_out_point_ms: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fade_in_point_ms: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fade_out_point_ms: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fade_in_curve: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fade_out_curve: Option<String>,
}

/// Enqueue response
#[derive(Debug, Clone, Serialize)]
pub struct EnqueueResponse {
    /// UUID of created queue entry
    pub guid: String,

    /// Position in queue
    pub position: usize,
}

/// Resolved timing parameters after applying precedence rules
#[derive(Debug, Clone)]
pub struct ResolvedTiming {
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub lead_in_point_ms: i64,
    pub lead_out_point_ms: i64,
    pub fade_in_point_ms: i64,
    pub fade_out_point_ms: i64,
    pub fade_in_curve: String,
    pub fade_out_curve: String,
}

impl ResolvedTiming {
    /// Create default timing for a file
    pub fn from_file_duration(duration_ms: i64) -> Self {
        Self {
            start_time_ms: 0,
            end_time_ms: duration_ms,
            lead_in_point_ms: 0,
            lead_out_point_ms: duration_ms,
            fade_in_point_ms: 0,
            fade_out_point_ms: duration_ms,
            fade_in_curve: "exponential".to_string(),
            fade_out_curve: "logarithmic".to_string(),
        }
    }
}

/// Volume control request
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VolumeRequest {
    /// Volume level (0-100 for user-facing, converted to 0.0-1.0 internally)
    pub volume: i32,
}

/// Seek request
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SeekRequest {
    /// Position in milliseconds
    pub position_ms: i64,
}

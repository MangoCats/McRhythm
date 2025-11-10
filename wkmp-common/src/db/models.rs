//! Database models
//!
//! Shared database models used across all WKMP microservices.
//! These correspond to tables in the shared wkmp.db SQLite database.

use serde::{Deserialize, Serialize};

#[cfg(feature = "sqlx")]
use sqlx::FromRow;

/// Application setting key-value pair
///
/// Stored in the `settings` table. Used for configuration values
/// that need to be persisted and shared across microservices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct Setting {
    /// Setting key (e.g., "api_shared_secret", "volume_level")
    pub key: String,
    /// Setting value as string (parsed by application)
    pub value: String,
}

/// Microservice configuration
///
/// Stored in the `module_config` table. Defines host/port and
/// enabled status for each WKMP microservice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct ModuleConfig {
    /// Module name (e.g., "wkmp-ap", "wkmp-ui", "wkmp-pd")
    pub module_name: String,
    /// Host address (e.g., "127.0.0.1", "localhost")
    pub host: String,
    /// Port number (e.g., 5720, 5721, 5722)
    pub port: i64,
    /// Whether module is enabled
    pub enabled: bool,
}

/// Playback queue entry
///
/// Stored in the `queue` table. Represents a passage scheduled for playback
/// with all timing and crossfade parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct QueueEntry {
    /// Unique identifier (UUID)
    pub guid: String,
    /// Path to audio file
    pub file_path: String,
    /// Passage UUID (if playing specific passage)
    pub passage_guid: Option<String>,
    /// Playback order (lower plays first)
    pub play_order: i64,
    /// Start time in milliseconds
    pub start_time_ms: Option<i64>,
    /// End time in milliseconds
    pub end_time_ms: Option<i64>,
    /// Lead-in point in milliseconds
    pub lead_in_point_ms: Option<i64>,
    /// Lead-out point in milliseconds
    pub lead_out_point_ms: Option<i64>,
    /// Fade-in start point in milliseconds
    pub fade_in_point_ms: Option<i64>,
    /// Fade-out start point in milliseconds
    pub fade_out_point_ms: Option<i64>,
    /// Fade-in curve type
    pub fade_in_curve: Option<String>,
    /// Fade-out curve type
    pub fade_out_curve: Option<String>,
}

/// Audio file metadata
///
/// Stored in the `files` table. Basic metadata for audio files
/// identified during import.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct File {
    /// Unique identifier (UUID)
    pub guid: String,
    /// File path
    pub path: String,
    /// SHA-256 hash for deduplication
    pub hash: String,
    /// Duration in seconds
    pub duration: Option<f64>,
}

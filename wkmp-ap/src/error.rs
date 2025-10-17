//! Error types for the audio player module
//!
//! Implements CO-162: Expected errors shall use Result<T, E> types with meaningful error enums
//! Implements Recommendation: Document error codes as they're defined

use thiserror::Error;

/// Main error type for the audio player
#[derive(Debug, Error)]
pub enum AudioPlayerError {
    /// IO errors from file operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Database errors
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Playback-specific errors
    #[error("Playback error: {0}")]
    Playback(#[from] PlaybackError),

    /// Decoding-specific errors
    #[error("Decode error: {0}")]
    Decode(#[from] DecodeError),

    /// Buffer management errors
    #[error("Buffer error: {0}")]
    Buffer(#[from] BufferError),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Generic errors
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Errors related to audio playback operations
#[derive(Debug, Error)]
pub enum PlaybackError {
    /// No audio device available
    #[error("No audio device available")]
    NoDevice,

    /// Audio device disconnected during playback
    #[error("Audio device disconnected")]
    DeviceDisconnected,

    /// Pipeline is not initialized
    #[error("Pipeline not initialized")]
    PipelineNotInitialized,

    /// Invalid state transition
    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: PlaybackState,
        to: PlaybackState,
    },

    /// Queue is empty
    #[error("Playback queue is empty")]
    QueueEmpty,

    /// Position out of bounds
    #[error("Seek position {position} out of bounds (0..{duration})")]
    PositionOutOfBounds {
        position: f64,
        duration: f64,
    },

    /// Crossfade configuration error
    #[error("Invalid crossfade configuration: {reason}")]
    InvalidCrossfade {
        reason: String,
    },
}

/// Errors related to audio decoding
#[derive(Debug, Error)]
pub enum DecodeError {
    /// Unsupported audio format
    #[error("Unsupported audio format: {format}")]
    UnsupportedFormat {
        format: String,
    },

    /// File not found
    #[error("Audio file not found: {path}")]
    FileNotFound {
        path: String,
    },

    /// Corrupted audio data
    #[error("Corrupted audio data at offset {offset}")]
    CorruptedData {
        offset: u64,
    },

    /// Failed to probe audio format
    #[error("Failed to probe audio format")]
    ProbeFailure,

    /// No audio tracks in file
    #[error("No audio tracks found in file")]
    NoAudioTracks,

    /// Seek failed
    #[error("Failed to seek to position {position}")]
    SeekFailed {
        position: u64,
    },

    /// Resampling error
    #[error("Resampling error: {reason}")]
    ResamplingError {
        reason: String,
    },
}

/// Errors related to buffer management
#[derive(Debug, Error)]
pub enum BufferError {
    /// Buffer not found
    #[error("Buffer not found for passage {passage_id}")]
    NotFound {
        passage_id: uuid::Uuid,
    },

    /// Buffer already exists
    #[error("Buffer already exists for passage {passage_id}")]
    AlreadyExists {
        passage_id: uuid::Uuid,
    },

    /// Buffer underrun
    #[error("Buffer underrun: needed {needed} samples, had {available}")]
    Underrun {
        needed: usize,
        available: usize,
    },

    /// Invalid buffer state
    #[error("Invalid buffer state: expected {expected:?}, was {actual:?}")]
    InvalidState {
        expected: BufferState,
        actual: BufferState,
    },

    /// Out of memory
    #[error("Out of memory: tried to allocate {requested_mb:.1} MB, {available_mb:.1} MB available")]
    OutOfMemory {
        requested_mb: f64,
        available_mb: f64,
    },
}

/// Playback states for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    Buffering,
}

/// Buffer states for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferState {
    Decoding,
    Ready,
    Playing,
    Exhausted,
}

/// Result type alias using our error type
pub type Result<T> = std::result::Result<T, AudioPlayerError>;

/// Error recovery strategies
///
/// Implements requirement from coding conventions: Network errors shall implement retry logic
#[derive(Debug, Clone, Copy)]
pub enum RecoveryStrategy {
    /// Retry the operation
    Retry {
        /// Maximum number of retry attempts
        max_attempts: u32,
        /// Delay between retries in milliseconds
        delay_ms: u64,
    },

    /// Skip the current item and continue
    Skip,

    /// Log the error and continue
    LogAndContinue,

    /// Fatal error - stop execution
    Fatal,
}

impl AudioPlayerError {
    /// Get the recommended recovery strategy for this error
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            // IO errors might be transient
            AudioPlayerError::Io(_) => RecoveryStrategy::Retry {
                max_attempts: 3,
                delay_ms: 1000,
            },

            // Database errors might be transient
            AudioPlayerError::Database(_) => RecoveryStrategy::Retry {
                max_attempts: 2,
                delay_ms: 500,
            },

            // Decode errors are usually permanent for a file
            AudioPlayerError::Decode(DecodeError::FileNotFound { .. }) => RecoveryStrategy::Skip,
            AudioPlayerError::Decode(DecodeError::UnsupportedFormat { .. }) => RecoveryStrategy::Skip,
            AudioPlayerError::Decode(DecodeError::CorruptedData { .. }) => RecoveryStrategy::Skip,
            AudioPlayerError::Decode(DecodeError::NoAudioTracks) => RecoveryStrategy::Skip,

            // Buffer underruns might recover
            AudioPlayerError::Buffer(BufferError::Underrun { .. }) => RecoveryStrategy::Retry {
                max_attempts: 5,
                delay_ms: 100,
            },

            // Device disconnection is fatal
            AudioPlayerError::Playback(PlaybackError::DeviceDisconnected) => RecoveryStrategy::Fatal,
            AudioPlayerError::Playback(PlaybackError::NoDevice) => RecoveryStrategy::Fatal,

            // Most other errors log and continue
            _ => RecoveryStrategy::LogAndContinue,
        }
    }

    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            AudioPlayerError::Decode(DecodeError::UnsupportedFormat { format }) => {
                format!("The audio format '{}' is not supported", format)
            }
            AudioPlayerError::Decode(DecodeError::FileNotFound { path }) => {
                format!("Could not find audio file: {}", path)
            }
            AudioPlayerError::Playback(PlaybackError::NoDevice) => {
                "No audio output device available".to_string()
            }
            AudioPlayerError::Playback(PlaybackError::DeviceDisconnected) => {
                "Audio device was disconnected".to_string()
            }
            AudioPlayerError::Buffer(BufferError::OutOfMemory { .. }) => {
                "Not enough memory to buffer audio".to_string()
            }
            _ => "An error occurred during playback".to_string(),
        }
    }

    /// Get an error code for logging/debugging
    pub fn error_code(&self) -> &'static str {
        match self {
            AudioPlayerError::Io(_) => "AP_IO_001",
            AudioPlayerError::Database(_) => "AP_DB_001",
            AudioPlayerError::Playback(e) => match e {
                PlaybackError::NoDevice => "AP_PB_001",
                PlaybackError::DeviceDisconnected => "AP_PB_002",
                PlaybackError::PipelineNotInitialized => "AP_PB_003",
                PlaybackError::InvalidStateTransition { .. } => "AP_PB_004",
                PlaybackError::QueueEmpty => "AP_PB_005",
                PlaybackError::PositionOutOfBounds { .. } => "AP_PB_006",
                PlaybackError::InvalidCrossfade { .. } => "AP_PB_007",
            },
            AudioPlayerError::Decode(e) => match e {
                DecodeError::UnsupportedFormat { .. } => "AP_DC_001",
                DecodeError::FileNotFound { .. } => "AP_DC_002",
                DecodeError::CorruptedData { .. } => "AP_DC_003",
                DecodeError::ProbeFailure => "AP_DC_004",
                DecodeError::NoAudioTracks => "AP_DC_005",
                DecodeError::SeekFailed { .. } => "AP_DC_006",
                DecodeError::ResamplingError { .. } => "AP_DC_007",
            },
            AudioPlayerError::Buffer(e) => match e {
                BufferError::NotFound { .. } => "AP_BF_001",
                BufferError::AlreadyExists { .. } => "AP_BF_002",
                BufferError::Underrun { .. } => "AP_BF_003",
                BufferError::InvalidState { .. } => "AP_BF_004",
                BufferError::OutOfMemory { .. } => "AP_BF_005",
            },
            AudioPlayerError::Config(_) => "AP_CF_001",
            AudioPlayerError::Other(_) => "AP_OT_001",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_unique() {
        use std::collections::HashSet;

        // Create various errors and check their codes are unique
        let errors = vec![
            AudioPlayerError::Playback(PlaybackError::NoDevice),
            AudioPlayerError::Playback(PlaybackError::QueueEmpty),
            AudioPlayerError::Decode(DecodeError::ProbeFailure),
            AudioPlayerError::Buffer(BufferError::NotFound {
                passage_id: uuid::Uuid::new_v4(),
            }),
        ];

        let codes: HashSet<_> = errors.iter().map(|e| e.error_code()).collect();
        assert_eq!(codes.len(), errors.len(), "Error codes must be unique");
    }

    #[test]
    fn test_recovery_strategies() {
        let file_not_found = AudioPlayerError::Decode(DecodeError::FileNotFound {
            path: "/test.mp3".to_string(),
        });

        matches!(file_not_found.recovery_strategy(), RecoveryStrategy::Skip);

        let buffer_underrun = AudioPlayerError::Buffer(BufferError::Underrun {
            needed: 1000,
            available: 500,
        });

        matches!(buffer_underrun.recovery_strategy(), RecoveryStrategy::Retry { .. });
    }
}
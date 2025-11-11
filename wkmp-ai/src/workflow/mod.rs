//! Per-song workflow engine for audio import processing
//!
//! Implements the sequential per-song processing pipeline:
//! - **Phase 0**: Passage boundary detection (silence-based segmentation)
//! - **Phases 1-6**: Per-passage hybrid fusion pipeline
//!
//! **[PLAN023]** [REQ-AI-010] Sequential per-song processing (not file-level atomic)
//!
//! # Architecture
//!
//! The workflow processes each audio file in stages:
//! 1. Detect passage boundaries using silence detection
//! 2. Extract metadata from each passage (ID3, Chromaprint, etc.)
//! 3. Fuse metadata from multiple sources
//! 4. Validate and score quality
//! 5. Store results in database
//!
//! Uses the 3-tier pipeline architecture (extractors → fusers → validators).

pub mod boundary_detector;
pub mod event_bridge;
pub mod pipeline;  // PLAN024: 3-tier pipeline orchestrator
pub mod storage;   // PLAN024: Database storage for processed passages

use crate::types::{ExtractionResult, ValidationResult};

// Re-exports for convenience
pub use pipeline::{Pipeline, PipelineConfig};

/// Passage boundary (start/end times in SPEC017 ticks)
///
/// Per SPEC017 [SRC-TICK-030]: One tick = 1/28,224,000 second
/// Per REQ-AI-088-03: SHALL use i64 ticks for sample-accurate precision
#[derive(Debug, Clone)]
pub struct PassageBoundary {
    /// Start time in ticks from file start (SPEC017 compliant)
    pub start_time: i64,
    /// End time in ticks from file start (SPEC017 compliant)
    pub end_time: i64,
    /// Confidence score for this boundary detection (0.0-1.0)
    pub confidence: f64,
}

/// Decoded audio file with detected passage boundaries
///
/// **[AIA-PERF-046]** Cache decoded audio to avoid re-decoding for each passage.
/// Boundary detection decodes entire file; this struct allows passages to reuse that audio.
#[derive(Debug, Clone)]
pub struct FileAudioData {
    /// Detected passage boundaries
    pub boundaries: Vec<PassageBoundary>,
    /// Decoded audio samples (interleaved if stereo, mono if 1 channel)
    pub samples: Vec<f32>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels (1=mono, 2=stereo)
    pub num_channels: u8,
}

/// SPEC017 tick rate constant
pub const TICK_RATE: i64 = 28_224_000;

/// Complete fused passage data (all fusers combined)
#[derive(Debug, Clone)]
pub struct FusedPassage {
    /// Fused metadata (title, artist, album)
    pub metadata: crate::types::FusedMetadata,
    /// Fused identity (recording MBID, work MBID)
    pub identity: crate::types::FusedIdentity,
    /// Fused musical flavor vector
    pub flavor: crate::types::FusedFlavor,
}

/// Complete passage result after fusion and validation
#[derive(Debug, Clone)]
pub struct ProcessedPassage {
    /// Passage boundary (start/end times in ticks)
    pub boundary: PassageBoundary,
    /// Individual extraction results from all extractors
    pub extractions: Vec<ExtractionResult>,
    /// Fused data from all fusers
    pub fusion: FusedPassage,
    /// Validation result (quality score, confidence)
    pub validation: ValidationResult,
}

/// Import workflow events for SSE broadcasting
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum WorkflowEvent {
    /// File processing started
    FileStarted {
        /// Path to audio file being processed
        file_path: String,
        /// Unix timestamp (seconds since epoch)
        timestamp: i64,
    },

    /// Passage boundary detected
    ///
    /// Note: Times stored in ticks internally, converted to seconds for SSE display
    BoundaryDetected {
        /// Passage index (0-based)
        passage_index: usize,
        /// Start time in ticks (SPEC017)
        start_time: i64,
        /// End time in ticks (SPEC017)
        end_time: i64,
        /// Boundary detection confidence (0.0-1.0)
        confidence: f64,
    },

    /// Passage processing started
    PassageStarted {
        /// Passage index (0-based)
        passage_index: usize,
        /// Total passages detected in file
        total_passages: usize,
    },

    /// Extraction phase progress
    ExtractionProgress {
        /// Passage index (0-based)
        passage_index: usize,
        /// Extractor name (e.g., "id3", "chromaprint")
        extractor: String,
        /// Status message (e.g., "extracting", "complete", "failed")
        status: String,
    },

    /// Fusion phase started
    FusionStarted {
        /// Passage index (0-based)
        passage_index: usize,
    },

    /// Validation phase started
    ValidationStarted {
        /// Passage index (0-based)
        passage_index: usize,
    },

    /// Passage processing completed
    PassageCompleted {
        /// Passage index (0-based)
        passage_index: usize,
        /// Quality score (0.0-1.0)
        quality_score: f64,
        /// Validation status (e.g., "accepted", "rejected")
        validation_status: String,
    },

    /// File processing completed
    FileCompleted {
        /// Path to audio file
        file_path: String,
        /// Number of passages successfully processed
        passages_processed: usize,
        /// Unix timestamp (seconds since epoch)
        timestamp: i64,
    },

    /// Error occurred
    Error {
        /// Passage index if error relates to specific passage
        passage_index: Option<usize>,
        /// Error message
        message: String,
    },

    /// **[AIA-SEC-030]** AcoustID API key invalid - user prompt required
    ///
    /// Emitted when AcoustID returns 400 error with "invalid API key".
    /// UI should prompt user to either:
    /// 1. Provide valid API key (will be validated before resuming)
    /// 2. Skip AcoustID functionality for this session
    AcoustIDKeyInvalid {
        /// Error message from AcoustID API
        error_message: String,
    },
}

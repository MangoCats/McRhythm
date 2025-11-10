// Per-Song Workflow Engine
//
// PLAN023: REQ-AI-010 series - Sequential per-song processing (not file-level atomic)
// Phase 0: Passage boundary detection
// Phases 1-6: Per-passage hybrid fusion pipeline

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

/// SPEC017 tick rate constant
pub const TICK_RATE: i64 = 28_224_000;

/// Complete fused passage data (all fusers combined)
#[derive(Debug, Clone)]
pub struct FusedPassage {
    pub metadata: crate::types::FusedMetadata,
    pub identity: crate::types::FusedIdentity,
    pub flavor: crate::types::FusedFlavor,
}

/// Complete passage result after fusion and validation
#[derive(Debug, Clone)]
pub struct ProcessedPassage {
    pub boundary: PassageBoundary,
    pub extractions: Vec<ExtractionResult>,
    pub fusion: FusedPassage,
    pub validation: ValidationResult,
}

/// Import workflow events for SSE broadcasting
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum WorkflowEvent {
    /// File processing started
    FileStarted {
        file_path: String,
        timestamp: i64,
    },

    /// Passage boundary detected
    ///
    /// Note: Times stored in ticks internally, converted to seconds for SSE display
    BoundaryDetected {
        passage_index: usize,
        start_time: i64,  // SPEC017 ticks
        end_time: i64,    // SPEC017 ticks
        confidence: f64,
    },

    /// Passage processing started
    PassageStarted {
        passage_index: usize,
        total_passages: usize,
    },

    /// Extraction phase progress
    ExtractionProgress {
        passage_index: usize,
        extractor: String,
        status: String,
    },

    /// Fusion phase started
    FusionStarted {
        passage_index: usize,
    },

    /// Validation phase started
    ValidationStarted {
        passage_index: usize,
    },

    /// Passage processing completed
    PassageCompleted {
        passage_index: usize,
        quality_score: f64,
        validation_status: String,
    },

    /// File processing completed
    FileCompleted {
        file_path: String,
        passages_processed: usize,
        timestamp: i64,
    },

    /// Error occurred
    Error {
        passage_index: Option<usize>,
        message: String,
    },
}

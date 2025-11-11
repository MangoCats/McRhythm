// PLAN023: Shared Types and Data Contracts
//
// This module defines the explicit "synchronizations" (data contracts) between
// the three tiers of the hybrid fusion architecture.
//
// Legible Software Principle: Explicit contracts between concepts make system
// behavior predictable and analyzable. Each type represents a well-defined
// interface between independent modules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Tier 1 Outputs: Extractor Results with Confidence Scores
// ============================================================================

/// Data extracted from a single source with confidence score
///
/// Contract: All Tier 1 extractors return results wrapped in this type.
/// Confidence range: 0.0 (unreliable) to 1.0 (authoritative)
#[derive(Debug, Clone)]
pub struct ExtractorResult<T> {
    pub data: T,
    pub confidence: f64,  // [0.0, 1.0]
    pub source: ExtractionSource,
}

/// Source of extracted data (for provenance tracking)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtractionSource {
    ID3Metadata,        // confidence: 0.5 (user-editable)
    Chromaprint,        // confidence: 0.7 (fingerprint matching)
    AcoustID,           // confidence: 0.8 (fingerprint + crowd-sourced)
    MusicBrainz,        // confidence: 0.9 (authoritative database)
    Essentia,           // confidence: 0.9 (same algorithms as AcousticBrainz)
    AudioDerived,       // confidence: 0.4 (computed from signal)
    GenreMapping,       // confidence: 0.3 (coarse approximation)
    AcousticBrainzArchive,  // confidence: 0.85 (historical data, may be stale)
}

impl ExtractionSource {
    /// Get default confidence for this source
    pub fn default_confidence(self) -> f64 {
        match self {
            Self::ID3Metadata => 0.5,
            Self::Chromaprint => 0.7,
            Self::AcoustID => 0.8,
            Self::MusicBrainz => 0.9,
            Self::Essentia => 0.9,
            Self::AudioDerived => 0.4,
            Self::GenreMapping => 0.3,
            Self::AcousticBrainzArchive => 0.85,
        }
    }
}

// ============================================================================
// Identity Resolution Types (Tier 2 Input/Output)
// ============================================================================

/// MusicBrainz Recording ID candidate with confidence
#[derive(Debug, Clone)]
pub struct MBIDCandidate {
    pub mbid: Uuid,  // MusicBrainz Recording ID
    pub confidence: f64,  // Posterior probability after Bayesian update
    pub sources: Vec<ExtractionSource>,  // Which sources contributed
}

/// Resolved identity (output of IdentityResolver)
#[derive(Debug, Clone)]
pub struct ResolvedIdentity {
    pub mbid: Option<Uuid>,  // None if no confident match
    pub confidence: f64,     // Final confidence after fusion
    pub candidates: Vec<MBIDCandidate>,  // All candidates considered
    pub has_conflict: bool,  // True if conflicting high-confidence sources
}

// ============================================================================
// Metadata Types (Tier 2 Input/Output)
// ============================================================================

/// Metadata field with source provenance
#[derive(Debug, Clone)]
pub struct MetadataField<T> {
    pub value: T,
    pub confidence: f64,
    pub source: ExtractionSource,
}

/// Collection of metadata fields from all sources
#[derive(Debug, Clone, Default)]
pub struct MetadataBundle {
    pub title: Vec<MetadataField<String>>,
    pub artist: Vec<MetadataField<String>>,
    pub album: Vec<MetadataField<String>>,
    pub release_date: Vec<MetadataField<String>>,
    pub track_number: Vec<MetadataField<u32>>,
    pub duration_ms: Vec<MetadataField<u32>>,
}

/// Fused metadata (output of MetadataFuser)
#[derive(Debug, Clone)]
pub struct FusedMetadata {
    pub title: Option<MetadataField<String>>,
    pub artist: Option<MetadataField<String>>,
    pub album: Option<MetadataField<String>>,
    pub release_date: Option<MetadataField<String>>,
    pub track_number: Option<MetadataField<u32>>,
    pub duration_ms: Option<MetadataField<u32>>,
    pub metadata_confidence: f64,  // Overall metadata quality
}

// ============================================================================
// Musical Flavor Types (Tier 2 Input/Output)
// ============================================================================

/// Single musical characteristic (binary or complex)
///
/// Examples:
/// - Binary: {"danceable": 0.7, "not_danceable": 0.3} (sums to 1.0)
/// - Complex: {"ambient": 0.5, "house": 0.3, "techno": 0.2} (sums to 1.0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Characteristic {
    pub name: String,  // e.g., "danceability", "genre_electronic"
    pub values: HashMap<String, f64>,  // dimension → probability
}

impl Characteristic {
    /// Verify that values sum to 1.0 (within tolerance)
    pub fn is_normalized(&self) -> bool {
        let sum: f64 = self.values.values().sum();
        (sum - 1.0).abs() < 0.0001
    }

    /// Normalize values to sum to 1.0
    pub fn normalize(&mut self) {
        let sum: f64 = self.values.values().sum();
        if sum > 0.0 {
            for v in self.values.values_mut() {
                *v /= sum;
            }
        }
    }
}

/// Complete musical flavor profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicalFlavor {
    pub characteristics: Vec<Characteristic>,
}

impl MusicalFlavor {
    /// Get characteristic by name
    pub fn get(&self, name: &str) -> Option<&Characteristic> {
        self.characteristics.iter().find(|c| c.name == name)
    }

    /// Verify all characteristics are normalized
    pub fn validate(&self) -> bool {
        self.characteristics.iter().all(|c| c.is_normalized())
    }

    /// Count present characteristics
    pub fn count_present(&self) -> usize {
        self.characteristics.len()
    }

    /// Calculate completeness score (present / expected)
    /// Expected: 18 characteristics (per CRITICAL-002)
    pub fn completeness(&self) -> f64 {
        const EXPECTED_CHARACTERISTICS: f64 = 18.0;
        (self.count_present() as f64) / EXPECTED_CHARACTERISTICS
    }
}

/// Musical flavor from a single source
#[derive(Debug, Clone)]
pub struct FlavorExtraction {
    pub flavor: MusicalFlavor,
    pub confidence: f64,
    pub source: ExtractionSource,
}

/// Synthesized musical flavor (output of FlavorSynthesizer)
#[derive(Debug, Clone)]
pub struct SynthesizedFlavor {
    pub flavor: MusicalFlavor,
    pub flavor_confidence: f64,  // Overall flavor quality
    pub flavor_completeness: f64,  // Percentage of expected characteristics
    pub sources_used: Vec<ExtractionSource>,
}

// ============================================================================
// Passage Boundary Types
// ============================================================================

/// Single passage boundary (start/end timestamps)
///
/// **[SRC-DB-010]** Time values are stored as ticks (i64) for sample-accurate precision.
/// Tick rate: 28,224,000 Hz (1 tick ≈ 35.4 nanoseconds)
#[derive(Debug, Clone, Copy)]
pub struct PassageBoundary {
    pub start_ticks: i64,  // Passage start (ticks from file start)
    pub end_ticks: i64,    // Passage end (ticks from file start)
    pub confidence: f64,
    pub detection_method: BoundaryDetectionMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundaryDetectionMethod {
    SilenceDetection,
    BeatTracking,  // Future
    StructuralAnalysis,  // Future
}

// ============================================================================
// Validation Types (Tier 3 Output)
// ============================================================================

/// Validation result for a single check
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Pass,
    Warning { message: String },
    Conflict { message: String, severity: ConflictSeverity },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictSeverity {
    Low,     // Minor inconsistency (e.g., capitalization difference)
    Medium,  // Moderate inconsistency (e.g., different release year)
    High,    // Major inconsistency (likely different songs)
}

/// Complete validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub quality_score: f64,  // [0.0, 1.0] overall quality
    pub has_conflicts: bool,
    pub warnings: Vec<String>,
    pub conflicts: Vec<(String, ConflictSeverity)>,
}

// ============================================================================
// Complete Passage Data (Final Output)
// ============================================================================

/// Complete data for a single passage after all processing
#[derive(Debug, Clone)]
pub struct ProcessedPassage {
    // Identity
    pub identity: ResolvedIdentity,

    // Metadata
    pub metadata: FusedMetadata,

    // Musical Flavor
    pub flavor: SynthesizedFlavor,

    // Boundary
    pub boundary: PassageBoundary,

    // Validation
    pub validation: ValidationReport,

    // Provenance
    pub import_duration_ms: u64,
    pub import_timestamp: String,
    pub import_version: String,  // PLAN023 version identifier
}

// ============================================================================
// Workflow Types
// ============================================================================

/// Per-song processing state
#[derive(Debug, Clone)]
pub enum ProcessingState {
    Pending,
    Extracting,
    Fusing,
    Validating,
    Complete,
    Failed { error: String },
}

/// Per-song workflow context (passed through pipeline)
#[derive(Debug, Clone)]
pub struct SongContext {
    pub song_index: usize,  // 0-based index within file
    pub total_songs: usize,
    pub boundary: PassageBoundary,
    pub audio_samples: Vec<f32>,  // PCM audio for this passage
    pub sample_rate: u32,
    pub state: ProcessingState,
}

// ============================================================================
// SSE Event Types
// ============================================================================

/// SSE events emitted during import
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImportEvent {
    /// Session started (PLAN024)
    SessionStarted {
        session_id: uuid::Uuid,
        root_folder: String,
    },

    /// Session failed (PLAN024)
    SessionFailed {
        session_id: uuid::Uuid,
        error: String,
    },

    /// Phase 0: Passage boundaries discovered
    PassagesDiscovered {
        session_id: uuid::Uuid,  // REQ-TD-006: Added for event correlation
        file_path: String,
        count: usize,
    },

    /// Phase 1: Starting song processing
    SongStarted {
        session_id: uuid::Uuid,  // REQ-TD-006: Added for event correlation
        song_index: usize,
        total_songs: usize,
    },

    /// Phase 2: Tier 1 extraction complete
    ExtractionComplete {
        session_id: uuid::Uuid,  // REQ-TD-006: Added for event correlation
        song_index: usize,
        sources: Vec<ExtractionSource>,
    },

    /// Phase 3: Tier 2 fusion complete
    FusionComplete {
        session_id: uuid::Uuid,  // REQ-TD-006: Added for event correlation
        song_index: usize,
        identity_confidence: f64,
        metadata_confidence: f64,
        flavor_confidence: f64,
    },

    /// Phase 4: Tier 3 validation complete
    ValidationComplete {
        session_id: uuid::Uuid,  // REQ-TD-006: Added for event correlation
        song_index: usize,
        quality_score: f64,
        has_conflicts: bool,
    },

    /// Phase 5: Song processing complete
    SongComplete {
        session_id: uuid::Uuid,  // REQ-TD-006: Added for event correlation
        song_index: usize,
        duration_ms: u64,
    },

    /// Song processing failed
    SongFailed {
        session_id: uuid::Uuid,  // REQ-TD-006: Added for event correlation
        song_index: usize,
        error: String,
    },

    /// File import complete
    FileComplete {
        session_id: uuid::Uuid,  // REQ-TD-006: Added for event correlation
        file_path: String,
        successes: usize,
        warnings: usize,
        failures: usize,
        total_duration_ms: u64,
    },
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Fusion failed: {0}")]
    FusionFailed(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Audio processing failed: {0}")]
    AudioProcessingFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("SSE broadcast error: {0}")]
    BroadcastError(String),
}

pub type ImportResult<T> = Result<T, ImportError>;

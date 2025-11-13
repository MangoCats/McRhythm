//! Core Types and Trait Definitions for WKMP-AI
//!
//! Defines base traits for the 3-tier hybrid fusion architecture:
//! - **Tier 1:** SourceExtractor (7 extractors)
//! - **Tier 2:** Fusion (4 fusers)
//! - **Tier 3:** Validation (3 validators)
//!
//! # Implementation
//! TASK-004: Base Traits & Types (PLAN024)
//!
//! # Architecture
//! Per-passage entity-precise workflow (Phase 0-6):
//! - Phase 0: Passage boundary detection
//! - Phase 1-6: Per-passage processing (extraction → fusion → validation)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Common Types
// ============================================================================

/// Passage context for extraction and processing
///
/// Contains all information needed for a single passage analysis pass.
#[derive(Debug, Clone)]
pub struct PassageContext {
    /// Passage UUID
    pub passage_id: Uuid,
    /// Parent file UUID
    pub file_id: Uuid,
    /// Path to source audio file
    pub file_path: PathBuf,
    /// Passage start time in ticks (SPEC017)
    pub start_time_ticks: i64,
    /// Passage end time in ticks (SPEC017)
    pub end_time_ticks: i64,
    /// Audio samples (f32 mono/stereo)
    pub audio_samples: Option<Vec<f32>>,
    /// Sample rate (Hz)
    pub sample_rate: Option<u32>,
    /// Number of channels (1=mono, 2=stereo)
    pub num_channels: Option<u8>,
    /// Import session ID (for grouping passages)
    pub import_session_id: Uuid,
}

/// Confidence-scored metadata value
///
/// Represents a metadata field with its source provenance and confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceValue<T> {
    /// Metadata value
    pub value: T,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Source that provided this value
    pub source: String,
}

impl<T> ConfidenceValue<T> {
    /// Create new confidence value with clamped confidence (0.0-1.0)
    pub fn new(value: T, confidence: f32, source: impl Into<String>) -> Self {
        Self {
            value,
            confidence: confidence.clamp(0.0, 1.0),
            source: source.into(),
        }
    }
}

// ============================================================================
// Tier 1: Source Extractor Trait
// ============================================================================

/// Tier 1 Source Extractor trait
///
/// All extractors implement this trait for uniform parallel execution.
/// Each extractor produces confidence-scored output for downstream fusion.
///
/// # Extractors (7 total)
/// 1. ID3 Extractor - Extract ID3 metadata tags
/// 2. Chromaprint Analyzer - Generate audio fingerprints
/// 3. AcoustID Client - Query fingerprint → MBID
/// 4. MusicBrainz Client - Query MBID → Recording metadata
/// 5. Essentia Analyzer - Extract musical features (optional)
/// 6. AudioDerived Extractor - Algorithmic feature extraction
/// 7. ID3 Genre Mapper - Map ID3 genre → musical flavor
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::types::{SourceExtractor, PassageContext, ExtractionResult};
///
/// pub struct ID3Extractor;
///
/// #[async_trait::async_trait]
/// impl SourceExtractor for ID3Extractor {
///     fn name(&self) -> &'static str { "ID3" }
///     fn base_confidence(&self) -> f32 { 0.6 }
///
///     async fn extract(&self, ctx: &PassageContext) -> Result<ExtractionResult, ExtractionError> {
///         // Extract ID3 tags from file
///         let tags = extract_id3_tags(&ctx.file_path)?;
///         Ok(ExtractionResult {
///             metadata: Some(tags.into()),
///             identity: None,
///             musical_flavor: None,
///         })
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait SourceExtractor: Send + Sync {
    /// Extractor name for provenance tracking
    fn name(&self) -> &'static str;

    /// Base confidence score for this extractor (0.0-1.0)
    ///
    /// Per-extraction confidence may vary based on data quality,
    /// but this provides the baseline.
    fn base_confidence(&self) -> f32;

    /// Extract data from passage context
    ///
    /// # Arguments
    /// * `ctx` - Passage context with audio file path, timing, samples
    ///
    /// # Returns
    /// Extraction result with confidence-scored outputs
    ///
    /// # Errors
    /// Returns `ExtractionError` if extraction fails (per-passage error isolation)
    async fn extract(&self, ctx: &PassageContext) -> Result<ExtractionResult, ExtractionError>;
}

/// Extraction result from a Tier 1 source extractor
///
/// Each field is optional - extractors return only what they can provide.
/// All outputs include confidence scores for downstream fusion.
#[derive(Debug, Clone, Default)]
pub struct ExtractionResult {
    /// Metadata extraction (title, artist, album, etc.)
    pub metadata: Option<MetadataExtraction>,
    /// Identity resolution (Recording MBID)
    pub identity: Option<IdentityExtraction>,
    /// Musical flavor extraction
    pub musical_flavor: Option<FlavorExtraction>,
}

/// Metadata extraction result
#[derive(Debug, Clone, Default)]
pub struct MetadataExtraction {
    /// Track title with confidence
    pub title: Option<ConfidenceValue<String>>,
    /// Artist name with confidence
    pub artist: Option<ConfidenceValue<String>>,
    /// Album title with confidence
    pub album: Option<ConfidenceValue<String>>,
    /// MusicBrainz Recording MBID with confidence
    pub recording_mbid: Option<ConfidenceValue<String>>,
    /// Additional metadata fields (e.g., "year", "genre")
    pub additional: HashMap<String, ConfidenceValue<String>>,
}

/// Identity resolution result (MusicBrainz Recording MBID)
#[derive(Debug, Clone)]
pub struct IdentityExtraction {
    /// MusicBrainz Recording MBID
    pub recording_mbid: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Source of identification ("AcoustID", "ID3", etc.)
    pub source: String,
}

/// Musical flavor extraction result
#[derive(Debug, Clone)]
pub struct FlavorExtraction {
    /// Musical flavor characteristics (e.g., "danceability": 0.7)
    pub characteristics: HashMap<String, f32>,
    /// Confidence score for this flavor extraction
    pub confidence: f32,
    /// Source extractor name
    pub source: String,
}

/// Extraction error
#[derive(Debug, Error)]
pub enum ExtractionError {
    /// I/O error (file read/write)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Audio decoding failed
    #[error("Audio decoding error: {0}")]
    AudioDecode(String),

    /// Network communication error
    #[error("Network error: {0}")]
    Network(String),

    /// External API error
    #[error("API error: {0}")]
    Api(String),

    /// Failed to parse response or data
    #[error("Parse error: {0}")]
    Parse(String),

    /// Unsupported audio format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Required extractor not available
    #[error("Extractor not available: {0}")]
    NotAvailable(String),

    /// Internal processing error
    #[error("Internal error: {0}")]
    Internal(String),
}

// ============================================================================
// Tier 2: Fusion Trait
// ============================================================================

/// Tier 2 Fusion trait
///
/// Fuses multiple extraction results into a single confidence-scored output.
/// Implements conflict resolution, confidence weighting, and provenance tracking.
///
/// # Fusers (4 total)
/// 1. IdentityResolver - Bayesian fusion of Recording MBIDs
/// 2. MetadataFuser - Field-wise metadata fusion with conflict resolution
/// 3. FlavorSynthesizer - Weighted fusion of musical flavor characteristics
/// 4. BoundaryFuser - Not implemented (boundaries already detected in Phase 0)
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::types::{Fusion, ExtractionResult, FusionResult};
///
/// pub struct MetadataFuser;
///
/// #[async_trait::async_trait]
/// impl Fusion for MetadataFuser {
///     type Input = Vec<ExtractionResult>;
///     type Output = FusedMetadata;
///
///     fn name(&self) -> &'static str { "MetadataFusion" }
///
///     async fn fuse(&self, inputs: Self::Input) -> Result<FusionResult<Self::Output>, FusionError> {
///         // Field-wise fusion with confidence weighting
///         let fused = fuse_metadata_fields(inputs)?;
///         Ok(FusionResult {
///             output: fused,
///             confidence: 0.85,
///             sources: vec!["ID3".into(), "MusicBrainz".into()],
///         })
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait Fusion: Send + Sync {
    /// Input type for fusion
    type Input;
    /// Output type after fusion
    type Output;

    /// Fuser name for provenance tracking
    fn name(&self) -> &'static str;

    /// Fuse multiple inputs into single output
    ///
    /// # Arguments
    /// * `inputs` - Collection of extraction results to fuse
    ///
    /// # Returns
    /// Fused result with overall confidence and source provenance
    ///
    /// # Errors
    /// Returns `FusionError` if fusion fails
    async fn fuse(&self, inputs: Self::Input) -> Result<FusionResult<Self::Output>, FusionError>;
}

/// Fusion result with confidence and provenance
#[derive(Debug, Clone)]
pub struct FusionResult<T> {
    /// Fused output
    pub output: T,
    /// Overall confidence score (0.0-1.0)
    pub confidence: f32,
    /// Contributing source names
    pub sources: Vec<String>,
}

/// Fused metadata result (Tier 2 output)
#[derive(Debug, Clone)]
pub struct FusedMetadata {
    /// Fused track title with confidence
    pub title: Option<ConfidenceValue<String>>,
    /// Fused artist name with confidence
    pub artist: Option<ConfidenceValue<String>>,
    /// Fused album title with confidence
    pub album: Option<ConfidenceValue<String>>,
    /// Fused MusicBrainz Recording MBID with confidence
    pub recording_mbid: Option<ConfidenceValue<String>>,
    /// Additional fused metadata (artist_mbid, release_mbid, etc.)
    pub additional: HashMap<String, ConfidenceValue<String>>,
    /// Metadata completeness score (0.0-1.0)
    pub metadata_completeness: f32,
}

/// Fused identity result (Tier 2 output)
#[derive(Debug, Clone)]
pub struct FusedIdentity {
    /// Fused MusicBrainz Recording MBID (None if no valid MBID found)
    pub recording_mbid: Option<String>,
    /// Overall confidence score (0.0-1.0)
    pub confidence: f32,
    /// Bayesian posterior probability
    pub posterior_probability: f32,
    /// List of conflicting identifications
    pub conflicts: Vec<String>,
}

/// Fused musical flavor result (Tier 2 output)
#[derive(Debug, Clone)]
pub struct FusedFlavor {
    /// Fused musical characteristics (e.g., "danceability": 0.7)
    pub characteristics: HashMap<String, f32>,
    /// Confidence for each characteristic
    pub confidence_map: HashMap<String, f32>,
    /// Source blend weights (source, weight)
    pub source_blend: Vec<(String, f32)>,
    /// Flavor completeness score (0.0-1.0)
    pub completeness: f32,
}

/// Fusion error
#[derive(Debug, Error)]
pub enum FusionError {
    /// Insufficient data to perform fusion
    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    /// Failed to resolve conflicting values
    #[error("Conflict resolution failed: {0}")]
    ConflictResolution(String),

    /// Invalid or inconsistent confidence scores
    #[error("Invalid confidence scores: {0}")]
    InvalidConfidence(String),

    /// Internal processing error
    #[error("Internal error: {0}")]
    Internal(String),
}

// ============================================================================
// Tier 3: Validation Trait
// ============================================================================

/// Tier 3 Validation trait
///
/// Validates fused outputs for consistency, completeness, and quality.
/// Produces quality scores and validation reports for decision-making.
///
/// # Validators (3 total)
/// 1. ConsistencyValidator - Cross-source consistency checks
/// 2. CompletenessScorer - Measure metadata/flavor completeness
/// 3. QualityScorer - Overall quality assessment
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::types::{Validation, FusedMetadata, ValidationResult};
///
/// pub struct ConsistencyValidator;
///
/// #[async_trait::async_trait]
/// impl Validation for ConsistencyValidator {
///     type Input = FusedMetadata;
///
///     fn name(&self) -> &'static str { "ConsistencyValidation" }
///
///     async fn validate(&self, input: &Self::Input) -> Result<ValidationResult, ValidationError> {
///         // Check for inconsistencies
///         let issues = check_consistency(input)?;
///         Ok(ValidationResult {
///             status: if issues.is_empty() { ValidationStatus::Pass } else { ValidationStatus::Warning },
///             score: 0.9,
///             issues,
///             report: serde_json::json!({ "consistency_checks": 5 }),
///         })
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait Validation: Send + Sync {
    /// Input type for validation
    type Input;

    /// Validator name for provenance tracking
    fn name(&self) -> &'static str;

    /// Validate input and produce quality assessment
    ///
    /// # Arguments
    /// * `input` - Fused data to validate
    ///
    /// # Returns
    /// Validation result with status, score, and detailed report
    ///
    /// # Errors
    /// Returns `ValidationError` if validation fails
    async fn validate(&self, input: &Self::Input)
        -> Result<ValidationResult, ValidationError>;
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Validation status
    pub status: ValidationStatus,
    /// Quality score (0.0-1.0)
    pub score: f32,
    /// List of validation issues
    pub issues: Vec<String>,
    /// Detailed validation report (JSON)
    pub report: serde_json::Value,
}

/// Validation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStatus {
    /// All checks passed
    Pass,
    /// Minor issues detected (non-blocking)
    Warning,
    /// Serious issues detected (review recommended)
    Fail,
    /// Validation not yet performed
    Pending,
}

/// Validation error
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Invalid input data
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Validation check failed
    #[error("Validation check failed: {0}")]
    CheckFailed(String),

    /// Internal processing error
    #[error("Internal error: {0}")]
    Internal(String),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_value_clamping() {
        let cv = ConfidenceValue::new("test".to_string(), 1.5, "TestSource");
        assert_eq!(cv.confidence, 1.0, "Confidence should be clamped to 1.0");

        let cv2 = ConfidenceValue::new("test".to_string(), -0.5, "TestSource");
        assert_eq!(cv2.confidence, 0.0, "Confidence should be clamped to 0.0");
    }

    #[test]
    fn test_validation_status_equality() {
        assert_eq!(ValidationStatus::Pass, ValidationStatus::Pass);
        assert_ne!(ValidationStatus::Pass, ValidationStatus::Warning);
    }

    #[test]
    fn test_passage_context_creation() {
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: None,
            sample_rate: Some(44100),
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        assert_eq!(ctx.sample_rate, Some(44100));
        assert_eq!(ctx.num_channels, Some(2));
    }
}

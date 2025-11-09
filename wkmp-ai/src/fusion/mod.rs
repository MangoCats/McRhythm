// Fusion Module - 3-Tier Hybrid Fusion Architecture
//
// PLAN023: WKMP-AI Ground-Up Recode
// Architecture: Tier 1 (Extractors) → Tier 2 (Fusers) → Tier 3 (Validators)

pub mod extractors;
pub mod fusers;
pub mod validators;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Source confidence score (0.0-1.0)
pub type Confidence = f64;

/// Musical flavor characteristic key (e.g., "danceability.danceable")
pub type CharacteristicKey = String;

/// Musical flavor value (0.0-1.0, normalized within categories)
pub type CharacteristicValue = f64;

/// Musical flavor characteristics map
pub type MusicalFlavor = HashMap<CharacteristicKey, CharacteristicValue>;

/// Extraction result from Tier 1 extractor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Source extractor identifier
    pub source: String,

    /// Overall confidence for this extraction (0.0-1.0)
    pub confidence: Confidence,

    /// Timestamp of extraction (Unix seconds)
    pub timestamp: i64,

    /// Extracted metadata (optional)
    pub metadata: Option<MetadataExtraction>,

    /// Extracted musical flavor (optional)
    pub flavor: Option<FlavorExtraction>,

    /// Extracted identity (optional)
    pub identity: Option<IdentityExtraction>,
}

/// Metadata extraction (title, artist, album, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataExtraction {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_seconds: Option<f64>,

    /// Field-specific confidence scores
    pub title_confidence: Option<Confidence>,
    pub artist_confidence: Option<Confidence>,
}

/// Musical flavor extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlavorExtraction {
    /// Musical flavor characteristics
    pub characteristics: MusicalFlavor,

    /// Per-characteristic confidence map (optional - if absent, use overall confidence)
    pub characteristic_confidence: Option<HashMap<CharacteristicKey, Confidence>>,
}

/// Identity extraction (MusicBrainz Recording MBID)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityExtraction {
    /// MusicBrainz Recording MBID
    pub recording_mbid: String,

    /// Confidence score for this MBID (0.0-1.0)
    pub confidence: Confidence,

    /// Additional context (e.g., AcoustID score, Levenshtein similarity)
    pub context: Option<HashMap<String, serde_json::Value>>,
}

/// Fused result from Tier 2 fuser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionResult {
    /// Fused metadata
    pub metadata: FusedMetadata,

    /// Fused musical flavor
    pub flavor: FusedFlavor,

    /// Fused identity
    pub identity: FusedIdentity,
}

/// Fused metadata with source provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedMetadata {
    pub title: Option<String>,
    pub title_source: Option<String>,
    pub title_confidence: Option<Confidence>,

    pub artist: Option<String>,
    pub artist_source: Option<String>,
    pub artist_confidence: Option<Confidence>,

    pub album: Option<String>,

    /// Metadata completeness (0.0-1.0)
    pub completeness: f64,

    /// Conflicts detected during fusion
    pub conflicts: Vec<ConflictReport>,
}

/// Fused musical flavor with source blend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedFlavor {
    /// Fused characteristics (normalized)
    pub characteristics: MusicalFlavor,

    /// Source blend (e.g., ["Essentia:0.9", "ID3Genre:0.3"])
    pub source_blend: Vec<String>,

    /// Per-characteristic confidence map
    pub confidence_map: HashMap<CharacteristicKey, Confidence>,

    /// Completeness score (present_characteristics / 18 expected)
    pub completeness: f64,
}

/// Fused identity with Bayesian posterior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedIdentity {
    /// Final resolved Recording MBID
    pub recording_mbid: Option<String>,

    /// Posterior confidence from Bayesian update
    pub confidence: f64,

    /// Conflicts detected during identity resolution
    pub conflicts: Vec<ConflictReport>,
}

/// Conflict report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub field: String,
    pub source1: String,
    pub value1: String,
    pub source2: String,
    pub value2: String,
    pub similarity: Option<f64>,
}

/// Validation result from Tier 3 validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Overall validation status
    pub status: ValidationStatus,

    /// Overall quality score (0-100%)
    pub quality_score: f64,

    /// Individual validation checks
    pub checks: Vec<ValidationCheck>,

    /// Warnings (non-fatal issues)
    pub warnings: Vec<String>,
}

/// Validation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStatus {
    Pass,
    Warning,
    Fail,
    Pending,
}

impl std::fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationStatus::Pass => write!(f, "Pass"),
            ValidationStatus::Warning => write!(f, "Warning"),
            ValidationStatus::Fail => write!(f, "Fail"),
            ValidationStatus::Pending => write!(f, "Pending"),
        }
    }
}

/// Individual validation check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub score: Option<f64>,
    pub message: Option<String>,
}

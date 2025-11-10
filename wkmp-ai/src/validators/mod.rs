//! Tier 3 Validation Layer
//!
//! Implements validators for the 3-tier hybrid fusion architecture.
//! Each validator assesses quality and consistency of fused data.
//!
//! # Validators
//! 1. **consistency_validator** - Cross-field consistency checks
//! 2. **completeness_scorer** - Data completeness assessment
//! 3. **quality_scorer** - Overall quality scoring
//!
//! # Implementation Status
//! - ✅ TASK-016: Consistency Validator
//! - ✅ TASK-017: Completeness Scorer
//! - ✅ TASK-018: Quality Scorer

// Module declarations (implemented validators)
pub mod consistency_validator; // TASK-016 ✅
pub mod completeness_scorer;   // TASK-017 ✅
pub mod quality_scorer;        // TASK-018 ✅

// Re-exports for convenience
pub use consistency_validator::ConsistencyValidator;
pub use completeness_scorer::CompletenessScorer;
pub use quality_scorer::QualityScorer;

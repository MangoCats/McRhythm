// PLAN023 Tier 3: Quality Validation & Enrichment
//
// Each module in this tier validates fused results from Tier 2 and detects conflicts.
// Validators run after fusion to ensure data quality and flag inconsistencies.
//
// Contract: All validators accept Tier 2 outputs and return ValidationResult

pub mod consistency_checker;  // ✅ Complete: Cross-source validation with Levenshtein
pub mod completeness_scorer;  // ✅ Complete: Quality scoring based on data presence
pub mod conflict_detector;    // ✅ Complete: High-level conflict detection and aggregation

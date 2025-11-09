// PLAN023 Tier 2: Confidence-Weighted Fusion Modules
//
// Each module in this tier is a "concept" with explicit "synchronizations" to Tier 1.
// Fusers combine multiple source extractions using confidence-weighted algorithms.
//
// Contract: All fusers accept Vec<ExtractorResult<T>> and output fused result with provenance.

pub mod flavor_synthesizer;  // ✅ Complete: Characteristic-wise weighted averaging
pub mod identity_resolver;   // ✅ Complete: Bayesian fusion of MBID candidates
pub mod metadata_fuser;      // ✅ Complete: Field-wise weighted selection
pub mod boundary_fuser;      // ✅ Complete: Multi-strategy boundary detection fusion

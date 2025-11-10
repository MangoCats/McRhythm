//! Tier 2 Fusion Layer
//!
//! Implements fusers for the 3-tier hybrid fusion architecture.
//! Each fuser combines extraction results from multiple Tier 1 sources.
//!
//! # Architecture
//! Per PLAN024 3-tier hybrid fusion:
//! - **Tier 1:** Extract raw data from multiple sources (extractors module)
//! - **Tier 2:** Fuse extracted data with confidence weighting (THIS MODULE)
//! - **Tier 3:** Validate fused results (validation module)
//!
//! # Fusers
//! 1. **identity_resolver** - Bayesian fusion of Recording MBIDs
//! 2. **metadata_fuser** - Field-wise metadata fusion with conflict resolution
//! 3. **flavor_synthesizer** - Weighted fusion of musical flavor characteristics
//!
//! # Implementation Status
//! - ✅ TASK-012: Identity Resolver
//! - ✅ TASK-013: Metadata Fuser
//! - ✅ TASK-014: Flavor Synthesizer

// Module declarations (implemented fusers)
pub mod identity_resolver; // TASK-012 ✅
pub mod metadata_fuser;    // TASK-013 ✅
pub mod flavor_synthesizer; // TASK-014 ✅

// Re-exports for convenience
pub use identity_resolver::IdentityResolver;
pub use metadata_fuser::MetadataFuser;
pub use flavor_synthesizer::FlavorSynthesizer;

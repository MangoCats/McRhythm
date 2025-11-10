// PLAN023: WKMP-AI Ground-Up Recode
//
// This module implements the 3-tier hybrid fusion architecture for audio import.
//
// Architecture (Legible Software Principles):
// - Each tier consists of independent "concepts" (modules with single, well-defined purpose)
// - Explicit "synchronizations" (data contracts) between tiers
// - Incrementality: Build tier-by-tier, test each independently
// - Integrity: Each module maintains its own invariants
// - Transparency: Behavior is explicit and traceable
//
// Reference: MIT Legible Software Model (Meng & Jackson, 2025)
// https://www.theregister.com/2025/11/07/researchers_detail_legible_software_model/

//! # PLAN023 Import System (v2)
//!
//! **3-Tier Hybrid Fusion Architecture:**
//!
//! ## Tier 1: Parallel Source Extractors (Independent Concepts)
//! - `id3_extractor` - Extract ID3 metadata tags
//! - `chromaprint_analyzer` - Generate audio fingerprints
//! - `acoustid_client` - Query AcoustID API for MBID candidates
//! - `musicbrainz_client` - Query MusicBrainz API for metadata
//! - `essentia_analyzer` - Extract musical flavor (when available)
//! - `audio_features` - Derive features from audio signal
//! - `genre_mapper` - Map ID3 genres to musical characteristics
//!
//! ## Tier 2: Confidence-Weighted Fusion (Explicit Synchronizations)
//! - `identity_resolver` - Bayesian fusion of MBID candidates
//! - `metadata_fuser` - Field-wise weighted selection
//! - `flavor_synthesizer` - Characteristic-wise weighted averaging
//! - `boundary_fuser` - Multi-strategy boundary detection fusion
//!
//! ## Tier 3: Quality Validation & Enrichment
//! - `consistency_checker` - Cross-source validation
//! - `completeness_scorer` - Quality scoring
//! - `conflict_detector` - Conflict detection and flagging
//!
//! ## Per-Song Workflow
//! - `workflow` - Sequential per-song processing engine
//! - `sse` - Real-time SSE event broadcasting
//!
//! Each module is a self-contained "concept" with explicit input/output contracts.

pub mod tier1;  // Source extractors (7 independent concepts) - 6/7 complete ✅
pub mod tier2;  // Fusion modules (4 concepts with explicit synchronizations) - 4/4 complete ✅
pub mod tier3;  // Validation modules (3 concepts) - 3/3 complete ✅
pub mod song_workflow_engine;  // Per-song sequential processing
pub mod session_orchestrator;  // Session-level workflow orchestration (PLAN024)
pub mod sse_broadcaster;  // SSE event broadcasting with throttling ✅
pub mod db_repository;  // Database repository for ProcessedPassage ✅

// Shared types and data contracts between tiers
pub mod types;

//! Album Matching Module
//!
//! Multi-stage algorithm for identifying full-album audio files against
//! MusicBrainz releases. Extracted from am28 prototype and integrated
//! into the wkmp-ai library.
//!
//! # Overview
//!
//! The album matcher uses a four-stage approach to match concatenated album
//! recordings to their source releases:
//!
//! - **Stage 2**: Parameter grid search - tests 180 combinations of silence
//!   detection parameters against candidate editions
//! - **Stage 3**: Over-segmentation assembly - uses dynamic programming to
//!   merge over-detected track boundaries
//! - **Stage 4**: Quiet spot detection - RMS-based boundary detection for
//!   albums without clear silences (25% penalty applied)
//! - **Stage 5**: Extra track merging - handles bonus/hidden tracks by
//!   merging final segments
//!
//! # Usage
//!
//! ```rust,ignore
//! use wkmp_ai::matching::{AlbumMatcher, AlbumMatcherConfig};
//! use std::path::Path;
//!
//! async fn match_album() -> Result<(), Box<dyn std::error::Error>> {
//!     let matcher = AlbumMatcher::new()?;
//!
//!     let result = matcher.match_album(
//!         Path::new("album.mp3"),
//!         None,  // artist hint
//!         None,  // album hint
//!     ).await?;
//!
//!     println!("Matched: {}", result.matched);
//!     println!("Match percentage: {:.1}%", result.match_percentage);
//!     println!("Stage: {:?}", result.matching_stage);
//!     println!("Artist verified: {}", result.artist_verified);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Configuration
//!
//! Key configuration options in [`AlbumMatcherConfig`]:
//!
//! | Parameter | Default | Description |
//! |-----------|---------|-------------|
//! | `match_tolerance_secs` | 10.0 | Track duration tolerance (seconds) |
//! | `min_artist_similarity` | 0.50 | Jaro-Winkler threshold for artist |
//! | `enable_early_exit` | true | Stop on 100% match |
//! | `early_exit_grace_secs` | 20 | Grace period after 100% match |
//! | `enable_stage3` | true | Enable over-segmentation assembly |
//! | `enable_stage4` | true | Enable quiet spot detection |
//! | `enable_stage5` | true | Enable extra track merging |
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                     AlbumMatcher                        │
//! ├─────────────────────────────────────────────────────────┤
//! │  1. Extract metadata (ID3/filename)                     │
//! │  2. Pre-decode single-track check                       │
//! │  3. Decode audio + MusicBrainz search (parallel)        │
//! │  4. Post-decode single-track check                      │
//! │  5. Group releases into editions                        │
//! │  6. Run multi-stage matching (Stages 2-5)               │
//! │  7. Verify artist match                                 │
//! │  8. Return AlbumMatchResult                             │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Module Structure
//!
//! - [`album_matcher`]: Main AlbumMatcher service
//! - [`types`]: Album matching result types (Edition, MatchedTrack, etc.)
//! - [`constants`]: Default parameters and thresholds
//! - [`metadata`]: ID3 tag and filename metadata extraction
//! - [`silence_detection`]: Silence-based track boundary detection
//! - [`single_track`]: Single-track file discriminator
//! - [`editions`]: MusicBrainz release grouping and filtering
//! - [`stages`]: Stage 2-5 implementations
//! - [`orchestrator`]: Multi-stage orchestration with early-exit

pub mod album_matcher;
pub mod constants;
pub mod editions;
pub mod metadata;
pub mod orchestrator;
pub mod silence_detection;
pub mod single_track;
pub mod stages;
pub mod types;

pub use album_matcher::{AlbumMatchError, AlbumMatcher, AlbumMatcherConfig};
pub use constants::*;
pub use types::{
    AlbumMatchResult,
    CacheConfig,
    // Cache types
    CacheMode,
    CacheStats,
    // Stage result types
    CandidateTestResult,
    Edition,
    EditionTestResult,
    // Metadata types
    ID3Metadata,
    MBMedia,
    MBRelease,
    MBReleaseDetails,
    // MusicBrainz types
    MBSearchResponse,
    MBTrack,
    MatchedTrack,
    MatchingStage,
    MetadataConfidence,
    MetadataSource,
    ReconciledMetadata,
    ReconciliationStrategy,
    SilenceCache,
    SingleTrackAnalysis,
    // Single-track types
    SingleTrackConfidence,
};

// Metadata extraction functions (PLAN030 Increment 3)
pub use metadata::{extract_and_reconcile_metadata, log_reconciliation_decision};

// Silence detection functions (PLAN030 Increment 4)
pub use silence_detection::{
    calculate_db, calculate_rms, compute_window_db_profile, detect_silence,
    find_silence_regions_from_profile, gaps_to_track_durations, get_track_durations,
    precompute_silence_cache, WindowDbProfile,
};

// Single-track discriminator (PLAN030 Increment 5)
pub use single_track::SingleTrackDiscriminator;

// Edition grouping and filtering (PLAN030 Increment 6)
pub use editions::{
    analyze_track_matching, calculate_name_distance, filter_and_sort_editions, group_into_editions,
    score_edition_match,
};

// Stage 2: Parameter Grid Search (PLAN030 Increment 7)
pub use stages::{run_stage2, EarlyExitConfig, Stage2Result};

// Stage 3: Over-Segmentation Assembly (PLAN030 Increment 8)
pub use stages::{run_stage3, Stage3Result};

// Stage 4: Quiet Spot Detection (PLAN030 Increment 9)
pub use stages::{run_stage4, RmsProfile, Stage4Result};

// Stage 5: Extra Track Merging (PLAN030 Increment 10)
pub use stages::{run_stage5, Stage5Result};

// Stage Orchestration (PLAN030 Increment 11)
pub use orchestrator::{run_orchestration, OrchestrationResult, OrchestratorConfig, StageResults};

//! Album Matching Module
//!
//! **[PLAN026/PLAN030]** Album matching via am28 algorithm
//!
//! Provides album matching functionality extracted from am28 example.
//! Integrates with ContentTypeClassifier for album identification.
//!
//! # Module Structure
//! - `album_matcher`: Main AlbumMatcher service
//! - `types`: Album matching result types
//! - `constants`: Default parameters and thresholds
//!
//! # Stage Flow (from am28)
//! - Stage 2: Silence detection parameter grid search (180 combinations)
//! - Stage 3: Dynamic programming assembly for over-segmented tracks
//! - Stage 4: Edition-guided quiet spot detection
//! - Stage 5: Extra track merging

pub mod album_matcher;
pub mod constants;
pub mod editions;
pub mod metadata;
pub mod silence_detection;
pub mod single_track;
pub mod types;

pub use album_matcher::{AlbumMatcher, AlbumMatcherConfig, AlbumMatchError};
pub use constants::*;
pub use types::{
    AlbumMatchResult, Edition, MatchedTrack, MatchingStage,
    // Cache types
    CacheMode, CacheConfig, CacheStats,
    // MusicBrainz types
    MBSearchResponse, MBRelease, MBReleaseDetails, MBMedia, MBTrack,
    // Stage result types
    CandidateTestResult, EditionTestResult, SilenceCache,
    // Metadata types
    ID3Metadata, ReconciledMetadata, ReconciliationStrategy, MetadataConfidence, MetadataSource,
    // Single-track types
    SingleTrackConfidence, SingleTrackAnalysis,
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
    analyze_track_matching, calculate_name_distance, filter_and_sort_editions,
    group_into_editions, score_edition_match,
};

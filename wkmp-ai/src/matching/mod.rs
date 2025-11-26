//! Album Matching Module
//!
//! **[PLAN026]** Increment 5: Album path integration
//!
//! Provides album matching functionality extracted from am28 example.
//! Integrates with ContentTypeClassifier for album identification.
//!
//! # Module Structure
//! - `album_matcher`: Main AlbumMatcher service
//! - `types`: Album matching result types
//!
//! # Stage Flow (from am28)
//! - Stage 2: Silence detection parameter grid search
//! - Stage 3: Dynamic programming assembly for over-segmented tracks
//! - Stage 4: Edition-guided quiet spot detection
//! - Stage 5: Extra track merging

pub mod album_matcher;
pub mod types;

pub use album_matcher::AlbumMatcher;
pub use types::{
    AlbumMatchResult, Edition, MatchedTrack, MatchingStage,
};

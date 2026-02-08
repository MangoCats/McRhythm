//! Edition Grouping and Filtering
//!
//! **[PLAN030]** Groups MusicBrainz releases into "editions" - unique combinations
//! of track count and duration pattern. Provides scoring and filtering for
//! selecting the best matching edition.
//!
//! ## Modules
//! - `grouping`: Groups releases by track count/duration signature
//! - `scoring`: Scores how well editions match detected durations
//! - `filtering`: Filters and sorts editions by relevance

pub mod filtering;
pub mod grouping;
pub mod scoring;

pub use filtering::{
    calculate_name_distance, filter_and_sort_editions, filter_editions_by_artist,
    filter_editions_by_file_duration,
};
pub use grouping::group_into_editions;

// PLAN030 functions
pub use scoring::{analyze_track_matching, score_edition_match};

// PLAN027 Edition Selection functions
#[allow(deprecated)] // Re-export deprecated function for backwards compatibility
pub use scoring::{
    calculate_edition_score, calculate_total_duration_score, calculate_total_duration_score_validated,
    calculate_track_count_penalty, calculate_track_quality_score, select_best_edition, EditionCandidate,
};

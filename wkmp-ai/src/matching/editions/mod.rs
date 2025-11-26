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

pub use filtering::{calculate_name_distance, filter_and_sort_editions};
pub use grouping::group_into_editions;
pub use scoring::{analyze_track_matching, score_edition_match};

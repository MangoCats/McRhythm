//! # MBID Selection Logic
//!
//! Selects the best MusicBrainz ID (MBID) from an edition using metadata prioritization.
//!
//! ## Features
//! - **Priority scoring**: CD releases, US country, Official status get bonuses
//! - **Score minimization**: Lower scores = higher priority
//! - **Fallback handling**: Returns first MBID if only one available, empty string if none
//!
//! ## Scoring System
//! Lower scores indicate higher priority (minimize score):
//! - CD release: -50 points (SCORE_CD_BONUS)
//! - Official status: -40 points (SCORE_OFFICIAL_BONUS)
//! - US country: -30 points (SCORE_US_BONUS)
//! - Base score: 0
//!
//! ## Example
//! ```ignore
//! Edition with 3 MBIDs:
//! - MBID1: CD=false, country=None, status=None         → score = 0
//! - MBID2: CD=true,  country=Some("US"), status=Some("Official") → score = -120
//! - MBID3: CD=true,  country=Some("GB"), status=Some("Official") → score = -90
//! Winner: MBID2 (lowest score = -120)
//! ```
//!
//! ## Related Modules
//! - `types`: Edition, EditionMBID
//! - `constants`: SCORE_CD_BONUS, SCORE_US_BONUS, SCORE_OFFICIAL_BONUS
//! - `edition`: cmp_f64() for score comparison

use crate::constants::*;
use crate::types::{Edition, EditionMBID};
use crate::matching::edition::cmp_f64;

// =============================================================================
// MBID Priority Scoring
// =============================================================================

/// Calculate priority score for MBID selection
///
/// Lower scores indicate higher priority (minimize score).
/// Bonuses are negative values to reduce the score.
///
/// # Arguments
/// * `is_cd` - Whether this is a CD release
/// * `country` - Release country code (e.g., "US", "GB")
/// * `status` - Release status (e.g., "Official", "Bootleg")
///
/// # Returns
/// Priority score (lower = better)
///
/// # Scoring Rules
/// - CD release: -50 points (SCORE_CD_BONUS)
/// - US country: -30 points (SCORE_US_BONUS)
/// - Official status: -40 points (SCORE_OFFICIAL_BONUS)
/// - Base: 0 points
///
/// # Example
/// ```ignore
/// // CD + US + Official = -50 + -30 + -40 = -120
/// let score = calculate_mbid_priority_score(
///     true,
///     Some("US"),
///     Some("Official")
/// );
/// assert_eq!(score, -120.0);
/// ```
pub(crate) fn calculate_mbid_priority_score(
    is_cd: bool,
    country: Option<&str>,
    status: Option<&str>,
) -> f64 {
    let mut score = 0.0;

    if is_cd {
        score += SCORE_CD_BONUS;
    }

    if country == Some("US") {
        score += SCORE_US_BONUS;
    }

    if status == Some("Official") {
        score += SCORE_OFFICIAL_BONUS;
    }

    score
}

// =============================================================================
// MBID Selection
// =============================================================================

/// Select best MBID from an edition using metadata prioritization
///
/// Evaluates all MBIDs in an edition and returns the one with the lowest
/// priority score (highest priority). If only one MBID exists, returns it
/// directly. If no MBIDs exist, returns empty string.
///
/// # Arguments
/// * `edition` - Edition containing one or more MBIDs with metadata
///
/// # Returns
/// Selected MBID string, or empty string if no MBIDs available
///
/// # Algorithm
/// 1. If no MBIDs → return empty string
/// 2. If one MBID → return it directly
/// 3. If multiple MBIDs:
///    - Score each MBID using calculate_mbid_priority_score()
///    - Sort by score (ascending = lowest/best first)
///    - Return MBID with lowest score
///
/// # Example
/// ```ignore
/// let edition = Edition {
///     mbids: vec![
///         EditionMBID {
///             mbid: "mbid-1".to_string(),
///             is_cd: false,
///             country: None,
///             status: None,
///         },
///         EditionMBID {
///             mbid: "mbid-2".to_string(),
///             is_cd: true,
///             country: Some("US".to_string()),
///             status: Some("Official".to_string()),
///         },
///     ],
///     // ... other fields
/// };
/// let best = select_best_mbid(&edition);
/// assert_eq!(best, "mbid-2");  // CD + US + Official = -120 < 0
/// ```
pub(crate) fn select_best_mbid(edition: &Edition) -> String {
    if edition.mbids.is_empty() {
        return String::new();
    }

    if edition.mbids.len() == 1 {
        return edition.mbids[0].mbid.clone();
    }

    // Score each MBID using the established criteria
    let mut scored: Vec<(&EditionMBID, f64)> = edition.mbids
        .iter()
        .map(|mbid_info| {
            let score = calculate_mbid_priority_score(
                mbid_info.is_cd,
                mbid_info.country.as_deref(),
                mbid_info.status.as_deref(),
            );
            (mbid_info, score)
        })
        .collect();

    // Sort by score (ascending = best first)
    scored.sort_by(|a, b| cmp_f64(a.1, b.1));

    scored[0].0.mbid.clone()
}

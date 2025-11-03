//! Passage database queries
//!
//! Read passage timing data and metadata from the database.
//! Implements Phase 2 validation strategy from crossfade.md.
//!
//! **Traceability:**
//! - DB-PASSAGE-010 (Passage table schema)
//! - XFD-IMPL-040 (Passage timing validation)
//! - REQ-DEF-035 (Ephemeral passage support)

use crate::error::{Error, Result};
use sqlx::{Pool, Row, Sqlite};
use std::path::PathBuf;
use uuid::Uuid;
use wkmp_common::FadeCurve;

/// Passage with all timing points resolved
///
/// All timing values are in ticks (28,224,000 Hz).
/// NULL values from database converted to appropriate defaults.
///
/// **Traceability:** SSD-DEC-020, SRC-TICK-020
#[derive(Debug, Clone)]
pub struct PassageWithTiming {
    pub passage_id: Option<Uuid>,
    pub file_path: PathBuf,
    pub start_time_ticks: i64,
    pub end_time_ticks: Option<i64>, // None = file end
    pub lead_in_point_ticks: i64,
    pub lead_out_point_ticks: Option<i64>, // None = calculated from global setting
    pub fade_in_point_ticks: i64,
    pub fade_out_point_ticks: Option<i64>, // None = calculated from global setting
    pub fade_in_curve: FadeCurve,
    pub fade_out_curve: FadeCurve,
}

/// Get passage by ID with timing information
///
/// Returns passage with all timing points. Performs Phase 2 validation
/// (correct invalid values, log warnings).
///
/// **Traceability:** DB-PASSAGE-020
pub async fn get_passage_with_timing(
    db: &Pool<Sqlite>,
    passage_id: Uuid,
) -> Result<PassageWithTiming> {
    // Query passage from database
    let row = sqlx::query(
        r#"
        SELECT p.guid, f.path, p.start_time, p.end_time,
               p.lead_in_point, p.lead_out_point,
               p.fade_in_point, p.fade_out_point,
               p.fade_in_curve, p.fade_out_curve,
               f.duration
        FROM passages p
        JOIN files f ON p.file_id = f.guid
        WHERE p.guid = ?
        "#,
    )
    .bind(passage_id.to_string())
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::PassageNotFound(passage_id.to_string()))?;

    // Extract values (all timing in seconds from DB, convert to ticks)
    let file_path = PathBuf::from(row.get::<String, _>("path"));
    let file_duration_s: Option<f64> = row.get("duration");

    // Convert seconds to ticks (28,224,000 Hz)
    // 1 second = 28,224,000 ticks
    let start_time_ticks = row
        .get::<Option<f64>, _>("start_time")
        .map(wkmp_common::timing::seconds_to_ticks)
        .unwrap_or(0);

    let end_time_ticks = match (row.get::<Option<f64>, _>("end_time"), file_duration_s) {
        (Some(end), _) => Some(wkmp_common::timing::seconds_to_ticks(end)),
        (None, Some(duration)) => Some(wkmp_common::timing::seconds_to_ticks(duration)),
        (None, None) => None, // File duration unknown
    };

    let lead_in_point_ticks = row
        .get::<Option<f64>, _>("lead_in_point")
        .map(wkmp_common::timing::seconds_to_ticks)
        .unwrap_or(start_time_ticks);

    let lead_out_point_ticks = row
        .get::<Option<f64>, _>("lead_out_point")
        .map(wkmp_common::timing::seconds_to_ticks);

    let fade_in_point_ticks = row
        .get::<Option<f64>, _>("fade_in_point")
        .map(wkmp_common::timing::seconds_to_ticks)
        .unwrap_or(start_time_ticks);

    let fade_out_point_ticks = row
        .get::<Option<f64>, _>("fade_out_point")
        .map(wkmp_common::timing::seconds_to_ticks);

    // Parse fade curves
    let fade_in_curve = row
        .get::<Option<String>, _>("fade_in_curve")
        .and_then(|s| FadeCurve::from_str(&s))
        .unwrap_or(FadeCurve::Exponential); // Default

    let fade_out_curve = row
        .get::<Option<String>, _>("fade_out_curve")
        .and_then(|s| FadeCurve::from_str(&s))
        .unwrap_or(FadeCurve::Logarithmic); // Default

    let passage = PassageWithTiming {
        passage_id: Some(passage_id),
        file_path,
        start_time_ticks,
        end_time_ticks,
        lead_in_point_ticks,
        lead_out_point_ticks,
        fade_in_point_ticks,
        fade_out_point_ticks,
        fade_in_curve,
        fade_out_curve,
    };

    // Apply Phase 2 validation
    validate_passage_timing(passage)
}

/// Get audio file path for a passage
///
/// **Traceability:** DB-PASSAGE-030
///
/// **Phase 4:** Standalone path query reserved for future features (superseded by get_passage_with_timing)
#[allow(dead_code)]
pub async fn get_audio_file_path(db: &Pool<Sqlite>, passage_id: Uuid) -> Result<PathBuf> {
    let path: String = sqlx::query_scalar(
        r#"
        SELECT f.path
        FROM passages p
        JOIN files f ON p.file_id = f.guid
        WHERE p.guid = ?
        "#,
    )
    .bind(passage_id.to_string())
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::PassageNotFound(passage_id.to_string()))?;

    Ok(PathBuf::from(path))
}

/// Create an ephemeral passage for ad-hoc playback
///
/// Creates a temporary passage definition from just a file path.
/// Used for immediate playback without database persistence.
/// All timing points default to zero (no fade, no lead).
///
/// **Traceability:** REQ-DEF-035 (Ephemeral passage), SRC-TICK-020
pub fn create_ephemeral_passage(file_path: PathBuf) -> PassageWithTiming {
    PassageWithTiming {
        passage_id: None, // Ephemeral = no database ID
        file_path,
        start_time_ticks: 0,
        end_time_ticks: None, // Will be determined during decode
        lead_in_point_ticks: 0,
        lead_out_point_ticks: None, // Will use global crossfade time
        fade_in_point_ticks: 0,
        fade_out_point_ticks: None, // Will use global crossfade time
        fade_in_curve: FadeCurve::Exponential,
        fade_out_curve: FadeCurve::Logarithmic,
    }
}

/// Validate and correct passage timing
///
/// Implements Phase 2 validation strategy from crossfade.md:
/// - Correct invalid values
/// - Log warnings for corrections
/// - Never fail on invalid data
///
/// **Traceability:** XFD-IMPL-040
pub fn validate_passage_timing(mut passage: PassageWithTiming) -> Result<PassageWithTiming> {
    // Validation happens in-order following crossfade.md spec

    // Step 1: Validate start < end (if end is known)
    if let Some(end) = passage.end_time_ticks {
        if passage.start_time_ticks >= end {
            tracing::warn!(
                "Passage {:?}: Invalid start/end times (start={}, end={}). \
                 Setting start=0.",
                passage.passage_id,
                passage.start_time_ticks,
                end
            );
            passage.start_time_ticks = 0;
        }
    }

    // Step 2: Validate fade-in point
    if let Some(end) = passage.end_time_ticks {
        if passage.fade_in_point_ticks < passage.start_time_ticks {
            tracing::warn!(
                "Passage {:?}: fade_in_point ({}) < start_time ({}). \
                 Clamping to start_time.",
                passage.passage_id,
                passage.fade_in_point_ticks,
                passage.start_time_ticks
            );
            passage.fade_in_point_ticks = passage.start_time_ticks;
        }

        if passage.fade_in_point_ticks > end {
            tracing::warn!(
                "Passage {:?}: fade_in_point ({}) > end_time ({}). \
                 Clamping to end_time.",
                passage.passage_id,
                passage.fade_in_point_ticks,
                end
            );
            passage.fade_in_point_ticks = end;
        }
    }

    // Step 3: Validate lead-in point
    if let Some(end) = passage.end_time_ticks {
        if passage.lead_in_point_ticks < passage.start_time_ticks {
            tracing::warn!(
                "Passage {:?}: lead_in_point ({}) < start_time ({}). \
                 Clamping to start_time.",
                passage.passage_id,
                passage.lead_in_point_ticks,
                passage.start_time_ticks
            );
            passage.lead_in_point_ticks = passage.start_time_ticks;
        }

        if passage.lead_in_point_ticks > end {
            tracing::warn!(
                "Passage {:?}: lead_in_point ({}) > end_time ({}). \
                 Clamping to end_time.",
                passage.passage_id,
                passage.lead_in_point_ticks,
                end
            );
            passage.lead_in_point_ticks = end;
        }
    }

    // Step 4: Validate lead-out point (if specified)
    if let Some(lead_out) = passage.lead_out_point_ticks {
        if let Some(end) = passage.end_time_ticks {
            if lead_out < passage.start_time_ticks {
                tracing::warn!(
                    "Passage {:?}: lead_out_point ({}) < start_time ({}). \
                     Clamping to start_time.",
                    passage.passage_id,
                    lead_out,
                    passage.start_time_ticks
                );
                passage.lead_out_point_ticks = Some(passage.start_time_ticks);
            }

            if lead_out > end {
                tracing::warn!(
                    "Passage {:?}: lead_out_point ({}) > end_time ({}). \
                     Clamping to end_time.",
                    passage.passage_id,
                    lead_out,
                    end
                );
                passage.lead_out_point_ticks = Some(end);
            }
        }

        // Validate lead-out >= lead-in
        if let Some(corrected_lead_out) = passage.lead_out_point_ticks {
            if corrected_lead_out < passage.lead_in_point_ticks {
                tracing::warn!(
                    "Passage {:?}: lead_out_point ({}) < lead_in_point ({}). \
                     Setting lead_out = lead_in.",
                    passage.passage_id,
                    corrected_lead_out,
                    passage.lead_in_point_ticks
                );
                passage.lead_out_point_ticks = Some(passage.lead_in_point_ticks);
            }
        }
    }

    // Step 5: Validate fade-out point (if specified)
    if let Some(fade_out) = passage.fade_out_point_ticks {
        if let Some(end) = passage.end_time_ticks {
            if fade_out < passage.start_time_ticks {
                tracing::warn!(
                    "Passage {:?}: fade_out_point ({}) < start_time ({}). \
                     Clamping to start_time.",
                    passage.passage_id,
                    fade_out,
                    passage.start_time_ticks
                );
                passage.fade_out_point_ticks = Some(passage.start_time_ticks);
            }

            if fade_out > end {
                tracing::warn!(
                    "Passage {:?}: fade_out_point ({}) > end_time ({}). \
                     Clamping to end_time.",
                    passage.passage_id,
                    fade_out,
                    end
                );
                passage.fade_out_point_ticks = Some(end);
            }
        }

        // Validate fade-out >= fade-in
        if let Some(corrected_fade_out) = passage.fade_out_point_ticks {
            if corrected_fade_out < passage.fade_in_point_ticks {
                tracing::warn!(
                    "Passage {:?}: fade_out_point ({}) < fade_in_point ({}). \
                     Setting fade_out = fade_in.",
                    passage.passage_id,
                    corrected_fade_out,
                    passage.fade_in_point_ticks
                );
                passage.fade_out_point_ticks = Some(passage.fade_in_point_ticks);
            }
        }
    }

    Ok(passage)
}

/// Get album UUIDs associated with a passage
///
/// Returns a vector of album UUIDs that are linked to the passage via the
/// `passage_albums` table. Returns empty vector if no albums are associated.
///
/// **[REQ-DEBT-FUNC-003]** Album metadata population for PassageStarted/Complete events
///
/// # Arguments
/// * `db` - Database connection pool
/// * `passage_id` - UUID of the passage to query
///
/// # Returns
/// Vector of album UUIDs (may be empty)
pub async fn get_passage_album_uuids(
    db: &Pool<Sqlite>,
    passage_id: Uuid,
) -> Result<Vec<Uuid>> {
    let rows = sqlx::query(
        r#"
        SELECT album_id
        FROM passage_albums
        WHERE passage_id = ?
        ORDER BY created_at
        "#,
    )
    .bind(passage_id.to_string())
    .fetch_all(db)
    .await?;

    let album_uuids: Vec<Uuid> = rows
        .iter()
        .filter_map(|row| {
            row.get::<String, _>("album_id")
                .parse::<Uuid>()
                .ok()
        })
        .collect();

    Ok(album_uuids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fade_curve_conversion() {
        assert_eq!(FadeCurve::from_str("linear"), Some(FadeCurve::Linear));
        assert_eq!(
            FadeCurve::from_str("exponential"),
            Some(FadeCurve::Exponential)
        );
        assert_eq!(
            FadeCurve::from_str("logarithmic"),
            Some(FadeCurve::Logarithmic)
        );
        assert_eq!(FadeCurve::from_str("cosine"), Some(FadeCurve::SCurve));
        assert_eq!(FadeCurve::from_str("invalid"), None);

        assert_eq!(FadeCurve::Linear.to_db_string(), "linear");
        assert_eq!(FadeCurve::Exponential.to_db_string(), "exponential");
    }

    #[test]
    fn test_ephemeral_passage_creation() {
        let path = PathBuf::from("/test/audio.mp3");
        let passage = create_ephemeral_passage(path.clone());

        assert_eq!(passage.passage_id, None);
        assert_eq!(passage.file_path, path);
        assert_eq!(passage.start_time_ticks, 0);
        assert_eq!(passage.end_time_ticks, None);
        assert_eq!(passage.lead_in_point_ticks, 0);
        assert_eq!(passage.lead_out_point_ticks, None);
    }

    #[test]
    fn test_passage_timing_validation_start_end() {
        use wkmp_common::timing::ms_to_ticks;

        // Test invalid start >= end
        let passage = PassageWithTiming {
            passage_id: Some(Uuid::new_v4()),
            file_path: PathBuf::from("/test.mp3"),
            start_time_ticks: ms_to_ticks(5000),
            end_time_ticks: Some(ms_to_ticks(3000)), // Invalid: start > end
            lead_in_point_ticks: ms_to_ticks(5000),
            lead_out_point_ticks: Some(ms_to_ticks(3000)),
            fade_in_point_ticks: ms_to_ticks(5000),
            fade_out_point_ticks: Some(ms_to_ticks(3000)),
            fade_in_curve: FadeCurve::Linear,
            fade_out_curve: FadeCurve::Linear,
        };

        let validated = validate_passage_timing(passage).unwrap();
        assert_eq!(validated.start_time_ticks, 0); // Corrected to 0
    }

    #[test]
    fn test_passage_timing_validation_fade_points() {
        use wkmp_common::timing::ms_to_ticks;

        // Test fade points outside bounds
        let passage = PassageWithTiming {
            passage_id: Some(Uuid::new_v4()),
            file_path: PathBuf::from("/test.mp3"),
            start_time_ticks: ms_to_ticks(1000),
            end_time_ticks: Some(ms_to_ticks(10000)),
            lead_in_point_ticks: ms_to_ticks(2000),
            lead_out_point_ticks: Some(ms_to_ticks(9000)),
            fade_in_point_ticks: ms_to_ticks(500), // Before start
            fade_out_point_ticks: Some(ms_to_ticks(15000)), // After end
            fade_in_curve: FadeCurve::Linear,
            fade_out_curve: FadeCurve::Linear,
        };

        let validated = validate_passage_timing(passage).unwrap();
        assert_eq!(validated.fade_in_point_ticks, ms_to_ticks(1000)); // Clamped to start
        assert_eq!(validated.fade_out_point_ticks, Some(ms_to_ticks(10000))); // Clamped to end
    }

    #[test]
    fn test_passage_timing_validation_lead_ordering() {
        use wkmp_common::timing::ms_to_ticks;

        // Test lead-out < lead-in
        let passage = PassageWithTiming {
            passage_id: Some(Uuid::new_v4()),
            file_path: PathBuf::from("/test.mp3"),
            start_time_ticks: 0,
            end_time_ticks: Some(ms_to_ticks(10000)),
            lead_in_point_ticks: ms_to_ticks(5000),
            lead_out_point_ticks: Some(ms_to_ticks(3000)), // Before lead-in (invalid)
            fade_in_point_ticks: 0,
            fade_out_point_ticks: Some(ms_to_ticks(10000)),
            fade_in_curve: FadeCurve::Linear,
            fade_out_curve: FadeCurve::Linear,
        };

        let validated = validate_passage_timing(passage).unwrap();
        assert_eq!(validated.lead_out_point_ticks, Some(ms_to_ticks(5000))); // Corrected to lead-in
    }
}

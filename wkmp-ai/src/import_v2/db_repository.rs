// PLAN023: Database Repository for ProcessedPassage
//
// Provides functions to save ProcessedPassage data to the database.
// Handles serialization of complex types (MusicalFlavor, ResolvedIdentity, etc.)
// into the 21 PLAN023 columns added by migration v3.
//
// Requirements: REQ-AI-081 through REQ-AI-087

use crate::import_v2::types::{ExtractionSource, ProcessedPassage};
use serde_json::json;
use sqlx::SqlitePool;
use tracing::info;
use uuid::Uuid;

/// Database repository for PLAN023 import data
pub struct ImportRepository {
    pool: SqlitePool,
}

impl ImportRepository {
    /// Create new repository with database pool
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Save processed passage to database
    ///
    /// **[REQ-AI-081 through REQ-AI-087]** Store complete provenance data
    ///
    /// Inserts a new passage record with all PLAN023 columns populated.
    /// Uses a transaction to ensure atomicity with provenance log entries.
    pub async fn save_processed_passage(
        &self,
        file_id: &Uuid,
        processed: &ProcessedPassage,
        import_session_id: &Uuid,
    ) -> Result<Uuid, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let passage_id = Uuid::new_v4();

        // Serialize complex types to JSON
        let identity_conflicts_json = self.serialize_identity_candidates(processed);

        let flavor_source_blend_json = self.serialize_flavor_sources(processed);
        let musical_flavor_json = self.serialize_musical_flavor(processed);
        let validation_report_json = self.serialize_validation(processed);

        // Insert passage with all PLAN023 columns
        sqlx::query(
            r#"
            INSERT INTO passages (
                guid, file_id, start_time_ticks, end_time_ticks,
                recording_mbid, identity_confidence, identity_conflicts,
                title, title_source, title_confidence,
                artist, artist_source, artist_confidence,
                album, album_source, album_confidence,
                musical_flavor_vector, flavor_source_blend, flavor_confidence_map,
                overall_quality_score, metadata_completeness, flavor_completeness,
                validation_status, validation_report,
                import_session_id, import_timestamp, import_strategy,
                import_duration_ms, import_version
            )
            VALUES (
                ?, ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?,
                ?, ?, ?,
                ?, ?
            )
            "#,
        )
        .bind(passage_id.to_string())
        .bind(file_id.to_string())
        .bind(processed.boundary.start_ticks)
        .bind(processed.boundary.end_ticks)
        // Identity (REQ-AI-083)
        .bind(processed.identity.mbid.as_ref().map(|u| u.to_string()))
        .bind(processed.identity.confidence)
        .bind(&identity_conflicts_json)
        // Metadata (REQ-AI-082)
        .bind(processed.metadata.title.as_ref().map(|f| &f.value))
        .bind(processed.metadata.title.as_ref().map(|f| source_to_string(&f.source)))
        .bind(processed.metadata.title.as_ref().map(|f| f.confidence))
        .bind(processed.metadata.artist.as_ref().map(|f| &f.value))
        .bind(processed.metadata.artist.as_ref().map(|f| source_to_string(&f.source)))
        .bind(processed.metadata.artist.as_ref().map(|f| f.confidence))
        .bind(processed.metadata.album.as_ref().map(|f| &f.value))
        .bind(processed.metadata.album.as_ref().map(|f| source_to_string(&f.source)))
        .bind(processed.metadata.album.as_ref().map(|f| f.confidence))
        // Flavor (REQ-AI-081)
        .bind(&musical_flavor_json)
        .bind(&flavor_source_blend_json)
        .bind(self.serialize_flavor_confidence(processed))
        // Validation (REQ-AI-084, REQ-AI-085)
        .bind(processed.validation.quality_score)
        .bind(self.calculate_metadata_completeness(processed))
        .bind(processed.flavor.flavor_completeness)
        .bind(self.validation_status(&processed.validation))
        .bind(&validation_report_json)
        // Import metadata (REQ-AI-086)
        .bind(import_session_id.to_string())
        .bind(chrono::Utc::now().timestamp())
        .bind("HybridFusion")
        .bind(processed.import_duration_ms as i64)
        .bind(&processed.import_version)
        .execute(&mut *tx)
        .await?;

        // Create import_provenance entries (REQ-AI-087)
        self.create_provenance_entries(&mut tx, &passage_id, processed)
            .await?;

        tx.commit().await?;

        info!(
            "Saved passage {} (file {}) with PLAN023 provenance",
            passage_id, file_id
        );

        Ok(passage_id)
    }

    /// Create import_provenance log entries
    async fn create_provenance_entries(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        passage_id: &Uuid,
        processed: &ProcessedPassage,
    ) -> Result<(), sqlx::Error> {
        let timestamp = chrono::Utc::now().timestamp();

        // Log identity candidates
        for candidate in &processed.identity.candidates {
            let sources_json = serde_json::to_string(&candidate.sources)
                .unwrap_or_else(|_| "[]".to_string());

            sqlx::query(
                r#"
                INSERT INTO import_provenance (
                    id, passage_id, source_type, data_extracted, confidence, timestamp
                )
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(passage_id.to_string())
            .bind("MBIDCandidate")
            .bind(json!({ "mbid": candidate.mbid, "sources": sources_json }).to_string())
            .bind(candidate.confidence)
            .bind(timestamp)
            .execute(&mut **tx)
            .await?;
        }

        // Log flavor sources
        for source in &processed.flavor.sources_used {
            sqlx::query(
                r#"
                INSERT INTO import_provenance (
                    id, passage_id, source_type, data_extracted, confidence, timestamp
                )
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(passage_id.to_string())
            .bind(source_to_string(source))
            .bind("{}".to_string()) // Placeholder for flavor data
            .bind(processed.flavor.flavor_confidence)
            .bind(timestamp)
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    /// Serialize musical flavor to JSON
    fn serialize_musical_flavor(&self, processed: &ProcessedPassage) -> String {
        serde_json::to_string(&processed.flavor.flavor).unwrap_or_else(|_| "{}".to_string())
    }

    /// Serialize flavor source blend (array of sources)
    fn serialize_flavor_sources(&self, processed: &ProcessedPassage) -> String {
        let sources: Vec<String> = processed
            .flavor
            .sources_used
            .iter()
            .map(source_to_string)
            .collect();
        serde_json::to_string(&sources).unwrap_or_else(|_| "[]".to_string())
    }

    /// Serialize flavor confidence (overall confidence for now)
    fn serialize_flavor_confidence(&self, processed: &ProcessedPassage) -> String {
        // Simple approach: store overall confidence
        json!({ "overall": processed.flavor.flavor_confidence }).to_string()
    }

    /// Serialize validation report
    fn serialize_validation(&self, processed: &ProcessedPassage) -> String {
        json!({
            "quality_score": processed.validation.quality_score,
            "has_conflicts": processed.validation.has_conflicts,
            "warnings": processed.validation.warnings,
            "conflicts": processed.validation.conflicts.iter().map(|(msg, sev)| {
                json!({ "message": msg, "severity": format!("{:?}", sev) })
            }).collect::<Vec<_>>()
        })
        .to_string()
    }

    /// Calculate metadata completeness percentage
    fn calculate_metadata_completeness(&self, processed: &ProcessedPassage) -> f64 {
        let mut filled = 0.0;
        let total = 6.0; // title, artist, album, release_date, track_number, duration_ms

        if processed.metadata.title.is_some() {
            filled += 1.0;
        }
        if processed.metadata.artist.is_some() {
            filled += 1.0;
        }
        if processed.metadata.album.is_some() {
            filled += 1.0;
        }
        if processed.metadata.release_date.is_some() {
            filled += 1.0;
        }
        if processed.metadata.track_number.is_some() {
            filled += 1.0;
        }
        if processed.metadata.duration_ms.is_some() {
            filled += 1.0;
        }

        filled / total
    }

    /// Determine validation status string
    fn validation_status(&self, validation: &crate::import_v2::types::ValidationReport) -> String {
        if validation.has_conflicts {
            "Fail".to_string()
        } else if !validation.warnings.is_empty() {
            "Warning".to_string()
        } else if validation.quality_score >= 0.8 {
            "Pass".to_string()
        } else {
            "Warning".to_string()
        }
    }

    /// Serialize identity candidates to JSON
    fn serialize_identity_candidates(&self, processed: &ProcessedPassage) -> String {
        let candidates_json: Vec<serde_json::Value> = processed
            .identity
            .candidates
            .iter()
            .map(|c| {
                let sources: Vec<String> = c.sources.iter().map(source_to_string).collect();
                json!({
                    "mbid": c.mbid.to_string(),
                    "confidence": c.confidence,
                    "sources": sources
                })
            })
            .collect();

        serde_json::to_string(&candidates_json).unwrap_or_else(|_| "[]".to_string())
    }
}

/// Convert ExtractionSource to string
fn source_to_string(source: &ExtractionSource) -> String {
    match source {
        ExtractionSource::ID3Metadata => "ID3Metadata".to_string(),
        ExtractionSource::Chromaprint => "Chromaprint".to_string(),
        ExtractionSource::AcoustID => "AcoustID".to_string(),
        ExtractionSource::MusicBrainz => "MusicBrainz".to_string(),
        ExtractionSource::Essentia => "Essentia".to_string(),
        ExtractionSource::AudioDerived => "AudioDerived".to_string(),
        ExtractionSource::GenreMapping => "GenreMapping".to_string(),
        ExtractionSource::AcousticBrainzArchive => "AcousticBrainzArchive".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_v2::types::{
        BoundaryDetectionMethod, ConflictSeverity, FusedMetadata, MetadataField, MusicalFlavor,
        PassageBoundary, ResolvedIdentity, SynthesizedFlavor, ValidationReport,
    };

    #[test]
    fn test_source_to_string() {
        assert_eq!(
            source_to_string(&ExtractionSource::ID3Metadata),
            "ID3Metadata"
        );
        assert_eq!(source_to_string(&ExtractionSource::AcoustID), "AcoustID");
        assert_eq!(
            source_to_string(&ExtractionSource::MusicBrainz),
            "MusicBrainz"
        );
    }

    #[tokio::test]
    async fn test_validation_status() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .unwrap();
        let repo = ImportRepository { pool };

        let validation_pass = ValidationReport {
            quality_score: 0.9,
            has_conflicts: false,
            warnings: vec![],
            conflicts: vec![],
        };

        assert_eq!(repo.validation_status(&validation_pass), "Pass");

        let validation_warning = ValidationReport {
            quality_score: 0.9,
            has_conflicts: false,
            warnings: vec!["Missing album".to_string()],
            conflicts: vec![],
        };

        assert_eq!(repo.validation_status(&validation_warning), "Warning");

        let validation_fail = ValidationReport {
            quality_score: 0.5,
            has_conflicts: true,
            warnings: vec![],
            conflicts: vec![("Conflict".to_string(), ConflictSeverity::High)],
        };

        assert_eq!(repo.validation_status(&validation_fail), "Fail");
    }
}

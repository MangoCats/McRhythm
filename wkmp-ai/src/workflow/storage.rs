// Database Storage for Processed Passages
//
// PLAN023: REQ-AI-081 through REQ-AI-087 - Write FusionResult to database with provenance
// SPEC017 Compliance: Stores passage times as INTEGER ticks, not REAL seconds

use super::{ProcessedPassage, TICK_RATE};
use anyhow::{Context, Result};
use sqlx::SqlitePool;
use uuid::Uuid;
use tracing::{debug, info};

/// Store a processed passage in the database
///
/// # Arguments
/// * `db` - Database connection pool
/// * `file_path` - Source audio file path
/// * `passage` - Processed passage with fusion and validation results
/// * `import_session_id` - UUID for this import session
///
/// # Returns
/// * Passage UUID (GUID)
pub async fn store_passage(
    db: &SqlitePool,
    file_path: &str,
    passage: &ProcessedPassage,
    import_session_id: &str,
) -> Result<String> {
    let passage_id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp();

    // Extract fusion result data
    let fusion = &passage.fusion;
    let validation = &passage.validation;

    // Serialize JSON fields
    let flavor_json = serde_json::to_string(&fusion.flavor.characteristics)
        .context("Failed to serialize flavor characteristics")?;

    let flavor_source_blend = serde_json::to_string(&fusion.flavor.source_blend)
        .context("Failed to serialize flavor source blend")?;

    let flavor_confidence_map = serde_json::to_string(&fusion.flavor.confidence_map)
        .context("Failed to serialize flavor confidence map")?;

    let identity_conflicts = if !fusion.identity.conflicts.is_empty() {
        Some(serde_json::to_string(&fusion.identity.conflicts)?)
    } else {
        None
    };

    let validation_report = serde_json::to_string(&validation.checks)
        .context("Failed to serialize validation report")?;

    // Insert passage record (SPEC017: times as INTEGER ticks)
    let query = r#"
        INSERT INTO passages (
            guid,
            file_path,
            start_time,
            end_time,
            title,
            artist,
            album,
            recording_mbid,
            musical_flavor,
            flavor_source_blend,
            flavor_confidence_map,
            flavor_completeness,
            title_source,
            title_confidence,
            artist_source,
            artist_confidence,
            identity_confidence,
            identity_conflicts,
            overall_quality_score,
            metadata_completeness,
            validation_status,
            validation_report,
            import_session_id,
            import_timestamp,
            import_strategy
        ) VALUES (
            ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?
        )
    "#;

    let validation_status_str = format!("{:?}", validation.status);

    sqlx::query(query)
        .bind(&passage_id)
        .bind(file_path)
        .bind(passage.boundary.start_time)
        .bind(passage.boundary.end_time)
        .bind(&fusion.metadata.title)
        .bind(&fusion.metadata.artist)
        .bind(&fusion.metadata.album)
        .bind(&fusion.identity.recording_mbid)
        .bind(&flavor_json)
        .bind(&flavor_source_blend)
        .bind(&flavor_confidence_map)
        .bind(fusion.flavor.completeness)
        .bind(&fusion.metadata.title_source)
        .bind(fusion.metadata.title_confidence)
        .bind(&fusion.metadata.artist_source)
        .bind(fusion.metadata.artist_confidence)
        .bind(fusion.identity.confidence)
        .bind(&identity_conflicts)
        .bind(validation.quality_score)
        .bind(fusion.metadata.completeness)
        .bind(&validation_status_str)
        .bind(&validation_report)
        .bind(import_session_id)
        .bind(timestamp)
        .bind("hybrid_fusion") // import_strategy
        .execute(db)
        .await
        .context("Failed to insert passage")?;

    // Convert ticks to seconds for logging display
    let start_seconds = passage.boundary.start_time as f64 / TICK_RATE as f64;
    let end_seconds = passage.boundary.end_time as f64 / TICK_RATE as f64;

    info!(
        "Stored passage {} ({:.1}s-{:.1}s) with quality {:.1}%",
        passage_id,
        start_seconds,
        end_seconds,
        validation.quality_score
    );

    // Store provenance log entries
    store_provenance_logs(db, &passage_id, passage).await?;

    Ok(passage_id)
}

/// Store import provenance log entries
async fn store_provenance_logs(
    db: &SqlitePool,
    passage_id: &str,
    passage: &ProcessedPassage,
) -> Result<()> {
    let timestamp = chrono::Utc::now().timestamp();

    // Insert provenance entry for each extraction source
    for extraction in &passage.extractions {
        let data_json = serde_json::json!({
            "source": extraction.source,
            "has_metadata": extraction.metadata.is_some(),
            "has_flavor": extraction.flavor.is_some(),
            "has_identity": extraction.identity.is_some(),
        });

        let data_str = serde_json::to_string(&data_json)?;

        sqlx::query(
            r#"
            INSERT INTO import_provenance (
                id,
                passage_id,
                source_type,
                data_extracted,
                confidence,
                timestamp
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(passage_id)
        .bind(&extraction.source)
        .bind(&data_str)
        .bind(extraction.confidence)
        .bind(timestamp)
        .execute(db)
        .await
        .context("Failed to insert provenance log")?;
    }

    debug!(
        "Stored {} provenance log entries for passage {}",
        passage.extractions.len(),
        passage_id
    );

    Ok(())
}

/// Store multiple passages in a transaction
///
/// # Arguments
/// * `db` - Database connection pool
/// * `file_path` - Source audio file path
/// * `passages` - All processed passages from the file
/// * `import_session_id` - UUID for this import session
///
/// # Returns
/// * Vec of passage UUIDs
pub async fn store_passages_batch(
    db: &SqlitePool,
    file_path: &str,
    passages: &[ProcessedPassage],
    import_session_id: &str,
) -> Result<Vec<String>> {
    let mut tx = db.begin().await.context("Failed to begin transaction")?;
    let mut passage_ids = Vec::new();

    for passage in passages {
        let passage_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp();

        // Extract fusion result data
        let fusion = &passage.fusion;
        let validation = &passage.validation;

        // Serialize JSON fields
        let flavor_json = serde_json::to_string(&fusion.flavor.characteristics)?;
        let flavor_source_blend = serde_json::to_string(&fusion.flavor.source_blend)?;
        let flavor_confidence_map = serde_json::to_string(&fusion.flavor.confidence_map)?;
        let identity_conflicts = if !fusion.identity.conflicts.is_empty() {
            Some(serde_json::to_string(&fusion.identity.conflicts)?)
        } else {
            None
        };
        let validation_report = serde_json::to_string(&validation.checks)?;

        // Insert passage (SPEC017: times as INTEGER ticks)
        let query = r#"
            INSERT INTO passages (
                guid, file_path, start_time, end_time,
                title, artist, album, recording_mbid, musical_flavor,
                flavor_source_blend, flavor_confidence_map, flavor_completeness,
                title_source, title_confidence, artist_source, artist_confidence,
                identity_confidence, identity_conflicts,
                overall_quality_score, metadata_completeness,
                validation_status, validation_report,
                import_session_id, import_timestamp, import_strategy
            ) VALUES (
                ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?
            )
        "#;

        let validation_status_str = format!("{:?}", validation.status);

        sqlx::query(query)
            .bind(&passage_id)
            .bind(file_path)
            .bind(passage.boundary.start_time)
            .bind(passage.boundary.end_time)
            .bind(&fusion.metadata.title)
            .bind(&fusion.metadata.artist)
            .bind(&fusion.metadata.album)
            .bind(&fusion.identity.recording_mbid)
            .bind(&flavor_json)
            .bind(&flavor_source_blend)
            .bind(&flavor_confidence_map)
            .bind(fusion.flavor.completeness)
            .bind(&fusion.metadata.title_source)
            .bind(fusion.metadata.title_confidence)
            .bind(&fusion.metadata.artist_source)
            .bind(fusion.metadata.artist_confidence)
            .bind(fusion.identity.confidence)
            .bind(&identity_conflicts)
            .bind(validation.quality_score)
            .bind(fusion.metadata.completeness)
            .bind(&validation_status_str)
            .bind(&validation_report)
            .bind(import_session_id)
            .bind(timestamp)
            .bind("hybrid_fusion")
            .execute(&mut *tx)
            .await?;

        // Store provenance logs
        for extraction in &passage.extractions {
            let data_json = serde_json::json!({
                "source": extraction.source,
                "has_metadata": extraction.metadata.is_some(),
                "has_flavor": extraction.flavor.is_some(),
                "has_identity": extraction.identity.is_some(),
            });

            let data_str = serde_json::to_string(&data_json)?;

            sqlx::query(
                r#"
                INSERT INTO import_provenance (
                    id, passage_id, source_type, data_extracted, confidence, timestamp
                ) VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&passage_id)
            .bind(&extraction.source)
            .bind(&data_str)
            .bind(extraction.confidence)
            .bind(timestamp)
            .execute(&mut *tx)
            .await?;
        }

        passage_ids.push(passage_id);
    }

    tx.commit().await.context("Failed to commit transaction")?;

    info!(
        "Stored {} passages from {} in transaction",
        passages.len(),
        file_path
    );

    Ok(passage_ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_generation() {
        let uuid1 = Uuid::new_v4().to_string();
        let uuid2 = Uuid::new_v4().to_string();
        assert_ne!(uuid1, uuid2);
    }
}

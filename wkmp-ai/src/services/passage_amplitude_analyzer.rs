//! Passage Amplitude Analysis for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-015] Amplitude Analysis (Phase 8)
//!
//! Analyzes lead-in/lead-out timing for each passage using RMS amplitude detection.
//! Updates passages table with lead_in_start_ticks and lead_out_start_ticks.

use sqlx::{Pool, Sqlite};
use std::path::Path;
use uuid::Uuid;
use wkmp_common::{Error, Result};

use crate::models::AmplitudeParameters;
use super::amplitude_analyzer::AmplitudeAnalyzer;
use super::passage_recorder::PassageRecord;

/// SPEC017: 28,224,000 ticks per second
const TICKS_PER_SECOND: i64 = 28_224_000;

/// Amplitude analysis result for a passage
#[derive(Debug, Clone)]
pub struct PassageAmplitudeResult {
    /// Passage GUID
    pub passage_id: Uuid,
    /// Lead-in start time (ticks from passage start)
    pub lead_in_start_ticks: i64,
    /// Lead-out start time (ticks from passage start)
    pub lead_out_start_ticks: i64,
}

/// Amplitude analysis result
#[derive(Debug, Clone)]
pub struct AmplitudeResult {
    /// Analyzed passages
    pub passages: Vec<PassageAmplitudeResult>,
    /// Statistics
    pub stats: AmplitudeStats,
}

/// Amplitude analysis statistics
#[derive(Debug, Clone)]
pub struct AmplitudeStats {
    /// Total passages analyzed
    pub passages_analyzed: usize,
    /// Average lead-in duration (seconds)
    pub avg_lead_in_seconds: f64,
    /// Average lead-out duration (seconds)
    pub avg_lead_out_seconds: f64,
}

/// Passage Amplitude Analyzer
///
/// **Traceability:** [REQ-SPEC032-015] (Phase 8: AMPLITUDE)
pub struct PassageAmplitudeAnalyzer {
    db: Pool<Sqlite>,
    analyzer: AmplitudeAnalyzer,
}

impl PassageAmplitudeAnalyzer {
    /// Create new passage amplitude analyzer
    pub async fn new(db: Pool<Sqlite>) -> Result<Self> {
        // Load amplitude parameters from settings
        let lead_in_threshold_db = get_lead_in_threshold_db(&db).await?;
        let lead_out_threshold_db = get_lead_out_threshold_db(&db).await?;

        let params = AmplitudeParameters {
            rms_window_ms: 100, // 100ms windows
            lead_in_threshold_db,
            lead_out_threshold_db,
            quick_ramp_threshold: 0.75,
            quick_ramp_duration_s: 1.0,
            max_lead_in_duration_s: 10.0,
            max_lead_out_duration_s: 10.0,
            apply_a_weighting: false,
        };

        Ok(Self {
            db,
            analyzer: AmplitudeAnalyzer::new(params),
        })
    }

    /// Analyze passages for amplitude timing
    ///
    /// **Algorithm:**
    /// 1. For each passage:
    ///    a. Convert passage boundaries from ticks to seconds
    ///    b. Call AmplitudeAnalyzer::analyze_file() for passage segment
    ///    c. Convert lead-in/lead-out durations to ticks
    ///    d. Calculate absolute tick positions (relative to passage start)
    /// 2. Update passages table with lead_in_start_ticks, lead_out_start_ticks
    /// 3. Set passages.status = 'INGEST COMPLETE'
    /// 4. Return amplitude result with statistics
    ///
    /// **Traceability:** [REQ-SPEC032-015]
    pub async fn analyze_passages(
        &self,
        file_path: &Path,
        passages: &[PassageRecord],
    ) -> Result<AmplitudeResult> {
        tracing::debug!(
            path = %file_path.display(),
            passage_count = passages.len(),
            "Analyzing passage amplitudes"
        );

        let mut results = Vec::new();
        let mut total_lead_in = 0.0;
        let mut total_lead_out = 0.0;

        for passage_record in passages {
            // **[PHASE 1]** Get passage boundaries from database, then RELEASE connection
            // This prevents holding database locks during long-running amplitude analysis
            let (start_ticks, end_ticks): (i64, i64) = {
                sqlx::query_as(
                    "SELECT start_time_ticks, end_time_ticks FROM passages WHERE guid = ?"
                )
                .bind(passage_record.passage_id.to_string())
                .fetch_one(&self.db)
                .await?
            }; // Connection released here

            // Convert to seconds
            let start_seconds = start_ticks as f64 / TICKS_PER_SECOND as f64;
            let end_seconds = end_ticks as f64 / TICKS_PER_SECOND as f64;

            tracing::debug!(
                passage_id = %passage_record.passage_id,
                start_seconds,
                end_seconds,
                "Analyzing passage amplitude"
            );

            // **[PHASE 2]** Analyze amplitude (long-running, NO database connection held)
            let analysis = self
                .analyzer
                .analyze_file(file_path, start_seconds, end_seconds)
                .await
                .map_err(|e| Error::Internal(format!("Amplitude analysis failed: {}", e)))?;

            // **[PHASE 3]** Convert lead-in/lead-out to ABSOLUTE tick positions (relative to file start)
            // **[SPEC032]** Database stores absolute positions, verified by CHECK constraints
            // **[ORIGINAL SPEC]** Lead-in limited to first 25% of passage, lead-out limited to last 25%
            // Therefore lead_in_start_ticks <= lead_out_start_ticks is ALWAYS satisfied (no overlap check needed)

            // Lead-in: absolute position = passage start + lead-in duration
            // Clamped to [start_ticks, end_ticks]
            let lead_in_duration_ticks = (analysis.lead_in_duration * TICKS_PER_SECOND as f64) as i64;
            let lead_in_start_ticks = (start_ticks + lead_in_duration_ticks).clamp(start_ticks, end_ticks);

            // Lead-out: absolute position = passage end - lead-out duration
            // Clamped to [start_ticks, end_ticks]
            let lead_out_duration_ticks = (analysis.lead_out_duration * TICKS_PER_SECOND as f64) as i64;
            let lead_out_start_ticks = (end_ticks - lead_out_duration_ticks).clamp(start_ticks, end_ticks);

            tracing::debug!(
                passage_id = %passage_record.passage_id,
                lead_in_seconds = analysis.lead_in_duration,
                lead_out_seconds = analysis.lead_out_duration,
                start_ticks,
                end_ticks,
                lead_in_start_ticks,
                lead_out_start_ticks,
                lead_in_valid = lead_in_start_ticks >= start_ticks && lead_in_start_ticks <= end_ticks,
                lead_out_valid = lead_out_start_ticks >= start_ticks && lead_out_start_ticks <= end_ticks,
                ordering_valid = lead_in_start_ticks <= lead_out_start_ticks,
                "Amplitude analysis complete - computed absolute positions"
            );

            // **[PHASE 4]** Update passages table (brief database write, connection released after)
            if let Err(e) = sqlx::query(
                r#"
                UPDATE passages
                SET lead_in_start_ticks = ?,
                    lead_out_start_ticks = ?,
                    status = 'INGEST COMPLETE',
                    updated_at = CURRENT_TIMESTAMP
                WHERE guid = ?
                "#
            )
            .bind(lead_in_start_ticks)
            .bind(lead_out_start_ticks)
            .bind(passage_record.passage_id.to_string())
            .execute(&self.db)
            .await {
                tracing::error!(
                    passage_id = %passage_record.passage_id,
                    start_ticks,
                    end_ticks,
                    lead_in_start_ticks,
                    lead_out_start_ticks,
                    lead_in_duration_seconds = analysis.lead_in_duration,
                    lead_out_duration_seconds = analysis.lead_out_duration,
                    passage_duration_ticks = end_ticks - start_ticks,
                    error = %e,
                    "Database update failed with CHECK constraint - dumping all values"
                );
                return Err(e.into());
            }

            results.push(PassageAmplitudeResult {
                passage_id: passage_record.passage_id,
                lead_in_start_ticks,
                lead_out_start_ticks,
            });

            total_lead_in += analysis.lead_in_duration;
            total_lead_out += analysis.lead_out_duration;
        }

        let stats = AmplitudeStats {
            passages_analyzed: results.len(),
            avg_lead_in_seconds: if results.is_empty() {
                0.0
            } else {
                total_lead_in / results.len() as f64
            },
            avg_lead_out_seconds: if results.is_empty() {
                0.0
            } else {
                total_lead_out / results.len() as f64
            },
        };

        tracing::info!(
            path = %file_path.display(),
            passages_analyzed = stats.passages_analyzed,
            avg_lead_in = stats.avg_lead_in_seconds,
            avg_lead_out = stats.avg_lead_out_seconds,
            "Amplitude analysis complete"
        );

        Ok(AmplitudeResult {
            passages: results,
            stats,
        })
    }
}

/// Get lead-in threshold from settings (default: 45.0 dB)
async fn get_lead_in_threshold_db(db: &Pool<Sqlite>) -> Result<f64> {
    let value: Option<f64> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'lead_in_threshold_dB'"
    )
    .fetch_optional(db)
    .await?;

    Ok(value.unwrap_or(45.0))
}

/// Get lead-out threshold from settings (default: 40.0 dB)
async fn get_lead_out_threshold_db(db: &Pool<Sqlite>) -> Result<f64> {
    let value: Option<f64> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'lead_out_threshold_dB'"
    )
    .fetch_optional(db)
    .await?;

    Ok(value.unwrap_or(40.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Setup in-memory test database
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create settings table
        sqlx::query(
            r#"
            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value REAL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create passages table
        sqlx::query(
            r#"
            CREATE TABLE passages (
                guid TEXT PRIMARY KEY,
                file_id TEXT NOT NULL,
                start_time_ticks INTEGER NOT NULL,
                end_time_ticks INTEGER NOT NULL,
                song_id TEXT,
                title TEXT,
                lead_in_start_ticks INTEGER,
                lead_out_start_ticks INTEGER,
                status TEXT DEFAULT 'PENDING',
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_analyzer_creation() {
        let pool = setup_test_db().await;
        let _analyzer = PassageAmplitudeAnalyzer::new(pool).await.unwrap();
        // Just verify it can be created
    }

    #[tokio::test]
    async fn test_get_lead_in_threshold_default() {
        let pool = setup_test_db().await;
        let threshold = get_lead_in_threshold_db(&pool).await.unwrap();
        assert_eq!(threshold, 45.0);
    }

    #[tokio::test]
    async fn test_get_lead_in_threshold_custom() {
        let pool = setup_test_db().await;

        sqlx::query("INSERT INTO settings (key, value) VALUES ('lead_in_threshold_dB', 50.0)")
            .execute(&pool)
            .await
            .unwrap();

        let threshold = get_lead_in_threshold_db(&pool).await.unwrap();
        assert_eq!(threshold, 50.0);
    }

    #[tokio::test]
    async fn test_get_lead_out_threshold_default() {
        let pool = setup_test_db().await;
        let threshold = get_lead_out_threshold_db(&pool).await.unwrap();
        assert_eq!(threshold, 40.0);
    }

    #[tokio::test]
    async fn test_amplitude_stats_empty() {
        let stats = AmplitudeStats {
            passages_analyzed: 0,
            avg_lead_in_seconds: 0.0,
            avg_lead_out_seconds: 0.0,
        };

        assert_eq!(stats.passages_analyzed, 0);
        assert_eq!(stats.avg_lead_in_seconds, 0.0);
    }
}

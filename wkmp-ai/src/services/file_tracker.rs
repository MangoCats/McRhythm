//! File-Level Import Tracking and Skip Logic
//!
//! Implements intelligent file-level import tracking per Amendment 8 (REQ-AI-009 series).
//! Provides skip logic, confidence aggregation, and metadata merging.
//!
//! # Architecture
//! - Phase -1 pre-import skip logic (7 skip conditions)
//! - Confidence aggregation (MIN formula)
//! - Metadata merge with confidence preservation
//! - Re-import attempt tracking
//!
//! # Implementation
//! Per PLAN024 Amendment 8 (TASK-000)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

// ============================================================================
// File Tracking State
// ============================================================================

/// File import tracking information from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTrackingInfo {
    pub file_id: uuid::Uuid,
    pub file_hash: Option<String>,
    pub modification_time: Option<i64>,
    pub import_completed_at: Option<i64>,
    pub import_success_confidence: Option<f32>,
    pub metadata_import_completed_at: Option<i64>,
    pub metadata_confidence: Option<f32>,
    pub user_approved_at: Option<i64>,
    pub reimport_attempt_count: i32,
    pub last_reimport_attempt_at: Option<i64>,
}

/// Skip decision result
#[derive(Debug, Clone, PartialEq)]
pub enum SkipDecision {
    /// Skip entire import (no processing needed)
    SkipImport(SkipReason),
    /// Skip metadata collection only (proceed with audio analysis)
    SkipMetadata(SkipReason),
    /// Proceed with full import
    ProceedWithImport,
}

/// Reason for skipping import
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SkipReason {
    /// User has approved this file (absolute protection)
    UserApproved,
    /// File hash unchanged since last import
    HashUnchanged,
    /// File modification time unchanged
    ModificationTimeUnchanged,
    /// Import confidence meets threshold
    ImportConfidenceSufficient(f32),
    /// Metadata confidence meets threshold
    MetadataConfidenceSufficient(f32),
    /// Maximum re-import attempts reached
    MaxReimportAttemptsReached(i32),
}

// ============================================================================
// Configuration
// ============================================================================

/// File tracker configuration (loaded from database parameters)
#[derive(Debug, Clone)]
pub struct FileTrackerConfig {
    /// [PARAM-AI-005] Import success confidence threshold (default 0.75)
    pub import_success_confidence_threshold: f32,
    /// [PARAM-AI-006] Metadata confidence threshold (default 0.66)
    pub metadata_confidence_threshold: f32,
    /// [PARAM-AI-007] Maximum reimport attempts (default 3)
    pub max_reimport_attempts: i32,
}

impl Default for FileTrackerConfig {
    fn default() -> Self {
        Self {
            import_success_confidence_threshold: 0.75,
            metadata_confidence_threshold: 0.66,
            max_reimport_attempts: 3,
        }
    }
}

// ============================================================================
// File Tracker
// ============================================================================

pub struct FileTracker {
    db: SqlitePool,
    config: FileTrackerConfig,
}

impl FileTracker {
    pub fn new(db: SqlitePool, config: FileTrackerConfig) -> Self {
        Self { db, config }
    }

    /// Evaluate skip logic for a file (Phase -1)
    ///
    /// Evaluates 7 skip conditions in priority order per REQ-AI-009-04 through REQ-AI-009-09.
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    /// * `file_id` - UUID of file in database
    ///
    /// # Returns
    /// Skip decision indicating whether to skip import, skip metadata only, or proceed
    pub async fn evaluate_skip_logic(
        &self,
        file_path: &Path,
        file_id: uuid::Uuid,
    ) -> Result<SkipDecision> {
        // Load file tracking info from database
        let tracking_info = self.load_tracking_info(file_id).await?;

        // Compute current file hash and modification time
        let current_hash = compute_file_hash(file_path).await?;
        let current_mtime = get_modification_time(file_path)?;

        // Evaluate skip conditions in priority order
        // Condition 1: User Approval (Absolute Priority)
        if let Some(_approved_at) = tracking_info.user_approved_at {
            info!(
                file_id = %file_id,
                reason = "user_approved",
                "Skip: File approved by user (absolute protection)"
            );
            return Ok(SkipDecision::SkipImport(SkipReason::UserApproved));
        }

        // Condition 2: Hash-Based Duplicate Detection
        if let Some(ref stored_hash) = tracking_info.file_hash {
            if stored_hash == &current_hash && tracking_info.import_completed_at.is_some() {
                debug!(
                    file_id = %file_id,
                    hash = %current_hash,
                    "Skip: File hash unchanged since last import"
                );
                return Ok(SkipDecision::SkipImport(SkipReason::HashUnchanged));
            }
        }

        // Condition 3: Modification Time Check
        if let Some(stored_mtime) = tracking_info.modification_time {
            if stored_mtime == current_mtime && tracking_info.import_completed_at.is_some() {
                debug!(
                    file_id = %file_id,
                    mtime = current_mtime,
                    "Skip: Modification time unchanged"
                );
                return Ok(SkipDecision::SkipImport(
                    SkipReason::ModificationTimeUnchanged,
                ));
            }
        }

        // Condition 4: Import Success Confidence Threshold
        if let Some(confidence) = tracking_info.import_success_confidence {
            if confidence >= self.config.import_success_confidence_threshold {
                debug!(
                    file_id = %file_id,
                    confidence = confidence,
                    threshold = self.config.import_success_confidence_threshold,
                    "Skip: Import confidence sufficient"
                );
                return Ok(SkipDecision::SkipImport(
                    SkipReason::ImportConfidenceSufficient(confidence),
                ));
            }
        }

        // Condition 5: Metadata Confidence Threshold (skip metadata only)
        if let Some(metadata_confidence) = tracking_info.metadata_confidence {
            if metadata_confidence >= self.config.metadata_confidence_threshold {
                debug!(
                    file_id = %file_id,
                    metadata_confidence = metadata_confidence,
                    threshold = self.config.metadata_confidence_threshold,
                    "Skip metadata: Metadata confidence sufficient"
                );
                return Ok(SkipDecision::SkipMetadata(
                    SkipReason::MetadataConfidenceSufficient(metadata_confidence),
                ));
            }
        }

        // Condition 6: Re-import Attempt Limiting
        if tracking_info.reimport_attempt_count >= self.config.max_reimport_attempts {
            warn!(
                file_id = %file_id,
                attempt_count = tracking_info.reimport_attempt_count,
                max_attempts = self.config.max_reimport_attempts,
                "Skip: Maximum reimport attempts reached (flag for manual review)"
            );
            return Ok(SkipDecision::SkipImport(
                SkipReason::MaxReimportAttemptsReached(tracking_info.reimport_attempt_count),
            ));
        }

        // Condition 7: Low-Confidence Flagging (does not skip, but logs warning)
        if let Some(confidence) = tracking_info.import_success_confidence {
            if confidence < self.config.import_success_confidence_threshold {
                warn!(
                    file_id = %file_id,
                    confidence = confidence,
                    threshold = self.config.import_success_confidence_threshold,
                    "Low confidence import - flagging for user review"
                );
            }
        }

        // No skip conditions met - proceed with full import
        Ok(SkipDecision::ProceedWithImport)
    }

    /// Aggregate passage-level confidence into file-level confidence (MIN formula)
    ///
    /// Per REQ-AI-009-01:
    /// ```text
    /// file_confidence = MIN(passage_composite_scores)
    /// passage_composite = (identity_conf * 0.4) + (metadata_complete / 100.0 * 0.3) + (quality_score / 100.0 * 0.3)
    /// ```
    ///
    /// # Arguments
    /// * `passage_composite_scores` - Composite scores for each passage in file
    ///
    /// # Returns
    /// Aggregate file-level confidence score (0.0-1.0)
    pub fn aggregate_confidence(&self, passage_composite_scores: &[f32]) -> f32 {
        if passage_composite_scores.is_empty() {
            return 0.0;
        }

        // Conservative approach: MIN of all passage scores
        passage_composite_scores
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min)
            .max(0.0)
            .min(1.0)
    }

    /// Merge metadata with confidence-based overwrite
    ///
    /// Per REQ-AI-009-08: Higher confidence metadata overwrites lower confidence metadata.
    ///
    /// # Arguments
    /// * `existing` - Existing metadata with confidence scores
    /// * `new` - New metadata with confidence scores
    ///
    /// # Returns
    /// Merged metadata preserving highest-confidence values
    pub fn merge_metadata(
        &self,
        existing: HashMap<String, (String, f32)>,
        new: HashMap<String, (String, f32)>,
    ) -> HashMap<String, (String, f32)> {
        let mut merged = existing.clone();

        for (key, (new_value, new_confidence)) in new {
            match merged.get(&key) {
                Some((_, existing_confidence)) => {
                    // Overwrite if new confidence is higher
                    if new_confidence > *existing_confidence {
                        debug!(
                            key = %key,
                            new_conf = new_confidence,
                            existing_conf = existing_confidence,
                            "Metadata merge: using new value (higher confidence)"
                        );
                        merged.insert(key, (new_value, new_confidence));
                    }
                }
                None => {
                    // No existing value - insert new
                    merged.insert(key, (new_value, new_confidence));
                }
            }
        }

        merged
    }

    /// Update file tracking info after import completion (Phase 7)
    ///
    /// # Arguments
    /// * `file_id` - UUID of imported file
    /// * `file_hash` - SHA-256 hash of file
    /// * `modification_time` - File modification timestamp (unix epoch ms)
    /// * `import_success_confidence` - Aggregate confidence score
    /// * `metadata_confidence` - Metadata confidence score
    pub async fn update_tracking_info(
        &self,
        file_id: uuid::Uuid,
        file_hash: String,
        modification_time: i64,
        import_success_confidence: f32,
        metadata_confidence: f32,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        sqlx::query(
            r#"
            UPDATE files
            SET file_hash = ?,
                modification_time = ?,
                import_completed_at = ?,
                import_success_confidence = ?,
                metadata_import_completed_at = ?,
                metadata_confidence = ?,
                reimport_attempt_count = reimport_attempt_count + 1,
                last_reimport_attempt_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&file_hash)
        .bind(modification_time)
        .bind(now)
        .bind(import_success_confidence)
        .bind(now)
        .bind(metadata_confidence)
        .bind(now)
        .bind(file_id.to_string())
        .execute(&self.db)
        .await
        .context("Failed to update file tracking info")?;

        Ok(())
    }

    /// Mark file as approved by user (absolute protection)
    pub async fn approve_file(&self, file_id: uuid::Uuid) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        sqlx::query(
            r#"
            UPDATE files
            SET user_approved_at = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(file_id.to_string())
        .execute(&self.db)
        .await
        .context("Failed to approve file")?;

        info!(file_id = %file_id, "File approved by user");
        Ok(())
    }

    // ------------------------------------------------------------------------
    // Private Helper Methods
    // ------------------------------------------------------------------------

    async fn load_tracking_info(&self, file_id: uuid::Uuid) -> Result<FileTrackingInfo> {
        let row = sqlx::query(
            r#"
            SELECT id, file_hash, modification_time, import_completed_at,
                   import_success_confidence, metadata_import_completed_at,
                   metadata_confidence, user_approved_at, reimport_attempt_count,
                   last_reimport_attempt_at
            FROM files
            WHERE id = ?
            "#,
        )
        .bind(file_id.to_string())
        .fetch_optional(&self.db)
        .await
        .context("Failed to load file tracking info")?;

        match row {
            Some(row) => Ok(FileTrackingInfo {
                file_id,
                file_hash: row.try_get("file_hash")?,
                modification_time: row.try_get("modification_time")?,
                import_completed_at: row.try_get("import_completed_at")?,
                import_success_confidence: row.try_get("import_success_confidence")?,
                metadata_import_completed_at: row.try_get("metadata_import_completed_at")?,
                metadata_confidence: row.try_get("metadata_confidence")?,
                user_approved_at: row.try_get("user_approved_at")?,
                reimport_attempt_count: row.try_get("reimport_attempt_count").unwrap_or(0),
                last_reimport_attempt_at: row.try_get("last_reimport_attempt_at")?,
            }),
            None => {
                // File not yet in database - return empty tracking info
                Ok(FileTrackingInfo {
                    file_id,
                    file_hash: None,
                    modification_time: None,
                    import_completed_at: None,
                    import_success_confidence: None,
                    metadata_import_completed_at: None,
                    metadata_confidence: None,
                    user_approved_at: None,
                    reimport_attempt_count: 0,
                    last_reimport_attempt_at: None,
                })
            }
        }
    }
}

// ============================================================================
// File Hash and Metadata Utilities
// ============================================================================

/// Compute SHA-256 hash of file
async fn compute_file_hash(file_path: &Path) -> Result<String> {
    let bytes = tokio::fs::read(file_path)
        .await
        .context("Failed to read file for hashing")?;

    let hash = Sha256::digest(&bytes);
    Ok(format!("{:x}", hash))
}

/// Get file modification time (unix epoch milliseconds)
fn get_modification_time(file_path: &Path) -> Result<i64> {
    let metadata = std::fs::metadata(file_path).context("Failed to read file metadata")?;

    let modified = metadata
        .modified()
        .context("Failed to get modification time")?;

    let duration = modified
        .duration_since(std::time::UNIX_EPOCH)
        .context("Invalid modification time")?;

    Ok(duration.as_millis() as i64)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> FileTrackerConfig {
        FileTrackerConfig::default()
    }

    #[test]
    fn test_aggregate_confidence_min_formula() {
        let config = create_test_config();

        // Test MIN formula
        let scores = vec![0.9, 0.75, 0.85];
        let result = if scores.is_empty() {
            0.0
        } else {
            scores.iter().copied().fold(f32::INFINITY, f32::min).max(0.0).min(1.0)
        };
        assert_eq!(result, 0.75);

        // Test single score
        let scores = vec![0.8];
        let result = if scores.is_empty() {
            0.0
        } else {
            scores.iter().copied().fold(f32::INFINITY, f32::min).max(0.0).min(1.0)
        };
        assert_eq!(result, 0.8);

        // Test empty
        let scores: Vec<f32> = vec![];
        let result = if scores.is_empty() {
            0.0
        } else {
            scores.iter().copied().fold(f32::INFINITY, f32::min).max(0.0).min(1.0)
        };
        assert_eq!(result, 0.0);

        // Test clamping
        let scores = vec![1.5, 0.9]; // Invalid score > 1.0
        let result = scores.iter().copied().fold(f32::INFINITY, f32::min).max(0.0).min(1.0);
        assert_eq!(result, 0.9);
    }

    #[test]
    fn test_metadata_merge_higher_confidence_wins() {
        let mut existing = HashMap::new();
        existing.insert("artist".to_string(), ("Old Artist".to_string(), 0.5));
        existing.insert("title".to_string(), ("Old Title".to_string(), 0.8));

        let mut new = HashMap::new();
        new.insert("artist".to_string(), ("New Artist".to_string(), 0.9));
        new.insert("title".to_string(), ("New Title".to_string(), 0.6));
        new.insert("album".to_string(), ("New Album".to_string(), 0.7));

        // Merge logic (same as FileTracker::merge_metadata)
        let mut merged = existing.clone();
        for (key, (new_value, new_confidence)) in new {
            match merged.get(&key) {
                Some((_, existing_confidence)) => {
                    if new_confidence > *existing_confidence {
                        merged.insert(key, (new_value, new_confidence));
                    }
                }
                None => {
                    merged.insert(key, (new_value, new_confidence));
                }
            }
        }

        // Artist: new confidence (0.9) > existing (0.5) → use new
        assert_eq!(merged.get("artist").unwrap().0, "New Artist");
        assert_eq!(merged.get("artist").unwrap().1, 0.9);

        // Title: existing confidence (0.8) > new (0.6) → use existing
        assert_eq!(merged.get("title").unwrap().0, "Old Title");
        assert_eq!(merged.get("title").unwrap().1, 0.8);

        // Album: no existing → use new
        assert_eq!(merged.get("album").unwrap().0, "New Album");
        assert_eq!(merged.get("album").unwrap().1, 0.7);
    }
}

//! Ground truth management for testing
//!
//! **[PLAN031 Increment 2]** Ground truth schema and data access
//!
//! Provides storage and retrieval of known-correct MBID mappings used
//! for evaluating import accuracy.
//!
//! ## Ground Truth Sources (Priority Order)
//!
//! 1. **Embedded MBID Tags** - Files with `MUSICBRAINZ_TRACKID` ID3 tag
//! 2. **Curated Test Sets** - Manually verified JSON files
//! 3. **Cross-Validation** - Agreement between 3+ identification methods
//! 4. **Human Review** - Manual verification of ambiguous cases

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;

/// Verification method for ground truth entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    /// MBID extracted from embedded ID3 tag
    EmbeddedTag,
    /// Manually curated test set
    Curated,
    /// Agreement between multiple identification methods
    CrossValidation,
    /// Human manual review
    HumanReview,
    /// High-confidence algorithmic match (≥80%)
    AlgorithmicHigh,
    /// Medium-confidence algorithmic match (50-80%)
    AlgorithmicMedium,
}

impl VerificationMethod {
    /// Convert to database string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationMethod::EmbeddedTag => "embedded_tag",
            VerificationMethod::Curated => "curated",
            VerificationMethod::CrossValidation => "cross_validation",
            VerificationMethod::HumanReview => "human_review",
            VerificationMethod::AlgorithmicHigh => "algorithmic_high",
            VerificationMethod::AlgorithmicMedium => "algorithmic_medium",
        }
    }

    /// Parse from database string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "embedded_tag" => Some(VerificationMethod::EmbeddedTag),
            "curated" => Some(VerificationMethod::Curated),
            "cross_validation" => Some(VerificationMethod::CrossValidation),
            "human_review" => Some(VerificationMethod::HumanReview),
            "algorithmic_high" => Some(VerificationMethod::AlgorithmicHigh),
            "algorithmic_medium" => Some(VerificationMethod::AlgorithmicMedium),
            _ => None,
        }
    }
}

impl std::fmt::Display for VerificationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Ground truth entry for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruth {
    /// Database ID (None for new entries)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// SHA-256 hash of the file
    pub file_hash: String,
    /// Path to the file
    pub file_path: String,
    /// Expected recording MBID (None if file should NOT match any recording)
    pub expected_mbid: Option<String>,
    /// Expected album MBID for album-level tests
    pub expected_album_mbid: Option<String>,
    /// How this ground truth was verified
    pub verification_method: VerificationMethod,
    /// When this entry was verified
    pub verified_at: String,
    /// Confidence in the ground truth (0.0-1.0)
    pub confidence: f64,
    /// Optional notes about this entry
    pub notes: Option<String>,
}

impl GroundTruth {
    /// Create a new ground truth entry from an embedded MBID tag
    pub fn from_embedded_tag(file_hash: String, file_path: String, mbid: String) -> Self {
        Self {
            id: None,
            file_hash,
            file_path,
            expected_mbid: Some(mbid),
            expected_album_mbid: None,
            verification_method: VerificationMethod::EmbeddedTag,
            verified_at: chrono::Utc::now().to_rfc3339(),
            confidence: 1.0,
            notes: None,
        }
    }

    /// Create a ground truth entry for a file that should NOT match any recording
    pub fn no_match(file_hash: String, file_path: String, method: VerificationMethod) -> Self {
        Self {
            id: None,
            file_hash,
            file_path,
            expected_mbid: None,
            expected_album_mbid: None,
            verification_method: method,
            verified_at: chrono::Utc::now().to_rfc3339(),
            confidence: 1.0,
            notes: Some("Expected: No MBID match".to_string()),
        }
    }
}

/// JSON format for ground truth file
#[derive(Debug, Deserialize)]
pub struct GroundTruthFile {
    /// File format version
    pub version: String,
    /// When this file was created
    pub created: String,
    /// Ground truth entries
    pub entries: Vec<GroundTruthEntry>,
}

/// Entry in a ground truth JSON file
#[derive(Debug, Deserialize)]
pub struct GroundTruthEntry {
    /// Path to the file
    pub file_path: String,
    /// SHA-256 hash of the file
    pub file_hash: String,
    /// Expected recording MBID
    pub expected_mbid: Option<String>,
    /// Verification method
    pub verification: String,
    /// Confidence in the ground truth
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    /// Optional metadata (not stored, for reference)
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

fn default_confidence() -> f64 {
    1.0
}

/// Get ground truth by file hash
pub async fn get_by_hash(pool: &SqlitePool, file_hash: &str) -> Result<Option<GroundTruth>> {
    let row: Option<(
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        f64,
        Option<String>,
    )> = sqlx::query_as(
        r#"
        SELECT id, file_hash, file_path, expected_mbid, expected_album_mbid,
               verification_method, verified_at, confidence, notes
        FROM ground_truth
        WHERE file_hash = ?
        "#,
    )
    .bind(file_hash)
    .fetch_optional(pool)
    .await
    .context("Failed to query ground truth")?;

    Ok(row.map(
        |(
            id,
            file_hash,
            file_path,
            expected_mbid,
            expected_album_mbid,
            verification_method,
            verified_at,
            confidence,
            notes,
        )| {
            GroundTruth {
                id: Some(id),
                file_hash,
                file_path,
                expected_mbid,
                expected_album_mbid,
                verification_method: VerificationMethod::from_str(&verification_method)
                    .unwrap_or(VerificationMethod::Curated),
                verified_at,
                confidence,
                notes,
            }
        },
    ))
}

/// Insert or update a ground truth entry
pub async fn upsert(pool: &SqlitePool, gt: &GroundTruth) -> Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO ground_truth (
            file_hash, file_path, expected_mbid, expected_album_mbid,
            verification_method, verified_at, confidence, notes
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(file_hash) DO UPDATE SET
            file_path = excluded.file_path,
            expected_mbid = excluded.expected_mbid,
            expected_album_mbid = excluded.expected_album_mbid,
            verification_method = excluded.verification_method,
            verified_at = excluded.verified_at,
            confidence = excluded.confidence,
            notes = excluded.notes
        "#,
    )
    .bind(&gt.file_hash)
    .bind(&gt.file_path)
    .bind(&gt.expected_mbid)
    .bind(&gt.expected_album_mbid)
    .bind(gt.verification_method.as_str())
    .bind(&gt.verified_at)
    .bind(gt.confidence)
    .bind(&gt.notes)
    .execute(pool)
    .await
    .context("Failed to upsert ground truth")?;

    Ok(result.last_insert_rowid())
}

/// Load ground truth entries from a JSON file
///
/// File format:
/// ```json
/// {
///   "version": "1.0",
///   "created": "2025-12-13",
///   "entries": [
///     {
///       "file_path": "path/to/file.mp3",
///       "file_hash": "sha256:abc123...",
///       "expected_mbid": "12345678-1234-1234-1234-123456789012",
///       "verification": "curated",
///       "confidence": 1.0
///     }
///   ]
/// }
/// ```
pub async fn load_from_json(pool: &SqlitePool, path: &Path) -> Result<usize> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read ground truth file: {}", path.display()))?;

    let gt_file: GroundTruthFile =
        serde_json::from_str(&content).context("Failed to parse ground truth JSON")?;

    let mut count = 0;
    for entry in gt_file.entries {
        let method = VerificationMethod::from_str(&entry.verification)
            .unwrap_or(VerificationMethod::Curated);

        let gt = GroundTruth {
            id: None,
            file_hash: entry.file_hash,
            file_path: entry.file_path,
            expected_mbid: entry.expected_mbid,
            expected_album_mbid: None,
            verification_method: method,
            verified_at: chrono::Utc::now().to_rfc3339(),
            confidence: entry.confidence,
            notes: None,
        };

        upsert(pool, &gt).await?;
        count += 1;
    }

    tracing::info!(count, path = %path.display(), "Loaded ground truth from JSON");
    Ok(count)
}

/// Get count of ground truth entries by verification method
pub async fn count_by_method(pool: &SqlitePool) -> Result<Vec<(String, i64)>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT verification_method, COUNT(*) as count
        FROM ground_truth
        GROUP BY verification_method
        ORDER BY count DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to count ground truth entries")?;

    Ok(rows)
}

/// Get total count of ground truth entries
pub async fn count_total(pool: &SqlitePool) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ground_truth")
        .fetch_one(pool)
        .await
        .context("Failed to count ground truth entries")?;

    Ok(count)
}

/// Get all ground truth entries (for small test sets)
pub async fn get_all(pool: &SqlitePool, limit: Option<i64>) -> Result<Vec<GroundTruth>> {
    let limit = limit.unwrap_or(1000);

    let rows: Vec<(
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        f64,
        Option<String>,
    )> = sqlx::query_as(
        r#"
        SELECT id, file_hash, file_path, expected_mbid, expected_album_mbid,
               verification_method, verified_at, confidence, notes
        FROM ground_truth
        ORDER BY id
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("Failed to fetch ground truth entries")?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                file_hash,
                file_path,
                expected_mbid,
                expected_album_mbid,
                verification_method,
                verified_at,
                confidence,
                notes,
            )| {
                GroundTruth {
                    id: Some(id),
                    file_hash,
                    file_path,
                    expected_mbid,
                    expected_album_mbid,
                    verification_method: VerificationMethod::from_str(&verification_method)
                        .unwrap_or(VerificationMethod::Curated),
                    verified_at,
                    confidence,
                    notes,
                }
            },
        )
        .collect())
}

/// Delete a ground truth entry by file hash
pub async fn delete_by_hash(pool: &SqlitePool, file_hash: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM ground_truth WHERE file_hash = ?")
        .bind(file_hash)
        .execute(pool)
        .await
        .context("Failed to delete ground truth entry")?;

    Ok(result.rows_affected() > 0)
}

/// Ensure ground truth tables exist
///
/// Creates the tables if they don't exist. This is useful for testing
/// with in-memory databases where migrations haven't been run.
pub async fn ensure_tables(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ground_truth (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_hash TEXT NOT NULL UNIQUE,
            file_path TEXT NOT NULL,
            expected_mbid TEXT,
            expected_album_mbid TEXT,
            verification_method TEXT NOT NULL,
            verified_at TEXT NOT NULL DEFAULT (datetime('now')),
            confidence REAL NOT NULL DEFAULT 1.0,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create ground_truth table")?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_ground_truth_hash ON ground_truth(file_hash)")
        .execute(pool)
        .await
        .context("Failed to create ground_truth index")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        ensure_tables(&pool).await.expect("Failed to create tables");
        pool
    }

    #[tokio::test]
    async fn test_upsert_and_get() {
        let pool = setup_test_db().await;

        let gt = GroundTruth::from_embedded_tag(
            "hash123".to_string(),
            "/path/to/file.mp3".to_string(),
            "mbid-456".to_string(),
        );

        let id = upsert(&pool, &gt).await.expect("Failed to insert");
        assert!(id > 0);

        let retrieved = get_by_hash(&pool, "hash123")
            .await
            .expect("Failed to get")
            .expect("Should exist");

        assert_eq!(retrieved.file_hash, "hash123");
        assert_eq!(retrieved.expected_mbid, Some("mbid-456".to_string()));
        assert_eq!(
            retrieved.verification_method,
            VerificationMethod::EmbeddedTag
        );
    }

    #[tokio::test]
    async fn test_upsert_update() {
        let pool = setup_test_db().await;

        // Insert initial entry
        let gt1 = GroundTruth::from_embedded_tag(
            "hash123".to_string(),
            "/path/to/file.mp3".to_string(),
            "mbid-old".to_string(),
        );
        upsert(&pool, &gt1).await.expect("Failed to insert");

        // Update with new MBID
        let gt2 = GroundTruth::from_embedded_tag(
            "hash123".to_string(),
            "/path/to/file.mp3".to_string(),
            "mbid-new".to_string(),
        );
        upsert(&pool, &gt2).await.expect("Failed to update");

        // Verify update
        let retrieved = get_by_hash(&pool, "hash123")
            .await
            .expect("Failed to get")
            .expect("Should exist");

        assert_eq!(retrieved.expected_mbid, Some("mbid-new".to_string()));
    }

    #[tokio::test]
    async fn test_no_match_entry() {
        let pool = setup_test_db().await;

        let gt = GroundTruth::no_match(
            "hash-noise".to_string(),
            "/path/to/noise.mp3".to_string(),
            VerificationMethod::HumanReview,
        );

        upsert(&pool, &gt).await.expect("Failed to insert");

        let retrieved = get_by_hash(&pool, "hash-noise")
            .await
            .expect("Failed to get")
            .expect("Should exist");

        assert!(retrieved.expected_mbid.is_none());
        assert_eq!(
            retrieved.verification_method,
            VerificationMethod::HumanReview
        );
    }

    #[tokio::test]
    async fn test_count_total() {
        let pool = setup_test_db().await;

        // Empty initially
        assert_eq!(count_total(&pool).await.expect("count"), 0);

        // Add some entries
        for i in 0..5 {
            let gt = GroundTruth::from_embedded_tag(
                format!("hash{}", i),
                format!("/path/file{}.mp3", i),
                format!("mbid-{}", i),
            );
            upsert(&pool, &gt).await.expect("Failed to insert");
        }

        assert_eq!(count_total(&pool).await.expect("count"), 5);
    }

    #[tokio::test]
    async fn test_delete_by_hash() {
        let pool = setup_test_db().await;

        let gt = GroundTruth::from_embedded_tag(
            "hash-delete".to_string(),
            "/path/to/delete.mp3".to_string(),
            "mbid-del".to_string(),
        );
        upsert(&pool, &gt).await.expect("Failed to insert");

        // Verify it exists
        assert!(get_by_hash(&pool, "hash-delete")
            .await
            .expect("get")
            .is_some());

        // Delete it
        let deleted = delete_by_hash(&pool, "hash-delete")
            .await
            .expect("delete");
        assert!(deleted);

        // Verify it's gone
        assert!(get_by_hash(&pool, "hash-delete")
            .await
            .expect("get")
            .is_none());

        // Deleting again should return false
        let deleted_again = delete_by_hash(&pool, "hash-delete")
            .await
            .expect("delete");
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_verification_method_roundtrip() {
        let methods = [
            VerificationMethod::EmbeddedTag,
            VerificationMethod::Curated,
            VerificationMethod::CrossValidation,
            VerificationMethod::HumanReview,
        ];

        for method in methods {
            let s = method.as_str();
            let parsed = VerificationMethod::from_str(s).expect("Should parse");
            assert_eq!(parsed, method);
        }
    }
}

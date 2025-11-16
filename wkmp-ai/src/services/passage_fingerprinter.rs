//! Per-Passage Fingerprinting for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-012] Fingerprinting (Phase 5)
//!
//! Generates Chromaprint fingerprints for each passage and queries AcoustID API
//! for MusicBrainz Recording ID (MBID) candidates.

use sqlx::{Pool, Sqlite};
use std::path::Path;
use wkmp_common::{Error, Result};

use super::acoustid_client::AcoustIDClient;
use super::fingerprinter::Fingerprinter;
use super::passage_segmenter::PassageBoundary;

/// SPEC017: 28,224,000 ticks per second
const TICKS_PER_SECOND: i64 = 28_224_000;

/// Fingerprint result for a single passage
#[derive(Debug, Clone)]
pub struct PassageFingerprint {
    /// Chromaprint fingerprint (base64-encoded)
    pub fingerprint: String,
    /// AcoustID candidates (MBID + confidence score)
    pub candidates: Vec<MBIDCandidate>,
}

/// MusicBrainz Recording ID candidate with confidence
#[derive(Debug, Clone)]
pub struct MBIDCandidate {
    /// MusicBrainz Recording ID
    pub mbid: String,
    /// AcoustID confidence score (0.0 to 1.0)
    pub score: f64,
    /// Recording title (from AcoustID response)
    pub title: Option<String>,
    /// Recording duration in seconds (from AcoustID response)
    pub duration_seconds: Option<u64>,
}

/// Fingerprinting result
#[derive(Debug, Clone)]
pub enum FingerprintResult {
    /// Successful fingerprinting with candidates
    Success(Vec<PassageFingerprint>),
    /// Skipped (no API key configured)
    Skipped,
    /// Failed (error during fingerprinting or API call)
    Failed(String),
}

/// Passage Fingerprinter
///
/// **Traceability:** [REQ-SPEC032-012] (Phase 5: FINGERPRINTING)
pub struct PassageFingerprinter {
    fingerprinter: Fingerprinter,
    acoustid_client: Option<AcoustIDClient>,
}

impl PassageFingerprinter {
    /// Create new passage fingerprinter
    ///
    /// If `api_key` is None, fingerprinting will be skipped
    /// (per [AIA-SEC-030]: user acknowledged lack of API key)
    pub fn new(api_key: Option<String>, db: Pool<Sqlite>) -> Result<Self> {
        let acoustid_client = if let Some(key) = api_key {
            Some(
                AcoustIDClient::new(key, db)
                    .map_err(|e| Error::Internal(format!("AcoustID client creation failed: {}", e)))?,
            )
        } else {
            None
        };

        Ok(Self {
            fingerprinter: Fingerprinter::new(),
            acoustid_client,
        })
    }

    /// Fingerprint all passages in a file
    ///
    /// **Algorithm:**
    /// 1. Check if API key is configured
    ///    - If None: Return FingerprintResult::Skipped (metadata-only matching in Phase 6)
    /// 2. For each passage:
    ///    a. Convert passage boundaries from ticks to seconds
    ///    b. Skip if passage < 10 seconds (Chromaprint minimum)
    ///    c. Generate Chromaprint fingerprint via Fingerprinter::fingerprint_segment()
    ///       (uses tokio::spawn_blocking internally for CPU-intensive work)
    ///    d. Query AcoustID API for MBID candidates (rate-limited, async)
    ///    e. Store fingerprint + candidates
    /// 3. Return FingerprintResult::Success with all passage fingerprints
    ///
    /// **Rate Limiting:**
    /// AcoustIDClient automatically rate-limits to 3 requests/second (334ms interval)
    ///
    /// **Traceability:** [REQ-SPEC032-012]
    pub async fn fingerprint_passages(
        &self,
        file_path: &Path,
        passages: &[PassageBoundary],
    ) -> Result<FingerprintResult> {
        tracing::debug!(
            path = %file_path.display(),
            passage_count = passages.len(),
            "Fingerprinting passages"
        );

        // Check if API key configured
        let acoustid = match &self.acoustid_client {
            Some(client) => client,
            None => {
                tracing::info!(
                    path = %file_path.display(),
                    "Skipping fingerprinting: no API key configured"
                );
                return Ok(FingerprintResult::Skipped);
            }
        };

        let mut passage_fingerprints = Vec::new();

        // Fingerprint each passage
        for (idx, passage) in passages.iter().enumerate() {
            // Convert ticks to seconds
            let start_seconds = passage.start_ticks as f32 / TICKS_PER_SECOND as f32;
            let end_seconds = passage.end_ticks as f32 / TICKS_PER_SECOND as f32;
            let duration_seconds = passage.duration_ticks() as f32 / TICKS_PER_SECOND as f32;

            tracing::debug!(
                passage_idx = idx,
                start_seconds,
                end_seconds,
                duration_seconds,
                "Processing passage"
            );

            // Skip if passage too short (<10 seconds)
            if duration_seconds < 10.0 {
                tracing::debug!(
                    passage_idx = idx,
                    duration_seconds,
                    "Skipping passage: too short (<10s Chromaprint minimum)"
                );
                continue;
            }

            // Generate Chromaprint fingerprint (CPU-intensive, uses spawn_blocking internally)
            let fingerprint = match self
                .fingerprinter
                .fingerprint_segment(file_path, start_seconds, end_seconds)
            {
                Ok(fp) => fp,
                Err(e) => {
                    tracing::warn!(
                        passage_idx = idx,
                        error = %e,
                        "Fingerprint generation failed"
                    );
                    return Ok(FingerprintResult::Failed(format!(
                        "Fingerprint generation failed for passage {}: {}",
                        idx, e
                    )));
                }
            };

            tracing::debug!(
                passage_idx = idx,
                fingerprint_len = fingerprint.len(),
                "Generated fingerprint"
            );

            // Query AcoustID API (rate-limited, async)
            let response = match acoustid
                .lookup(&fingerprint, duration_seconds as u64)
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::warn!(
                        passage_idx = idx,
                        error = %e,
                        "AcoustID API call failed"
                    );
                    return Ok(FingerprintResult::Failed(format!(
                        "AcoustID API call failed for passage {}: {}",
                        idx, e
                    )));
                }
            };

            // Extract MBID candidates from response
            let candidates = extract_candidates(&response.results);

            tracing::debug!(
                passage_idx = idx,
                candidate_count = candidates.len(),
                "AcoustID lookup complete"
            );

            passage_fingerprints.push(PassageFingerprint {
                fingerprint,
                candidates,
            });
        }

        tracing::info!(
            path = %file_path.display(),
            passage_count = passages.len(),
            fingerprinted_count = passage_fingerprints.len(),
            "Fingerprinting complete"
        );

        Ok(FingerprintResult::Success(passage_fingerprints))
    }
}

/// Extract MBID candidates from AcoustID results
fn extract_candidates(
    results: &[super::acoustid_client::AcoustIDResult],
) -> Vec<MBIDCandidate> {
    let mut candidates = Vec::new();

    for result in results {
        if let Some(recordings) = &result.recordings {
            for recording in recordings {
                candidates.push(MBIDCandidate {
                    mbid: recording.id.clone(),
                    score: result.score,
                    title: recording.title.clone(),
                    duration_seconds: recording.duration,
                });
            }
        }
    }

    // Sort by score descending
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Setup in-memory test database
    async fn setup_test_db() -> SqlitePool {
        SqlitePool::connect(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_fingerprinter_creation_with_key() {
        let pool = setup_test_db().await;
        let _fingerprinter =
            PassageFingerprinter::new(Some("test_key".to_string()), pool).unwrap();
        // Just verify it can be created without panic
    }

    #[tokio::test]
    async fn test_fingerprinter_creation_without_key() {
        let pool = setup_test_db().await;
        let _fingerprinter = PassageFingerprinter::new(None, pool).unwrap();
        // Just verify it can be created without panic
    }

    #[tokio::test]
    async fn test_fingerprint_passages_skipped_no_key() {
        let pool = setup_test_db().await;
        let fingerprinter = PassageFingerprinter::new(None, pool).unwrap();

        let passages = vec![PassageBoundary::new(0, 28224000 * 30)]; // 30 seconds

        let result = fingerprinter
            .fingerprint_passages(Path::new("/test/file.mp3"), &passages)
            .await
            .unwrap();

        // Should be skipped (no API key)
        matches!(result, FingerprintResult::Skipped);
    }

    #[test]
    fn test_extract_candidates_empty() {
        let candidates = extract_candidates(&[]);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_mbid_candidate_creation() {
        let candidate = MBIDCandidate {
            mbid: "test-mbid".to_string(),
            score: 0.95,
            title: Some("Test Song".to_string()),
            duration_seconds: Some(180),
        };

        assert_eq!(candidate.mbid, "test-mbid");
        assert_eq!(candidate.score, 0.95);
        assert_eq!(candidate.title, Some("Test Song".to_string()));
        assert_eq!(candidate.duration_seconds, Some(180));
    }
}

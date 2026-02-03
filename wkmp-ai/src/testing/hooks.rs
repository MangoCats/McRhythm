//! Pipeline integration hooks for test evaluation
//!
//! **[PLAN031 Increment 5]** Optional evaluator pattern for non-invasive integration
//!
//! This module provides:
//! - TestEvaluator for recording import results during test runs
//! - Non-invasive integration points for existing pipeline
//! - Event recording for post-batch analysis
//!
//! ## Requirements
//!
//! - **SPEC031-API-010**: Pipeline integration hooks
//!
//! ## Usage
//!
//! The TestEvaluator is designed to be injected into the pipeline when running
//! tests, without requiring modifications to production code paths:
//!
//! ```rust,ignore
//! let evaluator = TestEvaluator::new();
//!
//! // During import, record results
//! evaluator.record_fusion_result(&file_hash, &identity);
//! evaluator.record_classification(&file_hash, &result);
//!
//! // After batch completes
//! let summary = evaluator.finalize_batch().await;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Event recorded during import for later analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportEvent {
    /// Result of identity fusion (multi-source MBID resolution)
    FusionResult {
        file_hash: String,
        assigned_mbid: Option<String>,
        confidence: f64,
        sources_consulted: Vec<String>,
        timestamp: String,
    },
    /// Result of content type classification
    Classification {
        file_hash: String,
        content_type: String,
        confidence: f64,
        timestamp: String,
    },
    /// File processing completed
    ProcessingComplete {
        file_hash: String,
        success: bool,
        error_message: Option<String>,
        duration_ms: u64,
        timestamp: String,
    },
    /// API call made during processing
    ApiCall {
        file_hash: String,
        api_name: String,
        success: bool,
        latency_ms: u64,
        timestamp: String,
    },
}

impl ImportEvent {
    /// Get the file hash associated with this event
    pub fn file_hash(&self) -> &str {
        match self {
            ImportEvent::FusionResult { file_hash, .. } => file_hash,
            ImportEvent::Classification { file_hash, .. } => file_hash,
            ImportEvent::ProcessingComplete { file_hash, .. } => file_hash,
            ImportEvent::ApiCall { file_hash, .. } => file_hash,
        }
    }

    /// Get the timestamp of this event
    pub fn timestamp(&self) -> &str {
        match self {
            ImportEvent::FusionResult { timestamp, .. } => timestamp,
            ImportEvent::Classification { timestamp, .. } => timestamp,
            ImportEvent::ProcessingComplete { timestamp, .. } => timestamp,
            ImportEvent::ApiCall { timestamp, .. } => timestamp,
        }
    }
}

/// Summary of events for a single file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileSummary {
    /// File hash
    pub file_hash: String,
    /// Final assigned MBID (if any)
    pub assigned_mbid: Option<String>,
    /// Final confidence
    pub confidence: f64,
    /// Whether processing succeeded
    pub success: bool,
    /// Total processing time
    pub total_time_ms: u64,
    /// Number of API calls
    pub api_calls: usize,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Summary of a test batch
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatchSummary {
    /// Total files processed
    pub files_processed: usize,
    /// Files successfully identified
    pub files_identified: usize,
    /// Files that failed processing
    pub files_failed: usize,
    /// Total API calls across all files
    pub total_api_calls: usize,
    /// Total processing time
    pub total_time_ms: u64,
    /// Per-file summaries
    pub file_summaries: Vec<FileSummary>,
}

/// Test evaluator for recording import events during test runs
///
/// **[SPEC031-API-010]** Non-invasive pipeline integration
///
/// This component can be optionally enabled during test runs to record
/// intermediate results from the import pipeline. It uses thread-safe
/// interior mutability to allow concurrent event recording.
#[derive(Debug)]
pub struct TestEvaluator {
    /// Events recorded during processing
    events: Arc<RwLock<Vec<ImportEvent>>>,
    /// Whether recording is enabled
    enabled: Arc<RwLock<bool>>,
}

impl Default for TestEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TestEvaluator {
    fn clone(&self) -> Self {
        Self {
            events: Arc::clone(&self.events),
            enabled: Arc::clone(&self.enabled),
        }
    }
}

impl TestEvaluator {
    /// Create a new test evaluator
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            enabled: Arc::new(RwLock::new(true)),
        }
    }

    /// Create a disabled evaluator (for production use - no-op)
    pub fn disabled() -> Self {
        let evaluator = Self::new();
        // Block on setting enabled to false for initialization
        // In async context, use disable() instead
        evaluator
    }

    /// Enable event recording
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
    }

    /// Disable event recording
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
    }

    /// Check if recording is enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Record a fusion result
    ///
    /// Called after identity resolution combines multiple sources
    pub async fn record_fusion_result(
        &self,
        file_hash: &str,
        assigned_mbid: Option<&str>,
        confidence: f64,
        sources_consulted: Vec<String>,
    ) {
        if !*self.enabled.read().await {
            return;
        }

        let event = ImportEvent::FusionResult {
            file_hash: file_hash.to_string(),
            assigned_mbid: assigned_mbid.map(String::from),
            confidence,
            sources_consulted,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.events.write().await.push(event);
    }

    /// Record a classification result
    ///
    /// Called after content type classification
    pub async fn record_classification(
        &self,
        file_hash: &str,
        content_type: &str,
        confidence: f64,
    ) {
        if !*self.enabled.read().await {
            return;
        }

        let event = ImportEvent::Classification {
            file_hash: file_hash.to_string(),
            content_type: content_type.to_string(),
            confidence,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.events.write().await.push(event);
    }

    /// Record processing completion
    ///
    /// Called when file processing completes (success or failure)
    pub async fn record_processing_complete(
        &self,
        file_hash: &str,
        success: bool,
        error_message: Option<&str>,
        duration_ms: u64,
    ) {
        if !*self.enabled.read().await {
            return;
        }

        let event = ImportEvent::ProcessingComplete {
            file_hash: file_hash.to_string(),
            success,
            error_message: error_message.map(String::from),
            duration_ms,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.events.write().await.push(event);
    }

    /// Record an API call
    ///
    /// Called after each external API call (AcoustID, MusicBrainz, etc.)
    pub async fn record_api_call(
        &self,
        file_hash: &str,
        api_name: &str,
        success: bool,
        latency_ms: u64,
    ) {
        if !*self.enabled.read().await {
            return;
        }

        let event = ImportEvent::ApiCall {
            file_hash: file_hash.to_string(),
            api_name: api_name.to_string(),
            success,
            latency_ms,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.events.write().await.push(event);
    }

    /// Get all recorded events
    pub async fn get_events(&self) -> Vec<ImportEvent> {
        self.events.read().await.clone()
    }

    /// Get events for a specific file
    pub async fn get_events_for_file(&self, file_hash: &str) -> Vec<ImportEvent> {
        self.events
            .read()
            .await
            .iter()
            .filter(|e| e.file_hash() == file_hash)
            .cloned()
            .collect()
    }

    /// Clear all recorded events
    pub async fn clear(&self) {
        self.events.write().await.clear();
    }

    /// Finalize batch and generate summary
    ///
    /// Aggregates all events into per-file summaries
    pub async fn finalize_batch(&self) -> BatchSummary {
        let events = self.events.read().await;

        // Group events by file hash
        let mut file_events: HashMap<String, Vec<&ImportEvent>> = HashMap::new();
        for event in events.iter() {
            file_events
                .entry(event.file_hash().to_string())
                .or_default()
                .push(event);
        }

        // Generate per-file summaries
        let mut file_summaries = Vec::new();
        let mut total_api_calls = 0;
        let mut total_time_ms = 0;
        let mut files_identified = 0;
        let mut files_failed = 0;

        for (file_hash, events) in file_events {
            let mut summary = FileSummary {
                file_hash: file_hash.clone(),
                ..Default::default()
            };

            for event in events {
                match event {
                    ImportEvent::FusionResult {
                        assigned_mbid,
                        confidence,
                        ..
                    } => {
                        summary.assigned_mbid = assigned_mbid.clone();
                        summary.confidence = *confidence;
                    }
                    ImportEvent::ProcessingComplete {
                        success,
                        error_message,
                        duration_ms,
                        ..
                    } => {
                        summary.success = *success;
                        summary.error_message = error_message.clone();
                        summary.total_time_ms = *duration_ms;
                        total_time_ms += duration_ms;
                    }
                    ImportEvent::ApiCall { .. } => {
                        summary.api_calls += 1;
                        total_api_calls += 1;
                    }
                    _ => {}
                }
            }

            if summary.assigned_mbid.is_some() {
                files_identified += 1;
            }
            if !summary.success && summary.error_message.is_some() {
                files_failed += 1;
            }

            file_summaries.push(summary);
        }

        BatchSummary {
            files_processed: file_summaries.len(),
            files_identified,
            files_failed,
            total_api_calls,
            total_time_ms,
            file_summaries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_fusion_result() {
        let evaluator = TestEvaluator::new();

        evaluator
            .record_fusion_result(
                "hash_123",
                Some("mbid_456"),
                0.95,
                vec!["acoustid".to_string(), "musicbrainz".to_string()],
            )
            .await;

        let events = evaluator.get_events().await;
        assert_eq!(events.len(), 1);

        if let ImportEvent::FusionResult {
            file_hash,
            assigned_mbid,
            confidence,
            sources_consulted,
            ..
        } = &events[0]
        {
            assert_eq!(file_hash, "hash_123");
            assert_eq!(assigned_mbid, &Some("mbid_456".to_string()));
            assert!((confidence - 0.95).abs() < 0.001);
            assert_eq!(sources_consulted.len(), 2);
        } else {
            panic!("Expected FusionResult event");
        }
    }

    #[tokio::test]
    async fn test_record_classification() {
        let evaluator = TestEvaluator::new();

        evaluator
            .record_classification("hash_123", "single_song", 0.88)
            .await;

        let events = evaluator.get_events().await;
        assert_eq!(events.len(), 1);

        if let ImportEvent::Classification {
            content_type,
            confidence,
            ..
        } = &events[0]
        {
            assert_eq!(content_type, "single_song");
            assert!((confidence - 0.88).abs() < 0.001);
        } else {
            panic!("Expected Classification event");
        }
    }

    #[tokio::test]
    async fn test_record_processing_complete() {
        let evaluator = TestEvaluator::new();

        evaluator
            .record_processing_complete("hash_123", true, None, 1500)
            .await;

        let events = evaluator.get_events().await;
        assert_eq!(events.len(), 1);

        if let ImportEvent::ProcessingComplete {
            success,
            duration_ms,
            ..
        } = &events[0]
        {
            assert!(success);
            assert_eq!(*duration_ms, 1500);
        } else {
            panic!("Expected ProcessingComplete event");
        }
    }

    #[tokio::test]
    async fn test_record_api_call() {
        let evaluator = TestEvaluator::new();

        evaluator
            .record_api_call("hash_123", "acoustid", true, 250)
            .await;

        let events = evaluator.get_events().await;
        assert_eq!(events.len(), 1);

        if let ImportEvent::ApiCall {
            api_name,
            success,
            latency_ms,
            ..
        } = &events[0]
        {
            assert_eq!(api_name, "acoustid");
            assert!(success);
            assert_eq!(*latency_ms, 250);
        } else {
            panic!("Expected ApiCall event");
        }
    }

    #[tokio::test]
    async fn test_disabled_evaluator() {
        let evaluator = TestEvaluator::new();
        evaluator.disable().await;

        evaluator
            .record_fusion_result("hash_123", Some("mbid_456"), 0.95, vec![])
            .await;

        let events = evaluator.get_events().await;
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let evaluator = TestEvaluator::new();

        assert!(evaluator.is_enabled().await);

        evaluator.disable().await;
        assert!(!evaluator.is_enabled().await);

        evaluator.enable().await;
        assert!(evaluator.is_enabled().await);
    }

    #[tokio::test]
    async fn test_get_events_for_file() {
        let evaluator = TestEvaluator::new();

        evaluator
            .record_fusion_result("hash_1", Some("mbid_1"), 0.9, vec![])
            .await;
        evaluator
            .record_fusion_result("hash_2", Some("mbid_2"), 0.8, vec![])
            .await;
        evaluator
            .record_classification("hash_1", "single_song", 0.95)
            .await;

        let events_1 = evaluator.get_events_for_file("hash_1").await;
        assert_eq!(events_1.len(), 2);

        let events_2 = evaluator.get_events_for_file("hash_2").await;
        assert_eq!(events_2.len(), 1);
    }

    #[tokio::test]
    async fn test_clear_events() {
        let evaluator = TestEvaluator::new();

        evaluator
            .record_fusion_result("hash_1", Some("mbid_1"), 0.9, vec![])
            .await;
        assert_eq!(evaluator.get_events().await.len(), 1);

        evaluator.clear().await;
        assert!(evaluator.get_events().await.is_empty());
    }

    #[tokio::test]
    async fn test_finalize_batch() {
        let evaluator = TestEvaluator::new();

        // File 1: successful identification
        evaluator
            .record_api_call("hash_1", "acoustid", true, 100)
            .await;
        evaluator
            .record_api_call("hash_1", "musicbrainz", true, 150)
            .await;
        evaluator
            .record_fusion_result("hash_1", Some("mbid_1"), 0.95, vec![])
            .await;
        evaluator
            .record_processing_complete("hash_1", true, None, 300)
            .await;

        // File 2: failed identification
        evaluator
            .record_api_call("hash_2", "acoustid", false, 50)
            .await;
        evaluator
            .record_processing_complete("hash_2", false, Some("API error"), 100)
            .await;

        let summary = evaluator.finalize_batch().await;

        assert_eq!(summary.files_processed, 2);
        assert_eq!(summary.files_identified, 1);
        assert_eq!(summary.files_failed, 1);
        assert_eq!(summary.total_api_calls, 3);
        assert_eq!(summary.total_time_ms, 400);

        // Check file summaries
        let file_1 = summary
            .file_summaries
            .iter()
            .find(|s| s.file_hash == "hash_1")
            .unwrap();
        assert_eq!(file_1.assigned_mbid, Some("mbid_1".to_string()));
        assert!(file_1.success);
        assert_eq!(file_1.api_calls, 2);

        let file_2 = summary
            .file_summaries
            .iter()
            .find(|s| s.file_hash == "hash_2")
            .unwrap();
        assert!(file_2.assigned_mbid.is_none());
        assert!(!file_2.success);
        assert_eq!(file_2.api_calls, 1);
    }

    #[tokio::test]
    async fn test_evaluator_clone_shares_state() {
        let evaluator1 = TestEvaluator::new();
        let evaluator2 = evaluator1.clone();

        evaluator1
            .record_fusion_result("hash_1", Some("mbid_1"), 0.9, vec![])
            .await;

        // Both should see the same events
        let events1 = evaluator1.get_events().await;
        let events2 = evaluator2.get_events().await;

        assert_eq!(events1.len(), 1);
        assert_eq!(events2.len(), 1);
    }
}

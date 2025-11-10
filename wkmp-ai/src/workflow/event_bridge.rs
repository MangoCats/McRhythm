//! Workflow event bridge for SSE broadcasting
//!
//! Converts internal `WorkflowEvent`s to public `WkmpEvent`s for Server-Sent Events.
//!
//! **[PLAN023]** [REQ-AI-090] SSE event broadcasting integration
//!
//! # Purpose
//!
//! The workflow engine emits detailed internal events during import processing.
//! This bridge converts those events to the standardized `WkmpEvent` format
//! that the EventBus broadcasts to connected UI clients via SSE.
//!
//! # SPEC017 Compliance
//!
//! Converts internal tick-based timestamps to user-facing seconds for SSE events,
//! as ticks are an internal precision representation not exposed to clients.

use super::{WorkflowEvent, TICK_RATE};
use chrono::Utc;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info};
use uuid::Uuid;
use wkmp_common::events::WkmpEvent;

/// Bridge task that forwards workflow events to EventBus
///
/// # Arguments
/// * `workflow_rx` - Receiver for workflow events from SongProcessor
/// * `event_bus_tx` - EventBus broadcast sender
/// * `session_id` - Import session UUID for event correlation
pub async fn bridge_workflow_events(
    mut workflow_rx: mpsc::Receiver<WorkflowEvent>,
    event_bus_tx: broadcast::Sender<WkmpEvent>,
    session_id: Uuid,
) {
    info!("Workflow event bridge started for session {}", session_id);

    let start_time = std::time::Instant::now();
    let mut total_passages = 0;
    let mut processed_passages = 0;

    while let Some(event) = workflow_rx.recv().await {
        debug!("Bridge: Received workflow event: {:?}", event);

        // Convert WorkflowEvent to WkmpEvent::ImportProgressUpdate
        let wkmp_event = match event {
            WorkflowEvent::FileStarted { file_path, .. } => {
                info!("Bridge: File processing started: {}", file_path);
                Some(WkmpEvent::ImportProgressUpdate {
                    session_id,
                    state: "PROCESSING".to_string(),
                    current: processed_passages,
                    total: total_passages.max(1), // Avoid division by zero
                    percentage: 0.0,
                    current_operation: format!("Starting file: {}",
                        std::path::Path::new(&file_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&file_path)
                    ),
                    elapsed_seconds: start_time.elapsed().as_secs(),
                    estimated_remaining_seconds: None,
                    phases: Vec::new(),
                    current_file: Some(file_path),
                    timestamp: Utc::now(),
                })
            }

            WorkflowEvent::BoundaryDetected {
                passage_index,
                start_time: start_ticks,
                end_time: end_ticks,
                confidence,
            } => {
                // Convert SPEC017 ticks to seconds for user-facing display
                let start_seconds = start_ticks as f64 / TICK_RATE as f64;
                let end_seconds = end_ticks as f64 / TICK_RATE as f64;

                if passage_index == 0 {
                    // First boundary detected - update total count estimate
                    debug!("Bridge: First passage boundary detected");
                }
                Some(WkmpEvent::ImportProgressUpdate {
                    session_id,
                    state: "SEGMENTING".to_string(),
                    current: passage_index,
                    total: total_passages.max(1),
                    percentage: 0.0,
                    current_operation: format!(
                        "Detected passage {} ({:.1}s - {:.1}s, confidence: {:.0}%)",
                        passage_index + 1,
                        start_seconds,
                        end_seconds,
                        confidence * 100.0
                    ),
                    elapsed_seconds: start_time.elapsed().as_secs(),
                    estimated_remaining_seconds: None,
                    phases: Vec::new(),
                    current_file: None,
                    timestamp: Utc::now(),
                })
            }

            WorkflowEvent::PassageStarted {
                passage_index,
                total_passages: total,
            } => {
                total_passages = total;
                info!(
                    "Bridge: Passage {} of {} started",
                    passage_index + 1,
                    total
                );
                Some(WkmpEvent::ImportProgressUpdate {
                    session_id,
                    state: "EXTRACTING".to_string(),
                    current: passage_index,
                    total,
                    percentage: (passage_index as f32 / total as f32) * 100.0,
                    current_operation: format!("Processing passage {} of {}", passage_index + 1, total),
                    elapsed_seconds: start_time.elapsed().as_secs(),
                    estimated_remaining_seconds: None,
                    phases: Vec::new(),
                    current_file: None,
                    timestamp: Utc::now(),
                })
            }

            WorkflowEvent::ExtractionProgress {
                passage_index,
                extractor,
                status,
            } => {
                debug!(
                    "Bridge: Extraction progress - passage {}, extractor: {}, status: {}",
                    passage_index + 1,
                    extractor,
                    status
                );
                Some(WkmpEvent::ImportProgressUpdate {
                    session_id,
                    state: "EXTRACTING".to_string(),
                    current: passage_index,
                    total: total_passages.max(1),
                    percentage: (passage_index as f32 / total_passages.max(1) as f32) * 100.0,
                    current_operation: format!(
                        "Passage {}/{}: {} - {}",
                        passage_index + 1,
                        total_passages,
                        extractor,
                        status
                    ),
                    elapsed_seconds: start_time.elapsed().as_secs(),
                    estimated_remaining_seconds: None,
                    phases: Vec::new(),
                    current_file: None,
                    timestamp: Utc::now(),
                })
            }

            WorkflowEvent::FusionStarted { passage_index } => {
                debug!("Bridge: Fusion started for passage {}", passage_index + 1);
                Some(WkmpEvent::ImportProgressUpdate {
                    session_id,
                    state: "FUSING".to_string(),
                    current: passage_index,
                    total: total_passages.max(1),
                    percentage: (passage_index as f32 / total_passages.max(1) as f32) * 100.0,
                    current_operation: format!(
                        "Passage {}/{}: Fusing metadata and flavor",
                        passage_index + 1,
                        total_passages
                    ),
                    elapsed_seconds: start_time.elapsed().as_secs(),
                    estimated_remaining_seconds: None,
                    phases: Vec::new(),
                    current_file: None,
                    timestamp: Utc::now(),
                })
            }

            WorkflowEvent::ValidationStarted { passage_index } => {
                debug!("Bridge: Validation started for passage {}", passage_index + 1);
                Some(WkmpEvent::ImportProgressUpdate {
                    session_id,
                    state: "VALIDATING".to_string(),
                    current: passage_index,
                    total: total_passages.max(1),
                    percentage: (passage_index as f32 / total_passages.max(1) as f32) * 100.0,
                    current_operation: format!(
                        "Passage {}/{}: Validating quality",
                        passage_index + 1,
                        total_passages
                    ),
                    elapsed_seconds: start_time.elapsed().as_secs(),
                    estimated_remaining_seconds: None,
                    phases: Vec::new(),
                    current_file: None,
                    timestamp: Utc::now(),
                })
            }

            WorkflowEvent::PassageCompleted {
                passage_index,
                quality_score,
                validation_status,
            } => {
                processed_passages = passage_index + 1;
                info!(
                    "Bridge: Passage {} completed - quality: {:.1}%, status: {}",
                    passage_index + 1,
                    quality_score,
                    validation_status
                );
                Some(WkmpEvent::ImportProgressUpdate {
                    session_id,
                    state: "PROCESSING".to_string(),
                    current: processed_passages,
                    total: total_passages,
                    percentage: (processed_passages as f32 / total_passages as f32) * 100.0,
                    current_operation: format!(
                        "Completed passage {} of {} (quality: {:.0}%)",
                        processed_passages,
                        total_passages,
                        quality_score
                    ),
                    elapsed_seconds: start_time.elapsed().as_secs(),
                    estimated_remaining_seconds: {
                        if processed_passages > 0 {
                            let avg_time_per_passage = start_time.elapsed().as_secs() / processed_passages as u64;
                            let remaining = total_passages.saturating_sub(processed_passages);
                            Some(avg_time_per_passage * remaining as u64)
                        } else {
                            None
                        }
                    },
                    phases: Vec::new(),
                    current_file: None,
                    timestamp: Utc::now(),
                })
            }

            WorkflowEvent::FileCompleted {
                file_path,
                passages_processed,
                ..
            } => {
                info!(
                    "Bridge: File completed: {} ({} passages)",
                    file_path, passages_processed
                );
                Some(WkmpEvent::ImportProgressUpdate {
                    session_id,
                    state: "COMPLETED".to_string(),
                    current: passages_processed,
                    total: passages_processed,
                    percentage: 100.0,
                    current_operation: format!(
                        "Completed: {} ({} passages processed)",
                        std::path::Path::new(&file_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&file_path),
                        passages_processed
                    ),
                    elapsed_seconds: start_time.elapsed().as_secs(),
                    estimated_remaining_seconds: Some(0),
                    phases: Vec::new(),
                    current_file: None,
                    timestamp: Utc::now(),
                })
            }

            WorkflowEvent::Error {
                passage_index,
                message,
            } => {
                error!("Bridge: Workflow error - passage {:?}: {}", passage_index, message);
                Some(WkmpEvent::ImportProgressUpdate {
                    session_id,
                    state: "ERROR".to_string(),
                    current: passage_index.unwrap_or(0),
                    total: total_passages.max(1),
                    percentage: (passage_index.unwrap_or(0) as f32 / total_passages.max(1) as f32) * 100.0,
                    current_operation: format!(
                        "Error{}: {}",
                        passage_index.map(|i| format!(" in passage {}", i + 1)).unwrap_or_default(),
                        message
                    ),
                    elapsed_seconds: start_time.elapsed().as_secs(),
                    estimated_remaining_seconds: None,
                    phases: Vec::new(),
                    current_file: None,
                    timestamp: Utc::now(),
                })
            }
        };

        // Broadcast to EventBus
        if let Some(event) = wkmp_event {
            match event_bus_tx.send(event) {
                Ok(receiver_count) => {
                    debug!("Bridge: Event broadcast to {} receivers", receiver_count);
                }
                Err(e) => {
                    error!("Bridge: Failed to broadcast event: {}", e);
                }
            }
        }
    }

    info!("Workflow event bridge completed for session {}", session_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bridge_creation() {
        let (workflow_tx, workflow_rx) = mpsc::channel(100);
        let (event_bus_tx, _event_bus_rx) = broadcast::channel(100);
        let session_id = Uuid::new_v4();

        // Spawn bridge task
        let bridge_handle = tokio::spawn(bridge_workflow_events(
            workflow_rx,
            event_bus_tx.clone(),
            session_id,
        ));

        // Send a test event
        workflow_tx
            .send(WorkflowEvent::FileStarted {
                file_path: "/test/file.mp3".to_string(),
                timestamp: 0,
            })
            .await
            .unwrap();

        // Drop sender to complete bridge
        drop(workflow_tx);

        // Wait for bridge to complete
        bridge_handle.await.unwrap();
    }
}

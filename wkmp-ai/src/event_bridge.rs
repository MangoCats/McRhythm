//! Event bridge for converting import_v2 events to legacy WkmpEvent
//!
//! **[PLAN024 Event Integration]** Bridge between import_v2::ImportEvent and wkmp_common::WkmpEvent
//!
//! This bridge allows the import_v2 architecture to emit ImportEvent while maintaining
//! backward compatibility with the existing SSE infrastructure that expects WkmpEvent.
//!
//! **Architecture:**
//! - Subscribes to `import_event_tx` (import_v2 events)
//! - Converts ImportEvent → WkmpEvent
//! - Publishes to EventBus (legacy SSE infrastructure)
//!
//! **Migration Path:**
//! This is temporary scaffolding. As other modules migrate to import_v2 events,
//! the bridge can be incrementally simplified and eventually removed.

use crate::import_v2::types::ImportEvent;
use tokio::sync::broadcast;
use tracing::{debug, warn};
use wkmp_common::events::{EventBus, WkmpEvent};

/// Bridge task that converts ImportEvent to WkmpEvent
///
/// Runs as a background task, forwarding events from import_v2 channel to EventBus.
///
/// # Arguments
/// * `import_rx` - Receiver for ImportEvent from import_v2 workflow
/// * `event_bus` - EventBus for legacy WkmpEvent broadcasting
///
/// # Behavior
/// - Logs conversion errors but continues processing
/// - If EventBus emit fails (no subscribers), logs warning and drops event
/// - Runs indefinitely until import_rx sender is dropped
pub async fn run_event_bridge(
    mut import_rx: broadcast::Receiver<ImportEvent>,
    event_bus: EventBus,
) {
    debug!("Event bridge started (ImportEvent → WkmpEvent)");

    loop {
        match import_rx.recv().await {
            Ok(import_event) => {
                // Convert ImportEvent to WkmpEvent
                match convert_import_event(import_event) {
                    Some(wkmp_event) => {
                        // Emit to EventBus (not async - uses tokio::broadcast internally)
                        if let Err(e) = event_bus.emit(wkmp_event) {
                            warn!("Event bridge: Failed to emit to EventBus: {}", e);
                            // Continue processing - SSE has heartbeat for connection monitoring
                        }
                    }
                    None => {
                        // Event not mapped (intentional - some ImportEvent variants are internal only)
                        debug!("Event bridge: ImportEvent variant not mapped to WkmpEvent (internal-only event)");
                    }
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                warn!(
                    "Event bridge: Lagged {} events (channel too slow - increase buffer or reduce event rate)",
                    skipped
                );
                // Continue processing - skip the lagged events
            }
            Err(broadcast::error::RecvError::Closed) => {
                debug!("Event bridge: import_event_tx sender dropped, shutting down");
                break;
            }
        }
    }

    debug!("Event bridge stopped");
}

/// Convert ImportEvent to WkmpEvent
///
/// # Returns
/// - `Some(WkmpEvent)` if conversion succeeds
/// - `None` if event is internal-only (no WkmpEvent equivalent)
///
/// # Event Mappings
/// - `SessionStarted` → `ImportSessionStarted`
/// - `SessionFailed` → `ImportSessionFailed`
/// - `PassagesDiscovered`, `SongStarted`, `ExtractionComplete`, etc. → `ImportProgressUpdate`
fn convert_import_event(event: ImportEvent) -> Option<WkmpEvent> {
    let now = chrono::Utc::now();

    match event {
        ImportEvent::SessionStarted {
            session_id,
            root_folder,
        } => Some(WkmpEvent::ImportSessionStarted {
            session_id,
            root_folder,
            timestamp: now,
        }),

        ImportEvent::SessionFailed { session_id, error } => {
            Some(WkmpEvent::ImportSessionFailed {
                session_id,
                error_message: error,
                files_processed: 0, // Not tracked at session level yet
                timestamp: now,
            })
        }

        ImportEvent::PassagesDiscovered { file_path, count } => {
            // Convert to progress update
            Some(WkmpEvent::ImportProgressUpdate {
                session_id: uuid::Uuid::nil(), // TODO: Add session_id to PassagesDiscovered event
                state: "BOUNDARY_DETECTION".to_string(),
                current: count,
                total: count, // Approximate - actual total unknown at this point
                percentage: 0.0,
                current_operation: format!("Detected {} passages in file", count),
                elapsed_seconds: 0, // Not tracked yet
                estimated_remaining_seconds: None,
                phases: vec![], // Empty for backward compatibility
                current_file: Some(file_path),
                timestamp: now,
            })
        }

        ImportEvent::SongStarted {
            song_index,
            total_songs,
        } => Some(WkmpEvent::ImportProgressUpdate {
            session_id: uuid::Uuid::nil(), // TODO: Add session_id to SongStarted event
            state: "PROCESSING".to_string(),
            current: song_index,
            total: total_songs,
            percentage: (song_index as f32 / total_songs as f32) * 100.0,
            current_operation: format!("Processing song {} of {}", song_index, total_songs),
            elapsed_seconds: 0,
            estimated_remaining_seconds: None,
            phases: vec![],
            current_file: None,
            timestamp: now,
        }),

        ImportEvent::ExtractionComplete {
            song_index,
            sources,
        } => {
            // Granular progress update
            Some(WkmpEvent::ImportProgressUpdate {
                session_id: uuid::Uuid::nil(), // TODO: Add session_id
                state: format!("EXTRACTING ({} sources)", sources.len()),
                current: song_index,
                total: song_index + 1, // Approximate
                percentage: 0.0,
                current_operation: format!("Extracted data from {} sources", sources.len()),
                elapsed_seconds: 0,
                estimated_remaining_seconds: None,
                phases: vec![],
                current_file: None,
                timestamp: now,
            })
        }

        ImportEvent::FusionComplete {
            song_index,
            identity_confidence,
            metadata_confidence,
            flavor_confidence,
        } => {
            // Detailed fusion progress
            Some(WkmpEvent::ImportProgressUpdate {
                session_id: uuid::Uuid::nil(), // TODO: Add session_id
                state: format!(
                    "FUSING (ID:{:.0}% MD:{:.0}% FL:{:.0}%)",
                    identity_confidence * 100.0,
                    metadata_confidence * 100.0,
                    flavor_confidence * 100.0
                ),
                current: song_index,
                total: song_index + 1, // Approximate
                percentage: 0.0,
                current_operation: format!(
                    "Fused metadata (identity:{:.0}%, metadata:{:.0}%, flavor:{:.0}%)",
                    identity_confidence * 100.0,
                    metadata_confidence * 100.0,
                    flavor_confidence * 100.0
                ),
                elapsed_seconds: 0,
                estimated_remaining_seconds: None,
                phases: vec![],
                current_file: None,
                timestamp: now,
            })
        }

        ImportEvent::ValidationComplete {
            song_index,
            quality_score,
            has_conflicts,
        } => Some(WkmpEvent::ImportProgressUpdate {
            session_id: uuid::Uuid::nil(), // TODO: Add session_id
            state: if has_conflicts {
                format!("VALIDATING (quality:{:.0}% CONFLICTS)", quality_score * 100.0)
            } else {
                format!("VALIDATING (quality:{:.0}%)", quality_score * 100.0)
            },
            current: song_index,
            total: song_index + 1, // Approximate
            percentage: 0.0,
            current_operation: if has_conflicts {
                format!("Validation complete with conflicts (quality:{:.0}%)", quality_score * 100.0)
            } else {
                format!("Validation complete (quality:{:.0}%)", quality_score * 100.0)
            },
            elapsed_seconds: 0,
            estimated_remaining_seconds: None,
            phases: vec![],
            current_file: None,
            timestamp: now,
        }),

        ImportEvent::SongComplete {
            song_index,
            duration_ms,
        } => Some(WkmpEvent::ImportProgressUpdate {
            session_id: uuid::Uuid::nil(), // TODO: Add session_id
            state: format!("COMPLETED ({}ms)", duration_ms),
            current: song_index + 1, // Move to next
            total: song_index + 1,   // Approximate
            percentage: 0.0,
            current_operation: format!("Song completed in {}ms", duration_ms),
            elapsed_seconds: 0,
            estimated_remaining_seconds: None,
            phases: vec![],
            current_file: None,
            timestamp: now,
        }),

        ImportEvent::SongFailed { song_index, error } => {
            Some(WkmpEvent::ImportProgressUpdate {
                session_id: uuid::Uuid::nil(), // TODO: Add session_id
                state: format!("FAILED: {}", error),
                current: song_index,
                total: song_index + 1, // Approximate
                percentage: 0.0,
                current_operation: format!("Song failed: {}", error),
                elapsed_seconds: 0,
                estimated_remaining_seconds: None,
                phases: vec![],
                current_file: None,
                timestamp: now,
            })
        }

        ImportEvent::FileComplete {
            file_path,
            successes,
            warnings,
            failures,
            total_duration_ms,
        } => Some(WkmpEvent::ImportProgressUpdate {
            session_id: uuid::Uuid::nil(), // TODO: Add session_id
            state: format!(
                "FILE_COMPLETE (✓{} ⚠{} ✗{} {}ms)",
                successes, warnings, failures, total_duration_ms
            ),
            current: successes + failures,
            total: successes + failures,
            percentage: 100.0, // File complete
            current_operation: format!(
                "File complete: {} successes, {} warnings, {} failures in {}ms",
                successes, warnings, failures, total_duration_ms
            ),
            elapsed_seconds: (total_duration_ms / 1000) as u64,
            estimated_remaining_seconds: None,
            phases: vec![],
            current_file: Some(file_path),
            timestamp: now,
        }),

        // Future events (not yet implemented in SessionOrchestrator)
        // When SessionCompleted event is added to ImportEvent enum, map it here:
        // ImportEvent::SessionCompleted { session_id, passages_processed, duration_ms } => {
        //     Some(WkmpEvent::ImportSessionCompleted {
        //         session_id,
        //         files_processed: passages_processed,
        //         duration_seconds: duration_ms / 1000,
        //         timestamp: now,
        //     })
        // }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_session_started() {
        let session_id = uuid::Uuid::new_v4();
        let import_event = ImportEvent::SessionStarted {
            session_id,
            root_folder: "/music".to_string(),
        };

        let wkmp_event = convert_import_event(import_event).expect("Should convert");

        match wkmp_event {
            WkmpEvent::ImportSessionStarted {
                session_id: id,
                root_folder,
                ..
            } => {
                assert_eq!(id, session_id);
                assert_eq!(root_folder, "/music");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_convert_session_failed() {
        let session_id = uuid::Uuid::new_v4();
        let import_event = ImportEvent::SessionFailed {
            session_id,
            error: "Test error".to_string(),
        };

        let wkmp_event = convert_import_event(import_event).expect("Should convert");

        match wkmp_event {
            WkmpEvent::ImportSessionFailed {
                session_id: id,
                error_message,
                ..
            } => {
                assert_eq!(id, session_id);
                assert_eq!(error_message, "Test error");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_convert_song_started() {
        let import_event = ImportEvent::SongStarted {
            song_index: 5,
            total_songs: 10,
        };

        let wkmp_event = convert_import_event(import_event).expect("Should convert");

        match wkmp_event {
            WkmpEvent::ImportProgressUpdate {
                state,
                current,
                total,
                percentage,
                ..
            } => {
                assert_eq!(state, "PROCESSING");
                assert_eq!(current, 5);
                assert_eq!(total, 10);
                assert!((percentage - 50.0).abs() < 0.1); // 5/10 = 50%
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_convert_passages_discovered() {
        let import_event = ImportEvent::PassagesDiscovered {
            file_path: "/music/test.mp3".to_string(),
            count: 3,
        };

        let wkmp_event = convert_import_event(import_event).expect("Should convert");

        match wkmp_event {
            WkmpEvent::ImportProgressUpdate {
                state,
                current_file,
                ..
            } => {
                assert_eq!(state, "BOUNDARY_DETECTION");
                assert_eq!(current_file, Some("/music/test.mp3".to_string()));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_convert_file_complete() {
        let import_event = ImportEvent::FileComplete {
            file_path: "/music/album.flac".to_string(),
            successes: 10,
            warnings: 2,
            failures: 1,
            total_duration_ms: 5000,
        };

        let wkmp_event = convert_import_event(import_event).expect("Should convert");

        match wkmp_event {
            WkmpEvent::ImportProgressUpdate {
                state,
                current_file,
                percentage,
                elapsed_seconds,
                ..
            } => {
                assert!(state.contains("✓10"));
                assert!(state.contains("⚠2"));
                assert!(state.contains("✗1"));
                assert!(state.contains("5000ms"));
                assert_eq!(current_file, Some("/music/album.flac".to_string()));
                assert_eq!(percentage, 100.0);
                assert_eq!(elapsed_seconds, 5); // 5000ms = 5s
            }
            _ => panic!("Wrong event type"),
        }
    }
}

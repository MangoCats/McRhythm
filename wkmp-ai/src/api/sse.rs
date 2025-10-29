//! Server-Sent Events (SSE) for import progress streaming
//!
//! **[AIA-MS-010]** SSE event streaming for real-time import progress updates

use crate::AppState;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::time::Duration;
use tracing::{debug, info, warn};
use wkmp_common::events::WkmpEvent;

/// GET /import/events - SSE event stream for import progress
///
/// **[AIA-MS-010]** Real-time progress updates for import workflow
///
/// Streams events:
/// - ImportSessionStarted
/// - ImportProgressUpdate (during workflow progression)
/// - ImportSessionCompleted
/// - ImportSessionFailed
/// - ImportSessionCancelled
pub async fn import_event_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New SSE client connected to import events");

    // Subscribe to event broadcast
    let mut rx = state.event_bus.subscribe();

    // Create stream that forwards import events
    let stream = async_stream::stream! {
        info!("SSE: Import event stream started");

        loop {
            tokio::select! {
                // Heartbeat every 15 seconds
                _ = tokio::time::sleep(Duration::from_secs(15)) => {
                    debug!("SSE: Sending heartbeat");
                    yield Ok(Event::default().comment("heartbeat"));
                }

                // Broadcast events
                Ok(event) = rx.recv() => {
                    // Filter for import-related events only
                    match &event {
                        WkmpEvent::ImportSessionStarted { .. }
                        | WkmpEvent::ImportProgressUpdate { .. }
                        | WkmpEvent::ImportSessionCompleted { .. }
                        | WkmpEvent::ImportSessionFailed { .. }
                        | WkmpEvent::ImportSessionCancelled { .. } => {
                            let event_type = event.event_type();

                            match serde_json::to_string(&event) {
                                Ok(event_json) => {
                                    debug!("SSE: Broadcasting import event: {}", event_type);
                                    yield Ok(Event::default()
                                        .event(event_type)
                                        .data(event_json));
                                }
                                Err(e) => {
                                    warn!("SSE: Failed to serialize event {}: {}", event_type, e);
                                }
                            }
                        }
                        _ => {
                            // Ignore non-import events
                        }
                    }
                }
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat")
    )
}

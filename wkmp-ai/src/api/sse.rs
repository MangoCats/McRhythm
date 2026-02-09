//! Server-Sent Events (SSE) for import progress streaming
//!
//! **[AIA-MS-010]** SSE event streaming for real-time import progress updates
//!
//! **Throttling:** Import progress events are throttled to max 1/second to prevent
//! broadcast channel overflow. Lifecycle events (Started/Completed/Failed/Cancelled)
//! are always sent immediately. When the broadcast channel lags (producer outpaces
//! consumer), the receiver recovers automatically and logs a warning.

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

/// GET /events - SSE event stream for general connection status
///
/// **[AIA-SSE-020]** Connection status monitoring
///
/// Streams heartbeat events for connection status monitoring
pub async fn event_stream(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    wkmp_common::sse::create_heartbeat_sse_stream("wkmp-ai")
}

/// GET /import/events - SSE event stream for import progress
///
/// **[AIA-MS-010]** Real-time progress updates for import workflow
///
/// Streams events:
/// - ImportSessionStarted (immediate)
/// - ImportProgressUpdate (throttled to 1/second)
/// - ImportSessionCompleted (immediate)
/// - ImportSessionFailed (immediate)
/// - ImportSessionCancelled (immediate)
/// - AnalysisLog (throttled to 1/second)
pub async fn import_event_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New SSE client connected to import events");

    // Subscribe to event broadcast
    let mut rx = state.event_bus.subscribe();

    // Create stream that forwards import events with 1/s throttling
    let stream = async_stream::stream! {
        info!("SSE: Import event stream started");
        let mut last_event_time = tokio::time::Instant::now() - Duration::from_secs(1);
        let mut pending_event: Option<WkmpEvent> = None;

        loop {
            tokio::select! {
                // 1-second tick: flush pending event or send heartbeat
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    if let Some(event) = pending_event.take() {
                        let event_type = event.event_type();
                        match serde_json::to_string(&event) {
                            Ok(event_json) => {
                                debug!("SSE: Broadcasting {} (throttled)", event_type);
                                yield Ok(Event::default().event(event_type).data(event_json));
                                last_event_time = tokio::time::Instant::now();
                            }
                            Err(e) => {
                                warn!("SSE: Failed to serialize event {}: {}", event_type, e);
                            }
                        }
                    } else {
                        debug!("SSE: Sending heartbeat");
                        yield Ok(Event::default().comment("heartbeat"));
                    }
                }

                // Broadcast events from EventBus
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            match &event {
                                WkmpEvent::ImportSessionStarted { .. }
                                | WkmpEvent::ImportSessionCompleted { .. }
                                | WkmpEvent::ImportSessionFailed { .. }
                                | WkmpEvent::ImportSessionCancelled { .. } => {
                                    // Lifecycle events: always send immediately
                                    let event_type = event.event_type();
                                    match serde_json::to_string(&event) {
                                        Ok(event_json) => {
                                            debug!("SSE: Broadcasting {} (immediate)", event_type);
                                            yield Ok(Event::default()
                                                .event(event_type)
                                                .data(event_json));
                                            last_event_time = tokio::time::Instant::now();
                                        }
                                        Err(e) => {
                                            warn!("SSE: serialize error: {}", e);
                                        }
                                    }
                                }
                                WkmpEvent::ImportProgressUpdate { .. }
                                | WkmpEvent::AnalysisLog(_) => {
                                    // Throttleable: send if >1s elapsed, else buffer latest
                                    if last_event_time.elapsed() >= Duration::from_secs(1) {
                                        let event_type = event.event_type();
                                        match serde_json::to_string(&event) {
                                            Ok(event_json) => {
                                                debug!("SSE: Broadcasting {}", event_type);
                                                yield Ok(Event::default()
                                                    .event(event_type)
                                                    .data(event_json));
                                                last_event_time = tokio::time::Instant::now();
                                            }
                                            Err(e) => {
                                                warn!("SSE: serialize error: {}", e);
                                            }
                                        }
                                    } else {
                                        // <1s since last send — buffer (latest replaces pending)
                                        pending_event = Some(event);
                                    }
                                }
                                _ => {
                                    // Ignore non-import events
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            warn!("SSE: Receiver lagged, skipped {} events", n);
                            // Receiver auto-repositions; continue receiving
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            info!("SSE: Event bus closed, ending stream");
                            break;
                        }
                    }
                }
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    )
}

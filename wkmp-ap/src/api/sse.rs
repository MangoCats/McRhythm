//! Server-Sent Events (SSE) broadcaster
//!
//! Streams real-time playback events to connected clients.
//!
//! **Traceability:**
//! - API Design - SSE Event Formats
//! - Event System - EventBus subscription

use crate::api::server::AppContext;
use axum::{
    response::sse::{Event, Sse},
    extract::State,
};
use futures::stream::{Stream, StreamExt};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, warn};

/// GET /events - SSE event stream
///
/// **Traceability:** API Design - GET /events endpoint
pub async fn event_stream(
    State(ctx): State<AppContext>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    debug!("New SSE client connected");

    // Subscribe to event broadcast
    let rx = ctx.state.subscribe_events();

    // Convert broadcast receiver to stream
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| async move {
            match result {
                Ok(event) => {
                    // Serialize event to JSON
                    match serde_json::to_string(&event) {
                        Ok(json) => {
                            // Extract event type for SSE event field
                            let event_type = event_type_str(&event);
                            debug!("Broadcasting SSE event: {}", event_type);

                            Some(Ok(Event::default()
                                .event(event_type)
                                .data(json)))
                        }
                        Err(e) => {
                            warn!("Failed to serialize event: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    // BroadcastStream error (lagged or closed)
                    warn!("SSE stream error: {:?}", e);
                    None
                }
            }
        });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

/// Extract event type string from WkmpEvent
fn event_type_str(event: &wkmp_common::events::WkmpEvent) -> &'static str {
    use wkmp_common::events::WkmpEvent;
    match event {
        WkmpEvent::PlaybackStateChanged { .. } => "PlaybackStateChanged",
        WkmpEvent::PassageStarted { .. } => "PassageStarted",
        WkmpEvent::PassageCompleted { .. } => "PassageCompleted",
        WkmpEvent::CurrentSongChanged { .. } => "CurrentSongChanged",
        WkmpEvent::PlaybackProgress { .. } => "PlaybackProgress",
        WkmpEvent::QueueChanged { .. } => "QueueChanged",
        WkmpEvent::VolumeChanged { .. } => "VolumeChanged",
    }
}

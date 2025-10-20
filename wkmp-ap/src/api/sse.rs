//! Server-Sent Events (SSE) broadcaster
//!
//! Streams real-time playback events to connected clients.
//!
//! **Traceability:**
//! - API Design - SSE Event Formats
//! - Event System - EventBus subscription
//! - SPEC019 - SSE-Based Developer UI

use crate::api::server::AppContext;
use crate::state::PlaybackState;
use axum::{
    response::sse::{Event, Sse},
    extract::State,
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::time::Duration;
use tracing::{debug, info, warn};
use wkmp_common::events::{WkmpEvent, QueueEntryInfo, PlaybackPositionInfo};
use chrono::Utc;

/// GET /events - SSE event stream with initial state
///
/// **Traceability:**
/// - API Design - GET /events endpoint
/// - [SSE-UI-010] Unified Event Stream
/// - [SSE-UI-050] Initial State on Connection
pub async fn event_stream(
    State(ctx): State<AppContext>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New SSE client connected");

    // Subscribe to event broadcast
    let mut rx = ctx.state.subscribe_events();

    // === FETCH INITIAL STATE BEFORE CREATING STREAM ===
    // [SSE-UI-050] Fetch all initial state data outside the async_stream macro
    // to avoid deadlocks on RwLock reads inside the stream generator
    debug!("SSE: Fetching initial state data");

    debug!("SSE: About to call get_queue_entries()");
    let queue_entries = ctx.engine.get_queue_entries().await;
    debug!("SSE: get_queue_entries() returned {} entries", queue_entries.len());
    let queue_info: Vec<QueueEntryInfo> = queue_entries.into_iter()
        .map(|e| QueueEntryInfo {
            queue_entry_id: e.queue_entry_id,
            passage_id: e.passage_id,
            file_path: e.file_path.to_string_lossy().to_string(),
        })
        .collect();

    debug!("SSE: About to call get_playback_state()");
    let playback_state = ctx.state.get_playback_state().await;
    debug!("SSE: get_playback_state() returned: {:?}", playback_state);

    debug!("SSE: About to call get_current_passage()");
    let position_info = if let Some(current) = ctx.state.get_current_passage().await {
        debug!("SSE: get_current_passage() returned Some(...)");
        current.passage_id.map(|pid| PlaybackPositionInfo {
            passage_id: pid,
            position_ms: current.position_ms,
            duration_ms: current.duration_ms,
            playing: matches!(playback_state, PlaybackState::Playing),
        })
    } else {
        debug!("SSE: get_current_passage() returned None");
        None
    };

    debug!("SSE: About to call get_volume()");
    let volume = ctx.state.get_volume().await;
    debug!("SSE: get_volume() returned: {}", volume);

    // Create stream that sends initial state, then ongoing events
    let stream = async_stream::stream! {
        // === INITIAL STATE EMISSION ===
        // [SSE-UI-050] Send current state immediately on connection
        debug!("SSE: Sending InitialState to new SSE client");

        // Send InitialState event
        let initial_event = WkmpEvent::InitialState {
            timestamp: Utc::now(),
            queue: queue_info,
            position: position_info,
            volume,
        };

        debug!("SSE: About to serialize InitialState event");
        let event_json = serde_json::to_string(&initial_event)
            .unwrap_or_else(|_| "{}".to_string());
        debug!("SSE: Serialized InitialState, length: {} bytes", event_json.len());

        debug!("SSE: About to yield InitialState event");
        yield Ok(Event::default()
            .event("InitialState")
            .data(event_json));
        debug!("SSE: Successfully yielded InitialState event");

        // === ONGOING EVENT STREAM ===
        // [SSE-UI-010] Stream all subsequent events
        loop {
            tokio::select! {
                // Heartbeat every 15 seconds
                _ = tokio::time::sleep(Duration::from_secs(15)) => {
                    yield Ok(Event::default().comment("heartbeat"));
                }

                // Broadcast events
                Ok(event) = rx.recv() => {
                    let event_type = event.event_type();

                    match serde_json::to_string(&event) {
                        Ok(event_json) => {
                            debug!("Broadcasting SSE event: {}", event_type);
                            yield Ok(Event::default()
                                .event(event_type)
                                .data(event_json));
                        }
                        Err(e) => {
                            warn!("Failed to serialize event {}: {}", event_type, e);
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

// event_type_str function removed - now using event.event_type() method from wkmp-common

//! SSE broadcaster for real-time client updates

use axum::{
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
};
use futures::stream::{Stream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, info, warn};

use super::events::SseEvent;

/// SSE Broadcaster manages client connections and event distribution
#[derive(Clone)]
pub struct SseBroadcaster {
    tx: broadcast::Sender<SseEvent>,
}

impl SseBroadcaster {
    /// Create a new SSE broadcaster
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of events to buffer (recommended: 100 for SSE)
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        info!("SSE broadcaster initialized with capacity {}", capacity);
        Self { tx }
    }

    /// Broadcast an event to all connected clients
    ///
    /// Returns Ok(subscriber_count) if successful, or Err if no clients connected
    pub fn broadcast(&self, event: SseEvent) -> Result<usize, broadcast::error::SendError<SseEvent>> {
        let result = self.tx.send(event);
        if let Ok(count) = result {
            debug!("Broadcast event to {} clients", count);
        }
        result
    }

    /// Broadcast an event, ignoring if no clients are connected
    pub fn broadcast_lossy(&self, event: SseEvent) {
        let _ = self.tx.send(event);
    }

    /// Get current number of connected clients
    pub fn client_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Create an SSE stream for a new client connection
    ///
    /// This is called by the HTTP handler when a client connects to /events
    pub fn subscribe_stream(&self) -> impl Stream<Item = Result<Event, Infallible>> {
        let rx = self.tx.subscribe();
        let stream = BroadcastStream::new(rx);

        stream.filter_map(|result| async move {
            match result {
                Ok(sse_event) => {
                    // Convert SseEvent to axum SSE Event
                    let event = Event::default()
                        .event(&sse_event.event)
                        .json_data(&sse_event.data)
                        .ok();
                    event.map(Ok)
                }
                Err(e) => {
                    // BroadcastStream wraps RecvError, just log and continue
                    warn!("SSE client error: {:?}", e);
                    None
                }
            }
        })
    }

    /// Create an Axum SSE response handler
    ///
    /// This is the handler function for GET /events
    pub fn handle_sse_connection(&self) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
        info!("New SSE client connected, total clients: {}", self.client_count());

        Sse::new(self.subscribe_stream())
            .keep_alive(
                KeepAlive::new()
                    .interval(Duration::from_secs(30))
                    .text("keep-alive"),
            )
    }
}

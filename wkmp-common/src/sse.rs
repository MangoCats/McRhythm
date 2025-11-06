//! Server-Sent Events (SSE) utilities
//!
//! Shared SSE implementations for WKMP microservices.

use axum::response::sse::{Event, Sse};
use futures::stream::Stream;
use std::convert::Infallible;
use std::time::Duration;
use tracing::{debug, info};

/// Create a simple heartbeat-only SSE stream for connection status monitoring
///
/// Used by on-demand microservices (wkmp-dr, wkmp-ai, wkmp-le) that don't
/// have domain events to broadcast but still need connection status UI.
///
/// # Arguments
/// * `service_name` - Name of the service for logging (e.g., "wkmp-dr")
///
/// # Example
/// ```rust,ignore
/// pub async fn event_stream(
///     State(_state): State<AppState>,
/// ) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
///     wkmp_common::sse::create_heartbeat_sse_stream("wkmp-dr")
/// }
/// ```
pub fn create_heartbeat_sse_stream(
    service_name: &'static str,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New SSE client connected to {} general events", service_name);

    let stream = async_stream::stream! {
        info!("SSE: {} event stream started", service_name);

        // Send initial connected status
        yield Ok(Event::default()
            .event("ConnectionStatus")
            .data("connected"));

        loop {
            // Heartbeat every 15 seconds
            tokio::time::sleep(Duration::from_secs(15)).await;
            debug!("SSE: Sending heartbeat");
            yield Ok(Event::default().comment("heartbeat"));
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat")
    )
}

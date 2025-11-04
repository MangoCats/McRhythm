//! Server-Sent Events (SSE) for connection status
//!
//! **[REQ-DR-SSE-010]** SSE event streaming for real-time connection status

use crate::AppState;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use futures::stream::Stream;
use std::convert::Infallible;

/// GET /api/events - SSE event stream for connection status
///
/// **[REQ-DR-SSE-010]** Connection status monitoring
///
/// Streams events:
/// - ConnectionStatus (heartbeat with database connectivity)
pub async fn event_stream(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    wkmp_common::sse::create_heartbeat_sse_stream("wkmp-dr")
}

//! Background monitoring tasks for playback

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, info, warn};

use super::engine::PlaybackEngine;
use crate::sse::{SseBroadcaster, SseEvent, SseEventData};

/// Start background monitoring tasks
pub fn start_monitoring(engine: Arc<RwLock<PlaybackEngine>>, sse_broadcaster: SseBroadcaster) {
    // Position update task (5 seconds for SSE PlaybackProgress)
    tokio::spawn(position_update_task(engine.clone(), sse_broadcaster.clone()));

    // EOS and boundary check task (500ms for song boundaries)
    tokio::spawn(eos_check_task(engine.clone()));
}

/// Position update task - runs every 5 seconds
/// Updates position for SSE PlaybackProgress events
async fn position_update_task(engine: Arc<RwLock<PlaybackEngine>>, sse_broadcaster: SseBroadcaster) {
    let mut interval = time::interval(Duration::from_millis(5000));

    info!("Position update task started (5000ms interval)");

    loop {
        interval.tick().await;

        let engine_read = engine.read().await;
        engine_read.update_position().await;

        // Get current position and emit SSE event
        let (position_ms, duration_ms) = engine_read.shared_state().get_position().await;
        let currently_playing = engine_read.shared_state().get_currently_playing().await;

        let event_data = SseEventData::playback_progress(
            currently_playing.map(|id| id.to_string()),
            position_ms,
            duration_ms,
        );

        let sse_event = SseEvent::new("playback_progress", event_data);
        sse_broadcaster.broadcast_lossy(sse_event);

        debug!("Position updated and broadcast: {}ms / {}ms", position_ms, duration_ms);
    }
}

/// EOS check task - runs every 500ms
/// Checks for end-of-stream and song boundary crossings
async fn eos_check_task(engine: Arc<RwLock<PlaybackEngine>>) {
    let mut interval = time::interval(Duration::from_millis(500));

    info!("EOS check task started (500ms interval)");

    loop {
        interval.tick().await;

        let mut engine_write = engine.write().await;

        // Check for end of stream
        if let Err(e) = engine_write.check_eos().await {
            warn!("Error checking EOS: {}", e);
        }

        // TODO: Check song boundaries here when implemented
        // engine_write.check_song_boundaries().await
    }
}

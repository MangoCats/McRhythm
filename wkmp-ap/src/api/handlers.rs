//! API request handlers for Audio Player endpoints
//!
//! Implements the specific handlers for each API endpoint

use axum::{
    extract::{State, Path},
    response::{Json, sse::{Event, KeepAlive, Sse}},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use std::convert::Infallible;
use std::time::Duration;
use futures::stream::{self, Stream};
use tokio_stream::StreamExt;
use tracing::{info, error};

use crate::api::AppState;
use crate::playback::engine::{EnqueueRequest, PlaybackState};

// === Audio Device Endpoints ===

#[derive(Serialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub default: bool,
}

#[derive(Serialize)]
pub struct AudioDevicesResponse {
    pub devices: Vec<AudioDevice>,
}

/// GET /audio/devices - List available audio devices
pub async fn get_audio_devices(
    State(_state): State<AppState>,
) -> Result<Json<AudioDevicesResponse>, StatusCode> {
    // TODO: Get actual devices from cpal
    let devices = vec![
        AudioDevice {
            id: "default".to_string(),
            name: "System Default".to_string(),
            default: true,
        },
    ];

    Ok(Json(AudioDevicesResponse { devices }))
}

/// GET /audio/device - Get current audio device
pub async fn get_current_device(
    State(_state): State<AppState>,
) -> Json<Value> {
    Json(json!({
        "device_id": "default",
        "device_name": "System Default"
    }))
}

#[derive(Deserialize)]
pub struct SetDeviceRequest {
    pub device_id: String,
}

/// POST /audio/device - Set audio device
pub async fn set_audio_device(
    State(_state): State<AppState>,
    Json(request): Json<SetDeviceRequest>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: Actually set the device
    info!("Setting audio device: {}", request.device_id);

    Ok(Json(json!({
        "status": "ok",
        "device_id": request.device_id,
        "device_name": "System Default"
    })))
}

// === Volume Endpoints ===

/// GET /audio/volume - Get current volume
pub async fn get_volume(
    State(_state): State<AppState>,
) -> Json<Value> {
    // TODO: Get actual volume from audio output
    Json(json!({
        "volume": 75
    }))
}

#[derive(Deserialize)]
pub struct SetVolumeRequest {
    pub volume: u8,
}

/// POST /audio/volume - Set volume
pub async fn set_volume(
    State(_state): State<AppState>,
    Json(request): Json<SetVolumeRequest>,
) -> Result<Json<Value>, StatusCode> {
    if request.volume > 100 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // TODO: Actually set the volume
    info!("Setting volume: {}", request.volume);

    Ok(Json(json!({
        "status": "ok",
        "volume": request.volume
    })))
}

// === Playback Control Endpoints ===

/// POST /playback/play - Resume playback
pub async fn play(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    state.engine.play().await
        .map_err(|e| {
            error!("Failed to start playback: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "status": "ok",
        "state": "playing"
    })))
}

/// POST /playback/pause - Pause playback
pub async fn pause(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    state.engine.pause().await
        .map_err(|e| {
            error!("Failed to pause playback: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "status": "ok",
        "state": "paused"
    })))
}

/// GET /playback/state - Get playback state
pub async fn get_state(
    State(state): State<AppState>,
) -> Json<Value> {
    let current_state = state.engine.get_state().await;

    let state_str = match current_state {
        PlaybackState::Playing => "playing",
        PlaybackState::Paused => "paused",
    };

    Json(json!({
        "state": state_str
    }))
}

/// GET /playback/position - Get playback position
pub async fn get_position(
    State(state): State<AppState>,
) -> Json<Value> {
    let position = state.engine.get_position().await;

    Json(json!({
        "passage_id": position.passage_id,
        "position_ms": position.position_ms,
        "duration_ms": position.duration_ms,
        "state": match position.state {
            PlaybackState::Playing => "playing",
            PlaybackState::Paused => "paused",
        }
    }))
}

// === Queue Management Endpoints ===

/// GET /playback/queue - Get queue contents
pub async fn get_queue(
    State(state): State<AppState>,
) -> Json<Value> {
    let queue = state.engine.get_queue().await;

    Json(json!({
        "queue": queue.into_iter().map(|entry| {
            json!({
                "queue_entry_id": entry.queue_entry_id,
                "passage_id": entry.passage_id,
                "play_order": entry.play_order,
                "file_path": entry.file_path,
                "timing_override": entry.timing_override
            })
        }).collect::<Vec<_>>()
    }))
}

/// POST /playback/enqueue - Enqueue a passage
pub async fn enqueue(
    State(state): State<AppState>,
    Json(request): Json<EnqueueRequest>,
) -> Result<Json<Value>, StatusCode> {
    let entry = state.engine.enqueue(request).await
        .map_err(|e| {
            error!("Failed to enqueue: {}", e);
            if e.to_string().contains("Queue is full") {
                StatusCode::CONFLICT
            } else if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(Json(json!({
        "status": "ok",
        "queue_entry_id": entry.queue_entry_id,
        "play_order": entry.play_order
    })))
}

/// DELETE /playback/queue/:passage_id - Remove from queue
pub async fn dequeue(
    State(state): State<AppState>,
    Path(passage_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = Uuid::parse_str(&passage_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    state.engine.dequeue(uuid).await
        .map_err(|e| {
            error!("Failed to dequeue: {}", e);
            if e.to_string().contains("not_found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(Json(json!({
        "status": "ok",
        "removed": true,
        "queue_entry_id": uuid
    })))
}

// === Server-Sent Events ===

/// GET /events - SSE event stream
pub async fn sse_handler(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::repeat_with(|| {
        // TODO: Connect to actual event broadcaster
        Event::default()
            .event("heartbeat")
            .data("ping")
    })
    .map(Ok)
    .throttle(Duration::from_secs(30));

    Sse::new(stream).keep_alive(KeepAlive::default())
}
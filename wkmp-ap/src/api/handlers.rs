//! HTTP request handlers
//!
//! Implements REST API endpoints for playback control.
//!
//! **Traceability:** API Design - Audio Player API endpoints

use crate::api::server::AppContext;
use crate::state::PlaybackState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info};
use uuid::Uuid;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    status: String,
    module: String,
    version: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    status: String,
}

#[derive(Debug, Deserialize)]
pub struct VolumeRequest {
    volume: u8, // 0-100 user-facing scale
}

#[derive(Debug, Serialize)]
pub struct VolumeResponse {
    volume: u8,
}

#[derive(Debug, Deserialize)]
pub struct EnqueueRequest {
    file_path: String,
}

#[derive(Debug, Serialize)]
pub struct EnqueueResponse {
    status: String,
    queue_entry_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct QueueResponse {
    queue: Vec<QueueEntryInfo>,
}

#[derive(Debug, Serialize)]
pub struct QueueEntryInfo {
    queue_entry_id: Uuid,
    passage_id: Option<Uuid>,
    file_path: String,
}

#[derive(Debug, Serialize)]
pub struct PositionResponse {
    passage_id: Option<Uuid>,
    position_ms: u64,
    duration_ms: u64,
    state: String,
}

#[derive(Debug, Serialize)]
pub struct BufferStatusResponse {
    buffers: Vec<BufferInfo>,
}

#[derive(Debug, Serialize)]
pub struct BufferInfo {
    passage_id: Uuid,
    status: String,
    decode_progress_percent: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct SetDeviceRequest {
    device_name: String,
}

#[derive(Debug, Serialize)]
pub struct DeviceListResponse {
    devices: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    device_name: String,
}

// ============================================================================
// Health Endpoint
// ============================================================================

/// GET /health - Health check endpoint
///
/// **Traceability:** API Design - Health check (required for all modules)
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        module: "audio_player".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// ============================================================================
// Audio Device Endpoints
// ============================================================================

/// GET /audio/devices - List available audio output devices
///
/// **Traceability:** API Design - GET /audio/devices
pub async fn list_audio_devices() -> Result<Json<DeviceListResponse>, (StatusCode, Json<StatusResponse>)> {
    use crate::audio::output::AudioOutput;

    match AudioOutput::list_devices() {
        Ok(devices) => {
            info!("Found {} audio devices", devices.len());
            Ok(Json(DeviceListResponse { devices }))
        }
        Err(e) => {
            error!("Failed to list audio devices: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StatusResponse {
                    status: format!("error: {}", e),
                }),
            ))
        }
    }
}

/// GET /audio/device - Get current audio device setting
///
/// **Traceability:** API Design - GET /audio/device
pub async fn get_audio_device(
    State(ctx): State<AppContext>,
) -> Result<Json<DeviceResponse>, (StatusCode, Json<StatusResponse>)> {
    match crate::db::settings::get_audio_device(&ctx.db_pool).await {
        Ok(device_name) => {
            Ok(Json(DeviceResponse { device_name }))
        }
        Err(e) => {
            error!("Failed to get audio device setting: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StatusResponse {
                    status: format!("error: {}", e),
                }),
            ))
        }
    }
}

/// POST /audio/device - Set audio device
///
/// **Traceability:** API Design - POST /audio/device
pub async fn set_audio_device(
    State(ctx): State<AppContext>,
    Json(req): Json<SetDeviceRequest>,
) -> Result<StatusCode, (StatusCode, Json<StatusResponse>)> {
    info!("Set audio device request: {}", req.device_name);

    // Save to database
    match crate::db::settings::set_audio_device(&ctx.db_pool, req.device_name.clone()).await {
        Ok(_) => {
            info!("Audio device setting updated to: {}", req.device_name);

            // Note: Actual device restart would require stopping and restarting audio output
            // This is deferred to future implementation when full mixer integration is complete

            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!("Failed to set audio device: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StatusResponse {
                    status: format!("error: {}", e),
                }),
            ))
        }
    }
}

// ============================================================================
// Volume Endpoints
// ============================================================================

/// GET /audio/volume - Get current volume
///
/// **Traceability:** API Design - GET /audio/volume
pub async fn get_volume(
    State(ctx): State<AppContext>,
) -> Json<VolumeResponse> {
    let system_volume = ctx.state.get_volume().await;
    let user_volume = (system_volume * 100.0).ceil() as u8;

    Json(VolumeResponse {
        volume: user_volume,
    })
}

/// POST /audio/volume - Set volume level
///
/// **Traceability:** API Design - POST /audio/volume
pub async fn set_volume(
    State(ctx): State<AppContext>,
    Json(req): Json<VolumeRequest>,
) -> Result<Json<VolumeResponse>, StatusCode> {
    // Validate range
    if req.volume > 100 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Convert user scale (0-100) to system scale (0.0-1.0)
    let system_volume = req.volume as f32 / 100.0;
    let old_volume = ctx.state.get_volume().await;
    ctx.state.set_volume(system_volume).await;

    // Emit VolumeChanged event
    ctx.state.broadcast_event(wkmp_common::events::WkmpEvent::VolumeChanged {
        volume: system_volume as f64,
        timestamp: chrono::Utc::now(),
    });

    info!("Volume changed: {:.0}% -> {:.0}%", old_volume * 100.0, system_volume * 100.0);

    Ok(Json(VolumeResponse {
        volume: req.volume,
    }))
}

// ============================================================================
// Playback Control Endpoints (Stubs for Phase 1)
// ============================================================================

/// POST /playback/enqueue - Enqueue a passage for playback
///
/// **Traceability:** API Design - POST /playback/enqueue
pub async fn enqueue_passage(
    State(ctx): State<AppContext>,
    Json(req): Json<EnqueueRequest>,
) -> Result<Json<EnqueueResponse>, (StatusCode, Json<StatusResponse>)> {
    info!("Enqueue request for file: {}", req.file_path);

    // Convert string path to PathBuf
    let file_path = PathBuf::from(&req.file_path);

    // Call engine to enqueue
    match ctx.engine.enqueue_file(file_path).await {
        Ok(queue_entry_id) => {
            info!("Successfully enqueued passage: {}", queue_entry_id);

            // Emit QueueChanged event
            ctx.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueChanged {
                timestamp: chrono::Utc::now(),
            });

            Ok(Json(EnqueueResponse {
                status: "ok".to_string(),
                queue_entry_id,
            }))
        }
        Err(e) => {
            error!("Failed to enqueue passage: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StatusResponse {
                    status: format!("error: {}", e),
                }),
            ))
        }
    }
}

/// DELETE /playback/queue/:queue_entry_id - Remove queue entry
///
/// **Traceability:** API Design - DELETE /playback/queue/{queue_entry_id}
pub async fn remove_from_queue(
    State(ctx): State<AppContext>,
    Path(queue_entry_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<StatusResponse>)> {
    info!("Remove from queue request: {}", queue_entry_id);

    // Remove from database
    match crate::db::queue::remove_from_queue(&ctx.db_pool, queue_entry_id).await {
        Ok(_) => {
            info!("Successfully removed queue entry: {}", queue_entry_id);

            // Emit QueueChanged event
            ctx.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueChanged {
                timestamp: chrono::Utc::now(),
            });

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to remove queue entry: {}", e);
            Err((
                StatusCode::NOT_FOUND,
                Json(StatusResponse {
                    status: format!("error: {}", e),
                }),
            ))
        }
    }
}

/// POST /playback/queue/clear - Clear all queue entries
///
/// **Traceability:** API Design - POST /playback/queue/clear
pub async fn clear_queue(
    State(ctx): State<AppContext>,
) -> Result<StatusCode, (StatusCode, Json<StatusResponse>)> {
    info!("Clear queue request");

    // Clear database queue
    match crate::db::queue::clear_queue(&ctx.db_pool).await {
        Ok(_) => {
            info!("Successfully cleared queue");

            // Emit QueueChanged event
            ctx.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueChanged {
                timestamp: chrono::Utc::now(),
            });

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to clear queue: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StatusResponse {
                    status: format!("error: {}", e),
                }),
            ))
        }
    }
}

pub async fn play(
    State(ctx): State<AppContext>,
) -> Json<StatusResponse> {
    match ctx.engine.play().await {
        Ok(_) => {
            info!("Play command succeeded");
            Json(StatusResponse {
                status: "ok".to_string(),
            })
        }
        Err(e) => {
            error!("Play command failed: {}", e);
            Json(StatusResponse {
                status: "error".to_string(),
            })
        }
    }
}

pub async fn pause(
    State(ctx): State<AppContext>,
) -> Json<StatusResponse> {
    match ctx.engine.pause().await {
        Ok(_) => {
            info!("Pause command succeeded");
            Json(StatusResponse {
                status: "ok".to_string(),
            })
        }
        Err(e) => {
            error!("Pause command failed: {}", e);
            Json(StatusResponse {
                status: "error".to_string(),
            })
        }
    }
}

/// GET /playback/queue - Get queue contents
///
/// **Traceability:** API Design - GET /playback/queue
pub async fn get_queue(
    State(_ctx): State<AppContext>,
) -> Json<QueueResponse> {
    // TODO: Access queue from engine
    // For now, return empty queue
    Json(QueueResponse {
        queue: Vec::new(),
    })
}

pub async fn get_playback_state(
    State(ctx): State<AppContext>,
) -> Json<serde_json::Value> {
    let playback_state = ctx.state.get_playback_state().await;

    let state_str = match playback_state {
        PlaybackState::Playing => "playing",
        PlaybackState::Paused => "paused",
    };

    Json(serde_json::json!({
        "state": state_str
    }))
}

/// GET /playback/position - Get current playback position
///
/// **Traceability:** API Design - GET /playback/position
pub async fn get_position(
    State(ctx): State<AppContext>,
) -> Json<PositionResponse> {
    let playback_state = ctx.state.get_playback_state().await;
    let current_passage = ctx.state.get_current_passage().await;

    let state_str = match playback_state {
        PlaybackState::Playing => "playing",
        PlaybackState::Paused => "paused",
    };

    if let Some(passage) = current_passage {
        Json(PositionResponse {
            passage_id: passage.passage_id,
            position_ms: passage.position_ms,
            duration_ms: passage.duration_ms,
            state: state_str.to_string(),
        })
    } else {
        Json(PositionResponse {
            passage_id: None,
            position_ms: 0,
            duration_ms: 0,
            state: state_str.to_string(),
        })
    }
}

/// GET /playback/buffer_status - Get buffer status
///
/// **Traceability:** API Design - GET /playback/buffer_status
pub async fn get_buffer_status(
    State(_ctx): State<AppContext>,
) -> Json<BufferStatusResponse> {
    // TODO: Get actual buffer status from engine/buffer manager
    // For now, return empty list
    Json(BufferStatusResponse {
        buffers: Vec::new(),
    })
}

/// POST /playback/next - Skip to next passage
///
/// **Traceability:** API Design - POST /playback/next
pub async fn skip_next(
    State(ctx): State<AppContext>,
) -> Result<StatusCode, (StatusCode, Json<StatusResponse>)> {
    info!("Skip next request");

    match ctx.engine.skip_next().await {
        Ok(_) => {
            info!("Skip next command succeeded");
            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!("Skip next command failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StatusResponse {
                    status: format!("error: {}", e),
                }),
            ))
        }
    }
}

/// POST /playback/previous - Skip to previous passage (not implemented)
///
/// **Traceability:** API Design - POST /playback/previous
pub async fn skip_previous(
    State(_ctx): State<AppContext>,
) -> (StatusCode, Json<StatusResponse>) {
    // Previous/backwards playback not implemented in current design
    info!("Skip previous request (not implemented)");
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(StatusResponse {
            status: "Previous playback not implemented".to_string(),
        }),
    )
}

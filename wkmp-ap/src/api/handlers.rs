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
    response::Html,
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info, warn};
use uuid::Uuid;
use wkmp_common::events::BufferChainInfo;

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
    volume: f32, // 0.0-1.0 system-wide scale
}

#[derive(Debug, Serialize)]
pub struct VolumeResponse {
    volume: f32,
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
pub struct BufferChainsResponse {
    chains: Vec<BufferChainInfo>,
}

#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    device_name: String,
}

#[derive(Debug, Deserialize)]
pub struct SeekRequest {
    position_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct ReorderQueueRequest {
    queue_entry_id: Uuid,
    new_position: i32,
}

#[derive(Debug, Serialize)]
pub struct BuildInfoResponse {
    version: String,
    git_hash: String,
    build_timestamp: String,
    build_profile: String,
}

#[derive(Debug, Deserialize)]
pub struct BrowseFilesRequest {
    path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BrowseFilesResponse {
    current_path: String,
    parent_path: Option<String>,
    entries: Vec<FileEntry>,
}

#[derive(Debug, Serialize)]
pub struct FileEntry {
    name: String,
    path: String,
    is_directory: bool,
    is_audio_file: bool,
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
    let volume = ctx.state.get_volume().await;

    Json(VolumeResponse {
        volume,
    })
}

/// POST /audio/volume - Set volume level
///
/// **Traceability:** API Design - POST /audio/volume
/// **[ARCH-VOL-020]** Updates shared volume Arc (synchronized with AudioOutput)
pub async fn set_volume(
    State(ctx): State<AppContext>,
    Json(req): Json<VolumeRequest>,
) -> Result<Json<VolumeResponse>, StatusCode> {
    // Validate range (0.0-1.0)
    if req.volume < 0.0 || req.volume > 1.0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let old_volume = ctx.state.get_volume().await;

    // **[ARCH-VOL-020]** Update shared volume Arc (synchronized with AudioOutput)
    *ctx.volume.lock().unwrap() = req.volume.clamp(0.0, 1.0);

    // Update SharedState for consistency
    ctx.state.set_volume(req.volume).await;

    // Persist to database [ARCH-CFG-020] Database-first configuration
    if let Err(e) = crate::db::settings::set_volume(&ctx.db_pool, req.volume).await {
        error!("Failed to persist volume to database: {}", e);
        // Continue anyway - volume is updated in Arc and SharedState
    }

    // Emit VolumeChanged event
    ctx.state.broadcast_event(wkmp_common::events::WkmpEvent::VolumeChanged {
        volume: req.volume as f64,
        timestamp: chrono::Utc::now(),
    });

    info!("Volume changed: {:.2} -> {:.2}", old_volume, req.volume);

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

            // **[SSE-UI-020]** Emit QueueStateUpdate for SSE clients
            let queue_entries = ctx.engine.get_queue_entries().await;
            let queue_info: Vec<wkmp_common::events::QueueEntryInfo> = queue_entries.into_iter()
                .map(|e| wkmp_common::events::QueueEntryInfo {
                    queue_entry_id: e.queue_entry_id,
                    passage_id: e.passage_id,
                    file_path: e.file_path.to_string_lossy().to_string(),
                })
                .collect();
            ctx.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueStateUpdate {
                timestamp: chrono::Utc::now(),
                queue: queue_info,
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

            // **[SSE-UI-020]** Emit QueueStateUpdate for SSE clients
            let queue_entries = ctx.engine.get_queue_entries().await;
            let queue_info: Vec<wkmp_common::events::QueueEntryInfo> = queue_entries.into_iter()
                .map(|e| wkmp_common::events::QueueEntryInfo {
                    queue_entry_id: e.queue_entry_id,
                    passage_id: e.passage_id,
                    file_path: e.file_path.to_string_lossy().to_string(),
                })
                .collect();
            ctx.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueStateUpdate {
                timestamp: chrono::Utc::now(),
                queue: queue_info,
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

    // Clear engine state (stops playback, clears in-memory queue, clears buffers)
    match ctx.engine.clear_queue().await {
        Ok(_) => {
            // Also clear database queue to keep in sync
            if let Err(e) = crate::db::queue::clear_queue(&ctx.db_pool).await {
                warn!("Failed to clear database queue (continuing anyway): {}", e);
            }

            info!("Successfully cleared queue");

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
    State(ctx): State<AppContext>,
) -> Json<QueueResponse> {
    let entries = ctx.engine.get_queue_entries().await;

    let queue = entries.into_iter().map(|entry| QueueEntryInfo {
        queue_entry_id: entry.queue_entry_id,
        passage_id: entry.passage_id,
        file_path: entry.file_path.to_string_lossy().to_string(),
    }).collect();

    Json(QueueResponse { queue })
}

pub async fn get_buffer_chains(
    State(ctx): State<AppContext>,
) -> Json<BufferChainsResponse> {
    let chains = ctx.engine.get_buffer_chains().await;
    Json(BufferChainsResponse { chains })
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
    State(ctx): State<AppContext>,
) -> Json<BufferStatusResponse> {
    use crate::audio::types::BufferStatus;

    // Get all buffer statuses from engine
    let statuses = ctx.engine.get_buffer_statuses().await;

    // Convert to response format
    let buffers = statuses
        .into_iter()
        .map(|(passage_id, status)| {
            let (status_str, decode_progress) = match status {
                BufferStatus::Decoding { progress_percent } => {
                    ("decoding", Some(progress_percent as f32))
                }
                BufferStatus::Ready => ("ready", None),
                BufferStatus::Playing => ("playing", None),
                BufferStatus::Exhausted => ("exhausted", None),
            };

            BufferInfo {
                passage_id,
                status: status_str.to_string(),
                decode_progress_percent: decode_progress,
            }
        })
        .collect();

    Json(BufferStatusResponse { buffers })
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

/// POST /playback/seek - Seek to position in current passage
///
/// **Traceability:** API Design - POST /playback/seek
/// **Requirements:** [SSD-ENG-026] Seek functionality
pub async fn seek(
    State(ctx): State<AppContext>,
    Json(req): Json<SeekRequest>,
) -> Result<StatusCode, (StatusCode, Json<StatusResponse>)> {
    info!("Seek request: position={}ms", req.position_ms);

    match ctx.engine.seek(req.position_ms).await {
        Ok(_) => {
            info!("Seek command succeeded: {}ms", req.position_ms);
            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!("Seek command failed: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(StatusResponse {
                    status: format!("error: {}", e),
                }),
            ))
        }
    }
}

/// POST /playback/queue/reorder - Reorder a queue entry
///
/// **Traceability:** API Design - POST /playback/queue/reorder
/// **Requirements:** [DB-QUEUE-080] Queue reordering
pub async fn reorder_queue_entry(
    State(ctx): State<AppContext>,
    Json(req): Json<ReorderQueueRequest>,
) -> Result<StatusCode, (StatusCode, Json<StatusResponse>)> {
    info!("Reorder queue request: entry={}, position={}", req.queue_entry_id, req.new_position);

    match ctx.engine.reorder_queue_entry(req.queue_entry_id, req.new_position).await {
        Ok(_) => {
            info!("Queue reordered successfully");

            // Emit QueueChanged event
            ctx.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueChanged {
                timestamp: chrono::Utc::now(),
            });

            // **[SSE-UI-020]** Emit QueueStateUpdate for SSE clients
            let queue_entries = ctx.engine.get_queue_entries().await;
            let queue_info: Vec<wkmp_common::events::QueueEntryInfo> = queue_entries.into_iter()
                .map(|e| wkmp_common::events::QueueEntryInfo {
                    queue_entry_id: e.queue_entry_id,
                    passage_id: e.passage_id,
                    file_path: e.file_path.to_string_lossy().to_string(),
                })
                .collect();
            ctx.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueStateUpdate {
                timestamp: chrono::Utc::now(),
                queue: queue_info,
            });

            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!("Queue reorder failed: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(StatusResponse {
                    status: format!("error: {}", e),
                }),
            ))
        }
    }
}

// ============================================================================
// File Browser
// ============================================================================

/// GET /files/browse - Browse filesystem for audio files
///
/// **[ARCH-FB-010]** File browser for developer UI
/// Allows browsing directories and selecting audio files to enqueue.
/// Security: Only allows browsing within configured root folder.
pub async fn browse_files(
    State(ctx): State<AppContext>,
    axum::extract::Query(req): axum::extract::Query<BrowseFilesRequest>,
) -> Result<Json<BrowseFilesResponse>, (StatusCode, Json<StatusResponse>)> {
    use std::fs;

    // Get configured root folder from database settings
    let root_folder_str: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'root_folder'"
    )
    .fetch_optional(&ctx.db_pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(StatusResponse {
                status: format!("Database error: {}", e),
            }),
        )
    })?;

    let configured_root = match root_folder_str {
        Some(folder) => PathBuf::from(folder),
        None => {
            // Use OS default if not configured in database
            wkmp_common::config::get_default_root_folder()
        }
    };

    /// Helper: Clean up path for display (removes Windows \\?\ prefix)
    ///
    /// [CROSS-PLATFORM] Windows canonicalize() adds \\?\ prefix for extended-length paths.
    /// This is not user-friendly and should be stripped for display.
    fn clean_path_for_display(path: &std::path::Path) -> String {
        let path_str = path.to_string_lossy();

        #[cfg(target_os = "windows")]
        {
            // Strip \\?\ prefix on Windows
            if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
                return stripped.to_string();
            }
        }

        path_str.to_string()
    }

    // Determine target path (default to root folder)
    let target_path = if let Some(path_str) = req.path {
        PathBuf::from(&path_str)
    } else {
        configured_root.clone()
    };

    // Security: Canonicalize paths and ensure target is within root folder
    // [CROSS-PLATFORM] Try configured folder, fall back to OS default if it doesn't exist
    let canonical_root = match fs::canonicalize(&configured_root) {
        Ok(p) => p,
        Err(e) => {
            // Configured folder doesn't exist - try OS default
            let os_default = wkmp_common::config::get_default_root_folder();

            match fs::canonicalize(&os_default) {
                Ok(p) => {
                    use tracing::warn;
                    warn!(
                        "Configured root folder {:?} not found ({}), using OS default: {:?}",
                        configured_root, e, os_default
                    );
                    p
                }
                Err(e2) => {
                    // OS default also doesn't exist - try to create it
                    if let Err(create_err) = fs::create_dir_all(&os_default) {
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(StatusResponse {
                                status: format!(
                                    "Failed to access root folder {:?} ({}) and OS default {:?} ({}) and failed to create default ({})",
                                    configured_root, e, os_default, e2, create_err
                                ),
                            }),
                        ));
                    }

                    // Try to canonicalize after creating
                    match fs::canonicalize(&os_default) {
                        Ok(p) => {
                            use tracing::info;
                            info!("Created and using OS default root folder: {:?}", os_default);
                            p
                        }
                        Err(e3) => {
                            return Err((
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(StatusResponse {
                                    status: format!("Failed to canonicalize created folder: {}", e3),
                                }),
                            ))
                        }
                    }
                }
            }
        }
    };

    let canonical_target = match fs::canonicalize(&target_path) {
        Ok(p) => p,
        Err(_) => {
            // Path doesn't exist, fall back to root
            canonical_root.clone()
        }
    };

    // Prevent path traversal attacks
    if !canonical_target.starts_with(&canonical_root) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(StatusResponse {
                status: "Access denied: path outside root folder".to_string(),
            }),
        ));
    }

    // Read directory contents
    let entries = match fs::read_dir(&canonical_target) {
        Ok(entries) => entries,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(StatusResponse {
                    status: format!("Cannot read directory: {}", e),
                }),
            ))
        }
    };

    // Supported audio file extensions
    let audio_extensions = vec!["mp3", "flac", "ogg", "wav", "m4a", "aac", "opus", "wma"];

    // Build file list
    let mut file_entries = Vec::new();
    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata() {
            let is_directory = metadata.is_dir();
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();

            // Skip hidden files/directories (starting with .)
            if name.starts_with('.') {
                continue;
            }

            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let is_audio_file = !is_directory && audio_extensions.contains(&extension.as_str());

            // Only include directories and audio files
            if is_directory || is_audio_file {
                file_entries.push(FileEntry {
                    name,
                    path: clean_path_for_display(&path),
                    is_directory,
                    is_audio_file,
                });
            }
        }
    }

    // Sort: directories first, then files, alphabetically
    file_entries.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    // Determine parent path
    let parent_path = if canonical_target == canonical_root {
        None
    } else {
        canonical_target
            .parent()
            .map(|p| clean_path_for_display(p))
    };

    Ok(Json(BrowseFilesResponse {
        current_path: clean_path_for_display(&canonical_target),
        parent_path,
        entries: file_entries,
    }))
}

// ============================================================================
// Build Information
// ============================================================================

/// GET /build_info - Get build information
///
/// Returns version, git hash, build timestamp, and build profile
pub async fn get_build_info() -> Json<BuildInfoResponse> {
    Json(BuildInfoResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_hash: env!("GIT_HASH").to_string(),
        build_timestamp: env!("BUILD_TIMESTAMP").to_string(),
        build_profile: env!("BUILD_PROFILE").to_string(),
    })
}

// ============================================================================
// Buffer Chain Monitor Control
// ============================================================================

/// POST /playback/buffer_monitor/rate - Set buffer chain monitor update rate
///
/// **[SPEC020-MONITOR-120]** Client-controlled SSE emission rate
///
/// Sets the rate at which BufferChainStatus SSE events are emitted:
/// - `rate_ms: 100` - Fast updates (10Hz) for visualizing rapid buffer filling
/// - `rate_ms: 1000` - Normal updates (1Hz) for typical monitoring
/// - `rate_ms: 0` - Manual mode (no automatic updates, only on explicit trigger)
pub async fn set_buffer_monitor_rate(
    State(ctx): State<AppContext>,
    Json(payload): Json<SetBufferMonitorRateRequest>,
) -> StatusCode {
    ctx.engine.set_buffer_monitor_rate(payload.rate_ms).await;
    StatusCode::OK
}

/// POST /playback/buffer_monitor/update - Trigger immediate buffer chain status update
///
/// **[SPEC020-MONITOR-130]** Manual update trigger
///
/// Forces one immediate BufferChainStatus SSE emission, regardless of current mode.
/// Useful in manual mode or for forcing an update between automatic intervals.
pub async fn trigger_buffer_monitor_update(
    State(ctx): State<AppContext>,
) -> StatusCode {
    ctx.engine.trigger_buffer_monitor_update();
    StatusCode::OK
}

#[derive(serde::Deserialize)]
pub struct SetBufferMonitorRateRequest {
    /// Update interval in milliseconds (100, 1000, or 0 for manual)
    pub rate_ms: u64,
}

// ============================================================================
// Developer UI
// ============================================================================

/// Serve developer UI HTML (bundled at compile time)
///
/// **[ARCH-PC-010]** Developer UI with status display, API testing, and event monitoring
pub async fn developer_ui() -> Html<&'static str> {
    Html(include_str!("developer_ui.html"))
}

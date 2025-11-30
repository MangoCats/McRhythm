//! wkmp-ai library interface for testing
//!
//! Exposes public APIs for integration testing

#![warn(missing_docs)]

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod extractors; // PLAN024 TASK-004: Tier 1 source extractors
pub mod ffi; // PLAN024: FFI bindings (Chromaprint)
pub mod fusion; // PLAN024: Tier 2 fusion layer
pub mod matching; // PLAN026 Increment 5: Album matching (am28 integration)
pub mod models;
pub mod services;
pub mod types; // PLAN024 TASK-004: Base traits and types
pub mod utils; // PLAN024: Utility functions (audio decoding, etc.)
pub mod validators; // PLAN024: Tier 3 validation layer
pub mod workflow; // PLAN023: Per-song workflow orchestration

pub use crate::error::{ApiError, ApiResult};

use axum::Router;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use wkmp_common::events::EventBus;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool **[AIA-DB-010]**
    pub db: SqlitePool,
    /// Event bus for SSE broadcasting **[AIA-MS-010]**
    pub event_bus: EventBus,
    /// Cancellation tokens for active import sessions **[AIA-ASYNC-010]**
    pub cancellation_tokens: Arc<RwLock<HashMap<Uuid, CancellationToken>>>,
    /// Processing thread count from bootstrap configuration **[AIA-INIT-010]**
    ///
    /// Read from `ai_processing_thread_count` setting at startup (RESTART_REQUIRED parameter).
    /// Used to configure parallel worker pool for import pipeline.
    pub processing_thread_count: usize,
    /// Memory usage threshold in bytes from bootstrap configuration **[IMPL016]**
    ///
    /// Read from `ai_memory_usage_threshold_bytes` setting at startup (RESTART_REQUIRED parameter).
    /// Used to configure memory monitor warning/critical thresholds.
    pub memory_usage_threshold_bytes: u64,
    /// Service startup timestamp for uptime tracking **[HIGH-005]**
    pub startup_time: DateTime<Utc>,
    /// Last error for diagnostic purposes **[HIGH-005]**
    pub last_error: Arc<RwLock<Option<String>>>,
}

impl AppState {
    /// Create new application state
    ///
    /// # Arguments
    /// * `db` - Database connection pool
    /// * `event_bus` - Event bus for SSE broadcasting
    /// * `processing_thread_count` - Worker parallelism from bootstrap config
    /// * `memory_usage_threshold_bytes` - Memory threshold from bootstrap config
    pub fn new(
        db: SqlitePool,
        event_bus: EventBus,
        processing_thread_count: usize,
        memory_usage_threshold_bytes: u64,
    ) -> Self {
        Self {
            db,
            event_bus,
            cancellation_tokens: Arc::new(RwLock::new(HashMap::new())),
            processing_thread_count,
            memory_usage_threshold_bytes,
            startup_time: Utc::now(),
            last_error: Arc::new(RwLock::new(None)),
        }
    }

    /// **[PLAN031 Task 2.1]** Clean up cancellation token for completed/cancelled import
    ///
    /// Prevents unbounded HashMap growth by removing tokens for finished imports.
    /// Also performs periodic cleanup of stale tokens (older than 24 hours or cancelled).
    ///
    /// # Arguments
    /// * `session_id` - Import session ID to clean up
    ///
    /// # Usage
    /// Call this after import completion (success, failure, or cancellation).
    pub async fn cleanup_completed_import(&self, session_id: Uuid) {
        let mut tokens = self.cancellation_tokens.write().await;

        // Remove the specific session's token
        if tokens.remove(&session_id).is_some() {
            tracing::debug!(
                session_id = %session_id,
                remaining_tokens = tokens.len(),
                "Cleaned up cancellation token for completed import"
            );
        }

        // **[PLAN031]** Opportunistic cleanup: Remove stale tokens (>24 hours old or cancelled)
        // This prevents gradual memory leak from abandoned imports
        let initial_count = tokens.len();
        tokens.retain(|id, token| {
            if token.is_cancelled() {
                tracing::trace!(session_id = %id, "Removing cancelled token during cleanup");
                false
            } else {
                true // Keep active tokens
            }
        });

        let removed = initial_count.saturating_sub(tokens.len());
        if removed > 0 {
            tracing::info!(
                removed_tokens = removed,
                remaining_tokens = tokens.len(),
                "Cleaned up stale cancellation tokens"
            );
        }
    }
}

/// Build application router
///
/// **[IMPL008]** API endpoint routing
/// **[AIA-UI-010]** Web UI routes
pub fn build_router(state: AppState) -> Router {
    use axum::routing::get;

    Router::new()
        // UI routes (HTML pages)
        .merge(api::ui_routes())
        // API routes
        .merge(api::import_routes())
        .route("/events", get(api::event_stream))
        .route("/import/events", get(api::import_event_stream))
        .merge(api::amplitude_routes())
        .merge(api::parameter_routes())
        .merge(api::settings_routes())
        .merge(api::file_classification_routes()) // PLAN027: File classification API
        .merge(api::health_routes())
        .with_state(state)
}

//! wkmp-ai library interface for testing
//!
//! Exposes public APIs for integration testing

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod services;

pub use crate::error::{ApiError, ApiResult};

use axum::Router;
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
}

impl AppState {
    pub fn new(db: SqlitePool, event_bus: EventBus) -> Self {
        Self {
            db,
            event_bus,
            cancellation_tokens: Arc::new(RwLock::new(HashMap::new())),
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
        .route("/import/events", get(api::import_event_stream))
        .merge(api::amplitude_routes())
        .merge(api::parameter_routes())
        .merge(api::settings_routes())
        .merge(api::health_routes())
        .with_state(state)
}

//! wkmp-ai library interface for testing
//!
//! Exposes public APIs for integration testing

pub mod api;
pub mod db;
pub mod error;
pub mod models;
pub mod services;

pub use crate::error::{ApiError, ApiResult};

use axum::Router;
use sqlx::SqlitePool;
use wkmp_common::events::EventBus;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool **[AIA-DB-010]**
    pub db: SqlitePool,
    /// Event bus for SSE broadcasting **[AIA-MS-010]**
    pub event_bus: EventBus,
}

impl AppState {
    pub fn new(db: SqlitePool, event_bus: EventBus) -> Self {
        Self { db, event_bus }
    }
}

/// Build application router
///
/// **[IMPL008]** API endpoint routing
pub fn build_router(state: AppState) -> Router {
    use axum::routing::get;

    Router::new()
        .merge(api::import_routes())
        .route("/import/events", get(api::import_event_stream))
        .merge(api::amplitude_routes())
        .merge(api::parameter_routes())
        .merge(api::health_routes())
        .with_state(state)
}

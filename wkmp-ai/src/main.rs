//! wkmp-ai - Audio Ingest Microservice
//!
//! **Module Identity:**
//! - Name: wkmp-ai (Audio Ingest)
//! - Port: 5723
//! - Version: Full only (not in Lite/Minimal)
//!
//! **[AIA-OV-010]** Responsible for importing user music collections into WKMP database
//! with accurate MusicBrainz identification and optimal passage timing.
//!
//! **[AIA-MS-010]** Integrates with wkmp-ui via HTTP REST + SSE

mod api;
mod db;
mod error;
mod models;
mod services;

use anyhow::Result;
use axum::Router;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
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
    fn new(db: SqlitePool, event_bus: EventBus) -> Self {
        Self { db, event_bus }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting wkmp-ai (Audio Ingest) microservice");
    info!("Port: 5723");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Determine database path
    // TODO: Load from config file, for now use current directory
    let db_path = PathBuf::from("wkmp.db");
    info!("Database: {}", db_path.display());

    // Initialize database connection pool **[AIA-DB-010]**
    let db_pool = db::init_database_pool(&db_path).await?;
    info!("Database connection established");

    // Create event bus for SSE broadcasting **[AIA-MS-010]**
    let event_bus = EventBus::new(100); // 100 event capacity
    info!("Event bus initialized");

    // Create application state
    let state = AppState::new(db_pool, event_bus);

    // Build router
    let app = build_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:5723").await?;
    info!("Listening on http://127.0.0.1:5723");
    info!("Health check: http://127.0.0.1:5723/health");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Build application router
///
/// **[IMPL008]** API endpoint routing:
/// - POST /import/start - Begin import session
/// - GET /import/status/:id - Poll import progress
/// - POST /import/cancel/:id - Cancel import
/// - GET /import/events - SSE stream for real-time progress
/// - POST /analyze/amplitude - Analyze amplitude envelope
/// - GET /parameters/global - Get global parameters
/// - POST /parameters/global - Update parameters
/// - GET /health - Health check
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_state_creation() {
        // Create in-memory database for testing
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let event_bus = wkmp_common::events::EventBus::new(10);
        let state = AppState::new(pool, event_bus);
        // Just verify it compiles and creates
        drop(state);
    }
}

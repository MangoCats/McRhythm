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

use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use wkmp_common::events::EventBus;

// Use library definitions
use wkmp_ai::AppState;

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

    // Step 1: Resolve root folder [ARCH-INIT-005, REQ-NF-035]
    let resolver = wkmp_common::config::RootFolderResolver::new("audio-ingest");
    let root_folder = resolver.resolve();

    // Step 2: Create root folder directory if missing [REQ-NF-036]
    let initializer = wkmp_common::config::RootFolderInitializer::new(root_folder);
    initializer.ensure_directory_exists()
        .map_err(|e| anyhow::anyhow!("Failed to initialize root folder: {}", e))?;

    // Step 3: Open or create database [REQ-NF-036]
    let db_path = initializer.database_path();
    info!("Database: {}", db_path.display());

    // Initialize database connection pool **[AIA-DB-010]**
    let db_pool = wkmp_ai::db::init_database_pool(&db_path).await?;
    info!("Database connection established");

    // Create event bus for SSE broadcasting **[AIA-MS-010]**
    let event_bus = EventBus::new(100); // 100 event capacity
    info!("Event bus initialized");

    // Create application state
    let state = AppState::new(db_pool, event_bus);

    // Build router
    let app = wkmp_ai::build_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:5723").await?;
    info!("Listening on http://127.0.0.1:5723");
    info!("Health check: http://127.0.0.1:5723/health");

    axum::serve(listener, app).await?;

    Ok(())
}

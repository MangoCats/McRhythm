//! wkmp-dr (Database Review) - Read-only database inspection tool
//!
//! Provides web UI for inspecting wkmp.db contents with predefined filters
//! and custom searches. Part of WKMP Full version only.
//!
//! [REQ-DR-NF-010]: Zero-config startup
//! [REQ-DR-NF-020]: Read-only database access
//! [REQ-DR-NF-040]: Health endpoint
//! [REQ-DR-NF-050]: Port 5725

use anyhow::Result;
use tracing::{error, info};
use wkmp_common::api::auth::load_shared_secret;
use wkmp_common::config::{RootFolderInitializer, RootFolderResolver};
use wkmp_dr::{build_router, AppState};

mod db;

#[tokio::main]
async fn main() -> Result<()> {
    // [ARCH-INIT-003] Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // [ARCH-INIT-004] Log build identification IMMEDIATELY after tracing init
    // REQUIRED for all modules - provides instant startup feedback before database delays
    info!(
        "Starting WKMP Database Review (wkmp-dr) v{} [{}] built {} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
        env!("BUILD_TIMESTAMP"),
        env!("BUILD_PROFILE")
    );

    // [REQ-DR-NF-010]: Zero-config startup with 4-tier resolution
    let resolver = RootFolderResolver::new("database-review");
    let root_folder = resolver.resolve();

    let initializer = RootFolderInitializer::new(root_folder);
    initializer.ensure_directory_exists()?;

    let db_path = initializer.database_path();
    info!("Database path: {}", db_path.display());

    // [REQ-DR-NF-020]: Connect with read-only mode
    let pool = match db::connect_readonly(&db_path).await {
        Ok(pool) => {
            info!("✓ Connected to database (read-only)");
            pool
        }
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            return Err(e);
        }
    };

    // [REQ-DR-NF-030]: Load shared secret for API authentication
    // NOTE: wkmp-dr uses read-only connection, so cannot initialize secret if missing.
    // If secret doesn't exist, use 0 (disables auth per API-AUTH-028)
    let shared_secret = match load_shared_secret(&pool).await {
        Ok(secret) => {
            if secret == 0 {
                info!("API authentication disabled (shared_secret = 0)");
            } else {
                info!("✓ Loaded shared secret for API authentication");
            }
            secret
        }
        Err(_e) => {
            // Read-only connection cannot initialize secret
            // Use 0 (disables auth) and log warning
            info!("Shared secret not found in database (using 0 - auth disabled)");
            info!("To enable authentication, initialize shared secret in wkmp.db via wkmp-ui");
            0
        }
    };

    // Create application state and router
    let state = AppState::new(pool, shared_secret);
    let app = build_router(state);

    // [REQ-DR-NF-050]: Start server on port 5725
    let listener = tokio::net::TcpListener::bind("127.0.0.1:5725").await?;
    info!("wkmp-dr listening on http://127.0.0.1:5725");
    info!("Health check: http://127.0.0.1:5725/health");

    axum::serve(listener, app).await?;

    Ok(())
}


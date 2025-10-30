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
use tracing::{info, warn, error, Level};
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

    // Step 4: Determine TOML config path
    let toml_path = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(|home| std::path::PathBuf::from(home).join(".config").join("wkmp").join("wkmp-ai.toml"))
        .unwrap_or_else(|_| std::path::PathBuf::from("wkmp-ai.toml"));

    let toml_config = if toml_path.exists() {
        let content = std::fs::read_to_string(&toml_path)
            .map_err(|e| anyhow::anyhow!("Failed to read TOML config: {}", e))?;
        toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse TOML config: {}", e))?
    } else {
        wkmp_common::config::TomlConfig {
            root_folder: None,
            logging: wkmp_common::config::LoggingConfig::default(),
            static_assets: None,
            acoustid_api_key: None,
        }
    };

    // Step 5: Resolve AcoustID API key with auto-migration **[APIK-RES-010]**
    let _api_key = match wkmp_ai::config::resolve_acoustid_api_key(&db_pool, &toml_config).await {
        Ok(key) => {
            // Check if migration needed (ENV or TOML source, database empty)
            let db_key = wkmp_ai::db::settings::get_acoustid_api_key(&db_pool).await?;
            if db_key.is_none() {
                // Auto-migrate to database **[APIK-MIG-010]**
                let source = if std::env::var("WKMP_ACOUSTID_API_KEY").is_ok() {
                    "environment"
                } else {
                    "TOML"
                };
                wkmp_ai::config::migrate_key_to_database(
                    key.clone(),
                    source,
                    &db_pool,
                    &toml_path,
                )
                .await?;
            }
            key
        }
        Err(e) => {
            error!("Failed to resolve AcoustID API key: {}", e);
            warn!("AcoustID fingerprinting will not be available");
            warn!("Import workflow will be limited to file metadata only");
            // Don't fail startup - allow wkmp-ai to run in degraded mode
            String::new()
        }
    };

    // Check TOML permissions (security warning) **[APIK-SEC-040]**
    if toml_path.exists() {
        match wkmp_common::config::check_toml_permissions_loose(&toml_path) {
            Ok(true) => {
                warn!(
                    "TOML config file has loose permissions: {}",
                    toml_path.display()
                );
                warn!("Recommend: chmod 600 {}", toml_path.display());
            }
            Ok(false) => {
                // Permissions OK
            }
            Err(e) => {
                warn!("Failed to check TOML permissions: {}", e);
            }
        }
    }

    // **[AIA-INIT-010]** Cleanup stale import sessions from previous runs
    // Any session not in terminal state is from a previous run and will never complete
    match wkmp_ai::db::sessions::cleanup_stale_sessions(&db_pool).await {
        Ok(count) if count > 0 => {
            info!("Cleaned up {} stale import session(s) from previous run", count);
        }
        Ok(_) => {
            info!("No stale import sessions to clean up");
        }
        Err(e) => {
            tracing::warn!("Failed to cleanup stale sessions (non-fatal): {}", e);
        }
    }

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

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
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use wkmp_common::events::EventBus;

// Use library definitions
use wkmp_ai::AppState;

fn main() -> Result<()> {
    // Initialize tracing with file, line number, and thread ID information
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wkmp_ai=debug,wkmp_common=info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(true),
        )
        .init();

    // **[ARCH-INIT-004]** Log build identification IMMEDIATELY after tracing init
    info!(
        "Starting WKMP Audio Ingest (wkmp-ai) v{} [{}] built {} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
        env!("BUILD_TIMESTAMP"),
        env!("BUILD_PROFILE")
    );

    // Step 1: Resolve root folder [ARCH-INIT-005, REQ-NF-035]
    let resolver = wkmp_common::config::RootFolderResolver::new("audio-ingest");
    let root_folder = resolver.resolve();

    // Step 2: Create root folder directory if missing [REQ-NF-036]
    let initializer = wkmp_common::config::RootFolderInitializer::new(root_folder);
    initializer
        .ensure_directory_exists()
        .map_err(|e| anyhow::anyhow!("Failed to initialize root folder: {}", e))?;

    // Step 3: Get database path [REQ-NF-036]
    let db_path = initializer.database_path();
    info!("Database: {}", db_path.display());

    // **[AIA-INIT-010]** Two-stage database initialization
    // Stage 1: Bootstrap - Read RESTART_REQUIRED parameters with minimal connection
    info!("Stage 1: Reading RESTART_REQUIRED configuration parameters");

    // Need to use a temporary runtime for bootstrap config reading
    let bootstrap_runtime = tokio::runtime::Runtime::new()?;
    let bootstrap_config = bootstrap_runtime.block_on(async {
        wkmp_ai::models::WkmpAiBootstrapConfig::from_database(&db_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read bootstrap configuration: {}", e))
    })?;

    info!(
        "Configuration loaded: pool_size={}, lock_retry={}ms, max_wait={}ms, threads={}",
        bootstrap_config.connection_pool_size,
        bootstrap_config.lock_retry_ms,
        bootstrap_config.max_lock_wait_ms,
        bootstrap_config.processing_thread_count()
    );

    // **[AIA-RT-010]** Build production runtime with explicit worker_threads and max_blocking_threads
    // - worker_threads: Set to ai_processing_thread_count for controlled parallelism
    // **[PLAN034]** 3x blocking threads because hash + fingerprint + amplitude all use spawn_blocking concurrently
    let ai_processing_thread_count = bootstrap_config.processing_thread_count();
    let max_blocking_threads = (3 * ai_processing_thread_count).max(8);

    info!(
        "Building Tokio runtime: worker_threads={}, max_blocking_threads={}",
        ai_processing_thread_count, max_blocking_threads
    );

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(ai_processing_thread_count)
        .max_blocking_threads(max_blocking_threads)
        .enable_all()
        .build()?;

    runtime.block_on(async move { run_async(db_path, bootstrap_config).await })
}

async fn run_async(
    db_path: std::path::PathBuf,
    bootstrap_config: wkmp_ai::models::WkmpAiBootstrapConfig,
) -> Result<()> {
    // Stage 2: Production - Create configured pool and initialize schema
    info!("Stage 2: Creating production database pool with configuration");

    // Create production pool with bootstrap configuration
    let db_pool = bootstrap_config
        .create_pool(&db_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create production database pool: {}", e))?;

    // Initialize/verify schema using shared schema maintenance system **[AIA-DB-010]**
    // This runs all migrations, schema sync, and default settings initialization
    // Idempotent - safe to call on existing database
    info!("Initializing database schema");
    wkmp_common::db::init::init_database_schema(&db_pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize database schema: {}", e))?;

    info!(
        "Database pool ready ({} connections, schema verified)",
        bootstrap_config.connection_pool_size
    );

    // Step 4: Determine TOML config path
    let toml_path = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(|home| {
            std::path::PathBuf::from(home)
                .join(".config")
                .join("wkmp")
                .join("wkmp-ai.toml")
        })
        .unwrap_or_else(|_| std::path::PathBuf::from("wkmp-ai.toml"));

    // **[PLAN031 Task 2.4]** Use async file I/O for TOML config loading
    let toml_config = if toml_path.exists() {
        let content = tokio::fs::read_to_string(&toml_path)
            .await
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
                wkmp_ai::config::migrate_key_to_database(key.clone(), source, &db_pool, &toml_path)
                    .await?;
            }
            key
        }
        Err(e) => {
            error!("Failed to resolve AcoustID API key: {:?}", e);
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
            info!(
                "Cleaned up {} stale import session(s) from previous run",
                count
            );
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

    // Create application state with configured thread count and memory threshold
    let state = AppState::new(
        db_pool.clone(),
        event_bus,
        bootstrap_config.processing_thread_count(),
        bootstrap_config.memory_usage_threshold_bytes(),
    );

    // **[PERF001]** Spawn background task to monitor connection pool utilization
    // Logs pool state every 10 seconds to identify saturation patterns
    tokio::spawn({
        let pool = db_pool.clone();
        async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                let state = pool.size() as usize;
                let idle = pool.num_idle();
                let in_use = state.saturating_sub(idle);

                tracing::debug!(
                    pool_size = state,
                    idle_connections = idle,
                    in_use_connections = in_use,
                    utilization_pct = if state > 0 {
                        (in_use as f64 / state as f64 * 100.0) as u32
                    } else {
                        0
                    },
                    "Pool utilization snapshot"
                );
            }
        }
    });

    // Build router
    let app = wkmp_ai::build_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:5723").await?;
    info!("Listening on http://127.0.0.1:5723");
    info!("Health check: http://127.0.0.1:5723/health");

    axum::serve(listener, app).await?;

    Ok(())
}

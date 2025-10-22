//! # WKMP Audio Player (wkmp-ap)
//!
//! Core playback engine with sample-accurate crossfading.
//!
//! **Purpose:** Decode audio files, manage playback queue, perform sample-accurate
//! crossfading, and provide HTTP/SSE control interface.
//!
//! **Architecture:** Single-stream audio pipeline using symphonia + rubato + cpal
//!
//! **Traceability:** Implements requirements from single-stream-design.md,
//! api_design.md, and crossfade.md

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod audio;
mod config;
mod db;
mod error;
mod playback;
mod state;

use crate::config::Config;
use crate::playback::engine::PlaybackEngine;
use crate::state::SharedState;

#[derive(Parser, Debug)]
#[command(name = "wkmp-ap")]
#[command(about = "WKMP Audio Player - Sample-accurate crossfading playback engine")]
#[command(version)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "wkmp-ap.toml")]
    config: PathBuf,

    /// Database path (overrides config file)
    #[arg(short, long)]
    database: Option<PathBuf>,

    /// HTTP server port (overrides config file)
    #[arg(short, long)]
    port: Option<u16>,

    /// Root folder path for audio files (overrides config file)
    #[arg(short, long)]
    root_folder: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber for logging
    // File + line numbers enabled for debugging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wkmp_ap=debug,tower_http=debug,wkmp_common=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(true).with_file(true).with_line_number(true))
        .init();

    // [CO-226] Log build identification for debugging
    info!(
        "Starting WKMP Audio Player (wkmp-ap) v{} [{}] built {} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
        env!("BUILD_TIMESTAMP"),
        env!("BUILD_PROFILE")
    );

    // Step 1: Resolve root folder [ARCH-INIT-005, REQ-NF-035]
    // Uses 4-tier priority: CLI args > env vars > TOML > compiled defaults
    let resolver = wkmp_common::config::RootFolderResolver::new("audio-player");
    let root_folder = resolver.resolve();

    // Step 2: Create root folder directory if missing [REQ-NF-036]
    let initializer = wkmp_common::config::RootFolderInitializer::new(root_folder);
    initializer.ensure_directory_exists()
        .map_err(|e| anyhow::anyhow!("Failed to initialize root folder: {}", e))?;

    // Step 3: Open or create database [REQ-NF-036]
    let db_path = initializer.database_path();
    let db_pool = wkmp_common::db::init::init_database(&db_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?;
    info!("Database ready at {}", db_path.display());

    // Step 4: Read module configuration from database [DEP-CFG-035]
    let module_config = wkmp_common::config::load_module_config(&db_pool, "audio_player")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load module config: {}", e))?;

    info!("Audio Player configuration: {}:{}", module_config.host, module_config.port);

    // Initialize database tables specific to audio player
    // [ARCH-INIT-010] Module startup sequence
    // [ISSUE-3] Complete module initialization
    db::init::initialize_database(&db_pool).await?;
    info!("Audio Player database tables initialized");

    // Initialize shared state and load volume from database
    // [ARCH-CFG-020] Database-first configuration
    let shared_state = Arc::new(SharedState::new());
    let db_volume = db::settings::get_volume(&db_pool).await
        .map_err(|e| anyhow::anyhow!("Failed to load volume from database: {}", e))?;
    shared_state.set_volume(db_volume).await;
    info!("Volume loaded from database: {:.0}%", db_volume * 100.0);

    // Initialize playback engine
    let engine = Arc::new(PlaybackEngine::new(db_pool.clone(), Arc::clone(&shared_state)).await?);
    info!("Playback engine created");

    // **[DBD-LIFECYCLE-060]** Assign chains to queue entries loaded from database
    // This ensures database-restored passages receive chain assignments, preventing
    // "ghost passages" that appear in queue but not in buffer chain monitor
    engine.assign_chains_to_loaded_queue().await;
    info!("Chain assignments completed for loaded queue");

    // Start playback engine
    engine.start().await?;
    info!("Playback engine started");

    // Start automatic validation service
    // **[ARCH-AUTO-VAL-001]** Periodic pipeline integrity validation
    PlaybackEngine::start_validation_service(Arc::clone(&engine), db_pool.clone()).await;
    info!("Validation service started");

    // Create Config struct for API server (temporary bridge to old config system)
    let config = Config {
        database_path: db_path.clone(),
        port: module_config.port,
        root_folder: Some(initializer.database_path().parent().unwrap().to_path_buf()),
        db_pool: None,
    };

    // Start HTTP API server
    let api_handle = tokio::spawn({
        let state = Arc::clone(&shared_state);
        let engine_ref = Arc::clone(&engine);
        let db_pool_clone = db_pool.clone();
        async move {
            if let Err(e) = api::server::run(config, state, engine_ref, db_pool_clone).await {
                error!("API server error: {}", e);
            }
        }
    });

    // Wait for API server (main service loop)
    api_handle.await?;

    // Shutdown playback engine
    engine.stop().await?;

    info!("WKMP Audio Player shutting down");
    Ok(())
}

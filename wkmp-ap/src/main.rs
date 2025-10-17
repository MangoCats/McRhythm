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
                .unwrap_or_else(|_| "wkmp_ap=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(true).with_file(true).with_line_number(true))
        .init();

    info!("Starting WKMP Audio Player (wkmp-ap)");

    // Parse command-line arguments
    let args = Args::parse();
    info!("Configuration file: {:?}", args.config);

    // Load configuration
    let config = Config::load(&args.config, args.database, args.port, args.root_folder).await?;
    info!("Loaded configuration: database={}, port={}, root_folder={:?}",
          config.database_path.display(), config.port, config.root_folder);

    // Initialize shared state
    let shared_state = Arc::new(SharedState::new());

    // Create database connection pool
    let db_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}", config.database_path.display()))
        .await?;
    info!("Connected to database");

    // Initialize playback engine
    let engine = Arc::new(PlaybackEngine::new(db_pool.clone(), Arc::clone(&shared_state)).await?);
    info!("Playback engine created");

    // Start playback engine
    engine.start().await?;
    info!("Playback engine started");

    // Start HTTP API server
    let api_handle = tokio::spawn({
        let config = config.clone();
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

//! WKMP Audio Player (wkmp-ap)
//!
//! Microservice responsible for audio playback using GStreamer,
//! crossfade management, queue handling, and play history recording.

use clap::Parser;
use std::path::PathBuf;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod playback;
mod server;
mod sse;

/// WKMP Audio Player - GStreamer-based audio playback microservice
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Root folder path (overrides environment variable and config file)
    #[arg(short, long, value_name = "PATH")]
    root_folder: Option<PathBuf>,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Host address to bind to (overrides database configuration)
    #[arg(long, value_name = "HOST")]
    host: Option<String>,

    /// Port to bind to (overrides database configuration)
    #[arg(short, long, value_name = "PORT")]
    port: Option<u16>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let log_level = if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("wkmp_ap={},wkmp_common={}", log_level, log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("WKMP Audio Player starting...");

    // Initialize GStreamer
    info!("Initializing GStreamer...");
    gstreamer::init()?;

    // Resolve root folder
    info!("Resolving root folder...");
    let root_folder = wkmp_common::config::resolve_root_folder(
        args.root_folder.as_deref().and_then(|p| p.to_str()),
        "WKMP_ROOT_FOLDER",
        Some("root_folder"),
    )?;
    info!("Root folder: {}", root_folder.display());

    // Ensure root folder exists
    std::fs::create_dir_all(&root_folder)?;

    // Initialize database
    info!("Initializing database...");
    let db_path = root_folder.join("wkmp.db");
    let db = wkmp_common::db::init_database(&db_path).await?;
    info!("Database initialized at: {}", db_path.display());

    // Load module configuration
    info!("Loading module configuration...");
    let module_config = wkmp_common::config::load_module_config(&db, "audio_player").await?;

    if !module_config.enabled {
        error!("Audio Player module is disabled in configuration");
        return Err(anyhow::anyhow!("Module disabled"));
    }

    // Determine bind address
    let host = args.host.unwrap_or(module_config.host);
    let port = args.port.unwrap_or(module_config.port);
    let bind_addr = format!("{}:{}", host, port);

    // Initialize playback engine
    info!("Initializing playback engine...");
    let mut engine = playback::PlaybackEngine::new(db.clone(), root_folder.clone());
    engine.init().await?;

    info!("Starting HTTP server on {}...", bind_addr);

    // Start server
    server::start(&bind_addr, db, root_folder, engine).await?;

    Ok(())
}

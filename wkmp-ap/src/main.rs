//! Audio Player (wkmp-ap) - Main entry point
//!
//! This is the audio playback microservice for WKMP, implementing
//! sample-accurate crossfading using the single-stream architecture.
//!
//! Implements requirements from:
//! - single-stream-design.md: Single-stream audio architecture
//! - api_design.md: REST API endpoints for playback control

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod error;
mod playback;
mod api;

use playback::PlaybackEngine;

/// Command-line arguments for wkmp-ap
#[derive(Parser, Debug)]
#[command(name = "wkmp-ap")]
#[command(about = "Audio Player microservice for WKMP")]
#[command(version)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "5740", env = "WKMP_AP_PORT")]
    port: u16,

    /// Root folder containing music files
    #[arg(short, long, env = "WKMP_ROOT_FOLDER")]
    root_folder: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wkmp_ap=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse command-line arguments
    let args = Args::parse();

    info!(
        "Starting WKMP Audio Player on port {}",
        args.port
    );
    info!(
        "Root folder: {}",
        args.root_folder.display()
    );

    // Initialize playback engine
    let engine = Arc::new(
        PlaybackEngine::new(args.root_folder.clone())
            .await
            .context("Failed to initialize playback engine")?
    );
    info!("Playback engine initialized");

    // Build the application router
    let app_state = api::AppState {
        engine,
        root_folder: args.root_folder.to_string_lossy().to_string(),
        port: args.port,
    };

    let app = api::create_router(app_state);

    // Create socket address
    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));

    info!("Starting HTTP server on {}", addr);

    // Create and run the server
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server error")?;

    info!("Server shutdown complete");
    Ok(())
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down");
        },
        _ = terminate => {
            info!("Received terminate signal, shutting down");
        },
    }
}
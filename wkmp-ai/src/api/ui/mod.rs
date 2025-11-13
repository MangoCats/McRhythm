//! UI Routes - HTML pages for wkmp-ai web interface
//!
//! **[AIA-UI-010]** Web UI with HTML/CSS/JS (vanilla ES6+, no frameworks)
//! **[AIA-UI-030]** Return navigation to wkmp-ui on completion
//!
//! # Structure
//! This module contains all UI page handlers for the wkmp-ai import wizard:
//!
//! - **Static Assets** (`static_assets`): CSS/JS file serving
//! - **Root Page** (`root`): Import wizard landing page
//! - **Import Progress** (`import_progress`): Real-time import progress with SSE
//! - **Segment Editor** (`segment_editor`): Manual passage boundary adjustment
//! - **Import Complete** (`import_complete`): Completion summary with return link
//! - **Settings Page** (`settings`): Configuration interface

use axum::{routing::get, Router};
use crate::AppState;

// Module declarations
mod static_assets;
mod root;
mod import_progress;
mod segment_editor;
mod import_complete;
mod settings;

// Re-export handler functions for router assembly
use static_assets::{
    serve_wkmp_sse_js,
    serve_wkmp_ui_css,
    serve_import_progress_js,
    serve_settings_css,
    serve_settings_js,
};
use root::root_page;
use import_progress::import_progress_page;
use segment_editor::segment_editor_page;
use import_complete::import_complete_page;
use settings::settings_page;

/// Build UI routes
pub fn ui_routes() -> Router<AppState> {
    Router::new()
        // Page routes
        .route("/", get(root_page))
        .route("/import-progress", get(import_progress_page))
        .route("/segment-editor", get(segment_editor_page))
        .route("/import-complete", get(import_complete_page))
        .route("/settings", get(settings_page))
        // Static assets
        .route("/static/wkmp-sse.js", get(serve_wkmp_sse_js))
        .route("/static/wkmp-ui.css", get(serve_wkmp_ui_css))
        .route("/static/import-progress.js", get(serve_import_progress_js))
        .route("/static/settings.css", get(serve_settings_css))
        .route("/static/settings.js", get(serve_settings_js))
}

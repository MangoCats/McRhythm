//! Static asset handlers for wkmp-ai UI
//!
//! Embeds and serves CSS/JS files at compile time

use axum::{http::StatusCode, response::{IntoResponse, Response}};

// Embed static files at compile time (same pattern as wkmp-dr)
const WKMP_SSE_JS: &str = include_str!("../../../../wkmp-common/static/wkmp-sse.js");
const WKMP_UI_CSS: &str = include_str!("../../../../wkmp-common/static/wkmp-ui.css");
const IMPORT_PROGRESS_JS: &str = include_str!("../../../static/import-progress.js");
const SETTINGS_HTML: &str = include_str!("../../../static/settings.html");
const SETTINGS_CSS: &str = include_str!("../../../static/settings.css");
const SETTINGS_JS: &str = include_str!("../../../static/settings.js");

// Re-export settings HTML for use by settings page handler
pub const SETTINGS_HTML_CONTENT: &str = SETTINGS_HTML;

/// GET /static/wkmp-sse.js
///
/// Serves the shared WKMP SSE utility from wkmp-common
pub async fn serve_wkmp_sse_js() -> Response {
    (
        StatusCode::OK,
        [
            ("content-type", "application/javascript"),
            ("cache-control", "no-cache, no-store, must-revalidate"),
        ],
        WKMP_SSE_JS,
    )
        .into_response()
}

/// GET /static/wkmp-ui.css
///
/// Serves the shared WKMP UI styles from wkmp-common
pub async fn serve_wkmp_ui_css() -> Response {
    (
        StatusCode::OK,
        [
            ("content-type", "text/css"),
            ("cache-control", "no-cache, no-store, must-revalidate"),
        ],
        WKMP_UI_CSS,
    )
        .into_response()
}

/// GET /static/import-progress.js
///
/// Serves the import progress page JavaScript
pub async fn serve_import_progress_js() -> Response {
    (
        StatusCode::OK,
        [
            ("content-type", "application/javascript"),
            ("cache-control", "no-cache, no-store, must-revalidate"),
        ],
        IMPORT_PROGRESS_JS,
    )
        .into_response()
}

/// GET /static/settings.css
///
/// Serves the settings page CSS
pub async fn serve_settings_css() -> Response {
    (
        StatusCode::OK,
        [
            ("content-type", "text/css"),
            ("cache-control", "no-cache, no-store, must-revalidate"),
        ],
        SETTINGS_CSS,
    )
        .into_response()
}

/// GET /static/settings.js
///
/// Serves the settings page JavaScript
pub async fn serve_settings_js() -> Response {
    (
        StatusCode::OK,
        [
            ("content-type", "application/javascript"),
            ("cache-control", "no-cache, no-store, must-revalidate"),
        ],
        SETTINGS_JS,
    )
        .into_response()
}

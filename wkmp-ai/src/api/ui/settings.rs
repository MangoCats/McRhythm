//! Settings page handler - Configuration interface

use super::static_assets::SETTINGS_HTML_CONTENT;
use axum::response::{Html, IntoResponse};

/// GET /settings
///
/// Configuration interface page
pub async fn settings_page() -> impl IntoResponse {
    Html(SETTINGS_HTML_CONTENT)
}

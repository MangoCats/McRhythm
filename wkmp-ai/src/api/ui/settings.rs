//! Settings page handler - Configuration interface

use axum::response::{Html, IntoResponse};
use super::static_assets::SETTINGS_HTML_CONTENT;

/// GET /settings
///
/// Configuration interface page
pub async fn settings_page() -> impl IntoResponse {
    Html(SETTINGS_HTML_CONTENT)
}

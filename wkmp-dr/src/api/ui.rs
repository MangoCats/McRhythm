//! UI serving routes
//!
//! Serves the static HTML/JS UI for database review

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::AppState;

const INDEX_HTML: &str = include_str!("../ui/index.html");
const APP_JS: &str = include_str!("../ui/app.js");

/// GET /
///
/// Serves the main UI page with embedded shared_secret
/// Per API-AUTH-028-A: Shared secret embedded as JavaScript variable
pub async fn serve_index(State(state): State<AppState>) -> Html<String> {
    // Replace {{SHARED_SECRET}} placeholder with actual value
    let html_with_secret = INDEX_HTML.replace("{{SHARED_SECRET}}", &state.shared_secret.to_string());
    Html(html_with_secret)
}

/// GET /static/app.js
///
/// Serves the JavaScript application
pub async fn serve_app_js() -> Response {
    (
        StatusCode::OK,
        [("content-type", "application/javascript")],
        APP_JS,
    )
        .into_response()
}

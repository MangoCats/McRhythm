//! UI serving routes
//!
//! Serves the static HTML/JS UI for database review

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

const INDEX_HTML: &str = include_str!("../ui/index.html");
const APP_JS: &str = include_str!("../ui/app.js");

/// GET /
///
/// Serves the main UI page
pub async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
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

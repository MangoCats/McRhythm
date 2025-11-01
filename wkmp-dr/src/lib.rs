//! wkmp-dr library - Database Review module
//!
//! [REQ-DR-NF-060]: On-demand microservice for read-only database inspection

use axum::Router;
use sqlx::SqlitePool;

pub mod api;
pub mod db;
pub mod pagination;

/// Application state shared across HTTP handlers
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool (read-only)
    pub db: SqlitePool,
    /// Shared secret for API authentication [REQ-DR-NF-030]
    pub shared_secret: i64,
}

impl AppState {
    /// Create new application state
    pub fn new(db: SqlitePool, shared_secret: i64) -> Self {
        Self { db, shared_secret }
    }
}

/// Build application router
///
/// [REQ-DR-NF-040]: Health endpoint (no auth)
/// [REQ-DR-NF-030]: Protected endpoints require auth
pub fn build_router(state: AppState) -> Router {
    use axum::routing::get;
    use axum::middleware;

    // Protected routes (require authentication)
    let protected = Router::new()
        .route("/api/table/:name", get(api::get_table_data))
        .route("/api/semantics/:table_name", get(api::get_table_semantics))
        .route("/api/filters/passages-without-mbid", get(api::passages_without_mbid))
        .route("/api/filters/files-without-passages", get(api::files_without_passages))
        .route("/api/search/by-work-id", get(api::search_by_work_id))
        .route("/api/search/by-path", get(api::search_by_path))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            api::auth_middleware,
        ));

    // Public routes (no authentication)
    let public = Router::new()
        .route("/", get(api::serve_index))
        .route("/static/app.js", get(api::serve_app_js))
        .merge(api::health_routes());

    // Combine routers
    Router::new()
        .merge(protected)
        .merge(public)
        .with_state(state)
}

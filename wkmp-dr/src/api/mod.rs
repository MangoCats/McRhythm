//! HTTP API handlers for wkmp-dr

pub mod auth;
pub mod buildinfo;
pub mod filters;
pub mod health;
pub mod search;
pub mod semantics;
pub mod table;
pub mod ui;

pub use auth::auth_middleware;
pub use buildinfo::get_build_info;
pub use filters::{files_without_passages, passages_without_mbid};
pub use health::health_routes;
pub use search::{search_by_path, search_by_work_id};
pub use semantics::get_table_semantics;
pub use table::get_table_data;
pub use ui::{serve_app_js, serve_index};

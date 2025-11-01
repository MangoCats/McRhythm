//! HTTP API handlers for wkmp-dr

pub mod auth;
pub mod health;
pub mod table;

pub use auth::auth_middleware;
pub use health::health_routes;
pub use table::get_table_data;

//! HTTP API handlers for wkmp-ai
//!
//! **[AIA-MS-010]** Microservices integration via HTTP REST + SSE
//! **[IMPL008]** API endpoint implementations

pub mod amplitude_analysis;
pub mod file_classification;
pub mod health;
pub mod import_workflow;
pub mod parameters;
pub mod settings;
pub mod sse;
pub mod ui; // PLAN027: File classification API

pub use amplitude_analysis::amplitude_routes;
pub use file_classification::file_classification_routes;
pub use health::health_routes;
pub use import_workflow::import_routes;
pub use parameters::parameter_routes;
pub use settings::settings_routes;
pub use sse::{event_stream, import_event_stream};
pub use ui::ui_routes;

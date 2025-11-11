//! HTTP API handlers for wkmp-ai
//!
//! **[AIA-MS-010]** Microservices integration via HTTP REST + SSE
//! **[IMPL008]** API endpoint implementations

pub mod import_workflow;
// REQ-TD-003: Amplitude analysis module removed (deferred to future release when use case is clarified)
pub mod parameters;
pub mod health;
pub mod sse;
pub mod ui;
pub mod settings;

pub use import_workflow::import_routes;
pub use parameters::parameter_routes;
pub use health::health_routes;
pub use sse::{event_stream, import_event_stream};
pub use ui::ui_routes;
pub use settings::settings_routes;

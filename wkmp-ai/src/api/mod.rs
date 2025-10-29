//! HTTP API handlers for wkmp-ai
//!
//! **[AIA-MS-010]** Microservices integration via HTTP REST + SSE
//! **[IMPL008]** API endpoint implementations

pub mod import_workflow;
pub mod amplitude_analysis;
pub mod parameters;
pub mod health;
pub mod sse;

pub use import_workflow::import_routes;
pub use amplitude_analysis::amplitude_routes;
pub use parameters::parameter_routes;
pub use health::health_routes;
pub use sse::import_event_stream;

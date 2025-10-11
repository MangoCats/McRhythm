//! Server-Sent Events (SSE) module

pub mod broadcaster;
pub mod events;

pub use broadcaster::SseBroadcaster;
pub use events::{SseEvent, SseEventData};

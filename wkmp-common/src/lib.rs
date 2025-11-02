//! # WKMP Common Library
//!
//! Shared code for all WKMP microservices including:
//! - API authentication (timestamp/hash validation)
//! - Database models and queries
//! - Event types (WkmpEvent enum)
//! - API request/response types
//! - Configuration loading
//! - Utility functions
//! - Fade curve definitions and calculations

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod events;
pub mod fade_curves;
pub mod human_time;
pub mod params;
pub mod time;
pub mod timing;
pub mod uuid_utils;

pub use error::{Error, Result};
pub use events::{
    BufferStatus, EnqueueSource, EventBus, PlaybackState, QueueChangeTrigger, UserActionType,
    WkmpEvent,
};
pub use fade_curves::FadeCurve;

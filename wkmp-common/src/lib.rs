//! # WKMP Common Library
//!
//! Shared code for all WKMP microservices including:
//! - Database models and queries
//! - Event types (WkmpEvent enum)
//! - API request/response types
//! - Configuration loading
//! - Utility functions

pub mod config;
pub mod db;
pub mod error;
pub mod events;
pub mod time;
pub mod uuid_utils;

pub use error::{Error, Result};

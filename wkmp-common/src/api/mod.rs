//! API module for shared HTTP API functionality
//!
//! Provides common authentication, types, and utilities used across all WKMP modules.
//!
//! # Architecture
//!
//! This module contains code shared by all 5 WKMP microservices:
//! - wkmp-ap (Audio Player)
//! - wkmp-ui (User Interface)
//! - wkmp-pd (Program Director)
//! - wkmp-ai (Audio Ingest)
//! - wkmp-le (Lyric Editor)
//!
//! # Design Principle
//!
//! This module contains ONLY:
//! - Pure functions (no HTTP framework dependencies)
//! - Database operations (via sqlx)
//! - Shared types
//!
//! Each module wraps these with framework-specific middleware (Axum, etc.).

pub mod auth;
pub mod types;

pub use auth::{
    calculate_hash, initialize_shared_secret, load_shared_secret, validate_hash,
    validate_timestamp, ApiAuthError,
};
pub use types::{AuthErrorResponse, AuthQuery, AuthRequest};

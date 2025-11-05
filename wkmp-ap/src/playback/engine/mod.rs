//! Playback engine module - refactored from monolithic engine.rs
//!
//! **Module Structure:**
//! - `core.rs`: Lifecycle, state management, orchestration
//! - `queue.rs`: Queue operations (enqueue, skip, clear, reorder, remove)
//! - `diagnostics.rs`: Monitoring, status accessors, event handlers
//! - `chains.rs`: Buffer chain management (assign, release)
//! - `playback.rs`: Playback controls (play, pause, seek, watchdog, crossfade)
//!
//! **Traceability:**
//! - [REQ-DEBT-QUALITY-002-010] Split into functional modules
//! - [REQ-DEBT-QUALITY-002-020] Each module <1500 lines (target <1000 LOC per PLAN021)
//! - [REQ-DEBT-QUALITY-002-030] Public API unchanged
//! - [PLAN016] Engine refactoring implementation (3 modules)
//! - [PLAN021] Technical debt remediation (chains.rs, playback.rs extracted from core.rs)

mod core;
mod queue;
mod diagnostics;
mod chains;
mod playback;

// Re-export PlaybackEngine as public API
pub use core::PlaybackEngine;

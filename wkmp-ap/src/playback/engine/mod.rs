//! Playback engine module - refactored from monolithic engine.rs
//!
//! **Module Structure:**
//! - `core.rs`: Lifecycle, state management, orchestration (watchdog_check)
//! - `queue.rs`: Queue operations (enqueue, skip, clear, reorder, remove)
//! - `diagnostics.rs`: Monitoring, status accessors, event handlers
//!
//! **Traceability:**
//! - [REQ-DEBT-QUALITY-002-010] Split into 3 functional modules
//! - [REQ-DEBT-QUALITY-002-020] Each module <1500 lines
//! - [REQ-DEBT-QUALITY-002-030] Public API unchanged
//! - [PLAN016] Engine refactoring implementation

mod core;
mod queue;
mod diagnostics;

// Re-export PlaybackEngine as public API
pub use core::PlaybackEngine;

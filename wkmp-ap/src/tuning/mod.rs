//! # Buffer Auto-Tuning Module
//!
//! Automatically determines optimal buffer parameters (mixer_check_interval_ms and
//! audio_buffer_size) for stable audio playback on the current system.
//!
//! **Purpose:** Systematically test parameter combinations to find the minimum
//! latency configuration that maintains <0.1% underrun rate.
//!
//! **Algorithm:** Two-phase search:
//! - Phase 1: Coarse sweep of mixer intervals with default buffer
//! - Phase 2: Binary search for minimum stable buffer per viable interval
//!
//! **Traceability:** Implements SPEC008-buffer_autotuning.md (PLAN004)

pub mod curve;
pub mod metrics;
pub mod report;
pub mod safety;
pub mod search;
pub mod system_info;
pub mod test_harness;

pub use curve::{generate_recommendations, CurvePoint, CurveStatus, Recommendations, Recommendation, ConfidenceLevel};
pub use metrics::{TestResult, UnderrunMetrics, Verdict};
pub use report::{TuningReport, CliFormatter};
pub use search::binary_search_min_buffer;
pub use system_info::SystemInfo;
pub use test_harness::{TestConfig, TestHarness};

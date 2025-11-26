//! Multi-Stage Album Matching
//!
//! **[PLAN030]** Multi-stage algorithm for album identification:
//!
//! - Stage 2: Parameter grid search (180 combinations)
//! - Stage 3: Over-segmentation assembly (future)
//! - Stage 4: Quiet spot detection (future)
//! - Stage 5: Extra track merging (future)

pub mod stage2;

pub use stage2::{run_stage2, Stage2Result, EarlyExitConfig};

//! Multi-Stage Album Matching
//!
//! **[PLAN030]** Multi-stage algorithm for album identification:
//!
//! - Stage 2: Parameter grid search (180 combinations)
//! - Stage 3: Over-segmentation assembly
//! - Stage 4: Quiet spot detection
//! - Stage 5: Extra track merging
//!
//! Boundary refinement post-processes results to fix split failures.

pub mod boundary_refinement;
pub mod stage2;
pub mod stage3;
pub mod stage4;
pub mod stage5;
pub mod stage7_progressive_refinement;

pub use boundary_refinement::{apply_refinement_to_durations, refine_missed_boundaries};
pub use stage2::{run_stage2, EarlyExitConfig, Stage2Result};
pub use stage3::{run_stage3, Stage3Result};
pub use stage4::{run_stage4, RmsProfile, Stage4Result};
pub use stage5::{run_stage5, Stage5Result};
pub use stage7_progressive_refinement::apply_stage7_if_needed;

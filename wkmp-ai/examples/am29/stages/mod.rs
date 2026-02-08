//! # Matching Stages (Stage 2-5)
//!
//! Five-stage album matching pipeline.
//!
//! ## Stages
//! - Stage 2: 180-parameter silence detection sweep with early-exit
//! - Stage 3: Dynamic programming assembly for over-segmented tracks
//! - Stage 4: Edition-guided quiet spot detection (RMS profiling)
//! - Stage 5: Extra track merging

pub(crate) mod stage2;
pub(crate) mod stage3;
pub(crate) mod stage4;
pub(crate) mod stage5;

// Re-export key functions
pub use stage2::*;
pub use stage3::*;
pub use stage4::*;
pub use stage5::*;

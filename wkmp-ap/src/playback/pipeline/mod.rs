//! Audio pipeline components
//!
//! Single-stream pipeline implementation.
//!
//! **Traceability:** Single-Stream Design - Component Structure

pub mod fade_curves;
pub mod mixer;
pub mod timing;

pub use fade_curves::FadeCurve;
pub use mixer::CrossfadeMixer;
pub use timing::{CrossfadeTiming, PassageTiming};

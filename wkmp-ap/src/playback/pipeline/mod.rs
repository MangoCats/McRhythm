//! GStreamer pipeline management

pub mod dual;
pub mod single;

pub use dual::{ActivePipeline, DualPipeline};
pub use single::SinglePipeline;

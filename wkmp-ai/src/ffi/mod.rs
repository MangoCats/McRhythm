//! FFI bindings for external libraries
//!
//! Provides Rust bindings to C/C++ libraries used by wkmp-ai:
//! - **chromaprint**: Audio fingerprinting library for AcoustID integration
//!
//! **[PLAN024]** Ground-up recode with safe FFI wrappers

pub mod chromaprint;

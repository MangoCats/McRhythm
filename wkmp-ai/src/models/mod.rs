//! Data models for wkmp-ai (Audio Ingest microservice)
//!
//! - [AIA-WF-010]: Import workflow state machine
//! - [AIA-ASYNC-010]: Background job state tracking

pub mod import_session;
pub mod parameters;
pub mod amplitude_profile;
pub mod import_result;

pub use import_session::{ImportSession, ImportState, ImportProgress, StateTransition};
pub use parameters::{ImportParameters, AmplitudeParameters};
pub use amplitude_profile::{AmplitudeProfile, AmplitudeAnalysisRequest, AmplitudeAnalysisResponse};
pub use import_result::{ImportResult, ImportError, ErrorSeverity};

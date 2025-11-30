//! Data models for wkmp-ai (Audio Ingest microservice)
//!
//! - [AIA-WF-010]: Import workflow state machine
//! - [AIA-ASYNC-010]: Background job state tracking
//! - [AIA-INIT-010]: Two-stage database initialization

pub mod amplitude_profile;
pub mod bootstrap_config;
pub mod import_result;
pub mod import_session;
pub mod parameters;

pub use amplitude_profile::{
    AmplitudeAnalysisRequest, AmplitudeAnalysisResponse, AmplitudeProfile,
};
pub use bootstrap_config::WkmpAiBootstrapConfig;
pub use import_result::{ErrorSeverity, ImportError, ImportResult};
pub use import_session::{
    FileClassification,
    FileInfo,
    ImportProgress,
    ImportSession,
    ImportState,
    StateTransition,
    VerificationStatus, // PLAN027: File classification
};
pub use parameters::{AmplitudeParameters, ImportParameters};

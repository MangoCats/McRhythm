//! Service modules for audio ingest workflow
//!
//! **[AIA-COMP-010]** Component implementations

pub mod acousticbrainz_client;
pub mod acoustid_client;
pub mod amplitude_analyzer;
pub mod essentia_client;
pub mod file_scanner;
pub mod file_tracker;  // PLAN024 TASK-000: File-level import tracking
pub mod fingerprinter;
pub mod metadata_extractor;
pub mod musicbrainz_client;
pub mod silence_detector;
pub mod workflow_orchestrator;

pub use acousticbrainz_client::{ABError, ABLowLevel, AcousticBrainzClient, MusicalFlavorVector};
pub use acoustid_client::{AcoustIDClient, AcoustIDError, AcoustIDResponse};
pub use amplitude_analyzer::{AmplitudeAnalysisResult, AmplitudeAnalyzer, AnalysisError};
pub use essentia_client::{EssentiaClient, EssentiaError, EssentiaOutput};
pub use file_scanner::{FileScanner, ScanError, ScanResult};
pub use file_tracker::{
    FileTracker, FileTrackerConfig, FileTrackingInfo, SkipDecision, SkipReason,
};
pub use fingerprinter::{Fingerprinter, FingerprintError};
pub use metadata_extractor::{AudioMetadata, MetadataError, MetadataExtractor};
pub use musicbrainz_client::{MBError, MBRecording, MusicBrainzClient};
pub use silence_detector::{SilenceDetector, SilenceRegion};
pub use workflow_orchestrator::WorkflowOrchestrator;

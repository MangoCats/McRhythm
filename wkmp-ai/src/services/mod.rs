//! Service modules for audio ingest workflow
//!
//! **[AIA-COMP-010]** Component implementations
//!
//! **PLAN024 Sprint 3**: Legacy services remaining after migration to import_v2:
//! - AcoustID/MusicBrainz clients moved to import_v2/tier1/
//! - WorkflowOrchestrator replaced by import_v2::SessionOrchestrator

pub mod acousticbrainz_client;
pub mod amplitude_analyzer;
pub mod essentia_client;
pub mod file_scanner;
pub mod fingerprinter;
pub mod metadata_extractor;
pub mod silence_detector;

pub use acousticbrainz_client::{ABError, ABLowLevel, AcousticBrainzClient, MusicalFlavorVector};
pub use amplitude_analyzer::{AmplitudeAnalysisResult, AmplitudeAnalyzer, AnalysisError};
pub use essentia_client::{EssentiaClient, EssentiaError, EssentiaOutput};
pub use file_scanner::{FileScanner, ScanError, ScanResult};
pub use fingerprinter::{Fingerprinter, FingerprintError};
pub use metadata_extractor::{AudioMetadata, MetadataError, MetadataExtractor};
pub use silence_detector::{SilenceDetector, SilenceRegion};

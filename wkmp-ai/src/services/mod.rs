//! Service modules for audio ingest workflow
//!
//! **[AIA-COMP-010]** Component implementations

pub mod acousticbrainz_client;
pub mod acoustid_client;
pub mod amplitude_analyzer;
pub mod api_key_validator;  // PLAN024 Increment 4: AcoustID API key validation (Step 1)
pub mod confidence_assessor;  // PLAN025 Phase 2: Evidence-based confidence assessment
pub mod contextual_matcher;  // PLAN025 Phase 2: Contextual MusicBrainz matching
pub mod essentia_client;
pub mod file_scanner;
pub mod file_tracker;  // PLAN024 TASK-000: File-level import tracking
pub mod filename_matcher;  // PLAN024 Increment 6-7: Filename matching (Phase 1)
pub mod fingerprinter;
pub mod folder_selector;  // PLAN024 Increment 5: Folder selection (Step 2)
pub mod hash_deduplicator;  // PLAN024 Increment 6-7: Hash deduplication (Phase 2)
pub mod metadata_extractor;
pub mod metadata_merger;  // PLAN024 Increment 8-9: Metadata extraction & merging (Phase 3)
pub mod musicbrainz_client;
pub mod passage_fingerprinter;  // PLAN024 Increment 12-13: Per-passage fingerprinting (Phase 5)
pub mod passage_segmenter;  // PLAN024 Increment 10-11: Passage segmentation (Phase 4)
pub mod passage_song_matcher;  // PLAN024 Increment 14-15: Song matching (Phase 6)
pub mod pattern_analyzer;  // PLAN025 Phase 2: Pattern analysis for source media classification
pub mod silence_detector;
pub mod workflow_orchestrator;

pub use acousticbrainz_client::{ABError, ABLowLevel, AcousticBrainzClient, MusicalFlavorVector};
pub use acoustid_client::{AcoustIDClient, AcoustIDError, AcoustIDResponse};
pub use amplitude_analyzer::{AmplitudeAnalysisResult, AmplitudeAnalyzer, AnalysisError};
pub use api_key_validator::{ApiKeyValidator, UserChoice, ValidationResult};
pub use confidence_assessor::{ConfidenceAssessor, ConfidenceError, ConfidenceResult, Decision, Evidence};
pub use contextual_matcher::{ContextualMatcher, ContextualMatcherError, MatchCandidate};
pub use essentia_client::{EssentiaClient, EssentiaError, EssentiaOutput};
pub use file_scanner::{FileScanner, ScanError, ScanResult};
pub use file_tracker::{
    FileTracker, FileTrackerConfig, FileTrackingInfo, SkipDecision, SkipReason,
};
pub use filename_matcher::{FilenameMatcher, MatchResult};
pub use fingerprinter::{Fingerprinter, FingerprintError};
pub use folder_selector::{FolderSelector, SelectionResult};
pub use hash_deduplicator::{HashDeduplicator, HashResult};
pub use metadata_extractor::{AudioMetadata, MetadataError, MetadataExtractor};
pub use metadata_merger::{MergedMetadata, MetadataMerger};
pub use musicbrainz_client::{MBError, MBRecording, MusicBrainzClient};
pub use passage_fingerprinter::{
    FingerprintResult, MBIDCandidate, PassageFingerprint, PassageFingerprinter,
};
pub use passage_segmenter::{PassageBoundary, PassageSegmenter, SegmentResult};
pub use passage_song_matcher::{
    ConfidenceLevel, PassageSongMatch, PassageSongMatcher, SongMatchResult, SongMatchStats,
};
pub use pattern_analyzer::{
    GapPattern, PatternAnalyzer, PatternError, PatternMetadata, Segment, SourceMedia,
};
pub use silence_detector::{SilenceDetector, SilenceRegion};
pub use workflow_orchestrator::WorkflowOrchestrator;

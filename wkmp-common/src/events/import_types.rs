//! Import workflow type definitions
//!
//! Supporting types for wkmp-ai import workflow progress tracking.

use serde::{Deserialize, Serialize};

/// **PLAN024 Phase-Specific Statistics**
///
/// Per wkmp-ai_refinement.md UI requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "phase_name")]
pub enum PhaseStatistics {
    #[serde(rename = "SCANNING")]
    Scanning {
        potential_files_found: usize,
        is_scanning: bool,
        audio_files: usize,
        image_files: usize,
        other_files: usize,
        /// Total files discovered (sum of all file types)
        total_files: usize,
        /// Files that have completed magic byte analysis
        magic_byte_analyzed: usize,
        /// Confirmed audio files (extension + magic bytes match)
        audio_confirmed: usize,
        /// Confirmed image files (extension + magic bytes match)
        image_confirmed: usize,
        /// Confirmed other files (non-audio, non-image)
        other_confirmed: usize,
        /// Audio files with unrecognized extension (magic bytes say audio, extension doesn't)
        audio_unrecognized_ext: usize,
        /// Image files with unrecognized extension (magic bytes say image, extension doesn't)
        image_unrecognized_ext: usize,
        /// Files with misleading extension (extension says one thing, magic bytes say another)
        misleading_extension: usize,
    },
    #[serde(rename = "PROCESSING")]
    Processing {
        completed: usize,
        started: usize,
        total: usize,
        /// **[AIA-UI-010]** Real-time worker activity tracking
        workers: Vec<WorkerActivity>,
        /// Maximum concurrent worker threads configured
        max_workers: usize,
        /// List of files that have started or completed processing
        files: Vec<FileProcessingStatus>,
    },
    #[serde(rename = "FILENAME_MATCHING")]
    FilenameMatching {
        completed_filenames_found: usize,
    },
    #[serde(rename = "HASHING")]
    Hashing {
        hashes_computed: usize,
        matches_found: usize,
    },
    #[serde(rename = "EXTRACTING")]
    Extracting {
        successful_extractions: usize,
        failures: usize,
    },
    #[serde(rename = "SEGMENTING")]
    Segmenting {
        files_processed: usize,
        potential_passages: usize,
        finalized_passages: usize,
        songs_identified: usize,
    },
    #[serde(rename = "FINGERPRINTING")]
    Fingerprinting {
        passages_fingerprinted: usize,
        successful_matches: usize,
    },
    #[serde(rename = "SONG_MATCHING")]
    SongMatching {
        high_confidence: usize,
        medium_confidence: usize,
        low_confidence: usize,
        no_confidence: usize,
    },
    #[serde(rename = "RECORDING")]
    Recording {
        recorded_passages: Vec<RecordedPassageInfo>,
    },
    #[serde(rename = "AMPLITUDE")]
    Amplitude {
        analyzed_passages: Vec<AnalyzedPassageInfo>,
    },
    #[serde(rename = "FLAVORING")]
    Flavoring {
        pre_existing: usize,
        acousticbrainz: usize,
        essentia: usize,
        failed: usize,
    },
    #[serde(rename = "PASSAGES_COMPLETE")]
    PassagesComplete {
        passages_completed: usize,
    },
    #[serde(rename = "FILES_COMPLETE")]
    FilesComplete {
        files_completed: usize,
    },
}

/// Recorded passage information for RECORDING phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedPassageInfo {
    pub song_title: Option<String>,
    pub file_path: String,
}

/// Analyzed passage information for AMPLITUDE phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedPassageInfo {
    pub song_title: Option<String>,
    pub passage_length_seconds: f64,
    pub lead_in_ms: u64,
    pub lead_out_ms: u64,
}

/// **[REQ-AIA-UI-001]** Phase status for import workflow checklist
///
/// Used in SSE events for UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseStatusData {
    /// Phase not yet started
    Pending,
    /// Phase currently running
    InProgress,
    /// Phase completed successfully
    Completed,
    /// Phase failed with critical error
    Failed,
    /// Phase completed with warnings (partial success)
    CompletedWithWarnings,
}

/// **[REQ-AIA-UI-003]** Sub-task tracking for import phases
///
/// Used to show success/failure counts (e.g., Chromaprint, AcoustID, MusicBrainz)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskData {
    /// Sub-task name
    pub name: String,
    /// Number of successful operations
    pub success_count: usize,
    /// Number of failed operations
    pub failure_count: usize,
    /// Number of skipped operations
    pub skip_count: usize,
}

/// **[REQ-AIA-UI-001]** Phase progress data for SSE events
///
/// Contains progress information for a single workflow phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseProgressData {
    /// Phase name (e.g., "SCANNING", "EXTRACTING", "FINGERPRINTING")
    pub phase: String,
    /// Current status
    pub status: PhaseStatusData,
    /// Files processed in this phase
    pub progress_current: usize,
    /// Total files for this phase
    pub progress_total: usize,
    /// Sub-task counters
    pub subtasks: Vec<SubTaskData>,
    /// Brief description of what this phase does (8 words max)
    pub description: String,
}

/// **[AIA-UI-010]** Worker activity tracking for real-time progress visibility
///
/// Tracks what each parallel worker thread is currently doing during the PROCESSING phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerActivity {
    /// Worker identifier (thread ID or worker index)
    pub worker_id: String,
    /// File path being processed (relative to root folder)
    pub file_path: Option<String>,
    /// File index (for progress tracking)
    pub file_index: Option<usize>,
    /// Current phase number (1-10)
    pub phase_number: Option<u8>,
    /// Current phase name (e.g., "Filename Matching", "Hash Deduplication")
    pub phase_name: Option<String>,
    /// Timestamp when current phase started
    pub phase_started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Elapsed milliseconds in current phase
    pub elapsed_ms: Option<u64>,
    /// Passage start time in seconds (for passage-level processing phases)
    pub passage_start_seconds: Option<f64>,
    /// Passage end time in seconds (for passage-level processing phases)
    pub passage_end_seconds: Option<f64>,
}

/// File processing status for tracking individual file progress
///
/// Shows current state and completion status for all files that have started or completed processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProcessingStatus {
    /// File's sequence number in the processing queue
    pub file_index: usize,
    /// Relative path and filename from root folder
    pub file_path: String,
    /// Current processing state
    pub state: FileState,
    /// Total processing time in seconds (None if still in progress)
    pub total_time_seconds: Option<f64>,
}

/// Processing state for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "stage")]
pub enum FileState {
    /// File is currently being processed (with current stage name)
    Processing(String),
    /// File completed successfully
    IngestComplete,
    /// File skipped due to duplicate hash
    DuplicateHash,
    /// File skipped due to no audio content
    NoAudio,
}

// =============================================================================
// Analysis Log Types (PLAN032 - Real-Time Analysis Log UI)
// =============================================================================

/// **[PLAN032]** Detailed log message for UI display during import analysis
///
/// Provides timestamped, filterable log entries for album matching results,
/// AcousticBrainz lookups, and other analysis events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisLogEntry {
    /// When this log entry was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Current file index (1-based) in the import batch
    pub file_index: u32,
    /// Total files in the import batch
    pub total_files: u32,
    /// File path being processed
    pub file_path: String,
    /// Type of log message (for filtering)
    pub message_type: AnalysisLogType,
    /// Human-readable message
    pub message: String,
    /// Optional structured details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<AnalysisLogDetails>,
}

/// **[PLAN032]** Log message type for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisLogType {
    /// General informational message
    Info,
    /// Successful operation
    Success,
    /// Warning (non-fatal issue)
    Warning,
    /// Error (operation failed)
    Error,
    /// Album matching result
    AlbumMatch,
    /// AcousticBrainz/Essentia flavor lookup
    FlavorLookup,
}

/// **[PLAN032]** Structured details for specific log types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnalysisLogDetails {
    /// Album matching result with track-by-track timing errors
    AlbumMatch {
        /// Matched album title
        album_title: String,
        /// Matched artist name
        artist: String,
        /// Match percentage (0-100)
        match_percentage: f64,
        /// Number of tracks in the matched edition
        track_count: u32,
        /// Per-track timing errors
        track_errors: Vec<TrackTimingError>,
    },
    /// Flavor lookup result (per passage/song)
    FlavorLookup {
        /// Passage index within file (1-based)
        passage_index: u32,
        /// Total passages in file
        passage_total: u32,
        /// Song title if known
        #[serde(skip_serializing_if = "Option::is_none")]
        song_title: Option<String>,
        /// Source of flavor data ("AcousticBrainz", "Essentia", "PreExisting")
        source: String,
        /// Whether lookup succeeded
        success: bool,
        /// MusicBrainz recording ID if found
        #[serde(skip_serializing_if = "Option::is_none")]
        recording_mbid: Option<String>,
    },
}

/// **[PLAN032]** Track timing error for album match details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackTimingError {
    /// Track number (1-based)
    pub track_number: u32,
    /// Track title
    pub track_title: String,
    /// Expected duration from MusicBrainz (seconds)
    pub expected_duration_secs: f64,
    /// Detected duration from silence analysis (seconds)
    pub detected_duration_secs: f64,
    /// Timing error (detected - expected, seconds)
    pub error_secs: f64,
}

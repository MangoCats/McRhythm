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

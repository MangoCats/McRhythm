//! Import workflow state machine
//!
//! **[AIA-WF-010]** Import session progresses through states:
//!
//! **Current (PLAN024):** SCANNING → PROCESSING → COMPLETED
//!
//! **Deprecated (Legacy):** SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING → COMPLETED

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use uuid::Uuid;

// ========================================
// File Classification Data Structures
// **[AIA-CLASSIFY-030]** Per SPEC032 v2.2
// ========================================

/// **[AIA-CLASSIFY-040]** Verification status for file classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    /// Extension classification confirmed by magic byte signature
    Confirmed,
    /// Extension classification denied by magic byte signature (mismatched)
    Denied,
    /// Extension-only classification (no magic byte verification performed)
    ExtensionOnly,
}

/// **[AIA-CLASSIFY-030]** File metadata for classification report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Absolute path to file
    pub path: PathBuf,
    /// File size in bytes
    pub size_bytes: u64,
    /// File last modified timestamp
    #[serde(with = "systemtime_serde")]
    pub modified_at: SystemTime,
    /// **[AIA-CLASSIFY-040]** Magic byte verification status
    pub verification_status: VerificationStatus,
}

impl FileInfo {
    /// Create new FileInfo from path and metadata
    pub fn new(path: PathBuf, size_bytes: u64, modified_at: SystemTime) -> Self {
        Self {
            path,
            size_bytes,
            modified_at,
            verification_status: VerificationStatus::ExtensionOnly,
        }
    }

    /// Create new FileInfo with verification status
    pub fn with_verification(
        path: PathBuf,
        size_bytes: u64,
        modified_at: SystemTime,
        verification_status: VerificationStatus,
    ) -> Self {
        Self {
            path,
            size_bytes,
            modified_at,
            verification_status,
        }
    }
}

/// **[AIA-CLASSIFY-030]** File classification results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileClassification {
    /// Audio files (MP3, FLAC, OGG, M4A, AAC, OPUS, WAV)
    pub audio_files: Vec<FileInfo>,
    /// Image files (JPG, PNG, GIF, BMP, WEBP, TIFF)
    pub image_files: Vec<FileInfo>,
    /// Other files (all remaining)
    pub other_files: Vec<FileInfo>,
    /// When scan completed (classification finalized)
    pub scan_completed_at: Option<DateTime<Utc>>,
    /// **[AIA-CLASSIFY-040]** Verification statistics
    pub audio_confirmed: usize,
    pub audio_denied: usize,
    pub image_confirmed: usize,
    pub image_denied: usize,
}

impl FileClassification {
    /// Create new empty classification
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total count of all files
    pub fn total_count(&self) -> usize {
        self.audio_files.len() + self.image_files.len() + self.other_files.len()
    }

    /// Get total size of all audio files
    pub fn audio_total_size(&self) -> u64 {
        self.audio_files.iter().map(|f| f.size_bytes).sum()
    }

    /// Get total size of all image files
    pub fn image_total_size(&self) -> u64 {
        self.image_files.iter().map(|f| f.size_bytes).sum()
    }

    /// Get total size of all other files
    pub fn other_total_size(&self) -> u64 {
        self.other_files.iter().map(|f| f.size_bytes).sum()
    }

    /// Get total size of all files
    pub fn total_size(&self) -> u64 {
        self.audio_total_size() + self.image_total_size() + self.other_total_size()
    }

    /// Mark scan as completed with current timestamp
    pub fn mark_completed(&mut self) {
        self.scan_completed_at = Some(Utc::now());
    }

    /// Sort all file lists alphabetically by path (case-insensitive)
    pub fn sort_all(&mut self) {
        self.audio_files.sort_by(|a, b| {
            a.path
                .to_string_lossy()
                .to_lowercase()
                .cmp(&b.path.to_string_lossy().to_lowercase())
        });
        self.image_files.sort_by(|a, b| {
            a.path
                .to_string_lossy()
                .to_lowercase()
                .cmp(&b.path.to_string_lossy().to_lowercase())
        });
        self.other_files.sort_by(|a, b| {
            a.path
                .to_string_lossy()
                .to_lowercase()
                .cmp(&b.path.to_string_lossy().to_lowercase())
        });
    }

    /// **[AIA-CLASSIFY-040]** Recalculate verification statistics from file lists
    ///
    /// Should be called after files are added/modified to update statistics
    pub fn update_verification_stats(&mut self) {
        self.audio_confirmed = self
            .audio_files
            .iter()
            .filter(|f| f.verification_status == VerificationStatus::Confirmed)
            .count();
        self.audio_denied = self
            .audio_files
            .iter()
            .filter(|f| f.verification_status == VerificationStatus::Denied)
            .count();
        self.image_confirmed = self
            .image_files
            .iter()
            .filter(|f| f.verification_status == VerificationStatus::Confirmed)
            .count();
        self.image_denied = self
            .image_files
            .iter()
            .filter(|f| f.verification_status == VerificationStatus::Denied)
            .count();
    }

    /// **[AIA-CLASSIFY-040]** Get verification summary string for logging/UI
    pub fn verification_summary(&self) -> String {
        format!(
            "Audio: {} confirmed, {} denied | Images: {} confirmed, {} denied",
            self.audio_confirmed, self.audio_denied, self.image_confirmed, self.image_denied
        )
    }
}

// Custom serde module for SystemTime serialization
mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

// ========================================
// Import Workflow State Machine
// ========================================

/// **[AIA-WF-010]** Import workflow state
///
/// **PLAN024 Architecture (Current):**
/// - SCANNING → BULK_INSERTING → PROCESSING → COMPLETED
///
/// **Legacy Architecture (Deprecated):**
/// - SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING → COMPLETED
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ImportState {
    /// Phase 1: Directory traversal, finding audio files, magic byte verification
    Scanning,

    /// Phase 1.5: Creating minimal database records for confirmed audio files
    /// **[AIA-CLASSIFY-040]** Only files with confirmed magic bytes are inserted
    #[serde(rename = "BULK_INSERTING")]
    BulkInserting,

    /// Phase 2: Per-file pipeline (PLAN024) - Each file goes through 10 sub-phases
    /// **[AIA-ASYNC-020]** N workers process files concurrently
    #[serde(rename = "PROCESSING")]
    Processing,

    /// Import finished successfully
    Completed,
    /// Import cancelled by user
    Cancelled,
    /// Import failed with critical error
    Failed,

    // ========================================
    // DEPRECATED BATCH-PHASE STATES
    // Preserved for backward compatibility with existing database sessions
    // **[AIA-WF-020]** Batch phases DEPRECATED as of PLAN024
    // ========================================
    /// **DEPRECATED:** Batch metadata extraction phase (replaced by Processing)
    #[deprecated(since = "0.1.0", note = "Use Processing state with per-file pipeline")]
    Extracting,

    /// **DEPRECATED:** Batch passage boundary detection phase (replaced by Processing)
    #[deprecated(since = "0.1.0", note = "Use Processing state with per-file pipeline")]
    Segmenting,

    /// **DEPRECATED:** Batch fingerprinting phase (replaced by Processing)
    #[deprecated(since = "0.1.0", note = "Use Processing state with per-file pipeline")]
    Fingerprinting,

    /// **DEPRECATED:** Batch music identification phase (replaced by Processing)
    #[deprecated(since = "0.1.0", note = "Use Processing state with per-file pipeline")]
    Identifying,

    /// **DEPRECATED:** Batch amplitude analysis phase (replaced by Processing)
    #[deprecated(since = "0.1.0", note = "Use Processing state with per-file pipeline")]
    Analyzing,

    /// **DEPRECATED:** Batch musical flavor extraction phase (replaced by Processing)
    #[deprecated(since = "0.1.0", note = "Use Processing state with per-file pipeline")]
    Flavoring,
}

impl ImportState {
    /// **[REQ-AIA-UI-001]** Get brief description of what this phase does (8 words max)
    pub fn description(&self) -> &'static str {
        match self {
            ImportState::Scanning => "Finding files in directories",
            ImportState::BulkInserting => "Creating minimal records for confirmed audio files",
            ImportState::Extracting => "Calculating hashes and extracting basic metadata",
            ImportState::Segmenting => "Detecting silence and passage boundaries",
            ImportState::Fingerprinting => "Generating audio fingerprints via Chromaprint",
            ImportState::Identifying => "Resolving music identity via MusicBrainz",
            ImportState::Analyzing => "Analyzing amplitude for crossfade timing",
            ImportState::Flavoring => "Extracting musical characteristics via Essentia",
            ImportState::Processing => "Processing passages through hybrid fusion pipeline",
            ImportState::Completed => "Import completed successfully",
            ImportState::Cancelled => "Import cancelled by user",
            ImportState::Failed => "Import failed with errors",
        }
    }
}

/// **[AIA-WF-010]** State transition event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// Import session identifier
    pub session_id: Uuid,
    /// State before transition
    pub old_state: ImportState,
    /// State after transition
    pub new_state: ImportState,
    /// When transition occurred
    pub transitioned_at: DateTime<Utc>,
}

/// **[AIA-WF-020]** Import session (in-memory state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSession {
    /// Unique session identifier
    pub session_id: Uuid,

    /// Current workflow state
    pub state: ImportState,

    /// Root folder being imported
    pub root_folder: String,

    /// Import parameters
    pub parameters: crate::models::ImportParameters,

    /// Progress tracking
    pub progress: ImportProgress,

    /// Accumulated errors
    pub errors: Vec<crate::models::ImportError>,

    /// Session start time
    pub started_at: DateTime<Utc>,

    /// Session end time (if completed/cancelled/failed)
    pub ended_at: Option<DateTime<Utc>>,

    /// **[AIA-CLASSIFY-030]** File classification results (populated during SCANNING phase)
    pub file_classification: FileClassification,
}

/// **[REQ-AIA-UI-001]** Phase status for workflow checklist
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseStatus {
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

/// **[REQ-AIA-UI-003]** Sub-task success/failure tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskStatus {
    /// Sub-task name (e.g., "Chromaprint", "AcoustID", "MusicBrainz")
    pub name: String,
    /// Number of successful operations
    pub success_count: usize,
    /// Number of failed operations
    pub failure_count: usize,
    /// Number of skipped operations
    pub skip_count: usize,
}

impl SubTaskStatus {
    /// Create new sub-task status tracker
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            success_count: 0,
            failure_count: 0,
            skip_count: 0,
        }
    }

    /// Calculate success rate percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            return 0.0;
        }
        (self.success_count as f64 / total as f64) * 100.0
    }

    /// Get color indicator based on success rate thresholds
    /// Green: >95%, Yellow: 85-95%, Red: <85%
    pub fn color_indicator(&self) -> &'static str {
        let rate = self.success_rate();
        if rate > 95.0 {
            "green"
        } else if rate >= 85.0 {
            "yellow"
        } else {
            "red"
        }
    }
}

/// **[REQ-AIA-UI-001]** Individual phase progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseProgress {
    /// Which workflow phase this represents
    pub phase: ImportState,
    /// Current status of this phase
    pub status: PhaseStatus,
    /// Files processed in this phase
    pub progress_current: usize,
    /// Total files for this phase
    pub progress_total: usize,
    /// Sub-task counters (e.g., Chromaprint, AcoustID for Fingerprinting phase)
    pub subtasks: Vec<SubTaskStatus>,
}

impl PhaseProgress {
    /// Create new phase tracker
    pub fn new(phase: ImportState) -> Self {
        Self {
            phase,
            status: PhaseStatus::Pending,
            progress_current: 0,
            progress_total: 0,
            subtasks: Vec::new(),
        }
    }

    /// Calculate phase progress percentage
    pub fn percentage(&self) -> f64 {
        if self.progress_total == 0 {
            return 0.0;
        }
        (self.progress_current as f64 / self.progress_total as f64) * 100.0
    }

    /// Generate summary text for completed phase
    pub fn summary(&self) -> Option<String> {
        if self.status != PhaseStatus::Completed
            && self.status != PhaseStatus::CompletedWithWarnings
        {
            return None;
        }

        Some(match self.phase {
            ImportState::Scanning => format!("{} files found", self.progress_total),
            ImportState::Extracting => format!(
                "{}/{} extracted",
                self.progress_current, self.progress_total
            ),
            ImportState::Segmenting => format!("{} passages detected", self.progress_total),
            ImportState::Fingerprinting => format!(
                "{}/{} fingerprinted",
                self.progress_current, self.progress_total
            ),
            ImportState::Identifying => format!(
                "{}/{} identified",
                self.progress_current, self.progress_total
            ),
            ImportState::Analyzing => {
                format!("{}/{} analyzed", self.progress_current, self.progress_total)
            }
            ImportState::Flavoring => format!(
                "{}/{} characterized",
                self.progress_current, self.progress_total
            ),
            _ => format!(
                "{}/{} processed",
                self.progress_current, self.progress_total
            ),
        })
    }
}

/// **[AIA-SSE-010]** Progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    /// Files processed so far
    pub current: usize,

    /// Total files discovered
    pub total: usize,

    /// Percentage complete (0.0 - 100.0)
    pub percentage: f64,

    /// Current operation description
    pub current_operation: String,

    /// Elapsed time (seconds)
    pub elapsed_seconds: u64,

    /// Estimated remaining time (seconds), None if unknown
    pub estimated_remaining_seconds: Option<u64>,

    /// **[REQ-AIA-UI-001]** Phase-level progress tracking
    pub phases: Vec<PhaseProgress>,

    /// **[REQ-AIA-UI-004]** Current file being processed
    pub current_file: Option<String>,
}

impl ImportSession {
    /// Create new import session
    pub fn new(root_folder: String, parameters: crate::models::ImportParameters) -> Self {
        let mut progress = ImportProgress::default();
        // **[REQ-AIA-UI-001]** Initialize all 6 phases on session creation
        progress.initialize_phases();

        // **[REQ-AIA-UI-001]** Mark first phase (Scanning) as in progress
        if let Some(scanning_phase) = progress.get_phase_mut(ImportState::Scanning) {
            scanning_phase.status = PhaseStatus::InProgress;
        }

        Self {
            session_id: Uuid::new_v4(),
            state: ImportState::Scanning,
            root_folder,
            parameters,
            progress,
            errors: Vec::new(),
            started_at: Utc::now(),
            ended_at: None,
            file_classification: FileClassification::new(),
        }
    }

    /// Transition to new state
    pub fn transition_to(&mut self, new_state: ImportState) -> StateTransition {
        let transition = StateTransition {
            session_id: self.session_id,
            old_state: self.state,
            new_state,
            transitioned_at: Utc::now(),
        };

        // **[REQ-AIA-UI-001]** Update phase status on state transitions
        // Mark old phase as completed (if transitioning from a workflow phase)
        if let Some(old_phase) = self.progress.get_phase_mut(self.state) {
            if old_phase.status == PhaseStatus::InProgress {
                old_phase.status = PhaseStatus::Completed;
            }
        }

        self.state = new_state;

        // Mark new phase as in progress (if it's a workflow phase)
        let total = self.progress.total; // Copy before mutable borrow
        if let Some(new_phase) = self.progress.get_phase_mut(new_state) {
            new_phase.status = PhaseStatus::InProgress;
            // Set total for this phase to match overall total
            new_phase.progress_total = total;
        }

        // Set end time for terminal states
        match new_state {
            ImportState::Completed | ImportState::Cancelled | ImportState::Failed => {
                self.ended_at = Some(Utc::now());
            }
            _ => {}
        }

        transition
    }

    /// Update progress
    pub fn update_progress(&mut self, current: usize, total: usize, operation: String) {
        self.progress.current = current;
        self.progress.total = total;
        self.progress.percentage = if total > 0 {
            (current as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        self.progress.current_operation = operation;

        let elapsed = (Utc::now() - self.started_at).num_seconds() as u64;
        self.progress.elapsed_seconds = elapsed;

        // Estimate remaining time
        if current > 0 && total > current {
            let rate = elapsed as f64 / current as f64;
            let remaining = ((total - current) as f64 * rate) as u64;
            self.progress.estimated_remaining_seconds = Some(remaining);
        } else {
            self.progress.estimated_remaining_seconds = None;
        }

        // **[REQ-AIA-UI-002]** Update current phase progress
        if let Some(phase) = self.progress.get_phase_mut(self.state) {
            phase.progress_current = current;
            phase.progress_total = total;
        }
    }

    /// Add error to session
    pub fn add_error(&mut self, error: crate::models::ImportError) {
        self.errors.push(error);
    }

    /// Check if session is terminal (finished)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            ImportState::Completed | ImportState::Cancelled | ImportState::Failed
        )
    }
}

impl Default for ImportProgress {
    fn default() -> Self {
        Self {
            current: 0,
            total: 0,
            percentage: 0.0,
            current_operation: String::from("Initializing..."),
            elapsed_seconds: 0,
            estimated_remaining_seconds: None,
            phases: Vec::new(),
            current_file: None,
        }
    }
}

impl ImportProgress {
    /// **[REQ-AIA-UI-001]** Initialize phase tracking for PLAN024 workflow
    ///
    /// **Current Architecture:** SCANNING → PROCESSING → COMPLETED
    ///
    /// Processing phase contains 10 internal sub-phases per file:
    /// 1. Filename Matching, 2. Hashing, 3. Metadata Extraction,
    /// 4. Segmentation, 5. Fingerprinting, 6. Song Matching,
    /// 7. Recording, 8. Amplitude Analysis, 9. Flavoring, 10. Finalization
    pub fn initialize_phases(&mut self) {
        self.phases = vec![
            PhaseProgress::new(ImportState::Scanning),
            PhaseProgress::new(ImportState::Processing),
        ];
    }

    /// **[REQ-AIA-UI-001]** Get mutable reference to phase tracker by state
    pub fn get_phase_mut(&mut self, state: ImportState) -> Option<&mut PhaseProgress> {
        self.phases.iter_mut().find(|p| p.phase == state)
    }

    /// **[REQ-AIA-UI-001]** Get phase tracker by state
    pub fn get_phase(&self, state: ImportState) -> Option<&PhaseProgress> {
        self.phases.iter().find(|p| p.phase == state)
    }
}

// ========================================
// Conversion to SSE Event Types
// ========================================

impl From<PhaseStatus> for wkmp_common::events::PhaseStatusData {
    fn from(status: PhaseStatus) -> Self {
        match status {
            PhaseStatus::Pending => wkmp_common::events::PhaseStatusData::Pending,
            PhaseStatus::InProgress => wkmp_common::events::PhaseStatusData::InProgress,
            PhaseStatus::Completed => wkmp_common::events::PhaseStatusData::Completed,
            PhaseStatus::Failed => wkmp_common::events::PhaseStatusData::Failed,
            PhaseStatus::CompletedWithWarnings => {
                wkmp_common::events::PhaseStatusData::CompletedWithWarnings
            }
        }
    }
}

impl From<&SubTaskStatus> for wkmp_common::events::SubTaskData {
    fn from(subtask: &SubTaskStatus) -> Self {
        wkmp_common::events::SubTaskData {
            name: subtask.name.clone(),
            success_count: subtask.success_count,
            failure_count: subtask.failure_count,
            skip_count: subtask.skip_count,
        }
    }
}

impl From<&PhaseProgress> for wkmp_common::events::PhaseProgressData {
    fn from(phase: &PhaseProgress) -> Self {
        wkmp_common::events::PhaseProgressData {
            phase: format!("{:?}", phase.phase).to_uppercase(),
            status: phase.status.into(),
            progress_current: phase.progress_current,
            progress_total: phase.progress_total,
            subtasks: phase.subtasks.iter().map(|s| s.into()).collect(),
            description: phase.phase.description().to_string(),
        }
    }
}

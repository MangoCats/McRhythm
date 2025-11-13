//! Import workflow state machine
//!
//! **[AIA-WF-010]** Import session progresses through states:
//! Legacy: SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING → COMPLETED
//! PLAN024: SCANNING → PROCESSING → COMPLETED

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// **[AIA-WF-010]** Import workflow state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ImportState {
    /// Phase 1A: Directory traversal, finding audio files
    Scanning,
    /// Phase 1B: Hash calculation and basic metadata extraction
    Extracting,
    /// Phase 2A: Detecting silence and passage boundaries per file
    Segmenting,
    /// Phase 2B: Fingerprinting passages (Chromaprint → AcoustID)
    Fingerprinting,
    /// Phase 2C: Identifying music (MusicBrainz metadata resolution)
    Identifying,
    /// Phase 2D: Analyzing amplitude for crossfade timing
    Analyzing,
    /// Phase 2E: Extracting musical characteristics (Essentia/AcousticBrainz)
    Flavoring,
    /// Import finished successfully
    Completed,
    /// Import cancelled by user
    Cancelled,
    /// Import failed with critical error
    Failed,

    /// Legacy: Coarse-grained processing state (deprecated, use specific states)
    #[serde(rename = "PROCESSING")]
    Processing,
}

impl ImportState {
    /// **[REQ-AIA-UI-001]** Get brief description of what this phase does (8 words max)
    pub fn description(&self) -> &'static str {
        match self {
            ImportState::Scanning => "Finding audio files in directories",
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
        if self.status != PhaseStatus::Completed && self.status != PhaseStatus::CompletedWithWarnings {
            return None;
        }

        Some(match self.phase {
            ImportState::Scanning => format!("{} files found", self.progress_total),
            ImportState::Extracting => format!("{}/{} extracted", self.progress_current, self.progress_total),
            ImportState::Segmenting => format!("{} passages detected", self.progress_total),
            ImportState::Fingerprinting => format!("{}/{} fingerprinted", self.progress_current, self.progress_total),
            ImportState::Identifying => format!("{}/{} identified", self.progress_current, self.progress_total),
            ImportState::Analyzing => format!("{}/{} analyzed", self.progress_current, self.progress_total),
            ImportState::Flavoring => format!("{}/{} characterized", self.progress_current, self.progress_total),
            _ => format!("{}/{} processed", self.progress_current, self.progress_total),
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
    pub fn new(
        root_folder: String,
        parameters: crate::models::ImportParameters,
    ) -> Self {
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
    /// **[REQ-AIA-UI-001]** Initialize phase tracking for all 7 workflow phases
    pub fn initialize_phases(&mut self) {
        self.phases = vec![
            PhaseProgress::new(ImportState::Scanning),
            PhaseProgress::new(ImportState::Extracting),
            PhaseProgress::new(ImportState::Segmenting),
            PhaseProgress::new(ImportState::Fingerprinting),
            PhaseProgress::new(ImportState::Identifying),
            PhaseProgress::new(ImportState::Analyzing),
            PhaseProgress::new(ImportState::Flavoring),
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
            PhaseStatus::CompletedWithWarnings => wkmp_common::events::PhaseStatusData::CompletedWithWarnings,
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

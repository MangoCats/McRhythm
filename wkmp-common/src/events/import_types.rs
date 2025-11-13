//! Import workflow type definitions
//!
//! Supporting types for wkmp-ai import workflow progress tracking.

use serde::{Deserialize, Serialize};

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

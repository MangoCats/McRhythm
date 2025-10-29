//! Import workflow state machine
//!
//! **[AIA-WF-010]** Import session progresses through 7 defined states:
//! SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING → COMPLETED

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// **[AIA-WF-010]** Import workflow state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ImportState {
    /// Directory traversal, file discovery
    Scanning,
    /// Metadata extraction, hash calculation
    Extracting,
    /// Chromaprint → AcoustID → MusicBrainz
    Fingerprinting,
    /// Silence detection, passage boundaries
    Segmenting,
    /// Amplitude analysis, lead-in/lead-out
    Analyzing,
    /// AcousticBrainz or Essentia musical flavor
    Flavoring,
    /// Import finished successfully
    Completed,
    /// Import cancelled by user
    Cancelled,
    /// Import failed with critical error
    Failed,
}

/// **[AIA-WF-010]** State transition event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub session_id: Uuid,
    pub old_state: ImportState,
    pub new_state: ImportState,
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
}

impl ImportSession {
    /// Create new import session
    pub fn new(
        root_folder: String,
        parameters: crate::models::ImportParameters,
    ) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            state: ImportState::Scanning,
            root_folder,
            parameters,
            progress: ImportProgress::default(),
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
        self.state = new_state;

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
        }
    }
}

//! Workflow State Machine Tests
//! Test File: workflow_tests.rs
//! Requirements: AIA-WF-010 (State Machine), AIA-WF-020 (Session Management)
//!
//! **PLAN024 Architecture:**
//! - SCANNING → BULK_INSERTING → PROCESSING → COMPLETED

use uuid::Uuid;
use wkmp_ai::models::{ImportParameters, ImportSession, ImportState};

/// Helper function to create test session
fn create_test_session() -> ImportSession {
    ImportSession::new("/test/music".to_string(), ImportParameters::default())
}

/// TC-WF-001: SCANNING → BULK_INSERTING Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_001_scanning_to_bulk_inserting() {
    // Given: Import session in SCANNING state
    let mut session = create_test_session();
    assert_eq!(session.state, ImportState::Scanning);

    // When: Scanner emits completion event (N files found)
    let n_files = 10;
    session.update_progress(n_files, n_files, "Scanning complete".to_string());
    let transition = session.transition_to(ImportState::BulkInserting);

    // Then: Session transitions to BULK_INSERTING
    assert_eq!(session.state, ImportState::BulkInserting);
    assert_eq!(transition.old_state, ImportState::Scanning);
    assert_eq!(transition.new_state, ImportState::BulkInserting);
}

/// TC-WF-002: BULK_INSERTING → PROCESSING Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_002_bulk_inserting_to_processing() {
    // Given: Import session in BULK_INSERTING state
    let mut session = create_test_session();
    session.state = ImportState::BulkInserting;
    let n_files = 10;
    session.update_progress(n_files, n_files, "Bulk insert complete".to_string());

    // When: Bulk insert completes
    let transition = session.transition_to(ImportState::Processing);

    // Then: Session transitions to PROCESSING
    assert_eq!(session.state, ImportState::Processing);
    assert_eq!(transition.old_state, ImportState::BulkInserting);
    assert_eq!(transition.new_state, ImportState::Processing);
}

/// TC-WF-003: PROCESSING → COMPLETED Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_003_processing_to_completed() {
    // Given: Import session in PROCESSING state
    let mut session = create_test_session();
    session.state = ImportState::Processing;
    assert!(session.ended_at.is_none());

    // When: All files processed
    let transition = session.transition_to(ImportState::Completed);

    // Then: Session transitions to COMPLETED
    assert_eq!(session.state, ImportState::Completed);
    assert_eq!(transition.old_state, ImportState::Processing);
    assert_eq!(transition.new_state, ImportState::Completed);

    // End time is set
    assert!(session.ended_at.is_some());
    assert!(session.is_terminal());
}

/// TC-WF-004: Any State → CANCELLED Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_004_any_state_to_cancelled() {
    // Test cancellation from multiple states
    let states = vec![
        ImportState::Scanning,
        ImportState::BulkInserting,
        ImportState::Processing,
    ];

    for state in states {
        // Given: Import session in any active state
        let mut session = create_test_session();
        session.state = state;

        // When: User triggers cancellation
        let transition = session.transition_to(ImportState::Cancelled);

        // Then: Session transitions to CANCELLED
        assert_eq!(session.state, ImportState::Cancelled);
        assert_eq!(transition.old_state, state);
        assert_eq!(transition.new_state, ImportState::Cancelled);
        assert!(session.ended_at.is_some(), "End time should be set");
        assert!(session.is_terminal(), "Cancelled should be terminal");
    }
}

/// TC-WF-005: Error → FAILED Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_005_error_to_failed() {
    // Given: Import session in any state with critical error
    let mut session = create_test_session();
    session.state = ImportState::Processing;

    // When: Component emits critical error event
    let transition = session.transition_to(ImportState::Failed);

    // Then: Session transitions to FAILED
    assert_eq!(session.state, ImportState::Failed);
    assert_eq!(transition.new_state, ImportState::Failed);
    assert!(session.ended_at.is_some(), "End time should be set");
    assert!(session.is_terminal(), "Failed should be terminal");
}

/// TC-WF-006: Session State Persistence (In-Memory)
/// **Requirement:** AIA-WF-020 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_006_session_state_persistence() {
    // Given: New import session created
    let session = create_test_session();

    // Then: Session data persisted in-memory structure
    assert!(
        session.session_id.to_string().len() > 0,
        "UUID should be set"
    );
    assert_eq!(session.state, ImportState::Scanning);
    assert!(session.started_at.timestamp() > 0);
    assert!(session.ended_at.is_none());
    assert_eq!(session.errors.len(), 0);
    assert_eq!(session.progress.current, 0);
    assert_eq!(session.root_folder, "/test/music");
}

/// TC-WF-007: Session UUID Generation
/// **Requirement:** AIA-WF-020 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_007_session_uuid_generation() {
    // Given: Multiple import sessions created sequentially
    let session1 = create_test_session();
    let session2 = create_test_session();
    let session3 = create_test_session();

    // Then: Each session has unique UUID
    assert_ne!(session1.session_id, session2.session_id);
    assert_ne!(session2.session_id, session3.session_id);
    assert_ne!(session1.session_id, session3.session_id);

    // UUIDs are valid RFC 4122 v4
    assert!(Uuid::parse_str(&session1.session_id.to_string()).is_ok());
}

/// TC-WF-008: Progress Tracking
/// **Requirement:** AIA-WF-020 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_008_progress_tracking() {
    // Given: Import session with progress updates
    let mut session = create_test_session();

    // When: Progress updated
    session.update_progress(25, 100, "Processing files".to_string());

    // Then: Progress tracked accurately
    assert_eq!(session.progress.current, 25);
    assert_eq!(session.progress.total, 100);
    assert_eq!(session.progress.percentage, 25.0);
    assert_eq!(session.progress.current_operation, "Processing files");
    assert!(session.progress.elapsed_seconds >= 0);
}

/// TC-WF-009: Terminal States
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_009_terminal_states() {
    // Test that terminal states are correctly identified
    let mut session = create_test_session();

    // Active states are not terminal
    let active_states = vec![
        ImportState::Scanning,
        ImportState::BulkInserting,
        ImportState::Processing,
    ];

    for state in active_states {
        session.state = state;
        assert!(!session.is_terminal(), "{:?} should not be terminal", state);
    }

    // Terminal states
    session.transition_to(ImportState::Completed);
    assert!(session.is_terminal(), "Completed should be terminal");

    session.transition_to(ImportState::Cancelled);
    assert!(session.is_terminal(), "Cancelled should be terminal");

    session.transition_to(ImportState::Failed);
    assert!(session.is_terminal(), "Failed should be terminal");
}

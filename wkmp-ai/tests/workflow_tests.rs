//! Workflow State Machine Tests
//! Test File: workflow_tests.rs
//! Requirements: AIA-WF-010 (State Machine), AIA-WF-020 (Session Management)

use uuid::Uuid;
use wkmp_ai::models::{ImportSession, ImportState, ImportParameters};

/// Helper function to create test session
fn create_test_session() -> ImportSession {
    ImportSession::new(
        "/test/music".to_string(),
        ImportParameters::default(),
    )
}

/// TC-WF-001: SCANNING → EXTRACTING Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_001_scanning_to_extracting() {
    // Given: Import session in SCANNING state
    let mut session = create_test_session();
    assert_eq!(session.state, ImportState::Scanning);

    // When: Scanner emits completion event (N files found)
    let n_files = 10;
    session.update_progress(n_files, n_files, "Scanning complete".to_string());
    let transition = session.transition_to(ImportState::Extracting);

    // Then: Session transitions to EXTRACTING
    assert_eq!(session.state, ImportState::Extracting);
    assert_eq!(transition.old_state, ImportState::Scanning);
    assert_eq!(transition.new_state, ImportState::Extracting);

    // Progress counter ready for extraction phase
    session.update_progress(0, n_files, "Extracting metadata".to_string());
    assert_eq!(session.progress.current, 0);
    assert_eq!(session.progress.total, n_files);
}

/// TC-WF-002: EXTRACTING → FINGERPRINTING Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_002_extracting_to_fingerprinting() {
    // Given: Import session in EXTRACTING state
    let mut session = create_test_session();
    session.state = ImportState::Extracting;
    let n_files = 10;
    session.update_progress(n_files, n_files, "Extraction complete".to_string());

    // When: Extractor emits completion event
    let transition = session.transition_to(ImportState::Fingerprinting);

    // Then: Session transitions to FINGERPRINTING
    assert_eq!(session.state, ImportState::Fingerprinting);
    assert_eq!(transition.old_state, ImportState::Extracting);
    assert_eq!(transition.new_state, ImportState::Fingerprinting);
}

/// TC-WF-003: FINGERPRINTING → SEGMENTING Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_003_fingerprinting_to_segmenting() {
    // Given: Import session in FINGERPRINTING state
    let mut session = create_test_session();
    session.state = ImportState::Fingerprinting;

    // When: Fingerprinter emits completion event
    let transition = session.transition_to(ImportState::Segmenting);

    // Then: Session transitions to SEGMENTING
    assert_eq!(session.state, ImportState::Segmenting);
    assert_eq!(transition.old_state, ImportState::Fingerprinting);
    assert_eq!(transition.new_state, ImportState::Segmenting);
}

/// TC-WF-004: SEGMENTING → ANALYZING Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_004_segmenting_to_analyzing() {
    // Given: Import session in SEGMENTING state
    let mut session = create_test_session();
    session.state = ImportState::Segmenting;

    // When: Segmenter emits completion event
    let transition = session.transition_to(ImportState::Analyzing);

    // Then: Session transitions to ANALYZING
    assert_eq!(session.state, ImportState::Analyzing);
    assert_eq!(transition.old_state, ImportState::Segmenting);
    assert_eq!(transition.new_state, ImportState::Analyzing);
}

/// TC-WF-005: ANALYZING → FLAVORING Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_005_analyzing_to_flavoring() {
    // Given: Import session in ANALYZING state
    let mut session = create_test_session();
    session.state = ImportState::Analyzing;

    // When: Amplitude analyzer emits completion event
    let transition = session.transition_to(ImportState::Flavoring);

    // Then: Session transitions to FLAVORING
    assert_eq!(session.state, ImportState::Flavoring);
    assert_eq!(transition.old_state, ImportState::Analyzing);
    assert_eq!(transition.new_state, ImportState::Flavoring);
}

/// TC-WF-006: FLAVORING → COMPLETED Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_006_flavoring_to_completed() {
    // Given: Import session in FLAVORING state
    let mut session = create_test_session();
    session.state = ImportState::Flavoring;
    assert!(session.ended_at.is_none());

    // When: Essentia runner emits completion event
    let transition = session.transition_to(ImportState::Completed);

    // Then: Session transitions to COMPLETED
    assert_eq!(session.state, ImportState::Completed);
    assert_eq!(transition.old_state, ImportState::Flavoring);
    assert_eq!(transition.new_state, ImportState::Completed);

    // End time is set
    assert!(session.ended_at.is_some());
    assert!(session.is_terminal());
}

/// TC-WF-007: Any State → CANCELLED Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_007_any_state_to_cancelled() {
    // Test cancellation from multiple states
    let states = vec![
        ImportState::Scanning,
        ImportState::Extracting,
        ImportState::Fingerprinting,
        ImportState::Segmenting,
        ImportState::Analyzing,
        ImportState::Flavoring,
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

/// TC-WF-008: Error → FAILED Transition
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_008_error_to_failed() {
    // Given: Import session in any state with critical error
    let mut session = create_test_session();
    session.state = ImportState::Extracting;

    // When: Component emits critical error event
    let transition = session.transition_to(ImportState::Failed);

    // Then: Session transitions to FAILED
    assert_eq!(session.state, ImportState::Failed);
    assert_eq!(transition.new_state, ImportState::Failed);
    assert!(session.ended_at.is_some(), "End time should be set");
    assert!(session.is_terminal(), "Failed should be terminal");
}

/// TC-WF-009: Session State Persistence (In-Memory)
/// **Requirement:** AIA-WF-020 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_009_session_state_persistence() {
    // Given: New import session created
    let session = create_test_session();

    // Then: Session data persisted in-memory structure
    assert!(session.session_id.to_string().len() > 0, "UUID should be set");
    assert_eq!(session.state, ImportState::Scanning);
    assert!(session.started_at.timestamp() > 0);
    assert!(session.ended_at.is_none());
    assert_eq!(session.errors.len(), 0);
    assert_eq!(session.progress.current, 0);
    assert_eq!(session.root_folder, "/test/music");
}

/// TC-WF-010: Session UUID Generation
/// **Requirement:** AIA-WF-020 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_010_session_uuid_generation() {
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

/// TC-WF-011: Progress Tracking
/// **Requirement:** AIA-WF-020 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_011_progress_tracking() {
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

/// TC-WF-012: Terminal States
/// **Requirement:** AIA-WF-010 | **Type:** Unit | **Priority:** P0
#[test]
fn tc_wf_012_terminal_states() {
    // Test that terminal states are correctly identified
    let mut session = create_test_session();

    // Active states are not terminal
    let active_states = vec![
        ImportState::Scanning,
        ImportState::Extracting,
        ImportState::Fingerprinting,
        ImportState::Segmenting,
        ImportState::Analyzing,
        ImportState::Flavoring,
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

//! Test Suite 5: EOF Handling
//!
//! Tests for end-of-file detection and unreachable marker signaling.

use super::helpers::*;
use wkmp_ap::playback::mixer::MarkerEvent;

/// **[REQ-MIX-EOF-001]** Test EOF detection with unreachable markers
#[tokio::test]
async fn test_eof_with_unreachable_markers() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, 0);

    // Add markers at ticks 500, 1000, 1500
    // Buffer only has 800 frames, so ticks 1000 and 1500 are unreachable
    mixer.add_marker(create_position_update_marker(500, passage_id, 500));
    mixer.add_marker(create_position_update_marker(1000, passage_id, 1000));
    mixer.add_marker(create_position_update_marker(1500, passage_id, 1500));

    // Create buffer with only 800 frames (EOF at tick 800)
    let buffer_manager = create_test_buffer_manager(passage_id, 800, 0.5).await;

    // Mix to EOF (request 1000 frames, but only 800 available)
    let mut output = vec![0.0f32; 2000]; // Request 1000 frames
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should get:
    // - PositionUpdate at tick 500 (reachable)
    // - EndOfFile with unreachable markers [1000, 1500]
    assert_eq!(events.len(), 2, "Should have PositionUpdate + EndOfFile");
    assert_eq!(mixer.get_current_tick(), 800, "Tick should be at EOF (800)");

    // First event: PositionUpdate at 500
    match &events[0] {
        MarkerEvent::PositionUpdate { position_ms } => {
            assert_eq!(*position_ms, 500, "First marker at 500 should fire");
        }
        _ => panic!("Expected PositionUpdate, got {:?}", events[0]),
    }

    // Second event: EndOfFile with unreachable markers
    match &events[1] {
        MarkerEvent::EndOfFile { unreachable_markers } => {
            assert_eq!(unreachable_markers.len(), 2, "Two markers unreachable: 1000, 1500");
            assert_eq!(unreachable_markers[0].tick, 1000, "First unreachable at tick 1000");
            assert_eq!(unreachable_markers[1].tick, 1500, "Second unreachable at tick 1500");
        }
        _ => panic!("Expected EndOfFile, got {:?}", events[1]),
    }
}

/// **[REQ-MIX-EOF-002]** Test EOF before planned crossfade point
#[tokio::test]
async fn test_eof_before_crossfade_point() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();
    let next_passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, 0);

    // Add markers:
    // - PositionUpdate at tick 500 (reachable)
    // - StartCrossfade at tick 1000 (unreachable)
    // - PositionUpdate at tick 1200 (unreachable)
    mixer.add_marker(create_position_update_marker(500, passage_id, 500));
    mixer.add_marker(create_crossfade_marker(1000, passage_id, next_passage_id));
    mixer.add_marker(create_position_update_marker(1200, passage_id, 1200));

    // Create buffer with only 800 frames (EOF before crossfade at tick 1000)
    let buffer_manager = create_test_buffer_manager(passage_id, 800, 0.5).await;

    // Mix to EOF
    let mut output = vec![0.0f32; 2000]; // Request 1000 frames
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should get:
    // - PositionUpdate at tick 500
    // - EndOfFileBeforeLeadOut with planned crossfade tick 1000
    assert_eq!(events.len(), 2, "Should have PositionUpdate + EndOfFileBeforeLeadOut");
    assert_eq!(mixer.get_current_tick(), 800, "Tick at EOF");

    // First event: PositionUpdate
    match &events[0] {
        MarkerEvent::PositionUpdate { position_ms } => {
            assert_eq!(*position_ms, 500);
        }
        _ => panic!("Expected PositionUpdate"),
    }

    // Second event: EndOfFileBeforeLeadOut
    match &events[1] {
        MarkerEvent::EndOfFileBeforeLeadOut {
            planned_crossfade_tick,
            unreachable_markers,
        } => {
            assert_eq!(*planned_crossfade_tick, 1000, "Planned crossfade at tick 1000");
            assert_eq!(unreachable_markers.len(), 2, "Crossfade and PositionUpdate unreachable");
            // Verify crossfade marker is in unreachable list
            let has_crossfade = unreachable_markers.iter().any(|m| {
                matches!(m.event_type, MarkerEvent::StartCrossfade { .. })
            });
            assert!(has_crossfade, "Unreachable markers should include StartCrossfade");
        }
        _ => panic!("Expected EndOfFileBeforeLeadOut, got {:?}", events[1]),
    }
}

/// **[REQ-MIX-EOF-003]** Test EOF without markers (automatic queue advancement)
#[tokio::test]
async fn test_eof_without_markers() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, 0);

    // No markers set - passage plays to EOF naturally

    // Create buffer with 500 frames
    let buffer_manager = create_test_buffer_manager(passage_id, 500, 0.5).await;

    // Mix to EOF
    let mut output = vec![0.0f32; 1200]; // Request 600 frames, only 500 available
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should get EndOfFile with empty unreachable list
    assert_eq!(events.len(), 1, "Should have only EndOfFile event");
    assert_eq!(mixer.get_current_tick(), 500, "Tick at EOF");

    match &events[0] {
        MarkerEvent::EndOfFile { unreachable_markers } => {
            assert_eq!(unreachable_markers.len(), 0, "No unreachable markers");
        }
        _ => panic!("Expected EndOfFile, got {:?}", events[0]),
    }
}

/// Test EOF with all markers reachable (no unreachable)
#[tokio::test]
async fn test_eof_all_markers_reachable() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, 0);

    // Add markers at ticks 100, 200, 300 (all before EOF at 500)
    mixer.add_marker(create_position_update_marker(100, passage_id, 100));
    mixer.add_marker(create_position_update_marker(200, passage_id, 200));
    mixer.add_marker(create_position_update_marker(300, passage_id, 300));

    // Create buffer with 500 frames (all markers reachable)
    let buffer_manager = create_test_buffer_manager(passage_id, 500, 0.5).await;

    // Mix to EOF
    let mut output = vec![0.0f32; 1200]; // Request 600 frames
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should get all 3 PositionUpdates + EndOfFile with empty unreachable list
    assert_eq!(events.len(), 4, "3 position updates + EOF");
    assert_eq!(mixer.get_current_tick(), 500, "Tick at EOF");

    // Last event should be EndOfFile with no unreachable markers
    match &events[3] {
        MarkerEvent::EndOfFile { unreachable_markers } => {
            assert_eq!(unreachable_markers.len(), 0, "All markers were reachable");
        }
        _ => panic!("Expected EndOfFile as last event"),
    }
}

/// Test buffer underrun without EOF (decoder still running)
#[tokio::test]
async fn test_underrun_without_eof() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, 0);

    // Add marker at tick 800 (unreachable in this mix)
    mixer.add_marker(create_position_update_marker(800, passage_id, 800));

    // Create buffer with 500 frames, but DON'T mark decode complete
    let buffer_manager = create_test_buffer_manager_without_completion(passage_id, 500, 0.5).await;

    // Mix - will underrun but NOT EOF (decoder still running)
    let mut output = vec![0.0f32; 1200]; // Request 600 frames
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should get NO events (underrun but not EOF)
    assert_eq!(events.len(), 0, "No events - buffer underrun but decode not complete");
    assert_eq!(mixer.get_current_tick(), 500, "Tick advanced by frames read (500)");

    // Marker at 800 should still be in heap (not collected as unreachable)
    // We can verify by continuing to mix when more data arrives
}

/// Test EOF detection only when buffer exhausted (not just underrun)
#[tokio::test]
async fn test_eof_requires_exhaustion() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, 0);

    // Create buffer with 100 frames, mark as complete
    let buffer_manager = create_test_buffer_manager(passage_id, 100, 0.5).await;

    // Mix 50 frames (partial)
    let mut output1 = vec![0.0f32; 100]; // 50 frames
    let events1 = mixer.mix_single(&buffer_manager, passage_id, &mut output1)
        .await
        .expect("Mix should succeed");

    // No EOF yet (buffer not empty)
    assert_eq!(events1.len(), 0, "No EOF - buffer not empty");
    assert_eq!(mixer.get_current_tick(), 50, "Tick at 50");

    // Mix remaining 50 frames (request MORE than available to trigger underrun)
    let mut output2 = vec![0.0f32; 200]; // Request 100 frames, but only 50 available
    let events2 = mixer.mix_single(&buffer_manager, passage_id, &mut output2)
        .await
        .expect("Mix should succeed");

    // NOW EOF (decode complete AND buffer empty)
    assert_eq!(events2.len(), 1, "EOF detected now");
    assert_eq!(mixer.get_current_tick(), 100, "Tick at EOF");

    match &events2[0] {
        MarkerEvent::EndOfFile { .. } => {} // Expected
        _ => panic!("Expected EndOfFile"),
    }
}

/// Test multiple unreachable markers of different types
#[tokio::test]
async fn test_eof_mixed_unreachable_marker_types() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();
    let next_passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, 0);

    // Add mixed marker types, all beyond EOF
    mixer.add_marker(create_position_update_marker(1000, passage_id, 1000));
    mixer.add_marker(create_crossfade_marker(1200, passage_id, next_passage_id));
    mixer.add_marker(create_passage_complete_marker(1500, passage_id));

    // Buffer only has 500 frames (all markers unreachable)
    let buffer_manager = create_test_buffer_manager(passage_id, 500, 0.5).await;

    // Mix to EOF
    let mut output = vec![0.0f32; 1200];
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should get EndOfFileBeforeLeadOut (crossfade marker present)
    assert_eq!(events.len(), 1, "Single EOF event");

    match &events[0] {
        MarkerEvent::EndOfFileBeforeLeadOut {
            planned_crossfade_tick,
            unreachable_markers,
        } => {
            assert_eq!(*planned_crossfade_tick, 1200, "Crossfade planned at 1200");
            assert_eq!(unreachable_markers.len(), 3, "All 3 markers unreachable");

            // Verify markers are sorted by tick
            assert_eq!(unreachable_markers[0].tick, 1000);
            assert_eq!(unreachable_markers[1].tick, 1200);
            assert_eq!(unreachable_markers[2].tick, 1500);

            // Verify types
            assert!(matches!(unreachable_markers[0].event_type, MarkerEvent::PositionUpdate { .. }));
            assert!(matches!(unreachable_markers[1].event_type, MarkerEvent::StartCrossfade { .. }));
            assert!(matches!(unreachable_markers[2].event_type, MarkerEvent::PassageComplete));
        }
        _ => panic!("Expected EndOfFileBeforeLeadOut"),
    }
}

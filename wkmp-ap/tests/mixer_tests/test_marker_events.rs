//! Test Suite 3: Marker Event Emission
//!
//! Tests for event triggering at exact ticks.

use super::helpers::*;
use wkmp_ap::playback::mixer::MarkerEvent;

#[tokio::test]
async fn test_event_emission_exact_tick() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Add marker at tick 1000
    mixer.add_marker(create_position_update_marker(1000, passage_id, 1000));

    let buffer_manager = create_test_buffer_manager(passage_id, 1100, 0.5).await;

    // Mix 1998 samples (999 frames) → tick = 999
    let mut output1 = vec![0.0f32; 1998];
    let events1 = mixer.mix_single(&buffer_manager, passage_id, &mut output1)
        .await
        .expect("Mix should succeed");

    assert_eq!(events1.len(), 0, "No event at tick 999");
    assert_eq!(mixer.get_current_tick(), 999, "At tick 999");

    // Mix 2 more samples (1 frame) → tick = 1000
    let mut output2 = vec![0.0f32; 2];
    let events2 = mixer.mix_single(&buffer_manager, passage_id, &mut output2)
        .await
        .expect("Mix should succeed");

    assert_eq!(events2.len(), 1, "Event should be emitted at exactly tick 1000");
    assert_eq!(mixer.get_current_tick(), 1000, "At tick 1000");

    // Verify it's the correct event
    match &events2[0] {
        MarkerEvent::PositionUpdate { position_ms } => {
            assert_eq!(*position_ms, 1000, "Position should be 1000ms");
        }
        _ => panic!("Expected PositionUpdate event"),
    }
}

#[tokio::test]
async fn test_event_emission_past_tick() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Add marker at tick 100
    mixer.add_marker(create_position_update_marker(100, passage_id, 100));

    let buffer_manager = create_test_buffer_manager(passage_id, 1100, 0.5).await;

    // Mix 2000 samples (1000 frames) - jumps past marker at tick 100
    let mut output = vec![0.0f32; 2000];
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Event should still be emitted even though we jumped past it
    assert_eq!(events.len(), 1, "Event should be emitted even when past tick");
    assert_eq!(mixer.get_current_tick(), 1000, "At tick 1000");
}

#[tokio::test]
async fn test_multiple_events_same_batch() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Add markers at ticks 10, 20, 30
    mixer.add_marker(create_position_update_marker(10, passage_id, 10));
    mixer.add_marker(create_position_update_marker(20, passage_id, 20));
    mixer.add_marker(create_position_update_marker(30, passage_id, 30));

    let buffer_manager = create_test_buffer_manager(passage_id, 200, 0.5).await;

    // Mix 200 samples (100 frames) - covers all 3 markers
    let mut output = vec![0.0f32; 200];
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // All 3 events should be returned in single Vec
    assert_eq!(events.len(), 3, "All 3 events should be emitted in one batch");

    // Verify events are in order
    let positions = extract_position_updates(&events);
    assert_eq!(positions, vec![10, 20, 30], "Events should be in ascending tick order");
}

#[tokio::test]
async fn test_marker_removed_after_emission() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Add marker at tick 500
    mixer.add_marker(create_position_update_marker(500, passage_id, 500));

    let buffer_manager = create_test_buffer_manager(passage_id, 1000, 0.5).await;

    // Mix to tick 600
    let mut output1 = vec![0.0f32; 1200]; // 600 frames
    let events1 = mixer.mix_single(&buffer_manager, passage_id, &mut output1)
        .await
        .expect("Mix should succeed");

    assert_eq!(events1.len(), 1, "Event emitted at tick 500");
    assert_eq!(mixer.get_current_tick(), 600, "At tick 600");

    // Mix to tick 700
    let mut output2 = vec![0.0f32; 200]; // 100 frames
    let events2 = mixer.mix_single(&buffer_manager, passage_id, &mut output2)
        .await
        .expect("Mix should succeed");

    // No duplicate event
    assert_eq!(events2.len(), 0, "No duplicate event - marker was removed");
    assert_eq!(mixer.get_current_tick(), 700, "At tick 700");
}

#[tokio::test]
async fn test_marker_for_different_passage_ignored() {
    let mut mixer = create_test_mixer();
    let passage_a = test_passage_id();
    let passage_b = test_passage_id();

    // Set current passage to A
    mixer.set_current_passage(passage_a, passage_a, 0);

    // Add marker for passage B at tick 100
    mixer.add_marker(create_position_update_marker(100, passage_b, 100));

    let buffer_manager_a = create_test_buffer_manager(passage_a, 300, 0.5).await;

    // Mix passage A to tick 200
    let mut output = vec![0.0f32; 400]; // 200 frames
    let events = mixer.mix_single(&buffer_manager_a, passage_a, &mut output)
        .await
        .expect("Mix should succeed");

    // No events should be emitted (marker is for passage B)
    assert_eq!(events.len(), 0, "No events for passage A - marker is for passage B");
    assert_eq!(mixer.get_current_tick(), 200, "At tick 200");

    // Now switch to passage B
    mixer.set_current_passage(passage_b, passage_b, 0);

    // Note: The marker for passage B at tick 100 was already discarded during passage A mixing
    // (it was popped as a stale marker when passage A reached tick 200)
    // This is correct behavior - markers don't persist across passage switches

    // Add a new marker for passage B (simulating what engine would do)
    mixer.add_marker(create_position_update_marker(100, passage_b, 100));

    let buffer_manager_b = create_test_buffer_manager(passage_b, 300, 0.5).await;

    // Mix passage B to tick 200 - should trigger marker at tick 100
    let mut output_b = vec![0.0f32; 400]; // 200 frames
    let events_b = mixer.mix_single(&buffer_manager_b, passage_b, &mut output_b)
        .await
        .expect("Mix should succeed");

    // Now the marker should fire
    assert_eq!(events_b.len(), 1, "Event should fire for passage B");

    let positions = extract_position_updates(&events_b);
    assert_eq!(positions, vec![100], "Marker at tick 100 should fire");
}

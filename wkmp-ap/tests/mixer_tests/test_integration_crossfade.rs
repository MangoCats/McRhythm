//! Integration Test Suite 2: Crossfade Integration
//!
//! Tests realistic crossfade scenarios between passages.

use super::helpers::*;
use wkmp_ap::playback::mixer::MarkerEvent;

/// Test basic crossfade timing with StartCrossfade marker
#[tokio::test]
async fn test_basic_crossfade_marker_timing() {
    let mut mixer = create_test_mixer();
    let passage1_id = test_passage_id();
    let passage2_id = test_passage_id();

    // Passage 1: 10,000 frames, crossfade starts at tick 8,000
    mixer.set_current_passage(passage1_id, 0);
    mixer.add_marker(create_crossfade_marker(8_000, passage1_id, passage2_id));
    mixer.add_marker(create_passage_complete_marker(10_000, passage1_id));

    let buffer1 = create_test_buffer_manager(passage1_id, 10_000, 0.5).await;

    // Mix passage 1 completely
    let mut output = vec![0.0f32; 20_000]; // 10,000 frames stereo
    let events = mixer.mix_single(&buffer1, passage1_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should have 2 events: StartCrossfade at tick 8000, PassageComplete at tick 10000
    assert_eq!(events.len(), 2, "Should have StartCrossfade + PassageComplete");

    // First event: StartCrossfade
    match &events[0] {
        MarkerEvent::StartCrossfade { next_passage_id } => {
            assert_eq!(*next_passage_id, passage2_id, "Should start crossfade to passage 2");
        }
        _ => panic!("Expected StartCrossfade, got {:?}", events[0]),
    }

    // Second event: PassageComplete
    match &events[1] {
        MarkerEvent::PassageComplete => {} // Expected
        _ => panic!("Expected PassageComplete, got {:?}", events[1]),
    }

    // Verify mixer position
    assert_eq!(mixer.get_current_tick(), 10_000, "Should be at end of passage 1");
}

/// Test crossfade with multiple position updates
#[tokio::test]
async fn test_crossfade_with_position_updates() {
    let mut mixer = create_test_mixer();
    let passage1_id = test_passage_id();
    let passage2_id = test_passage_id();

    // Passage 1: 20,000 frames
    // Position updates every 2,000 frames
    // Crossfade starts at 15,000
    mixer.set_current_passage(passage1_id, 0);

    for i in 0..10_i64 {
        let tick = i * 2_000;
        let position_ms = (i * 2_000 * 1000 / 44_100) as u64; // Convert ticks to ms
        mixer.add_marker(create_position_update_marker(tick, passage1_id, position_ms));
    }
    mixer.add_marker(create_crossfade_marker(15_000, passage1_id, passage2_id));
    mixer.add_marker(create_passage_complete_marker(20_000, passage1_id));

    let buffer1 = create_test_buffer_manager(passage1_id, 20_000, 0.5).await;

    // Mix entire passage in one call
    let mut output = vec![0.0f32; 40_000]; // 20,000 frames stereo
    let events = mixer.mix_single(&buffer1, passage1_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should have 8 PositionUpdates (0, 2000, 4000, 6000, 8000, 10000, 12000, 14000)
    // + StartCrossfade (15000) + PositionUpdate (16000, 18000) + PassageComplete (20000)
    // Total: 10 PositionUpdates + 1 StartCrossfade + 1 PassageComplete = 12 events
    assert_eq!(events.len(), 12, "Should have all position updates + crossfade + complete");

    // Verify StartCrossfade is at correct position (9th event, after 8 position updates)
    match &events[8] {
        MarkerEvent::StartCrossfade { next_passage_id } => {
            assert_eq!(*next_passage_id, passage2_id);
        }
        _ => panic!("Expected StartCrossfade at index 8"),
    }

    // Last event should be PassageComplete
    match &events[11] {
        MarkerEvent::PassageComplete => {} // Expected
        _ => panic!("Expected PassageComplete at end"),
    }
}

/// Test sequential passages with crossfades
#[tokio::test]
async fn test_sequential_passages_with_crossfades() {
    let mut mixer = create_test_mixer();

    // Create 3 passages with crossfades between them
    let passage_ids: Vec<_> = (0..3).map(|_| test_passage_id()).collect();
    let passage_duration = 10_000_usize;

    let mut all_events = Vec::new();

    for i in 0..3 {
        let current_id = passage_ids[i];
        mixer.set_current_passage(current_id, 0);

        // Add position updates at start, middle, and 80% point
        mixer.add_marker(create_position_update_marker(0, current_id, 0));
        mixer.add_marker(create_position_update_marker(5_000, current_id, 113));
        mixer.add_marker(create_position_update_marker(8_000, current_id, 181));

        // Add crossfade marker at 80% (tick 8000) if not last passage
        if i < 2 {
            let next_id = passage_ids[i + 1];
            mixer.add_marker(create_crossfade_marker(8_000, current_id, next_id));
        }

        // Add passage complete at end
        mixer.add_marker(create_passage_complete_marker(passage_duration as i64, current_id));

        // Create buffer and mix
        let buffer = create_test_buffer_manager(current_id, passage_duration, 0.5).await;
        let mut output = vec![0.0f32; passage_duration * 2];
        let events = mixer.mix_single(&buffer, current_id, &mut output)
            .await
            .expect("Mix should succeed");

        all_events.extend(events);

        // Verify tick resets for each passage
        assert_eq!(mixer.get_current_tick(), passage_duration as i64,
                   "Passage {} should end at tick {}", i, passage_duration);
    }

    // Verify total event count:
    // Passage 0: 3 PositionUpdates + 1 StartCrossfade + 1 PassageComplete = 5 events
    // Passage 1: 3 PositionUpdates + 1 StartCrossfade + 1 PassageComplete = 5 events
    // Passage 2: 3 PositionUpdates + 1 PassageComplete = 4 events (no crossfade at end)
    // Total: 14 events
    assert_eq!(all_events.len(), 14, "Should have 14 total events across 3 passages");

    // Verify frames_written accumulates
    assert_eq!(mixer.get_frames_written(), (passage_duration * 3) as u64,
               "frames_written should accumulate across all passages");
}

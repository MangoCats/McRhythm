//! Integration Test Suite 3: State Transitions
//!
//! Tests mixer state changes and edge cases.

use super::helpers::*;
use wkmp_ap::playback::mixer::MarkerEvent;

/// Test switching passages mid-playback
#[tokio::test]
async fn test_switch_passage_mid_playback() {
    let mut mixer = create_test_mixer();
    let passage1_id = test_passage_id();
    let passage2_id = test_passage_id();

    // Set passage 1 with markers
    mixer.set_current_passage(passage1_id, 0);
    mixer.add_marker(create_position_update_marker(5_000, passage1_id, 113));
    mixer.add_marker(create_position_update_marker(10_000, passage1_id, 227));

    let buffer1 = create_test_buffer_manager(passage1_id, 20_000, 0.5).await;

    // Mix first 3,000 frames (before first marker)
    let mut output1 = vec![0.0f32; 6_000]; // 3,000 frames stereo
    let events1 = mixer.mix_single(&buffer1, passage1_id, &mut output1)
        .await
        .expect("Mix should succeed");

    assert_eq!(events1.len(), 0, "No markers reached yet");
    assert_eq!(mixer.get_current_tick(), 3_000, "Tick at 3000");

    // Switch to passage 2 mid-playback
    mixer.set_current_passage(passage2_id, 0);
    mixer.add_marker(create_position_update_marker(2_000, passage2_id, 45));

    let buffer2 = create_test_buffer_manager(passage2_id, 10_000, 0.7).await;

    // Mix passage 2 from start
    let mut output2 = vec![0.0f32; 8_000]; // 4,000 frames stereo
    let events2 = mixer.mix_single(&buffer2, passage2_id, &mut output2)
        .await
        .expect("Mix should succeed");

    // Should get the passage 2 marker (passage 1 markers discarded)
    assert_eq!(events2.len(), 1, "Should have passage 2 marker");
    match &events2[0] {
        MarkerEvent::PositionUpdate { position_ms } => {
            assert_eq!(*position_ms, 45, "Passage 2 marker at 45ms");
        }
        _ => panic!("Expected PositionUpdate"),
    }

    // Verify tick reset to passage 2 position
    assert_eq!(mixer.get_current_tick(), 4_000, "Tick at 4000 (passage 2)");

    // Verify frames_written continues accumulating (3000 + 4000)
    assert_eq!(mixer.get_frames_written(), 7_000, "frames_written should accumulate");
}

/// Test seeking within passage (set_current_passage with non-zero offset)
#[tokio::test]
async fn test_seek_within_passage() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    // Set passage starting at tick 5,000 (seek to middle)
    mixer.set_current_passage(passage_id, 5_000);

    // Add markers only AFTER seek point
    // (markers at or before current_tick would fire immediately on next check)
    mixer.add_marker(create_position_update_marker(6_000, passage_id, 136)); // After seek - SHOULD fire
    mixer.add_marker(create_position_update_marker(8_000, passage_id, 181)); // After seek - SHOULD fire
    mixer.add_marker(create_position_update_marker(12_000, passage_id, 272)); // After end - unreachable

    // Buffer has 15,000 frames total (enough to seek to 5000 and read beyond tick 8000)
    let buffer = create_test_buffer_manager(passage_id, 15_000, 0.5).await;

    // Mix 4,000 frames from seek point (tick 5000 → 9000)
    let mut output = vec![0.0f32; 8_000]; // 4,000 frames stereo
    let events = mixer.mix_single(&buffer, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should only get markers after seek point (6000 and 8000)
    // Marker at 12000 is beyond our mix range (not reached yet)
    assert_eq!(events.len(), 2, "Should have 2 position updates after seek");

    match &events[0] {
        MarkerEvent::PositionUpdate { position_ms } => {
            assert_eq!(*position_ms, 136, "First marker at tick 6000");
        }
        _ => panic!("Expected PositionUpdate"),
    }

    match &events[1] {
        MarkerEvent::PositionUpdate { position_ms } => {
            assert_eq!(*position_ms, 181, "Second marker at tick 8000");
        }
        _ => panic!("Expected PositionUpdate"),
    }

    // Verify final tick (started at 5000, mixed 4000 frames)
    assert_eq!(mixer.get_current_tick(), 9_000, "Tick at 9000 after mixing 4000 frames");
}

/// Test empty buffer (buffer with 0 frames)
#[tokio::test]
async fn test_empty_buffer_handling() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, 0);
    mixer.add_marker(create_position_update_marker(1_000, passage_id, 23));

    // Create buffer with 0 frames (immediately exhausted)
    let buffer = create_test_buffer_manager(passage_id, 0, 0.5).await;

    // Try to mix - should immediately hit EOF
    let mut output = vec![0.0f32; 2_000]; // Request 1,000 frames
    let events = mixer.mix_single(&buffer, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should get EndOfFile with marker at 1000 unreachable
    assert_eq!(events.len(), 1, "Should have EndOfFile event");

    match &events[0] {
        MarkerEvent::EndOfFile { unreachable_markers } => {
            assert_eq!(unreachable_markers.len(), 1, "One unreachable marker");
            assert_eq!(unreachable_markers[0].tick, 1_000, "Marker at tick 1000 unreachable");
        }
        _ => panic!("Expected EndOfFile, got {:?}", events[0]),
    }

    // Verify tick unchanged (no frames mixed)
    assert_eq!(mixer.get_current_tick(), 0, "Tick should still be 0");
}

/// Test passage switch without markers
#[tokio::test]
async fn test_passage_switch_no_markers() {
    let mut mixer = create_test_mixer();
    let passage1_id = test_passage_id();
    let passage2_id = test_passage_id();

    // Passage 1: Play without markers
    mixer.set_current_passage(passage1_id, 0);
    let buffer1 = create_test_buffer_manager(passage1_id, 5_000, 0.5).await;

    let mut output1 = vec![0.0f32; 10_000]; // 5,000 frames stereo
    let events1 = mixer.mix_single(&buffer1, passage1_id, &mut output1)
        .await
        .expect("Mix should succeed");

    // No markers, no events
    assert_eq!(events1.len(), 0, "No markers set for passage 1");
    assert_eq!(mixer.get_current_tick(), 5_000, "Tick at end of passage 1");

    // Switch to passage 2
    mixer.set_current_passage(passage2_id, 0);
    let buffer2 = create_test_buffer_manager(passage2_id, 8_000, 0.6).await;

    let mut output2 = vec![0.0f32; 16_000]; // 8,000 frames stereo
    let events2 = mixer.mix_single(&buffer2, passage2_id, &mut output2)
        .await
        .expect("Mix should succeed");

    // No markers, no events
    assert_eq!(events2.len(), 0, "No markers set for passage 2");
    assert_eq!(mixer.get_current_tick(), 8_000, "Tick at end of passage 2");

    // Verify frames_written accumulates (5000 + 8000)
    assert_eq!(mixer.get_frames_written(), 13_000, "frames_written should accumulate");
}

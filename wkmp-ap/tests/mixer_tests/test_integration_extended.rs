//! Integration Test Suite 1: Extended Playback Scenarios
//!
//! Tests realistic mixing patterns with longer passages and continuous playback.

use super::helpers::*;
use wkmp_ap::playback::mixer::MarkerEvent;

/// Test long passage with frequent position markers (10 seconds, markers every 100ms)
#[tokio::test]
async fn test_long_passage_with_frequent_markers() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // 10 seconds @ 44.1kHz = 441,000 frames
    // Markers every 100ms = every 4,410 frames
    // Total markers: 100 (every 100ms for 10 seconds)
    for i in 0..100_i64 {
        let tick = i * 4410;
        let position_ms = (i * 100) as u64;
        mixer.add_marker(create_position_update_marker(tick, passage_id, position_ms));
    }

    // Create 10-second buffer
    let buffer_manager = create_test_buffer_manager(passage_id, 441_000, 0.5).await;

    // Mix in realistic batches (1024 samples = 512 frames at a time)
    let mut all_events = Vec::new();
    let frames_per_batch = 512;
    let total_batches = (441_000 + frames_per_batch - 1) / frames_per_batch; // Ceiling division

    for _ in 0..total_batches {
        let mut output = vec![0.0f32; frames_per_batch * 2]; // Stereo
        let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
            .await
            .expect("Mix should succeed");
        all_events.extend(events);
    }

    // Should have received all 100 position update events
    let position_updates = extract_position_updates(&all_events);
    assert_eq!(position_updates.len(), 100, "Should have 100 position updates");

    // Verify events in ascending order
    for i in 0..100_usize {
        assert_eq!(position_updates[i], (i * 100) as u64, "Position update {} should be at {}ms", i, i * 100);
    }

    // Verify final tick position
    assert_eq!(mixer.get_current_tick(), 441_000, "Tick should be at end of passage");
    assert_eq!(mixer.get_frames_written(), 441_000, "All frames should be written");
}

/// Test continuous playback of multiple passages sequentially
#[tokio::test]
async fn test_continuous_playback_multiple_passages() {
    let mut mixer = create_test_mixer();

    // Create 5 passages, each 1 second long (44,100 frames)
    let passage_ids: Vec<_> = (0..5).map(|_| test_passage_id()).collect();
    let passage_duration = 44_100;

    let mut total_events = Vec::new();
    let mut expected_frames_written = 0;

    for (i, &passage_id) in passage_ids.iter().enumerate() {
        // Set current passage (resets tick to 0, frames_written continues)
        mixer.set_current_passage(passage_id, passage_id, 0);
        assert_eq!(mixer.get_current_tick(), 0, "Tick should reset for new passage");

        // Add markers at start, middle, and end
        mixer.add_marker(create_position_update_marker(0, passage_id, 0));
        mixer.add_marker(create_position_update_marker(22_050, passage_id, 500));
        mixer.add_marker(create_passage_complete_marker(44_100, passage_id));

        // Create buffer for this passage
        let buffer_manager = create_test_buffer_manager(passage_id, passage_duration, 0.5).await;

        // Mix entire passage
        let mut output = vec![0.0f32; passage_duration * 2];
        let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
            .await
            .expect("Mix should succeed");

        total_events.extend(events);

        // Verify tick at end of passage
        assert_eq!(mixer.get_current_tick(), passage_duration as i64, "Passage {} should end at tick {}", i, passage_duration);

        // Verify frames_written continues accumulating
        expected_frames_written += passage_duration;
        assert_eq!(mixer.get_frames_written(), expected_frames_written as u64,
                   "frames_written should accumulate across passages");
    }

    // Verify total events: 5 passages × 3 markers each = 15 events
    assert_eq!(total_events.len(), 15, "Should have 15 total events from 5 passages");

    // Verify final frames_written
    assert_eq!(mixer.get_frames_written(), 44_100 * 5, "Total frames should be 5 passages");
}

/// Test playback with varying batch sizes (simulate different buffer sizes)
#[tokio::test]
async fn test_playback_with_varying_batch_sizes() {
    let passage_id = test_passage_id();
    let passage_duration = 10_000; // 10,000 frames for testing

    // Add markers at specific ticks to verify accuracy across batch sizes
    let test_ticks = vec![100, 500, 1000, 2500, 5000, 7500, 9999];

    // Test with different batch sizes
    let batch_sizes = vec![64, 128, 512, 1024, 2048];

    for batch_size in batch_sizes {
        let mut mixer = create_test_mixer();
        mixer.set_current_passage(passage_id, passage_id, 0);

        // Add markers
        for &tick in &test_ticks {
            mixer.add_marker(create_position_update_marker(tick, passage_id, tick as u64));
        }

        let buffer_manager = create_test_buffer_manager(passage_id, passage_duration, 0.5).await;

        // Mix in batches of specified size
        let mut all_events = Vec::new();
        let mut current_frame = 0;

        while current_frame < passage_duration {
            let frames_to_mix = (passage_duration - current_frame).min(batch_size);
            let mut output = vec![0.0f32; frames_to_mix * 2];

            let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
                .await
                .expect("Mix should succeed");

            all_events.extend(events);
            current_frame += frames_to_mix;
        }

        // Verify all markers fired regardless of batch size
        let position_updates = extract_position_updates(&all_events);
        assert_eq!(position_updates.len(), test_ticks.len(),
                   "Batch size {} should emit all {} markers", batch_size, test_ticks.len());

        // Verify tick positions match
        for (i, &expected_tick) in test_ticks.iter().enumerate() {
            assert_eq!(position_updates[i], expected_tick as u64,
                       "Batch size {} marker {} should be at tick {}", batch_size, i, expected_tick);
        }

        // Verify final position
        assert_eq!(mixer.get_current_tick(), passage_duration as i64,
                   "Batch size {} should end at correct tick", batch_size);
    }
}

/// Test passage completion detection at exact end
#[tokio::test]
async fn test_passage_completion_detection() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();
    let passage_duration: usize = 44_100; // 1 second

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Add PassageComplete marker at exact end
    mixer.add_marker(create_passage_complete_marker(passage_duration as i64, passage_id));

    // Also add some position markers before end
    mixer.add_marker(create_position_update_marker(10_000, passage_id, 227));
    mixer.add_marker(create_position_update_marker(30_000, passage_id, 680));

    let buffer_manager = create_test_buffer_manager(passage_id, passage_duration, 0.5).await;

    // Mix up to just before end
    let mut output1 = vec![0.0f32; (passage_duration - 100) * 2];
    let events1 = mixer.mix_single(&buffer_manager, passage_id, &mut output1)
        .await
        .expect("Mix should succeed");

    // Should have 2 position updates, no PassageComplete yet
    assert_eq!(events1.len(), 2, "Should have 2 position updates before end");
    assert!(events1.iter().all(|e| matches!(e, MarkerEvent::PositionUpdate { .. })),
            "All events should be PositionUpdate");

    // Mix final 100 frames
    let mut output2 = vec![0.0f32; 200]; // 100 frames
    let events2 = mixer.mix_single(&buffer_manager, passage_id, &mut output2)
        .await
        .expect("Mix should succeed");

    // Should have PassageComplete event
    assert_eq!(events2.len(), 1, "Should have 1 PassageComplete event");
    assert!(matches!(events2[0], MarkerEvent::PassageComplete),
            "Event should be PassageComplete");

    // Verify exact tick
    assert_eq!(mixer.get_current_tick(), passage_duration as i64, "Should be at exact end tick");
}

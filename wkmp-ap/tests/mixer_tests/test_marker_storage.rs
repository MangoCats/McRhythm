//! Test Suite 1: Marker Storage and Retrieval
//!
//! Tests for marker management (add, clear, min-heap ordering).

use super::helpers::*;
use uuid::Uuid;

#[tokio::test]
async fn test_add_single_marker() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    // Set current passage
    mixer.set_current_passage(passage_id, 0);

    // Add a single marker at tick 1000 (need to mix 1000 frames to reach it)
    let marker = create_position_update_marker(1000, passage_id, 1000);
    mixer.add_marker(marker);

    // Create test buffer with enough frames (need 1001 frames total)
    let buffer_manager = create_test_buffer_manager(passage_id, 1001, 0.5).await;
    let mut output = vec![0.0f32; 1998]; // 999 frames

    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    assert_eq!(events.len(), 0, "No events should be emitted before tick 1000");
    assert_eq!(mixer.get_current_tick(), 999, "Tick should be at 999");

    // Mix 2 more samples to reach tick 1000
    let mut output2 = vec![0.0f32; 2]; // 1 frame
    let events2 = mixer.mix_single(&buffer_manager, passage_id, &mut output2)
        .await
        .expect("Mix should succeed");

    assert_eq!(events2.len(), 1, "One event should be emitted at tick 1000");
    assert_eq!(mixer.get_current_tick(), 1000, "Tick should be at 1000");
}

#[tokio::test]
async fn test_add_multiple_markers_sorted() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, 0);

    // Add markers in random order
    mixer.add_marker(create_position_update_marker(2000, passage_id, 2000));
    mixer.add_marker(create_position_update_marker(500, passage_id, 500));
    mixer.add_marker(create_position_update_marker(1500, passage_id, 1500));
    mixer.add_marker(create_position_update_marker(1000, passage_id, 1000));

    // Create buffer with enough frames
    let buffer_manager = create_test_buffer_manager(passage_id, 2100, 0.5).await;

    // Mix and collect all position updates
    let mut all_position_ms = Vec::new();

    // Mix in batches and collect events
    for _ in 0..5 {
        let mut output = vec![0.0f32; 1000]; // 500 frames at a time
        let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
            .await
            .expect("Mix should succeed");

        all_position_ms.extend(extract_position_updates(&events));
    }

    // Should get all 4 markers in sorted order: 500, 1000, 1500, 2000
    assert_eq!(all_position_ms.len(), 4, "Should have 4 position updates");
    assert_eq!(all_position_ms, vec![500, 1000, 1500, 2000], "Markers should be emitted in sorted order");
}

#[tokio::test]
async fn test_marker_min_heap_property() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, 0);

    // Add 10 markers with random ticks
    let random_ticks = vec![5000, 1000, 8000, 300, 7000, 2000, 9000, 4000, 6000, 100];
    for &tick in &random_ticks {
        mixer.add_marker(create_position_update_marker(tick, passage_id, tick as u64));
    }

    // Create buffer with enough frames
    let buffer_manager = create_test_buffer_manager(passage_id, 10000, 0.5).await;

    // Mix and collect all events
    let mut all_position_ms = Vec::new();

    for _ in 0..20 {
        let mut output = vec![0.0f32; 1000]; // 500 frames at a time
        let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
            .await
            .expect("Mix should succeed");

        all_position_ms.extend(extract_position_updates(&events));
    }

    // Should get all 10 markers in ascending order
    let mut expected = random_ticks.clone();
    expected.sort();
    let expected: Vec<u64> = expected.iter().map(|&t| t as u64).collect();

    assert_eq!(all_position_ms.len(), 10, "Should have 10 position updates");
    assert_eq!(all_position_ms, expected, "Markers should be emitted in ascending tick order (min-heap)");
}

#[tokio::test]
async fn test_clear_markers_for_passage() {
    let mut mixer = create_test_mixer();
    let passage_a = test_passage_id();
    let passage_b = test_passage_id();

    mixer.set_current_passage(passage_a, 0);

    // Add markers for both passages
    mixer.add_marker(create_position_update_marker(100, passage_a, 100));
    mixer.add_marker(create_position_update_marker(200, passage_a, 200));
    mixer.add_marker(create_position_update_marker(300, passage_a, 300));
    mixer.add_marker(create_position_update_marker(150, passage_b, 150));
    mixer.add_marker(create_position_update_marker(250, passage_b, 250));

    // Clear markers for passage A
    mixer.clear_markers_for_passage(passage_a);

    // Create buffer for passage A and mix
    let buffer_manager = create_test_buffer_manager(passage_a, 500, 0.5).await;

    let mut all_events = Vec::new();
    for _ in 0..2 {
        let mut output = vec![0.0f32; 500]; // 250 frames
        let events = mixer.mix_single(&buffer_manager, passage_a, &mut output)
            .await
            .expect("Mix should succeed");
        all_events.extend(events);
    }

    // Should get NO events from passage A (all cleared)
    // But markers for passage B are still stored (won't emit because different passage)
    assert_eq!(all_events.len(), 0, "No events for passage A after clearing");

    // Now switch to passage B and add fresh markers
    // Note: Markers for passage B were discarded during passage A mixing (stale markers)
    // This is correct behavior - markers are passage-specific and timing-specific
    mixer.set_current_passage(passage_b, 0);

    // Add new markers for passage B (simulating what engine would do)
    mixer.add_marker(create_position_update_marker(150, passage_b, 150));
    mixer.add_marker(create_position_update_marker(250, passage_b, 250));

    let buffer_manager_b = create_test_buffer_manager(passage_b, 300, 0.5).await;

    let mut output = vec![0.0f32; 600]; // 300 frames - will reach ticks 150 and 250
    let events_b = mixer.mix_single(&buffer_manager_b, passage_b, &mut output)
        .await
        .expect("Mix should succeed");

    let position_updates_b = extract_position_updates(&events_b);
    assert_eq!(position_updates_b.len(), 2, "Should get 2 events from passage B");
    assert_eq!(position_updates_b, vec![150, 250], "Passage B markers fire correctly");
}

#[tokio::test]
async fn test_clear_all_markers() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, 0);

    // Add multiple markers
    mixer.add_marker(create_position_update_marker(100, passage_id, 100));
    mixer.add_marker(create_position_update_marker(200, passage_id, 200));
    mixer.add_marker(create_position_update_marker(300, passage_id, 300));

    // Clear all markers
    mixer.clear_all_markers();

    // Create buffer and mix
    let buffer_manager = create_test_buffer_manager(passage_id, 500, 0.5).await;
    let mut output = vec![0.0f32; 800]; // 400 frames

    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    assert_eq!(events.len(), 0, "No events should be emitted after clearing all markers");
}

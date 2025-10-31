//! Test Suite 4: Event Type Verification
//!
//! Tests for different marker event types and their payloads.

use super::helpers::*;
use wkmp_ap::playback::mixer::{MarkerEvent, PositionMarker};
use uuid::Uuid;

#[tokio::test]
async fn test_position_update_event() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Add PositionUpdate marker
    mixer.add_marker(create_position_update_marker(500, passage_id, 12345));

    let buffer_manager = create_test_buffer_manager(passage_id, 1000, 0.5).await;

    // Mix to trigger marker
    let mut output = vec![0.0f32; 1200]; // 600 frames
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    assert_eq!(events.len(), 1, "Should have 1 event");

    // Verify event type and payload
    match &events[0] {
        MarkerEvent::PositionUpdate { position_ms } => {
            assert_eq!(*position_ms, 12345, "position_ms should match marker payload");
        }
        _ => panic!("Expected PositionUpdate event"),
    }
}

#[tokio::test]
async fn test_start_crossfade_event() {
    let mut mixer = create_test_mixer();
    let current_passage_id = test_passage_id();
    let next_passage_id = test_passage_id();

    mixer.set_current_passage(current_passage_id, current_passage_id, 0);

    // Add StartCrossfade marker
    mixer.add_marker(create_crossfade_marker(300, current_passage_id, next_passage_id));

    let buffer_manager = create_test_buffer_manager(current_passage_id, 1000, 0.5).await;

    // Mix to trigger marker
    let mut output = vec![0.0f32; 800]; // 400 frames
    let events = mixer.mix_single(&buffer_manager, current_passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    assert_eq!(events.len(), 1, "Should have 1 event");

    // Verify event type and payload
    match &events[0] {
        MarkerEvent::StartCrossfade { next_passage_id: next_id } => {
            assert_eq!(*next_id, next_passage_id, "next_passage_id should match marker payload");
        }
        _ => panic!("Expected StartCrossfade event"),
    }
}

#[tokio::test]
async fn test_song_boundary_event() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();
    let new_song_id = test_passage_id(); // Using UUID for song ID

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Add SongBoundary marker
    let marker = PositionMarker {
        tick: 200,
        passage_id,
        event_type: MarkerEvent::SongBoundary {
            new_song_id: Some(new_song_id),
        },
    };
    mixer.add_marker(marker);

    let buffer_manager = create_test_buffer_manager(passage_id, 1000, 0.5).await;

    // Mix to trigger marker
    let mut output = vec![0.0f32; 600]; // 300 frames
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    assert_eq!(events.len(), 1, "Should have 1 event");

    // Verify event type and payload
    match &events[0] {
        MarkerEvent::SongBoundary { new_song_id: song_id } => {
            assert_eq!(*song_id, Some(new_song_id), "new_song_id should match marker payload");
        }
        _ => panic!("Expected SongBoundary event"),
    }
}

#[tokio::test]
async fn test_passage_complete_event() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Add PassageComplete marker
    mixer.add_marker(create_passage_complete_marker(1000, passage_id));

    let buffer_manager = create_test_buffer_manager(passage_id, 1200, 0.5).await;

    // Mix to trigger marker
    let mut output = vec![0.0f32; 2200]; // 1100 frames
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    assert_eq!(events.len(), 1, "Should have 1 event");

    // Verify event type
    match &events[0] {
        MarkerEvent::PassageComplete => {
            // Correct event type
        }
        _ => panic!("Expected PassageComplete event"),
    }
}

#[tokio::test]
async fn test_multiple_event_types_in_sequence() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();
    let next_passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Add different event types at various ticks
    mixer.add_marker(create_position_update_marker(100, passage_id, 100));
    mixer.add_marker(create_crossfade_marker(200, passage_id, next_passage_id));
    mixer.add_marker(create_position_update_marker(300, passage_id, 300));
    mixer.add_marker(create_passage_complete_marker(400, passage_id));

    let buffer_manager = create_test_buffer_manager(passage_id, 600, 0.5).await;

    // Mix to trigger all markers
    let mut output = vec![0.0f32; 1000]; // 500 frames
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should get all 4 events
    assert_eq!(events.len(), 4, "Should have 4 events");

    // Verify types in order
    assert!(matches!(events[0], MarkerEvent::PositionUpdate { .. }), "Event 0 should be PositionUpdate");
    assert!(matches!(events[1], MarkerEvent::StartCrossfade { .. }), "Event 1 should be StartCrossfade");
    assert!(matches!(events[2], MarkerEvent::PositionUpdate { .. }), "Event 2 should be PositionUpdate");
    assert!(matches!(events[3], MarkerEvent::PassageComplete), "Event 3 should be PassageComplete");
}

#[tokio::test]
async fn test_song_boundary_with_none() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Add SongBoundary marker with None (exiting last song)
    let marker = PositionMarker {
        tick: 100,
        passage_id,
        event_type: MarkerEvent::SongBoundary {
            new_song_id: None,
        },
    };
    mixer.add_marker(marker);

    let buffer_manager = create_test_buffer_manager(passage_id, 200, 0.5).await;

    // Mix to trigger marker
    let mut output = vec![0.0f32; 300]; // 150 frames
    let events = mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    assert_eq!(events.len(), 1, "Should have 1 event");

    // Verify event payload is None
    match &events[0] {
        MarkerEvent::SongBoundary { new_song_id } => {
            assert_eq!(*new_song_id, None, "new_song_id should be None");
        }
        _ => panic!("Expected SongBoundary event"),
    }
}

//! Test helpers for mixer unit tests
//!
//! Provides utilities for testing the SPEC016-compliant mixer in isolation.

use std::sync::Arc;
use uuid::Uuid;
use wkmp_ap::audio::types::AudioFrame;
use wkmp_ap::playback::buffer_manager::BufferManager;
use wkmp_ap::playback::mixer::{Mixer, PositionMarker, MarkerEvent};

/// Generate a unique passage ID for testing
pub fn test_passage_id() -> Uuid {
    Uuid::new_v4()
}

/// Create a BufferManager with a pre-populated test buffer
///
/// # Arguments
///
/// * `passage_id` - UUID of passage
/// * `frame_count` - Number of frames to populate
/// * `amplitude` - Amplitude of test audio (0.0 to 1.0)
///
/// # Returns
///
/// Arc<BufferManager> with buffer allocated and populated
pub async fn create_test_buffer_manager(
    passage_id: Uuid,
    frame_count: usize,
    amplitude: f32,
) -> Arc<BufferManager> {
    let buffer_manager = Arc::new(BufferManager::new());

    // Allocate buffer for passage
    let buffer_arc = buffer_manager.allocate_buffer(passage_id).await;

    // Populate with test audio (simple DC signal at specified amplitude)
    for _ in 0..frame_count {
        let frame = AudioFrame {
            left: amplitude,
            right: amplitude,
        };
        let _ = buffer_arc.push_frame(frame);
    }

    // Mark as decode complete
    buffer_arc.mark_decode_complete();

    buffer_manager
}

/// Create a BufferManager with pre-populated buffer WITHOUT marking decode complete
///
/// Used for testing underrun scenarios where decoder is still running.
///
/// # Arguments
///
/// * `passage_id` - UUID of passage
/// * `frame_count` - Number of frames to populate
/// * `amplitude` - Amplitude of test audio (0.0 to 1.0)
///
/// # Returns
///
/// Arc<BufferManager> with buffer allocated and populated (but not complete)
pub async fn create_test_buffer_manager_without_completion(
    passage_id: Uuid,
    frame_count: usize,
    amplitude: f32,
) -> Arc<BufferManager> {
    let buffer_manager = Arc::new(BufferManager::new());

    // Allocate buffer for passage
    let buffer_arc = buffer_manager.allocate_buffer(passage_id).await;

    // Populate with test audio
    for _ in 0..frame_count {
        let frame = AudioFrame {
            left: amplitude,
            right: amplitude,
        };
        let _ = buffer_arc.push_frame(frame);
    }

    // NOTE: Do NOT call mark_decode_complete() - simulates decoder still running

    buffer_manager
}

/// Create mixer with standard test configuration
pub fn create_test_mixer() -> Mixer {
    Mixer::new(1.0) // Full volume
}

/// Create position marker for testing
pub fn create_position_update_marker(tick: i64, passage_id: Uuid, position_ms: u64) -> PositionMarker {
    PositionMarker {
        tick,
        passage_id,
        event_type: MarkerEvent::PositionUpdate { position_ms },
    }
}

/// Create crossfade start marker for testing
pub fn create_crossfade_marker(tick: i64, passage_id: Uuid, next_passage_id: Uuid) -> PositionMarker {
    PositionMarker {
        tick,
        passage_id,
        event_type: MarkerEvent::StartCrossfade { next_passage_id },
    }
}

/// Create passage complete marker for testing
pub fn create_passage_complete_marker(tick: i64, passage_id: Uuid) -> PositionMarker {
    PositionMarker {
        tick,
        passage_id,
        event_type: MarkerEvent::PassageComplete,
    }
}

/// Verify that output buffer contains expected audio
///
/// Checks that output is not all zeros and has expected amplitude range.
pub fn verify_audio_output(output: &[f32], expected_min: f32, expected_max: f32) -> bool {
    let has_non_zero = output.iter().any(|&sample| sample.abs() > 0.0001);
    let in_range = output.iter().all(|&sample| sample >= expected_min && sample <= expected_max);

    has_non_zero && in_range
}

/// Count how many events of a specific type are in the event list
pub fn count_events_of_type(events: &[MarkerEvent], event_type_check: impl Fn(&MarkerEvent) -> bool) -> usize {
    events.iter().filter(|e| event_type_check(e)).count()
}

/// Extract position_ms from PositionUpdate events
pub fn extract_position_updates(events: &[MarkerEvent]) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| {
            if let MarkerEvent::PositionUpdate { position_ms } = e {
                Some(*position_ms)
            } else {
                None
            }
        })
        .collect()
}

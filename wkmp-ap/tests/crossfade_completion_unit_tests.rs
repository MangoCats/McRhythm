//! Unit tests for crossfade completion flag (SPEC018)
//!
//! These tests verify the crossfade completion signaling mechanism
//! as specified in SPEC018-crossfade_completion_coordination.md Section 6.
//!
//! **Traceability:**
//! - [XFD-COMP-010] Crossfade completion detection
//! - [XFD-COMP-020] Queue advancement without mixer restart
//! - [XFD-COMP-030] State consistency during transition

use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use wkmp_ap::audio::types::PassageBuffer;
use wkmp_ap::playback::pipeline::mixer::CrossfadeMixer;
use wkmp_common::FadeCurve;

/// Create a test buffer with sine wave samples
///
/// **[PCF-DUR-010][PCF-COMP-010]** Test buffers are finalized (simulates completed decode)
fn create_test_buffer(passage_id: Uuid, sample_count: usize, amplitude: f32) -> Arc<RwLock<PassageBuffer>> {
    let mut samples = Vec::with_capacity(sample_count * 2);
    for i in 0..sample_count {
        let value = amplitude * (i as f32 * 0.01).sin();
        samples.push(value); // left
        samples.push(value); // right
    }

    let mut buffer = PassageBuffer::new(
        passage_id,
        samples,
        44100,
        2,
    );

    // Finalize buffer (test buffers are complete, like a finished decode)
    buffer.finalize();

    Arc::new(RwLock::new(buffer))
}

#[tokio::test]
async fn test_crossfade_sets_completion_flag() {
    // **[XFD-COMP-010]** Test that crossfade completion sets flag
    let mut mixer = CrossfadeMixer::new();
    let passage1_id = Uuid::new_v4();
    let passage2_id = Uuid::new_v4();

    // Create test buffers (0.5 seconds each @ 44.1kHz)
    let buffer1 = create_test_buffer(passage1_id, 22050, 0.5);
    let buffer2 = create_test_buffer(passage2_id, 22050, 0.5);

    // Start passage 1
    mixer.start_passage(buffer1, passage1_id, None, 0).await;

    // Start crossfade (0.1 seconds = 4410 samples)
    mixer.start_crossfade(
        buffer2,
        passage2_id,
        FadeCurve::Logarithmic,
        4410, // 100ms in samples
        FadeCurve::Logarithmic,
        4410,
    ).await.unwrap();

    // Read frames until crossfade completes (state becomes SinglePassage)
    let mut frames_read = 0;
    while mixer.is_crossfading() {
        mixer.get_next_frame().await;
        frames_read += 1;

        // Safety check to prevent infinite loop
        assert!(frames_read < 10000, "Crossfade did not complete within expected time");
    }

    // Call take_crossfade_completed()
    let completed = mixer.take_crossfade_completed();
    assert_eq!(
        completed,
        Some(passage1_id),
        "Should signal passage 1 (outgoing) completed"
    );

    // Call take_crossfade_completed() again
    let completed_again = mixer.take_crossfade_completed();
    assert_eq!(
        completed_again,
        None,
        "Flag should be cleared after take() - no duplicate signal"
    );

    // Verify mixer is now playing passage 2
    assert_eq!(mixer.get_current_passage_id(), Some(passage2_id));
}

#[tokio::test]
async fn test_stop_clears_completion_flag() {
    // **[XFD-COMP-010]** Test that stop() clears completion flag
    let mut mixer = CrossfadeMixer::new();
    let passage1_id = Uuid::new_v4();
    let passage2_id = Uuid::new_v4();

    let buffer1 = create_test_buffer(passage1_id, 22050, 0.5);
    let buffer2 = create_test_buffer(passage2_id, 22050, 0.5);

    // Start passage 1
    mixer.start_passage(buffer1, passage1_id, None, 0).await;

    // Start crossfade
    mixer.start_crossfade(
        buffer2,
        passage2_id,
        FadeCurve::Linear,
        4410,
        FadeCurve::Linear,
        4410,
    ).await.unwrap();

    // Complete crossfade
    while mixer.is_crossfading() {
        mixer.get_next_frame().await;
    }

    // Flag should be set (verify)
    assert!(mixer.take_crossfade_completed().is_some());

    // Start another crossfade
    let passage3_id = Uuid::new_v4();
    let buffer3 = create_test_buffer(passage3_id, 22050, 0.5);
    mixer.start_crossfade(
        buffer3,
        passage3_id,
        FadeCurve::Linear,
        4410,
        FadeCurve::Linear,
        4410,
    ).await.unwrap();

    // Complete second crossfade
    while mixer.is_crossfading() {
        mixer.get_next_frame().await;
    }

    // Call stop() before checking flag
    mixer.stop();

    // Call take_crossfade_completed()
    let completed = mixer.take_crossfade_completed();
    assert_eq!(
        completed,
        None,
        "Flag should be cleared by stop()"
    );
}

#[tokio::test]
async fn test_crossfade_completion_flag_atomicity() {
    // **[XFD-COMP-010]** Test that flag is consumed atomically (only one consumer gets it)
    let mut mixer = CrossfadeMixer::new();
    let passage1_id = Uuid::new_v4();
    let passage2_id = Uuid::new_v4();

    let buffer1 = create_test_buffer(passage1_id, 22050, 0.5);
    let buffer2 = create_test_buffer(passage2_id, 22050, 0.5);

    // Start crossfade
    mixer.start_passage(buffer1, passage1_id, None, 0).await;
    mixer.start_crossfade(
        buffer2,
        passage2_id,
        FadeCurve::Linear,
        4410,
        FadeCurve::Linear,
        4410,
    ).await.unwrap();

    // Complete crossfade
    while mixer.is_crossfading() {
        mixer.get_next_frame().await;
    }

    // Call take_crossfade_completed() twice
    let result1 = mixer.take_crossfade_completed();
    let result2 = mixer.take_crossfade_completed();

    // Exactly one should return Some(id), other should return None
    assert!(
        (result1.is_some() && result2.is_none()) || (result1.is_none() && result2.is_some()),
        "Exactly one call should return Some(id). Got result1={:?}, result2={:?}",
        result1, result2
    );

    // The one that succeeded should return passage1_id
    let completed_id = result1.or(result2);
    assert_eq!(completed_id, Some(passage1_id));
}

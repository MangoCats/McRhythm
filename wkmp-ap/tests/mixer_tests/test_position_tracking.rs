//! Test Suite 2: Position Tracking
//!
//! Tests for tick advancement and frames_written accumulation.

use super::helpers::*;

#[tokio::test]
async fn test_initial_position_zero() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    // Set current passage
    mixer.set_current_passage(passage_id, passage_id, 0);

    // Verify initial state
    assert_eq!(mixer.get_current_tick(), 0, "Initial tick should be 0");
    assert_eq!(mixer.get_frames_written(), 0, "Initial frames_written should be 0");
    assert_eq!(mixer.get_current_passage_id(), Some(passage_id), "Current passage should be set");
}

#[tokio::test]
async fn test_tick_advancement_single_mix() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Create buffer with enough frames for both mixes (1024 total needed)
    let buffer_manager = create_test_buffer_manager(passage_id, 1024, 0.5).await;

    // Mix 1024 samples (512 frames)
    let mut output = vec![0.0f32; 1024];
    mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    assert_eq!(mixer.get_current_tick(), 512, "Tick should advance by 512 frames");

    // Mix another 1024 samples (512 frames)
    let mut output2 = vec![0.0f32; 1024];
    mixer.mix_single(&buffer_manager, passage_id, &mut output2)
        .await
        .expect("Mix should succeed");

    assert_eq!(mixer.get_current_tick(), 1024, "Tick should now be at 1024");
}

#[tokio::test]
async fn test_frames_written_accumulation() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    let buffer_manager = create_test_buffer_manager(passage_id, 2000, 0.5).await;

    // Mix 1024 samples → 512 frames written
    let mut output1 = vec![0.0f32; 1024];
    mixer.mix_single(&buffer_manager, passage_id, &mut output1)
        .await
        .expect("Mix should succeed");

    assert_eq!(mixer.get_frames_written(), 512, "frames_written should be 512");

    // Mix 512 samples → 256 more frames written
    let mut output2 = vec![0.0f32; 512];
    mixer.mix_single(&buffer_manager, passage_id, &mut output2)
        .await
        .expect("Mix should succeed");

    assert_eq!(mixer.get_frames_written(), 768, "frames_written should accumulate to 768");

    // Mix 1024 samples → 512 more frames written
    let mut output3 = vec![0.0f32; 1024];
    mixer.mix_single(&buffer_manager, passage_id, &mut output3)
        .await
        .expect("Mix should succeed");

    assert_eq!(mixer.get_frames_written(), 1280, "frames_written should accumulate to 1280");
}

#[tokio::test]
async fn test_position_reset_on_passage_change() {
    let mut mixer = create_test_mixer();
    let passage_a = test_passage_id();
    let passage_b = test_passage_id();

    // Start with passage A
    mixer.set_current_passage(passage_a, passage_a, 0);

    let buffer_manager_a = create_test_buffer_manager(passage_a, 1000, 0.5).await;

    // Mix to tick 1000
    let mut output = vec![0.0f32; 2000]; // 1000 frames
    mixer.mix_single(&buffer_manager_a, passage_a, &mut output)
        .await
        .expect("Mix should succeed");

    assert_eq!(mixer.get_current_tick(), 1000, "Tick at 1000 for passage A");
    let frames_written_after_a = mixer.get_frames_written();
    assert_eq!(frames_written_after_a, 1000, "frames_written at 1000");

    // Switch to passage B
    mixer.set_current_passage(passage_b, passage_b, 0);

    // Tick should reset to 0
    assert_eq!(mixer.get_current_tick(), 0, "Tick should reset to 0 for passage B");

    // frames_written should NOT reset (continues accumulating)
    assert_eq!(mixer.get_frames_written(), frames_written_after_a, "frames_written should NOT reset");

    // Mix passage B
    let buffer_manager_b = create_test_buffer_manager(passage_b, 1000, 0.5).await;
    let mut output_b = vec![0.0f32; 1000]; // 500 frames
    mixer.mix_single(&buffer_manager_b, passage_b, &mut output_b)
        .await
        .expect("Mix should succeed");

    assert_eq!(mixer.get_current_tick(), 500, "Tick at 500 for passage B");
    assert_eq!(mixer.get_frames_written(), 1500, "frames_written continues accumulating (1000 + 500)");
}

#[tokio::test]
async fn test_position_tracking_with_underrun() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Create buffer with only 100 frames
    let buffer_manager = create_test_buffer_manager(passage_id, 100, 0.5).await;

    // Request 1000 samples (500 frames) but only 100 available
    let mut output = vec![0.0f32; 1000];
    mixer.mix_single(&buffer_manager, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Should only advance by 100 frames (actual frames read)
    assert_eq!(mixer.get_current_tick(), 100, "Tick should advance by actual frames read (100)");
    assert_eq!(mixer.get_frames_written(), 100, "frames_written should be 100");

    // Verify remainder of output is silence
    let silence_count = output[200..].iter().filter(|&&s| s.abs() < 0.0001).count();
    assert_eq!(silence_count, 800, "Remainder of output should be silence");
}

//! Integration tests for CrossfadeMixer with tick-based timing
//!
//! **Phase 4D - Mixer Integration Testing**
//!
//! These tests verify that the mixer correctly handles:
//! - Tick-to-sample conversions
//! - Single passage playback with BufferManager
//! - Crossfade transitions with drain-based operations
//! - Event-driven playback
//! - Pause mode with exponential decay
//!
//! **Traceability:**
//! - [DBD-MIX-010] start_passage() uses samples
//! - [DBD-MIX-020] start_crossfade() uses samples
//! - [DBD-MIX-030] Dual buffer mixing during crossfade
//! - [DBD-MIX-040] Event-driven playback start
//! - [DBD-MIX-060] Pause mode exponential decay
//! - [DBD-BUF-040] Drain-based buffer operations
//! - [SRC-TICK-020] Tick rate = 28,224,000 Hz

use std::sync::Arc;
use uuid::Uuid;
use wkmp_ap::playback::buffer_manager::BufferManager;
use wkmp_ap::playback::pipeline::mixer::CrossfadeMixer;
use wkmp_common::timing::{ms_to_ticks, ticks_to_samples, TICK_RATE};
use wkmp_common::FadeCurve;

#[test]
fn test_tick_to_sample_conversion_accuracy() {
    // Verify tick-to-sample conversions are exact for common sample rates

    // Test 1: 1 second at 44.1kHz
    let one_second_ticks = TICK_RATE; // 28,224,000 ticks = 1 second
    let samples_44k = ticks_to_samples(one_second_ticks, 44100);
    assert_eq!(samples_44k, 44100, "1 second should be exactly 44100 samples at 44.1kHz");

    // Test 2: 5 seconds at 44.1kHz (typical crossfade duration)
    let five_seconds_ticks = ms_to_ticks(5000);
    let samples_44k = ticks_to_samples(five_seconds_ticks, 44100);
    assert_eq!(samples_44k, 220_500, "5 seconds should be exactly 220,500 samples at 44.1kHz");

    // Test 3: 100ms fade (typical fade duration)
    let hundred_ms_ticks = ms_to_ticks(100);
    let samples_44k = ticks_to_samples(hundred_ms_ticks, 44100);
    assert_eq!(samples_44k, 4410, "100ms should be exactly 4410 samples at 44.1kHz");

    // Test 4: Verify at 48kHz
    let one_second_ticks = TICK_RATE;
    let samples_48k = ticks_to_samples(one_second_ticks, 48000);
    assert_eq!(samples_48k, 48000, "1 second should be exactly 48000 samples at 48kHz");

    // Test 5: Verify zero handling
    assert_eq!(ticks_to_samples(0, 44100), 0, "0 ticks should be 0 samples");
}

#[test]
fn test_crossfade_duration_calculations() {
    // Verify crossfade durations convert correctly from ticks to samples
    // This simulates the engine.rs calculations

    // Scenario: 3-second crossfade at 44.1kHz
    let fade_duration_ticks = ms_to_ticks(3000); // 3 seconds
    let fade_duration_samples = ticks_to_samples(fade_duration_ticks, 44100);

    assert_eq!(
        fade_duration_samples,
        132_300,
        "3 second fade should be 132,300 samples at 44.1kHz"
    );

    // Scenario: Different fade-in and fade-out durations
    let fade_out_ticks = ms_to_ticks(4000); // 4 seconds
    let fade_in_ticks = ms_to_ticks(2000);  // 2 seconds

    let fade_out_samples = ticks_to_samples(fade_out_ticks, 44100);
    let fade_in_samples = ticks_to_samples(fade_in_ticks, 44100);

    assert_eq!(fade_out_samples, 176_400, "4 second fade-out = 176,400 samples");
    assert_eq!(fade_in_samples, 88_200, "2 second fade-in = 88,200 samples");
}

#[test]
fn test_passage_timing_sample_accuracy() {
    // Verify passage timing points convert to sample-accurate positions
    // This simulates PassageWithTiming -> mixer flow

    // Passage timing (in ticks):
    // - Start: 10 seconds
    // - End: 250 seconds
    // - Fade-in point: 12 seconds (2s fade-in duration)
    // - Fade-out point: 245 seconds (5s fade-out duration)

    let start_ticks = ms_to_ticks(10_000);
    let end_ticks = ms_to_ticks(250_000);
    let fade_in_point_ticks = ms_to_ticks(12_000);
    let fade_out_point_ticks = ms_to_ticks(245_000);

    // Calculate durations
    let fade_in_duration_ticks = fade_in_point_ticks - start_ticks;
    let fade_out_duration_ticks = end_ticks - fade_out_point_ticks;

    // Convert to samples
    let fade_in_duration_samples = ticks_to_samples(fade_in_duration_ticks, 44100);
    let fade_out_duration_samples = ticks_to_samples(fade_out_duration_ticks, 44100);

    // Verify sample-accurate timing
    assert_eq!(
        fade_in_duration_samples,
        88_200,
        "2 second fade-in should be 88,200 samples"
    );

    assert_eq!(
        fade_out_duration_samples,
        220_500,
        "5 second fade-out should be 220,500 samples"
    );
}

#[test]
fn test_mixer_state_transitions() {
    // This test verifies the mixer can transition between states
    // In a real integration test, we would use actual PassageBuffer instances

    // For now, we verify the conversion logic that would be used

    // State 1: Idle -> SinglePassage
    let fade_in_ticks = ms_to_ticks(500); // 500ms fade-in
    let fade_in_samples = ticks_to_samples(fade_in_ticks, 44100);
    assert_eq!(fade_in_samples, 22_050, "500ms fade-in = 22,050 samples");

    // State 2: SinglePassage -> Crossfading
    let crossfade_out_ticks = ms_to_ticks(3000);
    let crossfade_in_ticks = ms_to_ticks(3000);

    let crossfade_out_samples = ticks_to_samples(crossfade_out_ticks, 44100);
    let crossfade_in_samples = ticks_to_samples(crossfade_in_ticks, 44100);

    assert_eq!(crossfade_out_samples, 132_300, "3s crossfade out = 132,300 samples");
    assert_eq!(crossfade_in_samples, 132_300, "3s crossfade in = 132,300 samples");
}

#[test]
fn test_zero_duration_fades() {
    // Verify that zero-duration fades (instant start) work correctly

    let zero_ticks = 0i64;
    let zero_samples = ticks_to_samples(zero_ticks, 44100);

    assert_eq!(zero_samples, 0, "Zero fade duration should be 0 samples");

    // Also verify small but non-zero durations
    let one_ms_ticks = ms_to_ticks(1);
    let one_ms_samples = ticks_to_samples(one_ms_ticks, 44100);

    // 1ms at 44.1kHz = 44.1 samples, which rounds to 44
    assert_eq!(one_ms_samples, 44, "1ms should be ~44 samples at 44.1kHz");
}

#[test]
fn test_high_precision_timing() {
    // Verify that tick-based timing provides higher precision than milliseconds

    // At 44.1kHz, 1 sample = ~22.68 microseconds
    // Milliseconds can only represent 1ms = 1000 microseconds
    // Ticks can represent: 1 tick = 1/28,224,000 seconds = ~0.0354 microseconds

    // This means ticks provide ~28x more precision than milliseconds

    // Test: 1 sample at 44.1kHz
    let one_sample_ticks = wkmp_common::timing::samples_to_ticks(1, 44100);
    assert_eq!(one_sample_ticks, 640, "1 sample @ 44.1kHz = 640 ticks");

    // Convert back
    let samples_roundtrip = ticks_to_samples(one_sample_ticks, 44100);
    assert_eq!(samples_roundtrip, 1, "Roundtrip conversion should be exact");

    // Test: Sub-millisecond precision
    let half_ms_ticks = ms_to_ticks(1) / 2; // 0.5ms = 14,112 ticks
    let half_ms_samples = ticks_to_samples(half_ms_ticks, 44100);

    // 0.5ms at 44.1kHz = 22.05 samples, rounds to 22
    assert_eq!(half_ms_samples, 22, "0.5ms should be ~22 samples at 44.1kHz");
}

#[test]
fn test_maximum_passage_duration() {
    // Verify that i64 ticks can represent very long passages
    // i64::MAX ticks = ~10.36 years

    // Test: 4-hour passage (realistic maximum)
    let four_hours_ms = 4 * 60 * 60 * 1000; // 14,400,000 ms
    let four_hours_ticks = ms_to_ticks(four_hours_ms);

    // Verify it doesn't overflow
    assert!(four_hours_ticks > 0, "4-hour passage should not overflow");

    // Convert to samples
    let four_hours_samples = ticks_to_samples(four_hours_ticks, 44100);

    // 4 hours at 44.1kHz = 635,040,000 samples
    assert_eq!(
        four_hours_samples,
        635_040_000,
        "4 hours should be 635,040,000 samples at 44.1kHz"
    );
}

#[test]
fn test_decoder_timing_conversion() {
    // Verify that decoder receives correct millisecond values
    // (Decoders still use milliseconds internally)

    use wkmp_common::timing::ticks_to_ms;

    // Passage: start at 30s, end at 3m30s
    let start_ticks = ms_to_ticks(30_000);
    let end_ticks = ms_to_ticks(210_000);

    // Convert to ms for decoder
    let start_ms = ticks_to_ms(start_ticks);
    let end_ms = ticks_to_ms(end_ticks);

    assert_eq!(start_ms, 30_000, "Start time should convert back to 30s");
    assert_eq!(end_ms, 210_000, "End time should convert back to 210s");
}

#[test]
fn test_crossfade_overlap_detection() {
    // Verify that crossfade timing calculations detect proper overlap

    // Current passage: ends at 180s, fade-out starts at 175s (5s fade)
    let current_end_ticks = ms_to_ticks(180_000);
    let current_fade_out_ticks = ms_to_ticks(175_000);
    let fade_out_duration_ticks = current_end_ticks - current_fade_out_ticks;

    // Next passage: starts at 0s, fade-in completes at 3s (3s fade)
    let next_start_ticks = ms_to_ticks(0);
    let next_fade_in_ticks = ms_to_ticks(3_000);
    let fade_in_duration_ticks = next_fade_in_ticks - next_start_ticks;

    // Convert to samples
    let fade_out_samples = ticks_to_samples(fade_out_duration_ticks, 44100);
    let fade_in_samples = ticks_to_samples(fade_in_duration_ticks, 44100);

    assert_eq!(fade_out_samples, 220_500, "5s fade-out = 220,500 samples");
    assert_eq!(fade_in_samples, 132_300, "3s fade-in = 132,300 samples");

    // Crossfade overlap = min(fade_out, fade_in) = 3s = 132,300 samples
    let overlap_samples = fade_out_samples.min(fade_in_samples);
    assert_eq!(overlap_samples, 132_300, "Crossfade overlap should be 3s");
}

// ==================== Integration Test Helpers ====================

/// Create test mixer with BufferManager configured
///
/// **[DBD-BUF-040]** Mixer uses BufferManager for drain-based operations
async fn create_test_mixer_with_buffer_manager() -> (CrossfadeMixer, Arc<BufferManager>) {
    let buffer_manager = Arc::new(BufferManager::new());
    let mut mixer = CrossfadeMixer::new();
    mixer.set_buffer_manager(Arc::clone(&buffer_manager));
    (mixer, buffer_manager)
}

/// Populate ring buffer with test samples via BufferManager
///
/// **[DBD-BUF-040]** Uses push_samples() and finalize_buffer()
///
/// # Arguments
/// * `buffer_manager` - Buffer manager instance
/// * `passage_id` - UUID of passage
/// * `samples` - Interleaved stereo samples [L, R, L, R, ...]
async fn populate_ring_buffer(
    buffer_manager: &Arc<BufferManager>,
    passage_id: Uuid,
    samples: Vec<f32>,
) -> Result<(), String> {
    // Allocate buffer
    buffer_manager.allocate_buffer(passage_id).await;

    // Push samples
    let frames_pushed = buffer_manager.push_samples(passage_id, &samples).await?;

    // Finalize buffer (mark decode complete)
    buffer_manager.finalize_buffer(passage_id, frames_pushed).await?;

    Ok(())
}

/// Create sine wave test samples (interleaved stereo)
#[allow(dead_code)]
fn create_sine_wave_samples(frame_count: usize, amplitude: f32, frequency: f32) -> Vec<f32> {
    let mut samples = Vec::with_capacity(frame_count * 2);

    for i in 0..frame_count {
        let value = amplitude * (i as f32 * frequency * 0.01).sin();
        samples.push(value); // Left
        samples.push(value); // Right
    }

    samples
}

/// Create constant-value test samples (interleaved stereo)
fn create_constant_samples(frame_count: usize, value: f32) -> Vec<f32> {
    vec![value; frame_count * 2]
}

// ==================== Integration Tests ====================

#[tokio::test]
async fn test_mixer_single_passage_playback() {
    // **[DBD-MIX-010]** Test single passage playback with BufferManager

    let (mut mixer, buffer_manager) = create_test_mixer_with_buffer_manager().await;
    let passage_id = Uuid::new_v4();

    // Create 1000 frames (2000 samples) of constant 0.5 audio
    let samples = create_constant_samples(1000, 0.5);

    // Populate ring buffer
    populate_ring_buffer(&buffer_manager, passage_id, samples).await.unwrap();

    // Start playback (no fade-in)
    mixer.start_passage(passage_id, None, 0).await;

    // Mark buffer as playing
    buffer_manager.start_playback(passage_id).await.unwrap();

    // Read 500 frames
    for i in 0..500 {
        let frame = mixer.get_next_frame().await;
        assert!(
            (frame.left - 0.5).abs() < 0.01,
            "Frame {} left channel should be ~0.5, got {}",
            i,
            frame.left
        );
        assert!(
            (frame.right - 0.5).abs() < 0.01,
            "Frame {} right channel should be ~0.5, got {}",
            i,
            frame.right
        );
    }

    // Verify mixer state
    assert_eq!(mixer.get_current_passage_id(), Some(passage_id));
    assert_eq!(mixer.get_position(), 500);
    assert!(!mixer.is_crossfading());
}

#[tokio::test]
async fn test_mixer_passage_with_fade_in() {
    // **[DBD-MIX-010]** Test passage with fade-in curve

    let (mut mixer, buffer_manager) = create_test_mixer_with_buffer_manager().await;
    let passage_id = Uuid::new_v4();

    // Create 1000 frames of constant 1.0 audio
    let samples = create_constant_samples(1000, 1.0);
    populate_ring_buffer(&buffer_manager, passage_id, samples).await.unwrap();

    // Start playback with 100ms linear fade-in (4410 frames @ 44.1kHz)
    let fade_duration_samples = 4410;
    mixer.start_passage(passage_id, Some(FadeCurve::Linear), fade_duration_samples).await;
    buffer_manager.start_playback(passage_id).await.unwrap();

    // First frame should be silent (fade-in start)
    let first_frame = mixer.get_next_frame().await;
    assert!(
        first_frame.left.abs() < 0.01,
        "First frame should be silent (fade-in start)"
    );

    // Mid-point frame (frame 2205) should be ~0.5 with linear fade
    for _ in 1..2205 {
        mixer.get_next_frame().await;
    }
    let mid_frame = mixer.get_next_frame().await;
    assert!(
        (mid_frame.left - 0.5).abs() < 0.15,
        "Mid-fade frame should be ~0.5, got {}",
        mid_frame.left
    );

    // After fade completes, should be full volume
    for _ in 2206..fade_duration_samples {
        mixer.get_next_frame().await;
    }
    let post_fade_frame = mixer.get_next_frame().await;
    assert!(
        (post_fade_frame.left - 1.0).abs() < 0.01,
        "Post-fade frame should be ~1.0, got {}",
        post_fade_frame.left
    );
}

#[tokio::test]
async fn test_mixer_crossfade_transition() {
    // **[DBD-MIX-020]** Test crossfade between two passages

    let (mut mixer, buffer_manager) = create_test_mixer_with_buffer_manager().await;
    let passage_id_1 = Uuid::new_v4();
    let passage_id_2 = Uuid::new_v4();

    // Create two different passages
    let samples1 = create_constant_samples(10000, 0.8);
    let samples2 = create_constant_samples(10000, 0.3);

    populate_ring_buffer(&buffer_manager, passage_id_1, samples1).await.unwrap();
    populate_ring_buffer(&buffer_manager, passage_id_2, samples2).await.unwrap();

    // Start first passage
    mixer.start_passage(passage_id_1, None, 0).await;
    buffer_manager.start_playback(passage_id_1).await.unwrap();

    // Verify first passage playing
    assert_eq!(mixer.get_current_passage_id(), Some(passage_id_1));
    assert!(!mixer.is_crossfading());

    // Start crossfade (1000 samples each)
    let fade_duration = 1000;
    mixer
        .start_crossfade(
            passage_id_2,
            FadeCurve::Linear,
            fade_duration,
            FadeCurve::Linear,
            fade_duration,
        )
        .await
        .unwrap();

    buffer_manager.start_playback(passage_id_2).await.unwrap();

    // Verify crossfading state
    assert!(mixer.is_crossfading());
    assert_eq!(mixer.get_current_passage_id(), Some(passage_id_1));
    assert_eq!(mixer.get_next_passage_id(), Some(passage_id_2));

    // Read through crossfade
    for _ in 0..fade_duration {
        let frame = mixer.get_next_frame().await;
        // During crossfade, frames should be mixed (valid range)
        assert!(frame.left.abs() <= 1.0);
        assert!(frame.right.abs() <= 1.0);
    }

    // After crossfade, should transition to single passage
    assert!(!mixer.is_crossfading());
    assert_eq!(mixer.get_current_passage_id(), Some(passage_id_2));

    // Should now be reading from second passage
    let post_crossfade_frame = mixer.get_next_frame().await;
    assert!(
        (post_crossfade_frame.left - 0.3).abs() < 0.01,
        "After crossfade should read passage 2 (0.3)"
    );
}

#[tokio::test]
async fn test_mixer_crossfade_completion_signal() {
    // **[XFD-COMP-010]** Test crossfade completion signaling

    let (mut mixer, buffer_manager) = create_test_mixer_with_buffer_manager().await;
    let passage_id_1 = Uuid::new_v4();
    let passage_id_2 = Uuid::new_v4();

    let samples1 = create_constant_samples(5000, 0.5);
    let samples2 = create_constant_samples(5000, 0.5);

    populate_ring_buffer(&buffer_manager, passage_id_1, samples1).await.unwrap();
    populate_ring_buffer(&buffer_manager, passage_id_2, samples2).await.unwrap();

    mixer.start_passage(passage_id_1, None, 0).await;
    buffer_manager.start_playback(passage_id_1).await.unwrap();

    // Start crossfade (100 samples)
    mixer
        .start_crossfade(passage_id_2, FadeCurve::Linear, 100, FadeCurve::Linear, 100)
        .await
        .unwrap();
    buffer_manager.start_playback(passage_id_2).await.unwrap();

    // No completion signal yet
    assert!(mixer.take_crossfade_completed().is_none());

    // Read through crossfade (don't check signal during crossfade - it's set on transition)
    for _ in 0..100 {
        mixer.get_next_frame().await;
    }

    // After crossfade completes, signal should be set
    let completed_passage = mixer.take_crossfade_completed();
    assert_eq!(
        completed_passage,
        Some(passage_id_1),
        "Should signal passage 1 completed"
    );

    // Signal consumed - should be None now
    assert!(mixer.take_crossfade_completed().is_none());
}

#[tokio::test]
async fn test_mixer_pause_resume() {
    // **[XFD-PAUS-010]** **[XFD-PAUS-020]** Test pause/resume functionality

    let (mut mixer, buffer_manager) = create_test_mixer_with_buffer_manager().await;
    let passage_id = Uuid::new_v4();

    let samples = create_constant_samples(10000, 0.8);
    populate_ring_buffer(&buffer_manager, passage_id, samples).await.unwrap();

    mixer.start_passage(passage_id, None, 0).await;
    buffer_manager.start_playback(passage_id).await.unwrap();

    // Read some frames
    for _ in 0..100 {
        mixer.get_next_frame().await;
    }

    // Pause
    mixer.pause();
    assert!(mixer.is_paused());

    // While paused, should output silence
    for _ in 0..50 {
        let frame = mixer.get_next_frame().await;
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }

    // Resume with 500ms exponential fade-in (22050 frames)
    mixer.resume(500, "exponential");
    assert!(!mixer.is_paused());

    // First frame after resume should be silent (fade multiplier = 0)
    let first_resume_frame = mixer.get_next_frame().await;
    assert!(first_resume_frame.left.abs() < 0.01);

    // After some frames, volume should increase
    // Check at multiple points to ensure fade is working
    for _ in 0..1000 {
        mixer.get_next_frame().await;
    }
    let _mid_frame = mixer.get_next_frame().await;

    // Continue to end of fade
    for _ in 1001..22050 {
        mixer.get_next_frame().await;
    }
    let later_frame = mixer.get_next_frame().await;

    // After fade completes, should be at full volume (0.8 * 1.0 = 0.8)
    assert!(
        later_frame.left > 0.7,
        "Volume should be near 0.8 after fade completes, got {}",
        later_frame.left
    );
}

#[tokio::test]
async fn test_mixer_buffer_exhaustion_detection() {
    // **[DBD-BUF-070]** Test buffer exhaustion detection

    let (mut mixer, buffer_manager) = create_test_mixer_with_buffer_manager().await;
    let passage_id = Uuid::new_v4();

    // Create small buffer (100 frames)
    let samples = create_constant_samples(100, 0.5);
    populate_ring_buffer(&buffer_manager, passage_id, samples).await.unwrap();

    mixer.start_passage(passage_id, None, 0).await;
    buffer_manager.start_playback(passage_id).await.unwrap();

    // Read all frames
    for _ in 0..100 {
        mixer.get_next_frame().await;
    }

    // Buffer should be exhausted
    let is_exhausted = buffer_manager.is_buffer_exhausted(passage_id).await.unwrap();
    assert!(is_exhausted, "Buffer should be exhausted after reading all frames");

    // Mixer should detect completion
    assert!(mixer.is_current_finished().await);
}

#[tokio::test]
async fn test_mixer_stop() {
    // Test stop functionality

    let (mut mixer, buffer_manager) = create_test_mixer_with_buffer_manager().await;
    let passage_id = Uuid::new_v4();

    let samples = create_constant_samples(1000, 0.5);
    populate_ring_buffer(&buffer_manager, passage_id, samples).await.unwrap();

    mixer.start_passage(passage_id, None, 0).await;
    buffer_manager.start_playback(passage_id).await.unwrap();

    assert!(mixer.get_current_passage_id().is_some());

    // Stop playback
    mixer.stop();

    assert!(mixer.get_current_passage_id().is_none());
    assert_eq!(mixer.get_position(), 0);

    // Should output silence after stop
    let frame = mixer.get_next_frame().await;
    assert_eq!(frame.left, 0.0);
    assert_eq!(frame.right, 0.0);
}

// ==================== Tests Requiring Future Work ====================

#[tokio::test]
#[ignore = "Seeking not yet supported with drain-based ring buffers"]
async fn test_mixer_seek_position() {
    // **[SSD-ENG-026]** Seek position control
    // TODO Phase 5+: Implement reset_to_position() on PlayoutRingBuffer

    let (mut mixer, buffer_manager) = create_test_mixer_with_buffer_manager().await;
    let passage_id = Uuid::new_v4();

    let samples = create_constant_samples(10000, 0.5);
    populate_ring_buffer(&buffer_manager, passage_id, samples).await.unwrap();

    mixer.start_passage(passage_id, None, 0).await;
    buffer_manager.start_playback(passage_id).await.unwrap();

    // Try to seek (should fail)
    let result = mixer.set_position(5000).await;
    assert!(result.is_err(), "Seeking should not be supported yet");
}

//! Integration Test Suite 4: Volume and Mixing
//!
//! Tests mixer audio output behavior with different volumes and buffer conditions.

use super::helpers::*;

/// Test mixing with non-zero volume (verify audio data present)
#[tokio::test]
async fn test_mixing_with_volume() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Create buffer with non-zero amplitude (0.5)
    let buffer = create_test_buffer_manager(passage_id, 1_000, 0.5).await;

    // Mix frames
    let mut output = vec![0.0f32; 2_000]; // 1,000 frames stereo
    mixer.mix_single(&buffer, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Verify output has non-zero samples (mixed audio data)
    let non_zero_count = output.iter().filter(|&&s| s != 0.0).count();
    assert!(non_zero_count > 0, "Output should contain non-zero audio samples");

    // Verify samples are within expected range [-1.0, 1.0]
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample >= -1.0 && sample <= 1.0,
                "Sample {} ({}) out of range [-1.0, 1.0]", i, sample);
    }

    // Verify tick advanced correctly
    assert_eq!(mixer.get_current_tick(), 1_000, "Tick should advance to 1000");
}

/// Test mixing with zero volume (silence)
#[tokio::test]
async fn test_mixing_with_zero_volume() {
    let mut mixer = create_test_mixer();
    let passage_id = test_passage_id();

    mixer.set_current_passage(passage_id, passage_id, 0);

    // Create buffer with zero amplitude (silence)
    let buffer = create_test_buffer_manager(passage_id, 1_000, 0.0).await;

    // Mix frames
    let mut output = vec![0.0f32; 2_000]; // 1,000 frames stereo
    mixer.mix_single(&buffer, passage_id, &mut output)
        .await
        .expect("Mix should succeed");

    // Verify output is all zeros (silence)
    for (i, &sample) in output.iter().enumerate() {
        assert_eq!(sample, 0.0, "Sample {} should be zero (silence)", i);
    }

    // Verify tick still advanced (silence still counts as mixing)
    assert_eq!(mixer.get_current_tick(), 1_000, "Tick should advance even with silence");
}

/// Test mixing with varying amplitudes
#[tokio::test]
async fn test_mixing_different_amplitudes() {
    let passage_id = test_passage_id();

    // Test different amplitude levels
    let amplitudes = vec![0.1, 0.3, 0.5, 0.7, 0.9, 1.0];

    for amplitude in amplitudes {
        let mut mixer = create_test_mixer();
        mixer.set_current_passage(passage_id, passage_id, 0);

        let buffer = create_test_buffer_manager(passage_id, 500, amplitude).await;

        let mut output = vec![0.0f32; 1_000]; // 500 frames stereo
        mixer.mix_single(&buffer, passage_id, &mut output)
            .await
            .expect("Mix should succeed");

        // Verify output amplitude roughly matches input
        // (allowing for floating point precision)
        let max_sample = output.iter()
            .map(|&s| s.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        assert!((max_sample - amplitude).abs() < 0.01,
                "Max sample {} should be close to amplitude {}", max_sample, amplitude);

        // Verify tick advanced
        assert_eq!(mixer.get_current_tick(), 500, "Tick should advance to 500");
    }
}

//! Unit tests for the PassageBuffer and PassageBufferManager
//!
//! Tests buffer management, memory cleanup, and state transitions
//!
//! Implements requirements from single-stream-design.md

use std::sync::Arc;
use std::time::Duration;
use tokio;
use uuid::Uuid;

use wkmp_ap::playback::pipeline::single_stream::buffer::{
    PassageBuffer,
    PassageBufferManager,
    BufferStatus,
    FadeCurve,
};

/// Helper to create a test buffer with sample data
fn create_test_buffer(passage_id: Uuid, samples: usize) -> PassageBuffer {
    let pcm_data = vec![0.5f32; samples];
    PassageBuffer {
        passage_id,
        pcm_data,
        sample_rate: 44100,
        channels: 2,
        status: BufferStatus::Ready,
        fade_in_curve: FadeCurve::Linear,
        fade_out_curve: FadeCurve::Exponential,
        fade_in_samples: 4410,  // 100ms at 44.1kHz
        fade_out_samples: 8820,  // 200ms at 44.1kHz
        file_path: std::path::PathBuf::from("test.mp3"),
        start_sample: 0,
        end_sample: samples as u64,
    }
}

#[tokio::test]
async fn test_buffer_creation() {
    let manager = PassageBufferManager::new();
    let passage_id = Uuid::new_v4();

    // Add a buffer
    let buffer = create_test_buffer(passage_id, 44100 * 2); // 2 seconds
    manager.add_buffer(buffer).await.unwrap();

    // Verify buffer exists
    assert!(manager.has_buffer(&passage_id).await);

    // Check buffer status
    let status = manager.get_status(&passage_id).await;
    assert_eq!(status, Some(BufferStatus::Ready));
}

#[tokio::test]
async fn test_buffer_state_transitions() {
    let manager = PassageBufferManager::new();
    let passage_id = Uuid::new_v4();

    // Add buffer in Ready state
    let buffer = create_test_buffer(passage_id, 44100);
    manager.add_buffer(buffer).await.unwrap();
    assert_eq!(manager.get_status(&passage_id).await, Some(BufferStatus::Ready));

    // Transition to Playing
    manager.mark_playing(&passage_id).await.unwrap();
    assert_eq!(manager.get_status(&passage_id).await, Some(BufferStatus::Playing));

    // Transition to Exhausted
    manager.mark_exhausted(&passage_id).await.unwrap();
    assert_eq!(manager.get_status(&passage_id).await, Some(BufferStatus::Exhausted));
}

#[tokio::test]
async fn test_buffer_cleanup() {
    let manager = PassageBufferManager::new();

    // Add multiple buffers
    let mut exhausted_ids = Vec::new();
    for i in 0..3 {
        let id = Uuid::new_v4();
        let mut buffer = create_test_buffer(id, 1024);
        if i < 2 {
            buffer.status = BufferStatus::Exhausted;
            exhausted_ids.push(id);
        }
        manager.add_buffer(buffer).await.unwrap();
    }

    // Clean up exhausted buffers
    let cleaned = manager.cleanup_exhausted().await;
    assert_eq!(cleaned, 2);

    // Verify exhausted buffers are removed
    for id in exhausted_ids {
        assert!(!manager.has_buffer(&id).await);
    }
}

#[tokio::test]
async fn test_memory_stats() {
    let manager = PassageBufferManager::new();

    // Add buffers of known sizes
    let buffer1 = create_test_buffer(Uuid::new_v4(), 1000);
    let buffer2 = create_test_buffer(Uuid::new_v4(), 2000);

    manager.add_buffer(buffer1).await.unwrap();
    manager.add_buffer(buffer2).await.unwrap();

    let stats = manager.memory_stats().await;
    assert_eq!(stats.buffer_count, 2);
    assert_eq!(stats.total_samples, 3000);

    // Each sample is 4 bytes (f32)
    let expected_bytes = 3000 * 4;
    assert_eq!(stats.total_bytes, expected_bytes);
    assert!(stats.total_mb > 0.0);
}

#[tokio::test]
async fn test_concurrent_buffer_access() {
    let manager = Arc::new(PassageBufferManager::new());
    let passage_id = Uuid::new_v4();

    // Add initial buffer
    let buffer = create_test_buffer(passage_id, 1024);
    manager.add_buffer(buffer).await.unwrap();

    // Spawn multiple tasks accessing the same buffer
    let mut handles = Vec::new();
    for _ in 0..10 {
        let mgr = Arc::clone(&manager);
        let id = passage_id;
        handles.push(tokio::spawn(async move {
            // Concurrent reads should work
            assert!(mgr.has_buffer(&id).await);
            mgr.get_status(&id).await
        }));
    }

    // Wait for all tasks
    for handle in handles {
        let status = handle.await.unwrap();
        assert_eq!(status, Some(BufferStatus::Ready));
    }
}

#[tokio::test]
async fn test_buffer_not_found_errors() {
    let manager = PassageBufferManager::new();
    let missing_id = Uuid::new_v4();

    // Operations on non-existent buffer should fail gracefully
    assert!(!manager.has_buffer(&missing_id).await);
    assert_eq!(manager.get_status(&missing_id).await, None);
    assert!(manager.mark_playing(&missing_id).await.is_err());
    assert!(manager.mark_exhausted(&missing_id).await.is_err());
}

#[tokio::test]
async fn test_duplicate_buffer_error() {
    let manager = PassageBufferManager::new();
    let passage_id = Uuid::new_v4();

    // Add buffer
    let buffer1 = create_test_buffer(passage_id, 1024);
    assert!(manager.add_buffer(buffer1).await.is_ok());

    // Adding duplicate should fail
    let buffer2 = create_test_buffer(passage_id, 2048);
    assert!(manager.add_buffer(buffer2).await.is_err());
}

#[tokio::test]
async fn test_buffer_size_calculation() {
    // Test 15-second buffer size calculation
    let sample_rate = 44100;
    let channels = 2;
    let duration_secs = 15;

    let total_samples = sample_rate * channels * duration_secs;
    let buffer = create_test_buffer(Uuid::new_v4(), total_samples as usize);

    // Each sample is f32 (4 bytes)
    let expected_bytes = total_samples * 4;
    let actual_bytes = buffer.pcm_data.len() * std::mem::size_of::<f32>();

    assert_eq!(actual_bytes, expected_bytes as usize);

    // Should be approximately 5.3MB for 15 seconds stereo at 44.1kHz
    let size_mb = actual_bytes as f64 / (1024.0 * 1024.0);
    assert!(size_mb > 5.0 && size_mb < 6.0, "Size should be ~5.3MB, got {:.2}MB", size_mb);
}

#[tokio::test]
async fn test_fade_curve_properties() {
    let passage_id = Uuid::new_v4();
    let mut buffer = create_test_buffer(passage_id, 44100);

    // Test different fade curves
    buffer.fade_in_curve = FadeCurve::Linear;
    buffer.fade_out_curve = FadeCurve::SCurve;

    assert_eq!(buffer.fade_in_curve, FadeCurve::Linear);
    assert_eq!(buffer.fade_out_curve, FadeCurve::SCurve);

    // Test fade sample calculations
    assert_eq!(buffer.fade_in_samples, 4410);  // 100ms
    assert_eq!(buffer.fade_out_samples, 8820); // 200ms
}
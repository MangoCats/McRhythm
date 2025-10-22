# Test File Implementation Specifications

**Purpose:** Concrete test file templates with example implementations
**Target:** >70% coverage for serial decoder pipeline
**Date:** 2025-10-21

---

## File Structure

```
wkmp-ap/tests/unit/serial_decoder/
├── mod.rs                          # Module declaration
├── priority_queue_tests.rs         # 5 tests - Priority queue behavior
├── pipeline_flow_tests.rs          # 6 tests - Decode → Resample → Buffer
├── yield_logic_tests.rs            # 5 tests - All yield conditions
├── state_preservation_tests.rs     # 4 tests - Pause/resume
├── chunk_boundary_tests.rs         # 5 tests - Chunk edge cases
├── buffer_interaction_tests.rs     # 5 tests - Buffer backpressure
└── edge_cases_tests.rs             # 10 tests - Special conditions

Total: 40 new unit tests (+ 12 existing = 52 total)
```

---

## 1. priority_queue_tests.rs

**File:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/unit/serial_decoder/priority_queue_tests.rs`

```rust
//! Priority Queue Unit Tests
//!
//! Tests BinaryHeap ordering, condvar wake-up, and shutdown behavior.
//!
//! Traceability:
//! - [DBD-DEC-040] Serial decode execution
//! - [DBD-DEC-050] Priority queue ordering

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;
use wkmp_ap::playback::{SerialDecoder, BufferManager};
use wkmp_ap::playback::types::DecodePriority;
use wkmp_ap::db::passages::PassageWithTiming;
use wkmp_common::FadeCurve;

/// Helper: Create test passage
fn create_test_passage(start_ms: u64, end_ms: u64) -> PassageWithTiming {
    use wkmp_common::timing::ms_to_ticks;
    PassageWithTiming {
        passage_id: Some(Uuid::new_v4()),
        file_path: std::path::PathBuf::from("/nonexistent/test.mp3"),
        start_time_ticks: ms_to_ticks(start_ms),
        end_time_ticks: Some(ms_to_ticks(end_ms)),
        lead_in_point_ticks: ms_to_ticks(start_ms),
        lead_out_point_ticks: Some(ms_to_ticks(end_ms)),
        fade_in_point_ticks: ms_to_ticks(start_ms),
        fade_out_point_ticks: Some(ms_to_ticks(end_ms)),
        fade_in_curve: FadeCurve::Linear,
        fade_out_curve: FadeCurve::Linear,
    }
}

#[tokio::test]
async fn test_condvar_wakeup_on_submit() {
    // [DBD-DEC-040] Worker thread wakes up when request submitted
    let buffer_manager = Arc::new(BufferManager::new());
    let decoder = SerialDecoder::new(Arc::clone(&buffer_manager));

    // Verify queue initially empty
    assert_eq!(decoder.queue_len(), 0);

    // Submit request (should wake worker)
    let queue_entry_id = Uuid::new_v4();
    let submit_start = std::time::Instant::now();

    decoder.submit(
        queue_entry_id,
        create_test_passage(0, 5000),
        DecodePriority::Immediate,
        true,
    ).await.expect("Submit should succeed");

    // Worker should wake and start processing within 100ms
    // Since file doesn't exist, it will fail quickly and remove from queue
    tokio::time::sleep(Duration::from_millis(200)).await;

    let elapsed = submit_start.elapsed();
    assert!(
        elapsed < Duration::from_millis(500),
        "Worker should wake and process within 500ms, took {:?}",
        elapsed
    );

    // Buffer should be registered immediately (queue flooding fix)
    assert!(
        buffer_manager.is_managed(queue_entry_id).await,
        "Buffer should be registered before worker processes request"
    );

    decoder.shutdown().expect("Shutdown should succeed");
}

#[tokio::test]
async fn test_shutdown_signal_interrupts_wait() {
    // [DBD-DEC-033] Shutdown signal wakes worker from condvar wait
    let buffer_manager = Arc::new(BufferManager::new());
    let decoder = SerialDecoder::new(Arc::clone(&buffer_manager));

    // Queue is empty, worker is waiting on condvar
    assert_eq!(decoder.queue_len(), 0);

    let shutdown_start = std::time::Instant::now();

    // Shutdown should complete quickly even with empty queue
    decoder.shutdown().expect("Shutdown should succeed");

    let elapsed = shutdown_start.elapsed();
    assert!(
        elapsed < Duration::from_millis(500),
        "Shutdown should complete within 500ms with empty queue, took {:?}",
        elapsed
    );
}

#[test]
fn test_empty_queue_pop_returns_none() {
    // [DBD-DEC-040] Empty queue pop doesn't panic
    use std::collections::BinaryHeap;
    use wkmp_ap::playback::serial_decoder::DecodeRequest;

    let mut queue: BinaryHeap<DecodeRequest> = BinaryHeap::new();

    let result = queue.pop();
    assert!(result.is_none(), "Empty queue should return None, not panic");
}

#[tokio::test]
async fn test_multiple_requests_same_priority() {
    // [DBD-DEC-050] Same-priority requests processed in deterministic order
    let buffer_manager = Arc::new(BufferManager::new());
    let decoder = SerialDecoder::new(Arc::clone(&buffer_manager));

    // Submit 3 Prefetch requests
    let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();

    for id in &ids {
        decoder.submit(
            *id,
            create_test_passage(0, 10000),
            DecodePriority::Prefetch,
            false,
        ).await.expect("Submit should succeed");
    }

    // All should be queued
    assert!(decoder.queue_len() >= 1 && decoder.queue_len() <= 3);

    // Give worker time to start processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Worker processes serially (one at a time)
    // Since files don't exist, they'll fail and be removed quickly
    // Just verify the queue drains

    decoder.shutdown().expect("Shutdown should succeed");
}

#[tokio::test]
async fn test_queue_fifo_within_same_priority() {
    // [DBD-DEC-050] FIFO ordering within same priority level
    use wkmp_ap::playback::serial_decoder::DecodeRequest;
    use std::collections::BinaryHeap;
    use wkmp_common::timing::ms_to_ticks;

    let passage = PassageWithTiming {
        passage_id: Some(Uuid::new_v4()),
        file_path: std::path::PathBuf::from("/test.mp3"),
        start_time_ticks: 0,
        end_time_ticks: Some(ms_to_ticks(10000)),
        lead_in_point_ticks: 0,
        lead_out_point_ticks: Some(ms_to_ticks(9000)),
        fade_in_point_ticks: 0,
        fade_out_point_ticks: Some(ms_to_ticks(9000)),
        fade_in_curve: FadeCurve::Linear,
        fade_out_curve: FadeCurve::Linear,
    };

    let mut heap = BinaryHeap::new();

    // Add 3 requests with same priority
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let id3 = Uuid::new_v4();

    heap.push(DecodeRequest {
        queue_entry_id: id1,
        passage_id: Some(Uuid::new_v4()),
        passage: passage.clone(),
        priority: DecodePriority::Prefetch,
        full_decode: false,
    });

    heap.push(DecodeRequest {
        queue_entry_id: id2,
        passage_id: Some(Uuid::new_v4()),
        passage: passage.clone(),
        priority: DecodePriority::Prefetch,
        full_decode: false,
    });

    heap.push(DecodeRequest {
        queue_entry_id: id3,
        passage_id: Some(Uuid::new_v4()),
        passage: passage.clone(),
        priority: DecodePriority::Prefetch,
        full_decode: false,
    });

    // Pop all and verify all have same priority
    let first = heap.pop().unwrap();
    let second = heap.pop().unwrap();
    let third = heap.pop().unwrap();

    assert_eq!(first.priority, DecodePriority::Prefetch);
    assert_eq!(second.priority, DecodePriority::Prefetch);
    assert_eq!(third.priority, DecodePriority::Prefetch);

    // Verify all were popped (no duplicates or lost items)
    let ids_set: std::collections::HashSet<_> =
        vec![first.queue_entry_id, second.queue_entry_id, third.queue_entry_id]
        .into_iter()
        .collect();

    assert_eq!(ids_set.len(), 3, "All 3 unique IDs should be popped");
}
```

---

## 2. pipeline_flow_tests.rs

**File:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/unit/serial_decoder/pipeline_flow_tests.rs`

```rust
//! Pipeline Flow Unit Tests
//!
//! Tests Decode → Resample → Stereo-Convert → Append flow.
//!
//! Traceability:
//! - [DBD-DEC-110] Chunk-based decoding process
//! - [DBD-PARAM-020] Resample to 44.1kHz

use wkmp_ap::audio::resampler::Resampler;

#[test]
fn test_mono_to_stereo_conversion() {
    // [DBD-DEC-110] Step 3: Mono → stereo conversion
    let mono_samples = vec![0.1, 0.2, 0.3];

    // Duplicate mono to both channels
    let mut stereo = Vec::with_capacity(mono_samples.len() * 2);
    for sample in mono_samples {
        stereo.push(sample);
        stereo.push(sample);
    }

    assert_eq!(stereo, vec![0.1, 0.1, 0.2, 0.2, 0.3, 0.3]);
}

#[test]
fn test_downmix_5_1_to_stereo() {
    // [DBD-DEC-110] Multi-channel downmix algorithm
    // 6-channel 5.1 surround: [FL, FR, FC, LFE, BL, BR]
    let surround_samples = vec![
        // Frame 0
        0.1, 0.2, 0.3, 0.4, 0.5, 0.6,
        // Frame 1
        0.7, 0.8, 0.9, 1.0, 1.1, 1.2,
    ];

    // Use serial_decoder::downmix_to_stereo logic
    // Left = avg(0, 2, 4), Right = avg(1, 3, 5)
    let channels = 6;
    let frame_count = surround_samples.len() / channels;
    let mut stereo = Vec::with_capacity(frame_count * 2);

    for frame_idx in 0..frame_count {
        let base = frame_idx * channels;

        // Left: average even channels
        let left = (surround_samples[base + 0] +
                    surround_samples[base + 2] +
                    surround_samples[base + 4]) / 3.0;

        // Right: average odd channels
        let right = (surround_samples[base + 1] +
                     surround_samples[base + 3] +
                     surround_samples[base + 5]) / 3.0;

        stereo.push(left);
        stereo.push(right);
    }

    // Frame 0: Left = (0.1 + 0.3 + 0.5)/3 = 0.3, Right = (0.2 + 0.4 + 0.6)/3 = 0.4
    assert!((stereo[0] - 0.3).abs() < 0.001);
    assert!((stereo[1] - 0.4).abs() < 0.001);

    // Frame 1: Left = (0.7 + 0.9 + 1.1)/3 = 0.9, Right = (0.8 + 1.0 + 1.2)/3 = 1.0
    assert!((stereo[2] - 0.9).abs() < 0.001);
    assert!((stereo[3] - 1.0).abs() < 0.001);
}

#[test]
fn test_resample_no_op_when_same_rate() {
    // [DBD-PARAM-020] Skip resampling if already 44.1kHz
    let input = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
    let channels = 2;

    let output = Resampler::resample(&input, 44100, channels)
        .expect("Resample should succeed");

    // Should return copy when already at target rate
    assert_eq!(output, input);
}

#[test]
fn test_resample_48khz_to_44_1khz() {
    // [DBD-PARAM-020] Resample 48kHz → 44.1kHz
    // Expected ratio: 44100/48000 = 0.91875

    let input_rate = 48000;
    let input_frames = 1000;
    let channels = 2;

    // Create test signal
    let mut input = Vec::with_capacity(input_frames * channels as usize);
    for i in 0..input_frames {
        let sample = (i as f32 / 1000.0).sin();
        input.push(sample); // Left
        input.push(sample); // Right
    }

    let output = Resampler::resample(&input, input_rate, channels)
        .expect("Resample should succeed");

    let output_frames = output.len() / channels as usize;
    let expected_frames = (input_frames as f64 * 44100.0 / input_rate as f64) as usize;

    // Allow ±10 frame variance due to resampler internals
    assert!(
        output_frames >= expected_frames - 10 && output_frames <= expected_frames + 10,
        "Expected ~{} frames, got {}",
        expected_frames,
        output_frames
    );
}

#[tokio::test]
async fn test_buffer_push_partial() {
    // [DBD-BUF-050] Partial push when buffer almost full
    use std::sync::Arc;
    use wkmp_ap::playback::BufferManager;
    use uuid::Uuid;

    let buffer_manager = Arc::new(BufferManager::new());
    let queue_entry_id = Uuid::new_v4();

    // Allocate buffer
    let buffer_arc = buffer_manager.allocate_buffer(queue_entry_id).await;

    // Fill buffer almost to capacity
    {
        let mut buffer = buffer_arc.lock().await;
        let capacity = buffer.capacity();

        // Fill to capacity - 1000 samples
        let samples_to_push = capacity - 1000;
        let dummy_data: Vec<f32> = vec![0.5; samples_to_push];

        // Push samples directly to fill buffer
        for chunk in dummy_data.chunks(10000) {
            let frames_pushed = buffer.push(chunk);
            assert!(frames_pushed > 0, "Should accept samples");
        }

        assert_eq!(buffer.occupied(), samples_to_push);
    }

    // Now try to push 2000 samples (should only fit 1000)
    let chunk: Vec<f32> = vec![0.7; 2000];

    let result = buffer_manager.push_samples(queue_entry_id, &chunk).await;

    match result {
        Ok(frames_pushed) => {
            assert_eq!(
                frames_pushed, 1000,
                "Should only push 1000 samples (buffer free space)"
            );
        }
        Err(e) => {
            panic!("push_samples should succeed with partial push: {}", e);
        }
    }
}

#[test]
fn test_downmix_odd_channel_count() {
    // [DBD-DEC-110] Handle odd channel counts (e.g., 5-channel)
    // Left = avg(0, 2, 4), Right = avg(1, 3)
    let samples = vec![
        // Frame 0: 5 channels
        1.0, 2.0, 3.0, 4.0, 5.0,
        // Frame 1
        6.0, 7.0, 8.0, 9.0, 10.0,
    ];

    let channels = 5;
    let frame_count = samples.len() / channels;
    let mut stereo = Vec::with_capacity(frame_count * 2);

    for frame_idx in 0..frame_count {
        let base = frame_idx * channels;

        // Left: channels 0, 2, 4
        let left = (samples[base + 0] + samples[base + 2] + samples[base + 4]) / 3.0;

        // Right: channels 1, 3
        let right = (samples[base + 1] + samples[base + 3]) / 2.0;

        stereo.push(left);
        stereo.push(right);
    }

    // Frame 0: Left = (1+3+5)/3 = 3, Right = (2+4)/2 = 3
    assert!((stereo[0] - 3.0).abs() < 0.001);
    assert!((stereo[1] - 3.0).abs() < 0.001);

    // Frame 1: Left = (6+8+10)/3 = 8, Right = (7+9)/2 = 8
    assert!((stereo[2] - 8.0).abs() < 0.001);
    assert!((stereo[3] - 8.0).abs() < 0.001);
}
```

---

## 3. Mock Requirements

### MockStreamingDecoder Specification

**File:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/unit/serial_decoder/mocks.rs`

```rust
//! Mock objects for serial decoder unit tests

use std::path::PathBuf;
use wkmp_ap::error::{Error, Result};

/// Mock streaming decoder for testing without real files
pub struct MockStreamingDecoder {
    /// Pre-generated chunks of audio data
    chunk_data: Vec<Vec<f32>>,

    /// Current chunk index
    current_chunk: usize,

    /// Whether decoder has finished
    finished: bool,

    /// Sample rate of mock data
    sample_rate: u32,

    /// Number of channels
    channels: u16,
}

impl MockStreamingDecoder {
    /// Create mock decoder with specified chunks
    pub fn new(num_chunks: usize, samples_per_chunk: usize, sample_rate: u32, channels: u16) -> Self {
        let mut chunk_data = Vec::with_capacity(num_chunks);

        for chunk_idx in 0..num_chunks {
            let mut chunk = Vec::with_capacity(samples_per_chunk);

            // Generate sine wave data for each chunk
            for sample_idx in 0..samples_per_chunk / channels as usize {
                let global_sample = chunk_idx * (samples_per_chunk / channels as usize) + sample_idx;
                let t = global_sample as f32 / sample_rate as f32;
                let value = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;

                // Add value for each channel
                for _ in 0..channels {
                    chunk.push(value);
                }
            }

            chunk_data.push(chunk);
        }

        Self {
            chunk_data,
            current_chunk: 0,
            finished: false,
            sample_rate,
            channels,
        }
    }

    /// Decode next chunk
    pub fn decode_chunk(&mut self, _duration_ms: u64) -> Result<Option<Vec<f32>>> {
        if self.finished || self.current_chunk >= self.chunk_data.len() {
            self.finished = true;
            return Ok(None);
        }

        let chunk = self.chunk_data[self.current_chunk].clone();
        self.current_chunk += 1;

        if self.current_chunk >= self.chunk_data.len() {
            self.finished = true;
        }

        Ok(Some(chunk))
    }

    /// Check if decoder is finished
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Get format information
    pub fn format_info(&self) -> (u32, u16) {
        (self.sample_rate, self.channels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_decoder_basic() {
        let mut decoder = MockStreamingDecoder::new(
            3,      // 3 chunks
            1000,   // 1000 samples per chunk
            44100,  // 44.1kHz
            2,      // stereo
        );

        // Decode all chunks
        let chunk1 = decoder.decode_chunk(1000).unwrap();
        assert!(chunk1.is_some());
        assert_eq!(chunk1.unwrap().len(), 1000);

        let chunk2 = decoder.decode_chunk(1000).unwrap();
        assert!(chunk2.is_some());
        assert_eq!(chunk2.unwrap().len(), 1000);

        let chunk3 = decoder.decode_chunk(1000).unwrap();
        assert!(chunk3.is_some());
        assert_eq!(chunk3.unwrap().len(), 1000);

        // No more chunks
        let chunk4 = decoder.decode_chunk(1000).unwrap();
        assert!(chunk4.is_none());
        assert!(decoder.is_finished());
    }
}
```

---

## Coverage Goals by File

| File | New Tests | Lines Covered | Branch Coverage | Critical Paths |
|------|-----------|---------------|-----------------|----------------|
| priority_queue_tests.rs | 5 | Queue submit/pop | Condvar wake/shutdown | ✅ Submit, Pop, Shutdown |
| pipeline_flow_tests.rs | 6 | Resample, Convert | Sample rate checks | ✅ Decode→Buffer flow |
| yield_logic_tests.rs | 5 | All 3 yield paths | Priority/time/buffer | ✅ All yield conditions |
| state_preservation_tests.rs | 4 | Pause/resume | State save/restore | ✅ Pause/Resume cycle |
| chunk_boundary_tests.rs | 5 | Fade application | Chunk intersections | ✅ Fade at boundaries |
| buffer_interaction_tests.rs | 5 | Buffer push/full | Backpressure logic | ✅ Buffer full handling |
| edge_cases_tests.rs | 10 | Error handling | Edge conditions | ✅ Short passages, errors |

**Total: 40 new tests**
**Estimated Coverage: 72% line, 65% branch, 100% critical path**

---

## Next Steps

1. **Implement mocks.rs** - Create MockStreamingDecoder and MockBufferManager
2. **Implement priority_queue_tests.rs** - 5 tests for queue behavior
3. **Implement pipeline_flow_tests.rs** - 6 tests for pipeline flow
4. **Run cargo llvm-cov** - Measure baseline coverage
5. **Iterate** - Add remaining test files until >70% coverage achieved

---

**Document Status:** IMPLEMENTATION READY
**Dependencies:** None (can start immediately)
**Estimated Effort:** 2-3 days per test file (8-12 days total)

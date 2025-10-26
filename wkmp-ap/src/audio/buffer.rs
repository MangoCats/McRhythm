//! Ring buffer for PCM sample storage
//!
//! Implements lock-free ring buffer for audio sample buffering per SPEC016.
//!
//! # Sample Format
//!
//! Per SPEC016 DBD-FMT-010:
//! - Stereo f32 samples (interleaved: [L, R, L, R, ...])
//! - Working sample rate: 44,100 Hz
//! - One "sample" = one stereo frame (L+R pair)
//!
//! # Architecture
//!
//! Phase 3: Basic ring buffer with push/pop operations
//! Phase 4: Add backpressure and hysteresis per SPEC016 DBD-BUF-050/060

use crate::{AudioPlayerError, Result};
use ringbuf::{
    traits::{Consumer, Observer, Producer, Split},
    HeapRb,
};
use std::sync::{Arc, Mutex};

/// Ring buffer for PCM audio samples
///
/// Stores interleaved stereo f32 samples for playback.
/// Lock-free implementation using ringbuf crate.
///
/// # Examples
///
/// ```ignore
/// let buffer = RingBuffer::new(88200); // 2 seconds at 44.1kHz
/// buffer.push(&samples)?;
/// let samples = buffer.pop(4410)?; // Read 0.1 seconds
/// ```
pub struct RingBuffer {
    /// Producer handle (write end)
    producer: Arc<Mutex<ringbuf::HeapProd<f32>>>,

    /// Consumer handle (read end)
    consumer: Arc<Mutex<ringbuf::HeapCons<f32>>>,

    /// Buffer capacity in stereo samples
    capacity: usize,
}

impl RingBuffer {
    /// Create new ring buffer
    ///
    /// # Arguments
    ///
    /// * `capacity` - Buffer capacity in stereo samples (NOT individual f32 values)
    ///
    /// Per SPEC016 DBD-PARAM-070:
    /// - Default: 661,941 samples (15.01 seconds at 44.1kHz)
    /// - Memory: capacity * 2 (L+R) * 4 bytes = capacity * 8 bytes
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let buffer = RingBuffer::new(661941); // 15 seconds
    /// ```
    pub fn new(capacity: usize) -> Self {
        // Create ring buffer for interleaved stereo samples
        // Capacity in f32 values = capacity_in_samples * 2 (L+R)
        let rb = HeapRb::new(capacity * 2);
        let (producer, consumer) = rb.split();

        Self {
            producer: Arc::new(Mutex::new(producer)),
            consumer: Arc::new(Mutex::new(consumer)),
            capacity,
        }
    }

    /// Push samples into buffer
    ///
    /// # Arguments
    ///
    /// * `samples` - Interleaved stereo f32 samples [L, R, L, R, ...]
    ///
    /// # Returns
    ///
    /// Number of stereo samples actually written (may be less than requested if buffer full)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let samples = vec![0.1, 0.2, 0.3, 0.4]; // 2 stereo samples
    /// let written = buffer.push(&samples)?;
    /// assert_eq!(written, 2);
    /// ```
    pub fn push(&mut self, samples: &[f32]) -> Result<usize> {
        if samples.len() % 2 != 0 {
            return Err(AudioPlayerError::Buffer(
                crate::error::BufferError::InvalidSampleCount(samples.len()),
            ));
        }

        // Get producer handle
        let mut producer = self.producer.lock().unwrap();

        // Write samples
        let written = producer.push_slice(samples);

        // Return number of stereo samples written
        Ok(written / 2)
    }

    /// Pop samples from buffer
    ///
    /// # Arguments
    ///
    /// * `count` - Number of stereo samples to read
    ///
    /// # Returns
    ///
    /// Vector of interleaved stereo f32 samples. May return fewer samples
    /// than requested if buffer doesn't contain enough data.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let samples = buffer.pop(4410)?; // Try to read 0.1 seconds
    /// println!("Read {} stereo samples", samples.len() / 2);
    /// ```
    pub fn pop(&mut self, count: usize) -> Result<Vec<f32>> {
        let mut consumer = self.consumer.lock().unwrap();
        let mut output = vec![0.0f32; count * 2]; // Allocate for stereo samples

        // Read available samples
        let read = consumer.pop_slice(&mut output);

        // Truncate to actual read count
        output.truncate(read);

        Ok(output)
    }

    /// Get number of stereo samples currently in buffer
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let filled = buffer.len();
    /// println!("Buffer contains {} stereo samples", filled);
    /// ```
    pub fn len(&self) -> usize {
        let consumer = self.consumer.lock().unwrap();
        consumer.occupied_len() / 2
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        let consumer = self.consumer.lock().unwrap();
        consumer.is_empty()
    }

    /// Get buffer capacity in stereo samples
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get free space in stereo samples
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let free = buffer.free_space();
    /// println!("Can write {} more stereo samples", free);
    /// ```
    pub fn free_space(&self) -> usize {
        let producer = self.producer.lock().unwrap();
        producer.vacant_len() / 2
    }

    /// Clear all samples from buffer
    pub fn clear(&mut self) {
        let mut consumer = self.consumer.lock().unwrap();
        consumer.clear();
    }
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_new() {
        let buffer = RingBuffer::new(1000);
        assert_eq!(buffer.capacity(), 1000);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.free_space(), 1000);
    }

    #[test]
    fn test_push_pop() {
        let mut buffer = RingBuffer::new(1000);

        // Push 2 stereo samples
        let samples = vec![0.1, 0.2, 0.3, 0.4];
        let written = buffer.push(&samples).unwrap();
        assert_eq!(written, 2);
        assert_eq!(buffer.len(), 2);

        // Pop 1 stereo sample
        let output = buffer.pop(1).unwrap();
        assert_eq!(output.len(), 2);
        assert_eq!(output, vec![0.1, 0.2]);
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_push_odd_samples_fails() {
        let mut buffer = RingBuffer::new(1000);

        // Try to push odd number of f32 values
        let samples = vec![0.1, 0.2, 0.3];
        let result = buffer.push(&samples);
        assert!(result.is_err());
    }

    #[test]
    fn test_buffer_full() {
        let mut buffer = RingBuffer::new(2); // Capacity: 2 stereo samples

        // Fill buffer
        let samples = vec![0.1, 0.2, 0.3, 0.4]; // 2 stereo samples
        let written = buffer.push(&samples).unwrap();
        assert_eq!(written, 2);
        assert_eq!(buffer.free_space(), 0);

        // Try to write more (should write 0)
        let more = vec![0.5, 0.6];
        let written = buffer.push(&more).unwrap();
        assert_eq!(written, 0);
    }

    #[test]
    fn test_pop_more_than_available() {
        let mut buffer = RingBuffer::new(1000);

        // Push 2 stereo samples
        let samples = vec![0.1, 0.2, 0.3, 0.4];
        buffer.push(&samples).unwrap();

        // Try to pop 10 samples (only 2 available)
        let output = buffer.pop(10).unwrap();
        assert_eq!(output.len(), 4); // Got 2 stereo samples (4 f32 values)
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut buffer = RingBuffer::new(1000);

        // Add samples
        let samples = vec![0.1, 0.2, 0.3, 0.4];
        buffer.push(&samples).unwrap();
        assert_eq!(buffer.len(), 2);

        // Clear
        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_wraparound() {
        let mut buffer = RingBuffer::new(3); // Small buffer to test wraparound

        // Fill buffer
        let samples1 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 3 stereo samples
        buffer.push(&samples1).unwrap();
        assert_eq!(buffer.len(), 3);

        // Pop 2 samples
        buffer.pop(2).unwrap();
        assert_eq!(buffer.len(), 1);

        // Push 2 more (should wrap around)
        let samples2 = vec![7.0, 8.0, 9.0, 10.0]; // 2 stereo samples
        let written = buffer.push(&samples2).unwrap();
        assert_eq!(written, 2);
        assert_eq!(buffer.len(), 3);
    }
}

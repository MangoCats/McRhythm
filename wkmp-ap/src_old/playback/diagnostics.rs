//! Pipeline Diagnostics and Validation
//!
//! **[PHASE1-INTEGRITY]** End-to-end sample integrity validation
//!
//! Provides validation of the decoder → buffer → mixer pipeline to ensure:
//! 1. All decoded samples are written to buffers
//! 2. All buffer writes are eventually read
//! 3. All buffer reads are mixed to output
//!
//! **Conservation Laws:**
//! - Rule 1: decoder_frames × 2 channels ≈ buffer_samples_written (±tolerance)
//! - Rule 2: buffer_samples_written ≥ buffer_samples_read (FIFO invariant)
//! - Rule 3: buffer_samples_read ≤ mixer_total_frames_mixed (mixer consumes buffer)
//!
//! **Traceability:**
//! - [DBD-INT-010] End-to-end sample integrity validation
//! - [DBD-INT-020] Conservation law validation
//! - [DBD-INT-030] Tolerance-based pass/fail thresholds

use std::collections::HashMap;
use uuid::Uuid;

/// Metrics for a single passage in the pipeline
#[derive(Debug, Clone)]
pub struct PassageMetrics {
    /// Passage/queue entry ID
    pub passage_id: Uuid,

    /// Total stereo frames pushed by decoder to buffer
    pub decoder_frames_pushed: usize,

    /// Total samples written to playout ring buffer (frames × 2)
    pub buffer_samples_written: u64,

    /// Total samples read from playout ring buffer
    pub buffer_samples_read: u64,

    /// File path for debugging
    pub file_path: Option<String>,
}

impl PassageMetrics {
    /// Create new passage metrics
    pub fn new(
        passage_id: Uuid,
        decoder_frames_pushed: usize,
        buffer_samples_written: u64,
        buffer_samples_read: u64,
        file_path: Option<String>,
    ) -> Self {
        Self {
            passage_id,
            decoder_frames_pushed,
            buffer_samples_written,
            buffer_samples_read,
            file_path,
        }
    }
}

/// Aggregated pipeline metrics across all passages
#[derive(Debug, Clone)]
pub struct PipelineMetrics {
    /// Per-passage metrics
    pub passages: HashMap<Uuid, PassageMetrics>,

    /// Total frames mixed by mixer (should match sum of buffer_samples_read)
    pub mixer_total_frames_mixed: u64,
}

impl PipelineMetrics {
    /// Create new pipeline metrics
    pub fn new(passages: HashMap<Uuid, PassageMetrics>, mixer_total_frames_mixed: u64) -> Self {
        Self {
            passages,
            mixer_total_frames_mixed,
        }
    }

    /// Validate pipeline integrity with tolerance
    ///
    /// **[DBD-INT-020]** Conservation law validation
    ///
    /// # Arguments
    /// * `tolerance_samples` - Allowed deviation in sample counts (e.g., 8192 samples = ~0.18s)
    ///
    /// # Returns
    /// Validation result with pass/fail and detailed errors
    pub fn validate(&self, tolerance_samples: u64) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Accumulate totals
        let mut total_decoder_samples = 0u64;
        let mut total_buffer_written = 0u64;
        let mut total_buffer_read = 0u64;

        // Per-passage validation
        for (passage_id, metrics) in &self.passages {
            let decoder_samples = (metrics.decoder_frames_pushed as u64) * 2; // frames → samples
            total_decoder_samples += decoder_samples;
            total_buffer_written += metrics.buffer_samples_written;
            total_buffer_read += metrics.buffer_samples_read;

            // Rule 1: decoder_frames × 2 ≈ buffer_samples_written (±tolerance)
            let diff_1 = if decoder_samples > metrics.buffer_samples_written {
                decoder_samples - metrics.buffer_samples_written
            } else {
                metrics.buffer_samples_written - decoder_samples
            };

            if diff_1 > tolerance_samples {
                result.add_error(ValidationError::DecoderBufferMismatch {
                    passage_id: *passage_id,
                    decoder_samples,
                    buffer_samples_written: metrics.buffer_samples_written,
                    difference: diff_1,
                    tolerance: tolerance_samples,
                    file_path: metrics.file_path.clone(),
                });
            }

            // Rule 2: buffer_samples_written ≥ buffer_samples_read (FIFO invariant)
            if metrics.buffer_samples_written < metrics.buffer_samples_read {
                result.add_error(ValidationError::BufferFifoViolation {
                    passage_id: *passage_id,
                    buffer_samples_written: metrics.buffer_samples_written,
                    buffer_samples_read: metrics.buffer_samples_read,
                    file_path: metrics.file_path.clone(),
                });
            }
        }

        // Rule 3: Total buffer_samples_read should approximately match mixer frames
        // Mixer counts frames (stereo pairs), buffer counts samples
        // So mixer_frames should ≈ buffer_samples_read / 2
        let expected_mixer_frames = total_buffer_read / 2;
        let diff_3 = if expected_mixer_frames > self.mixer_total_frames_mixed {
            expected_mixer_frames - self.mixer_total_frames_mixed
        } else {
            self.mixer_total_frames_mixed - expected_mixer_frames
        };

        // Tolerance for mixer should account for buffering (divide by 2 since comparing frames not samples)
        let mixer_tolerance_frames = tolerance_samples / 2;
        if diff_3 > mixer_tolerance_frames {
            result.add_error(ValidationError::MixerTotalMismatch {
                expected_mixer_frames,
                actual_mixer_frames: self.mixer_total_frames_mixed,
                total_buffer_read,
                difference: diff_3,
                tolerance: mixer_tolerance_frames,
            });
        }

        // Add summary info
        result.total_decoder_samples = total_decoder_samples;
        result.total_buffer_written = total_buffer_written;
        result.total_buffer_read = total_buffer_read;
        result.total_mixer_frames = self.mixer_total_frames_mixed;
        result.passage_count = self.passages.len();

        result
    }
}

/// Result of pipeline validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// List of validation errors (empty if all passed)
    pub errors: Vec<ValidationError>,

    /// Summary statistics
    pub total_decoder_samples: u64,
    pub total_buffer_written: u64,
    pub total_buffer_read: u64,
    pub total_mixer_frames: u64,
    pub passage_count: usize,
}

impl ValidationResult {
    /// Create new empty validation result
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            total_decoder_samples: 0,
            total_buffer_written: 0,
            total_buffer_read: 0,
            total_mixer_frames: 0,
            passage_count: 0,
        }
    }

    /// Check if validation passed (no errors)
    pub fn passed(&self) -> bool {
        self.errors.is_empty()
    }

    /// Add validation error
    fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Format validation result as human-readable string
    pub fn format_report(&self) -> String {
        let mut report = String::new();

        report.push_str("=== Pipeline Integrity Validation Report ===\n\n");

        // Summary
        report.push_str(&format!("Status: {}\n", if self.passed() { "PASS ✓" } else { "FAIL ✗" }));
        report.push_str(&format!("Passages validated: {}\n", self.passage_count));
        report.push_str(&format!("Errors found: {}\n\n", self.errors.len()));

        // Statistics
        report.push_str("Pipeline Statistics:\n");
        report.push_str(&format!("  Decoder samples:      {:12} samples\n", self.total_decoder_samples));
        report.push_str(&format!("  Buffer samples written: {:12} samples\n", self.total_buffer_written));
        report.push_str(&format!("  Buffer samples read:    {:12} samples\n", self.total_buffer_read));
        report.push_str(&format!("  Mixer frames mixed:   {:12} frames ({} samples)\n\n",
            self.total_mixer_frames, self.total_mixer_frames * 2));

        // Errors
        if !self.errors.is_empty() {
            report.push_str("Validation Errors:\n");
            for (i, error) in self.errors.iter().enumerate() {
                report.push_str(&format!("\n{}. {}\n", i + 1, error.format()));
            }
        }

        report
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Types of validation errors
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Decoder output doesn't match buffer input
    DecoderBufferMismatch {
        passage_id: Uuid,
        decoder_samples: u64,
        buffer_samples_written: u64,
        difference: u64,
        tolerance: u64,
        file_path: Option<String>,
    },

    /// Buffer read more samples than written (FIFO violation)
    BufferFifoViolation {
        passage_id: Uuid,
        buffer_samples_written: u64,
        buffer_samples_read: u64,
        file_path: Option<String>,
    },

    /// Mixer total doesn't match accumulated buffer reads
    MixerTotalMismatch {
        expected_mixer_frames: u64,
        actual_mixer_frames: u64,
        total_buffer_read: u64,
        difference: u64,
        tolerance: u64,
    },
}

impl ValidationError {
    /// Format error as human-readable string
    pub fn format(&self) -> String {
        match self {
            ValidationError::DecoderBufferMismatch {
                passage_id,
                decoder_samples,
                buffer_samples_written,
                difference,
                tolerance,
                file_path,
            } => {
                format!(
                    "Decoder-Buffer Mismatch (Rule 1 violation)\n   \
                    Passage: {}\n   \
                    File: {}\n   \
                    Decoder output: {} samples\n   \
                    Buffer written:  {} samples\n   \
                    Difference: {} samples (tolerance: {})",
                    passage_id,
                    file_path.as_deref().unwrap_or("unknown"),
                    decoder_samples,
                    buffer_samples_written,
                    difference,
                    tolerance
                )
            }
            ValidationError::BufferFifoViolation {
                passage_id,
                buffer_samples_written,
                buffer_samples_read,
                file_path,
            } => {
                format!(
                    "Buffer FIFO Violation (Rule 2 violation)\n   \
                    Passage: {}\n   \
                    File: {}\n   \
                    Buffer written: {} samples\n   \
                    Buffer read:    {} samples\n   \
                    ERROR: Read {} more samples than written!",
                    passage_id,
                    file_path.as_deref().unwrap_or("unknown"),
                    buffer_samples_written,
                    buffer_samples_read,
                    buffer_samples_read - buffer_samples_written
                )
            }
            ValidationError::MixerTotalMismatch {
                expected_mixer_frames,
                actual_mixer_frames,
                total_buffer_read,
                difference,
                tolerance,
            } => {
                format!(
                    "Mixer Total Mismatch (Rule 3 violation)\n   \
                    Expected mixer frames: {} frames (from {} buffer samples read)\n   \
                    Actual mixer frames:   {} frames\n   \
                    Difference: {} frames (tolerance: {})",
                    expected_mixer_frames,
                    total_buffer_read,
                    actual_mixer_frames,
                    difference,
                    tolerance
                )
            }
        }
    }

    /// Get the discrepancy value (for warning threshold detection)
    ///
    /// Returns the absolute difference for errors that have tolerance,
    /// or None for hard errors (FIFO violation).
    pub fn discrepancy(&self) -> Option<u64> {
        match self {
            ValidationError::DecoderBufferMismatch { difference, .. } => Some(*difference),
            ValidationError::BufferFifoViolation { .. } => None, // Hard error, no tolerance
            ValidationError::MixerTotalMismatch { difference, .. } => Some(*difference),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_pipeline() {
        // Perfect pipeline: 44100 frames decoded → 88200 samples written/read → 44100 frames mixed
        let mut passages = HashMap::new();
        let passage_id = Uuid::new_v4();

        passages.insert(
            passage_id,
            PassageMetrics::new(
                passage_id,
                44100,  // decoder frames
                88200,  // buffer samples written (44100 × 2)
                88200,  // buffer samples read (equals written)
                Some("test.mp3".to_string()),
            ),
        );

        let metrics = PipelineMetrics::new(passages, 44100); // mixer frames
        let result = metrics.validate(8192);

        assert!(result.passed(), "Perfect pipeline should pass validation");
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.total_decoder_samples, 88200);
        assert_eq!(result.total_buffer_written, 88200);
        assert_eq!(result.total_buffer_read, 88200);
        assert_eq!(result.total_mixer_frames, 44100);
    }

    #[test]
    fn test_decoder_buffer_mismatch() {
        let mut passages = HashMap::new();
        let passage_id = Uuid::new_v4();

        // Decoder says 44100 frames, but buffer only wrote 40000 samples (off by 8200)
        passages.insert(
            passage_id,
            PassageMetrics::new(
                passage_id,
                44100,  // decoder frames → 88200 samples expected
                40000,  // buffer samples written (MISMATCH!)
                40000,  // buffer samples read
                Some("test.mp3".to_string()),
            ),
        );

        let metrics = PipelineMetrics::new(passages, 20000); // mixer frames
        let result = metrics.validate(8192); // tolerance

        assert!(!result.passed(), "Should fail validation");
        assert_eq!(result.errors.len(), 1);

        match &result.errors[0] {
            ValidationError::DecoderBufferMismatch { difference, .. } => {
                assert_eq!(*difference, 48200); // 88200 - 40000
            }
            _ => panic!("Expected DecoderBufferMismatch error"),
        }
    }

    #[test]
    fn test_buffer_fifo_violation() {
        let mut passages = HashMap::new();
        let passage_id = Uuid::new_v4();

        // Buffer read MORE than written (impossible!)
        passages.insert(
            passage_id,
            PassageMetrics::new(
                passage_id,
                44100,  // decoder frames
                88200,  // buffer samples written
                90000,  // buffer samples read (MORE than written!)
                Some("test.mp3".to_string()),
            ),
        );

        let metrics = PipelineMetrics::new(passages, 45000); // mixer frames
        let result = metrics.validate(8192);

        assert!(!result.passed(), "Should fail validation");

        // Should have at least the FIFO violation error
        let has_fifo_error = result.errors.iter().any(|e| matches!(e, ValidationError::BufferFifoViolation { .. }));
        assert!(has_fifo_error, "Should detect FIFO violation");
    }

    #[test]
    fn test_within_tolerance() {
        let mut passages = HashMap::new();
        let passage_id = Uuid::new_v4();

        // Slightly off but within tolerance (8192 samples)
        passages.insert(
            passage_id,
            PassageMetrics::new(
                passage_id,
                44100,  // decoder frames → 88200 samples
                88205,  // buffer samples written (+5 samples, within tolerance)
                88205,  // buffer samples read
                Some("test.mp3".to_string()),
            ),
        );

        let metrics = PipelineMetrics::new(passages, 44102); // mixer frames (+2 frames = +4 samples, within tolerance)
        let result = metrics.validate(8192);

        assert!(result.passed(), "Small differences within tolerance should pass");
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_multiple_passages() {
        let mut passages = HashMap::new();

        // Passage 1: Perfect
        let passage_1 = Uuid::new_v4();
        passages.insert(
            passage_1,
            PassageMetrics::new(passage_1, 44100, 88200, 88200, Some("song1.mp3".to_string())),
        );

        // Passage 2: Perfect
        let passage_2 = Uuid::new_v4();
        passages.insert(
            passage_2,
            PassageMetrics::new(passage_2, 22050, 44100, 44100, Some("song2.mp3".to_string())),
        );

        // Total: 66150 frames decoded → 132300 samples → 66150 frames mixed
        let metrics = PipelineMetrics::new(passages, 66150);
        let result = metrics.validate(8192);

        assert!(result.passed(), "Multiple perfect passages should pass");
        assert_eq!(result.passage_count, 2);
        assert_eq!(result.total_decoder_samples, 132300);
        assert_eq!(result.total_mixer_frames, 66150);
    }

    #[test]
    fn test_validation_report_format() {
        let mut passages = HashMap::new();
        let passage_id = Uuid::new_v4();

        passages.insert(
            passage_id,
            PassageMetrics::new(passage_id, 44100, 88200, 88200, Some("test.mp3".to_string())),
        );

        let metrics = PipelineMetrics::new(passages, 44100);
        let result = metrics.validate(8192);

        let report = result.format_report();

        assert!(report.contains("Pipeline Integrity Validation Report"));
        assert!(report.contains("Status: PASS ✓"));
        assert!(report.contains("Passages validated: 1"));
        assert!(report.contains("Decoder samples:"));
        assert!(report.contains("88200"));
    }
}

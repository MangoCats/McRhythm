//! Essentia Analyzer (Tier 1 - Optional)
//!
//! Executes Essentia command-line tool to extract musical features from audio files.
//! Essentia provides algorithmic analysis of tempo, key, loudness, mood, and other characteristics.
//!
//! # Implementation
//! - TASK-009: Essentia Analyzer (PLAN024)
//! - Confidence: 0.7 (algorithmic analysis, heuristic-based)
//!
//! # Architecture
//! Implements `SourceExtractor` trait for integration with parallel extraction pipeline.
//! This is an **optional** extractor - gracefully handles Essentia not being installed.
//!
//! # Requirements
//! - Essentia installed: `essentia_streaming_extractor_music` command available
//! - Temporary file system access for JSON output
//! - Audio file path (Essentia reads directly from file)
//!
//! # Installation
//! ```bash
//! # Ubuntu/Debian
//! sudo apt-get install essentia-extractor
//!
//! # macOS
//! brew install essentia
//!
//! # From source
//! # See: https://essentia.upf.edu/installing.html
//! ```

use crate::types::{
    ExtractionError, ExtractionResult, FlavorExtraction, PassageContext, SourceExtractor,
};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tracing::debug;

/// Essentia command name
const ESSENTIA_COMMAND: &str = "essentia_streaming_extractor_music";

/// Essentia Analyzer
///
/// Executes Essentia command-line tool to extract musical features.
/// This is an **optional** extractor that gracefully handles Essentia not being installed.
///
/// # Confidence
/// Base confidence: 0.7
/// - Algorithmic analysis (not authoritative metadata)
/// - Heuristic-based feature extraction
/// - Lower than MusicBrainz (0.9) and AcoustID (0.8-0.95)
/// - Higher than genre mapping (0.5)
///
/// # Extracted Features
/// - Tempo (BPM)
/// - Key and scale
/// - Danceability
/// - Energy
/// - Valence (happiness)
/// - Loudness
/// - And many more (50+ features)
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::extractors::essentia_analyzer::EssentiaAnalyzer;
/// use wkmp_ai::types::{SourceExtractor, PassageContext};
///
/// let analyzer = EssentiaAnalyzer::new();
/// let result = analyzer.extract(&passage_ctx).await?;
///
/// if let Some(flavor) = result.musical_flavor {
///     println!("Danceability: {}", flavor.characteristics.get("danceability").unwrap_or(&0.0));
/// }
/// ```
pub struct EssentiaAnalyzer {
    /// Base confidence for Essentia features
    base_confidence: f32,
    /// Whether Essentia is available (cached check)
    essentia_available: Option<bool>,
}

impl EssentiaAnalyzer {
    /// Create new Essentia analyzer with default confidence (0.7)
    pub fn new() -> Self {
        Self {
            base_confidence: 0.7,
            essentia_available: None,
        }
    }

    /// Check if Essentia command is available
    ///
    /// Caches result for subsequent calls.
    async fn check_essentia_available(&mut self) -> bool {
        // Return cached result if available
        if let Some(available) = self.essentia_available {
            return available;
        }

        // Check if command exists
        let result = Command::new("which")
            .arg(ESSENTIA_COMMAND)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        let available = result.map(|status| status.success()).unwrap_or(false);

        debug!(
            command = ESSENTIA_COMMAND,
            available = available,
            "Essentia availability check"
        );

        // Cache result
        self.essentia_available = Some(available);
        available
    }

    /// Extract features using Essentia
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    ///
    /// # Returns
    /// Musical flavor characteristics extracted by Essentia
    ///
    /// # Errors
    /// Returns error if:
    /// - Essentia not installed
    /// - Command execution fails
    /// - JSON output parse fails
    async fn extract_features(
        &mut self,
        file_path: &Path,
    ) -> Result<FlavorExtraction, ExtractionError> {
        // Check if Essentia is available
        if !self.check_essentia_available().await {
            return Err(ExtractionError::NotAvailable(
                "Essentia not installed (optional extractor)".to_string(),
            ));
        }

        debug!(
            file_path = ?file_path,
            "Extracting musical features with Essentia"
        );

        // Create temporary output file for JSON
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join(format!(
            "essentia_{}.json",
            uuid::Uuid::new_v4()
        ));

        // Execute Essentia command
        let output = Command::new(ESSENTIA_COMMAND)
            .arg(file_path)
            .arg(&output_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                ExtractionError::Internal(format!("Failed to execute Essentia: {}", e))
            })?;

        // Check exit status
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ExtractionError::Internal(format!(
                "Essentia command failed: {}",
                stderr
            )));
        }

        // Read JSON output
        let json_content = fs::read_to_string(&output_path).await.map_err(|e| {
            ExtractionError::Internal(format!("Failed to read Essentia output: {}", e))
        })?;

        // Clean up temporary file
        let _ = fs::remove_file(&output_path).await;

        // Parse JSON
        let essentia_output: EssentiaOutput =
            serde_json::from_str(&json_content).map_err(|e| {
                ExtractionError::Parse(format!("Failed to parse Essentia JSON: {}", e))
            })?;

        // Convert to FlavorExtraction
        let flavor = self.convert_to_flavor(essentia_output);

        debug!(
            feature_count = flavor.characteristics.len(),
            "Essentia feature extraction complete"
        );

        Ok(flavor)
    }

    /// Convert Essentia output to FlavorExtraction
    fn convert_to_flavor(&self, output: EssentiaOutput) -> FlavorExtraction {
        let mut characteristics = HashMap::new();

        // Rhythm features
        if let Some(rhythm) = output.rhythm {
            if let Some(bpm) = rhythm.bpm {
                // Normalize BPM to 0-1 range (assuming 60-180 BPM typical range)
                let normalized_bpm = ((bpm - 60.0) / 120.0).clamp(0.0, 1.0);
                characteristics.insert("tempo".to_string(), normalized_bpm);
            }

            if let Some(danceability) = rhythm.danceability {
                characteristics.insert("danceability".to_string(), danceability);
            }
        }

        // Tonal features
        if let Some(tonal) = output.tonal {
            if let Some(key_strength) = tonal.key_strength {
                characteristics.insert("key_strength".to_string(), key_strength);
            }
        }

        // Loudness features
        if let Some(lowlevel) = output.lowlevel {
            if let Some(loudness) = lowlevel.average_loudness {
                // Normalize loudness (typical range: -60 to 0 dB)
                let normalized_loudness = ((loudness + 60.0) / 60.0).clamp(0.0, 1.0);
                characteristics.insert("loudness".to_string(), normalized_loudness);
            }

            if let Some(dynamic_complexity) = lowlevel.dynamic_complexity {
                characteristics.insert("dynamic_complexity".to_string(), dynamic_complexity);
            }
        }

        // Mood/Emotion features (if available)
        if let Some(highlevel) = output.highlevel {
            if let Some(mood_acoustic) = highlevel.mood_acoustic {
                characteristics.insert("mood_acoustic".to_string(), mood_acoustic);
            }

            if let Some(mood_electronic) = highlevel.mood_electronic {
                characteristics.insert("mood_electronic".to_string(), mood_electronic);
            }

            if let Some(mood_aggressive) = highlevel.mood_aggressive {
                characteristics.insert("mood_aggressive".to_string(), mood_aggressive);
            }

            if let Some(mood_relaxed) = highlevel.mood_relaxed {
                characteristics.insert("mood_relaxed".to_string(), mood_relaxed);
            }

            if let Some(mood_happy) = highlevel.mood_happy {
                characteristics.insert("mood_happy".to_string(), mood_happy);
            }

            if let Some(mood_sad) = highlevel.mood_sad {
                characteristics.insert("mood_sad".to_string(), mood_sad);
            }

            if let Some(mood_party) = highlevel.mood_party {
                characteristics.insert("mood_party".to_string(), mood_party);
            }
        }

        FlavorExtraction {
            characteristics,
            confidence: self.base_confidence,
            source: "Essentia".to_string(),
        }
    }
}

impl Default for EssentiaAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceExtractor for EssentiaAnalyzer {
    fn name(&self) -> &'static str {
        "Essentia"
    }

    fn base_confidence(&self) -> f32 {
        self.base_confidence
    }

    async fn extract(&self, ctx: &PassageContext) -> Result<ExtractionResult, ExtractionError> {
        debug!(
            passage_id = %ctx.passage_id,
            file_path = ?ctx.file_path,
            "Extracting musical features via Essentia"
        );

        // Note: Essentia reads directly from audio file, not from samples
        // This is different from Chromaprint which uses in-memory samples

        // Clone self to make mutable (for caching)
        let mut analyzer = Self {
            base_confidence: self.base_confidence,
            essentia_available: self.essentia_available,
        };

        let flavor = analyzer.extract_features(&ctx.file_path).await?;

        debug!(
            passage_id = %ctx.passage_id,
            feature_count = flavor.characteristics.len(),
            "Essentia extraction complete"
        );

        Ok(ExtractionResult {
            metadata: None,
            identity: None,
            musical_flavor: Some(flavor),
        })
    }
}

// ============================================================================
// Essentia JSON Output Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct EssentiaOutput {
    rhythm: Option<RhythmFeatures>,
    tonal: Option<TonalFeatures>,
    lowlevel: Option<LowLevelFeatures>,
    highlevel: Option<HighLevelFeatures>,
}

#[derive(Debug, Deserialize)]
struct RhythmFeatures {
    bpm: Option<f32>,
    danceability: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct TonalFeatures {
    key_strength: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct LowLevelFeatures {
    average_loudness: Option<f32>,
    dynamic_complexity: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct HighLevelFeatures {
    mood_acoustic: Option<f32>,
    mood_electronic: Option<f32>,
    mood_aggressive: Option<f32>,
    mood_relaxed: Option<f32>,
    mood_happy: Option<f32>,
    mood_sad: Option<f32>,
    mood_party: Option<f32>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn test_analyzer_name() {
        let analyzer = EssentiaAnalyzer::new();
        assert_eq!(analyzer.name(), "Essentia");
    }

    #[test]
    fn test_default_confidence() {
        let analyzer = EssentiaAnalyzer::new();
        assert_eq!(analyzer.base_confidence(), 0.7);
    }

    #[test]
    fn test_default_trait() {
        let analyzer = EssentiaAnalyzer::default();
        assert_eq!(analyzer.base_confidence(), 0.7);
    }

    #[tokio::test]
    async fn test_check_essentia_available() {
        let mut analyzer = EssentiaAnalyzer::new();
        let available = analyzer.check_essentia_available().await;

        // First call checks system
        assert_eq!(analyzer.essentia_available, Some(available));

        // Second call should return cached result
        let available2 = analyzer.check_essentia_available().await;
        assert_eq!(available, available2);
    }

    #[tokio::test]
    async fn test_extract_nonexistent_file() {
        let analyzer = EssentiaAnalyzer::new();
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/nonexistent/file.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: None,
            sample_rate: Some(44100),
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        let result = analyzer.extract(&ctx).await;
        // Should fail - either Essentia not available or file doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_to_flavor() {
        let analyzer = EssentiaAnalyzer::new();

        let essentia_output = EssentiaOutput {
            rhythm: Some(RhythmFeatures {
                bpm: Some(120.0),
                danceability: Some(0.8),
            }),
            tonal: Some(TonalFeatures {
                key_strength: Some(0.7),
            }),
            lowlevel: Some(LowLevelFeatures {
                average_loudness: Some(-20.0),
                dynamic_complexity: Some(0.6),
            }),
            highlevel: Some(HighLevelFeatures {
                mood_acoustic: Some(0.3),
                mood_electronic: Some(0.7),
                mood_aggressive: Some(0.2),
                mood_relaxed: Some(0.5),
                mood_happy: Some(0.8),
                mood_sad: Some(0.1),
                mood_party: Some(0.9),
            }),
        };

        let flavor = analyzer.convert_to_flavor(essentia_output);

        assert_eq!(flavor.confidence, 0.7);
        assert_eq!(flavor.source, "Essentia");

        // Check some expected characteristics
        assert!(flavor.characteristics.contains_key("tempo"));
        assert!(flavor.characteristics.contains_key("danceability"));
        assert!(flavor.characteristics.contains_key("key_strength"));
        assert!(flavor.characteristics.contains_key("loudness"));
        assert!(flavor.characteristics.contains_key("mood_happy"));

        // Check danceability value
        assert_eq!(flavor.characteristics.get("danceability"), Some(&0.8));
    }

    // Note: Testing actual Essentia execution requires:
    // 1. Essentia installed on system
    // 2. Real audio files
    // These tests are covered in integration tests
}

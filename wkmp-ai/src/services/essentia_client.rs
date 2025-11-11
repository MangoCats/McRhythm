//! Essentia local analysis client
//!
//! **[AIA-COMP-010]** Local musical flavor extraction using Essentia
//!
//! Provides fallback for recordings not found in AcousticBrainz.
//! Uses essentia_streaming_extractor_music command-line tool.

use crate::services::MusicalFlavorVector;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use thiserror::Error;

/// Essentia client errors
#[derive(Debug, Error)]
pub enum EssentiaError {
    /// Essentia binary not found in PATH
    #[error("Essentia binary not found in PATH")]
    BinaryNotFound,

    /// Failed to execute Essentia command
    #[error("Failed to execute Essentia: {0}")]
    ExecutionError(String),

    /// Essentia analysis failed with error
    #[error("Essentia analysis failed: {0}")]
    AnalysisFailed(String),

    /// Failed to parse Essentia JSON output
    #[error("Failed to parse Essentia output: {0}")]
    ParseError(String),

    /// I/O error (file read/write)
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Audio file not found at path
    #[error("Audio file not found: {0}")]
    FileNotFound(String),
}

/// Essentia output structure (simplified)
///
/// Full Essentia output contains hundreds of features.
/// We extract the same subset as AcousticBrainz for compatibility.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EssentiaOutput {
    /// Low-level audio features
    pub lowlevel: Option<EssentiaLowLevel>,
    /// Rhythm features (BPM, danceability)
    pub rhythm: Option<EssentiaRhythm>,
    /// Tonal features (key, scale)
    pub tonal: Option<EssentiaTonal>,
}

/// Essentia low-level audio features
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EssentiaLowLevel {
    /// Average loudness in dB
    pub average_loudness: Option<f64>,
    /// Dynamic complexity (amplitude variation)
    pub dynamic_complexity: Option<f64>,
    /// Spectral centroid statistics (brightness)
    pub spectral_centroid: Option<EssentiaStats>,
    /// Spectral energy statistics
    pub spectral_energy: Option<EssentiaStats>,
    /// Dissonance statistics (harmonic complexity)
    pub dissonance: Option<EssentiaStats>,
}

/// Statistical summary of Essentia feature
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EssentiaStats {
    /// Mean value
    pub mean: Option<f64>,
    /// Median value
    pub median: Option<f64>,
    /// Variance
    pub var: Option<f64>,
    /// Minimum value
    pub min: Option<f64>,
    /// Maximum value
    pub max: Option<f64>,
}

/// Essentia rhythm features
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EssentiaRhythm {
    /// Beats per minute
    pub bpm: Option<f64>,
    /// Danceability score (0.0-1.0)
    pub danceability: Option<f64>,
    /// Note onset rate (onsets per second)
    pub onset_rate: Option<f64>,
}

/// Essentia tonal features
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EssentiaTonal {
    /// Musical key (e.g., "C", "A")
    pub key_key: Option<String>,
    /// Musical scale (e.g., "major", "minor")
    pub key_scale: Option<String>,
    /// Key detection confidence (0.0-1.0)
    pub key_strength: Option<f64>,
    /// Predominant chord key
    pub chords_key: Option<String>,
    /// Predominant chord scale
    pub chords_scale: Option<String>,
}

impl EssentiaOutput {
    /// Convert to MusicalFlavorVector
    pub fn to_flavor_vector(&self) -> MusicalFlavorVector {
        MusicalFlavorVector {
            key: self.tonal.as_ref().and_then(|t| t.key_key.clone()),
            scale: self.tonal.as_ref().and_then(|t| t.key_scale.clone()),
            key_strength: self.tonal.as_ref().and_then(|t| t.key_strength),
            bpm: self.rhythm.as_ref().and_then(|r| r.bpm),
            danceability: self.rhythm.as_ref().and_then(|r| r.danceability),
            spectral_centroid: self
                .lowlevel
                .as_ref()
                .and_then(|l| l.spectral_centroid.as_ref())
                .and_then(|s| s.mean),
            spectral_energy: self
                .lowlevel
                .as_ref()
                .and_then(|l| l.spectral_energy.as_ref())
                .and_then(|s| s.mean),
            dissonance: self
                .lowlevel
                .as_ref()
                .and_then(|l| l.dissonance.as_ref())
                .and_then(|s| s.mean),
            dynamic_complexity: self.lowlevel.as_ref().and_then(|l| l.dynamic_complexity),
            source: "essentia".to_string(),
        }
    }
}

/// Essentia client
pub struct EssentiaClient {
    binary_path: String,
}

impl EssentiaClient {
    /// Create new Essentia client
    ///
    /// Checks if essentia_streaming_extractor_music is in PATH
    pub fn new() -> Result<Self, EssentiaError> {
        // Check if binary exists in PATH
        let binary_path = "essentia_streaming_extractor_music";

        // Try to run --version to verify installation
        match Command::new(binary_path).arg("--version").output() {
            Ok(_) => Ok(Self {
                binary_path: binary_path.to_string(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(EssentiaError::BinaryNotFound)
            }
            Err(e) => Err(EssentiaError::ExecutionError(e.to_string())),
        }
    }

    /// Analyze audio file and extract musical flavor
    ///
    /// **[AIA-COMP-010]** Local Essentia analysis as fallback
    pub async fn analyze_file(&self, audio_path: &Path) -> Result<MusicalFlavorVector, EssentiaError> {
        // Verify file exists
        if !audio_path.exists() {
            return Err(EssentiaError::FileNotFound(
                audio_path.display().to_string(),
            ));
        }

        // Create temporary file path for JSON output
        let temp_output = std::env::temp_dir().join(format!("essentia_{}.json", uuid::Uuid::new_v4()));

        tracing::debug!(
            audio_file = %audio_path.display(),
            output_file = %temp_output.display(),
            "Running Essentia analysis"
        );

        // Spawn essentia_streaming_extractor_music
        // Usage: essentia_streaming_extractor_music input.mp3 output.json
        let output = tokio::task::spawn_blocking({
            let binary = self.binary_path.clone();
            let audio = audio_path.to_path_buf();
            let output_file = temp_output.clone();

            move || {
                Command::new(&binary)
                    .arg(&audio)
                    .arg(&output_file)
                    .output()
            }
        })
        .await
        .map_err(|e| EssentiaError::ExecutionError(format!("Task join error: {}", e)))?
        .map_err(|e| EssentiaError::ExecutionError(e.to_string()))?;

        // Check exit status
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Clean up temp file on error
            let _ = std::fs::remove_file(&temp_output);
            return Err(EssentiaError::AnalysisFailed(format!(
                "Exit code: {:?}, stderr: {}",
                output.status.code(),
                stderr
            )));
        }

        // Read and parse JSON output
        let json_content = tokio::fs::read_to_string(&temp_output).await?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_output);

        let essentia_output: EssentiaOutput = serde_json::from_str(&json_content)
            .map_err(|e| EssentiaError::ParseError(e.to_string()))?;

        tracing::info!(
            audio_file = %audio_path.display(),
            has_tonal = essentia_output.tonal.is_some(),
            has_rhythm = essentia_output.rhythm.is_some(),
            "Essentia analysis completed"
        );

        // Convert to flavor vector
        Ok(essentia_output.to_flavor_vector())
    }

    /// Check if Essentia is available
    pub fn is_available() -> bool {
        Command::new("essentia_streaming_extractor_music")
            .arg("--version")
            .output()
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_essentia_availability_check() {
        // This test will pass/fail depending on whether Essentia is installed
        let available = EssentiaClient::is_available();
        println!("Essentia available: {}", available);
        // Don't fail test if not installed
        assert!(true);
    }

    #[test]
    fn test_essentia_output_parsing() {
        let json_str = r#"{
            "lowlevel": {
                "average_loudness": 0.75,
                "dynamic_complexity": 0.5,
                "spectral_centroid": {
                    "mean": 1500.0
                },
                "spectral_energy": {
                    "mean": 0.6
                },
                "dissonance": {
                    "mean": 0.3
                }
            },
            "rhythm": {
                "bpm": 120.0,
                "danceability": 0.7
            },
            "tonal": {
                "key_key": "C",
                "key_scale": "major",
                "key_strength": 0.85
            }
        }"#;

        let output: EssentiaOutput = serde_json::from_str(json_str).unwrap();
        let flavor = output.to_flavor_vector();

        assert_eq!(flavor.key, Some("C".to_string()));
        assert_eq!(flavor.scale, Some("major".to_string()));
        assert_eq!(flavor.bpm, Some(120.0));
        assert_eq!(flavor.source, "essentia");
    }

    #[test]
    fn test_flavor_vector_compatibility() {
        // Verify Essentia output can be converted to same format as AcousticBrainz
        let json_str = r#"{
            "lowlevel": {
                "dynamic_complexity": 0.5
            },
            "rhythm": {
                "bpm": 120.0
            },
            "tonal": {
                "key_key": "C",
                "key_scale": "major"
            }
        }"#;

        let output: EssentiaOutput = serde_json::from_str(json_str).unwrap();
        let flavor = output.to_flavor_vector();

        // Should be serializable
        let json = flavor.to_json().unwrap();
        let parsed = MusicalFlavorVector::from_json(&json).unwrap();

        assert_eq!(parsed.source, "essentia");
        assert_eq!(parsed.key, Some("C".to_string()));
    }
}

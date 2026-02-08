//! Essentia local analysis client
//!
//! **[AIA-COMP-010]** Local musical flavor extraction using Essentia
//!
//! **[PLAN035]** Strategy pattern: NativeBinary (Linux) or DockerHttp (Windows/Linux fallback).
//! Detection order controlled by `ai_essentia_mode` setting:
//!   - `auto` (default): try native binary first, fall back to Docker
//!   - `native`: native binary only
//!   - `docker`: Docker container only
//!   - `disabled`: skip Essentia entirely

use crate::services::MusicalFlavorVector;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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

    /// Docker container not available
    #[error("Docker not available: {0}")]
    DockerNotAvailable(String),

    /// Docker container failed to start
    #[error("Container start failed: {0}")]
    ContainerStartFailed(String),

    /// HTTP request to Docker container failed
    #[error("HTTP error: {0}")]
    HttpError(String),

    /// Essentia disabled via settings
    #[error("Essentia disabled via ai_essentia_mode setting")]
    Disabled,
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
///
/// Essentia v2.1 outputs key detection as nested objects (`key_edma`, `key_krumhansl`, etc.)
/// rather than flat fields. We use `key_edma` as the primary source (recommended by MTG).
/// Flat `key_key`/`key_scale` fields are kept for backwards compatibility with test data.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EssentiaTonal {
    /// Key detection using EDMA profile (primary)
    pub key_edma: Option<EssentiaKeyResult>,
    /// Flat key field (legacy/test compatibility)
    pub key_key: Option<String>,
    /// Flat scale field (legacy/test compatibility)
    pub key_scale: Option<String>,
    /// Flat key strength field (legacy/test compatibility)
    pub key_strength: Option<f64>,
    /// Predominant chord key
    pub chords_key: Option<String>,
    /// Predominant chord scale
    pub chords_scale: Option<String>,
}

/// Key detection result from Essentia
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EssentiaKeyResult {
    /// Musical key (e.g., "C", "A")
    pub key: Option<String>,
    /// Musical scale (e.g., "major", "minor")
    pub scale: Option<String>,
    /// Detection confidence (0.0-1.0)
    pub strength: Option<f64>,
}

impl EssentiaOutput {
    /// Convert to MusicalFlavorVector
    ///
    /// Prefers `key_edma` (real Essentia output) over flat `key_key`/`key_scale` (test data).
    pub fn to_flavor_vector(&self) -> MusicalFlavorVector {
        // Prefer key_edma (real Essentia) over flat fields (legacy/test)
        let (key, scale, key_strength) = match self.tonal.as_ref() {
            Some(t) => {
                if let Some(ref edma) = t.key_edma {
                    (edma.key.clone(), edma.scale.clone(), edma.strength)
                } else {
                    (t.key_key.clone(), t.key_scale.clone(), t.key_strength)
                }
            }
            None => (None, None, None),
        };

        MusicalFlavorVector {
            key,
            scale,
            key_strength,
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

/// **[PLAN035]** Backend strategy for Essentia analysis
enum EssentiaBackend {
    /// Direct binary execution (Linux with Essentia installed)
    NativeBinary { binary_path: String },
    /// Docker container HTTP API
    DockerHttp {
        http_client: reqwest::Client,
        base_url: String,
        music_root: PathBuf,
    },
}

/// Essentia client with strategy-pattern backend
///
/// **[PLAN035]** Supports native binary (Linux) and Docker HTTP (Windows/Linux fallback).
pub struct EssentiaClient {
    backend: EssentiaBackend,
}

impl EssentiaClient {
    /// Create new Essentia client with auto-detection
    ///
    /// **[PLAN035]** Reads `ai_essentia_mode` from settings to determine backend:
    /// - `auto`: try native → Docker fallback
    /// - `native`: native binary only
    /// - `docker`: Docker container only
    /// - `disabled`: return Err(Disabled)
    pub async fn new(
        db: &sqlx::SqlitePool,
        music_root: PathBuf,
    ) -> Result<Self, EssentiaError> {
        // Read mode from settings
        let mode: String = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'ai_essentia_mode'",
        )
        .fetch_optional(db)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "auto".to_string());

        match mode.as_str() {
            "disabled" => Err(EssentiaError::Disabled),
            "native" => Self::try_native(),
            "docker" => Self::try_docker(db, music_root).await,
            _ => {
                // "auto" or unrecognized: try native first, fall back to Docker
                match Self::try_native() {
                    Ok(client) => Ok(client),
                    Err(native_err) => {
                        tracing::info!(
                            error = ?native_err,
                            "Native Essentia not found, trying Docker backend"
                        );
                        Self::try_docker(db, music_root).await
                    }
                }
            }
        }
    }

    /// Try to create a native binary backend
    fn try_native() -> Result<Self, EssentiaError> {
        let binary_path = "essentia_streaming_extractor_music";
        match Command::new(binary_path).arg("--version").output() {
            Ok(_) => {
                tracing::info!("Essentia native binary found");
                Ok(Self {
                    backend: EssentiaBackend::NativeBinary {
                        binary_path: binary_path.to_string(),
                    },
                })
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(EssentiaError::BinaryNotFound)
            }
            Err(e) => Err(EssentiaError::ExecutionError(e.to_string())),
        }
    }

    /// Try to create a Docker HTTP backend
    async fn try_docker(
        db: &sqlx::SqlitePool,
        music_root: PathBuf,
    ) -> Result<Self, EssentiaError> {
        // Read Docker settings
        let image: String = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'ai_essentia_docker_image'",
        )
        .fetch_optional(db)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "wkmp-essentia:latest".to_string());

        let port: String = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'ai_essentia_docker_port'",
        )
        .fetch_optional(db)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "5780".to_string());

        let container: String = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'ai_essentia_docker_container'",
        )
        .fetch_optional(db)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "wkmp-essentia".to_string());

        let base_url = format!("http://localhost:{}", port);
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(310)) // slightly over server's 300s timeout
            .build()
            .map_err(|e| EssentiaError::HttpError(format!("Failed to create HTTP client: {}", e)))?;

        // Check if container is already running via health check
        if Self::health_check(&http_client, &base_url).await {
            tracing::info!(container = %container, "Essentia Docker container already running");
            return Ok(Self {
                backend: EssentiaBackend::DockerHttp {
                    http_client,
                    base_url,
                    music_root,
                },
            });
        }

        // Try to start existing container
        tracing::info!(container = %container, "Attempting to start Essentia Docker container");
        let start_result = tokio::task::spawn_blocking({
            let container = container.clone();
            move || {
                Command::new("docker")
                    .args(["start", &container])
                    .output()
            }
        })
        .await
        .map_err(|e| EssentiaError::ContainerStartFailed(format!("Task join error: {}", e)))?;

        if start_result.is_ok() {
            // Wait for health check
            if Self::wait_for_health(&http_client, &base_url, 10).await {
                tracing::info!(container = %container, "Essentia Docker container started");
                return Ok(Self {
                    backend: EssentiaBackend::DockerHttp {
                        http_client,
                        base_url,
                        music_root,
                    },
                });
            }
        }

        // Container doesn't exist or failed to start — create new one
        tracing::info!(
            container = %container,
            image = %image,
            "Creating new Essentia Docker container"
        );

        let music_root_str = music_root.display().to_string();
        let volume_mount = format!("{}:/music:ro", music_root_str);
        let port_mapping = format!("{}:5780", port);

        let run_result = tokio::task::spawn_blocking({
            let container = container.clone();
            move || {
                Command::new("docker")
                    .args([
                        "run", "-d",
                        "--name", &container,
                        "-p", &port_mapping,
                        "-v", &volume_mount,
                        &image,
                    ])
                    .output()
            }
        })
        .await
        .map_err(|e| EssentiaError::ContainerStartFailed(format!("Task join error: {}", e)))?
        .map_err(|e| EssentiaError::DockerNotAvailable(format!("Docker not installed or not running: {}", e)))?;

        if !run_result.status.success() {
            let stderr = String::from_utf8_lossy(&run_result.stderr);
            return Err(EssentiaError::ContainerStartFailed(format!(
                "docker run failed: {}",
                stderr.trim()
            )));
        }

        // Wait for container to become healthy
        if Self::wait_for_health(&http_client, &base_url, 15).await {
            tracing::info!(container = %container, "Essentia Docker container created and running");
            Ok(Self {
                backend: EssentiaBackend::DockerHttp {
                    http_client,
                    base_url,
                    music_root,
                },
            })
        } else {
            Err(EssentiaError::ContainerStartFailed(
                "Container started but health check timed out after 15s".to_string(),
            ))
        }
    }

    /// Single health check request
    async fn health_check(client: &reqwest::Client, base_url: &str) -> bool {
        let url = format!("{}/health", base_url);
        match client
            .get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Poll health endpoint with timeout
    async fn wait_for_health(
        client: &reqwest::Client,
        base_url: &str,
        timeout_secs: u64,
    ) -> bool {
        let deadline = tokio::time::Instant::now()
            + tokio::time::Duration::from_secs(timeout_secs);

        while tokio::time::Instant::now() < deadline {
            if Self::health_check(client, base_url).await {
                return true;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        false
    }

    /// Analyze audio file and extract musical flavor
    ///
    /// **[AIA-COMP-010]** Local Essentia analysis as fallback.
    /// Dispatches to native binary or Docker HTTP depending on backend.
    pub async fn analyze_file(
        &self,
        audio_path: &Path,
    ) -> Result<MusicalFlavorVector, EssentiaError> {
        match &self.backend {
            EssentiaBackend::NativeBinary { binary_path } => {
                self.analyze_native(audio_path, binary_path).await
            }
            EssentiaBackend::DockerHttp {
                http_client,
                base_url,
                music_root,
            } => {
                self.analyze_docker(audio_path, http_client, base_url, music_root)
                    .await
            }
        }
    }

    /// Native binary analysis (original implementation)
    async fn analyze_native(
        &self,
        audio_path: &Path,
        binary_path: &str,
    ) -> Result<MusicalFlavorVector, EssentiaError> {
        if !audio_path.exists() {
            return Err(EssentiaError::FileNotFound(
                audio_path.display().to_string(),
            ));
        }

        let temp_output =
            std::env::temp_dir().join(format!("essentia_{}.json", uuid::Uuid::new_v4()));

        tracing::debug!(
            audio_file = %audio_path.display(),
            output_file = %temp_output.display(),
            "Running Essentia native analysis"
        );

        let output = tokio::task::spawn_blocking({
            let binary = binary_path.to_string();
            let audio = audio_path.to_path_buf();
            let output_file = temp_output.clone();
            move || Command::new(&binary).arg(&audio).arg(&output_file).output()
        })
        .await
        .map_err(|e| EssentiaError::ExecutionError(format!("Task join error: {}", e)))?
        .map_err(|e| EssentiaError::ExecutionError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = std::fs::remove_file(&temp_output);
            return Err(EssentiaError::AnalysisFailed(format!(
                "Exit code: {:?}, stderr: {}",
                output.status.code(),
                stderr
            )));
        }

        let json_content = tokio::fs::read_to_string(&temp_output).await?;
        let _ = std::fs::remove_file(&temp_output);

        Self::parse_essentia_output(&json_content, audio_path)
    }

    /// Docker HTTP analysis
    async fn analyze_docker(
        &self,
        audio_path: &Path,
        http_client: &reqwest::Client,
        base_url: &str,
        music_root: &Path,
    ) -> Result<MusicalFlavorVector, EssentiaError> {
        if !audio_path.exists() {
            return Err(EssentiaError::FileNotFound(
                audio_path.display().to_string(),
            ));
        }

        // Translate host path to container path
        let container_path = Self::translate_path(audio_path, music_root)?;

        tracing::debug!(
            audio_file = %audio_path.display(),
            container_path = %container_path,
            "Running Essentia Docker analysis"
        );

        let url = format!("{}/analyze", base_url);
        let body = serde_json::json!({ "file_path": container_path });

        let response = http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| EssentiaError::HttpError(format!("Request failed: {}", e)))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| EssentiaError::HttpError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(EssentiaError::AnalysisFailed(format!(
                "Docker API returned {}: {}",
                status,
                &response_text[..response_text.len().min(500)]
            )));
        }

        Self::parse_essentia_output(&response_text, audio_path)
    }

    /// Translate host file path to Docker container path (/music/...)
    fn translate_path(host_path: &Path, music_root: &Path) -> Result<String, EssentiaError> {
        let relative = host_path.strip_prefix(music_root).map_err(|_| {
            EssentiaError::FileNotFound(format!(
                "Audio file {} is not under music root {}",
                host_path.display(),
                music_root.display()
            ))
        })?;
        Ok(format!(
            "/music/{}",
            relative.to_string_lossy().replace('\\', "/")
        ))
    }

    /// Parse Essentia JSON output into a MusicalFlavorVector
    fn parse_essentia_output(
        json_content: &str,
        audio_path: &Path,
    ) -> Result<MusicalFlavorVector, EssentiaError> {
        let essentia_output: EssentiaOutput = serde_json::from_str(json_content)
            .map_err(|e| EssentiaError::ParseError(e.to_string()))?;

        tracing::info!(
            audio_file = %audio_path.display(),
            has_tonal = essentia_output.tonal.is_some(),
            has_rhythm = essentia_output.rhythm.is_some(),
            "Essentia analysis completed"
        );

        Ok(essentia_output.to_flavor_vector())
    }

    /// Check if Essentia native binary is available
    pub fn is_available() -> bool {
        Command::new("essentia_streaming_extractor_music")
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Return a description of the active backend
    pub fn backend_name(&self) -> &'static str {
        match &self.backend {
            EssentiaBackend::NativeBinary { .. } => "native",
            EssentiaBackend::DockerHttp { .. } => "docker",
        }
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

    #[test]
    fn test_translate_path_unix() {
        let music_root = Path::new("/home/user/Music");
        let audio_path = Path::new("/home/user/Music/Artist/Album/track.mp3");
        let result = EssentiaClient::translate_path(audio_path, music_root).unwrap();
        assert_eq!(result, "/music/Artist/Album/track.mp3");
    }

    #[test]
    fn test_translate_path_outside_root() {
        let music_root = Path::new("/home/user/Music");
        let audio_path = Path::new("/tmp/other/track.mp3");
        let result = EssentiaClient::translate_path(audio_path, music_root);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_essentia_output_fn() {
        let json_str = r#"{
            "rhythm": { "bpm": 140.0 },
            "tonal": { "key_key": "A", "key_scale": "minor" }
        }"#;
        let flavor =
            EssentiaClient::parse_essentia_output(json_str, Path::new("/test.mp3")).unwrap();
        assert_eq!(flavor.bpm, Some(140.0));
        assert_eq!(flavor.key, Some("A".to_string()));
        assert_eq!(flavor.scale, Some("minor".to_string()));
    }
}

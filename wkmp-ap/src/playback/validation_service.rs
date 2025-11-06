//! Automatic Pipeline Validation Service
//!
//! Runs periodic validation of the audio pipeline during playback,
//! checking conservation laws and emitting events on validation results.
//!
//! **Traceability:**
//! - **[ARCH-AUTO-VAL-001]** Automatic validation and tuning architecture
//! - **[PHASE1-INTEGRITY]** Pipeline integrity validation

use crate::playback::engine::PlaybackEngine;
use crate::state::{PlaybackState, SharedState};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn, trace};
use wkmp_common::events::WkmpEvent;

/// Validation service configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Validation interval in seconds (default: 10s)
    pub interval_secs: u64,

    /// Sample tolerance for validation (default: 8192 samples)
    pub tolerance_samples: u64,

    /// Enable automatic validation (default: true)
    pub enabled: bool,

    /// Maximum history entries to keep (default: 100)
    pub history_size: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            interval_secs: 10,
            tolerance_samples: 8192,
            enabled: true,
            history_size: 100,
        }
    }
}

impl ValidationConfig {
    /// Load validation configuration from database settings
    ///
    /// **[ARCH-AUTO-VAL-001]** Loads settings from database, falls back to defaults
    ///
    /// # Arguments
    /// * `db_pool` - Database connection pool
    ///
    /// # Returns
    /// ValidationConfig loaded from database settings
    pub async fn from_database(db_pool: &Pool<Sqlite>) -> Self {
        let mut config = Self::default();

        // Load validation_enabled
        if let Ok(enabled_str) = sqlx::query_scalar::<_, String>(
            "SELECT value FROM settings WHERE key = 'validation_enabled'"
        )
        .fetch_one(db_pool)
        .await
        {
            config.enabled = enabled_str.to_lowercase() == "true";
        }

        // Load validation_interval_secs
        if let Ok(interval_str) = sqlx::query_scalar::<_, String>(
            "SELECT value FROM settings WHERE key = 'validation_interval_secs'"
        )
        .fetch_one(db_pool)
        .await
        {
            if let Ok(interval) = interval_str.parse::<u64>() {
                config.interval_secs = interval;
            }
        }

        // Load validation_tolerance_samples
        if let Ok(tolerance_str) = sqlx::query_scalar::<_, String>(
            "SELECT value FROM settings WHERE key = 'validation_tolerance_samples'"
        )
        .fetch_one(db_pool)
        .await
        {
            if let Ok(tolerance) = tolerance_str.parse::<u64>() {
                config.tolerance_samples = tolerance;
            }
        }

        config
    }
}

/// Validation history entry
#[derive(Debug, Clone)]
pub struct ValidationHistoryEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub passed: bool,
    pub passage_count: usize,
    pub total_decoder_samples: u64,
    pub total_buffer_written: u64,
    pub total_buffer_read: u64,
    pub total_mixer_frames: u64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Validation Service
///
/// Runs periodic validation checks during playback and emits events
pub struct ValidationService {
    config: ValidationConfig,
    engine: Arc<PlaybackEngine>,
    state: Arc<SharedState>,
    history: Arc<tokio::sync::RwLock<Vec<ValidationHistoryEntry>>>,
}

impl ValidationService {
    /// Create a new validation service
    pub fn new(
        config: ValidationConfig,
        engine: Arc<PlaybackEngine>,
        state: Arc<SharedState>,
    ) -> Self {
        Self {
            config,
            engine,
            state,
            history: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// Get validation history (most recent first)
    ///
    /// **Phase 4:** History API reserved for diagnostics UI (validation service not yet enabled)
    pub async fn get_history(&self) -> Vec<ValidationHistoryEntry> {
        let history = self.history.read().await;
        history.clone()
    }

    /// Get the most recent validation result
    ///
    /// **Phase 4:** Latest result API reserved for diagnostics UI (validation service not yet enabled)
    pub async fn get_latest(&self) -> Option<ValidationHistoryEntry> {
        let history = self.history.read().await;
        history.first().cloned()
    }

    /// Run the validation service (spawns background task)
    ///
    /// This function spawns a background task that runs periodic validation
    /// checks during playback. The task continues until the service is dropped.
    pub fn run(self: Arc<Self>) {
        if !self.config.enabled {
            info!("ValidationService disabled by configuration");
            return;
        }

        info!(
            "Starting ValidationService (interval: {}s, tolerance: {} samples)",
            self.config.interval_secs, self.config.tolerance_samples
        );

        tokio::spawn(async move {
            let mut timer = interval(Duration::from_secs(self.config.interval_secs));
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                timer.tick().await;

                // Only validate during active playback
                let is_playing = {
                    let state = self.state.playback_state.read().await;
                    *state == PlaybackState::Playing
                };

                if !is_playing {
                    debug!("ValidationService: Skipping validation (not playing)");
                    continue;
                }

                // Perform validation check
                if let Err(e) = self.validate_and_emit().await {
                    error!("ValidationService: Validation check failed: {}", e);
                }
            }
        });
    }

    /// Perform a single validation check and emit event
    async fn validate_and_emit(&self) -> Result<(), String> {
        debug!("ValidationService: Running validation check");

        // Collect pipeline metrics
        let metrics = self.engine.get_pipeline_metrics().await;

        // Run validation
        let validation = metrics.validate(self.config.tolerance_samples);

        // Calculate totals
        let passage_count = metrics.passages.len();
        let total_decoder_samples: u64 = metrics
            .passages
            .values()
            .map(|p| p.decoder_frames_pushed as u64 * 2)
            .sum();
        let total_buffer_written: u64 = metrics
            .passages
            .values()
            .map(|p| p.buffer_samples_written)
            .sum();
        let total_buffer_read: u64 = metrics
            .passages
            .values()
            .map(|p| p.buffer_samples_read)
            .sum();
        let total_mixer_frames = metrics.mixer_total_frames_mixed;

        // Format errors and warnings
        let errors: Vec<String> = validation.errors.iter().map(|e| e.format()).collect();
        let mut warnings = Vec::new();

        // Check if approaching tolerance (>80%)
        let tolerance_threshold = (self.config.tolerance_samples as f64 * 0.8) as u64;
        for error in &validation.errors {
            if let Some(discrepancy) = error.discrepancy() {
                if discrepancy > tolerance_threshold {
                    warnings.push(format!(
                        "Approaching tolerance threshold: {} samples ({}% of limit)",
                        discrepancy,
                        (discrepancy as f64 / self.config.tolerance_samples as f64 * 100.0) as u64
                    ));
                }
            }
        }

        // Create history entry
        let history_entry = ValidationHistoryEntry {
            timestamp: chrono::Utc::now(),
            passed: validation.passed(),
            passage_count,
            total_decoder_samples,
            total_buffer_written,
            total_buffer_read,
            total_mixer_frames,
            errors: errors.clone(),
            warnings: warnings.clone(),
        };

        // Add to history (keep only last N entries)
        {
            let mut history = self.history.write().await;
            history.insert(0, history_entry.clone()); // Insert at front (most recent first)
            if history.len() > self.config.history_size {
                history.truncate(self.config.history_size);
            }
        }

        // Check if audio output is expected (Playing with non-empty queue)
        // When false (Paused or empty queue), validation failures/warnings are expected
        let audio_expected = self.engine.is_audio_expected();

        // Emit appropriate event
        let timestamp = chrono::Utc::now();
        let event = if validation.passed() {
            debug!(
                "ValidationService: PASS (passages: {}, errors: 0)",
                passage_count
            );
            WkmpEvent::ValidationSuccess {
                timestamp,
                passage_count,
                total_decoder_samples,
                total_buffer_written,
                total_buffer_read,
                total_mixer_frames,
            }
        } else if !warnings.is_empty() {
            if !audio_expected {
                // Idle/Paused state: warnings are expected (nothing to validate)
                trace!(
                    "ValidationService: WARNING during idle (passages: {}, warnings: {})",
                    passage_count,
                    warnings.len()
                );
            } else {
                warn!(
                    "ValidationService: WARNING (passages: {}, warnings: {})",
                    passage_count,
                    warnings.len()
                );
            }
            WkmpEvent::ValidationWarning {
                timestamp,
                passage_count,
                total_decoder_samples,
                total_buffer_written,
                total_buffer_read,
                total_mixer_frames,
                warnings,
            }
        } else {
            if !audio_expected {
                // Idle/Paused state: validation failures are expected (nothing to validate)
                trace!(
                    "ValidationService: FAIL during idle (passages: {}, errors: {})",
                    passage_count,
                    errors.len()
                );
            } else {
                error!(
                    "ValidationService: FAIL (passages: {}, errors: {})",
                    passage_count,
                    errors.len()
                );
            }
            WkmpEvent::ValidationFailure {
                timestamp,
                passage_count,
                total_decoder_samples,
                total_buffer_written,
                total_buffer_read,
                total_mixer_frames,
                errors,
            }
        };

        // Broadcast event
        if let Err(e) = self.state.event_tx.send(event) {
            debug!("ValidationService: Failed to broadcast event (no receivers): {}", e);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_config_defaults() {
        let config = ValidationConfig::default();
        assert_eq!(config.interval_secs, 10);
        assert_eq!(config.tolerance_samples, 8192);
        assert!(config.enabled);
        assert_eq!(config.history_size, 100);
    }

    #[test]
    fn test_validation_history_entry_creation() {
        let entry = ValidationHistoryEntry {
            timestamp: chrono::Utc::now(),
            passed: true,
            passage_count: 1,
            total_decoder_samples: 1000,
            total_buffer_written: 1000,
            total_buffer_read: 500,
            total_mixer_frames: 250,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        assert!(entry.passed);
        assert_eq!(entry.passage_count, 1);
        assert!(entry.errors.is_empty());
        assert!(entry.warnings.is_empty());
    }
}

# WKMP Import Parameter Management

**⚙️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines parameter storage and management for audio import. Derived from [SPEC024](SPEC024-audio_ingest_architecture.md) and [SPEC025](SPEC025-amplitude_analysis.md).

> **Related:** [Audio Ingest Architecture](SPEC024-audio_ingest_architecture.md) | [Amplitude Analysis](SPEC025-amplitude_analysis.md) | [Database Schema](IMPL001-database_schema.md)

---

## Overview

**Storage Model:** Hybrid approach
- **Global defaults:** JSON blob in `settings` table
- **Per-passage overrides:** JSON column in `passages` table

**Precedence:** Per-passage parameters override global defaults

---

## Database Schema

### Settings Table (Global Parameters)

```sql
-- Add to existing settings table
INSERT INTO settings (key, value_type, value_text)
VALUES ('import_parameters', 'json', '{
  "rms_window_ms": 100,
  "lead_in_threshold_db": -12.0,
  "lead_out_threshold_db": -12.0,
  "quick_ramp_threshold": 0.75,
  "quick_ramp_duration_s": 1.0,
  "max_lead_in_duration_s": 5.0,
  "max_lead_out_duration_s": 5.0,
  "apply_a_weighting": true,
  "silence_threshold_db": -60.0,
  "min_silence_duration_ms": 500,
  "import_parallelism": 4
}');
```

### Passages Table (Per-Passage Metadata)

```sql
-- Add columns to passages table
ALTER TABLE passages ADD COLUMN import_metadata TEXT;
ALTER TABLE passages ADD COLUMN additional_metadata TEXT;
```

**import_metadata format:**
```json
{
  "amplitude_analysis": {
    "peak_rms": 0.95,
    "lead_in_detected_s": 2.3,
    "lead_out_detected_s": 3.2,
    "quick_ramp_up": false,
    "quick_ramp_down": false,
    "parameters_used": {
      "rms_window_ms": 100,
      "lead_in_threshold_db": -12.0
    },
    "analyzed_at": "2025-10-27T12:34:56Z"
  }
}
```

**additional_metadata format:**
```json
{
  "seasonal_holiday": 0.0,
  "profanity_level": 0.0,
  "energy_level": 0.85,
  "danceability": 0.72
}
```

---

## Rust Implementation

```rust
// wkmp-ai/src/services/parameter_manager.rs

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportParameters {
    // Amplitude analysis
    pub rms_window_ms: u32,
    pub lead_in_threshold_db: f32,
    pub lead_out_threshold_db: f32,
    pub quick_ramp_threshold: f32,
    pub quick_ramp_duration_s: f32,
    pub max_lead_in_duration_s: f32,
    pub max_lead_out_duration_s: f32,
    pub apply_a_weighting: bool,
    
    // Silence detection
    pub silence_threshold_db: f32,
    pub min_silence_duration_ms: u32,
    
    // Import workflow
    pub import_parallelism: u32,
}

impl Default for ImportParameters {
    fn default() -> Self {
        Self {
            rms_window_ms: 100,
            lead_in_threshold_db: -12.0,
            lead_out_threshold_db: -12.0,
            quick_ramp_threshold: 0.75,
            quick_ramp_duration_s: 1.0,
            max_lead_in_duration_s: 5.0,
            max_lead_out_duration_s: 5.0,
            apply_a_weighting: true,
            silence_threshold_db: -60.0,
            min_silence_duration_ms: 500,
            import_parallelism: 4,
        }
    }
}

pub struct ParameterManager {
    db: SqlitePool,
}

impl ParameterManager {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
    
    /// Load global import parameters
    pub async fn load_global(&self) -> Result<ImportParameters, sqlx::Error> {
        let row: (String,) = sqlx::query_as(
            "SELECT value_text FROM settings WHERE key = 'import_parameters'"
        )
        .fetch_one(&self.db)
        .await?;
        
        let params: ImportParameters = serde_json::from_str(&row.0)
            .unwrap_or_default();
        
        Ok(params)
    }
    
    /// Save global import parameters
    pub async fn save_global(&self, params: &ImportParameters) -> Result<(), sqlx::Error> {
        let json = serde_json::to_string(params)
            .map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
        
        sqlx::query(
            "INSERT INTO settings (key, value_type, value_text)
             VALUES ('import_parameters', 'json', ?)
             ON CONFLICT(key) DO UPDATE SET value_text = excluded.value_text"
        )
        .bind(&json)
        .execute(&self.db)
        .await?;
        
        Ok(())
    }
    
    /// Update specific parameters (partial update)
    pub async fn update_global(&self, updates: serde_json::Value) -> Result<(), sqlx::Error> {
        let mut params = self.load_global().await?;
        
        // Merge updates into existing parameters
        if let serde_json::Value::Object(map) = updates {
            let mut params_json = serde_json::to_value(&params).unwrap();
            if let serde_json::Value::Object(ref mut params_map) = params_json {
                for (key, value) in map {
                    params_map.insert(key, value);
                }
            }
            params = serde_json::from_value(params_json).unwrap();
        }
        
        self.save_global(&params).await
    }
}
```

---

## Parameter Validation

```rust
impl ImportParameters {
    /// Validate parameter ranges
    pub fn validate(&self) -> Result<(), ParameterError> {
        if self.rms_window_ms < 10 || self.rms_window_ms > 1000 {
            return Err(ParameterError::OutOfRange("rms_window_ms", 10, 1000));
        }
        
        if self.lead_in_threshold_db < -60.0 || self.lead_in_threshold_db > 0.0 {
            return Err(ParameterError::OutOfRange("lead_in_threshold_db", -60.0, 0.0));
        }
        
        if self.quick_ramp_threshold < 0.0 || self.quick_ramp_threshold > 1.0 {
            return Err(ParameterError::OutOfRange("quick_ramp_threshold", 0.0, 1.0));
        }
        
        if self.import_parallelism < 1 || self.import_parallelism > 16 {
            return Err(ParameterError::OutOfRange("import_parallelism", 1, 16));
        }
        
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParameterError {
    #[error("Parameter {0} out of range: must be between {1} and {2}")]
    OutOfRange(&'static str, f32, f32),
}
```

---

## Presets

```rust
impl ImportParameters {
    /// Classical music preset
    pub fn classical() -> Self {
        Self {
            rms_window_ms: 200,
            lead_in_threshold_db: -15.0,
            lead_out_threshold_db: -15.0,
            quick_ramp_duration_s: 2.0,
            max_lead_in_duration_s: 5.0,
            max_lead_out_duration_s: 5.0,
            silence_threshold_db: -80.0,
            ..Default::default()
        }
    }
    
    /// Rock/pop preset
    pub fn rock_pop() -> Self {
        Self {
            rms_window_ms: 50,
            lead_in_threshold_db: -8.0,
            lead_out_threshold_db: -8.0,
            quick_ramp_duration_s: 0.5,
            max_lead_in_duration_s: 2.0,
            max_lead_out_duration_s: 2.0,
            silence_threshold_db: -60.0,
            ..Default::default()
        }
    }
    
    /// Electronic/ambient preset
    pub fn electronic() -> Self {
        Self {
            rms_window_ms: 250,
            lead_in_threshold_db: -18.0,
            lead_out_threshold_db: -18.0,
            quick_ramp_duration_s: 3.0,
            max_lead_in_duration_s: 8.0,
            max_lead_out_duration_s: 8.0,
            silence_threshold_db: -70.0,
            ..Default::default()
        }
    }
}
```

---

**Document Version:** 1.0
**Last Updated:** 2025-10-27

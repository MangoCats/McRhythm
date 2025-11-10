# WKMP Import Parameter Management

**⚙️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines parameter storage and management for audio import. Derived from [SPEC032](SPEC032-audio_ingest_architecture.md) and [SPEC025](SPEC025-amplitude_analysis.md).

> **Related:** [Audio Ingest Architecture](SPEC032-audio_ingest_architecture.md) | [Amplitude Analysis](SPEC025-amplitude_analysis.md) | [Database Schema](IMPL001-database_schema.md)

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
  "import_parallelism": 4,
  "acoustid_rate_limit_ms": 400,
  "musicbrainz_rate_limit_ms": 1200,
  "chromaprint_fingerprint_duration_seconds": 120,
  "expected_musical_flavor_characteristics": 50,
  "import_success_confidence_threshold": 0.75,
  "metadata_confidence_threshold": 0.66,
  "max_reimport_attempts": 3
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

    // Audio Import (PLAN024 - Ground-up recode)
    // [PARAM-AI-001] AcoustID API rate limiting
    pub acoustid_rate_limit_ms: u32,

    // [PARAM-AI-002] MusicBrainz API rate limiting
    pub musicbrainz_rate_limit_ms: u32,

    // [PARAM-AI-003] Chromaprint fingerprint duration
    pub chromaprint_fingerprint_duration_seconds: u32,

    // [PARAM-AI-004] Expected musical flavor characteristics count
    pub expected_musical_flavor_characteristics: u32,

    // [PARAM-AI-005] Import success confidence threshold (Amendment 8)
    pub import_success_confidence_threshold: f32,

    // [PARAM-AI-006] Metadata confidence threshold (Amendment 8)
    pub metadata_confidence_threshold: f32,

    // [PARAM-AI-007] Maximum reimport attempts (Amendment 8)
    pub max_reimport_attempts: u32,
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
            acoustid_rate_limit_ms: 400,
            musicbrainz_rate_limit_ms: 1200,
            chromaprint_fingerprint_duration_seconds: 120,
            expected_musical_flavor_characteristics: 50,
            import_success_confidence_threshold: 0.75,
            metadata_confidence_threshold: 0.66,
            max_reimport_attempts: 3,
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

        // PLAN024 Audio Import parameters
        if self.acoustid_rate_limit_ms < 100 || self.acoustid_rate_limit_ms > 10000 {
            return Err(ParameterError::OutOfRange("acoustid_rate_limit_ms", 100, 10000));
        }

        if self.musicbrainz_rate_limit_ms < 500 || self.musicbrainz_rate_limit_ms > 10000 {
            return Err(ParameterError::OutOfRange("musicbrainz_rate_limit_ms", 500, 10000));
        }

        if self.chromaprint_fingerprint_duration_seconds < 10
            || self.chromaprint_fingerprint_duration_seconds > 300
        {
            return Err(ParameterError::OutOfRange(
                "chromaprint_fingerprint_duration_seconds",
                10,
                300,
            ));
        }

        if self.expected_musical_flavor_characteristics < 1
            || self.expected_musical_flavor_characteristics > 200
        {
            return Err(ParameterError::OutOfRange(
                "expected_musical_flavor_characteristics",
                1,
                200,
            ));
        }

        if self.import_success_confidence_threshold < 0.0
            || self.import_success_confidence_threshold > 1.0
        {
            return Err(ParameterError::OutOfRange(
                "import_success_confidence_threshold",
                0.0,
                1.0,
            ));
        }

        if self.metadata_confidence_threshold < 0.0 || self.metadata_confidence_threshold > 1.0 {
            return Err(ParameterError::OutOfRange(
                "metadata_confidence_threshold",
                0.0,
                1.0,
            ));
        }

        if self.max_reimport_attempts < 1 || self.max_reimport_attempts > 10 {
            return Err(ParameterError::OutOfRange("max_reimport_attempts", 1, 10));
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

## Audio Import Parameters (PLAN024)

The following parameters support the ground-up WKMP-AI recode per PLAN024:

### [PARAM-AI-001] acoustid_rate_limit_ms
- **Type:** `u32` (milliseconds)
- **Default:** 400
- **Valid Range:** 100-10000
- **Purpose:** Rate limit for AcoustID API requests
- **Rationale:** AcoustID allows 3 requests/second (333ms). Default 400ms includes 20% safety margin to prevent API throttling.
- **Implementation:** `wkmp-ai/src/extractors/acoustid_client.rs`

### [PARAM-AI-002] musicbrainz_rate_limit_ms
- **Type:** `u32` (milliseconds)
- **Default:** 1200
- **Valid Range:** 500-10000
- **Purpose:** Rate limit for MusicBrainz API requests
- **Rationale:** MusicBrainz allows 1 request/second (1000ms). Default 1200ms includes 20% safety margin. Server returns HTTP 503 if exceeded.
- **Implementation:** `wkmp-ai/src/extractors/musicbrainz_client.rs`

### [PARAM-AI-003] chromaprint_fingerprint_duration_seconds
- **Type:** `u32` (seconds)
- **Default:** 120
- **Valid Range:** 10-300
- **Purpose:** Duration of audio to fingerprint with Chromaprint for AcoustID lookup
- **Rationale:** AcoustID recommends 30-120 seconds for optimal accuracy. Use full passage if shorter than this value. Minimum 10 seconds (reduced accuracy).
- **Implementation:** IMPL015-chromaprint_integration.md

### [PARAM-AI-004] expected_musical_flavor_characteristics
- **Type:** `u32` (count)
- **Default:** 50
- **Valid Range:** 1-200
- **Purpose:** Expected count of musical flavor characteristics for completeness scoring
- **Formula:** `completeness = (present_characteristics / expected_characteristics) × 100%`
- **Update Mechanism:** Manual database UPDATE during early implementation testing based on actual feature counts
- **Implementation:** Completeness scoring in FlavorSynthesizer (Tier 2)

### [PARAM-AI-005] import_success_confidence_threshold
- **Type:** `f32` (0.0-1.0)
- **Default:** 0.75
- **Valid Range:** 0.0-1.0
- **Purpose:** Minimum confidence threshold for file import success (Amendment 8)
- **Behavior:** Files with `import_success_confidence < 0.75` trigger low-confidence flagging
- **Implementation:** File-level import tracking, skip logic (Phase -1)

### [PARAM-AI-006] metadata_confidence_threshold
- **Type:** `f32` (0.0-1.0)
- **Default:** 0.66
- **Valid Range:** 0.0-1.0
- **Purpose:** Minimum confidence threshold for metadata acceptance (Amendment 8)
- **Behavior:** Files with `metadata_confidence < 0.66` may be reimported if threshold increased
- **Implementation:** Metadata merge algorithm, skip logic (Phase -1)

### [PARAM-AI-007] max_reimport_attempts
- **Type:** `u32` (count)
- **Default:** 3
- **Valid Range:** 1-10
- **Purpose:** Maximum reimport attempts per file (Amendment 8)
- **Behavior:** Prevents infinite reimport loops. Files with `reimport_attempt_count >= 3` skip reimport.
- **Implementation:** Skip logic (Phase -1), `files.reimport_attempt_count` tracking

---

**Document Version:** 2.0
**Last Updated:** 2025-11-09
**Changes:**
- v2.0: Added 7 Audio Import parameters (PARAM-AI-001 through PARAM-AI-007) for PLAN024
- v1.0: Initial version with amplitude analysis parameters

# WKMP AcoustID Client Implementation

**⚙️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines Rust implementation for Chromaprint fingerprinting and AcoustID API client. Derived from [SPEC032](SPEC032-audio_ingest_architecture.md) and [SPEC008](SPEC008-library_management.md).

> **Related:** [Audio Ingest Architecture](SPEC032-audio_ingest_architecture.md) | [Library Management](SPEC008-library_management.md) | [MusicBrainz Client](IMPL011-musicbrainz_client.md)

---

## Overview

**Module:** `wkmp-ai/src/services/fingerprinter.rs` + `acoustid_client.rs`
**Purpose:** Generate audio fingerprints and lookup MusicBrainz Recording MBIDs
**Rate Limit:** 3 requests/second (AcoustID)
**Dependencies:** chromaprint library, symphonia, rubato

---

## Build Requirements

### LLVM/Clang Dependency

**Required for:** Building `chromaprint-sys-next` crate (Rust bindings to Chromaprint C library)

The `chromaprint-sys-next` crate uses `bindgen` to generate Rust bindings from Chromaprint C headers. This requires LLVM/Clang to be installed on the build system.

**Installation:**

- **Windows:** Download and install LLVM from https://releases.llvm.org/
  - Install LLVM with "Add LLVM to system PATH" option enabled
  - Alternatively, set `LIBCLANG_PATH` environment variable to point to `libclang.dll`

- **Linux:** Install via package manager
  ```bash
  # Debian/Ubuntu
  sudo apt-get install llvm-dev libclang-dev clang

  # Fedora/RHEL
  sudo dnf install llvm-devel clang-devel
  ```

- **macOS:** Install via Homebrew
  ```bash
  brew install llvm
  ```

**Verification:**
```bash
clang --version  # Should show LLVM version
```

**Note:** LLVM is only required at build time. The compiled wkmp-ai binary does not require LLVM to run (Chromaprint is statically linked).

---

## Chromaprint Integration

### Audio Processing Pipeline

```
Audio File (any format)
    ↓
Symphonia Decode → PCM samples
    ↓
Rubato Resample → 44.1kHz mono
    ↓
Chromaprint → Fingerprint (u32 array)
    ↓
Chromaprint Encode → Base64 string
```

### Rust Implementation

```rust
// wkmp-ai/src/services/fingerprinter.rs

use symphonia::core::formats::FormatReader;
use symphonia::core::io::MediaSourceStream;
use symphonia::default::get_probe;
use rubato::{Resampler, SincFixedIn, InterpolationType, InterpolationParameters, WindowFunction};
use std::path::Path;

/// Audio fingerprinter using Chromaprint
pub struct Fingerprinter {
    target_sample_rate: u32,  // 44100 Hz for Chromaprint
}

impl Fingerprinter {
    pub fn new() -> Self {
        Self {
            target_sample_rate: 44100,
        }
    }

    /// Generate Chromaprint fingerprint from audio file
    pub fn fingerprint_file(&self, file_path: &Path) -> Result<String, FingerprintError> {
        // 1. Decode audio to PCM
        let samples = self.decode_audio(file_path)?;

        // 2. Ensure 44.1kHz mono
        let resampled = self.resample_to_44100(&samples)?;

        // 3. Generate fingerprint
        let fingerprint = self.generate_fingerprint(&resampled)?;

        Ok(fingerprint)
    }

    /// Decode audio file to PCM samples
    fn decode_audio(&self, file_path: &Path) -> Result<AudioData, FingerprintError> {
        let file = std::fs::File::open(file_path)
            .map_err(|e| FingerprintError::IoError(e.to_string()))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut format_reader = get_probe()
            .format(&Default::default(), mss, &Default::default(), &Default::default())
            .map_err(|e| FingerprintError::DecodeError(e.to_string()))?
            .format;

        let track = format_reader.default_track()
            .ok_or_else(|| FingerprintError::DecodeError("No audio track".to_string()))?;

        let sample_rate = track.codec_params.sample_rate
            .ok_or_else(|| FingerprintError::DecodeError("No sample rate".to_string()))?;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &Default::default())
            .map_err(|e| FingerprintError::DecodeError(e.to_string()))?;

        let mut samples = Vec::new();

        // Decode up to 120 seconds (Chromaprint recommendation)
        let max_samples = sample_rate as usize * 120;

        while let Ok(packet) = format_reader.next_packet() {
            if samples.len() >= max_samples {
                break;
            }

            let decoded = decoder.decode(&packet)
                .map_err(|e| FingerprintError::DecodeError(e.to_string()))?;

            // Convert to f32 mono
            let pcm_samples = self.convert_to_mono_f32(decoded);
            samples.extend_from_slice(&pcm_samples);
        }

        Ok(AudioData {
            samples,
            sample_rate,
        })
    }

    /// Convert symphonia AudioBuffer to mono f32
    fn convert_to_mono_f32(&self, buffer: symphonia::core::audio::AudioBufferRef) -> Vec<f32> {
        use symphonia::core::audio::Signal;
        use symphonia::core::conv::FromSample;

        let channels = buffer.spec().channels.count();
        let mut mono = Vec::with_capacity(buffer.frames());

        // Mix down to mono by averaging channels
        for frame_idx in 0..buffer.frames() {
            let mut sum = 0.0f32;
            for ch in 0..channels {
                let sample = buffer.chan(ch)[frame_idx];
                sum += f32::from_sample(sample);
            }
            mono.push(sum / channels as f32);
        }

        mono
    }

    /// Resample audio to 44.1kHz if needed
    fn resample_to_44100(&self, audio: &AudioData) -> Result<Vec<f32>, FingerprintError> {
        if audio.sample_rate == self.target_sample_rate {
            return Ok(audio.samples.clone());
        }

        let params = InterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: InterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let mut resampler = SincFixedIn::<f32>::new(
            self.target_sample_rate as f64 / audio.sample_rate as f64,
            2.0,
            params,
            audio.samples.len(),
            1,  // Mono
        ).map_err(|e| FingerprintError::ResampleError(e.to_string()))?;

        let waves_in = vec![audio.samples.clone()];
        let waves_out = resampler.process(&waves_in, None)
            .map_err(|e| FingerprintError::ResampleError(e.to_string()))?;

        Ok(waves_out[0].clone())
    }

    /// Generate Chromaprint fingerprint using low-level FFI bindings
    ///
    /// **SAFETY:** Uses unsafe FFI calls to chromaprint C library.
    /// All FFI calls are wrapped with error checking and proper resource cleanup.
    fn generate_fingerprint(&self, samples: &[f32]) -> Result<String, FingerprintError> {
        use chromaprint_sys_next::*;

        unsafe {
            // Step 1: Allocate Chromaprint context
            let ctx = chromaprint_new(CHROMAPRINT_ALGORITHM_TEST2);
            if ctx.is_null() {
                return Err(FingerprintError::ChromaprintError(
                    "Failed to create Chromaprint context".to_string()
                ));
            }

            // Step 2: Start fingerprinting (44100 Hz, 1 channel)
            let ret = chromaprint_start(ctx, self.target_sample_rate as i32, 1);
            if ret != 1 {
                chromaprint_free(ctx);
                return Err(FingerprintError::ChromaprintError(
                    "chromaprint_start failed".to_string()
                ));
            }

            // Step 3: Convert f32 samples to i16
            let samples_i16: Vec<i16> = samples.iter()
                .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect();

            // Step 4: Feed audio samples to Chromaprint
            let ret = chromaprint_feed(ctx, samples_i16.as_ptr(), samples_i16.len() as i32);
            if ret != 1 {
                chromaprint_free(ctx);
                return Err(FingerprintError::ChromaprintError(
                    "chromaprint_feed failed".to_string()
                ));
            }

            // Step 5: Finish processing
            let ret = chromaprint_finish(ctx);
            if ret != 1 {
                chromaprint_free(ctx);
                return Err(FingerprintError::ChromaprintError(
                    "chromaprint_finish failed".to_string()
                ));
            }

            // Step 6: Get fingerprint as compressed string
            let mut fp_ptr: *mut i8 = std::ptr::null_mut();
            let ret = chromaprint_get_fingerprint(ctx, &mut fp_ptr);
            if ret != 1 || fp_ptr.is_null() {
                chromaprint_free(ctx);
                return Err(FingerprintError::ChromaprintError(
                    "chromaprint_get_fingerprint failed".to_string()
                ));
            }

            // Step 7: Convert C string to Rust String
            let c_str = std::ffi::CStr::from_ptr(fp_ptr);
            let fingerprint = c_str.to_str()
                .map_err(|e| {
                    chromaprint_dealloc(fp_ptr as *mut std::ffi::c_void);
                    chromaprint_free(ctx);
                    FingerprintError::ChromaprintError(format!("UTF-8 conversion failed: {}", e))
                })?
                .to_string();

            // Step 8: Free resources
            chromaprint_dealloc(fp_ptr as *mut std::ffi::c_void);
            chromaprint_free(ctx);

            Ok(fingerprint)
        }
    }
}

struct AudioData {
    samples: Vec<f32>,
    sample_rate: u32,
}
```

---

## Database Schema

### AcoustID Fingerprint Cache

**Purpose:** Cache fingerprint → MBID mappings to reduce API calls and improve re-import performance.

**Expected Performance:** Reduces AcoustID API calls by ~60% on re-import of existing libraries.

**Schema:**

```sql
CREATE TABLE IF NOT EXISTS acoustid_cache (
    fingerprint_hash TEXT PRIMARY KEY,
    mbid TEXT NOT NULL,
    cached_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (length(fingerprint_hash) = 64)  -- SHA-256 hex
);

CREATE INDEX IF NOT EXISTS idx_acoustid_cache_cached_at
    ON acoustid_cache(cached_at);
```

**Columns:**
- `fingerprint_hash` (TEXT, PRIMARY KEY): SHA-256 hash of Chromaprint fingerprint string
  - Rationale: Fingerprints are large (~1-5 KB Base64 strings), hashing saves storage space
  - SHA-256 provides negligible collision probability
  - Always lowercase hexadecimal (64 characters)
- `mbid` (TEXT, NOT NULL): MusicBrainz Recording MBID (UUID format)
- `cached_at` (TEXT, NOT NULL): Timestamp of cache entry creation (ISO 8601 format)
  - Index supports future cache expiration feature

**Implementation Location:** `wkmp-common/src/db/init.rs`

**Pattern:**
```rust
async fn create_acoustid_cache_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(/* schema above */).execute(pool).await?;
    sqlx::query(/* index above */).execute(pool).await?;
    Ok(())
}

// Called from init_database()
pub async fn init_database(db_path: &Path) -> Result<SqlitePool> {
    // ... existing tables ...
    create_acoustid_cache_table(&pool).await?;
    // ... continue
}
```

---

## AcoustID API Client

### API Endpoint

**URL:** `https://api.acoustid.org/v2/lookup`

**Method:** POST (URL-encoded form data)

**Parameters:**
- `client` - API key (required)
- `duration` - Track duration in seconds (required)
- `fingerprint` - Chromaprint fingerprint (required)
- `meta` - Metadata to include: `recordings` (optional)

**Example Request:**
```
POST https://api.acoustid.org/v2/lookup
Content-Type: application/x-www-form-urlencoded

client=YOUR_API_KEY&duration=123&fingerprint=AQADtN...&meta=recordings+recordingids
```

**Response Structure:**
```json
{
  "status": "ok",
  "results": [
    {
      "id": "acoustid-uuid",
      "score": 0.95,
      "recordings": [
        {
          "id": "5e8d5f0b-3f8a-4c7e-9c4b-5e8d5f0b3f8a"
        }
      ]
    }
  ]
}
```

### Rust Implementation

```rust
// wkmp-ai/src/services/acoustid_client.rs

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct AcoustIDResponse {
    pub status: String,
    pub results: Option<Vec<AcoustIDResult>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AcoustIDResult {
    pub id: String,  // AcoustID UUID
    pub score: f64,
    pub recordings: Option<Vec<AcoustIDRecording>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AcoustIDRecording {
    pub id: String,  // MusicBrainz Recording MBID
}

pub struct AcoustIDClient {
    http_client: reqwest::Client,
    api_key: String,
    db: SqlitePool,
}

impl AcoustIDClient {
    pub fn new(api_key: String, db: SqlitePool) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            api_key,
            db,
        }
    }

    /// Lookup MusicBrainz MBID from fingerprint
    pub async fn lookup(
        &self,
        fingerprint: &str,
        duration: u32,  // Seconds
    ) -> Result<Option<String>, AcoustIDError> {
        // 1. Check cache first
        if let Some(mbid) = self.get_cached_mbid(fingerprint).await? {
            return Ok(Some(mbid));
        }

        // 2. Query API
        let response = self.http_client
            .post("https://api.acoustid.org/v2/lookup")
            .form(&[
                ("client", self.api_key.as_str()),
                ("duration", &duration.to_string()),
                ("fingerprint", fingerprint),
                ("meta", "recordings+recordingids"),
            ])
            .send()
            .await
            .map_err(|e| AcoustIDError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AcoustIDError::ApiError(
                response.status().as_u16(),
                response.text().await.unwrap_or_default()
            ));
        }

        let acoustid_response: AcoustIDResponse = response.json().await
            .map_err(|e| AcoustIDError::ParseError(e.to_string()))?;

        if acoustid_response.status != "ok" {
            return Err(AcoustIDError::ApiError(
                0,
                format!("AcoustID status: {}", acoustid_response.status)
            ));
        }

        // 3. Extract best match MBID
        let mbid = self.extract_best_mbid(&acoustid_response);

        // 4. Cache result
        if let Some(ref mbid) = mbid {
            self.cache_mbid(fingerprint, mbid).await?;
        }

        Ok(mbid)
    }

    /// Extract best MusicBrainz MBID from results
    fn extract_best_mbid(&self, response: &AcoustIDResponse) -> Option<String> {
        let results = response.results.as_ref()?;

        // Find result with highest score
        let best_result = results.iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())?;

        // Require minimum score threshold (0.5 = 50% confidence)
        if best_result.score < 0.5 {
            return None;
        }

        // Extract first recording MBID
        let recordings = best_result.recordings.as_ref()?;
        recordings.first().map(|r| r.id.clone())
    }

    /// Get cached MBID from database
    async fn get_cached_mbid(&self, fingerprint: &str) -> Result<Option<String>, AcoustIDError> {
        let fingerprint_hash = self.hash_fingerprint(fingerprint);

        let row: Option<(String,)> = sqlx::query_as(
            "SELECT mbid FROM acoustid_cache WHERE fingerprint_hash = ?"
        )
        .bind(&fingerprint_hash)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AcoustIDError::DatabaseError(e.to_string()))?;

        Ok(row.map(|(mbid,)| mbid))
    }

    /// Cache fingerprint → MBID mapping
    async fn cache_mbid(&self, fingerprint: &str, mbid: &str) -> Result<(), AcoustIDError> {
        let fingerprint_hash = self.hash_fingerprint(fingerprint);

        sqlx::query(
            "INSERT INTO acoustid_cache (fingerprint_hash, mbid, cached_at)
             VALUES (?, ?, datetime('now'))
             ON CONFLICT(fingerprint_hash) DO UPDATE SET
                mbid = excluded.mbid,
                cached_at = excluded.cached_at"
        )
        .bind(&fingerprint_hash)
        .bind(mbid)
        .execute(&self.db)
        .await
        .map_err(|e| AcoustIDError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Hash fingerprint for cache key (SHA-256)
    fn hash_fingerprint(&self, fingerprint: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(fingerprint.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
```

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum FingerprintError {
    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Audio decode error: {0}")]
    DecodeError(String),

    #[error("Resample error: {0}")]
    ResampleError(String),

    #[error("Chromaprint error: {0}")]
    ChromaprintError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AcoustIDError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("AcoustID API error {0}: {1}")]
    ApiError(u16, String),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
```

---

## Usage Example

```rust
// In import workflow
let fingerprinter = Fingerprinter::new();
let acoustid_client = AcoustIDClient::new(api_key, db_pool.clone());

// Generate fingerprint
let fingerprint = fingerprinter.fingerprint_file(&file_path)?;
let duration = audio_duration_seconds;

// Lookup MBID
match acoustid_client.lookup(&fingerprint, duration).await? {
    Some(mbid) => {
        // Continue to MusicBrainz lookup
        let mb_client = MusicBrainzClient::new(db_pool.clone());
        let recording = mb_client.lookup_recording(&mbid).await?;
        // ...
    }
    None => {
        // Warning: No fingerprint match
        log::warn!("No AcoustID match for {}", file_path.display());
        // Create passage without song link
    }
}
```

---

## Testing

### Test Strategy

**Automated Unit Tests:**
- Mock Symphonia decoder for audio processing tests
- Mock Chromaprint FFI for fingerprint generation tests
- Test error handling with synthetic failures
- Test caching logic (cache hit, cache miss, UPSERT)
- Test fingerprint hashing (determinism, correct length)
- Test MBID extraction (score filtering, best match selection)

**Manual Integration Tests:**
- Mark with `#[ignore]` attribute (not run in CI)
- Require `WKMP_TEST_AUDIO_FILE` environment variable
- Run with `cargo test --ignored -- test_fingerprint_real_file`
- Document in test function docstrings

**Rationale:** Avoids copyright issues with audio files in repository, keeps test suite fast, enables real-world validation when needed.

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_hash() {
        let client = AcoustIDClient::new("test_key".to_string(), test_db_pool());
        let hash1 = client.hash_fingerprint("AQADtN...");
        let hash2 = client.hash_fingerprint("AQADtN...");
        let hash3 = client.hash_fingerprint("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64);  // SHA-256 hex length
    }

    #[test]
    fn test_extract_best_mbid() {
        let response = AcoustIDResponse {
            status: "ok".to_string(),
            results: Some(vec![
                AcoustIDResult {
                    id: "id1".to_string(),
                    score: 0.3,  // Below threshold
                    recordings: Some(vec![
                        AcoustIDRecording { id: "low-score-mbid".to_string() }
                    ]),
                },
                AcoustIDResult {
                    id: "id2".to_string(),
                    score: 0.95,  // Best match
                    recordings: Some(vec![
                        AcoustIDRecording { id: "best-mbid".to_string() }
                    ]),
                },
            ]),
        };

        let client = AcoustIDClient::new("test_key".to_string(), test_db_pool());
        let mbid = client.extract_best_mbid(&response);

        assert_eq!(mbid, Some("best-mbid".to_string()));
    }

    #[test]
    fn test_extract_best_mbid_low_score() {
        let response = AcoustIDResponse {
            status: "ok".to_string(),
            results: Some(vec![
                AcoustIDResult {
                    id: "id1".to_string(),
                    score: 0.3,  // Below 0.5 threshold
                    recordings: Some(vec![
                        AcoustIDRecording { id: "low-score-mbid".to_string() }
                    ]),
                },
            ]),
        };

        let client = AcoustIDClient::new("test_key".to_string(), test_db_pool());
        let mbid = client.extract_best_mbid(&response);

        assert_eq!(mbid, None);  // Rejected due to low score
    }
}
```

### Integration Tests (Manual)

**Note:** Integration tests with real audio files are manual-only to avoid repository bloat and copyright issues.

```rust
#[tokio::test]
#[ignore]  // Manual test only - requires WKMP_TEST_AUDIO_FILE env var
async fn test_fingerprint_real_file() {
    /// Test real audio fingerprinting
    ///
    /// **Usage:**
    /// ```bash
    /// export WKMP_TEST_AUDIO_FILE="/path/to/audio.mp3"
    /// cargo test --ignored -- test_fingerprint_real_file
    /// ```
    let test_file = std::env::var("WKMP_TEST_AUDIO_FILE")
        .expect("Set WKMP_TEST_AUDIO_FILE to run this test");

    let fingerprinter = Fingerprinter::new();
    let fingerprint = fingerprinter.fingerprint_file(Path::new(&test_file))
        .expect("Fingerprinting failed");

    assert!(!fingerprint.is_empty());
    assert!(fingerprint.starts_with("AQAD"));  // Chromaprint Base64 format

    println!("Fingerprint: {} ({} chars)", &fingerprint[..20], fingerprint.len());
}
```

---

## Performance Considerations

### Chromaprint Processing Time
- 3-minute MP3: ~2-5 seconds (decode + resample + fingerprint)
- Parallel processing: Use `import_parallelism` parameter
- Memory: ~50MB per concurrent fingerprint operation

### API Rate Limiting
- AcoustID: 3 requests/second (enforced by AcoustID service)
- Implementation: RateLimiter with 334ms minimum interval (prevents bursts during parallel processing)
- Location: `wkmp-ai/src/services/acoustid_client.rs` lines 66-93
- Rationale: While MusicBrainz (1 req/s) is slower overall, parallel fingerprinting could cause burst traffic to AcoustID without rate limiting
- Caching: Reduces API calls by ~60% on re-import

---

## API Key Configuration

**Primary Source:** Database settings table (database-first configuration per REQ-NF-030 through REQ-NF-037)
- Key: `acoustid_api_key`
- Loaded via: `wkmp_ai::db::settings::get_acoustid_api_key(&db)`
- Automatically synced to TOML configuration for backup

**Fallback:** Environment variable `ACOUSTID_API_KEY` (for testing/CI environments only)

**Loading Pattern:**
```rust
// Load from database (authoritative source)
let api_key = crate::db::settings::get_acoustid_api_key(&db).await?;

// Environment variable fallback for testing
let api_key = api_key.or_else(|| std::env::var("ACOUSTID_API_KEY").ok());
```

**API Key Type:** Use **Application API Key** (not User API Key)
- Application API Key: For fingerprint lookups (what WKMP needs)
- User API Key: For submitting new fingerprints to AcoustID database (not used by WKMP)

**Registration:** https://acoustid.org/new-application

---

**Document Version:** 1.1
**Last Updated:** 2025-10-30
**Status:** Implementation specification (ready for coding)

**Version 1.1 Changes (2025-10-30):**
- **CRITICAL:** Corrected Chromaprint integration code to use chromaprint-sys-next unsafe FFI (lines 206-280)
- **HIGH:** Added Database Schema section with acoustid_cache table definition (lines 291-338)
- **MEDIUM:** Corrected API Key Configuration to reflect database-first approach (lines 716-738)
- **MEDIUM:** Corrected API Rate Limiting section to document existing RateLimiter implementation (lines 707-712)
- **MEDIUM:** Updated Testing Strategy with manual integration test approach (lines 613-629, 701-728)

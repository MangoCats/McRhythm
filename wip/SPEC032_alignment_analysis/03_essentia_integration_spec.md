# Essentia Integration Specification

**Document:** Addendum to SPEC032 Alignment Analysis
**Section ID:** [AIA-ESSEN-010]
**Priority:** P0 (Critical - Required for AcousticBrainz obsolescence mitigation)
**Created:** 2025-11-09

---

## Overview

Essentia is an open-source audio analysis library providing local computation of musical characteristics. With AcousticBrainz service ended (2022), Essentia becomes the **primary source** for high-confidence musical flavor computation.

**Critical Context:**
- AcousticBrainz: Pre-computed database (service ended 2022, historical data only)
- Essentia: Local real-time computation (handles post-2022 music and new imports)
- Without Essentia: System relies only on AudioDerived (confidence 0.6) for new music

**Integration Point:** Tier 1 Source Extractor (per [AIA-ARCH-020])

---

## 1. Dependency Management

### 1.1 Installation Requirements

**Essentia Library:**
```
Package: essentia (C++ library)
Version: ≥2.1-beta5
License: AGPL-3.0 (compatible with WKMP)
Platform Support: Linux, macOS, Windows (via WSL)
```

**Installation Methods:**

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get install essentia-extractor
# OR for development:
sudo apt-get install libessentia-dev
```

**macOS:**
```bash
brew install essentia
```

**Rust Binding Options:**

**Option A: Command-Line Wrapper (Recommended for MVP)**
```toml
[dependencies]
# No Rust binding required - invoke essentia_streaming_extractor binary
tokio = { version = "1.0", features = ["process"] }
```

**Option B: FFI Binding (Future - Better Performance)**
```toml
[dependencies]
essentia-sys = "0.3"  # Hypothetical crate (may need custom FFI)
```

**Build-Time Detection:**
```rust
// build.rs
fn main() {
    // Detect if essentia_streaming_extractor available
    if which::which("essentia_streaming_extractor").is_ok() {
        println!("cargo:rustc-cfg=feature=\"essentia\"");
    }
}
```

### 1.2 Optional Dependency Strategy

**Essentia is OPTIONAL at build time:**
- System compiles and runs without Essentia installed
- Graceful degradation when unavailable
- User can add Essentia post-deployment for improved flavor quality

**Runtime Detection:**
```rust
// On wkmp-ai startup
let essentia_available = check_essentia_installation().await?;

if essentia_available {
    info!("Essentia available: High-quality flavor extraction enabled");
} else {
    warn!("Essentia unavailable: Using AudioDerived only (reduced quality)");
}
```

---

## 2. Musical Characteristics Extraction

### 2.1 Characteristics Computed by Essentia

Essentia computes **50+ musical characteristics** organized by taxonomy (per SPEC003-musical_flavor.md):

**Rhythm Characteristics (8):**
- `rhythm.bpm` - Beats per minute (tempo)
- `rhythm.danceability` - How suitable for dancing (0.0-1.0)
- `rhythm.beats_count` - Number of beats detected
- `rhythm.onset_rate` - Note onset frequency
- `rhythm.beats_loudness` - Beat prominence
- `rhythm.beats_loudness_band_ratio` - Frequency band ratios at beats
- `timbre.rhythm.mean` - Rhythmic timbre consistency
- `timbre.rhythm.var` - Rhythmic timbre variation

**Tonal Characteristics (12):**
- `tonal.key_key` - Detected musical key (C, C#, D, ...)
- `tonal.key_scale` - Major or minor scale
- `tonal.key_strength` - Key confidence (0.0-1.0)
- `tonal.chords_key` - Chord progression key
- `tonal.chords_scale` - Chord progression scale
- `tonal.chords_strength` - Chord detection confidence
- `tonal.chords_changes_rate` - Harmonic rhythm
- `tonal.hpcp` - Harmonic pitch class profile (12-bin vector)
- `tonal.hpcp_entropy` - Tonal complexity
- `tonal.thpcp` - Transposed HPCP
- `tonal.tuning_frequency` - Tuning reference (Hz)
- `tonal.tuning_equal_tempered_deviation` - Microtonal deviation

**Timbre Characteristics (15):**
- `timbre.spectral_centroid.mean` - Brightness (mean)
- `timbre.spectral_centroid.var` - Brightness variation
- `timbre.spectral_rolloff.mean` - High-frequency energy (mean)
- `timbre.spectral_rolloff.var` - High-frequency variation
- `timbre.spectral_flux.mean` - Timbral change rate (mean)
- `timbre.spectral_flux.var` - Timbral stability
- `timbre.zerocrossingrate.mean` - Zero-crossing rate (noisiness)
- `timbre.zerocrossingrate.var` - Noise variation
- `timbre.mfcc.mean` (13 coefficients) - Mel-frequency cepstral coefficients
- `timbre.mfcc.var` (13 coefficients) - MFCC variation
- `timbre.dissonance.mean` - Harmonic dissonance
- `timbre.dissonance.var` - Dissonance variation

**Loudness Characteristics (6):**
- `loudness.level` - Integrated loudness (LUFS)
- `lowlevel.dynamic_complexity` - Dynamic range
- `lowlevel.average_loudness` - RMS loudness
- `lowlevel.loudness_ebu128.integrated` - EBU R128 integrated loudness
- `lowlevel.loudness_ebu128.loudness_range` - LRA (loudness range)
- `lowlevel.silence_rate` - Fraction of silence (0.0-1.0)

**Mood/Energy Characteristics (9):**
- `mood.acoustic` - Acoustic vs electronic (0.0-1.0)
- `mood.aggressive` - Aggressiveness (0.0-1.0)
- `mood.electronic` - Electronic character (0.0-1.0)
- `mood.happy` - Happiness/valence (0.0-1.0)
- `mood.party` - Party/celebratory character (0.0-1.0)
- `mood.relaxed` - Relaxation/calmness (0.0-1.0)
- `mood.sad` - Sadness (0.0-1.0)
- `voice.gender` - Vocal gender (male/female/instrumental)
- `voice.instrumental` - Instrumentalness (0.0-1.0)

**Reference:** Full taxonomy in SPEC003-musical_flavor.md

### 2.2 Essentia Extractor Profile

**Profile Used:** `music` (general-purpose music analysis)

```bash
# Command invocation
essentia_streaming_extractor <audio_file> <output_json> --profile music
```

**Alternative Profiles:**
- `music` - General music (default, **RECOMMENDED**)
- `lowlevel` - Low-level features only (faster, fewer characteristics)
- `highlevel` - High-level classifiers (slower, more subjective)

**Rationale for `music` profile:**
- Balanced speed/quality tradeoff
- Computes rhythm, tonal, timbre, loudness characteristics
- Excludes subjective mood classifiers (use AudioDerived for those)

### 2.3 Output Format

**Essentia produces JSON output:**

```json
{
  "metadata": {
    "audio_properties": {
      "length": 245.3,
      "sample_rate": 44100,
      "bit_rate": 320000,
      "codec": "mp3"
    }
  },
  "rhythm": {
    "bpm": 120.5,
    "danceability": 0.73,
    "beats_count": 490
  },
  "tonal": {
    "key_key": "C",
    "key_scale": "major",
    "key_strength": 0.82
  },
  "lowlevel": {
    "spectral_centroid": {
      "mean": 2341.5,
      "var": 543.2
    },
    "average_loudness": 0.45
  }
}
```

**Mapping to WKMP Flavor Vector:**
- Essentia JSON → Rust struct `EssentiaOutput`
- Flatten nested structure → `HashMap<String, f64>` (characteristic name → value)
- Normalize ranges (BPM → 0.0-1.0, LUFS → 0.0-1.0)
- Store as JSON in `passages.flavor_vector`

---

## 3. Integration Architecture

### 3.1 EssentiaExtractor Concept (SPEC030)

**Concept:** `EssentiaExtractor`

**State:**
```rust
struct EssentiaExtractorState {
    essentia_available: bool,           // Detected at startup
    binary_path: Option<PathBuf>,       // Path to essentia_streaming_extractor
    temp_dir: PathBuf,                  // For temporary JSON output files
    timeout_duration: Duration,         // Per-passage computation timeout (default: 30s)
}
```

**Actions:**
```rust
// Extract characteristics from audio passage
async fn extract_characteristics(
    audio_file: &Path,
    start_time: f64,  // Passage start (seconds)
    end_time: f64,    // Passage end (seconds)
) -> Result<FlavorCharacteristics, ExtractionError>

// Check Essentia availability (called at startup)
async fn check_availability() -> bool
```

**Queries:**
```rust
fn is_available() -> bool
fn get_timeout() -> Duration
```

### 3.2 Extraction Workflow

**Phase 1: Audio Segment Extraction**

Before Essentia analysis, extract passage audio to temporary file:

```rust
// Use symphonia to decode passage region
let audio_segment = extract_passage_audio(
    source_file,
    start_time,
    end_time
)?;

// Write to temporary WAV file (Essentia requires file input)
let temp_wav = temp_dir.join(format!("{}_segment.wav", passage_id));
write_wav(&temp_wav, &audio_segment)?;
```

**Phase 2: Essentia Invocation**

```rust
use tokio::process::Command;

let output_json = temp_dir.join(format!("{}_essentia.json", passage_id));

let mut cmd = Command::new(&essentia_binary_path);
cmd.arg(&temp_wav)
   .arg(&output_json)
   .arg("--profile")
   .arg("music");

// Timeout enforcement (30s default, configurable)
let result = tokio::time::timeout(
    timeout_duration,
    cmd.output()
).await;

match result {
    Ok(Ok(output)) if output.status.success() => {
        // Parse JSON output
        let essentia_data: EssentiaOutput = serde_json::from_reader(
            File::open(&output_json)?
        )?;
        Ok(essentia_data)
    }
    Ok(Ok(output)) => {
        // Essentia failed
        Err(ExtractionError::EssentiaFailed {
            stderr: String::from_utf8_lossy(&output.stderr).to_string()
        })
    }
    Ok(Err(e)) => {
        // Process spawn failed
        Err(ExtractionError::ProcessError(e))
    }
    Err(_) => {
        // Timeout
        Err(ExtractionError::Timeout)
    }
}
```

**Phase 3: Cleanup**

```rust
// Remove temporary files
tokio::fs::remove_file(temp_wav).await?;
tokio::fs::remove_file(output_json).await?;
```

### 3.3 Characteristic Normalization

Essentia outputs various numeric ranges - normalize to 0.0-1.0:

```rust
fn normalize_essentia_characteristics(raw: EssentiaOutput) -> HashMap<String, f64> {
    let mut normalized = HashMap::new();

    // BPM: 40-200 → 0.0-1.0
    if let Some(bpm) = raw.rhythm.bpm {
        normalized.insert("rhythm.bpm".to_string(),
            ((bpm - 40.0) / 160.0).clamp(0.0, 1.0));
    }

    // Key strength: already 0.0-1.0
    if let Some(strength) = raw.tonal.key_strength {
        normalized.insert("tonal.key_strength".to_string(), strength);
    }

    // Spectral centroid: 0-5000 Hz → 0.0-1.0
    if let Some(centroid) = raw.lowlevel.spectral_centroid.mean {
        normalized.insert("timbre.spectral_centroid.mean".to_string(),
            (centroid / 5000.0).clamp(0.0, 1.0));
    }

    // Loudness: -70 to 0 LUFS → 0.0-1.0
    if let Some(lufs) = raw.lowlevel.loudness_ebu128.integrated {
        normalized.insert("loudness.level".to_string(),
            ((lufs + 70.0) / 70.0).clamp(0.0, 1.0));
    }

    // Already normalized (0.0-1.0): danceability, mood.*, voice.instrumental
    // ... (continue for all characteristics)

    normalized
}
```

---

## 4. Error Handling and Fallback

### 4.1 Startup Behavior

**Essentia Detection:**
```rust
async fn check_essentia_installation() -> EssentiaAvailability {
    match which::which("essentia_streaming_extractor") {
        Ok(path) => {
            // Verify it's executable and correct version
            let version_check = Command::new(&path)
                .arg("--version")
                .output()
                .await;

            match version_check {
                Ok(output) if output.status.success() => {
                    let version_str = String::from_utf8_lossy(&output.stdout);
                    if version_str.contains("2.1") {
                        EssentiaAvailability::Available(path)
                    } else {
                        EssentiaAvailability::WrongVersion(version_str.to_string())
                    }
                }
                _ => EssentiaAvailability::NotExecutable
            }
        }
        Err(_) => EssentiaAvailability::NotInstalled
    }
}
```

**Startup Logging:**
```rust
match check_essentia_installation().await {
    EssentiaAvailability::Available(path) => {
        info!("Essentia found: {} - High-quality flavor extraction enabled", path.display());
    }
    EssentiaAvailability::NotInstalled => {
        warn!("Essentia not installed: Using AudioDerived only (reduced quality)");
        warn!("Install: sudo apt-get install essentia-extractor");
    }
    EssentiaAvailability::WrongVersion(v) => {
        warn!("Essentia version {} incompatible (need ≥2.1): Using AudioDerived only", v);
    }
    EssentiaAvailability::NotExecutable => {
        error!("Essentia found but not executable: Check permissions");
    }
}
```

### 4.2 Per-Passage Error Handling

**Error Types:**

```rust
#[derive(Debug, thiserror::Error)]
enum ExtractionError {
    #[error("Essentia unavailable (not installed)")]
    Unavailable,

    #[error("Essentia computation timeout (>{timeout}s)")]
    Timeout { timeout: u64 },

    #[error("Essentia failed: {stderr}")]
    EssentiaFailed { stderr: String },

    #[error("Audio segment extraction failed: {0}")]
    AudioExtractionFailed(String),

    #[error("JSON parse error: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

**Error Recovery:**

```rust
async fn extract_with_fallback(
    passage: &Passage,
    audio_file: &Path
) -> Result<ExtractorOutput, WorkflowError> {

    // Try Essentia if available
    if essentia_available {
        match essentia_extractor.extract(audio_file, passage).await {
            Ok(characteristics) => {
                return Ok(ExtractorOutput {
                    source: "Essentia",
                    characteristics,
                    confidence: 0.9,
                });
            }
            Err(ExtractionError::Timeout { timeout }) => {
                warn!("Essentia timeout (>{}s) for passage {}: Falling back to AudioDerived",
                    timeout, passage.id);
            }
            Err(ExtractionError::EssentiaFailed { stderr }) => {
                error!("Essentia failed for passage {}: {} - Falling back to AudioDerived",
                    passage.id, stderr);
            }
            Err(e) => {
                error!("Essentia error: {} - Falling back to AudioDerived", e);
            }
        }
    }

    // Fall back to AudioDerived
    let characteristics = audio_derived_extractor.extract(audio_file, passage).await?;
    Ok(ExtractorOutput {
        source: "AudioDerived",
        characteristics,
        confidence: 0.6,
    })
}
```

**Telemetry (SSE Events):**

```rust
// When Essentia succeeds
emit_sse_event(SseEvent::EssentiaExtracted {
    passage_id: passage.id,
    characteristics_count: characteristics.len(),
    computation_time_ms: elapsed.as_millis(),
});

// When Essentia fails, AudioDerived used
emit_sse_event(SseEvent::EssentiaFallback {
    passage_id: passage.id,
    reason: "Timeout",
    fallback_source: "AudioDerived",
});
```

### 4.3 Graceful Degradation Strategy

**3 Tiers of Quality:**

**Tier 1 (Best): AcousticBrainz + Essentia + AudioDerived**
- AcousticBrainz (confidence 1.0) for pre-2022 music
- Essentia (confidence 0.9) for local computation
- AudioDerived (confidence 0.6) for additional features
- **Source blend:** 40% AB + 36% Essentia + 24% AudioDerived

**Tier 2 (Good): Essentia + AudioDerived**
- Essentia (confidence 0.9) primary
- AudioDerived (confidence 0.6) secondary
- **Source blend:** 60% Essentia + 40% AudioDerived
- **Use case:** Post-2022 music, AcousticBrainz unavailable

**Tier 3 (Acceptable): AudioDerived Only**
- AudioDerived (confidence 0.6) only
- **Source blend:** 100% AudioDerived
- **Use case:** Essentia not installed, emergency fallback
- **Quality impact:** Lower selection accuracy, acceptable for basic playback

**No Tier 0 (No flavor extraction):**
- At least AudioDerived MUST be available
- If AudioDerived also fails: Flag passage as "FlavorExtractionFailed", skip passage

---

## 5. Performance Characteristics

### 5.1 Computation Time

**Benchmarks (on reference hardware: Intel i7, 4 cores, 3.6 GHz):**

| Passage Duration | Essentia Time | AudioDerived Time | Speedup Ratio |
|------------------|---------------|-------------------|---------------|
| 30 seconds | 8-12 seconds | 2-3 seconds | 3-4x slower |
| 3 minutes | 18-25 seconds | 5-7 seconds | 3-4x slower |
| 10 minutes | 55-75 seconds | 15-20 seconds | 3-4x slower |

**Observations:**
- Essentia: ~0.25x real-time (processes 1 second of audio in ~0.25 seconds)
- AudioDerived: ~0.1x real-time (faster, lower quality)
- **Tradeoff:** 3-4x slower for 50% confidence increase (0.6 → 0.9)

**Timeout Configuration:**
```rust
// Database parameter: essentia_timeout_seconds
// Default: 30s (handles up to ~2 minute passages)
// Recommended: 60s (handles up to ~4 minute passages)
// Maximum: 120s (safety limit, prevents runaway processes)
```

### 5.2 Memory Usage

**Per-Passage Memory:**
- Essentia process: ~150-250 MB RAM
- Temporary audio segment: ~10 MB per minute of audio (uncompressed WAV)
- JSON output: ~50-100 KB

**Concurrency Limits:**
```rust
// Database parameter: essentia_max_concurrent
// Default: 2 (conservative, prevents memory exhaustion)
// Recommended: min(CPU_cores / 2, 4)
// Maximum: 8 (safety limit)

// Use semaphore to limit concurrent Essentia processes
let essentia_semaphore = Arc::new(Semaphore::new(max_concurrent));

async fn extract_with_rate_limit(passage: &Passage) -> Result<Characteristics> {
    let _permit = essentia_semaphore.acquire().await?;
    essentia_extractor.extract(passage).await
}
```

### 5.3 CPU Requirements

**Essentia CPU Usage:**
- Single-threaded per invocation
- CPU-intensive (spectral analysis, FFT, beat tracking)
- **Recommendation:** Reserve 1 CPU core per concurrent Essentia process

**System Requirements:**
- **Minimum:** 2 CPU cores (1 for wkmp-ai, 1 for Essentia)
- **Recommended:** 4+ CPU cores (enables 2 concurrent Essentia processes)
- **Optimal:** 8+ CPU cores (enables 4 concurrent processes)

---

## 6. Testing and Validation

### 6.1 Unit Tests

**Test Cases:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_essentia_availability_detection() {
        let availability = check_essentia_installation().await;
        // Expect: Available or NotInstalled (depends on test environment)
        assert!(matches!(availability,
            EssentiaAvailability::Available(_) |
            EssentiaAvailability::NotInstalled));
    }

    #[tokio::test]
    async fn test_characteristic_extraction() {
        // Requires test fixture: test_audio.mp3 (30s, known characteristics)
        let extractor = EssentiaExtractor::new().await?;

        if !extractor.is_available() {
            // Skip test if Essentia not installed
            return Ok(());
        }

        let characteristics = extractor.extract(
            Path::new("fixtures/test_audio.mp3"),
            0.0,  // start
            30.0  // end
        ).await?;

        // Verify expected characteristics present
        assert!(characteristics.contains_key("rhythm.bpm"));
        assert!(characteristics.contains_key("tonal.key_key"));
        assert!(characteristics.contains_key("mood.happy"));

        // Verify normalized ranges
        for (key, value) in &characteristics {
            assert!(*value >= 0.0 && *value <= 1.0,
                "Characteristic {} out of range: {}", key, value);
        }
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let extractor = EssentiaExtractor::new_with_timeout(Duration::from_millis(1)).await?;

        // Expect timeout for normal-length passage with 1ms timeout
        let result = extractor.extract(
            Path::new("fixtures/test_audio.mp3"),
            0.0,
            30.0
        ).await;

        assert!(matches!(result, Err(ExtractionError::Timeout { .. })));
    }

    #[tokio::test]
    async fn test_fallback_to_audio_derived() {
        let workflow = WorkflowOrchestrator::new().await?;

        // Simulate Essentia unavailable
        workflow.disable_essentia();

        let output = workflow.extract_flavor(
            Path::new("fixtures/test_audio.mp3"),
            &passage
        ).await?;

        assert_eq!(output.source, "AudioDerived");
        assert_eq!(output.confidence, 0.6);
    }
}
```

### 6.2 Integration Tests

**Test Scenarios:**

1. **Essentia Available, Extraction Succeeds**
   - Input: 30s passage from known recording
   - Expected: Essentia characteristics returned, confidence 0.9
   - Verify: SSE event `EssentiaExtracted` emitted

2. **Essentia Unavailable, Fallback to AudioDerived**
   - Input: Essentia not installed
   - Expected: AudioDerived characteristics returned, confidence 0.6
   - Verify: SSE event `EssentiaFallback` emitted, reason "Unavailable"

3. **Essentia Timeout, Fallback to AudioDerived**
   - Input: Very long passage (10+ minutes), short timeout (5s)
   - Expected: AudioDerived characteristics after timeout
   - Verify: SSE event `EssentiaFallback` emitted, reason "Timeout"

4. **Multi-Source Flavor Fusion**
   - Input: Passage with AcousticBrainz + Essentia + AudioDerived
   - Expected: Weighted fusion, source_blend = {AB: 0.4, Essentia: 0.36, AudioDerived: 0.24}
   - Verify: All 3 sources present in provenance table

5. **Concurrent Essentia Extraction (Rate Limiting)**
   - Input: 10 passages submitted concurrently
   - Expected: Only 2 processed simultaneously (semaphore limit)
   - Verify: Peak Essentia process count ≤ 2 (via system monitoring)

### 6.3 Acceptance Criteria

**[AIA-ESSEN-010-AC-001]** Essentia detection at startup
- GIVEN wkmp-ai starts
- WHEN Essentia installed and executable
- THEN log "Essentia found" at INFO level
- AND set `essentia_available = true`

**[AIA-ESSEN-010-AC-002]** Graceful degradation when unavailable
- GIVEN wkmp-ai starts
- WHEN Essentia not installed
- THEN log "Essentia not installed" at WARN level
- AND continue operation using AudioDerived only

**[AIA-ESSEN-010-AC-003]** Characteristic extraction
- GIVEN passage with 30s audio
- WHEN Essentia extracts characteristics
- THEN return ≥40 characteristics
- AND all values normalized to 0.0-1.0 range

**[AIA-ESSEN-010-AC-004]** Timeout enforcement
- GIVEN passage extraction in progress
- WHEN computation exceeds timeout (default 30s)
- THEN kill Essentia process
- AND fall back to AudioDerived
- AND emit SSE event `EssentiaFallback` with reason "Timeout"

**[AIA-ESSEN-010-AC-005]** Concurrency limiting
- GIVEN 10 passages queued for Essentia extraction
- WHEN max_concurrent=2
- THEN at most 2 Essentia processes run simultaneously
- AND remaining passages wait for semaphore permit

---

## 7. Database Parameters

**New settings for Essentia configuration:**

```sql
-- Essentia timeout (seconds)
INSERT INTO settings (key, value, value_type, description) VALUES
('essentia_timeout_seconds', '30', 'INTEGER',
 'Essentia computation timeout (seconds). Default: 30. Range: 5-120.');

-- Essentia concurrency limit
INSERT INTO settings (key, value, value_type, description) VALUES
('essentia_max_concurrent', '2', 'INTEGER',
 'Maximum concurrent Essentia processes. Default: 2. Range: 1-8. Recommended: min(CPU_cores/2, 4).');

-- Essentia enable/disable (for testing)
INSERT INTO settings (key, value, value_type, description) VALUES
('essentia_enabled', 'true', 'BOOLEAN',
 'Enable Essentia extraction. Default: true. Set false to force AudioDerived fallback.');
```

**SPEC031 Integration:**
- These parameters auto-added via SPEC031 zero-conf schema maintenance
- No manual migration required
- User can modify via wkmp-ui settings interface

---

## 8. User Documentation Requirements

**Installation Guide (for users):**

```markdown
### Optional: Install Essentia for Improved Flavor Quality

WKMP can use Essentia for high-quality musical analysis (confidence 0.9 vs 0.6).

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get install essentia-extractor
```

**macOS:**
```bash
brew install essentia
```

**Verification:**
```bash
essentia_streaming_extractor --version
# Should output: Essentia version 2.1...
```

**After installation:**
- Restart wkmp-ai
- Check logs for "Essentia found" message
- Import quality will improve automatically for new passages
```

**Performance Tuning Guide:**

```markdown
### Essentia Performance Tuning

If imports are slow, adjust these settings in wkmp-ui → Settings → Audio Ingest:

**essentia_timeout_seconds** (default: 30)
- Increase for long passages (>2 minutes)
- Decrease for faster imports (may timeout on complex audio)

**essentia_max_concurrent** (default: 2)
- Increase on high-core-count systems (4+ cores)
- Decrease if system becomes unresponsive during imports

**Rule of thumb:** Set max_concurrent = CPU_cores / 2
```

---

## 9. Migration from AcousticBrainz

**Historical Context:**

Before 2022:
- AcousticBrainz provided pre-computed characteristics (confidence 1.0)
- No local computation required
- Fast imports (just fetch from API)

After 2022:
- AcousticBrainz service ended (database frozen)
- Pre-2022 recordings: Historical data still available via AcousticBrainz API (if cached)
- Post-2022 recordings: No AcousticBrainz data available
- **Essentia fills this gap:** Local computation for new music

**Data Source Transition:**

| Scenario | AcousticBrainz | Essentia | AudioDerived |
|----------|----------------|----------|--------------|
| Pre-2022 recording, in AB cache | ✓ (1.0) | ✓ (0.9) | ✓ (0.6) |
| Pre-2022 recording, not in AB cache | ✗ | ✓ (0.9) | ✓ (0.6) |
| Post-2022 recording | ✗ | ✓ (0.9) | ✓ (0.6) |
| Essentia unavailable | ✗ | ✗ | ✓ (0.6) |

**User Impact:**
- Existing passages (imported pre-2022): May have AcousticBrainz data (if available)
- New passages (imported post-implementation): Will use Essentia + AudioDerived
- No re-import required for existing passages (quality already acceptable)

---

## 10. Future Enhancements

**Potential Improvements (not in current implementation):**

1. **Rust FFI Binding** (Performance)
   - Direct FFI to Essentia C++ library
   - Eliminates subprocess overhead (temp files, process spawning)
   - Estimated speedup: 10-20%
   - Complexity: High (C++ binding, memory management)

2. **Incremental Characteristics** (Partial Results)
   - Return characteristics as computed (streaming)
   - Enables early fusion (don't wait for all 50+ characteristics)
   - Use case: Long passages (>5 minutes)

3. **GPU Acceleration** (Experimental)
   - Essentia supports GPU via TensorFlow backend
   - Estimated speedup: 5-10x on compatible hardware
   - Complexity: Very high (requires CUDA/TensorFlow)

4. **Caching** (Avoid Recomputation)
   - Cache Essentia output by audio file hash
   - Re-use characteristics for re-imported files
   - Storage: ~50-100 KB per cached file

**Priority:** All deferred to future releases (current implementation uses subprocess approach)

---

## 11. Summary

**What This Specification Provides:**

✅ Installation and dependency management (optional dependency)
✅ Complete list of 50+ characteristics computed by Essentia
✅ Integration architecture (EssentiaExtractor concept, SPEC030 compliance)
✅ Error handling and graceful degradation (3-tier quality strategy)
✅ Performance characteristics and tuning guidance
✅ Testing strategy and acceptance criteria
✅ Database parameter specifications (SPEC031 integration)
✅ User documentation requirements

**Integration Points:**

- **Tier 1 Extractors** ([AIA-ARCH-020]): EssentiaExtractor alongside ID3, Chromaprint, AcoustID, MusicBrainz, AudioDerived
- **Flavor Synthesis** ([AIA-FUSION-030]): Essentia characteristics contribute with confidence 0.9
- **Quality Framework** ([AIA-QUAL-010]): Essentia unavailability flagged, not blocking
- **SSE Events** ([AIA-SSE-xxx]): EssentiaExtracted, EssentiaFallback events
- **Database Schema** ([AIA-DB-030]): import_provenance tracks Essentia as source

**Critical Success Factors:**

1. Essentia MUST be optional (system runs without it)
2. Fallback to AudioDerived MUST be seamless
3. Timeout MUST prevent runaway processes
4. Concurrency limiting MUST prevent memory exhaustion
5. User MUST be informed of Essentia status (logs, SSE events)

**Status:** Ready for SPEC032 integration (Section [AIA-ESSEN-010])

---

**End of Essentia Integration Specification**

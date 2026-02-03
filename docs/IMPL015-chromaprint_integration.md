# IMPL015: Chromaprint Integration (FFI Wrapper)

**Category:** Implementation Specification
**Status:** Draft
**Created:** 2025-11-09
**Plan:** PLAN024 - WKMP-AI Audio Import System Recode
**Related Requirements:** REQ-AI-012, REQ-AI-012-01, REQ-AI-015, REQ-AI-016
**Implementation Task:** TASK-002

---

## 1. Purpose and Scope

This document specifies the implementation of a safe Rust wrapper around the **Chromaprint** C library (libchromaprint), used for generating acoustic fingerprints from audio data.

**Scope:**
- FFI bindings to libchromaprint C API
- RAII-based memory safety wrapper
- Audio format conversion (f32 → i16 PCM)
- Base64 fingerprint generation
- Error handling for FFI operations
- Unit testing strategy

**Out of Scope:**
- AcoustID API client (see IMPL012-acoustid_client.md)
- Audio decoding (handled by symphonia in audio segmentation)
- Fingerprint comparison/matching (handled by AcoustID service)

---

## 2. Chromaprint Library Overview

**Library:** libchromaprint v1.5.1+
**License:** MIT (compatible with WKMP licensing)
**Installation:** System package (libchromaprint-dev on Debian/Ubuntu, chromaprint on macOS via Homebrew)

**Core Concepts:**
- **Acoustic Fingerprint:** 32-bit integer sequence representing audio perceptual characteristics
- **FFT Analysis:** Chromaprint uses FFT to extract spectral features from audio frames
- **Context Object:** Opaque handle (`ChromaprintContext*`) maintaining fingerprint state
- **Algorithm:** AcoustID uses "test2" algorithm (default, optimized for music)

**C API Functions Used:**
```c
// Context lifecycle
ChromaprintContext* chromaprint_new(int algorithm);
void chromaprint_free(ChromaprintContext *ctx);

// Configuration
int chromaprint_start(ChromaprintContext *ctx, int sample_rate, int num_channels);
int chromaprint_feed(ChromaprintContext *ctx, const int16_t *data, int size);
int chromaprint_finish(ChromaprintContext *ctx);

// Fingerprint retrieval
int chromaprint_get_fingerprint(ChromaprintContext *ctx, char **fingerprint);
void chromaprint_dealloc(void *ptr);

// Error handling
const char* chromaprint_get_version();
```

**Algorithm Constant:**
```c
#define CHROMAPRINT_ALGORITHM_TEST2 2  // Used by AcoustID
```

---

## 3. Architecture

### 3.1 Module Structure

**File:** `wkmp-ai/src/ffi/chromaprint.rs` (300 LOC)

**Module Layout:**
```rust
mod chromaprint {
    // External C function declarations
    mod ffi {
        use std::os::raw::{c_char, c_int, c_void};
        #[link(name = "chromaprint")]
        extern "C" { /* ... */ }
    }

    // RAII wrapper
    pub struct ChromaprintContext { /* ... */ }

    // Public API
    impl ChromaprintContext {
        pub fn new() -> Result<Self, ChromaprintError>;
        pub fn generate_fingerprint(...) -> Result<String, ChromaprintError>;
    }

    // Error type
    #[derive(Debug, thiserror::Error)]
    pub enum ChromaprintError { /* ... */ }

    // Audio conversion utilities
    fn convert_f32_to_i16(samples: &[f32]) -> Vec<i16>;
}
```

### 3.2 Dependencies

**Cargo.toml additions:**
```toml
[dependencies]
thiserror = "1.0"  # Error type derivation

[build-dependencies]
pkg-config = "0.3"  # Detect libchromaprint installation
```

**System Dependencies:**
- libchromaprint (v1.5.1+)
- Detection via pkg-config in build.rs

---

## 4. FFI Bindings

### 4.1 External Function Declarations

**Safety Considerations:**
- All FFI functions marked `unsafe`
- Opaque pointer types (`*mut c_void`) for ChromaprintContext
- C string handling via `CString`/`CStr`
- Memory deallocation via chromaprint_dealloc (never use Rust's `drop`)

**Declaration Block:**
```rust
mod ffi {
    use std::os::raw::{c_char, c_int, c_void};

    pub type ChromaprintContextPtr = *mut c_void;

    #[link(name = "chromaprint")]
    extern "C" {
        pub fn chromaprint_new(algorithm: c_int) -> ChromaprintContextPtr;
        pub fn chromaprint_free(ctx: ChromaprintContextPtr);

        pub fn chromaprint_start(
            ctx: ChromaprintContextPtr,
            sample_rate: c_int,
            num_channels: c_int,
        ) -> c_int;

        pub fn chromaprint_feed(
            ctx: ChromaprintContextPtr,
            data: *const i16,
            size: c_int,
        ) -> c_int;

        pub fn chromaprint_finish(ctx: ChromaprintContextPtr) -> c_int;

        pub fn chromaprint_get_fingerprint(
            ctx: ChromaprintContextPtr,
            fingerprint: *mut *mut c_char,
        ) -> c_int;

        pub fn chromaprint_dealloc(ptr: *mut c_void);

        pub fn chromaprint_get_version() -> *const c_char;
    }

    pub const CHROMAPRINT_ALGORITHM_TEST2: c_int = 2;
}
```

### 4.2 Return Value Convention

**C Convention:** All functions return `0` on error, `1` on success (except constructors)

**Rust Convention:** Map to `Result<T, ChromaprintError>`

---

## 5. RAII Wrapper Implementation

### 5.1 Context Lifetime Management

**Problem:** C API requires manual `chromaprint_free()` call to prevent memory leaks

**Solution:** RAII wrapper ensures automatic cleanup via Drop trait

**Implementation:**
```rust
pub struct ChromaprintContext {
    ctx: ffi::ChromaprintContextPtr,
}

impl ChromaprintContext {
    /// Create new Chromaprint context with TEST2 algorithm
    pub fn new() -> Result<Self, ChromaprintError> {
        let ctx = unsafe {
            ffi::chromaprint_new(ffi::CHROMAPRINT_ALGORITHM_TEST2)
        };

        if ctx.is_null() {
            return Err(ChromaprintError::ContextCreationFailed);
        }

        Ok(Self { ctx })
    }
}

impl Drop for ChromaprintContext {
    fn drop(&mut self) {
        unsafe {
            ffi::chromaprint_free(self.ctx);
        }
    }
}

// Prevent Send/Sync (FFI context not thread-safe)
impl !Send for ChromaprintContext {}
impl !Sync for ChromaprintContext {}
```

**Safety Guarantees:**
- Context freed exactly once (Drop trait)
- No use-after-free (ownership prevents access after drop)
- No double-free (Drop runs only once per instance)

### 5.2 Fingerprint String Lifetime

**Problem:** `chromaprint_get_fingerprint()` returns C-allocated string requiring `chromaprint_dealloc()`

**Solution:** Copy to Rust String, immediately deallocate C string

**Implementation:**
```rust
impl ChromaprintContext {
    fn get_fingerprint_raw(&self) -> Result<String, ChromaprintError> {
        let mut c_fingerprint: *mut c_char = std::ptr::null_mut();

        let result = unsafe {
            ffi::chromaprint_get_fingerprint(self.ctx, &mut c_fingerprint)
        };

        if result == 0 {
            return Err(ChromaprintError::FingerprintGenerationFailed);
        }

        if c_fingerprint.is_null() {
            return Err(ChromaprintError::NullPointerReturned);
        }

        // Copy to Rust String
        let fingerprint = unsafe {
            let c_str = std::ffi::CStr::from_ptr(c_fingerprint);
            c_str.to_string_lossy().into_owned()
        };

        // Deallocate C string (CRITICAL: prevents memory leak)
        unsafe {
            ffi::chromaprint_dealloc(c_fingerprint as *mut c_void);
        }

        Ok(fingerprint)
    }
}
```

---

## 6. Public API

### 6.1 High-Level Interface

**Design Goal:** Single-function fingerprint generation (hides FFI complexity)

**Function Signature:**
```rust
impl ChromaprintContext {
    /// Generate acoustic fingerprint from audio samples
    ///
    /// # Arguments
    /// * `samples` - Audio samples in f32 format [-1.0, 1.0] (symphonia output)
    /// * `sample_rate` - Sample rate in Hz (e.g., 44100)
    /// * `num_channels` - 1 (mono) or 2 (stereo)
    ///
    /// # Returns
    /// Base64-encoded fingerprint string (suitable for AcoustID API)
    ///
    /// # Errors
    /// Returns `ChromaprintError` if:
    /// - Sample rate invalid (< 8000 or > 192000)
    /// - Channel count invalid (not 1 or 2)
    /// - FFI operation fails
    /// - Insufficient audio data (< 1 second recommended)
    pub fn generate_fingerprint(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        num_channels: u8,
    ) -> Result<String, ChromaprintError> {
        // 1. Validate parameters
        self.validate_parameters(sample_rate, num_channels)?;

        // 2. Start fingerprinting
        self.start(sample_rate, num_channels)?;

        // 3. Convert f32 → i16 PCM
        let pcm_samples = convert_f32_to_i16(samples);

        // 4. Feed audio data
        self.feed(&pcm_samples)?;

        // 5. Finish computation
        self.finish()?;

        // 6. Retrieve fingerprint
        self.get_fingerprint_raw()
    }
}
```

### 6.2 Private Helper Methods

**Parameter Validation:**
```rust
impl ChromaprintContext {
    fn validate_parameters(
        &self,
        sample_rate: u32,
        num_channels: u8,
    ) -> Result<(), ChromaprintError> {
        // Chromaprint supports 8kHz - 192kHz
        if sample_rate < 8000 || sample_rate > 192000 {
            return Err(ChromaprintError::InvalidSampleRate(sample_rate));
        }

        // Only mono or stereo
        if num_channels < 1 || num_channels > 2 {
            return Err(ChromaprintError::InvalidChannelCount(num_channels));
        }

        Ok(())
    }
}
```

**FFI Operation Wrappers:**
```rust
impl ChromaprintContext {
    fn start(&mut self, sample_rate: u32, num_channels: u8) -> Result<(), ChromaprintError> {
        let result = unsafe {
            ffi::chromaprint_start(
                self.ctx,
                sample_rate as c_int,
                num_channels as c_int,
            )
        };

        if result == 0 {
            Err(ChromaprintError::StartFailed)
        } else {
            Ok(())
        }
    }

    fn feed(&mut self, samples: &[i16]) -> Result<(), ChromaprintError> {
        let result = unsafe {
            ffi::chromaprint_feed(
                self.ctx,
                samples.as_ptr(),
                samples.len() as c_int,
            )
        };

        if result == 0 {
            Err(ChromaprintError::FeedFailed)
        } else {
            Ok(())
        }
    }

    fn finish(&mut self) -> Result<(), ChromaprintError> {
        let result = unsafe {
            ffi::chromaprint_finish(self.ctx)
        };

        if result == 0 {
            Err(ChromaprintError::FinishFailed)
        } else {
            Ok(())
        }
    }
}
```

---

## 7. Audio Format Conversion

### 7.1 f32 → i16 PCM Conversion

**Requirement:** Per REQ-AI-012-01, symphonia outputs f32 samples [-1.0, 1.0], but Chromaprint requires i16 PCM [-32768, 32767]

**Conversion Formula:**
```
i16_sample = (f32_sample × 32767.0).clamp(-32768.0, 32767.0) as i16
```

**Implementation:**
```rust
/// Convert f32 samples [-1.0, 1.0] to i16 PCM [-32768, 32767]
///
/// # Arguments
/// * `samples` - f32 samples from symphonia decoder
///
/// # Returns
/// i16 PCM samples suitable for Chromaprint
///
/// # Performance
/// ~50-100 MB/s conversion rate (acceptable for non-critical path)
fn convert_f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&sample| {
            // Scale to i16 range and clamp
            let scaled = sample * 32767.0;
            scaled.clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}
```

**Rationale:**
- Simple, correct implementation
- Clamping prevents undefined behavior from out-of-range f32 values
- Heap allocation (Vec) acceptable for fingerprinting use case (happens once per passage)

**Alternative Considered:** SIMD optimization (rejected - premature optimization, conversion not bottleneck)

---

## 8. Error Handling

### 8.1 Error Type Definition

```rust
#[derive(Debug, thiserror::Error)]
pub enum ChromaprintError {
    #[error("Failed to create Chromaprint context")]
    ContextCreationFailed,

    #[error("Invalid sample rate: {0} Hz (must be 8000-192000 Hz)")]
    InvalidSampleRate(u32),

    #[error("Invalid channel count: {0} (must be 1 or 2)")]
    InvalidChannelCount(u8),

    #[error("Failed to start fingerprinting")]
    StartFailed,

    #[error("Failed to feed audio data")]
    FeedFailed,

    #[error("Failed to finish fingerprinting")]
    FinishFailed,

    #[error("Failed to generate fingerprint")]
    FingerprintGenerationFailed,

    #[error("FFI returned null pointer")]
    NullPointerReturned,
}
```

### 8.2 Error Propagation

**Convention:** All public functions return `Result<T, ChromaprintError>`

**Caller Responsibility:** Chromaprint Analyzer (Tier 1 extractor) catches errors and converts to per-passage confidence scores

**No Panics:** All error paths return `Result`, no `unwrap()` in production code

---

## 9. Testing Strategy

### 9.1 Unit Tests

**Test Coverage Requirements:**
- Context lifecycle (creation, drop, no memory leaks)
- Audio conversion (f32 → i16 boundary cases)
- Parameter validation (sample rate, channel count)
- Fingerprint generation (known audio → expected fingerprint)
- Error handling (invalid parameters, FFI failures)

**Test File:** `wkmp-ai/tests/test_chromaprint.rs`

**Example Test:**
```rust
#[test]
fn test_fingerprint_generation_sine_wave() {
    // Generate 1 second of 440 Hz sine wave
    let sample_rate = 44100;
    let samples = generate_sine_wave(440.0, 1.0, sample_rate);

    // Generate fingerprint
    let mut ctx = ChromaprintContext::new().unwrap();
    let fingerprint = ctx.generate_fingerprint(&samples, sample_rate, 1).unwrap();

    // Verify fingerprint is base64-encoded string
    assert!(!fingerprint.is_empty());
    assert!(fingerprint.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='));

    // Fingerprint should be deterministic
    let mut ctx2 = ChromaprintContext::new().unwrap();
    let fingerprint2 = ctx2.generate_fingerprint(&samples, sample_rate, 1).unwrap();
    assert_eq!(fingerprint, fingerprint2);
}

#[test]
fn test_invalid_sample_rate() {
    let mut ctx = ChromaprintContext::new().unwrap();
    let samples = vec![0.0f32; 44100];

    // Too low
    assert!(matches!(
        ctx.generate_fingerprint(&samples, 4000, 1),
        Err(ChromaprintError::InvalidSampleRate(4000))
    ));

    // Too high
    assert!(matches!(
        ctx.generate_fingerprint(&samples, 384000, 1),
        Err(ChromaprintError::InvalidSampleRate(384000))
    ));
}

#[test]
fn test_audio_conversion_boundary_cases() {
    let test_cases = vec![
        (0.0f32, 0i16),           // Zero
        (1.0f32, 32767i16),       // Max positive
        (-1.0f32, -32767i16),     // Max negative
        (1.5f32, 32767i16),       // Clamp positive overflow
        (-1.5f32, -32768i16),     // Clamp negative overflow
    ];

    for (input, expected) in test_cases {
        let result = convert_f32_to_i16(&[input]);
        assert_eq!(result[0], expected, "Failed for input {}", input);
    }
}
```

### 9.2 Memory Leak Detection

**Tool:** Valgrind (Linux) or Instruments (macOS)

**Test Command:**
```bash
# Run under valgrind
valgrind --leak-check=full --show-leak-kinds=all \
    cargo test --package wkmp-ai --test test_chromaprint

# Expected output: "All heap blocks were freed -- no leaks are possible"
```

**Acceptance Criterion:** Zero memory leaks in all test scenarios

### 9.3 Integration Tests

**Test with Real Audio:**
- Use test MP3 file (e.g., 30 seconds of royalty-free music)
- Decode with symphonia
- Generate fingerprint
- Verify fingerprint matches known reference (from official Chromaprint tool)

**Reference Fingerprint Tool:**
```bash
# Generate reference using official fpcalc tool
fpcalc test_audio.mp3
```

---

## 10. Integration with WKMP-AI

### 10.1 Caller: Chromaprint Analyzer (Tier 1)

**File:** `wkmp-ai/src/extractors/chromaprint_analyzer.rs`

**Usage Pattern:**
```rust
use crate::ffi::chromaprint::{ChromaprintContext, ChromaprintError};

pub struct ChromaprintAnalyzer;

impl SourceExtractor for ChromaprintAnalyzer {
    fn extract(&self, segment: &AudioSegment) -> Result<ExtractionResult, ExtractionError> {
        // Create context (RAII ensures cleanup)
        let mut ctx = ChromaprintContext::new()
            .map_err(|e| ExtractionError::ChromaprintInitFailed(e))?;

        // Generate fingerprint
        let fingerprint = ctx.generate_fingerprint(
            &segment.samples,
            segment.sample_rate,
            segment.num_channels,
        ).map_err(|e| ExtractionError::ChromaprintFailed(e))?;

        // Return result with confidence=1.0 (fingerprint generation is deterministic)
        Ok(ExtractionResult {
            source: "chromaprint",
            data: ExtractionData::Fingerprint(fingerprint),
            confidence: 1.0,
        })
    }
}
```

### 10.2 Error Handling in Workflow

**Per-Passage Isolation:** Chromaprint errors for one passage do NOT fail entire import

**Confidence Impact:** If fingerprinting fails:
- Passage receives `fingerprint = NULL`
- Identity resolution confidence reduced (no AcoustID lookup)
- Metadata fusion uses only ID3 and AudioDerived sources

**Logging:** Chromaprint errors logged at `warn` level (not `error`, since graceful degradation is expected)

---

## 11. Build Configuration

### 11.1 build.rs

**Purpose:** Detect libchromaprint installation via pkg-config

**File:** `wkmp-ai/build.rs`

```rust
fn main() {
    // Check for libchromaprint
    match pkg_config::probe_library("libchromaprint") {
        Ok(lib) => {
            println!("cargo:warning=Found libchromaprint: {}", lib.version);
        }
        Err(e) => {
            println!("cargo:warning=libchromaprint not found via pkg-config: {}", e);
            println!("cargo:warning=Install libchromaprint-dev (Debian/Ubuntu) or chromaprint (Homebrew)");
            // Don't fail build - allow compilation without Chromaprint (graceful degradation)
        }
    }
}
```

**Graceful Degradation:** If libchromaprint not found, module compiles but returns runtime error on context creation

### 11.2 Installation Instructions

**Debian/Ubuntu:**
```bash
sudo apt-get install libchromaprint-dev
```

**macOS:**
```bash
brew install chromaprint
```

**Verification:**
```bash
pkg-config --modversion libchromaprint
# Expected: 1.5.1 or higher
```

---

## 12. Performance Characteristics

**Typical Performance:**
- **Fingerprint Generation:** ~0.5-1.0 seconds per passage (30-second audio segment)
- **Memory Usage:** ~10 MB per context (FFT buffers)
- **Throughput:** 30-60 passages/minute (single-threaded)

**Bottleneck:** FFT computation (handled by libchromaprint, optimized C code)

**Parallelization:** Multiple ChromaprintContext instances can run concurrently (no global state)

---

## 13. Security Considerations

### 13.1 Memory Safety

**Mitigations:**
- RAII wrapper prevents use-after-free
- Drop trait ensures context freed exactly once
- No manual memory management exposed to caller
- C string lifetime managed via immediate copy

### 13.2 Input Validation

**Validated Inputs:**
- Sample rate: 8000-192000 Hz
- Channel count: 1-2
- Sample data: No validation (Chromaprint handles gracefully)

**Attack Surface:** Minimal (no user input processed, audio already decoded by symphonia)

---

## 14. Future Enhancements (Out of Scope for TASK-002)

**Potential Improvements:**
1. SIMD optimization for f32 → i16 conversion (if profiling identifies bottleneck)
2. Streaming API (feed audio incrementally instead of entire segment)
3. Support for longer fingerprints (increase duration parameter)
4. Multi-algorithm support (TEST1, TEST3, TEST4)

**Not Planned:** These are optimizations beyond MVP requirements

---

## 15. References

**External Documentation:**
- Chromaprint C API: https://acoustid.org/chromaprint
- libchromaprint source: https://github.com/acoustid/chromaprint
- AcoustID algorithm details: https://acoustid.org/docs

**Internal Documentation:**
- IMPL012-acoustid_client.md (AcoustID API integration)
- REQ-AI-012-01 (audio segment format)
- TASK-002 (implementation task specification)

---

**Document Version:** 1.0
**Last Updated:** 2025-11-09
**Status:** Ready for TASK-002 implementation

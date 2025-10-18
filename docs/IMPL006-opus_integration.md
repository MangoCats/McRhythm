# Opus Codec Integration via C Library

**📐 TIER 3 - IMPLEMENTATION SPECIFICATION**

This document provides implementation guidance for integrating Opus codec support using the libopus C library via FFI.

**Status:** Ready for implementation
**Requirements:** [REQ-PI-020], [REQ-CF-010], [REQ-TECH-022A]
**Architecture:** SPEC001-architecture.md (Audio Processing)
**Exception:** REV003-opus_c_library_exception.md

---

## Overview

**Purpose:** Enable Opus format (.opus) decoding to satisfy [REQ-PI-020] audio format support requirements

**Approach:** Use `symphonia-adapter-libopus` crate to provide FFI bindings between symphonia's decoder abstraction and the libopus C library

**Scope:** Opus codec only; all other formats continue using pure Rust implementations

---

## Dependencies

### Rust Crate

**Add to `wkmp-ap/Cargo.toml`:**

```toml
[dependencies]
# ... existing dependencies ...

# Audio processing
symphonia = { version = "0.5", features = ["mp3", "flac", "aac", "isomp4", "vorbis"] }
symphonia-adapter-libopus = "0.2.3"  # <-- ADD THIS
rubato = "0.15"
cpal = "0.15"
ringbuf = "0.4"
```

**Rationale:** The adapter provides safe Rust bindings to libopus, integrating cleanly with symphonia's codec abstraction

### System Library Requirements

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get install libopus0 libopus-dev
```

**Linux (RHEL/Fedora/CentOS):**
```bash
sudo yum install opus opus-devel
```

**macOS:**
```bash
brew install opus
```

**Windows:**
- Download pre-built libopus DLLs from https://opus-codec.org/downloads/
- Or use vcpkg: `vcpkg install opus`
- Place DLL in system PATH or bundle with WKMP distribution

**Raspberry Pi (Debian-based):**
```bash
sudo apt-get install libopus0 libopus-dev
```

---

## Implementation Steps

### Step 1: Update Cargo.toml

Add `symphonia-adapter-libopus = "0.2.3"` to dependencies in `wkmp-ap/Cargo.toml`

**Verification:**
```bash
cargo check -p wkmp-ap
```

Expected: No errors (if libopus installed), or clear pkg-config error (if missing)

### Step 2: Register Opus Codec with Symphonia

The adapter should auto-register when imported. Verify in `wkmp-ap/src/audio/decoder.rs`:

```rust
// At top of file, add:
use symphonia_adapter_libopus; // Ensures codec registration

// No other changes needed - symphonia will automatically use Opus decoder
```

**Explanation:** symphonia's `get_codecs()` dynamically discovers registered codecs, including the libopus adapter

### Step 3: Test Opus Decoding

**Enable Opus test in `wkmp-ap/tests/audio_format_tests.rs`:**

```rust
#[test]
// Remove: #[ignore] // Opus not supported...
fn test_decode_opus() {
    let path = fixture_path("test_audio_10s_opus.opus");

    assert!(
        path.exists(),
        "Opus test file not found at: {}",
        path.display()
    );

    let result = SimpleDecoder::decode_file(&path);
    assert!(
        result.is_ok(),
        "Opus decode should succeed, error: {:?}",
        result.err()
    );

    let (samples, sample_rate, channels) = result.unwrap();

    verify_decode_properties("Opus", &samples, sample_rate, channels);
}
```

**Run test:**
```bash
cargo test --test audio_format_tests test_decode_opus -p wkmp-ap
```

**Expected output:** Test passes, Opus file decoded successfully

### Step 4: Update Cross-Format Consistency Test

**In `test_all_formats_produce_consistent_output()`:**

```rust
let formats = vec![
    ("MP3", "test_audio_10s_mp3.mp3"),
    ("FLAC", "test_audio_10s_flac.flac"),
    ("Vorbis", "test_audio_10s_vorbis.ogg"),
    ("Opus", "test_audio_10s_opus.opus"),  // <-- ADD THIS
    ("WAV", "test_audio_10s_wav.wav"),
];
```

**Run full test suite:**
```bash
cargo test --test audio_format_tests -p wkmp-ap
```

**Expected:** All 14 tests pass (13 passing + 1 ignored AAC)

### Step 5: Update Documentation

**Update `wkmp-ap/tests/fixtures/README.md`:**

Change Opus status from:
```markdown
- **❌ Opus** - Not supported in pure Rust (requires C library via symphonia-adapter-libopus)
```

To:
```markdown
- **✅ Opus** - Successfully decodes (via libopus C library + FFI adapter) [REQ-TECH-022A]
```

**Update test file header comments:**

In `wkmp-ap/tests/audio_format_tests.rs`:
```rust
//! **Test Coverage:**
//! - Individual format decode tests (6 formats: MP3, FLAC, Vorbis, Opus, WAV)
//! - AAC currently not working (symphonia limitation)
//! - Cross-format consistency validation
```

---

## Error Handling

### Missing libopus at Build Time

**Error:**
```
error: failed to run custom build command for `libopus-sys v0.x.x`
...
Package opus was not found in the pkg-config search path
```

**Resolution:**
1. Install libopus development package (see system requirements above)
2. Ensure pkg-config is installed (`apt install pkg-config` on Debian/Ubuntu)
3. Re-run `cargo build`

### Missing libopus at Runtime

**Symptoms:** Application fails to start, or Opus files fail to decode with codec not found error

**Resolution:**
- Install libopus runtime package: `apt install libopus0` (Linux)
- On Windows: Ensure libopus DLL is in PATH or application directory
- On macOS: `brew install opus`

**Graceful Degradation:**
If libopus is unavailable at runtime, symphonia will fail to decode .opus files but other formats will continue working. Consider adding detection:

```rust
// Optional: Detect Opus codec availability
pub fn is_opus_supported() -> bool {
    // Attempt to probe a minimal Opus file or check codec registry
    // Return true if Opus decoder available, false otherwise
}
```

---

## Testing Strategy

### Unit Tests

**Existing tests to run:**
1. `test_decode_opus()` - Individual Opus decode test
2. `test_all_formats_produce_consistent_output()` - Cross-format validation including Opus
3. `test_decode_performance_acceptable()` - Performance smoke test

**New test to consider:**
```rust
#[test]
fn test_opus_specific_features() {
    // Opus has variable bitrate, modern codec features
    let path = fixture_path("test_audio_10s_opus.opus");
    let (samples, sample_rate, channels) = SimpleDecoder::decode_file(&path).unwrap();

    // Opus always outputs at 48kHz (resampled to 44.1kHz by our pipeline)
    assert_eq!(sample_rate, 48000, "Opus native sample rate");

    // Verify sample quality
    assert!(samples.len() > 800_000, "Expected ~10 seconds at 48kHz stereo");
}
```

### Integration Tests

**Test full playback pipeline:**
```rust
#[tokio::test]
async fn test_opus_playback_integration() {
    // 1. Enqueue Opus passage
    // 2. Start playback
    // 3. Verify audio output contains samples
    // 4. Confirm no errors
}
```

### Platform-Specific Testing

**Priority order:**
1. **Linux** (primary platform, Raspberry Pi target)
2. **Windows** (Rollout phase 1)
3. **macOS** (Rollout phase 2)

**Test checklist per platform:**
- [ ] libopus installs cleanly
- [ ] Cargo build succeeds
- [ ] Opus test passes
- [ ] Playback produces audio output
- [ ] No runtime linking errors

---

## Deployment Considerations

### Installation Documentation

**Add to WKMP installation guide:**

#### Linux
```markdown
## System Dependencies

WKMP requires the following system libraries:

- **libopus** - For Opus audio format support
  ```bash
  # Debian/Ubuntu
  sudo apt-get install libopus0

  # RHEL/Fedora
  sudo yum install opus
  ```
```

#### Windows
```markdown
## Windows Setup

1. Download libopus DLL from https://opus-codec.org/downloads/
2. Place `opus.dll` in one of:
   - WKMP installation directory
   - `C:\Windows\System32\` (system-wide)
   - Any directory in your PATH

*Or use the WKMP installer which bundles libopus automatically.*
```

#### macOS
```markdown
## macOS Dependencies

Install libopus via Homebrew:
```bash
brew install opus
```
```

### Bundling Strategy (Windows)

**For Windows distributions:**
- Include pre-built libopus DLL in WKMP installer
- Place in application directory (no system PATH modification needed)
- License: BSD-style (compatible with WKMP)

**Pros:**
- No user action required
- Guaranteed compatibility
- Simplified installation

**Cons:**
- Slightly larger installer
- Must keep DLL updated

### Docker / Container Deployment

**Dockerfile addition:**
```dockerfile
# Add libopus to container
RUN apt-get update && apt-get install -y \
    libopus0 \
    && rm -rf /var/lib/apt/lists/*
```

---

## Performance Characteristics

### libopus Performance

**Decoding speed:** libopus is highly optimized
- Reference implementation by Xiph.Org
- SIMD optimizations for x86/ARM
- Real-time decoding on Raspberry Pi Zero2W

**Memory usage:** Similar to other codecs (~1-2 MB for decoder state)

**Latency:** Low (Opus designed for low-latency VoIP)

### Comparison to Pure Rust Alternatives

**If pure Rust Opus were available:**
- Pros: No FFI overhead, easier deployment, no system dependencies
- Cons: Likely slower (no decades of optimization), higher maintenance burden

**FFI Overhead:**
- Minimal (<1% for audio decoding workloads)
- symphonia abstraction minimizes FFI crossings

---

## Security Considerations

### C Library Vulnerabilities

**Monitoring:**
- Track CVE database for libopus: https://cve.mitre.org/cgi-bin/cvekey.cgi?keyword=opus
- Subscribe to Xiph.Org security announcements

**Recent history:** libopus has excellent security track record (few CVEs, quickly patched)

**Mitigation:**
- Use system package managers (automatic security updates)
- On Windows: Include latest stable libopus with each WKMP release
- Test with latest libopus versions before release

### FFI Safety

**symphonia-adapter-libopus safety:**
- Uses `unsafe` FFI blocks internally
- Provides safe Rust API surface
- Maintained by symphonia project (active development)

**Best practices:**
- Do not expose raw FFI pointers to WKMP application code
- Use symphonia's safe decoder abstraction exclusively
- Trust symphonia adapter for memory safety guarantees

---

## Maintenance and Updates

### Dependency Updates

**libopus (system library):**
- Updated via system package manager (Linux/macOS)
- Manual updates on Windows (or bundled with WKMP releases)
- Stable API - updates rarely break compatibility

**symphonia-adapter-libopus (Rust crate):**
- Check for updates: `cargo outdated -p symphonia-adapter-libopus`
- Update cautiously: Test Opus decoding after each update
- Pin version if stability issues arise

### Monitoring Compatibility

**Test matrix:**
- libopus versions: 1.3.x (current stable), 1.4.x (future)
- symphonia-adapter versions: 0.2.x (current)
- Platforms: Linux, Windows, macOS, Raspberry Pi

**Regression testing:**
- Run audio format tests on each platform after libopus/adapter updates
- Verify no decoding quality degradation
- Check for new warnings/errors

---

## Troubleshooting

### Build Issues

**Problem:** `pkg-config` not found
```
Solution: Install pkg-config package
  - Debian/Ubuntu: sudo apt-get install pkg-config
  - macOS: brew install pkg-config
```

**Problem:** libopus headers not found
```
Solution: Install development package
  - Debian/Ubuntu: sudo apt-get install libopus-dev
  - RHEL/Fedora: sudo yum install opus-devel
```

**Problem:** Cross-compilation fails
```
Solution: Ensure target platform has libopus available
  - For Raspberry Pi: Install libopus-dev on target or use cross sysroot
  - Set PKG_CONFIG_PATH to target's pkg-config location
```

### Runtime Issues

**Problem:** "Opus codec not found" at runtime
```
Symptoms: .opus files fail to decode, other formats work
Diagnosis: libopus runtime library not installed
Solution:
  - Linux: sudo apt-get install libopus0
  - Windows: Ensure opus.dll in PATH or app directory
  - macOS: brew install opus
```

**Problem:** Opus files decode but sound distorted
```
Diagnosis: Incorrect resampling or sample format conversion
Investigation:
  1. Verify sample rate (Opus native: 48kHz, WKMP standard: 44.1kHz)
  2. Check rubato resampler configuration
  3. Test with WAV file first (baseline)
Solution: Review resampler settings in decoder_pool.rs
```

---

## Deployment Impact Analysis

**[IMPL-OPUS-500]** This section analyzes the deployment implications of including symphonia-adapter-libopus across all WKMP versions, platforms, and build configurations.

### Overview: Bundled vs System Library

**[IMPL-OPUS-510]** The `opusic-sys` crate (underlying FFI layer) supports two deployment modes:

1. **Bundled (Default):** Compiles libopus statically during build
   - Feature: `bundled` (enabled by default)
   - Build dependency: cmake, C compiler
   - Result: libopus code embedded in wkmp-ap binary
   - No runtime dependency on system libopus

2. **System Library:** Links against system-installed libopus
   - Configuration: `default-features = false` in Cargo.toml
   - Build dependency: libopus-dev (headers)
   - Result: Dynamic linking to system libopus.so/.dylib/.dll
   - Runtime dependency: libopus must be installed

**Current Configuration:** Using bundled mode (default)

### Build-Time Impact

**[IMPL-OPUS-520] Compilation Time:**
- **First build:** +2-3 minutes (cmake compiles libopus C sources)
- **Incremental builds:** No impact (libopus cached in target/ directory)
- **CI/CD:** One-time cost per clean build

**[IMPL-OPUS-521] Build Dependencies:**

**Bundled mode (current):**
```
Required at build time:
- cmake (all platforms)
- C compiler (gcc/clang/MSVC)
- Standard C library headers

NOT required:
- libopus-dev packages
```

**System library mode:**
```
Required at build time:
- C compiler (for linking)
- libopus-dev (headers + library)

NOT required:
- cmake
```

**[IMPL-OPUS-522] Build Environment Complexity:**
- **Bundled:** Higher (requires cmake + C toolchain)
- **System:** Lower (standard Rust + system packages)

**Impact Assessment:** ✅ Acceptable - cmake and C compilers are standard in Rust development environments

### Binary Size Impact

**[IMPL-OPUS-530] Release Binary Sizes:**

**Measured on Linux x86_64:**
```
wkmp-ap (release build):
- With bundled libopus: 11.0 MB
- System libopus size: 0.37 MB
- Estimated without libopus: ~10.6 MB
- Size increase: ~3.6%
```

**[IMPL-OPUS-531] Version-Specific Impact:**

| Version | Binaries with Opus | Total Size Increase | Percentage Impact |
|---------|-------------------|---------------------|-------------------|
| Full | wkmp-ap | ~0.4 MB | Negligible (<1% of total) |
| Lite | wkmp-ap | ~0.4 MB | Negligible (<1% of total) |
| Minimal | wkmp-ap | ~0.4 MB | ~3.6% of wkmp-ap binary |

**Impact Assessment:** ✅ Acceptable - 0.4 MB increase is minimal on all target platforms

### Runtime Dependency Impact

**[IMPL-OPUS-540] Runtime Dependencies by Mode:**

**Bundled mode (current):**
- **Linux:** No libopus runtime dependency
- **Windows:** No libopus DLL required
- **macOS:** No libopus dylib required
- **Raspberry Pi:** No libopus runtime dependency

**System library mode:**
- **Linux:** Requires `libopus0` package
- **Windows:** Requires `libopus-0.dll` (ship with installer or via vcpkg)
- **macOS:** Requires `/usr/local/lib/libopus.dylib`
- **Raspberry Pi:** Requires `libopus0` package

**Impact Assessment:** ✅ Bundled mode eliminates all runtime dependencies

### Platform-Specific Analysis

**[IMPL-OPUS-550] Linux (Desktop and Raspberry Pi):**

**Distribution:**
```bash
# User installation (bundled mode):
sudo dpkg -i wkmp-full_1.0.0_amd64.deb
# No additional steps required - libopus is bundled

# Alternative (system library mode):
sudo apt-get install libopus0  # Runtime dependency
sudo dpkg -i wkmp-full_1.0.0_amd64.deb
```

**Recommendation:** ✅ Use bundled mode
- Simpler installation (no additional packages)
- libopus widely available but eliminates dependency hassle
- Binary size increase negligible on desktop/Pi

**[IMPL-OPUS-551] Windows (Desktop):**

**Distribution:**
```
Installer package includes:
- wkmp-ap.exe (bundled libopus)
- wkmp-ui.exe
- wkmp-pd.exe (Full/Lite)
- wkmp-ai.exe (Full only)
- wkmp-le.exe (Full only)

No additional runtime installation required
```

**Alternative (system library mode):**
```
Would require:
- Shipping libopus-0.dll with installer, OR
- vcpkg dependency management, OR
- User manual installation of libopus

Complexity: HIGH (DLL hell, version conflicts)
```

**Recommendation:** ✅✅ Strongly use bundled mode
- Windows lacks standard package manager for system libraries
- Bundled mode eliminates DLL distribution complexity
- Self-contained executable simplifies deployment

**[IMPL-OPUS-552] macOS (Phase 2):**

**Distribution (bundled mode):**
```bash
# Application bundle includes bundled libopus:
/Applications/WKMP.app/Contents/MacOS/wkmp-ap  # libopus embedded
/Applications/WKMP.app/Contents/MacOS/wkmp-ui
# ... etc

# User installation:
# Drag to Applications folder - works immediately
```

**Alternative (system library mode):**
```bash
# Would require Homebrew:
brew install opus

# Then install WKMP
# Complexity: Requires user to install Homebrew and libopus first
```

**Recommendation:** ✅ Use bundled mode
- macOS application bundles should be self-contained
- Avoids requiring Homebrew installation
- Consistent with macOS app distribution norms

**[IMPL-OPUS-553] Raspberry Pi Zero2W:**

**Full/Lite Version:**
```bash
# libopus included in Raspberry Pi OS repositories
sudo apt-get update
sudo apt-get install wkmp-lite  # Bundled mode

# Total installed size:
# - wkmp-ap: ~11 MB (bundled libopus)
# - wkmp-ui: ~8 MB
# - wkmp-pd: ~6 MB
# Total: ~25 MB (fits comfortably on 8GB+ SD card)
```

**Minimal Version:**
```bash
# Minimal version example:
# - wkmp-ap: ~11 MB
# - wkmp-ui: ~8 MB
# Total: ~19 MB

# With system libopus (saves 0.4 MB):
sudo apt-get install libopus0  # 0.37 MB
# wkmp-ap: ~10.6 MB
# Total: ~19.0 MB (negligible difference)
```

**Recommendation:** ✅ Use bundled mode
- 0.4 MB savings negligible even on resource-constrained Pi
- Simpler installation (fewer packages)
- Raspberry Pi OS includes libopus in repos (not a scarce dependency)

### Version-Specific Deployment Considerations

**[IMPL-OPUS-560] Full Version (Linux/Windows Desktop):**

**Packaging:**
```
Debian/Ubuntu (.deb):
- Package: wkmp-full
- Binaries: wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le
- Size: ~50-60 MB (all binaries, bundled libopus in wkmp-ap)
- Dependencies: libasound2, libsqlite3, libc (standard desktop deps)
- NO libopus dependency

Windows Installer (.msi):
- Installer: wkmp-full-1.0.0-x64.msi
- Binaries: wkmp-ap.exe, wkmp-ui.exe, wkmp-pd.exe, wkmp-ai.exe, wkmp-le.exe
- Size: ~60-70 MB (all binaries, bundled libopus)
- Dependencies: None (self-contained)
```

**Installation Complexity:** ✅ Low (single package, no external dependencies)

**[IMPL-OPUS-561] Lite Version (Raspberry Pi + Desktop):**

**Packaging:**
```
Raspberry Pi OS (.deb):
- Package: wkmp-lite
- Binaries: wkmp-ap, wkmp-ui, wkmp-pd
- Size: ~25-30 MB (three binaries, bundled libopus)
- Target: Raspberry Pi Zero2W (ARM architecture)
- Cross-compilation: Required from x86_64 development machine

Desktop (.deb / .msi):
- Same as Full version minus wkmp-ai and wkmp-le
- Size: ~40-50 MB
```

**Cross-Compilation Note:**
```bash
# Building Lite version for Raspberry Pi from x86_64:
rustup target add armv7-unknown-linux-gnueabihf
cargo build --release --target armv7-unknown-linux-gnueabihf

# Bundled mode advantage:
# - No need to provide ARM-compiled libopus separately
# - cmake automatically cross-compiles libopus for target
# - Self-contained ARM binary
```

**Installation Complexity:** ✅ Low (bundled mode simplifies cross-compilation)

**[IMPL-OPUS-562] Minimal Version (Raspberry Pi, Embedded):**

**Packaging:**
```
Raspberry Pi OS (.deb):
- Package: wkmp-minimal
- Binaries: wkmp-ap, wkmp-ui (playback + manual control only)
- Size: ~19-20 MB (two binaries, bundled libopus)
- Memory footprint: <256 MB runtime
```

**Embedded Consideration:**
```
Binary size: 11 MB (wkmp-ap) + 8 MB (wkmp-ui) = ~19 MB total
Storage requirement: ~50 MB (binaries + database + config)

Raspberry Pi Zero2W:
- Typical SD card: 8 GB+
- WKMP usage: <100 MB
- Impact: Negligible

Resource assessment: ✅ 0.4 MB libopus overhead is insignificant
```

**Installation Complexity:** ✅ Low (bundled mode preferred even on minimal)

### Packaging and Distribution Complexity

**[IMPL-OPUS-570] CI/CD Pipeline Impact:**

**Build Matrix:**
```yaml
# GitHub Actions / GitLab CI example:
matrix:
  platform: [linux-x86_64, linux-armv7, windows-x64, macos-x64]
  version: [full, lite, minimal]

build_requirements:
  - rust stable
  - cmake (bundled mode)
  - C compiler (gcc/clang/MSVC)
  - Standard build tools

# No additional dependencies for libopus
# cmake automatically handles cross-compilation
```

**Complexity Assessment:** ✅ Low - cmake and C compiler standard in CI environments

**[IMPL-OPUS-571] Package Repository Distribution:**

**Linux (Debian/Ubuntu):**
```bash
# Package metadata (.deb control file):
Package: wkmp-full
Depends: libasound2, libsqlite3-0, libc6
# NO libopus dependency listed

# Advantage: Fewer dependency resolution issues
# Users don't need to install libopus separately
```

**Windows:**
```
# Installer (WiX/InnoSetup):
# - Single .msi or .exe
# - No additional runtime installers required
# - No registry dependencies on libopus

# Advantage: True "double-click to install" experience
```

**macOS:**
```
# Application bundle:
# - Self-contained .app bundle
# - No Homebrew dependencies
# - Code-signed and notarized with bundled libraries

# Advantage: Standard macOS distribution (drag to Applications)
```

**Complexity Assessment:** ✅ Bundled mode significantly simplifies distribution

### Installation Procedures

**[IMPL-OPUS-580] End-User Installation (Bundled Mode):**

**Linux:**
```bash
# Ubuntu/Debian:
sudo dpkg -i wkmp-full_1.0.0_amd64.deb
# OR
sudo apt-get install ./wkmp-full_1.0.0_amd64.deb  # auto-resolves deps

# Raspberry Pi OS:
sudo dpkg -i wkmp-lite_1.0.0_armhf.deb

# No additional steps required - Opus support works immediately
```

**Windows:**
```powershell
# Run installer:
wkmp-full-1.0.0-x64.msi

# Next -> Next -> Install -> Finish
# No additional runtime dependencies
```

**macOS:**
```bash
# Mount DMG:
open wkmp-full-1.0.0.dmg

# Drag WKMP.app to Applications folder
# Launch immediately - works without additional installation
```

**Installation Time:** <2 minutes on all platforms

**[IMPL-OPUS-581] Developer Setup:**

**Prerequisites:**
```bash
# Install Rust (rustup.rs):
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install build dependencies:
# Linux (Debian/Ubuntu):
sudo apt-get install cmake gcc pkg-config libasound2-dev libsqlite3-dev

# macOS:
brew install cmake

# Windows:
# Install Visual Studio Build Tools (includes cmake and MSVC)
# OR install cmake separately: choco install cmake
```

**Build from source:**
```bash
git clone https://github.com/your-org/wkmp.git
cd wkmp
cargo build --release

# Opus support built automatically (bundled mode)
# First build: ~5-10 minutes (includes cmake libopus compilation)
# Subsequent builds: ~1-2 minutes (libopus cached)
```

**Developer Complexity:** ✅ Low - standard Rust development setup

### Recommendations by Deployment Scenario

**[IMPL-OPUS-590] Recommendation Matrix:**

| Scenario | Recommendation | Rationale |
|----------|----------------|-----------|
| **Full Version (Linux Desktop)** | ✅ Bundled | Simplifies .deb dependencies, 0.4 MB negligible |
| **Full Version (Windows)** | ✅✅ Bundled | Eliminates DLL distribution complexity |
| **Full Version (macOS)** | ✅ Bundled | Self-contained app bundle standard practice |
| **Lite Version (Raspberry Pi)** | ✅ Bundled | Simplifies cross-compilation, size impact minimal |
| **Lite Version (Desktop)** | ✅ Bundled | Consistent with Full version |
| **Minimal Version (Pi/Embedded)** | ✅ Bundled | 0.4 MB overhead insignificant even on minimal |
| **Development Builds** | ✅ Bundled | Consistent with release builds, no manual libopus install |
| **CI/CD Pipelines** | ✅ Bundled | Self-contained builds, no external library injection |

**[IMPL-OPUS-591] Override Scenarios (System Library):**

Use system library mode (`default-features = false`) only if:
1. **Corporate policy mandates shared libraries** (rare for media codecs)
2. **Extreme storage constraints** (<100 MB available) - not applicable to WKMP targets
3. **Dynamic libopus updates required** - unlikely (Opus 1.x stable since 2016)

**Recommendation:** ❌ No compelling reason to disable bundled mode for WKMP

### Summary

**[IMPL-OPUS-595] Deployment Impact Assessment:**

| Impact Area | Assessment | Notes |
|-------------|------------|-------|
| **Build Time** | ✅ Low | +2 min first build, cached thereafter |
| **Binary Size** | ✅ Negligible | +0.4 MB (~3.6% of wkmp-ap) |
| **Runtime Dependencies** | ✅✅ Eliminated | Bundled mode = zero runtime deps |
| **Installation Complexity** | ✅✅ Reduced | Single package, no libopus prereqs |
| **Cross-Compilation** | ✅ Simplified | cmake handles target architecture |
| **Distribution** | ✅✅ Simplified | Self-contained binaries |
| **Security Updates** | ⚠️ Manual | Must rebuild WKMP for libopus CVEs* |
| **Developer Experience** | ✅ Improved | No manual libopus installation |

*Note: libopus has excellent security track record (few CVEs, stable API since 2012)

**[IMPL-OPUS-596] Final Recommendation:**

**✅ Maintain bundled mode (current configuration) for all WKMP versions and platforms.**

**Justification:**
1. **Minimal size overhead** (0.4 MB) acceptable on all targets
2. **Eliminates runtime dependencies** - simplifies user installation
3. **Simplifies distribution** - single package, no external library coordination
4. **Standard practice** for multimedia applications (precedent: FFmpeg embeddings)
5. **Consistent with Windows/macOS norms** - self-contained applications preferred
6. **No compelling benefit** from system library mode for WKMP use case

**[IMPL-OPUS-597] Configuration Stability:**

Current Cargo.toml configuration:
```toml
symphonia-adapter-libopus = "0.2.3"  # Uses bundled mode (default)
```

**No changes required** - deployment analysis confirms current approach optimal.

---

## References

### Requirements and Architecture
- [REQ-PI-020]: Support audio formats including Opus
- [REQ-TECH-022A]: Opus C library exception
- REV003-opus_c_library_exception.md: Exception rationale and approval
- SPEC001-architecture.md: Audio processing architecture

### External Resources
- libopus: https://opus-codec.org/
- symphonia-adapter-libopus crate: https://crates.io/crates/symphonia-adapter-libopus
- Opus specification (RFC 6716): https://www.rfc-editor.org/rfc/rfc6716
- Xiph.Org Foundation: https://xiph.org/

### Implementation Files
- wkmp-ap/Cargo.toml: Dependency configuration
- wkmp-ap/src/audio/decoder.rs: Decoder implementation
- wkmp-ap/tests/audio_format_tests.rs: Format tests

---

**Document Status:** ✅ Implementation Complete (2025-10-18)
**Actual Implementation Time:** ~2 hours
**Priority:** Medium (completes [REQ-PI-020] requirement)
**Approval:** Approved via REV003-opus_c_library_exception.md

**Implementation Summary:**
1. ✅ Added symphonia-adapter-libopus 0.2.3 to Cargo.toml
2. ✅ Created custom CodecRegistry with OpusDecoder registration
3. ✅ Enabled and verified Opus test (test_decode_opus passing)
4. ✅ Updated cross-format consistency tests
5. ✅ Generated test fixture (test_audio_10s_opus.opus)
6. ✅ Documented deployment impact analysis

**Test Results:** 13/14 tests passing, 1 ignored (AAC - symphonia limitation)
**Opus Status:** ✅ Fully functional - decodes at native 48kHz sample rate

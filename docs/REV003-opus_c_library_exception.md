# Opus C Library Exception - Requirements Amendment

**📜 TIER R - REVIEW & CHANGE CONTROL**

This document records an architectural discovery during implementation that requires amendment to requirements and architecture documents.

**Document Type:** REV (Revision/Review) - Architectural change request and rationale
**Status:** ✅ Accepted (2025-10-18)
**Impact:** Requirements amendment, architecture clarification
**Related Requirements:** [REQ-PI-020], [REQ-CF-010], [REQ-TECH-021], [REQ-TECH-022]
**Related Architecture:** [ARCH-AUDIO-010] (SPEC001-architecture.md line 1301)

---

## Executive Summary

During implementation of audio format decoder tests (2025-10-18), it was discovered that **pure Rust Opus codec support is not available** in symphonia 0.5.x, contrary to what was documented in SPEC001-architecture.md line 1301. To satisfy [REQ-PI-020] which mandates Opus format support, this document proposes a **requirements exception** to allow use of the `symphonia-adapter-libopus` crate, which provides Rust FFI bindings to the libopus C library.

**Decision:** ✅ **APPROVED** - Exception granted for Opus C library integration

**Rationale:** Opus format support is a documented requirement that cannot be satisfied with pure Rust libraries. The exception is narrowly scoped, has precedent (Chromaprint), and maintains cross-platform compatibility.

---

## Background

### Original Requirements

**[REQ-PI-020]** Support audio formats: MP3, FLAC, OGG, M4A, AAC, OPUS, WAV

**[REQ-CF-010]** Plays passages from local files (.mp3, .opus, .aac, .flac and similar)

**[REQ-TECH-021]** Rust

**[REQ-TECH-022]** Single-stream audio architecture (symphonia for decoding, rubato for resampling, cpal for output)

### Architectural Assumption (Incorrect)

**SPEC001-architecture.md line 1301:**
> "Audio decoding: symphonia 0.5.x (pure Rust, supports MP3, FLAC, AAC, Vorbis, Opus, etc.)"

### Discovery During Implementation

**Date:** 2025-10-18
**Context:** Implementation of `wkmp-ap/tests/audio_format_tests.rs`
**Finding:** Symphonia 0.5.5 **does not include Opus codec support**

**Evidence:**
```bash
$ cargo info symphonia
features:
 +default = [adpcm, flac, mkv, ogg, pcm, vorbis, wav]
  adpcm   = [symphonia-codec-adpcm]
  flac    = [symphonia-bundle-flac]
  mkv     = [symphonia-format-mkv]
  ogg     = [symphonia-format-ogg]
  pcm     = [symphonia-codec-pcm]
  vorbis  = [symphonia-codec-vorbis]
  wav     = [symphonia-format-riff/wav]
  # No 'opus' feature available
```

**Available Alternative:**
```bash
$ cargo search symphonia | grep opus
symphonia-adapter-libopus = "0.2.3"    # Adapter to use libopus with Symphonia
```

**Description:** FFI adapter wrapping libopus C library for use with symphonia

---

## Problem Statement

**The conflict:**
1. [REQ-PI-020] and [REQ-CF-010] **require** Opus format support
2. [REQ-TECH-022] specifies symphonia for audio decoding
3. Symphonia 0.5.x does **not** include pure Rust Opus codec
4. SPEC001-architecture.md incorrectly stated Opus support exists

**Options:**
1. ❌ Remove Opus from requirements (violates user needs)
2. ❌ Wait for pure Rust Opus implementation (timeline unknown, may be years)
3. ✅ **Allow C library exception for Opus via symphonia-adapter-libopus**
4. ❌ Switch to different audio framework (massive rework, other trade-offs)

---

## Proposed Solution

### Requirements Exception

**Create new requirement: [REQ-TECH-022A]**

Add to REQ001-requirements.md under [REQ-TECH-022]:

```markdown
**[REQ-TECH-022]** Single-stream audio architecture (symphonia for decoding, rubato for resampling, cpal for output)
  - **[REQ-TECH-022A]** Exception: Opus codec support via C library FFI
    - **Rationale:** Pure Rust Opus implementation not available in symphonia 0.5.x
    - **Implementation:** `symphonia-adapter-libopus` crate providing FFI bindings to libopus
    - **Scope:** Limited to Opus codec only; all other codecs use pure Rust implementations
    - **Precedent:** Consistent with existing Chromaprint C library usage for fingerprinting
    - **Deployment:** Requires libopus system library installation on target platforms
```

### Architecture Correction

**Update SPEC001-architecture.md line 1301:**

**Before:**
```markdown
- Audio decoding: symphonia 0.5.x (pure Rust, supports MP3, FLAC, AAC, Vorbis, Opus, etc.)
```

**After:**
```markdown
- Audio decoding: symphonia 0.5.x (primarily pure Rust)
  - Pure Rust codecs: MP3, FLAC, AAC (M4A), Vorbis (OGG), WAV (PCM)
  - FFI-based codecs: Opus (via symphonia-adapter-libopus + libopus C library)
  - [REQ-TECH-022A]: Opus exception approved for C library integration
```

---

## Justification and Precedent

### 1. Requirements Take Precedence

Per GOV001-document_hierarchy.md, Tier 1 (Requirements) documents define **WHAT** the system must do. [REQ-PI-020] mandates Opus support. Implementation details must satisfy requirements, not constrain them.

### 2. Existing Precedent for C Libraries

**SPEC001-architecture.md already documents C library usage:**

**Line 1317-1318:**
```markdown
- Chromaprint C library
- Rust FFI bindings (custom or via existing crate)
```

**Observation:** The project already uses C libraries with FFI when necessary. Opus is a similar case where no mature pure Rust alternative exists.

### 3. Narrow Scope of Exception

**Exception characteristics:**
- **Limited to one codec:** Opus only
- **Well-maintained C library:** libopus is the reference implementation, actively maintained by Xiph.Org Foundation
- **Stable FFI:** symphonia-adapter-libopus provides safe Rust bindings
- **Cross-platform:** libopus available on all target platforms (Linux, Windows, macOS, Raspberry Pi)
- **No architectural impact:** Integrates cleanly with symphonia's decoder abstraction

### 4. Alternative Solutions Are Impractical

**Pure Rust Opus implementation:**
- No production-ready pure Rust Opus decoder exists as of 2025
- Opus is a complex codec (RFC 6716, 108 pages)
- Developing in-house would take months/years and introduce maintenance burden
- Reference libopus implementation is already battle-tested

**Waiting for third-party pure Rust implementation:**
- Timeline unknown (could be years)
- Delays delivery of [REQ-PI-020]
- User need is immediate (Opus is a modern, efficient codec)

**Switching audio frameworks:**
- GStreamer: Heavier weight, C-based
- FFmpeg: C-based
- No pure Rust alternative with comparable codec support exists

### 5. Deployment Considerations

**System library requirements (libopus):**

| Platform | Installation Method | Availability |
|----------|---------------------|--------------|
| **Linux** | `apt install libopus0` (Debian/Ubuntu)<br>`yum install opus` (RHEL/Fedora) | ✅ Standard repos |
| **Windows** | Pre-built DLLs available<br>Can be bundled with WKMP installer | ✅ Easy distribution |
| **macOS** | `brew install opus` | ✅ Homebrew |
| **Raspberry Pi** | `apt install libopus0` | ✅ Standard repos |

**Observation:** libopus is widely available and easy to deploy on all target platforms.

---

## Trade-offs and Risks

### Benefits ✅

1. **Satisfies requirements:** Delivers mandated Opus support
2. **Best-in-class codec:** Uses reference libopus implementation (highest quality, performance)
3. **Minimal code impact:** Clean integration via symphonia adapter
4. **Consistent precedent:** Follows existing Chromaprint C library pattern
5. **User benefit:** Opus is a modern, high-efficiency codec (better than MP3/Vorbis at low bitrates)

### Drawbacks ⚠️

1. **External dependency:** Requires libopus system library installation
2. **Not pure Rust:** Introduces FFI surface and C library dependency
3. **Build complexity:** Slightly more complex than pure Rust (needs pkg-config or vcpkg)
4. **Deployment note required:** Installation documentation must mention libopus

### Risk Mitigation

**Deployment risk (missing libopus):**
- **Mitigation:** Document libopus requirement in installation guides
- **Fallback:** WKMP can still play other formats if libopus unavailable
- **Detection:** Cargo build will fail with clear error if libopus not found

**Security risk (C library vulnerabilities):**
- **Mitigation:** libopus is actively maintained by Xiph.Org Foundation
- **Monitoring:** Track CVE database for libopus security advisories
- **Updates:** System package managers handle libopus security updates

**Maintenance risk (FFI bindings):**
- **Mitigation:** symphonia-adapter-libopus is maintained by symphonia project
- **Stability:** libopus API is stable (1.x series since 2013)

---

## Implementation Impact

### Files to Modify

**1. docs/REQ001-requirements.md**
- Add [REQ-TECH-022A] exception under [REQ-TECH-022]

**2. docs/SPEC001-architecture.md**
- Correct line 1301 audio decoding section
- Clarify pure Rust vs FFI-based codecs
- Reference [REQ-TECH-022A]

**3. wkmp-ap/Cargo.toml**
- Add dependency: `symphonia-adapter-libopus = "0.2.3"`
- Update features/dependencies section

**4. wkmp-ap/src/audio/decoder.rs** (if needed)
- Verify Opus decoding works with adapter (minimal changes expected)

**5. Installation Documentation** (to be created)
- Document libopus system library requirement
- Provide installation commands per platform

**6. wkmp-ap/tests/audio_format_tests.rs**
- Remove `#[ignore]` from `test_decode_opus()` test
- Update comments to reflect C library exception

### Implementation Effort

**Estimated time:** 2-3 hours
- Requirements/architecture updates: 30 minutes
- Cargo.toml integration: 15 minutes
- Testing and verification: 1 hour
- Documentation updates: 30-60 minutes

---

## Alternatives Considered and Rejected

### Alternative 1: Remove Opus Requirement
**Rejected:** Opus is a superior codec for many use cases (low bitrate streaming, voice content). Removing it degrades user experience.

### Alternative 2: Wait for Pure Rust Implementation
**Rejected:** Indefinite timeline blocks delivery. No active pure Rust Opus projects identified.

### Alternative 3: Fork and Implement Pure Rust Opus
**Rejected:** Massive scope (months of work), ongoing maintenance burden, high risk of bugs/incompatibility.

### Alternative 4: Use FFmpeg for All Codecs
**Rejected:** FFmpeg is entirely C-based, contradicting [REQ-TECH-021] Rust requirement. Would require wrapping entire audio pipeline in FFI.

### Alternative 5: Use GStreamer
**Rejected:** Similar to FFmpeg - heavier C dependency, not aligned with Rust-first approach.

---

## Decision

**✅ APPROVED - Requirements Exception Granted**

**Decision Authority:** Technical Lead / Project Architect
**Date:** 2025-10-18

**Scope of Exception:**
- **What:** Allow FFI integration of libopus C library via `symphonia-adapter-libopus` crate
- **Why:** Pure Rust Opus implementation unavailable; requirement [REQ-PI-020] mandates Opus support
- **Limitations:** Exception applies **only to Opus codec**; all other audio formats continue using pure Rust
- **Precedent:** Consistent with existing Chromaprint C library usage documented in SPEC001-architecture.md

**Conditions:**
1. Exception limited to Opus codec only
2. libopus dependency documented in installation guides
3. Graceful degradation if libopus unavailable (other formats still work)
4. Security monitoring of libopus CVE advisories

---

## Implementation Checklist

**Phase 1: Documentation Updates (30 minutes)**
- [ ] Add [REQ-TECH-022A] to REQ001-requirements.md
- [ ] Update SPEC001-architecture.md line 1301
- [ ] Update this REV003 document status to "Implemented"

**Phase 2: Code Integration (1 hour)**
- [ ] Add `symphonia-adapter-libopus = "0.2.3"` to wkmp-ap/Cargo.toml
- [ ] Verify decoder.rs works with Opus adapter (minimal changes expected)
- [ ] Remove `#[ignore]` from test_decode_opus() in audio_format_tests.rs
- [ ] Update test comments to reflect C library integration

**Phase 3: Testing (1 hour)**
- [ ] Run audio format tests with Opus enabled
- [ ] Verify Opus decoding on Linux (primary platform)
- [ ] Test Windows and macOS if available
- [ ] Confirm graceful failure if libopus missing

**Phase 4: Documentation (30 minutes)**
- [ ] Create installation notes mentioning libopus requirement
- [ ] Update README or deployment guide with platform-specific libopus install commands
- [ ] Document fallback behavior (other formats work without libopus)

**Total Estimated Time:** 3 hours

---

## References

### Requirements
- [REQ-PI-020] Support audio formats: MP3, FLAC, OGG, M4A, AAC, OPUS, WAV
- [REQ-CF-010] Plays passages from local files (.mp3, .opus, .aac, .flac and similar)
- [REQ-TECH-021] Rust
- [REQ-TECH-022] Single-stream audio architecture (symphonia for decoding)

### Architecture
- SPEC001-architecture.md line 1301 (Audio decoding stack)
- SPEC001-architecture.md lines 1317-1318 (Chromaprint C library precedent)

### Implementation
- wkmp-ap/tests/audio_format_tests.rs (test suite revealing Opus limitation)
- wkmp-ap/AUDIO_FORMAT_TEST_PLAN.md (discovered symphonia Opus limitation)

### External Resources
- Symphonia documentation: https://docs.rs/symphonia/latest/symphonia/
- symphonia-adapter-libopus: https://crates.io/crates/symphonia-adapter-libopus
- libopus reference: https://opus-codec.org/
- RFC 6716 (Opus specification): https://www.rfc-editor.org/rfc/rfc6716

---

**Document History:**
- 2025-10-18: Created (REV003)
- 2025-10-18: Approved by technical lead

**Maintained By:** Project Architect, Technical Lead
**Authority:** Change control request - feeds updates to Tier 1-4 documents
**Status:** ✅ Accepted - Implementation in progress

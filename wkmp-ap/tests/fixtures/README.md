# Test Fixtures for wkmp-ap Audio Format Tests

This directory contains audio test files used for verifying decoder functionality.

## Test Audio Files

All test files in the `audio/` subdirectory are 10-second 440 Hz sine wave test tones with the following specifications:

| File | Format | Codec | Sample Rate | Channels | Duration | Size |
|------|--------|-------|-------------|----------|----------|------|
| `test_audio_10s_mp3.mp3` | MP3 | MPEG-1 Layer III | 44.1 kHz | Stereo | 10s | ~236 KB |
| `test_audio_10s_flac.flac` | FLAC | FLAC lossless | 44.1 kHz | Stereo | 10s | ~127 KB |
| `test_audio_10s_vorbis.ogg` | Vorbis | Vorbis in OGG | 44.1 kHz | Stereo | 10s | ~46 KB |
| `test_audio_10s_wav.wav` | WAV | PCM uncompressed | 44.1 kHz | Stereo | 10s | ~1.7 MB |
| `test_audio_10s_aac.m4a` | AAC | AAC in MP4 | 44.1 kHz | Stereo | 10s | ~133 KB |
| `test_audio_10s_opus.opus` | Opus | Opus in OGG | 48 kHz (native) | Stereo | 10s | ~160 KB |

**Note:** Opus files decode at their native 48kHz sample rate. AAC files are present but not tested due to symphonia AAC demuxer limitations (see below).

## File Generation

Test files were generated using FFmpeg with the following commands:

### WAV (source file)
```bash
ffmpeg -f lavfi -i "sine=frequency=440:duration=10:sample_rate=44100" \
       -ac 2 -ar 44100 -af "volume=0.8" -sample_fmt s16 \
       test_audio_10s_wav.wav -y
```

### Derived formats (from WAV)
```bash
# MP3
ffmpeg -i test_audio_10s_wav.wav -c:a libmp3lame -b:a 192k \
       test_audio_10s_mp3.mp3 -y

# FLAC
ffmpeg -i test_audio_10s_wav.wav -c:a flac \
       test_audio_10s_flac.flac -y

# AAC/M4A
ffmpeg -i test_audio_10s_wav.wav -c:a aac -b:a 192k \
       test_audio_10s_aac.m4a -y

# Vorbis/OGG
ffmpeg -i test_audio_10s_wav.wav -c:a libvorbis -q:a 5 \
       test_audio_10s_vorbis.ogg -y

# Opus (not currently used)
ffmpeg -i test_audio_10s_wav.wav -c:a libopus -b:a 128k \
       test_audio_10s_opus.opus -y
```

## Regenerating Test Files

If test files need to be regenerated (e.g., after changing test parameters):

```bash
cd wkmp-ap/tests/fixtures/audio

# Generate all files with the above commands
# Or use the convenience script (if created):
./regenerate_test_files.sh
```

## Test Coverage

These files are used by `wkmp-ap/tests/audio_format_tests.rs` to verify:

- **✅ MP3** - Successfully decodes (libmp3lame encoder)
- **✅ FLAC** - Successfully decodes (lossless)
- **✅ Vorbis** - Successfully decodes (OGG container)
- **✅ WAV** - Successfully decodes (uncompressed PCM)
- **❌ AAC** - Decoder fails with "Channel count not found" (symphonia 0.5 limitation)
- **❌ Opus** - Not supported in pure Rust (requires C library via symphonia-adapter-libopus)

### Current Status

**Working formats (5):**
- MP3 (via `mp3` feature in symphonia)
- FLAC (via `flac` feature in symphonia)
- Vorbis/OGG (via `vorbis` feature in symphonia)
- WAV/PCM (default symphonia support)
- **Opus** ✅ (via symphonia-adapter-libopus + libopus C library) [REQ-TECH-022A]

**Non-working formats (1):**
- **AAC**: Symphonia 0.5's AAC decoder has issues reading channel count from MP4 containers. Test marked as `#[ignore]`.

## Requirements Traceability

These test fixtures support verification of:

- **[REQ-PI-020]**: Support audio formats: MP3, FLAC, OGG, M4A, AAC, OPUS, WAV
  - **Status**: Partial (4 of 6 formats working, AAC and Opus have issues)
- **[REQ-TECH-022]**: Single-stream audio architecture (symphonia for decoding)
  - **Status**: Verified for MP3, FLAC, Vorbis, WAV

## Known Issues and Limitations

### AAC Decoding Issue
**Problem:** Symphonia 0.5's AAC decoder fails with "Channel count not found" error
**Root Cause:** AAC/MP4 demuxer limitation in current symphonia version
**Workaround:** Test ignored; AAC support deferred to future symphonia update
**Tracking:** See test file comments for details

### Opus C Library Integration
**Status:** ✅ **IMPLEMENTED** (2025-10-18)
**Solution:** Added `symphonia-adapter-libopus` dependency with libopus C library FFI
**Requirements:** [REQ-TECH-022A] exception approved per REV003-opus_c_library_exception.md
**System Library:** Requires libopus installed (`apt install libopus0` on Debian/Ubuntu)
**Sample Rate:** Opus decodes at native 48kHz (vs 44.1kHz for other formats)

## File Integrity Verification

To verify test file integrity, check MD5 sums (if regenerated, update these values):

```bash
cd wkmp-ap/tests/fixtures/audio
md5sum test_audio_10s_*.{mp3,flac,ogg,wav,m4a}
```

## Future Enhancements

Potential additions to test fixture coverage:

1. **Variable sample rates** (48 kHz, 96 kHz) to test resampling
2. **Mono sources** to test mono-to-stereo conversion
3. **Multi-channel sources** (5.1, 7.1) to test downmixing
4. **Different bit depths** (24-bit, 32-bit float) for format coverage
5. **Edge cases** (zero-length, truncated, corrupt files)
6. **Very long files** (1+ hours) for memory efficiency testing

Current fixture set covers the essential formats required for basic decoder validation.

## License and Attribution

These test files were generated synthetically using FFmpeg's sine wave generator.
**No copyright restrictions** - pure sine wave test tones are not copyrightable.

---

**Last Updated:** 2025-10-18
**Generated By:** FFmpeg 4.4.2 (Ubuntu 22.04)
**Purpose:** Audio decoder format verification tests

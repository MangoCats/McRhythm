# Test Fixtures for wkmp-ai

This directory contains audio test fixtures for component and integration testing.

## Overview

Test fixtures are small audio files with known metadata used to verify:
- Metadata extraction (ID3, Vorbis comments)
- Codec detection (MP3, FLAC, etc.)
- File hashing and deduplication
- Audio fingerprinting (Chromaprint)

## Required Fixtures

### 1. MP3 with ID3 Tags (`test_tagged.mp3`)

**Specifications:**
- Duration: 5 seconds
- Format: MP3 (CBR 128kbps)
- Sample rate: 44100 Hz
- Channels: Stereo
- Metadata:
  - Title: "Test Track"
  - Artist: "Test Artist"
  - Album: "Test Album"
  - Year: 2024
  - Genre: "Electronic"
  - Track: 1

**Generation Command (requires ffmpeg):**
```bash
ffmpeg -f lavfi -i "sine=frequency=440:duration=5" \
  -metadata title="Test Track" \
  -metadata artist="Test Artist" \
  -metadata album="Test Album" \
  -metadata date="2024" \
  -metadata genre="Electronic" \
  -metadata track="1" \
  -codec:a libmp3lame -b:a 128k \
  tests/fixtures/test_tagged.mp3
```

### 2. FLAC with Vorbis Comments (`test_tagged.flac`)

**Specifications:**
- Duration: 5 seconds
- Format: FLAC (lossless)
- Sample rate: 44100 Hz
- Channels: Stereo
- Metadata (Vorbis comments):
  - TITLE: "Test FLAC Track"
  - ARTIST: "Test FLAC Artist"
  - ALBUM: "Test FLAC Album"
  - DATE: 2024
  - GENRE: "Classical"
  - TRACKNUMBER: 2

**Generation Command (requires ffmpeg):**
```bash
ffmpeg -f lavfi -i "sine=frequency=880:duration=5" \
  -metadata TITLE="Test FLAC Track" \
  -metadata ARTIST="Test FLAC Artist" \
  -metadata ALBUM="Test FLAC Album" \
  -metadata DATE="2024" \
  -metadata GENRE="Classical" \
  -metadata TRACKNUMBER="2" \
  -codec:a flac \
  tests/fixtures/test_tagged.flac
```

### 3. MP3 without Tags (`test_untagged.mp3`)

**Specifications:**
- Duration: 3 seconds
- Format: MP3 (no metadata)
- Sample rate: 44100 Hz
- Channels: Mono

**Generation Command:**
```bash
ffmpeg -f lavfi -i "sine=frequency=220:duration=3" \
  -ac 1 \
  -codec:a libmp3lame -b:a 96k \
  tests/fixtures/test_untagged.mp3
```

## Generating All Fixtures

**Automated Script (Bash):**

Create `tests/fixtures/generate_fixtures.sh`:

```bash
#!/bin/bash
# Generate test audio fixtures for wkmp-ai

set -e  # Exit on error

FIXTURES_DIR="$(dirname "$0")"
cd "$FIXTURES_DIR"

echo "Generating test fixtures..."

# Check ffmpeg availability
if ! command -v ffmpeg &> /dev/null; then
    echo "ERROR: ffmpeg not found. Please install ffmpeg first."
    echo "  Ubuntu/Debian: sudo apt-get install ffmpeg"
    echo "  macOS: brew install ffmpeg"
    echo "  Windows: Download from https://ffmpeg.org/download.html"
    exit 1
fi

# 1. MP3 with ID3 tags
echo "Creating test_tagged.mp3..."
ffmpeg -y -f lavfi -i "sine=frequency=440:duration=5" \
  -metadata title="Test Track" \
  -metadata artist="Test Artist" \
  -metadata album="Test Album" \
  -metadata date="2024" \
  -metadata genre="Electronic" \
  -metadata track="1" \
  -codec:a libmp3lame -b:a 128k \
  test_tagged.mp3

# 2. FLAC with Vorbis comments
echo "Creating test_tagged.flac..."
ffmpeg -y -f lavfi -i "sine=frequency=880:duration=5" \
  -metadata TITLE="Test FLAC Track" \
  -metadata ARTIST="Test FLAC Artist" \
  -metadata ALBUM="Test FLAC Album" \
  -metadata DATE="2024" \
  -metadata GENRE="Classical" \
  -metadata TRACKNUMBER="2" \
  -codec:a flac \
  test_tagged.flac

# 3. Untagged MP3
echo "Creating test_untagged.mp3..."
ffmpeg -y -f lavfi -i "sine=frequency=220:duration=3" \
  -ac 1 \
  -codec:a libmp3lame -b:a 96k \
  test_tagged.flac

echo ""
echo "✓ Fixtures generated successfully!"
echo ""
echo "Generated files:"
ls -lh test_tagged.mp3 test_tagged.flac test_untagged.mp3
echo ""
echo "To verify metadata:"
echo "  ffprobe test_tagged.mp3"
echo "  ffprobe test_tagged.flac"
```

**Windows PowerShell Script:**

Create `tests/fixtures/generate_fixtures.ps1`:

```powershell
# Generate test audio fixtures for wkmp-ai

$ErrorActionPreference = "Stop"

$FixturesDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $FixturesDir

Write-Host "Generating test fixtures..."

# Check ffmpeg availability
if (!(Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Error @"
ERROR: ffmpeg not found. Please install ffmpeg first.
  Download from: https://ffmpeg.org/download.html
  Or use Chocolatey: choco install ffmpeg
"@
    exit 1
}

# 1. MP3 with ID3 tags
Write-Host "Creating test_tagged.mp3..."
ffmpeg -y -f lavfi -i "sine=frequency=440:duration=5" `
  -metadata title="Test Track" `
  -metadata artist="Test Artist" `
  -metadata album="Test Album" `
  -metadata date="2024" `
  -metadata genre="Electronic" `
  -metadata track="1" `
  -codec:a libmp3lame -b:a 128k `
  test_tagged.mp3

# 2. FLAC with Vorbis comments
Write-Host "Creating test_tagged.flac..."
ffmpeg -y -f lavfi -i "sine=frequency=880:duration=5" `
  -metadata TITLE="Test FLAC Track" `
  -metadata ARTIST="Test FLAC Artist" `
  -metadata ALBUM="Test FLAC Album" `
  -metadata DATE="2024" `
  -metadata GENRE="Classical" `
  -metadata TRACKNUMBER="2" `
  -codec:a flac `
  test_tagged.flac

# 3. Untagged MP3
Write-Host "Creating test_untagged.mp3..."
ffmpeg -y -f lavfi -i "sine=frequency=220:duration=3" `
  -ac 1 `
  -codec:a libmp3lame -b:a 96k `
  test_untagged.mp3

Write-Host ""
Write-Host "✓ Fixtures generated successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Generated files:"
Get-ChildItem test_tagged.mp3, test_tagged.flac, test_untagged.mp3 | Format-Table Name, Length
Write-Host ""
Write-Host "To verify metadata:"
Write-Host "  ffprobe test_tagged.mp3"
Write-Host "  ffprobe test_tagged.flac"
```

## Git Considerations

**File Sizes:**
- `test_tagged.mp3`: ~40-50 KB
- `test_tagged.flac`: ~200-300 KB
- `test_untagged.mp3`: ~25-30 KB

Total: <400 KB (acceptable for git)

**`.gitignore` rules:**
```gitignore
# Allow small test fixtures
!test_tagged.mp3
!test_tagged.flac
!test_untagged.mp3

# Ignore large audio files
*.wav
*.aiff
*.m4a
*.ogg
```

## Usage in Tests

### Example: Metadata Extraction Test

```rust
#[test]
fn test_mp3_metadata_extraction() {
    let fixture_path = Path::new("tests/fixtures/test_tagged.mp3");

    // Skip if fixture not generated
    if !fixture_path.exists() {
        eprintln!("SKIP: Fixture not found. Run generate_fixtures.sh first.");
        return;
    }

    let extractor = MetadataExtractor::new();
    let metadata = extractor.extract(fixture_path).expect("Extract failed");

    assert_eq!(metadata.title, Some("Test Track".to_string()));
    assert_eq!(metadata.artist, Some("Test Artist".to_string()));
    assert_eq!(metadata.album, Some("Test Album".to_string()));
}
```

### Example: File Hashing Test

```rust
#[test]
fn test_file_hash_consistency() {
    let fixture = Path::new("tests/fixtures/test_untagged.mp3");

    if !fixture.exists() {
        eprintln!("SKIP: Fixture not found.");
        return;
    }

    let hash1 = calculate_file_hash(fixture).expect("Hash failed");
    let hash2 = calculate_file_hash(fixture).expect("Hash failed");

    assert_eq!(hash1, hash2, "Hash should be deterministic");
}
```

## Troubleshooting

### ffmpeg not found

**Linux:**
```bash
sudo apt-get update
sudo apt-get install ffmpeg
```

**macOS:**
```bash
brew install ffmpeg
```

**Windows:**
1. Download from https://ffmpeg.org/download.html
2. Extract and add to PATH
3. Or use Chocolatey: `choco install ffmpeg`

### Generated files too large

Reduce duration or bitrate:
```bash
# Shorter duration (1 second instead of 5)
ffmpeg -f lavfi -i "sine=frequency=440:duration=1" ...

# Lower bitrate (64kbps instead of 128kbps)
... -codec:a libmp3lame -b:a 64k ...
```

### Metadata not embedded

Use `ffprobe` to verify:
```bash
ffprobe -show_format -show_streams test_tagged.mp3
```

Check for:
- `TAG:title=Test Track`
- `TAG:artist=Test Artist`
- etc.

## CI/CD Integration

For continuous integration, generate fixtures as part of test setup:

**GitHub Actions:**
```yaml
- name: Install ffmpeg
  run: sudo apt-get install -y ffmpeg

- name: Generate test fixtures
  run: |
    cd wkmp-ai/tests/fixtures
    bash generate_fixtures.sh

- name: Run tests
  run: cargo test --package wkmp-ai
```

**GitLab CI:**
```yaml
before_script:
  - apt-get update && apt-get install -y ffmpeg
  - cd wkmp-ai/tests/fixtures && bash generate_fixtures.sh
```

## Future Enhancements

**Additional fixtures to consider:**
- [ ] WAV file (uncompressed PCM)
- [ ] OGG Vorbis (for Vorbis comment testing)
- [ ] M4A/AAC (for MP4 container testing)
- [ ] Files with album art (APIC frames)
- [ ] Multi-track files (for passage segmentation tests)
- [ ] Corrupt/invalid files (for error handling tests)

**Automated validation:**
- [ ] Script to verify fixture metadata matches expectations
- [ ] SHA-256 checksums for fixture verification
- [ ] Automated regeneration on metadata changes

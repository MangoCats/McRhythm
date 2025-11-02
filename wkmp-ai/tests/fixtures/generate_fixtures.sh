#!/bin/bash
# Generate test audio fixtures for wkmp-ai
#
# **[AIA-TEST-010]** Creates minimal audio files with known metadata for testing

set -e  # Exit on error

FIXTURES_DIR="$(dirname "$0")"
cd "$FIXTURES_DIR"

echo "========================================="
echo "wkmp-ai Test Fixture Generator"
echo "========================================="
echo ""

# Check ffmpeg availability
if ! command -v ffmpeg &> /dev/null; then
    echo "ERROR: ffmpeg not found. Please install ffmpeg first."
    echo ""
    echo "Installation instructions:"
    echo "  Ubuntu/Debian: sudo apt-get install ffmpeg"
    echo "  macOS: brew install ffmpeg"
    echo "  Windows: Download from https://ffmpeg.org/download.html"
    echo ""
    exit 1
fi

echo "ffmpeg found: $(ffmpeg -version | head -1)"
echo ""

# 1. MP3 with ID3 tags
echo "[1/3] Generating test_tagged.mp3 (MP3 with ID3 tags)..."
ffmpeg -loglevel warning -y -f lavfi -i "sine=frequency=440:duration=5" \
  -metadata title="Test Track" \
  -metadata artist="Test Artist" \
  -metadata album="Test Album" \
  -metadata date="2024" \
  -metadata genre="Electronic" \
  -metadata track="1" \
  -codec:a libmp3lame -b:a 128k \
  test_tagged.mp3

# 2. FLAC with Vorbis comments
echo "[2/3] Generating test_tagged.flac (FLAC with Vorbis comments)..."
ffmpeg -loglevel warning -y -f lavfi -i "sine=frequency=880:duration=5" \
  -metadata TITLE="Test FLAC Track" \
  -metadata ARTIST="Test FLAC Artist" \
  -metadata ALBUM="Test FLAC Album" \
  -metadata DATE="2024" \
  -metadata GENRE="Classical" \
  -metadata TRACKNUMBER="2" \
  -codec:a flac \
  test_tagged.flac

# 3. Untagged MP3
echo "[3/3] Generating test_untagged.mp3 (MP3 without metadata)..."
ffmpeg -loglevel warning -y -f lavfi -i "sine=frequency=220:duration=3" \
  -ac 1 \
  -codec:a libmp3lame -b:a 96k \
  test_untagged.mp3

echo ""
echo "========================================="
echo "✓ Fixtures generated successfully!"
echo "========================================="
echo ""
echo "Generated files:"
ls -lh test_tagged.mp3 test_tagged.flac test_untagged.mp3 2>/dev/null || ls -l test_tagged.mp3 test_tagged.flac test_untagged.mp3
echo ""
echo "To verify metadata:"
echo "  ffprobe test_tagged.mp3"
echo "  ffprobe test_tagged.flac"
echo ""
echo "Total size: $(du -sh . | cut -f1)"

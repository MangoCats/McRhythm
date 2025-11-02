# Generate test audio fixtures for wkmp-ai
#
# **[AIA-TEST-010]** Creates minimal audio files with known metadata for testing

$ErrorActionPreference = "Stop"

$FixturesDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $FixturesDir

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "wkmp-ai Test Fixture Generator" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

# Check ffmpeg availability
if (!(Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: ffmpeg not found. Please install ffmpeg first." -ForegroundColor Red
    Write-Host ""
    Write-Host "Installation instructions:"
    Write-Host "  Download from: https://ffmpeg.org/download.html"
    Write-Host "  Or use Chocolatey: choco install ffmpeg"
    Write-Host "  Or use Scoop: scoop install ffmpeg"
    Write-Host ""
    exit 1
}

$ffmpegVersion = (ffmpeg -version 2>&1 | Select-Object -First 1)
Write-Host "ffmpeg found: $ffmpegVersion"
Write-Host ""

# 1. MP3 with ID3 tags
Write-Host "[1/3] Generating test_tagged.mp3 (MP3 with ID3 tags)..." -ForegroundColor Yellow
ffmpeg -loglevel warning -y -f lavfi -i "sine=frequency=440:duration=5" `
  -metadata title="Test Track" `
  -metadata artist="Test Artist" `
  -metadata album="Test Album" `
  -metadata date="2024" `
  -metadata genre="Electronic" `
  -metadata track="1" `
  -codec:a libmp3lame -b:a 128k `
  test_tagged.mp3

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Failed to generate test_tagged.mp3" -ForegroundColor Red
    exit 1
}

# 2. FLAC with Vorbis comments
Write-Host "[2/3] Generating test_tagged.flac (FLAC with Vorbis comments)..." -ForegroundColor Yellow
ffmpeg -loglevel warning -y -f lavfi -i "sine=frequency=880:duration=5" `
  -metadata TITLE="Test FLAC Track" `
  -metadata ARTIST="Test FLAC Artist" `
  -metadata ALBUM="Test FLAC Album" `
  -metadata DATE="2024" `
  -metadata GENRE="Classical" `
  -metadata TRACKNUMBER="2" `
  -codec:a flac `
  test_tagged.flac

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Failed to generate test_tagged.flac" -ForegroundColor Red
    exit 1
}

# 3. Untagged MP3
Write-Host "[3/3] Generating test_untagged.mp3 (MP3 without metadata)..." -ForegroundColor Yellow
ffmpeg -loglevel warning -y -f lavfi -i "sine=frequency=220:duration=3" `
  -ac 1 `
  -codec:a libmp3lame -b:a 96k `
  test_untagged.mp3

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Failed to generate test_untagged.mp3" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "✓ Fixtures generated successfully!" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Generated files:"
Get-ChildItem test_tagged.mp3, test_tagged.flac, test_untagged.mp3 | ForEach-Object {
    $sizeKB = [math]::Round($_.Length / 1KB, 0)
    Write-Host "  $($_.Name) - ${sizeKB} KB"
}
Write-Host ""
Write-Host "To verify metadata:"
Write-Host "  ffprobe test_tagged.mp3"
Write-Host "  ffprobe test_tagged.flac"
Write-Host ""
$totalSize = (Get-ChildItem test_tagged.mp3, test_tagged.flac, test_untagged.mp3 | Measure-Object -Property Length -Sum).Sum
$totalKB = [math]::Round($totalSize / 1KB, 0)
Write-Host "Total size: ${totalKB} KB"

# TC-U-STEP-001: Folder Scan Tests

**Requirement:** REQ-STEP-001 (Step 1: Folder scan with magic byte analysis)
**Type:** Unit Tests
**Source:** refactor1126.md lines 85-107

---

## TC-U-STEP-001-01: Audio Files Detected by Magic Bytes

**Scope:** Magic byte detection for audio files

**Given:**
- A test directory containing:
  - file1.mp3 (valid MP3 with ID3 header)
  - file2.flac (valid FLAC with fLaC magic)
  - file3.m4a (valid M4A with ftyp atom)
  - file4.wav (valid WAV with RIFF header)

**When:**
- Folder scan is executed on the test directory

**Then:**
- All 4 files are identified as audio files
- Each file's magic byte pattern is correctly recognized
- File list contains all 4 paths

**Pass Criteria:**
- `scan_result.audio_files.len() == 4`
- All files have `classification == AudioWithProperExtension`

**Estimated Effort:** 30 minutes

---

## TC-U-STEP-001-02: File Classification (7 Types) Correct

**Scope:** Classification into 7 file type categories

**Given:**
- A test directory containing:
  - audio_proper.mp3 (MP3 magic, .mp3 extension) → Type 1
  - audio_wrong_ext.mp3 (MP3 magic, .txt extension) → Type 2
  - image_proper.jpg (JPEG magic, .jpg extension) → Type 3
  - image_wrong_ext.jpg (JPEG magic, .mp3 extension) → Type 4
  - other_proper.pdf (PDF magic, .pdf extension) → Type 5
  - other_audio_ext.txt (text, .mp3 extension) → Type 6
  - other_image_ext.txt (text, .jpg extension) → Type 7

**When:**
- Folder scan is executed on the test directory

**Then:**
- Each file is classified into the correct type (1-7)
- Types 1, 3, 5 are marked as "proper extensions"
- Types 2, 4, 6, 7 are marked as "improper extensions"

**Pass Criteria:**
- Each file's classification matches expected type
- `improper_extension_files.len() == 4`
- `proper_extension_files.len() == 3`

**Estimated Effort:** 45 minutes

---

## TC-U-STEP-001-03: Improper Extension Files Flagged

**Scope:** Improper extension detection and reporting

**Given:**
- A file with MP3 magic bytes but .txt extension
- A file with JPEG magic bytes but .mp3 extension

**When:**
- Folder scan processes these files

**Then:**
- Both files appear in improper extension report
- Report includes: path, filename, detected type, extension, issue description
- Files are NOT passed to Step 2 for audio processing

**Pass Criteria:**
- `improper_files.len() == 2`
- Each entry has all required fields populated
- Neither file appears in `audio_files_for_processing` list

**Estimated Effort:** 30 minutes

---

## Test Data

**Required Test Files:**
- Valid audio files: MP3, FLAC, M4A, WAV, OGG, Opus
- Valid image files: JPEG, PNG, GIF
- Corrupted files: Truncated headers, zero-length
- Mismatched extensions: Various combinations

**Test Data Location:** `wkmp-ai/tests/fixtures/scan/`

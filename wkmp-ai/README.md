# wkmp-ai: Audio Ingest Microservice

Audio file import workflow with file scanning, MusicBrainz identification, and amplitude analysis.

## Features

### Import Workflow

Three-phase import process:
1. **SCANNING**: Discover and classify files in root folder
2. **PROCESSING**: Fingerprint audio files and query MusicBrainz
3. **COMPLETED**: Import finished successfully

### File Classification (PLAN027)

During SCANNING phase, all files are classified into three categories:

- **Audio files**: MP3, FLAC, OGG, M4A, AAC, Opus, WAV
- **Image files**: JPG, JPEG, PNG, GIF, BMP, WebP, TIFF
- **Other files**: All remaining files

After scanning completes, access the File Classification Report at `/file-report` to view:
- Count of files in each category
- Complete file listings with path, size, and last modified date
- Human-readable file sizes (B, KB, MB, GB)
- Category filtering and pagination

### REST API

**Import Control:**
- `POST /import/start` - Start new import session
- `POST /import/cancel/:session_id` - Cancel running import
- `GET /import/status/:session_id` - Get session status
- `GET /import/events` - Server-Sent Events stream for real-time progress

**File Classification:**
- `GET /api/import/file-classification` - Get all categories (first 500 files each)
- `GET /api/import/file-classification?category=audio` - Get audio files only
- `GET /api/import/file-classification?category=image` - Get image files only
- `GET /api/import/file-classification?category=other` - Get other files only

Pagination: `?offset=0&limit=500` (max 1000)

**Amplitude Analysis:**
- `POST /analyze/amplitude` - Analyze audio file amplitude profile

**Settings:**
- `GET /settings` - Settings management UI
- `GET /api/settings` - Get all settings as JSON
- `POST /api/settings` - Update settings

### Web UI

- `/` - Import progress page (real-time SSE updates)
- `/file-report` - File classification report (post-SCANNING)
- `/settings` - Settings management

## Architecture

**[SPEC032]** Import workflow state machine with database persistence
**[IMPL008]** REST API with Axum framework
**[PLAN027]** File classification during SCANNING phase

## Development

**Build:**
```bash
cargo build -p wkmp-ai
```

**Run:**
```bash
cargo run -p wkmp-ai
```

**Test:**
```bash
# All tests
cargo test -p wkmp-ai

# Integration tests only
cargo test -p wkmp-ai --test api_integration_tests

# File classification tests
cargo test -p wkmp-ai test_file_classification
```

## Configuration

Zero-configuration startup with 4-tier root folder resolution:
1. CLI argument: `--root-folder /path`
2. Environment variable: `WKMP_ROOT_FOLDER=/path`
3. TOML config: `~/.config/wkmp/wkmp-ai.toml`
4. Default: `~/Music`

Database: `<root_folder>/wkmp.db` (SQLite)

## Port

HTTP server: `http://localhost:5723`

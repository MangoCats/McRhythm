# PLAN023 Workflow Engine - Usage Guide

## Overview

The PLAN023 workflow engine provides a complete 3-tier hybrid fusion pipeline for audio file metadata extraction, fusion, and validation. It processes audio files, extracts metadata from multiple sources, fuses the results using Bayesian methods, validates quality, and stores to database with complete provenance tracking.

## Quick Start

```rust
use wkmp_ai::workflow::song_processor::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create event channel for progress tracking
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);

    // Configure processor
    let config = SongProcessorConfig {
        acoustid_api_key: "YOUR_API_KEY_HERE".to_string(),
        enable_musicbrainz: true,
        enable_audio_derived: true,
        enable_database_storage: false,
    };

    let processor = SongProcessor::new(config, event_tx);

    // Process audio file
    let file = Path::new("/path/to/audio.mp3");
    let passages = processor.process_file(file).await?;

    println!("Extracted {} passages", passages.len());
    for (i, passage) in passages.iter().enumerate() {
        println!("Passage {}: {:?} - {}",
                 i + 1,
                 passage.fusion.metadata.title,
                 passage.validation.status);
    }

    Ok(())
}
```

## Architecture

### 3-Tier Fusion Pipeline

**Tier 1: Extraction** (Parallel)
- ID3 tags (lofty)
- Chromaprint fingerprinting
- AcoustID lookup
- MusicBrainz metadata
- Audio-derived features (RMS, ZCR, spectral centroid)
- Genre mapping (fallback)

**Tier 2: Fusion** (Bayesian + Weighted)
- Identity resolution: Bayesian posterior update
- Metadata fusion: Confidence-weighted selection
- Flavor synthesis: Characteristic-wise weighted averaging

**Tier 3: Validation**
- Title consistency (Levenshtein ≥ 0.8)
- Duration consistency (≤ 5% difference)
- Quality scoring (0-100%)
- Status assignment (Pass/Warning/Fail)

### Workflow Phases

```
Phase 0: Boundary Detection
  → Silence-based passage segmentation
  → Configurable thresholds (default: 0.01 RMS, 2s silence, 30s min passage)
  → Confidence scoring (0.8 for clear boundaries, 0.5 for whole-file fallback)

For Each Passage (Sequential):
  Phase 1: Extraction
    → Parallel extractor execution
    → Error isolation (individual extractor failures don't kill pipeline)
    → Event emission per extractor

  Phase 2: Fusion
    → Identity: Bayesian update (posterior = 1 - (1 - prior) * (1 - evidence))
    → Metadata: Weighted selection (highest confidence wins)
    → Flavor: Weighted averaging + category normalization

  Phase 3: Validation
    → Consistency checks (title, duration)
    → Quality scoring (passed/total * 100%)
    → Status assignment

  Phase 4: Storage (Optional)
    → Database persistence with full provenance
    → Import session tracking
    → Transactional safety
```

## Configuration

### SongProcessorConfig

```rust
pub struct SongProcessorConfig {
    /// AcoustID API key (empty string disables)
    pub acoustid_api_key: String,

    /// Enable MusicBrainz metadata fetch
    pub enable_musicbrainz: bool,

    /// Enable audio-derived feature extraction
    pub enable_audio_derived: bool,

    /// Enable database storage
    pub enable_database_storage: bool,
}
```

**Example configurations:**

```rust
// Minimal (ID3 only)
let config = SongProcessorConfig {
    acoustid_api_key: String::new(),
    enable_musicbrainz: false,
    enable_audio_derived: false,
    enable_database_storage: false,
};

// Full (all extractors + database)
let config = SongProcessorConfig {
    acoustid_api_key: env::var("ACOUSTID_API_KEY").unwrap(),
    enable_musicbrainz: true,
    enable_audio_derived: true,
    enable_database_storage: true,
};
```

## Database Storage

### With Database Persistence

```rust
use sqlx::SqlitePool;

let pool = SqlitePool::connect("sqlite://wkmp.db").await?;

let processor = SongProcessor::with_database(config, event_tx, pool);

let passages = processor.process_file(&file).await?;
// Passages are automatically written to database with provenance tracking
```

### Provenance Tracking

The workflow engine writes complete provenance data:

**`passages` table:**
- All metadata fields (title, artist, album, MBID)
- Flavor characteristics + source blend
- Confidence scores per field
- Validation results + quality score
- Import session ID + timestamp

**`import_provenance` table:**
- One row per extraction source per passage
- Source type, confidence, data summary
- Full audit trail

## SSE Event Broadcasting

### Event Bridge Integration

```rust
use wkmp_ai::workflow::event_bridge;
use wkmp_common::events::EventBus;

// Create infrastructure
let (workflow_tx, workflow_rx) = tokio::sync::mpsc::channel(100);
let event_bus = EventBus::new(1000);
let session_id = uuid::Uuid::new_v4();

// Spawn event bridge
tokio::spawn(event_bridge::bridge_workflow_events(
    workflow_rx,
    event_bus.tx.clone(),
    session_id,
));

// Create processor
let processor = SongProcessor::new(config, workflow_tx);

// Subscribe to events
let mut event_rx = event_bus.subscribe();
tokio::spawn(async move {
    while let Ok(event) = event_rx.recv().await {
        println!("Event: {:?}", event);
    }
});

// Process files
let passages = processor.process_file(&file).await?;
```

### Event Types

All `WorkflowEvent` types are converted to `WkmpEvent::ImportProgressUpdate`:

- `FileStarted` → state="PROCESSING"
- `BoundaryDetected` → state="SEGMENTING"
- `PassageStarted` → state="EXTRACTING"
- `ExtractionProgress` → state="EXTRACTING" (per extractor)
- `FusionStarted` → state="FUSING"
- `ValidationStarted` → state="VALIDATING"
- `PassageCompleted` → state="PROCESSING" (with quality score)
- `FileCompleted` → state="COMPLETED"
- `Error` → state="ERROR"

## Performance

**Target:** ≤2 minutes per song

**Typical Performance:**
- 45s audio file: <10s processing (without network calls)
- 3-minute song with AcoustID: <30s
- 10-file batch: <2min total

**Optimizations:**
- Parallel extraction within passages
- Sequential passage processing (fault isolation)
- Efficient audio decoding (symphonia)
- Database connection pooling

## Error Handling

**Fault Isolation:**
- Individual extractor failures → logged as warnings, pipeline continues
- Passage-level failures → isolated, other passages continue
- File-level failures → return error, processor remains usable

**Error Recovery:**

```rust
let processor = SongProcessor::new(config, event_tx);

// First file fails
match processor.process_file(Path::new("/bad/file.mp3")).await {
    Err(e) => eprintln!("Failed: {}", e),
    Ok(_) => println!("Success"),
}

// Processor can be reused
let passages = processor.process_file(Path::new("/good/file.mp3")).await?;
println!("Recovered: {} passages", passages.len());
```

## Testing

**Unit Tests:** 91 (library)
**Integration Tests:** 5 (workflow)
**System Tests:** 6 (end-to-end)
**Total:** 177 tests (233% of target)

Run tests:
```bash
# All tests
cargo test -p wkmp-ai

# Unit tests only
cargo test -p wkmp-ai --lib

# Integration tests
cargo test -p wkmp-ai --test workflow_integration

# System tests
cargo test -p wkmp-ai --test system_tests

# Database test (requires schema)
cargo test -p wkmp-ai --test system_tests test_system_database_persistence -- --ignored
```

## API Reference

### SongProcessor

```rust
impl SongProcessor {
    /// Create processor without database
    pub fn new(
        config: SongProcessorConfig,
        event_tx: mpsc::Sender<WorkflowEvent>
    ) -> Self;

    /// Create processor with database
    pub fn with_database(
        config: SongProcessorConfig,
        event_tx: mpsc::Sender<WorkflowEvent>,
        db: SqlitePool
    ) -> Self;

    /// Process audio file (all passages)
    pub async fn process_file(
        &self,
        file_path: &Path
    ) -> Result<Vec<ProcessedPassage>>;
}
```

### ProcessedPassage

```rust
pub struct ProcessedPassage {
    pub boundary: PassageBoundary,       // Start/end times + confidence
    pub extractions: Vec<ExtractionResult>, // Raw extractor outputs
    pub fusion: FusionResult,            // Fused metadata + flavor
    pub validation: ValidationResult,     // Quality checks
}
```

### FusionResult

```rust
pub struct FusionResult {
    pub identity: FusedIdentity,     // MBID + confidence + conflicts
    pub metadata: FusedMetadata,     // Title, artist, album + sources
    pub flavor: FusedFlavor,         // Characteristics + blend + completeness
}
```

## Troubleshooting

**No passages detected:**
- Check min_passage_duration (default: 30s)
- Verify audio file is valid
- Check silence_threshold (default: 0.01 RMS)

**Low quality scores:**
- Enable more extractors (acoustid_api_key, enable_musicbrainz)
- Check audio quality (low-bitrate files → less reliable fingerprints)
- Review validation_report in ProcessedPassage

**Database errors:**
- Ensure schema exists (run migrations)
- Check connection pool configuration
- Verify write permissions

**Memory issues with large files:**
- Boundary detector loads entire file into memory
- Consider processing in batches for very large libraries
- Use database storage to persist results incrementally

## License

Part of WKMP (Auto DJ Music Player) - See project LICENSE file.

## Support

For issues, questions, or feature requests, see the main WKMP repository.

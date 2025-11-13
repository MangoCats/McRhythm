# Per-Song Import Workflow Architecture

**Section:** 01 - Workflow Architecture
**Parent Document:** [00_SUMMARY.md](00_SUMMARY.md)
**Next Section:** [02_realtime_ui_design.md](02_realtime_ui_design.md)

---

## 1.1 Current State (File-Level Processing)

**Current Implementation (wkmp-ai):**
```
┌─────────────────────────────────────────────────────────┐
│ AUDIO FILE: album.flac (60 minutes, 10 songs)         │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ Phase 1: SCANNING                                      │
│   → Discover file: album.flac                         │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ Phase 2: EXTRACTING                                    │
│   → Read ID3 tags: Album title, artist, genre         │
│   → NO PER-SONG METADATA (file-level only)            │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ Phase 3: FINGERPRINTING                                │
│   → Chromaprint of first 120 seconds                  │
│   → AcoustID lookup → SINGLE Recording MBID           │
│   → LIMITATION: Only identifies first song            │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ Phase 4: SEGMENTING                                    │
│   → Silence detection → Find 10 passage boundaries    │
│   → Result: 10 passages, BUT no per-passage identity  │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ Phase 5-7: ANALYZING + FLAVORING + COMPLETE           │
│   → Amplitude analysis (stub)                         │
│   → Musical flavor (file-level, not per-passage)      │
│   → Write 10 passages to database                     │
│   → ALL PASSAGES SHARE SAME MBID (incorrect!)         │
└─────────────────────────────────────────────────────────┘
```

**Problem:**
- File-level identification → First song MBID applied to ALL passages
- No per-song fingerprinting → Songs 2-10 not identified
- User sees: "Processing album.flac... 45% complete"
- User doesn't know: Which songs succeeded, which failed, which have conflicts

---

## 1.2 Proposed: Per-Song Sequential Processing

**New Workflow:**

```
┌─────────────────────────────────────────────────────────┐
│ AUDIO FILE: album.flac (60 minutes, 10 songs)         │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ PHASE 0: PASSAGE BOUNDARY DETECTION (NEW)             │
│   → Silence detection → 10 passages found             │
│   → Extract timing: P1[0:00-3:45], P2[3:45-7:20]...   │
│   → Create "passage queue" for sequential processing   │
│   → SSE Event: PassagesDiscovered(count=10)           │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
        ┌─────────────┴─────────────┐
        │ For each passage (1 to 10): │
        └─────────────┬─────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ PASSAGE 1: "Breathe (In The Air)" [0:00-3:45]         │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ Step 1: EXTRACT AUDIO SEGMENT                         │
│   → Decode passage from album.flac [0:00-3:45]        │
│   → Generate passage-specific fingerprint             │
│   → SSE Event: SongExtracting(passage_id=1)           │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ Step 2: IDENTITY RESOLUTION (Hybrid Fusion Tier 1+2)  │
│   → ID3 tags (if passage-specific tags exist)         │
│   → Chromaprint fingerprint (passage audio)           │
│   → AcoustID lookup                                   │
│   → MusicBrainz query                                 │
│   → **Bayesian fusion** of ID3 + AcoustID MBIDs      │
│   → SSE Event: IdentityResolved(                      │
│       passage_id=1,                                   │
│       mbid="abc-123",                                 │
│       confidence=0.92,                                │
│       sources=["AcoustID", "ID3"]                     │
│     )                                                 │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ Step 3: METADATA FUSION (Hybrid Fusion Tier 2)        │
│   → Fetch MusicBrainz metadata (title, artist, work)  │
│   → Compare with ID3 tags                             │
│   → Weighted selection (prefer higher quality)        │
│   → SSE Event: MetadataFused(                         │
│       passage_id=1,                                   │
│       title="Breathe (In The Air)",                   │
│       source="MusicBrainz",                           │
│       conflicts=[]                                    │
│     )                                                 │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ Step 4: MUSICAL FLAVOR SYNTHESIS (Hybrid Fusion T2)   │
│   → Query AcousticBrainz (if pre-2022 MBID)          │
│   → Compute Essentia features (local analysis)        │
│   → Map ID3 genre → characteristics                   │
│   → Characteristic-wise weighted fusion               │
│   → SSE Event: FlavorSynthesized(                     │
│       passage_id=1,                                   │
│       completeness=0.85,                              │
│       sources=["AcousticBrainz": 0.6,                 │
│                 "Essentia": 0.3,                      │
│                 "ID3": 0.1]                           │
│     )                                                 │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ Step 5: VALIDATION (Hybrid Fusion Tier 3)             │
│   → Title consistency check (ID3 vs MusicBrainz)      │
│   → Duration validation                               │
│   → Genre-flavor alignment check                      │
│   → Compute overall quality score                     │
│   → SSE Event: ValidationComplete(                    │
│       passage_id=1,                                   │
│       status="Pass",                                  │
│       quality_score=92,                               │
│       warnings=[]                                     │
│     )                                                 │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ Step 6: PASSAGE CREATION                              │
│   → Create Passage record in database                 │
│   → Link to Song (MBID)                               │
│   → Store musical flavor JSON                         │
│   → Store source provenance metadata                  │
│   → SSE Event: SongCompleted(                         │
│       passage_id=1,                                   │
│       title="Breathe (In The Air)",                   │
│       status="Success"                                │
│     )                                                 │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
        ┌─────────────┴─────────────┐
        │ Repeat for passages 2-10   │
        └─────────────┬─────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│ ALL PASSAGES COMPLETE                                  │
│   → SSE Event: FileImportComplete(                    │
│       file="album.flac",                              │
│       passages_created=10,                            │
│       successes=8,                                    │
│       warnings=2,                                     │
│       failures=0                                      │
│     )                                                 │
└─────────────────────────────────────────────────────────┘
```

---

## 1.3 Key Architectural Changes

### Change 1: Passage Detection First

**Current:** Segmenting happens after fingerprinting (Phase 4 after Phase 3)

**Proposed:** Passage detection is Phase 0 (before any per-song processing)

**Rationale:**
- Need passage boundaries to extract individual songs for fingerprinting
- Allows UI to show "10 songs discovered" immediately
- Enables sequential processing of known song count

**Implementation:**
```rust
// Phase 0: Detect passages
let passages = detect_passage_boundaries(&audio_file)?;
event_bus.broadcast(ImportEvent::PassagesDiscovered {
    file_path: audio_file.path.clone(),
    count: passages.len(),
});

// Phase 1-6: Process each passage
for (index, passage) in passages.iter().enumerate() {
    let song_result = process_song(passage, &event_bus).await?;
    // ... handle result
}
```

---

### Change 2: Per-Song Hybrid Fusion Pipeline

**Current:** Single fingerprint for entire file, single MBID applied to all passages

**Proposed:** Each passage flows through complete hybrid fusion pipeline

**Fusion Stages Per Song:**
```rust
async fn process_song(passage: &Passage, event_bus: &EventBus) -> Result<SongImportResult> {
    // Stage 1: Extract passage audio
    let audio_segment = extract_audio_segment(&passage.file_path, passage.start_ms, passage.end_ms)?;
    event_bus.broadcast(ImportEvent::SongExtracting { passage_id: passage.id });

    // Stage 2: Identity Resolution (Tier 1 + Tier 2 of Hybrid Fusion)
    let id3_mbid = extract_id3_mbid(&audio_segment)?;  // If passage has embedded tags
    let fingerprint = generate_fingerprint(&audio_segment)?;
    let acoustid_result = acoustid_lookup(&fingerprint).await?;
    let identity = resolve_identity(id3_mbid, acoustid_result)?;  // Bayesian fusion
    event_bus.broadcast(ImportEvent::IdentityResolved {
        passage_id: passage.id,
        mbid: identity.mbid,
        confidence: identity.confidence,
        sources: identity.sources,
    });

    // Stage 3: Metadata Fusion (Tier 2)
    let mb_metadata = musicbrainz_lookup(&identity.mbid).await?;
    let fused_metadata = fuse_metadata(id3_metadata, mb_metadata, identity.confidence)?;
    event_bus.broadcast(ImportEvent::MetadataFused {
        passage_id: passage.id,
        title: fused_metadata.title.clone(),
        source: fused_metadata.source,
        conflicts: fused_metadata.conflicts,
    });

    // Stage 4: Musical Flavor Synthesis (Tier 2)
    let flavor = synthesize_musical_flavor(&identity.mbid, &audio_segment, &id3_metadata).await?;
    event_bus.broadcast(ImportEvent::FlavorSynthesized {
        passage_id: passage.id,
        completeness: flavor.completeness,
        sources: flavor.source_blend,
    });

    // Stage 5: Validation (Tier 3)
    let validation = validate_passage(&fused_metadata, &flavor)?;
    event_bus.broadcast(ImportEvent::ValidationComplete {
        passage_id: passage.id,
        status: validation.status,
        quality_score: validation.quality_score,
        warnings: validation.warnings,
    });

    // Stage 6: Create passage record
    let passage_record = create_passage_record(passage, identity, fused_metadata, flavor, validation)?;
    db::insert_passage(&passage_record).await?;
    event_bus.broadcast(ImportEvent::SongCompleted {
        passage_id: passage.id,
        title: fused_metadata.title,
        status: "Success",
    });

    Ok(SongImportResult::Success(passage_record))
}
```

---

### Change 3: Granular Event Emission

**Current:** File-level events only (ImportProgressUpdate with percentage)

**Proposed:** Per-song, per-stage events

**Event Types:**
```rust
enum ImportEvent {
    // File-level events
    FileImportStarted { file_path: String },
    PassagesDiscovered { file_path: String, count: usize },
    FileImportComplete { file_path: String, summary: ImportSummary },

    // Song-level events (emitted for each passage)
    SongExtracting { passage_id: Uuid },
    IdentityResolved {
        passage_id: Uuid,
        mbid: Option<Uuid>,
        confidence: f64,
        sources: Vec<String>,
    },
    MetadataFused {
        passage_id: Uuid,
        title: String,
        source: String,
        conflicts: Vec<ConflictReport>,
    },
    FlavorSynthesized {
        passage_id: Uuid,
        completeness: f64,
        sources: HashMap<String, f64>,  // "AcousticBrainz" → 0.6
    },
    ValidationComplete {
        passage_id: Uuid,
        status: ValidationStatus,  // Pass, Warning, Fail
        quality_score: f64,
        warnings: Vec<String>,
    },
    SongCompleted {
        passage_id: Uuid,
        title: String,
        status: String,  // "Success", "Warning", "Failed"
    },

    // Error events
    SongFailed {
        passage_id: Uuid,
        error: String,
    },
}
```

**SSE Broadcast:**
```rust
fn broadcast_event(&self, event: ImportEvent) {
    let json = serde_json::to_string(&event).unwrap();
    self.event_bus.send(format!("event: {}\ndata: {}\n\n",
        event.event_type(),
        json
    ));
}
```

---

## 1.4 Performance Considerations

### Sequential vs Parallel Processing

**Sequential (Recommended for Phase 1):**
```rust
for passage in passages {
    let result = process_song(&passage).await?;
    // ... handle result
}
```

**Pros:**
- Simple control flow
- Easy to debug (linear logs)
- Predictable resource usage (1 song at a time)
- Natural ordering for UI display

**Cons:**
- Slower than parallel (10 songs × 2 min/song = 20 minutes)

**Performance:**
- 10-song album: ~20 minutes (2 min/song average)
- Acceptable for initial implementation

---

**Parallel (Future Optimization):**
```rust
use tokio::sync::Semaphore;

let semaphore = Arc::new(Semaphore::new(4));  // 4 concurrent workers
let mut tasks = vec![];

for passage in passages {
    let permit = semaphore.clone().acquire_owned().await?;
    let task = tokio::spawn(async move {
        let result = process_song(&passage).await;
        drop(permit);
        result
    });
    tasks.push(task);
}

let results = futures::future::join_all(tasks).await;
```

**Pros:**
- Faster (4x speedup with 4 workers)
- Better resource utilization (multi-core CPU)

**Cons:**
- Complex coordination (songs finish out of order)
- Resource contention (4 concurrent audio decodes)
- UI must handle out-of-order completion
- Error handling more complex

**Performance:**
- 10-song album: ~5-7 minutes (2 min/song ÷ 4 workers, with overhead)

**Recommendation:** Start with sequential, optimize to parallel in Phase 2 if needed

---

## 1.5 Error Handling Strategy

**Per-Song Error Isolation:**

**Current (File-Level):**
- If any phase fails → Entire file import fails
- User loses work (all passages discarded)

**Proposed (Song-Level):**
- If Song 3 fails → Songs 1-2 succeed, Song 3 marked failed, Songs 4-10 continue
- User can review failed songs and retry individually

**Implementation:**
```rust
for (index, passage) in passages.iter().enumerate() {
    match process_song(passage, &event_bus).await {
        Ok(result) => {
            successes.push(result);
            event_bus.broadcast(ImportEvent::SongCompleted {
                passage_id: passage.id,
                title: result.title,
                status: "Success",
            });
        },
        Err(error) => {
            failures.push((passage.id, error.to_string()));
            event_bus.broadcast(ImportEvent::SongFailed {
                passage_id: passage.id,
                error: error.to_string(),
            });
            // CONTINUE to next song (don't abort entire import)
        }
    }
}

// File import completes even if some songs failed
event_bus.broadcast(ImportEvent::FileImportComplete {
    file_path: file.path,
    summary: ImportSummary {
        total: passages.len(),
        successes: successes.len(),
        warnings: warnings.len(),
        failures: failures.len(),
    },
});
```

**User Experience:**
```
Import Complete: album.flac
✓ 8 songs imported successfully
⚠️ 2 songs with warnings (low confidence)
✗ 0 songs failed

[View Details] [Retry Failed Songs]
```

---

## 1.6 Integration with Existing Import Workflow

**Backward Compatibility:**

Current wkmp-ai has 7-phase workflow:
1. Scanning
2. Extracting
3. Fingerprinting
4. Segmenting
5. Analyzing
6. Flavoring
7. Completed

**Proposed Mapping:**

**Phase 0 (NEW): Passage Detection**
- Replaces Phase 4 (Segmenting)
- Moved to beginning of workflow

**Phases 1-6 (REFACTORED): Per-Song Processing**
- Old Phase 2 (Extracting) → Per-song identity resolution
- Old Phase 3 (Fingerprinting) → Per-song fingerprinting
- Old Phase 5 (Analyzing) → Per-song amplitude analysis
- Old Phase 6 (Flavoring) → Per-song flavor synthesis
- OLD Phase 7 (Completed) → Per-song passage creation

**Phase 7 (ENHANCED): File Completion**
- Aggregate results from all songs
- Emit file-level summary event

**Migration Path:**
1. Keep existing file-level import as "Legacy Mode"
2. Add per-song import as "Granular Mode"
3. User selects mode at import start
4. Eventually deprecate Legacy Mode

---

[⬅️ Back to Summary](00_SUMMARY.md) | [Next: Real-Time UI Design ➡️](02_realtime_ui_design.md)

# Next Steps: Complete Workflow Progress Implementation

## 🎉 IMPLEMENTATION COMPLETED (2025-11-11)

All planned features have been successfully implemented and tested:

✅ **COMPLETED:**
- ImportState enum with 7 granular states
- Scanning → Extracting transition implemented
- File discovery progress (every 100 files)
- Phase tracking infrastructure (`PhaseProgress`, `SubTaskStatus`)
- **State transitions in processing phase** (via WorkflowEvent listening)
- **Passage-level progress tracking** (total detected, processed, confidence breakdown)
- **Confidence breakdown display** (high/medium/low/unidentified via SubTaskStatus)
- All 244 unit tests passing

⚠️ **MANUAL TESTING REQUIRED:**
- User must perform manual import test to verify UI displays all 7 phases correctly
- Verify passage counts and confidence breakdown appear in progress messages

---

## Implementation Approach

### Strategy: Listen to Existing Pipeline Events

The pipeline already emits detailed `WorkflowEvent` through the event channel. We need to listen to these events in `phase_processing_plan024()` and:

1. Trigger state transitions
2. Track passage counts
3. Track confidence levels
4. Update progress displays

**Key principle:** Do NOT modify pipeline logic - only add listeners.

---

## Detailed Implementation Plan

### Part 1: Add State Transition Logic

**Location:** `phase_processing_plan024()` in [mod.rs:422-465](wkmp-ai/src/services/workflow_orchestrator/mod.rs#L422-L465)

**Current Code:**
```rust
tokio::spawn(async move {
    use crate::workflow::WorkflowEvent;

    while let Some(event) = event_rx.recv().await {
        // Currently just logs events
        match event {
            WorkflowEvent::FileStarted { .. } => { /* log */ }
            WorkflowEvent::PassageCompleted { .. } => { /* log */ }
            // ...
        }
    }
});
```

**Need to Add:**
```rust
// Track which phases have started (to avoid duplicate transitions)
let mut segmenting_started = false;
let mut fingerprinting_started = false;
let mut identifying_started = false;
let mut analyzing_started = false;
let mut flavoring_started = false;

// Passage counters
let mut total_passages_detected = 0;
let mut passages_processed = 0;

// Confidence breakdown
let mut high_confidence = 0;    // quality_score > 0.8
let mut medium_confidence = 0;  // 0.5 < quality_score ≤ 0.8
let mut low_confidence = 0;     // 0.2 < quality_score ≤ 0.5
let mut unidentified = 0;       // quality_score ≤ 0.2

while let Some(event) = event_rx.recv().await {
    match event {
        // Transition to SEGMENTING on first boundary detection
        WorkflowEvent::BoundaryDetected { .. } => {
            total_passages_detected += 1;

            if !segmenting_started {
                segmenting_started = true;
                // Trigger state transition (need to communicate back to main task)
            }
        }

        // Transition to FINGERPRINTING when chromaprint extraction starts
        WorkflowEvent::ExtractionProgress { extractor, .. } => {
            if extractor == "chromaprint" && !fingerprinting_started {
                fingerprinting_started = true;
                // Trigger state transition
            }
            if extractor == "acoustid" && !identifying_started {
                identifying_started = true;
                // Trigger state transition to IDENTIFYING
            }
        }

        // Track passage completion and confidence
        WorkflowEvent::PassageCompleted { quality_score, .. } => {
            passages_processed += 1;

            if quality_score > 0.8 {
                high_confidence += 1;
            } else if quality_score > 0.5 {
                medium_confidence += 1;
            } else if quality_score > 0.2 {
                low_confidence += 1;
            } else {
                unidentified += 1;
            }
        }

        _ => {}
    }
}
```

### Part 2: Bridge Event Task → Main Task Communication

**Problem:** The event listener runs in a spawned task, but state transitions need to happen in the main task (which holds `&mut session`).

**Solution:** Use a channel to send state transition commands from event task to main task.

```rust
// Create command channel
let (state_tx, mut state_rx) = mpsc::channel(10);

// Spawn event listener
let state_tx_clone = state_tx.clone();
tokio::spawn(async move {
    // ... event processing logic ...

    if !segmenting_started {
        state_tx_clone.send(StateCommand::TransitionTo(ImportState::Segmenting)).await.ok();
        segmenting_started = true;
    }
});

// In main task, process state commands
while let Some(command) = state_rx.try_recv() {
    match command {
        StateCommand::TransitionTo(new_state) => {
            session.transition_to(new_state);
            crate::db::sessions::save_session(&self.db, &session).await?;
            self.broadcast_progress(&session, start_time);
        }
    }
}
```

### Part 3: Update Progress Display with Passage Counts

**In main file processing loop:**

```rust
session.update_progress(
    files_processed,
    total_files,
    format!(
        "Processing file {} of {} | {} passages detected, {} processed ({} high, {} medium, {} low, {} unidentified)",
        files_processed + 1,
        total_files,
        total_passages_detected,
        passages_processed,
        high_confidence,
        medium_confidence,
        low_confidence,
        unidentified
    ),
);
```

### Part 4: Use SubTaskStatus for Confidence Breakdown

**Update phase progress with confidence subtasks:**

```rust
if let Some(identifying_phase) = session.progress.get_phase_mut(ImportState::Identifying) {
    identifying_phase.subtasks = vec![
        SubTaskStatus { name: "High Confidence".into(), success_count: high_confidence, failure_count: 0, skip_count: 0 },
        SubTaskStatus { name: "Medium Confidence".into(), success_count: medium_confidence, failure_count: 0, skip_count: 0 },
        SubTaskStatus { name: "Low Confidence".into(), success_count: low_confidence, failure_count: 0, skip_count: 0 },
        SubTaskStatus { name: "Unidentified".into(), success_count: unidentified, failure_count: 0, skip_count: 0 },
    ];
}
```

---

## Implementation Order

1. ✅ **Add StateCommand enum** (for event → main task communication) - COMPLETED
2. ✅ **Add command channel** (`mpsc::channel`) - COMPLETED
3. ✅ **Update event listener** to track passages and send state commands - COMPLETED
4. ✅ **Add command processing loop** in main task - COMPLETED
5. ✅ **Update progress messages** with passage counts and confidence breakdown - COMPLETED
6. ⚠️ **Test with manual import** - Requires user to perform manual testing

---

## Expected Console Log Output

```
Phase 1A: SCANNING (file discovery)
... [files discovered] ...
Phase 1B: EXTRACTING (hash + metadata)
... [files processed] ...
Phase 2A: SEGMENTING (boundary detection)
Phase 2B: FINGERPRINTING (chromaprint)
Phase 2C: IDENTIFYING (MusicBrainz)
Processing file 1 of 10 | 15 passages detected, 12 processed (8 high, 3 medium, 1 low, 0 unidentified)
Phase 2D: ANALYZING (amplitude)
Phase 2E: FLAVORING (Essentia)
Import completed successfully
```

---

## Key Files to Modify

1. **mod.rs** - Add StateCommand enum, command channel, event listener logic
2. **import_session.rs** - Already updated ✅
3. **phase_scanning.rs** - Already updated ✅

---

## Testing Checklist

- [ ] Build succeeds
- [ ] All 244 tests pass
- [ ] Manual import shows all 7 phases in console log
- [ ] Progress messages show passage counts
- [ ] Confidence breakdown appears in UI
- [ ] State transitions logged with file:line numbers

---

## Estimated Implementation Time

- Part 1 (State transitions): ~30 minutes
- Part 2 (Command channel): ~20 minutes
- Part 3 (Progress display): ~15 minutes
- Part 4 (Confidence breakdown): ~10 minutes
- Testing: ~15 minutes

**Total:** ~90 minutes of focused work

---

## Notes

- Pipeline logic remains unchanged ✅
- Only adds visibility via event listening ✅
- Uses existing `WorkflowEvent` emissions ✅
- No breaking changes to tests ✅

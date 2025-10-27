# Requirements Index - Fix Remove Currently Playing Passage Bug

**Source:** BUG001_remove_currently_playing.md
**Date:** 2025-10-27
**Scope:** Bug fix for playback state corruption when removing currently playing passage

---

## Requirements Table

| Req ID | Type | Brief Description | Source Line | Priority |
|--------|------|-------------------|-------------|----------|
| REQ-FIX-010 | Functional | Stop playback immediately when removing currently playing passage | 103 | Critical |
| REQ-FIX-020 | Functional | Release decoder chain resources when removing currently playing passage | 104 | Critical |
| REQ-FIX-030 | Functional | Clear mixer state when removing currently playing passage | 105 | Critical |
| REQ-FIX-040 | Functional | Update queue structure correctly (already working) | 106 | High |
| REQ-FIX-050 | Functional | Start next passage if queue non-empty after removal | 107 | Critical |
| REQ-FIX-060 | Functional | Remove from queue without disrupting playback when removing non-current passage | 193-200 | High |
| REQ-FIX-070 | Functional | Prevent removed passage from resuming playback | 25, 189 | Critical |
| REQ-FIX-080 | Functional | Ensure enqueued passage starts after removed passage cleared | 24, 188 | Critical |

---

## Derived from Existing Requirements

| Original Req | Status | Notes |
|--------------|--------|-------|
| REQ-QUE-070 | Broken | Queue management (remove operation) - currently doesn't handle playing passage |
| REQ-PB-010 | Broken | Playback control (stop) - mixer not notified on removal |
| REQ-PB-040 | Broken | Queue advancement (skip) - state inconsistency with decoder/mixer |

---

## Requirements by Subsystem

### Queue Management (`queue_manager.rs`)
- REQ-FIX-040: Update queue structure (working)
- REQ-FIX-050: Determine next passage after removal

### Playback Engine (`engine.rs`)
- REQ-FIX-010: Coordinate stop operation
- REQ-FIX-020: Clear decoder chain
- REQ-FIX-030: Clear mixer state
- REQ-FIX-050: Trigger next passage playback
- REQ-FIX-070: Prevent resume of removed passage

### Decoder Worker (`decoder_worker.rs`)
- REQ-FIX-020: Release decoder resources
- REQ-FIX-070: Discard stale buffer data

### Mixer (`pipeline/mixer.rs`)
- REQ-FIX-010: Stop audio output
- REQ-FIX-030: Clear playback state
- REQ-FIX-070: Don't resume from stale chain

---

## Total Requirements: 8 functional requirements
**Priority Breakdown:**
- Critical: 6 requirements
- High: 2 requirements

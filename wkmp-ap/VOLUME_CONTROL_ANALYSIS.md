# Volume Control Architecture Analysis

**Date:** 2025-10-18
**Issue:** Volume control UI updates status correctly but has no audible effect

---

## Current Implementation Analysis

### 1. Intended Volume Control Flow

```
User adjusts volume slider
    ↓
POST /audio/volume (handlers.rs:250)
    ↓
Update SharedState.volume (handlers.rs:262)
    ↓
Persist to database (handlers.rs:265)
    ↓
Broadcast VolumeChanged event (handlers.rs:271)
    ↓
??? → Audio Output ???
```

### 2. Actual Implementation

#### **API Handler** (`api/handlers.rs:250-276`)
```rust
pub async fn set_volume(
    State(ctx): State<AppContext>,
    Json(req): Json<VolumeRequest>,
) -> Result<Json<VolumeResponse>, StatusCode> {
    // ✓ Update SharedState
    ctx.state.set_volume(req.volume).await;

    // ✓ Persist to database
    crate::db::settings::set_volume(&ctx.db_pool, req.volume).await;

    // ✓ Broadcast event
    ctx.state.broadcast_event(WkmpEvent::VolumeChanged { ... });

    // ✗ MISSING: No call to update AudioOutput volume!
}
```

#### **AudioOutput** (`audio/output.rs`)
- Has `volume: Arc<Mutex<f32>>` field (line 29)
- Initialized to `1.0` (line 131)
- Audio callback **DOES** apply volume (lines 240, 246):
  ```rust
  let current_volume = *volume.lock().unwrap();
  let left = audio_frame.left * current_volume;
  let right = audio_frame.right * current_volume;
  ```
- Has `set_volume()` method (line 382):
  ```rust
  pub fn set_volume(&self, volume: f32) {
      *self.volume.lock().unwrap() = volume.clamp(0.0, 1.0);
  }
  ```

#### **PlaybackEngine** (`playback/engine.rs:320-367`)
- AudioOutput created in separate thread (line 322)
- Local variable `let mut audio_output` - not stored
- NO field in PlaybackEngine struct to hold reference
- **Inaccessible after creation**

```rust
std::thread::spawn(move || {
    let mut audio_output = AudioOutput::new(None)?;  // ← Local variable
    audio_output.start(audio_callback)?;

    // Loop keeps it alive, but no external access
    loop { ... }
});
// audio_output dropped when thread exits
```

---

## Root Cause

**The volume control is completely disconnected from audio output.**

```
┌─────────────┐                    ┌──────────────┐
│ POST        │                    │ AudioOutput  │
│ /audio/     │  Updates           │              │
│ volume      │────────────────┐   │ volume: 1.0  │
└─────────────┘                │   │ (NEVER       │
                               │   │  UPDATED)    │
                               ↓   └──────────────┘
                         ┌──────────┐       ↓
                         │ Shared   │   Audio callback
                         │ State    │   applies 1.0
                         │ volume   │   (unchanged)
                         └──────────┘
```

**The API updates SharedState, but AudioOutput reads from its own independent volume field which is never synchronized.**

---

## Why Volume Appears to Update

The status API reads from `SharedState.volume`, which WAS updated:
- `GET /audio/volume` → returns `ctx.state.get_volume()` ✓
- UI displays updated value ✓
- But `AudioOutput.volume` still at 1.0 ✗

---

## Solution Options

### **Option 1: Store AudioOutput in Engine** ⭐ RECOMMENDED
Add `AudioOutput` reference to `PlaybackEngine` so it can be accessed from API handlers.

**Pros:**
- Clean separation of concerns
- Engine controls all playback aspects
- Easy to add `engine.set_volume()` method

**Cons:**
- AudioOutput not Send (due to cpal::Stream)
- Requires Arc<Mutex<AudioOutput>> wrapper
- More complex engine initialization

**Implementation:**
```rust
// engine.rs
pub struct PlaybackEngine {
    // ... existing fields ...
    audio_output: Arc<Mutex<AudioOutput>>,
}

impl PlaybackEngine {
    pub async fn set_volume(&self, volume: f32) -> Result<()> {
        self.audio_output.lock().unwrap().set_volume(volume);
        Ok(())
    }
}

// handlers.rs
pub async fn set_volume(...) {
    ctx.state.set_volume(req.volume).await;
    crate::db::settings::set_volume(&ctx.db_pool, req.volume).await;

    // NEW: Update audio output
    ctx.engine.set_volume(req.volume).await?;

    ctx.state.broadcast_event(...);
}
```

### **Option 2: Shared Volume Arc**
Pass the same `Arc<Mutex<f32>>` to both AudioOutput and API context.

**Pros:**
- Minimal code changes
- Direct synchronization
- No engine modification needed

**Cons:**
- Adds lock to audio callback path (already exists)
- Volume state duplicated between SharedState and Arc

**Implementation:**
```rust
// main.rs or engine.rs
let shared_volume = Arc::new(Mutex::new(0.5_f32));

// Pass to AudioOutput::new()
let audio_output = AudioOutput::new_with_volume(None, Arc::clone(&shared_volume))?;

// Pass to AppContext
let ctx = AppContext {
    volume: Arc::clone(&shared_volume),
    // ... other fields ...
};

// handlers.rs
pub async fn set_volume(...) {
    *ctx.volume.lock().unwrap() = req.volume.clamp(0.0, 1.0);
    ctx.state.set_volume(req.volume).await;
    // AudioOutput automatically sees updated value
}
```

### **Option 3: Volume Event Listener**
AudioOutput subscribes to VolumeChanged events from SharedState.

**Pros:**
- Event-driven architecture (matches existing pattern)
- No engine changes
- Clean separation

**Cons:**
- More complex threading
- Event processing overhead
- AudioOutput needs async runtime access

---

## Recommended Approach

**Use Option 2 (Shared Volume Arc)** because:
1. Minimal code changes
2. AudioOutput already uses `Arc<Mutex<f32>>` for volume
3. Audio callback already locks volume (no new performance impact)
4. No engine restructuring required
5. Immediate synchronization

---

## Traceability

- **[ARCH-VOL-010]** Master volume control (0.0-1.0)
- **[ARCH-VOL-020]** Volume applied multiplicatively in audio output
- **[DB-SETTINGS-020]** Volume persistence
- **[API]** POST /audio/volume - Set volume level
- **[API]** GET /audio/volume - Get current volume

---

## Next Steps

1. Implement Option 2 (Shared Volume Arc)
2. Add unit tests for volume synchronization
3. Manual test volume control during playback
4. Document volume architecture in SPEC documents

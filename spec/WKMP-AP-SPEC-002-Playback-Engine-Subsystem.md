# WKMP Audio Player - Playback Engine Subsystem Specification

**Document ID:** WKMP-AP-SPEC-002
**Version:** 1.0
**Date:** 2025-10-22
**Parent:** WKMP-AP-SPEC-001 (System Architecture Overview)

---

## 1. Purpose

The Playback Engine Subsystem is the central orchestrator of the WKMP Audio Player. It coordinates queue progression, buffer management, decode scheduling, mixer timing, and event emission. It runs as a continuous loop at approximately 100Hz, making decisions about when to start decoding, when to initiate crossfades, and when to advance the queue.

---

## 2. Architecture Overview

### 2.1 Component Hierarchy

```
PlaybackEngine (orchestrator)
├── QueueManager (current/next/queued tracking)
├── BufferManager (buffer lifecycle state machine)
├── DecoderWorker (serial decode processing)
├── CrossfadeMixer (sample-accurate mixing)
├── AudioOutput (cpal device output)
└── Event channels (buffer events, position events)
```

### 2.2 Execution Model

**Main Loop (Engine Tick):**
- Runs at ~100Hz (10ms intervals)
- Implemented as tokio interval timer
- Non-blocking: all operations are async or lock-free

**Responsibilities per Tick:**
1. Check for buffer events (Ready, Finished, Exhausted)
2. Check for new queue entries
3. Check if crossfade should start
4. Check if current passage finished
5. Advance queue if necessary
6. Pull mixer frame if audio output callback needs data
7. Emit position events periodically

---

## 3. PlaybackEngine Component

### 3.1 Data Structure

```rust
pub struct PlaybackEngine {
    // Core components
    queue: Arc<RwLock<QueueManager>>,
    buffer_manager: Arc<BufferManager>,
    decoder_worker: Arc<DecoderWorker>,
    mixer: Arc<Mutex<CrossfadeMixer>>,
    audio_output: Arc<Mutex<AudioOutput>>,

    // State management
    shared_state: Arc<SharedState>,
    db_pool: Pool<Sqlite>,

    // Event channels
    buffer_event_rx: UnboundedReceiver<BufferEvent>,
    position_event_rx: UnboundedReceiver<PlaybackEvent>,

    // Timing and configuration
    tick_interval: tokio::time::Interval,  // ~10ms (100Hz)
    volume: Arc<Mutex<f32>>,

    // Chain assignment tracking
    chain_assignments: HashMap<Uuid, usize>,  // queue_entry_id -> chain_index
    available_chains: BinaryHeap<Reverse<usize>>,  // Min-heap for allocation

    // Shutdown signal
    shutdown_flag: Arc<AtomicBool>,
}
```

### 3.2 Initialization Sequence

```rust
pub async fn new(db_pool: Pool<Sqlite>, shared_state: Arc<SharedState>) -> Result<Self> {
    // Step 1: Create buffer manager
    let buffer_manager = Arc::new(BufferManager::new());

    // Step 2: Create buffer event channel
    let (buffer_event_tx, buffer_event_rx) = mpsc::unbounded_channel();
    buffer_manager.set_event_channel(buffer_event_tx).await;

    // Step 3: Load configuration from database
    let min_buffer_threshold = load_setting(&db_pool, "min_buffer_threshold_ms").await?;
    buffer_manager.set_min_buffer_threshold(min_buffer_threshold).await;

    // Step 4: Create position event channel
    let (position_event_tx, position_event_rx) = mpsc::unbounded_channel();

    // Step 5: Create mixer
    let mut mixer = CrossfadeMixer::new();
    mixer.set_buffer_manager(Arc::clone(&buffer_manager));
    mixer.set_event_channel(position_event_tx);

    // Step 6: Load queue from database
    let queue_manager = QueueManager::load_from_db(&db_pool).await?;
    let queue = Arc::new(RwLock::new(queue_manager));

    // Step 7: Create decoder worker
    let decoder_worker = Arc::new(DecoderWorker::new(
        Arc::clone(&buffer_manager),
        db_pool.clone(),
    ));

    // Step 8: Initialize audio output
    let device_name = load_setting(&db_pool, "audio_device").await?;
    let volume = Arc::new(Mutex::new(0.75)); // Default 75%
    let audio_output = AudioOutput::new_with_volume(device_name, Some(Arc::clone(&volume)))?;

    // Step 9: Create tick interval
    let tick_interval = tokio::time::interval(Duration::from_millis(10)); // 100Hz

    // Step 10: Initialize chain assignments (for 3 buffer chains)
    let mut available_chains = BinaryHeap::new();
    for i in 0..3 {
        available_chains.push(Reverse(i));
    }

    Ok(Self {
        queue,
        buffer_manager,
        decoder_worker,
        mixer: Arc::new(Mutex::new(mixer)),
        audio_output: Arc::new(Mutex::new(audio_output)),
        shared_state,
        db_pool,
        buffer_event_rx,
        position_event_rx,
        tick_interval,
        volume,
        chain_assignments: HashMap::new(),
        available_chains,
        shutdown_flag: Arc::new(AtomicBool::new(false)),
    })
}
```

### 3.3 Main Loop Implementation

```rust
pub async fn start(self: Arc<Self>) -> Result<()> {
    // Start decoder worker
    self.decoder_worker.start().await?;

    // Start audio output with mixer callback
    let mixer_clone = Arc::clone(&self.mixer);
    self.audio_output.lock().unwrap().start(move || {
        mixer_clone.lock().unwrap().get_next_frame()
    })?;

    // Main loop
    loop {
        // Check for shutdown
        if self.shutdown_flag.load(Ordering::Relaxed) {
            break;
        }

        // Wait for next tick
        self.tick_interval.tick().await;

        // Process one tick
        self.tick().await?;
    }

    Ok(())
}

async fn tick(&self) -> Result<()> {
    // Step 1: Process buffer events
    while let Ok(event) = self.buffer_event_rx.try_recv() {
        self.handle_buffer_event(event).await?;
    }

    // Step 2: Process position events
    while let Ok(event) = self.position_event_rx.try_recv() {
        self.handle_position_event(event).await?;
    }

    // Step 3: Check for new queue entries (need decode)
    self.check_new_queue_entries().await?;

    // Step 4: Check if crossfade should start
    self.check_crossfade_timing().await?;

    // Step 5: Check for crossfade completion
    self.check_crossfade_completion().await?;

    // Step 6: Check if current passage finished
    if self.mixer.lock().unwrap().is_current_finished().await {
        self.handle_passage_finished().await?;
    }

    Ok(())
}
```

### 3.4 Event Handling

**Buffer Events:**

```rust
async fn handle_buffer_event(&self, event: BufferEvent) -> Result<()> {
    match event {
        BufferEvent::ReadyForStart { queue_entry_id, samples_buffered, buffer_duration_ms } => {
            // Buffer reached threshold, can start playback
            let queue = self.queue.read().await;

            // If this is the current passage and mixer is idle, start it
            if let Some(current) = queue.current() {
                if current.queue_entry_id == queue_entry_id {
                    let mixer_state = self.mixer.lock().unwrap().get_current_passage_id();
                    if mixer_state.is_none() {
                        self.start_current_passage().await?;
                    }
                }
            }

            // Emit event to SSE subscribers
            self.shared_state.broadcast_event(WkmpEvent::BufferReady {
                queue_entry_id,
                samples_buffered,
                duration_ms: buffer_duration_ms,
            });
        }

        BufferEvent::Finished { queue_entry_id, total_samples } => {
            // Decode completed for this passage
            debug!("Buffer finished: {}, total_samples={}", queue_entry_id, total_samples);
        }

        BufferEvent::Exhausted { queue_entry_id, headroom } => {
            // Buffer underrun detected
            warn!("Buffer exhausted: {}, headroom={}", queue_entry_id, headroom);
            // Mixer will handle pausing automatically via underrun detection
        }

        // ... other events
    }

    Ok(())
}
```

**Position Events:**

```rust
async fn handle_position_event(&self, event: PlaybackEvent) -> Result<()> {
    match event {
        PlaybackEvent::PositionUpdate { queue_entry_id, position_ms } => {
            // Update shared state for API queries
            self.shared_state.set_current_passage(Some(CurrentPassage {
                queue_entry_id,
                position_ms,
                // ... other fields
            })).await;

            // Broadcast to SSE subscribers
            self.shared_state.broadcast_event(WkmpEvent::PositionUpdate {
                queue_entry_id,
                position_ms,
            });
        }
    }

    Ok(())
}
```

### 3.5 Queue Progression Logic

**Check for New Queue Entries:**

```rust
async fn check_new_queue_entries(&self) -> Result<()> {
    let queue = self.queue.read().await;

    // Decode current if not already decoding
    if let Some(current) = queue.current() {
        if !self.buffer_manager.is_managed(current.queue_entry_id).await {
            self.start_decode(current, DecodePriority::Immediate).await?;
        }
    }

    // Decode next if not already decoding
    if let Some(next) = queue.next() {
        if !self.buffer_manager.is_managed(next.queue_entry_id).await {
            self.start_decode(next, DecodePriority::Next).await?;
        }
    }

    // Decode queued passages (prefetch)
    for queued in queue.queued() {
        if !self.buffer_manager.is_managed(queued.queue_entry_id).await {
            self.start_decode(queued, DecodePriority::Prefetch).await?;
        }
    }

    Ok(())
}
```

**Start Decode:**

```rust
async fn start_decode(&self, entry: &QueueEntry, priority: DecodePriority) -> Result<()> {
    // Allocate buffer chain
    let chain_index = self.allocate_chain(entry.queue_entry_id).await?;

    // Allocate buffer
    let buffer = self.buffer_manager.allocate_buffer(entry.queue_entry_id).await;

    // Dispatch decode task to worker
    self.decoder_worker.dispatch_decode(
        entry.queue_entry_id,
        entry.file_path.clone(),
        entry.start_time_ms.unwrap_or(0),
        entry.end_time_ms.unwrap_or(0),
        priority,
    ).await?;

    Ok(())
}
```

### 3.6 Crossfade Timing Logic

**Check Crossfade Timing:**

```rust
async fn check_crossfade_timing(&self) -> Result<()> {
    let mixer = self.mixer.lock().unwrap();
    let queue = self.queue.read().await;

    // Only start crossfade if in single passage mode
    if mixer.is_crossfading() {
        return Ok(());
    }

    // Need current and next passages
    let Some(current) = queue.current() else { return Ok(()) };
    let Some(next) = queue.next() else { return Ok(()) };

    // Check if next buffer is ready
    if !self.buffer_manager.is_ready(next.queue_entry_id).await {
        return Ok(());
    }

    // Get current mixer position
    let current_position_frames = mixer.get_position();
    let current_position_ms = (current_position_frames as u64 * 1000) / 44100;

    // Calculate crossfade start point
    let lead_out_ms = current.lead_out_point_ms.unwrap_or_else(|| {
        // Default: end of passage
        current.end_time_ms.unwrap_or(0)
    });

    let fade_out_duration_ms = calculate_fade_duration(
        current.lead_out_point_ms,
        current.fade_out_point_ms,
    );

    let crossfade_start_ms = lead_out_ms.saturating_sub(fade_out_duration_ms);

    // Start crossfade if we've reached the start point
    if current_position_ms >= crossfade_start_ms {
        self.initiate_crossfade(current, next).await?;
    }

    Ok(())
}
```

**Initiate Crossfade:**

```rust
async fn initiate_crossfade(&self, current: &QueueEntry, next: &QueueEntry) -> Result<()> {
    // Calculate fade parameters
    let fade_out_curve = parse_fade_curve(&current.fade_out_curve);
    let fade_out_duration_samples = ms_to_samples(calculate_fade_duration(...));

    let fade_in_curve = parse_fade_curve(&next.fade_in_curve);
    let fade_in_duration_samples = ms_to_samples(calculate_fade_duration(...));

    // Start crossfade
    let mut mixer = self.mixer.lock().unwrap();
    mixer.start_crossfade(
        next.queue_entry_id,
        fade_out_curve,
        fade_out_duration_samples,
        fade_in_curve,
        fade_in_duration_samples,
    ).await?;

    // Update shared state
    self.shared_state.broadcast_event(WkmpEvent::CrossfadeStarted {
        current_queue_entry_id: current.queue_entry_id,
        next_queue_entry_id: next.queue_entry_id,
    });

    Ok(())
}
```

**Check Crossfade Completion:**

```rust
async fn check_crossfade_completion(&self) -> Result<()> {
    let mut mixer = self.mixer.lock().unwrap();

    // Check if crossfade just completed
    if let Some(completed_passage_id) = mixer.take_crossfade_completed() {
        // Remove old buffer
        self.buffer_manager.remove(completed_passage_id).await;
        self.release_chain(completed_passage_id).await;

        // Advance queue
        let mut queue = self.queue.write().await;
        queue.advance();

        // Start playback on new passage in mixer
        self.buffer_manager.start_playback(completed_passage_id).await?;

        // Broadcast event
        self.shared_state.broadcast_event(WkmpEvent::PassageCompleted {
            queue_entry_id: completed_passage_id,
        });
    }

    Ok(())
}
```

### 3.7 Passage Completion Logic

```rust
async fn handle_passage_finished(&self) -> Result<()> {
    let mixer = self.mixer.lock().unwrap();
    let Some(finished_id) = mixer.get_current_passage_id() else {
        return Ok(());
    };

    // Remove finished buffer
    self.buffer_manager.remove(finished_id).await;
    self.release_chain(finished_id).await;

    // Advance queue
    let mut queue = self.queue.write().await;
    let new_current = queue.advance();

    if let Some(current) = new_current {
        // Start new passage in mixer
        self.start_current_passage().await?;

        // Broadcast events
        self.shared_state.broadcast_event(WkmpEvent::PassageCompleted {
            queue_entry_id: finished_id,
        });
        self.shared_state.broadcast_event(WkmpEvent::PassageStarted {
            queue_entry_id: current.queue_entry_id,
        });
    } else {
        // Queue empty, stop mixer
        mixer.stop();

        self.shared_state.broadcast_event(WkmpEvent::PassageCompleted {
            queue_entry_id: finished_id,
        });
        self.shared_state.broadcast_event(WkmpEvent::QueueEmpty);
    }

    Ok(())
}
```

### 3.8 Chain Assignment Management

Buffer chains are abstract allocation units (0, 1, 2) that allow the engine to track which queue entries are associated with which buffers. This prevents "ghost passages" in monitoring endpoints.

```rust
async fn allocate_chain(&mut self, queue_entry_id: Uuid) -> Result<usize> {
    if let Some(index) = self.available_chains.pop() {
        let chain_index = index.0;
        self.chain_assignments.insert(queue_entry_id, chain_index);
        Ok(chain_index)
    } else {
        Err(Error::NoAvailableChains)
    }
}

async fn release_chain(&mut self, queue_entry_id: Uuid) {
    if let Some(chain_index) = self.chain_assignments.remove(&queue_entry_id) {
        self.available_chains.push(Reverse(chain_index));
    }
}

pub fn get_chain_assignments(&self) -> HashMap<Uuid, usize> {
    self.chain_assignments.clone()
}
```

---

## 4. QueueManager Component

### 4.1 Data Structure

```rust
pub struct QueueManager {
    // Current passage (playing now)
    current: Option<QueueEntry>,

    // Next passage (gets full buffer)
    next: Option<QueueEntry>,

    // Queued passages (after next)
    queued: Vec<QueueEntry>,

    // Cached total count for O(1) length queries
    total_count: usize,
}

pub struct QueueEntry {
    queue_entry_id: Uuid,
    passage_id: Option<Uuid>,
    file_path: PathBuf,
    play_order: i64,

    // Timing overrides (milliseconds)
    start_time_ms: Option<u64>,
    end_time_ms: Option<u64>,
    lead_in_point_ms: Option<u64>,
    lead_out_point_ms: Option<u64>,
    fade_in_point_ms: Option<u64>,
    fade_out_point_ms: Option<u64>,

    // Fade curves
    fade_in_curve: Option<String>,   // "linear", "exponential", "cosine", etc.
    fade_out_curve: Option<String>,

    // Discovered endpoint (for undefined endpoints)
    discovered_end_ticks: Option<i64>,
}
```

### 4.2 Key Operations

**Load from Database:**

```rust
pub async fn load_from_db(db: &Pool<Sqlite>) -> Result<Self> {
    // Query: SELECT * FROM queue ORDER BY play_order ASC
    let db_entries = queue::get_queue(db).await?;

    // Convert to QueueEntry structs
    let mut entries: Vec<QueueEntry> = db_entries
        .into_iter()
        .map(QueueEntry::from_db)
        .collect::<Result<Vec<_>>>()?;

    // Split into current/next/queued
    let mut manager = Self::new();

    if !entries.is_empty() {
        manager.current = Some(entries.remove(0));
        manager.total_count += 1;
    }

    if !entries.is_empty() {
        manager.next = Some(entries.remove(0));
        manager.total_count += 1;
    }

    manager.queued = entries;
    manager.total_count += manager.queued.len();

    Ok(manager)
}
```

**Advance Queue:**

```rust
pub fn advance(&mut self) -> Option<QueueEntry> {
    // Discard old current (-1 count)
    if self.current.is_some() {
        self.total_count -= 1;
    }

    // Move next → current
    self.current = self.next.take();

    // Move first queued → next
    if !self.queued.is_empty() {
        self.next = Some(self.queued.remove(0));
    }

    self.current.clone()
}
```

**Add Entry:**

```rust
pub fn enqueue(&mut self, entry: QueueEntry) {
    self.total_count += 1;

    // Fill slots in order: current → next → queued
    if self.current.is_none() {
        self.current = Some(entry);
    } else if self.next.is_none() {
        self.next = Some(entry);
    } else {
        self.queued.push(entry);
    }
}
```

**Remove Entry:**

```rust
pub fn remove(&mut self, queue_entry_id: Uuid) -> bool {
    // Check current
    if let Some(ref current) = self.current {
        if current.queue_entry_id == queue_entry_id {
            self.advance();  // Handles count update
            return true;
        }
    }

    // Check next
    if let Some(ref next) = self.next {
        if next.queue_entry_id == queue_entry_id {
            // Replace next with first queued
            if !self.queued.is_empty() {
                self.next = Some(self.queued.remove(0));
            } else {
                self.next = None;
            }
            self.total_count -= 1;
            return true;
        }
    }

    // Check queued
    if let Some(index) = self.queued.iter().position(|e| e.queue_entry_id == queue_entry_id) {
        self.queued.remove(index);
        self.total_count -= 1;
        return true;
    }

    false
}
```

---

## 5. DecoderWorker Component

### 5.1 Purpose

Single-threaded worker that processes decode requests in priority order. Maintains a priority queue of decode tasks and processes them serially to maximize CPU cache efficiency.

### 5.2 Data Structure

```rust
pub struct DecoderWorker {
    buffer_manager: Arc<BufferManager>,
    db_pool: Pool<Sqlite>,
    task_queue: Arc<Mutex<BinaryHeap<DecodeTask>>>,
    shutdown_flag: Arc<AtomicBool>,
}

struct DecodeTask {
    queue_entry_id: Uuid,
    file_path: PathBuf,
    start_ms: u64,
    end_ms: u64,
    priority: DecodePriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecodePriority {
    Immediate = 3,  // Underrun recovery
    Next = 2,       // Next passage in queue
    Prefetch = 1,   // Queued passages
}
```

### 5.3 Worker Loop

```rust
async fn worker_loop(&self) -> Result<()> {
    loop {
        // Check for shutdown
        if self.shutdown_flag.load(Ordering::Relaxed) {
            break;
        }

        // Pop highest priority task
        let task = {
            let mut queue = self.task_queue.lock().unwrap();
            queue.pop()
        };

        if let Some(task) = task {
            self.process_task(task).await?;
        } else {
            // No tasks, sleep briefly
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    Ok(())
}

async fn process_task(&self, task: DecodeTask) -> Result<()> {
    // Create decoder chain
    let chain = DecoderChain::new(
        task.queue_entry_id,
        task.file_path.clone(),
        task.start_ms,
        task.end_ms,
        Arc::clone(&self.buffer_manager),
    );

    // Process decode (decode → resample → push to buffer)
    chain.process().await?;

    Ok(())
}
```

---

## 6. Integration Points

### 6.1 With Buffer Management Subsystem

- Engine allocates buffers via `BufferManager::allocate_buffer()`
- Engine receives `BufferEvent` via channel
- Engine queries buffer readiness via `BufferManager::is_ready()`
- Engine removes buffers via `BufferManager::remove()`

### 6.2 With Audio Processing Subsystem

- Engine creates and configures `CrossfadeMixer`
- Engine calls mixer methods: `start_passage()`, `start_crossfade()`, `stop()`
- Engine queries mixer state: `get_current_passage_id()`, `is_crossfading()`
- Engine receives `PlaybackEvent` via channel

### 6.3 With HTTP API Subsystem

- API handlers call engine methods: `enqueue()`, `play()`, `pause()`, `skip()`
- API handlers query engine state: `get_queue()`, `get_position()`, `get_buffer_status()`
- Engine broadcasts events to SSE subscribers via `SharedState`

### 6.4 With Database Layer

- Engine loads queue on startup via `QueueManager::load_from_db()`
- Engine persists queue changes via database operations
- Engine loads configuration settings
- Engine updates passage timing when endpoints discovered

---

## 7. Timing Precision

### 7.1 Tick Interval

- **Target:** 100Hz (10ms ticks)
- **Implementation:** `tokio::time::interval(Duration::from_millis(10))`
- **Jitter:** Typically <1ms on modern systems
- **Drift:** Corrected automatically by tokio interval

### 7.2 Crossfade Timing Accuracy

- **Position tracking:** Updated every mixer frame (~0.02ms @ 44.1kHz)
- **Crossfade check:** Every engine tick (~10ms)
- **Worst-case delay:** One tick interval (10ms) + mixer frame time (~0.02ms)
- **Typical accuracy:** ±5ms from calculated crossfade start point

### 7.3 Position Event Precision

- **Frame counting:** Atomic counter incremented per frame
- **Event emission:** Configurable interval (default: 1 second)
- **Timestamp accuracy:** Sample-accurate to the frame

---

## 8. Error Handling

### 8.1 Recoverable Errors

**Buffer Underrun:**
- Detection: Mixer checks buffer occupancy before reading
- Response: Output last valid frame (flatline), wait for buffer to refill
- Recovery: Auto-resume when buffer >10% full

**Decode Failure:**
- Detection: Decoder returns error
- Response: Skip passage, advance queue
- Recovery: Continue with next passage

**Audio Device Error:**
- Detection: Audio callback error handler
- Response: Set error flag, attempt device fallback
- Recovery: Reconnect to default device

### 8.2 Non-Recoverable Errors

**Database Corruption:**
- Detection: SQLite query error
- Response: Log error, terminate gracefully

**No Audio Devices:**
- Detection: Device enumeration returns empty list
- Response: Return error from initialization

---

## 9. Performance Optimization

### 9.1 Cache Efficiency

- Serial decode processing maximizes instruction cache hits
- Pre-allocated buffers minimize allocation overhead
- Lock-free ring buffers avoid contention

### 9.2 Memory Management

- Buffer reuse: Buffers cleared and reused
- Chain pooling: Fixed number of chain indices (0, 1, 2)
- String interning: Fade curve names stored as enum

### 9.3 CPU Usage

- Idle: <1% (tick loop overhead)
- Active decode: 10-30% (single-threaded)
- Active playback: <5% (mixer + audio callback)

---

## 10. Testing Approach

### 10.1 Unit Tests

- Queue progression logic
- Chain allocation/deallocation
- Event handling
- Timing calculations

### 10.2 Integration Tests

- End-to-end playback flow
- Crossfade timing accuracy
- Buffer exhaustion recovery
- Queue advancement

### 10.3 Stress Tests

- Large queue (100+ entries)
- Rapid queue modifications
- Sustained playback (hours)
- Memory leak detection

---

## 11. Related Specifications

- `WKMP-AP-SPEC-001` - System Architecture Overview (parent)
- `WKMP-AP-SPEC-004` - Buffer Management Subsystem
- `WKMP-AP-SPEC-010` - Crossfade Mixer Component
- `WKMP-AP-SPEC-008` - Initialization and Lifecycle

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-22 | System Analyst | Initial detailed specification |

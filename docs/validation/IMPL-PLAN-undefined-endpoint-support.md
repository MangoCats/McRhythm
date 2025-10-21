# Implementation Plan: Undefined Endpoint Support

**Document Type:** Implementation Plan
**Created:** 2025-10-21
**Status:** Planning - Ready for Review
**Priority:** High
**Related Requirements:** [XFD-PT-060], [ENT-MP-035], [DBD-DEC-070], [DBD-DEC-080]

---

## Executive Summary

This document provides a detailed implementation plan for supporting passages with undefined endpoints (`end_time = None`). When a passage's `end_time` is not explicitly defined in the database, the system must discover the actual audio file duration during decoding and propagate this information through the playback pipeline.

**Key Challenge:** The file duration is unknown until decode begins, but crossfade timing calculation, buffer allocation, and mixer logic all depend on knowing the passage endpoint.

**Solution Strategy:** Implement two-phase passage lifecycle:
1. **Phase 1 (Discovery):** Decoder discovers actual endpoint during initial decode pass
2. **Phase 2 (Propagation):** Endpoint information flows: decoder → buffer → queue → mixer

---

## 1. Gap Analysis Summary

Based on IMPL-ANALYSIS-002-gap-analysis.json and codebase review, the undefined endpoint issue manifests in:

### 1.1 Current Behavior

**Database Layer (`wkmp-ap/src/db/passages.rs:76-80`):**
```rust
let end_time_ticks = match (row.get::<Option<f64>, _>("end_time"), file_duration_s) {
    (Some(end), _) => Some(wkmp_common::timing::seconds_to_ticks(end)),
    (None, Some(duration)) => Some(wkmp_common::timing::seconds_to_ticks(duration)),
    (None, None) => None, // File duration unknown ← PROBLEM
};
```

**Issue:** When both `end_time` (passage field) and `file_duration_s` (file metadata) are NULL, `PassageWithTiming.end_time_ticks` is set to `None`. This creates problems downstream.

### 1.2 Affected Components

| Component | Current State | Problem | Impact |
|-----------|---------------|---------|--------|
| **Decoder** | Decodes until EOF | No endpoint validation | Over-decodes files |
| **BufferManager** | Allocates fixed-size buffer | Can't pre-calculate capacity | Wastes memory |
| **CrossfadeMixer** | Requires known endpoint | Can't calculate crossfade timing | Crossfade fails |
| **QueueManager** | Stores timing metadata | Missing completion detection | Can't determine when passage ends |
| **SSE Events** | Reports buffer fill % | Denominator unknown | Incorrect progress display |

### 1.3 Requirements Violated

- **[XFD-IMPL-020]**: Crossfade calculation requires `passage_a.end_time ?? file_duration_a`
- **[DBD-DEC-080]**: "Timing for passage boundaries handled with exact sample accuracy"
- **[DBD-BUF-060]**: "When end time sample removed from buffer, inform queue passage playout completed"
- **[ENT-MP-035]**: "Passage undefined metadata shall be handled as passage which ends at end of file"

---

## 2. Proposed Architecture

### 2.1 Endpoint Discovery Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Enqueue Passage                             │
│  PassageWithTiming { end_time_ticks: None }                        │
└────────────────────────┬────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    BufferManager.register_decoding()                │
│  Creates ManagedBuffer with endpoint_discovered: false              │
└────────────────────────┬────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   SerialDecoder.submit()                            │
│  DecodeRequest { passage.end_time_ticks: None }                    │
└────────────────────────┬────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────────┐
│              Worker: decode_passage_with_fades()                    │
│  1. SimpleDecoder::decode_passage()                                 │
│     → Decodes until EOF                                             │
│     → Returns: (samples, sample_rate, channels, ACTUAL_DURATION)    │
│                                                                      │
│  2. Convert actual duration to ticks:                               │
│     discovered_end_ticks = samples_to_ticks(total_samples, rate)    │
│                                                                      │
│  3. Notify BufferManager:                                           │
│     buffer_manager.set_discovered_endpoint(queue_entry_id, ticks)   │
└────────────────────────┬────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────────┐
│        BufferManager.set_discovered_endpoint()                      │
│  1. Update ManagedBuffer.metadata.discovered_end_ticks              │
│  2. Update PassageBuffer.total_samples (exact count)                │
│  3. Emit BufferEvent::EndpointDiscovered { queue_entry_id, ticks }  │
└────────────────────────┬────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────────┐
│               PlaybackEngine event handler                          │
│  Listen: BufferEvent::EndpointDiscovered                            │
│  Action: queue_manager.update_endpoint(queue_entry_id, ticks)       │
└────────────────────────┬────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────────┐
│          QueueManager.update_endpoint()                             │
│  1. Find QueueEntry by id                                           │
│  2. Update cached_end_ticks field                                   │
│  3. Recalculate crossfade timing if entry is position 0 or 1        │
│  4. Emit QueueEvent::TimingUpdated                                  │
└────────────────────────┬────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────────┐
│            CrossfadeMixer uses updated endpoint                     │
│  - Accurate crossfade duration calculation                          │
│  - Correct passage completion detection                             │
│  - Sample-accurate fade-out timing                                  │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 Data Flow Diagram

```
Passage DB         PassageWithTiming      DecodeRequest       Worker Thread
(end_time=NULL) → (end_time_ticks=None) → (passage.end=None) → Decode to EOF
                                                                    │
                                                                    ▼
                                                           Discover: 15,234,567 ticks
                                                                    │
                                                                    ▼
BufferManager ← set_discovered_endpoint(ticks) ← BufferEvent::EndpointDiscovered
    │
    ├→ Update ManagedBuffer.metadata.discovered_end_ticks
    ├→ Update PassageBuffer.total_samples
    └→ Emit event to PlaybackEngine
                                    │
                                    ▼
                          QueueManager.update_endpoint()
                                    │
                                    ├→ Update QueueEntry.cached_end_ticks
                                    ├→ Recalculate crossfade timing
                                    └→ Emit QueueEvent::TimingUpdated
                                                    │
                                                    ▼
                                          CrossfadeMixer uses endpoint
                                          for accurate completion detection
```

---

## 3. Component Modification Plan

### 3.1 Phase 1: Core Endpoint Discovery (Decoder → Buffer)

#### File: `wkmp-ap/src/audio/decoder.rs`

**Change 1: Modify SimpleDecoder::decode_passage() return type**

**Current Signature (Line ~45):**
```rust
pub fn decode_passage(
    file_path: &Path,
    start_ticks: i64,
    end_ticks: Option<i64>,
) -> Result<(Vec<f32>, u32, u16)>
```

**New Signature:**
```rust
pub fn decode_passage(
    file_path: &Path,
    start_ticks: i64,
    end_ticks: Option<i64>,
) -> Result<DecodeResult>

pub struct DecodeResult {
    /// Decoded samples (interleaved)
    pub samples: Vec<f32>,

    /// Source sample rate
    pub sample_rate: u32,

    /// Channel count
    pub channels: u16,

    /// Discovered file duration in ticks (always populated)
    /// For passages with end_ticks=Some, this equals end_ticks.
    /// For passages with end_ticks=None, this is the actual file duration.
    pub actual_end_ticks: i64,
}
```

**Implementation (Lines ~80-150):**
```rust
impl SimpleDecoder {
    pub fn decode_passage(
        file_path: &Path,
        start_ticks: i64,
        end_ticks: Option<i64>,
    ) -> Result<DecodeResult> {
        // Open file and get source sample rate
        let file = File::open(file_path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut reader = symphonia::default::get_probe()
            .format(&Default::default(), mss, &Default::default(), &Default::default())?
            .format;

        let track = reader.default_track()
            .ok_or_else(|| Error::Decode("No audio track".into()))?;
        let sample_rate = track.codec_params.sample_rate
            .ok_or_else(|| Error::Decode("No sample rate".into()))? as u32;
        let channels = track.codec_params.channels
            .ok_or_else(|| Error::Decode("No channels".into()))?
            .count() as u16;

        // Convert start/end ticks to source sample positions
        let start_sample = wkmp_common::timing::ticks_to_samples(start_ticks, sample_rate);

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &Default::default())?;

        let mut samples = Vec::new();
        let mut current_sample: usize = 0;
        let mut total_decoded: usize = 0;

        // Decode loop
        loop {
            let packet = match reader.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // Reached EOF - this is the actual file end
                    break;
                }
                Err(e) => return Err(Error::from(e)),
            };

            let decoded = decoder.decode(&packet)?;
            let spec = *decoded.spec();
            let duration = decoded.capacity() as usize;

            // Process decoded samples
            for frame_idx in 0..duration {
                if let Some(end_sample) = end_ticks.map(|t| wkmp_common::timing::ticks_to_samples(t, sample_rate)) {
                    // Explicit end point - stop when reached
                    if current_sample >= end_sample {
                        break;
                    }
                }

                // Collect samples between start and end (or EOF)
                if current_sample >= start_sample {
                    // Extract samples for all channels
                    for ch in 0..channels as usize {
                        let sample = decoded.chan(ch)[frame_idx];
                        samples.push(sample);
                    }
                }

                current_sample += 1;
                total_decoded += 1;
            }

            // Check if we've reached explicit end point
            if let Some(end_sample) = end_ticks.map(|t| wkmp_common::timing::ticks_to_samples(t, sample_rate)) {
                if current_sample >= end_sample {
                    break;
                }
            }
        }

        // Calculate actual end ticks
        let actual_end_ticks = if let Some(end_t) = end_ticks {
            // Explicit end point was provided
            end_t
        } else {
            // Undefined end - use total samples decoded
            wkmp_common::timing::samples_to_ticks(total_decoded, sample_rate)
        };

        Ok(DecodeResult {
            samples,
            sample_rate,
            channels,
            actual_end_ticks,
        })
    }
}
```

**Impact:**
- **Callers Updated:** All decode_passage() call sites must be updated
- **Breaking Change:** Yes - return type changed
- **Traceability:** [DBD-DEC-070], [DBD-DEC-080]

---

#### File: `wkmp-common/src/timing.rs`

**Change 2: Add samples_to_ticks() conversion function**

**Location:** New function (append to module)
```rust
/// Convert sample count to ticks
///
/// **[SRC-CONV-030]** ticks = samples × (TICK_RATE ÷ sample_rate)
///
/// # Arguments
/// * `samples` - Number of samples
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// Tick count representing the duration of `samples` at `sample_rate`
pub fn samples_to_ticks(samples: usize, sample_rate: u32) -> i64 {
    // Use i128 intermediate to prevent overflow for large sample counts
    let samples_i128 = samples as i128;
    let tick_rate_i128 = TICK_RATE as i128;
    let sample_rate_i128 = sample_rate as i128;

    let ticks = (samples_i128 * tick_rate_i128) / sample_rate_i128;
    ticks as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_samples_to_ticks() {
        // 220,500 samples @ 44.1kHz = 5 seconds = 141,120,000 ticks
        assert_eq!(samples_to_ticks(220_500, 44100), 141_120_000);

        // 240,000 samples @ 48kHz = 5 seconds = 141,120,000 ticks
        assert_eq!(samples_to_ticks(240_000, 48000), 141_120_000);

        // 1 sample @ 44.1kHz = 640 ticks (exact division)
        assert_eq!(samples_to_ticks(1, 44100), 640);
    }

    #[test]
    fn test_samples_to_ticks_roundtrip() {
        // Verify roundtrip: samples → ticks → samples
        let original_samples = 500_000usize;
        let sample_rate = 44100u32;

        let ticks = samples_to_ticks(original_samples, sample_rate);
        let recovered_samples = ticks_to_samples(ticks, sample_rate);

        // Should recover exact sample count (no precision loss)
        assert_eq!(recovered_samples, original_samples);
    }
}
```

**Impact:**
- **New Function:** Added to existing timing module
- **Traceability:** [SRC-CONV-030]

---

#### File: `wkmp-ap/src/audio/types.rs`

**Change 3: Add discovered endpoint tracking to PassageBuffer**

**Current PassageBuffer (Lines ~45-80):**
```rust
#[derive(Debug)]
pub struct PassageBuffer {
    pub samples: Vec<f32>,
    pub capacity: usize,
    pub write_position: usize,
    pub is_finalized: bool,
}
```

**Modified PassageBuffer:**
```rust
#[derive(Debug)]
pub struct PassageBuffer {
    /// Interleaved stereo samples [L, R, L, R, ...]
    pub samples: Vec<f32>,

    /// Maximum buffer capacity (stereo frames × 2)
    pub capacity: usize,

    /// Current write position (stereo frames written × 2)
    pub write_position: usize,

    /// Total samples expected (set when endpoint discovered)
    /// None = endpoint not yet discovered
    /// Some(n) = buffer will contain exactly n samples when complete
    pub total_samples: Option<usize>,

    /// Whether decode has completed
    pub is_finalized: bool,
}

impl PassageBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(capacity),
            capacity,
            write_position: 0,
            total_samples: None, // Unknown until endpoint discovered
            is_finalized: false,
        }
    }

    /// Set the total sample count (called when endpoint discovered)
    pub fn set_total_samples(&mut self, total: usize) {
        self.total_samples = Some(total);
    }

    /// Get buffer fill percentage
    pub fn fill_percent(&self) -> f32 {
        if let Some(total) = self.total_samples {
            // Endpoint known - use actual total
            (self.write_position as f32 / total as f32) * 100.0
        } else {
            // Endpoint unknown - use capacity as denominator
            (self.write_position as f32 / self.capacity as f32) * 100.0
        }
    }

    /// Check if buffer has reached expected total
    pub fn is_complete(&self) -> bool {
        match self.total_samples {
            Some(total) => self.write_position >= total,
            None => false, // Can't be complete if we don't know the endpoint
        }
    }
}
```

**Impact:**
- **Data Structure Change:** PassageBuffer modified
- **Traceability:** [DBD-BUF-010], [DBD-BUF-060]

---

#### File: `wkmp-ap/src/playback/buffer_manager.rs`

**Change 4: Add endpoint discovery support to BufferManager**

**New Event Type (add to wkmp-common/src/events.rs):**
```rust
/// Buffer lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BufferEvent {
    /// Buffer registered for passage
    Registered {
        queue_entry_id: Uuid,
        passage_id: Option<Uuid>,
    },

    /// NEW: Endpoint discovered during decode
    EndpointDiscovered {
        queue_entry_id: Uuid,
        discovered_end_ticks: i64,
        total_samples: usize,
    },

    /// Buffer ready for playback
    ReadyForStart {
        queue_entry_id: Uuid,
    },

    /// Buffer exhausted (needs refill)
    Exhausted {
        queue_entry_id: Uuid,
    },

    /// Buffer state changed
    StateChanged {
        queue_entry_id: Uuid,
        state: BufferState,
    },
}
```

**ManagedBuffer Metadata Extension (Lines ~35-60):**
```rust
#[derive(Debug)]
pub struct BufferMetadata {
    pub queue_entry_id: Uuid,
    pub passage_id: Option<Uuid>,
    pub state: BufferState,
    pub created_at: Instant,
    pub read_position: usize,

    /// NEW: Discovered endpoint (ticks)
    /// None = endpoint not yet discovered
    /// Some(ticks) = actual file end time in ticks
    pub discovered_end_ticks: Option<i64>,

    /// NEW: Discovered total samples
    /// None = total not yet known
    /// Some(n) = buffer will contain exactly n samples
    pub discovered_total_samples: Option<usize>,
}

impl BufferMetadata {
    pub fn new(queue_entry_id: Uuid, passage_id: Option<Uuid>) -> Self {
        Self {
            queue_entry_id,
            passage_id,
            state: BufferState::Decoding,
            created_at: Instant::now(),
            read_position: 0,
            discovered_end_ticks: None,      // NEW
            discovered_total_samples: None,  // NEW
        }
    }

    /// Check if endpoint has been discovered
    pub fn has_discovered_endpoint(&self) -> bool {
        self.discovered_end_ticks.is_some()
    }
}
```

**New Method: set_discovered_endpoint (Lines ~450-490):**
```rust
impl BufferManager {
    /// Set the discovered endpoint for a passage buffer
    ///
    /// Called by decoder when it discovers the actual file duration.
    /// Updates buffer metadata and emits EndpointDiscovered event.
    ///
    /// **[DBD-BUF-060]** Buffer tracks discovered endpoint for completion detection
    ///
    /// # Arguments
    /// * `queue_entry_id` - Buffer identifier
    /// * `discovered_end_ticks` - Actual file end time in ticks
    /// * `total_samples` - Total stereo samples (interleaved count)
    pub async fn set_discovered_endpoint(
        &self,
        queue_entry_id: Uuid,
        discovered_end_ticks: i64,
        total_samples: usize,
    ) -> Result<(), String> {
        let mut buffers = self.buffers.write().await;

        let managed = buffers.get_mut(&queue_entry_id)
            .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;

        // Update metadata
        managed.metadata.discovered_end_ticks = Some(discovered_end_ticks);
        managed.metadata.discovered_total_samples = Some(total_samples);

        // Update buffer total_samples
        let mut buffer = managed.buffer.write().await;
        buffer.set_total_samples(total_samples);
        drop(buffer); // Release lock before event emission

        info!(
            "Endpoint discovered for queue_entry={}: end_ticks={}, total_samples={}",
            queue_entry_id, discovered_end_ticks, total_samples
        );

        // Emit event
        self.event_tx.send(BufferEvent::EndpointDiscovered {
            queue_entry_id,
            discovered_end_ticks,
            total_samples,
        }).ok();

        Ok(())
    }

    /// Get discovered endpoint for a buffer
    pub async fn get_discovered_endpoint(&self, queue_entry_id: Uuid) -> Option<i64> {
        let buffers = self.buffers.read().await;
        buffers.get(&queue_entry_id)
            .and_then(|m| m.metadata.discovered_end_ticks)
    }
}
```

**Impact:**
- **New Method:** set_discovered_endpoint()
- **Data Structure Change:** BufferMetadata extended
- **New Event:** BufferEvent::EndpointDiscovered
- **Traceability:** [DBD-BUF-060]

---

### 3.2 Phase 2: Endpoint Propagation (Buffer → Queue → Mixer)

#### File: `wkmp-ap/src/playback/serial_decoder.rs`

**Change 5: Call set_discovered_endpoint after decode**

**Modified decode_passage_with_fades (Lines ~350-450):**
```rust
fn decode_passage_with_fades(
    request: &DecodeRequest,
    buffer_handle: Arc<RwLock<PassageBuffer>>,
    buffer_manager: Arc<BufferManager>,
    rt_handle: &tokio::runtime::Handle,
) -> Result<()> {
    let passage = &request.passage;
    let queue_entry_id = request.queue_entry_id;

    // 1. Decode passage (returns actual endpoint)
    let decode_result = SimpleDecoder::decode_passage(
        &passage.file_path,
        passage.start_time_ticks,
        passage.end_time_ticks, // May be None
    )?;

    let DecodeResult {
        samples,
        sample_rate,
        channels,
        actual_end_ticks, // NEW: always populated
    } = decode_result;

    // 2. Resample to 44.1kHz if needed
    let resampled_samples = if sample_rate != STANDARD_SAMPLE_RATE {
        Resampler::resample(&samples, sample_rate, channels)?
    } else {
        samples
    };

    // 3. Convert to stereo
    let stereo_samples = convert_to_stereo(resampled_samples, channels);

    // 4. Calculate fade points in samples (at 44.1kHz)
    let total_samples = stereo_samples.len();
    let start_sample = wkmp_common::timing::ticks_to_samples(passage.start_time_ticks, STANDARD_SAMPLE_RATE);
    let end_sample = wkmp_common::timing::ticks_to_samples(actual_end_ticks, STANDARD_SAMPLE_RATE);

    // NEW: Notify buffer manager of discovered endpoint
    if passage.end_time_ticks.is_none() {
        // Endpoint was undefined, now discovered
        rt_handle.block_on(async {
            if let Err(e) = buffer_manager.set_discovered_endpoint(
                queue_entry_id,
                actual_end_ticks,
                total_samples,
            ).await {
                warn!("Failed to set discovered endpoint: {}", e);
            }
        });
    }

    // 5. Apply fade-in (pre-buffer) [DBD-FADE-030]
    let fade_in_end_sample = wkmp_common::timing::ticks_to_samples(passage.fade_in_point_ticks, STANDARD_SAMPLE_RATE);
    apply_fade_in(&mut stereo_samples[..fade_in_end_sample], passage.fade_in_curve);

    // 6. Apply fade-out (pre-buffer) [DBD-FADE-050]
    let fade_out_start_ticks = passage.fade_out_point_ticks.unwrap_or(actual_end_ticks); // Use discovered endpoint
    let fade_out_start_sample = wkmp_common::timing::ticks_to_samples(fade_out_start_ticks, STANDARD_SAMPLE_RATE);
    apply_fade_out(&mut stereo_samples[fade_out_start_sample..], passage.fade_out_curve);

    // 7. Append samples to buffer in chunks
    const CHUNK_SIZE: usize = 88200; // 1 second @ 44.1kHz stereo
    for (chunk_idx, chunk) in stereo_samples.chunks(CHUNK_SIZE).enumerate() {
        rt_handle.block_on(async {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(chunk.to_vec());
        });

        rt_handle.block_on(async {
            buffer_manager.notify_samples_appended(queue_entry_id, chunk.len()).await;
        });

        // Update progress
        let progress = ((chunk_idx + 1) * 100 / ((total_samples + CHUNK_SIZE - 1) / CHUNK_SIZE)).min(100) as u8;
        if progress % 10 == 0 || progress == 100 {
            rt_handle.block_on(async {
                buffer_manager.update_decode_progress(queue_entry_id, progress).await;
            });
        }
    }

    // 8. Finalize buffer
    rt_handle.block_on(async {
        buffer_manager.finalize_buffer(queue_entry_id, total_samples).await;
        buffer_manager.mark_ready(queue_entry_id).await;
    });

    Ok(())
}
```

**Impact:**
- **Decoder Modified:** Calls set_discovered_endpoint()
- **Traceability:** [DBD-DEC-080], [DBD-FADE-030], [DBD-FADE-050]

---

#### File: `wkmp-ap/src/playback/queue_manager.rs`

**Change 6: Add endpoint caching to QueueEntry**

**Modified QueueEntry (Lines ~25-50):**
```rust
#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub id: Uuid,
    pub passage_id: Option<Uuid>,
    pub passage: PassageWithTiming,
    pub enqueued_at: DateTime<Utc>,

    /// NEW: Cached endpoint (ticks)
    /// Initially copied from passage.end_time_ticks.
    /// Updated when decoder discovers actual endpoint.
    pub cached_end_ticks: Option<i64>,
}

impl QueueEntry {
    pub fn new(passage: PassageWithTiming) -> Self {
        Self {
            id: Uuid::new_v4(),
            passage_id: passage.passage_id,
            cached_end_ticks: passage.end_time_ticks, // Copy initial value
            passage,
            enqueued_at: Utc::now(),
        }
    }

    /// Get effective end time (cached or passage default)
    pub fn effective_end_ticks(&self) -> Option<i64> {
        self.cached_end_ticks.or(self.passage.end_time_ticks)
    }
}
```

**New Method: update_endpoint (Lines ~350-390):**
```rust
impl QueueManager {
    /// Update cached endpoint for a queue entry
    ///
    /// Called when decoder discovers actual file duration.
    /// Updates cached endpoint and recalculates crossfade timing if needed.
    ///
    /// **[DBD-BUF-060]** Queue uses discovered endpoint for completion detection
    ///
    /// # Arguments
    /// * `queue_entry_id` - Queue entry identifier
    /// * `discovered_end_ticks` - Actual file end time in ticks
    pub async fn update_endpoint(
        &self,
        queue_entry_id: Uuid,
        discovered_end_ticks: i64,
    ) -> Result<(), String> {
        let mut queue = self.queue.write().await;

        // Find entry
        let entry = queue.iter_mut()
            .find(|e| e.id == queue_entry_id)
            .ok_or_else(|| format!("Queue entry not found: {}", queue_entry_id))?;

        // Update cached endpoint
        let old_end = entry.cached_end_ticks;
        entry.cached_end_ticks = Some(discovered_end_ticks);

        info!(
            "Updated endpoint for queue_entry={}: {:?} → {} ticks",
            queue_entry_id, old_end, discovered_end_ticks
        );

        // If this entry is position 0 or 1, recalculate crossfade
        let entry_position = queue.iter().position(|e| e.id == queue_entry_id);
        if matches!(entry_position, Some(0) | Some(1)) {
            info!("Recalculating crossfade timing due to endpoint discovery");
            // Recalculation happens in get_current_and_next() / crossfade logic
        }

        drop(queue); // Release lock before event emission

        // Emit timing updated event
        self.event_tx.send(QueueEvent::TimingUpdated {
            queue_entry_id,
            discovered_end_ticks,
        }).ok();

        Ok(())
    }
}
```

**Impact:**
- **Data Structure Change:** QueueEntry extended
- **New Method:** update_endpoint()
- **Traceability:** [DBD-BUF-060], [XFD-IMPL-020]

---

#### File: `wkmp-ap/src/playback/engine.rs`

**Change 7: Listen to BufferEvent::EndpointDiscovered**

**Add event handler (Lines ~800-850):**
```rust
impl PlaybackEngine {
    /// Handle buffer manager events
    async fn handle_buffer_event(&self, event: BufferEvent) {
        match event {
            BufferEvent::EndpointDiscovered {
                queue_entry_id,
                discovered_end_ticks,
                total_samples,
            } => {
                info!(
                    "Endpoint discovered: queue_entry={}, end_ticks={}, total_samples={}",
                    queue_entry_id, discovered_end_ticks, total_samples
                );

                // Update queue manager with discovered endpoint
                if let Err(e) = self.queue_manager.update_endpoint(
                    queue_entry_id,
                    discovered_end_ticks,
                ).await {
                    warn!("Failed to update queue endpoint: {}", e);
                }
            }

            BufferEvent::ReadyForStart { queue_entry_id } => {
                // Existing handler
                // ...
            }

            BufferEvent::Exhausted { queue_entry_id } => {
                // Existing handler
                // ...
            }

            // ... other event handlers
        }
    }

    /// Start event listener task
    fn spawn_event_listener(&self) {
        let buffer_rx = self.buffer_manager.subscribe_events();
        let engine = self.clone();

        tokio::spawn(async move {
            let mut rx = buffer_rx;
            while let Ok(event) = rx.recv().await {
                engine.handle_buffer_event(event).await;
            }
        });
    }
}
```

**Impact:**
- **Event Handler Added:** handle_buffer_event()
- **Traceability:** [DBD-OV-050]

---

### 3.3 Phase 3: Crossfade Timing Integration

#### File: `wkmp-ap/src/playback/pipeline/mixer.rs`

**Change 8: Use cached endpoint in crossfade calculations**

**Modified crossfade timing calculation (Lines ~650-750):**
```rust
impl CrossfadeMixer {
    /// Calculate when to start next passage for crossfade
    ///
    /// [XFD-IMPL-020] Crossfade timing algorithm
    fn calculate_crossfade_start(
        &self,
        current_entry: &QueueEntry,
        next_entry: &QueueEntry,
        crossfade_time_setting: f64,
    ) -> CrossfadeTiming {
        // Step 1: Get effective end time for current passage
        // Use cached_end_ticks if available (handles undefined endpoints)
        let current_end_ticks = current_entry.effective_end_ticks()
            .unwrap_or_else(|| {
                warn!(
                    "Current passage {} has no endpoint (cached or defined), using 0",
                    current_entry.id
                );
                0
            });

        // Step 2: Calculate lead-out duration
        let current_lead_out_point = current_entry.passage.lead_out_point_ticks
            .unwrap_or_else(|| {
                // Use global crossfade time
                let crossfade_ticks = wkmp_common::timing::seconds_to_ticks(crossfade_time_setting);
                current_end_ticks - crossfade_ticks.max(0)
            });

        let current_lead_out_duration = current_end_ticks - current_lead_out_point;

        // Step 3: Calculate lead-in duration for next passage
        let next_start_ticks = next_entry.passage.start_time_ticks;
        let next_lead_in_point = next_entry.passage.lead_in_point_ticks;
        let next_lead_in_duration = next_lead_in_point - next_start_ticks;

        // Step 4: Determine crossfade duration
        let crossfade_duration_ticks = current_lead_out_duration.min(next_lead_in_duration);

        // Step 5: Calculate when to start next passage
        let next_start_time_ticks = current_end_ticks - crossfade_duration_ticks;

        CrossfadeTiming {
            current_end_ticks,
            next_start_ticks: next_start_time_ticks,
            crossfade_duration_ticks,
        }
    }
}
```

**Impact:**
- **Crossfade Logic Updated:** Uses QueueEntry.effective_end_ticks()
- **Traceability:** [XFD-IMPL-020], [XFD-IMPL-030]

---

### 3.4 Phase 4: Fade Application with Discovered Endpoints

#### File: `wkmp-ap/src/playback/serial_decoder.rs`

**Change 9: Fade-out timing uses discovered endpoint**

**Already implemented in Change 5 (Lines ~425-430):**
```rust
// 6. Apply fade-out (pre-buffer) [DBD-FADE-050]
let fade_out_start_ticks = passage.fade_out_point_ticks
    .unwrap_or(actual_end_ticks); // Use discovered endpoint if fade_out_point undefined
let fade_out_start_sample = wkmp_common::timing::ticks_to_samples(fade_out_start_ticks, STANDARD_SAMPLE_RATE);
apply_fade_out(&mut stereo_samples[fade_out_start_sample..], passage.fade_out_curve);
```

**Impact:**
- **Fade-Out Timing:** Uses discovered endpoint when passage.fade_out_point_ticks is None
- **Traceability:** [DBD-FADE-050]

---

### 3.5 Phase 5: Metadata Caching (Optional Optimization)

#### File: `wkmp-ap/src/db/files.rs` (new)

**Change 10: Cache discovered file durations in database**

**New Function: update_file_duration (optional enhancement):**
```rust
/// Update file duration in database after discovery
///
/// Called when decoder discovers actual file duration.
/// Caches duration to avoid re-discovery on future enqueues.
///
/// **Performance Optimization:** Reduces decode overhead for passages with undefined endpoints.
pub async fn update_file_duration(
    db: &Pool<Sqlite>,
    file_path: &Path,
    duration_ticks: i64,
) -> Result<()> {
    let duration_seconds = wkmp_common::timing::ticks_to_seconds(duration_ticks);

    sqlx::query(
        r#"
        UPDATE files
        SET duration = ?
        WHERE path = ? AND duration IS NULL
        "#,
    )
    .bind(duration_seconds)
    .bind(file_path.to_str().ok_or_else(|| Error::InvalidPath)?)
    .execute(db)
    .await?;

    Ok(())
}
```

**Impact:**
- **Optional:** Reduces re-discovery overhead for frequently-played files
- **Traceability:** Performance optimization (no requirement ID)

---

## 4. Implementation Phases

### Phase 1: Core Endpoint Discovery (Decoder → Buffer)
**Goal:** Decoder discovers actual file duration and reports it to BufferManager

**Deliverables:**
1. ✅ SimpleDecoder::decode_passage() returns DecodeResult with actual_end_ticks
2. ✅ wkmp_common::timing::samples_to_ticks() conversion function
3. ✅ PassageBuffer.total_samples field
4. ✅ BufferManager.set_discovered_endpoint() method
5. ✅ BufferEvent::EndpointDiscovered event

**Files Changed:**
- `wkmp-ap/src/audio/decoder.rs` (Change 1)
- `wkmp-common/src/timing.rs` (Change 2)
- `wkmp-ap/src/audio/types.rs` (Change 3)
- `wkmp-ap/src/playback/buffer_manager.rs` (Change 4)
- `wkmp-common/src/events.rs` (BufferEvent variant)

**Testing:**
- Unit test: samples_to_ticks() roundtrip accuracy
- Unit test: PassageBuffer.fill_percent() with/without total_samples
- Integration test: Decode file with undefined endpoint, verify EndpointDiscovered event

**Estimated Effort:** 6-8 hours

---

### Phase 2: Endpoint Propagation (Buffer → Queue)
**Goal:** BufferManager events flow to QueueManager, update cached endpoints

**Deliverables:**
1. ✅ SerialDecoder calls set_discovered_endpoint() after decode
2. ✅ QueueEntry.cached_end_ticks field
3. ✅ QueueManager.update_endpoint() method
4. ✅ PlaybackEngine event listener for BufferEvent::EndpointDiscovered

**Files Changed:**
- `wkmp-ap/src/playback/serial_decoder.rs` (Change 5)
- `wkmp-ap/src/playback/queue_manager.rs` (Change 6)
- `wkmp-ap/src/playback/engine.rs` (Change 7)

**Testing:**
- Integration test: Enqueue passage with end_time=None, verify queue updated after decode
- Integration test: Multiple passages with undefined endpoints, verify all cached
- Manual test: Developer UI shows correct passage duration after discovery

**Estimated Effort:** 4-6 hours

---

### Phase 3: Crossfade Timing Integration
**Goal:** Mixer uses cached endpoints for accurate crossfade timing

**Deliverables:**
1. ✅ CrossfadeMixer.calculate_crossfade_start() uses QueueEntry.effective_end_ticks()
2. ✅ Crossfade timing recalculated when endpoint discovered

**Files Changed:**
- `wkmp-ap/src/playback/pipeline/mixer.rs` (Change 8)

**Testing:**
- Integration test: Crossfade between passage with undefined end → defined start
- Integration test: Verify crossfade duration calculated correctly after discovery
- Audio quality test: Listen to crossfade, verify no clicks/pops

**Estimated Effort:** 3-4 hours

---

### Phase 4: Fade Application with Discovered Endpoints
**Goal:** Fade-out timing uses discovered endpoint

**Deliverables:**
1. ✅ Fade-out application uses actual_end_ticks when passage.fade_out_point_ticks is None

**Files Changed:**
- `wkmp-ap/src/playback/serial_decoder.rs` (Change 9 - already in Change 5)

**Testing:**
- Audio quality test: Passage with undefined end + undefined fade-out fades correctly
- Unit test: Fade-out samples have correct curve applied at discovered endpoint

**Estimated Effort:** 2-3 hours (mostly testing)

---

### Phase 5: Metadata Caching (Optional)
**Goal:** Cache discovered durations in database to avoid re-discovery

**Deliverables:**
1. ✅ update_file_duration() function
2. ✅ SerialDecoder calls update_file_duration() after discovery

**Files Changed:**
- `wkmp-ap/src/db/files.rs` (Change 10)
- `wkmp-ap/src/playback/serial_decoder.rs` (add cache call)

**Testing:**
- Integration test: First enqueue discovers duration, second enqueue uses cached value
- Performance test: Measure decode time reduction for cached files

**Estimated Effort:** 2-3 hours

**Deferred:** Can be implemented after Phases 1-4 are complete and tested.

---

## 5. Risk Assessment

### 5.1 Breaking Changes

| Change | Impact | Mitigation |
|--------|--------|------------|
| SimpleDecoder return type | All callers must update | Compile-time error, easy to find all sites |
| PassageBuffer fields | Serialization compatibility | PassageBuffer not serialized, safe |
| QueueEntry fields | Database schema | QueueEntry not persisted to DB, safe |
| BufferEvent variants | Event handlers | Add new variant, existing handlers unaffected |

**Overall Risk:** **Low** - Most changes are additive or compile-time enforced.

### 5.2 Backward Compatibility

**Passages with defined endpoints:**
- No behavior change - actual_end_ticks equals passage.end_time_ticks
- Endpoint discovery still occurs but uses known value
- Performance: Minimal overhead (one extra field assignment)

**Database migrations:**
- None required - no schema changes
- Optional Phase 5 adds UPDATE to files.duration (backward compatible)

**API compatibility:**
- No REST API changes required
- SSE events add new BufferEvent::EndpointDiscovered (clients ignore unknown events)

**Overall Risk:** **Very Low** - No breaking changes to existing functionality.

### 5.3 Performance Implications

**Decode overhead:**
- Decoder already processes entire file (decode-and-skip strategy)
- New: One additional sample count → ticks conversion (~1μs)
- Impact: Negligible

**Event overhead:**
- New: One BufferEvent::EndpointDiscovered per passage
- ~50 bytes per event, sent once per passage
- Impact: Negligible

**Memory overhead:**
- PassageBuffer: +8 bytes (Option<usize>)
- BufferMetadata: +16 bytes (Option<i64> + Option<usize>)
- QueueEntry: +8 bytes (Option<i64>)
- Per passage: ~32 bytes total
- Impact: Negligible (typical queue has 12 passages = 384 bytes)

**Overall Risk:** **Very Low** - Performance impact unmeasurable.

---

## 6. Testing Requirements

### 6.1 Unit Tests

**File: `wkmp-common/src/timing.rs`**
1. ✅ `test_samples_to_ticks()` - Verify conversion accuracy
2. ✅ `test_samples_to_ticks_roundtrip()` - Verify samples → ticks → samples lossless

**File: `wkmp-ap/src/audio/types.rs`**
3. ✅ `test_passage_buffer_fill_percent_with_total()` - fill_percent() with total_samples set
4. ✅ `test_passage_buffer_fill_percent_without_total()` - fill_percent() with total_samples=None
5. ✅ `test_passage_buffer_is_complete()` - Completion detection with/without total_samples

**File: `wkmp-ap/src/playback/buffer_manager.rs`**
6. ✅ `test_set_discovered_endpoint()` - Metadata update + event emission
7. ✅ `test_get_discovered_endpoint()` - Retrieve cached endpoint

**File: `wkmp-ap/src/playback/queue_manager.rs`**
8. ✅ `test_update_endpoint()` - QueueEntry.cached_end_ticks updated
9. ✅ `test_effective_end_ticks()` - Fallback logic (cached → passage → None)

### 6.2 Integration Tests

**File: `wkmp-ap/tests/endpoint_discovery_test.rs` (new)**

10. ✅ `test_undefined_endpoint_discovery` - End-to-end flow:
    - Enqueue passage with end_time_ticks=None
    - Verify BufferEvent::EndpointDiscovered emitted
    - Verify QueueEntry.cached_end_ticks updated
    - Verify buffer fill % calculated correctly

11. ✅ `test_defined_endpoint_no_discovery` - Passages with defined endpoints:
    - Enqueue passage with end_time_ticks=Some(...)
    - Verify actual_end_ticks equals defined value
    - Verify no unexpected events

12. ✅ `test_crossfade_with_undefined_endpoints` - Crossfade timing:
    - Passage A: end_time=None, Passage B: start_time=Some(...)
    - Verify crossfade starts at correct time after discovery
    - Verify crossfade duration calculated correctly

13. ✅ `test_multiple_undefined_endpoints` - Queue with 5 passages, all undefined:
    - Verify all endpoints discovered independently
    - Verify correct order of event emissions
    - Verify no race conditions

14. ✅ `test_fadeout_with_undefined_endpoint` - Fade-out timing:
    - Passage with fade_out_point=None, end_time=None
    - Verify fade-out applied at discovered endpoint
    - Audio quality: No clicks/pops at fade boundary

### 6.3 Edge Cases

15. ✅ `test_zero_duration_file` - File with 0 samples:
    - actual_end_ticks = start_time_ticks
    - Buffer empty, mark complete immediately

16. ✅ `test_very_long_file` - File > 1 hour:
    - Verify no overflow in samples_to_ticks() (uses i128 intermediate)

17. ✅ `test_endpoint_discovery_race` - Concurrent updates:
    - Enqueue passage, start playback immediately
    - Mixer requests endpoint before discovery complete
    - Verify graceful fallback (wait or skip crossfade)

18. ✅ `test_queue_removal_before_discovery` - Passage removed from queue:
    - Decode discovers endpoint, emits event
    - QueueManager ignores event (entry not found)
    - Verify no panic

### 6.4 Manual Testing

19. ✅ **Real Audio Files:**
    - MP3, FLAC, OGG, M4A with undefined end_time
    - Verify playback completes correctly
    - Verify crossfades smooth

20. ✅ **Developer UI:**
    - Buffer chain monitor shows correct fill %
    - Passage duration displayed after discovery
    - No "Unknown" or "NaN" values

21. ✅ **Stress Test:**
    - Enqueue 50 passages with undefined endpoints
    - Verify all discovered within 30 seconds
    - Monitor memory usage (no leaks)

### 6.5 Coverage Targets

- **Unit tests:** 95% coverage of new code
- **Integration tests:** 90% coverage of event flow paths
- **Edge cases:** 100% coverage of None/Some branches

---

## 7. Dependencies on Other Phases

### 7.1 Blockers

This implementation depends on:
- ✅ **Tick-based timing system** (`wkmp-common::timing`)
  - Status: Implemented (TICK_RATE, ticks_to_samples, seconds_to_ticks)
  - Required for: samples_to_ticks() conversion

- ✅ **BufferManager event system** (`BufferEvent` enum)
  - Status: Implemented (register/state/exhausted events)
  - Required for: EndpointDiscovered event

- ✅ **QueueManager structure** (`QueueEntry`, queue storage)
  - Status: Implemented
  - Required for: cached_end_ticks field

### 7.2 Parallel Work

Can be implemented in parallel with:
- Decoder pause/resume mechanism (separate concern)
- Ring buffer refactoring (PassageBuffer is abstracted)
- Mixer drain refactoring (endpoint discovery is independent)

### 7.3 Future Enhancements Enabled

After this implementation completes:
- **Dynamic queue visualization** - Real-time duration updates in UI
- **Intelligent prefetch** - Calculate decode priority based on actual durations
- **Playlist validation** - Warn about missing file metadata before playback

---

## 8. Open Questions for Review

1. **Caching Strategy (Phase 5):**
   - Should we cache discovered durations in `files` table immediately?
   - Or defer caching until file has been played N times?
   - Recommendation: Defer to Phase 5 (optional optimization)

2. **Event Emission Timing:**
   - Should EndpointDiscovered emit before or after first chunk appended?
   - Current: After decode completes, before chunk append
   - Alternative: After first chunk (faster feedback to UI)
   - Recommendation: Keep current (more reliable)

3. **Crossfade Recalculation:**
   - Should crossfade timing be recalculated immediately when endpoint discovered?
   - Or only recalculate when passage becomes position 0/1?
   - Current: Recalculate only for position 0/1
   - Recommendation: Keep current (avoids unnecessary work)

4. **Error Handling:**
   - What if decoder discovers endpoint but QueueEntry already removed?
   - Current: QueueManager.update_endpoint() returns Err, logged as warning
   - Alternative: Silently ignore (not an error condition)
   - Recommendation: Keep current (aids debugging)

5. **Buffer Completion Detection:**
   - Should PassageBuffer.is_complete() be used by mixer for completion detection?
   - Or keep existing is_exhausted() logic?
   - Recommendation: Use both - is_complete() validates correctness, is_exhausted() triggers event

---

## 9. Glossary

- **Endpoint:** The end boundary of a passage (end_time), may be undefined (None)
- **Discovery:** Process of determining actual file duration by decoding until EOF
- **Cached Endpoint:** QueueEntry.cached_end_ticks - discovered or defined endpoint
- **Effective Endpoint:** Result of cached_end_ticks.or(passage.end_time_ticks) - fallback chain
- **Actual End Ticks:** Decoder's discovered file duration in ticks (always populated)
- **Total Samples:** Exact interleaved stereo sample count (set when endpoint discovered)

---

## 10. References

**Specifications:**
- [SPEC002 Crossfade Design](../SPEC002-crossfade.md) - [XFD-IMPL-020] Crossfade timing algorithm
- [SPEC016 Decoder Buffer Design](../SPEC016-decoder_buffer_design.md) - [DBD-DEC-070], [DBD-DEC-080], [DBD-BUF-060]
- [SPEC017 Sample Rate Conversion](../SPEC017-sample_rate_conversion.md) - [SRC-CONV-030] samples_to_ticks
- [REQ002 Entity Definitions](../REQ002-entity_definitions.md) - [ENT-MP-035] Audio file as passage

**Gap Analysis:**
- [IMPL-ANALYSIS-002-gap-analysis.json](IMPL-ANALYSIS-002-gap-analysis.json)

**Related Plans:**
- [decoder_pause_resume_design.md](decoder_pause_resume_design.md) - Separate concern
- [ring_buffer_refactoring_plan.md](ring_buffer_refactoring_plan.md) - Compatible with endpoint discovery

---

## 11. Approval & Sign-Off

**Reviewed By:** [Pending]
**Approved By:** [Pending]
**Approval Date:** [Pending]

**Next Steps:**
1. Technical review by lead engineer
2. Approve implementation plan
3. Begin Phase 1 implementation
4. Incremental testing after each phase
5. Final integration test before merge

---

**Document Status:** Ready for Review
**Estimated Implementation Time:** 17-24 hours (Phases 1-4), +2-3 hours (Phase 5 optional)
**Priority:** High - Blocks crossfade accuracy for passages with undefined endpoints
**Target Completion:** TBD after approval

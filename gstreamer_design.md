# GStreamer Pipeline Architecture

**🔧 TIER 3 - IMPLEMENTATION SPECIFICATION**

Technical design for wkmp-ap audio playback engine using GStreamer. See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Architecture](architecture.md) | [Crossfade Design](crossfade.md) | [API Design](api_design.md)

---

## Overview

This document specifies the GStreamer pipeline architecture for the wkmp-ap (Audio Player) microservice, including dual pipeline structure for seamless crossfading, element selection, state management, and buffer handling.

**Key Design Principles:**
- **Dual Pipeline Architecture**: Two independent pipelines for current and next passages
- **Seamless Crossfading**: Audio mixer enables smooth transitions without gaps
- **Format Agnostic**: Automatic decoder selection via decodebin
- **Volume Control**: Per-pipeline and master volume control
- **State Isolation**: Each pipeline can be prepared, played, paused independently

---

## 1. Pipeline Architecture Overview

**[GST-ARCH-010]** wkmp-ap uses **two independent GStreamer pipelines** (Pipeline A and Pipeline B) to enable seamless crossfading between passages.

**[GST-ARCH-020]** Both pipelines feed into a single **audiomixer** element, which combines their outputs with per-stream volume control for crossfade effects.

### 1.1. High-Level Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         wkmp-ap Process                              │
│                                                                       │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ Pipeline A (Currently Playing)                                │   │
│  │                                                                │   │
│  │  ┌─────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐   │   │
│  │  │filesrc  │──→│decodebin │──→│audioconv.│──→│audiores. │──┐│   │
│  │  │         │   │(auto)    │   │          │   │          │  ││   │
│  │  └─────────┘   └──────────┘   └──────────┘   └──────────┘  ││   │
│  └────────────────────────────────────────────────────────────┼┘   │
│                                                                 │    │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ Pipeline B (Pre-loaded / Next)                               │   │
│  │                                                                │   │
│  │  ┌─────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐   │   │
│  │  │filesrc  │──→│decodebin │──→│audioconv.│──→│audiores. │──┐│   │
│  │  │         │   │(auto)    │   │          │   │          │  ││   │
│  │  └─────────┘   └──────────┘   └──────────┘   └──────────┘  ││   │
│  └────────────────────────────────────────────────────────────┼┘   │
│                                                                 │    │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ Mixer and Output Pipeline                                    │   │
│  │                                                              ┌┴─┐ │
│  │  ┌───────────┐   ┌────────┐   ┌──────────┐   ┌──────────┐ │  │ │
│  │  │audiomixer │──→│ volume │──→│audioconv.│──→│audiosink │←┘  │ │
│  │  │  (2 pads) │   │        │   │          │   │(auto)    │    │ │
│  │  └───────────┘   └────────┘   └──────────┘   └──────────┘    │ │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

**[GST-ARCH-030]** Pipeline roles:
- **Pipeline A / Pipeline B**: Decode and prepare audio streams
- **Audiomixer**: Combines both streams with independent volume control per input
- **Master volume**: Global volume control after mixing
- **Audiosink**: Platform-specific audio output (PulseAudio, ALSA, CoreAudio, WASAPI)

---

## 2. Pipeline Element Selection

### 2.1. Source Elements

**[GST-ELEM-010]** File source: **filesrc**
```rust
let filesrc = gst::ElementFactory::make("filesrc")
    .property("location", "/path/to/audio/file.mp3")
    .build()?;
```

**Properties:**
- `location`: File path (string)
- Reads audio file from disk
- Supports all local file formats

**[GST-ELEM-011]** Alternative for future network streaming: **souphttpsrc** or **uridecodebin**

### 2.2. Decoder Elements

**[GST-ELEM-020]** Automatic decoder: **decodebin**
```rust
let decodebin = gst::ElementFactory::make("decodebin")
    .build()?;
```

**Properties:**
- Automatically selects appropriate decoder based on file format
- Emits `pad-added` signal when audio pad becomes available
- Handles: MP3, FLAC, Ogg Vorbis, Opus, AAC, ALAC, WavPack, WAV, AIFF, APE, etc.

**[GST-ELEM-021]** Decoder selection handled by GStreamer plugin system:
- **MP3**: mpg123audiodec, mad
- **FLAC**: flacdec
- **Ogg Vorbis**: vorbisdec
- **Opus**: opusdec
- **AAC**: faad
- **ALAC**: alacdec
- **WavPack**: wavpackdec

> See [Deployment - GStreamer Bundling](deployment.md#12a1-gstreamer-bundling-audio-player) for required plugins.

### 2.3. Format Conversion Elements

**[GST-ELEM-030]** Audio format converter: **audioconvert**
```rust
let audioconvert = gst::ElementFactory::make("audioconvert")
    .build()?;
```

**Purpose:**
- Converts between different audio formats (int16, int32, float32, etc.)
- Handles channel layout conversions (mono, stereo, 5.1, etc.)
- Required for compatibility between decoder output and audiomixer input

**[GST-ELEM-040]** Audio resampler: **audioresample**
```rust
let audioresample = gst::ElementFactory::make("audioresample")
    .build()?;
```

**Purpose:**
- Converts sample rates (44.1kHz, 48kHz, 96kHz, etc.)
- Required when files have different sample rates
- Uses high-quality resampling algorithms

### 2.4. Mixing Element

**[GST-ELEM-050]** Audio mixer: **audiomixer**
```rust
let audiomixer = gst::ElementFactory::make("audiomixer")
    .property("latency", 10_000_000u64)  // 10ms latency
    .build()?;
```

**Properties:**
- `latency`: Buffering latency in nanoseconds (default: 10ms)
- Mixes multiple audio streams into one output
- Per-pad volume control for crossfading

**Sink Pads:**
- Dynamically created when source elements connect
- Each pad has independent volume property (0.0 to 1.0)

**[GST-ELEM-051]** Per-pad volume control:
```rust
let sink_pad = audiomixer.request_pad_simple("sink_%u")?;
sink_pad.set_property("volume", 1.0f64);  // Full volume
sink_pad.set_property("volume", 0.0f64);  // Silent
```

### 2.5. Volume Control Element

**[GST-ELEM-060]** Master volume: **volume**
```rust
let volume = gst::ElementFactory::make("volume")
    .property("volume", 0.75f64)  // 75% volume
    .build()?;
```

**Properties:**
- `volume`: Volume multiplier (0.0 to 10.0, typical 0.0 to 1.0)
- `mute`: Boolean mute flag
- Applied after mixing (affects both pipelines)

### 2.6. Output Elements

**[GST-ELEM-070]** Automatic audio sink: **autoaudiosink**
```rust
let audiosink = gst::ElementFactory::make("autoaudiosink")
    .build()?;
```

**Purpose:**
- Automatically selects appropriate audio backend for platform
- Linux: pulsesink (PulseAudio) or alsasink (ALSA)
- macOS: osxaudiosink (CoreAudio)
- Windows: wasapisink (WASAPI)

**[GST-ELEM-071]** Manual sink selection (if user configures specific device):
```rust
// Example: Force PulseAudio with specific device
let audiosink = gst::ElementFactory::make("pulsesink")
    .property("device", "alsa_output.pci-0000_00_1f.3.analog-stereo")
    .build()?;
```

---

## 3. Pipeline Construction

### 3.1. Pipeline A/B Structure

**[GST-PIPE-010]** Each playback pipeline (A and B) consists of:

```rust
struct PlaybackPipeline {
    pipeline: gst::Pipeline,
    filesrc: gst::Element,
    decodebin: gst::Element,
    audioconvert: gst::Element,
    audioresample: gst::Element,
    mixer_sink_pad: Option<gst::Pad>,  // Connected to audiomixer
    current_file: Option<PathBuf>,
    current_position: Option<ClockTime>,
}
```

**[GST-PIPE-020]** Pipeline construction:
```rust
fn create_playback_pipeline(name: &str) -> Result<PlaybackPipeline> {
    let pipeline = gst::Pipeline::with_name(name);

    let filesrc = gst::ElementFactory::make("filesrc").build()?;
    let decodebin = gst::ElementFactory::make("decodebin").build()?;
    let audioconvert = gst::ElementFactory::make("audioconvert").build()?;
    let audioresample = gst::ElementFactory::make("audioresample").build()?;

    pipeline.add_many(&[&filesrc, &decodebin, &audioconvert, &audioresample])?;

    // Link filesrc → decodebin (static connection)
    filesrc.link(&decodebin)?;

    // Link decodebin → audioconvert → audioresample (dynamic, via pad-added signal)
    let audioconvert_clone = audioconvert.clone();
    let audioresample_clone = audioresample.clone();
    decodebin.connect_pad_added(move |_, src_pad| {
        let sink_pad = audioconvert_clone.static_pad("sink").unwrap();
        if sink_pad.is_linked() {
            return;  // Already linked
        }
        src_pad.link(&sink_pad).expect("Failed to link decodebin to audioconvert");
    });

    audioconvert.link(&audioresample)?;

    Ok(PlaybackPipeline {
        pipeline,
        filesrc,
        decodebin,
        audioconvert,
        audioresample,
        mixer_sink_pad: None,
        current_file: None,
        current_position: None,
    })
}
```

### 3.2. Mixer Pipeline Structure

**[GST-PIPE-030]** The mixer pipeline combines outputs from Pipeline A and B:

```rust
struct MixerPipeline {
    pipeline: gst::Pipeline,
    audiomixer: gst::Element,
    volume: gst::Element,
    audioconvert: gst::Element,
    audiosink: gst::Element,
}
```

**[GST-PIPE-040]** Mixer pipeline construction:
```rust
fn create_mixer_pipeline() -> Result<MixerPipeline> {
    let pipeline = gst::Pipeline::with_name("mixer");

    let audiomixer = gst::ElementFactory::make("audiomixer")
        .property("latency", 10_000_000u64)
        .build()?;
    let volume = gst::ElementFactory::make("volume")
        .property("volume", 1.0f64)
        .build()?;
    let audioconvert = gst::ElementFactory::make("audioconvert").build()?;
    let audiosink = gst::ElementFactory::make("autoaudiosink").build()?;

    pipeline.add_many(&[&audiomixer, &volume, &audioconvert, &audiosink])?;
    audiomixer.link(&volume)?;
    volume.link(&audioconvert)?;
    audioconvert.link(&audiosink)?;

    Ok(MixerPipeline {
        pipeline,
        audiomixer,
        volume,
        audioconvert,
        audiosink,
    })
}
```

### 3.3. Connecting Pipelines to Mixer

**[GST-PIPE-050]** Connect playback pipeline output to mixer input:

```rust
fn connect_to_mixer(
    playback: &mut PlaybackPipeline,
    mixer: &MixerPipeline
) -> Result<()> {
    // Request a sink pad from the mixer
    let mixer_sink_pad = mixer.audiomixer.request_pad_simple("sink_%u")?;

    // Get the src pad from audioresample (last element in playback pipeline)
    let src_pad = playback.audioresample.static_pad("src")
        .ok_or("No src pad on audioresample")?;

    // Link playback pipeline to mixer
    src_pad.link(&mixer_sink_pad)?;

    // Store reference to mixer pad for volume control
    playback.mixer_sink_pad = Some(mixer_sink_pad);

    Ok(())
}
```

---

## 4. State Management

### 4.1. GStreamer State Model

**[GST-STATE-010]** GStreamer pipeline states (from GStreamer documentation):

| State | Description |
|-------|-------------|
| NULL | Default state, no resources allocated |
| READY | Resources allocated, ready to PAUSE |
| PAUSED | Pipeline paused, ready to PLAY, pre-rolled |
| PLAYING | Pipeline actively playing |

**[GST-STATE-020]** State transitions:

```
NULL ──→ READY ──→ PAUSED ──→ PLAYING
  ←──      ←──       ←──
```

### 4.2. wkmp-ap Pipeline State Management

**[GST-STATE-030]** wkmp-ap pipeline state model:

| wkmp-ap State | Pipeline A State | Pipeline B State | Description |
|---------------|------------------|------------------|-------------|
| **Idle** | NULL | NULL | No passage loaded |
| **Loading** | NULL→PAUSED | NULL | Loading first passage |
| **Playing** | PLAYING | NULL or PAUSED | Passage A playing, B may be pre-loaded |
| **Crossfading** | PLAYING (fade out) | PLAYING (fade in) | Both playing, transitioning |
| **Paused** | PAUSED | PAUSED or NULL | Playback paused |

**[GST-STATE-040]** State transition diagram:

```
        ┌──────────────────────────────────────┐
        │              Idle                    │
        │  Pipeline A: NULL                    │
        │  Pipeline B: NULL                    │
        └──────────────┬───────────────────────┘
                       │ load_passage()
                       ▼
        ┌──────────────────────────────────────┐
        │            Loading                   │
        │  Pipeline A: NULL → PAUSED           │
        │  Pipeline B: NULL                    │
        └──────────────┬───────────────────────┘
                       │ play()
                       ▼
        ┌──────────────────────────────────────┐
        │            Playing                   │◄────┐
        │  Pipeline A: PLAYING                 │     │
        │  Pipeline B: NULL                    │     │
        └──────────────┬───────────────────────┘     │
                       │ lead_out_reached() +        │
                       │ preload_next()              │
                       ▼                              │
        ┌──────────────────────────────────────┐     │
        │      Playing (Pre-loaded)            │     │
        │  Pipeline A: PLAYING                 │     │
        │  Pipeline B: PAUSED (ready)          │     │
        └──────────────┬───────────────────────┘     │
                       │ crossfade_start()           │
                       ▼                              │
        ┌──────────────────────────────────────┐     │
        │         Crossfading                  │     │
        │  Pipeline A: PLAYING (vol↓)          │     │
        │  Pipeline B: PLAYING (vol↑)          │     │
        └──────────────┬───────────────────────┘     │
                       │ crossfade_complete()        │
                       ▼                              │
        ┌──────────────────────────────────────┐     │
        │      Playing (Swapped)               │     │
        │  Pipeline A: NULL (stopped)          │     │
        │  Pipeline B: PLAYING                 │     │
        └──────────────┬───────────────────────┘     │
                       │ swap(A ↔ B)                 │
                       └─────────────────────────────┘

        Pause: PLAYING → PAUSED (reversible)
        Stop: any state → NULL
```

### 4.3. State Transition Implementation

**[GST-STATE-050]** Loading a passage:
```rust
fn load_passage(&mut self, file_path: &Path) -> Result<()> {
    // Set NULL state first (cleanup)
    self.pipeline_a.pipeline.set_state(gst::State::Null)?;

    // Set file location
    self.pipeline_a.filesrc.set_property("location", file_path.to_str().unwrap());

    // Transition to PAUSED (pre-roll, ready to play)
    self.pipeline_a.pipeline.set_state(gst::State::Paused)?;

    // Wait for state change to complete
    self.pipeline_a.pipeline.state(gst::ClockTime::NONE).0?;

    self.pipeline_a.current_file = Some(file_path.to_path_buf());
    Ok(())
}
```

**[GST-STATE-060]** Starting playback:
```rust
fn play(&mut self) -> Result<()> {
    self.pipeline_a.pipeline.set_state(gst::State::Playing)?;
    self.mixer.pipeline.set_state(gst::State::Playing)?;
    Ok(())
}
```

**[GST-STATE-070]** Pausing playback:
```rust
fn pause(&mut self) -> Result<()> {
    self.pipeline_a.pipeline.set_state(gst::State::Paused)?;
    if let Some(ref pipeline_b) = self.pipeline_b {
        pipeline_b.pipeline.set_state(gst::State::Paused)?;
    }
    Ok(())
}
```

---

## 5. Crossfade Implementation

### 5.1. Pre-loading Strategy

**[GST-XFADE-010]** Pre-load trigger calculation:

```rust
fn calculate_preload_trigger(passage: &Passage) -> ClockTime {
    // Pre-load when: lead_out_point - 5 seconds
    let preload_advance = ClockTime::from_seconds(5);
    let lead_out = passage.lead_out_point_ms.unwrap_or(
        passage.end_time_ms - passage.global_crossfade_ms
    );

    ClockTime::from_mseconds(lead_out).saturating_sub(preload_advance)
}
```

**[GST-XFADE-020]** Pre-load next passage into idle pipeline:

```rust
fn preload_next_passage(&mut self, next_passage: &Passage) -> Result<()> {
    // Determine which pipeline is idle (not currently playing)
    let idle_pipeline = if self.pipeline_a_active {
        &mut self.pipeline_b
    } else {
        &mut self.pipeline_a
    };

    // Load file into idle pipeline
    idle_pipeline.filesrc.set_property("location", &next_passage.file_path);

    // Transition to PAUSED (pre-rolled, ready to play)
    idle_pipeline.pipeline.set_state(gst::State::Paused)?;
    idle_pipeline.pipeline.state(gst::ClockTime::NONE).0?;

    // Set initial volume to 0 (will fade in during crossfade)
    if let Some(ref pad) = idle_pipeline.mixer_sink_pad {
        pad.set_property("volume", 0.0f64);
    }

    idle_pipeline.current_file = Some(next_passage.file_path.clone());
    Ok(())
}
```

### 5.2. Crossfade Execution

**[GST-XFADE-030]** Crossfade timing calculation:

```rust
fn calculate_crossfade_params(
    passage_a: &Passage,
    passage_b: &Passage
) -> CrossfadeParams {
    let remaining_a = passage_a.end_time_ms - passage_a.lead_out_point_ms;
    let lead_in_b = passage_b.lead_in_point_ms;

    let duration_ms = std::cmp::min(remaining_a, lead_in_b);

    CrossfadeParams {
        start_time_a: passage_a.end_time_ms - duration_ms,
        start_time_b: 0,
        duration_ms,
        fade_out_curve: passage_a.fade_out_curve,
        fade_in_curve: passage_b.fade_in_curve,
    }
}
```

**[GST-XFADE-040]** Start crossfade:

```rust
fn start_crossfade(&mut self, params: CrossfadeParams) -> Result<()> {
    // Start Pipeline B playback
    self.pipeline_b.pipeline.set_state(gst::State::Playing)?;

    // Launch crossfade controller thread
    let fade_out_pad = self.pipeline_a.mixer_sink_pad.clone().unwrap();
    let fade_in_pad = self.pipeline_b.mixer_sink_pad.clone().unwrap();

    std::thread::spawn(move || {
        crossfade_controller(
            fade_out_pad,
            fade_in_pad,
            params.duration_ms,
            params.fade_out_curve,
            params.fade_in_curve,
        );
    });

    self.crossfading = true;
    Ok(())
}
```

**[GST-XFADE-050]** Crossfade controller (volume fade thread):

```rust
fn crossfade_controller(
    fade_out_pad: gst::Pad,
    fade_in_pad: gst::Pad,
    duration_ms: u64,
    fade_out_curve: FadeCurve,
    fade_in_curve: FadeCurve,
) {
    let start_time = std::time::Instant::now();
    let duration = std::time::Duration::from_millis(duration_ms);
    let update_interval = std::time::Duration::from_millis(50);  // 50ms updates (20Hz)

    loop {
        let elapsed = start_time.elapsed();
        if elapsed >= duration {
            // Crossfade complete
            fade_out_pad.set_property("volume", 0.0f64);
            fade_in_pad.set_property("volume", 1.0f64);
            break;
        }

        let t = elapsed.as_secs_f64() / duration.as_secs_f64();  // 0.0 to 1.0

        // Calculate fade out volume (1.0 → 0.0)
        let vol_out = apply_fade_curve(1.0 - t, fade_out_curve);
        fade_out_pad.set_property("volume", vol_out);

        // Calculate fade in volume (0.0 → 1.0)
        let vol_in = apply_fade_curve(t, fade_in_curve);
        fade_in_pad.set_property("volume", vol_in);

        std::thread::sleep(update_interval);
    }
}
```

**[GST-XFADE-060]** Fade curve implementation:

```rust
fn apply_fade_curve(t: f64, curve: FadeCurve) -> f64 {
    match curve {
        FadeCurve::Linear => t,
        FadeCurve::Exponential => t.powi(2),
        FadeCurve::Logarithmic => (1.0 - t).powi(2),
        FadeCurve::Cosine => 0.5 * (1.0 - (std::f64::consts::PI * t).cos()),
    }
}
```

> See [Crossfade Design](crossfade.md#implementation-algorithm) for fade curve mathematical specifications.

### 5.3. Crossfade Completion

**[GST-XFADE-070]** After crossfade completes:

```rust
fn on_crossfade_complete(&mut self) -> Result<()> {
    // Stop and cleanup the old pipeline (Pipeline A)
    self.pipeline_a.pipeline.set_state(gst::State::Null)?;
    self.pipeline_a.current_file = None;

    // Swap pipeline roles (B becomes primary)
    std::mem::swap(&mut self.pipeline_a, &mut self.pipeline_b);
    self.pipeline_a_active = true;

    self.crossfading = false;
    Ok(())
}
```

---

## 6. Position and Duration Queries

### 6.1. Querying Playback Position

**[GST-QUERY-010]** Get current playback position:

```rust
fn get_position(&self) -> Option<ClockTime> {
    self.pipeline_a.pipeline.query_position::<ClockTime>()
}
```

**[GST-QUERY-020]** Position query for SSE updates (called every 5 seconds):

```rust
fn emit_playback_progress(&self) -> Result<()> {
    if let Some(position) = self.get_position() {
        if let Some(duration) = self.get_duration() {
            let event = PlaybackProgress {
                passage_id: self.current_passage_id.clone(),
                position_ms: position.mseconds(),
                duration_ms: duration.mseconds(),
                timestamp: Utc::now(),
            };
            self.event_tx.send(Event::PlaybackProgress(event))?;
        }
    }
    Ok(())
}
```

### 6.2. Querying Duration

**[GST-QUERY-030]** Get passage duration:

```rust
fn get_duration(&self) -> Option<ClockTime> {
    self.pipeline_a.pipeline.query_duration::<ClockTime>()
}
```

**[GST-QUERY-040]** Duration is available after pipeline reaches PAUSED state (pre-rolled).

---

## 7. Seeking

### 7.1. Seek Implementation

**[GST-SEEK-010]** Seek to position (used for resume-from-position):

```rust
fn seek(&mut self, position_ms: u64) -> Result<()> {
    let position = ClockTime::from_mseconds(position_ms);

    self.pipeline_a.pipeline.seek_simple(
        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
        position,
    )?;

    Ok(())
}
```

**[GST-SEEK-020]** Seek flags:
- `FLUSH`: Clear buffers before seeking (prevents audio glitches)
- `KEY_UNIT`: Seek to nearest keyframe (faster, slight position inaccuracy acceptable for audio)

**[GST-SEEK-030]** Seeking during playback:
- If in PLAYING state: Seek completes smoothly, playback continues from new position
- If in PAUSED state: Seek updates position, remains paused

---

## 8. Error Handling

### 8.1. GStreamer Bus Messages

**[GST-ERROR-010]** Monitor pipeline bus for errors:

```rust
fn setup_bus_watch(&mut self) -> Result<()> {
    let bus = self.pipeline_a.pipeline.bus().unwrap();

    let event_tx = self.event_tx.clone();
    bus.add_watch(move |_, msg| {
        match msg.view() {
            gst::MessageView::Error(err) => {
                eprintln!("GStreamer Error: {} - Debug: {:?}",
                    err.error(), err.debug());

                // Emit error event
                let _ = event_tx.send(Event::PlaybackError {
                    message: err.error().to_string(),
                });

                glib::Continue(false)  // Stop watch
            }
            gst::MessageView::Eos(_) => {
                // End of stream - passage complete
                let _ = event_tx.send(Event::PassageCompleted {
                    completed: true
                });
                glib::Continue(true)
            }
            gst::MessageView::StateChanged(state) => {
                // Log state changes
                if let Some(src) = msg.src() {
                    if src.is::<gst::Pipeline>() {
                        println!("State: {:?} -> {:?}",
                            state.old(), state.current());
                    }
                }
                glib::Continue(true)
            }
            _ => glib::Continue(true),
        }
    })?;

    Ok(())
}
```

### 8.2. Error Recovery

**[GST-ERROR-020]** File not found error:
```rust
// GStreamer will emit error message
// wkmp-ap catches error, logs, emits PassageCompleted(completed=false)
// Advances to next passage per ARCH-STARTUP-020
```

**[GST-ERROR-030]** Unsupported format error:
```rust
// decodebin fails to find suitable decoder
// Same recovery as file not found
```

**[GST-ERROR-040]** Audio device error:
```rust
// audiosink fails to open device
// wkmp-ap logs error, attempts fallback to different sink
// If all sinks fail, enters degraded mode (queue advances but no audio)
```

---

## 9. Volume Control

### 9.1. Master Volume

**[GST-VOL-010]** Set master volume (affects all playback):

```rust
fn set_volume(&mut self, volume: f64) -> Result<()> {
    // volume: 0.0 to 1.0
    self.mixer.volume.set_property("volume", volume);
    Ok(())
}
```

### 9.2. Per-Pipeline Volume (for crossfading)

**[GST-VOL-020]** Set pipeline-specific volume:

```rust
fn set_pipeline_volume(&mut self, pipeline: Pipeline, volume: f64) -> Result<()> {
    let pad = match pipeline {
        Pipeline::A => &self.pipeline_a.mixer_sink_pad,
        Pipeline::B => &self.pipeline_b.mixer_sink_pad,
    };

    if let Some(ref pad) = pad {
        pad.set_property("volume", volume);
    }

    Ok(())
}
```

---

## 10. Audio Device Enumeration

Audio device enumeration allows users to select which audio output device (speakers, headphones, HDMI audio, etc.) wkmp-ap uses for playback. This is exposed via the `GET /audio/devices` and `POST /audio/device` API endpoints.

### 10.1. Device Identifier Format

**[GST-DEV-005]** Device identifiers follow a platform-specific format:

| Platform | Format | Example | Notes |
|----------|--------|---------|-------|
| **Linux (PulseAudio)** | `pulse-sink-N` | `pulse-sink-0`, `pulse-sink-1` | N is the PulseAudio sink index |
| **Linux (ALSA)** | `alsa-hw-N-M` | `alsa-hw-0-0`, `alsa-hw-1-0` | N = card number, M = device number |
| **macOS (CoreAudio)** | `coreaudio-UID` | `coreaudio-AppleHDAEngineOutput:1B,0,1,0:0` | UID is the CoreAudio device UID |
| **Windows (WASAPI)** | `wasapi-{GUID}` | `wasapi-{0.0.0.00000000}.{guid}` | GUID is the WASAPI endpoint identifier |
| **Special** | `default` | `default` | System default output (delegates to OS) |

**[GST-DEV-006]** Device identifier construction from GStreamer DeviceMonitor:

```rust
fn construct_device_id(device: &gst::Device) -> String {
    let props = device.properties();

    // Try to get platform-specific device identifier
    if let Some(device_path) = props.and_then(|p| p.get::<String>("device.path").ok()) {
        // PulseAudio: device.path = sink index
        return format!("pulse-sink-{}", device_path);
    } else if let Some(device_name) = props.and_then(|p| p.get::<String>("device.name").ok()) {
        // ALSA: device.name = "hw:0,0"
        if device_name.starts_with("hw:") {
            let parts: Vec<&str> = device_name[3..].split(',').collect();
            if parts.len() == 2 {
                return format!("alsa-hw-{}-{}", parts[0], parts[1]);
            }
        }
        // CoreAudio: device.name = device UID
        if device_name.contains(':') {
            return format!("coreaudio-{}", device_name);
        }
        // WASAPI: device.name = device GUID
        if device_name.starts_with('{') {
            return format!("wasapi-{}", device_name);
        }
    }

    // Fallback: use raw device name
    device.display_name().to_string()
}
```

### 10.2. Enumerating Available Devices

**[GST-DEV-010]** List audio output devices using GStreamer DeviceMonitor:

```rust
fn enumerate_audio_devices() -> Result<Vec<AudioDevice>> {
    let device_monitor = gst::DeviceMonitor::new();

    // Filter for audio output (sink) devices only
    device_monitor.add_filter(Some("Audio/Sink"), None);

    device_monitor.start()?;
    let devices = device_monitor.devices();
    device_monitor.stop();

    let mut audio_devices = vec![
        // Always include "default" as first option
        AudioDevice {
            id: "default".to_string(),
            name: "System Default".to_string(),
            is_default: true,
        }
    ];

    for dev in devices.iter() {
        let id = construct_device_id(dev);
        let name = dev.display_name().to_string();

        // Check if this device is currently the system default
        let is_default = dev.properties()
            .and_then(|props| props.get::<bool>("is-default").ok())
            .unwrap_or(false);

        audio_devices.push(AudioDevice {
            id,
            name,
            is_default,
        });
    }

    Ok(audio_devices)
}

struct AudioDevice {
    id: String,       // Platform-specific identifier
    name: String,     // Human-readable name
    is_default: bool, // True if this is the system default device
}
```

**[GST-DEV-015]** API Response Format:

The `enumerate_audio_devices()` function result is serialized to JSON for the `GET /audio/devices` endpoint:

```json
{
  "devices": [
    {"id": "default", "name": "System Default", "default": true},
    {"id": "pulse-sink-0", "name": "Built-in Audio Analog Stereo", "default": false},
    {"id": "pulse-sink-1", "name": "HDMI Audio Output", "default": false}
  ]
}
```

### 10.3. Selecting Audio Device

**[GST-DEV-020]** Set audio output device (called from `POST /audio/device` endpoint):

```rust
fn set_audio_device(&mut self, device_id: &str) -> Result<()> {
    // Determine which GStreamer sink element and property to use
    let (sink_name, property_name, property_value) = if device_id == "default" {
        // Use autoaudiosink (delegates to system default)
        ("autoaudiosink", None, None)
    } else if device_id.starts_with("pulse-sink-") {
        // PulseAudio sink with specific device
        let sink_index = device_id.strip_prefix("pulse-sink-").unwrap();
        ("pulsesink", Some("device"), Some(sink_index.to_string()))
    } else if device_id.starts_with("alsa-hw-") {
        // ALSA sink with specific hardware device
        let hw_path = device_id.strip_prefix("alsa-hw-").unwrap().replace('-', ",");
        ("alsasink", Some("device"), Some(format!("hw:{}", hw_path)))
    } else if device_id.starts_with("coreaudio-") {
        // CoreAudio sink with specific device UID
        let device_uid = device_id.strip_prefix("coreaudio-").unwrap();
        ("osxaudiosink", Some("device"), Some(device_uid.to_string()))
    } else if device_id.starts_with("wasapi-") {
        // WASAPI sink with specific device GUID
        let device_guid = device_id.strip_prefix("wasapi-").unwrap();
        ("wasapisink", Some("device"), Some(device_guid.to_string()))
    } else {
        return Err(Error::DeviceNotFound(device_id.to_string()));
    };

    // Create new audiosink with specified device
    let mut builder = gst::ElementFactory::make(sink_name);
    if let (Some(prop_name), Some(prop_value)) = (property_name, property_value) {
        builder = builder.property(prop_name, prop_value);
    }
    let new_audiosink = builder.build()?;

    // Replace audiosink in mixer pipeline
    // This requires stopping playback, unlinking old sink, linking new sink, restarting
    self.replace_audio_sink(new_audiosink)?;

    // Persist device selection in settings table
    self.db.set_setting("audio_output_device", device_id)?;

    Ok(())
}
```

**[GST-DEV-030]** Replacing audiosink element during playback:

```rust
fn replace_audio_sink(&mut self, new_sink: gst::Element) -> Result<()> {
    // 1. Pause both pipelines
    self.pipeline_a.pipeline.set_state(gst::State::Paused)?;
    self.pipeline_b.pipeline.set_state(gst::State::Paused)?;
    self.mixer_pipeline.pipeline.set_state(gst::State::Paused)?;

    // 2. Unlink old audiosink from audiomixer
    let old_sink = self.mixer_pipeline.audiosink.clone();
    self.mixer_pipeline.volume.unlink(&old_sink)?;

    // 3. Remove old sink from pipeline
    self.mixer_pipeline.pipeline.remove(&old_sink)?;
    old_sink.set_state(gst::State::Null)?;

    // 4. Add and link new sink
    self.mixer_pipeline.pipeline.add(&new_sink)?;
    self.mixer_pipeline.volume.link(&new_sink)?;

    // 5. Sync state with pipeline
    new_sink.sync_state_with_parent()?;

    // 6. Update mixer_pipeline reference
    self.mixer_pipeline.audiosink = new_sink;

    // 7. Resume playback if we were playing before
    if self.state == PlaybackState::Playing {
        self.mixer_pipeline.pipeline.set_state(gst::State::Playing)?;
        self.pipeline_a.pipeline.set_state(gst::State::Playing)?;
    }

    Ok(())
}
```

### 10.4. Platform-Specific Considerations

**[GST-DEV-040]** Linux (PulseAudio):
- Most common on modern Linux distributions
- Supports hot-plugging (headphones, USB audio)
- Device IDs may change across reboots (use `default` for stability)
- Sink names queryable via: `pactl list short sinks`

**[GST-DEV-050]** Linux (ALSA):
- Direct hardware access (lower latency, less flexibility)
- Device numbers are stable across reboots
- Requires exclusive access (other apps cannot use device simultaneously)
- Use for embedded systems or when PulseAudio unavailable

**[GST-DEV-060]** macOS (CoreAudio):
- CoreAudio is the only audio system on macOS
- Device UIDs are stable and persistent
- Automatic device switching when headphones plugged in (if using `default`)

**[GST-DEV-070]** Windows (WASAPI):
- Modern Windows audio API (Windows Vista+)
- Device GUIDs are persistent but complex
- WASAPI supports both exclusive and shared mode (GStreamer uses shared)

**[GST-DEV-080]** Automatic device detection:
- Use `autoaudiosink` (device_id = `default`) for automatic device selection
- Recommended for most users (system handles device changes automatically)
- Explicit device selection useful for:
  - Multi-zone audio setups
  - Recording/streaming applications
  - Testing specific audio hardware

---

## 11. Buffer Management

### 11.1. Buffer Sizing

**[GST-BUF-010]** Default buffer configuration:

- **filesrc buffer-size**: 64 KB (GStreamer default, no override needed)
- **audiomixer latency**: 10ms (configurable, see GST-ELEM-050)
- **audiosink buffer-time**: Platform default (typically 200ms)

**[GST-BUF-020]** Memory usage:
- Each pre-loaded pipeline: ~2-5 MB (compressed audio in buffer)
- After decoding: ~10-20 MB per pipeline (uncompressed audio)
- Typical total: 20-40 MB for dual pipeline with pre-loading

### 11.2. Buffering State

**[GST-BUF-030]** Buffering messages (for future network streaming):

```rust
gst::MessageView::Buffering(buffering) => {
    let percent = buffering.percent();
    if percent < 100 {
        // Pause playback until buffering complete
        self.pipeline_a.pipeline.set_state(gst::State::Paused)?;
    } else {
        // Resume playback
        self.pipeline_a.pipeline.set_state(gst::State::Playing)?;
    }
}
```

---

## 12. Threading Model

### 12.1. GStreamer Thread Model

**[GST-THREAD-010]** GStreamer internal threading:
- Each pipeline runs in its own thread pool (managed by GStreamer)
- Bus messages delivered to main thread via GLib main loop
- No manual thread management required for basic playback

**[GST-THREAD-020]** wkmp-ap threading:
- **Main thread**: HTTP server, API handlers
- **GLib main loop thread**: GStreamer bus message processing
- **Crossfade thread**: Volume fade controller (spawned during crossfade)
- **Position update thread**: Periodic position queries for SSE (every 5 seconds)

### 12.2. Thread Safety

**[GST-THREAD-030]** GStreamer element thread safety:
- Element property changes: Thread-safe (can be called from any thread)
- Pipeline state changes: Thread-safe
- Pad linking: Must be done on main thread or in pad-added signal handler

---

## 13. Platform-Specific Considerations

### 13.1. Linux

**[GST-PLAT-010]** Audio backends:
- **PulseAudio** (preferred): pulsesink
- **ALSA** (fallback): alsasink
- **JACK** (pro audio): jackaudiosink

**[GST-PLAT-011]** Latency tuning:
```rust
// For low-latency audio (JACK, pro audio setups)
let audiosink = gst::ElementFactory::make("jackaudiosink")
    .property("buffer-time", 50_000i64)  // 50ms
    .build()?;
```

### 13.2. macOS

**[GST-PLAT-020]** Audio backend:
- **CoreAudio**: osxaudiosink

**[GST-PLAT-021]** Device selection on macOS:
```rust
let audiosink = gst::ElementFactory::make("osxaudiosink")
    .property("device", 0)  // Device index
    .build()?;
```

### 13.3. Windows

**[GST-PLAT-030]** Audio backend:
- **WASAPI** (preferred): wasapisink
- **DirectSound** (fallback): directsoundsink

**[GST-PLAT-031]** Exclusive mode (optional, for audiophile setups):
```rust
let audiosink = gst::ElementFactory::make("wasapisink")
    .property("low-latency", true)
    .build()?;
```

---

## 14. Performance Optimization

### 14.1. Pre-loading Optimization

**[GST-OPT-010]** Pre-load timing:
- Trigger: lead_out_point - 5 seconds
- Ensures file is decoded and ready before crossfade starts
- 5 seconds provides buffer for slow disk I/O or network delays

**[GST-OPT-020]** Avoid pre-loading too early:
- Don't pre-load until lead_out - 5s reached
- Reduces memory usage (only one active pipeline most of the time)
- Prevents unnecessary disk I/O

### 14.2. Crossfade Update Rate

**[GST-OPT-030]** Volume update frequency:
- 50ms intervals (20 Hz update rate)
- Balance between smooth fade and CPU usage
- Higher frequencies (10ms) unnecessary (human perception limit ~20ms)

### 14.3. Position Query Throttling

**[GST-OPT-040]** Position query rate:
- Query position every 5 seconds (configurable via `playback_progress_interval_ms`)
- GStreamer position queries are relatively expensive (lock pipeline)
- 5 seconds adequate for UI progress bar updates

---

## 15. Future Enhancements

### 15.1. Gapless Playback Without Crossfade

**[GST-FUTURE-010]** For passages with crossfade_duration = 0:
- Use single pipeline with queue element
- Append next file to queue (gapless playback)
- Eliminates need for dual pipeline in this case
- Lower memory usage, simpler state management

### 15.2. ReplayGain / Volume Normalization

**[GST-FUTURE-020]** Add rgvolume element for automatic volume leveling:
```rust
let rgvolume = gst::ElementFactory::make("rgvolume")
    .property("album-mode", true)
    .build()?;
```

Insert between audioresample and mixer.

### 15.3. Audio Visualization

**[GST-FUTURE-030]** Add visualization element for spectrum analyzer:
```rust
let spectrum = gst::ElementFactory::make("spectrum")
    .property("bands", 128)
    .property("interval", 50_000_000u64)  // 50ms updates
    .build()?;
```

Tap audio stream before mixer, emit spectrum data via SSE.

---

End of document - GStreamer Pipeline Architecture

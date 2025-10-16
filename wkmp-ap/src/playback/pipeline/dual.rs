//! Dual pipeline structure for crossfading between passages

use anyhow::Result;
use gstreamer as gst;
use gstreamer::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Which pipeline is currently active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePipeline {
    A,
    B,
}

impl ActivePipeline {
    /// Get the other pipeline
    pub fn other(&self) -> Self {
        match self {
            ActivePipeline::A => ActivePipeline::B,
            ActivePipeline::B => ActivePipeline::A,
        }
    }
}

/// Dual pipeline manager for seamless crossfading
///
/// Implements the dual pipeline architecture from gstreamer_design.md:
/// - Pipeline A and Pipeline B for independent playback
/// - Audiomixer element to combine both streams
/// - Per-pipeline volume control for crossfade effects
/// - Master volume control after mixing
pub struct DualPipeline {
    /// Main GStreamer pipeline containing both sub-pipelines and mixer
    main_pipeline: gst::Pipeline,

    /// Pipeline A components
    pipeline_a: PipelineComponents,

    /// Pipeline B components
    pipeline_b: PipelineComponents,

    /// Audiomixer element
    audiomixer: gst::Element,

    /// Master volume element (after mixer)
    master_volume: gst::Element,

    /// Audio output sink
    audio_sink: gst::Element,

    /// Which pipeline is currently active
    active: Arc<RwLock<ActivePipeline>>,

    /// Master volume level (0.0 to 1.0)
    master_volume_level: Arc<RwLock<f64>>,
}

/// Components for a single playback pipeline
struct PipelineComponents {
    /// File source element
    filesrc: gst::Element,

    /// Decoder bin
    decodebin: gst::Element,

    /// Audio converter
    audioconvert: gst::Element,

    /// Audio resampler
    audioresample: gst::Element,

    /// Volume control for this pipeline
    volume: gst::Element,

    /// Current volume level (0.0 to 1.0)
    volume_level: Arc<RwLock<f64>>,

    /// Bin containing all elements
    bin: gst::Bin,
}

impl DualPipeline {
    /// Create a new dual pipeline manager
    pub fn new() -> Result<Self> {
        info!("Creating dual pipeline with audiomixer");

        // Create main pipeline
        let main_pipeline = gst::Pipeline::new();

        // Create audiomixer
        let audiomixer = gst::ElementFactory::make("audiomixer")
            .name("mixer")
            .build()?;

        // Create master volume control
        let master_volume = gst::ElementFactory::make("volume")
            .name("master_volume")
            .property("volume", 0.75f64) // Default 75% volume
            .build()?;

        // Create audio sink
        let audio_sink = gst::ElementFactory::make("autoaudiosink")
            .name("sink")
            .build()?;

        // Create pipeline A
        let pipeline_a = Self::create_pipeline_components("a")?;

        // Create pipeline B
        let pipeline_b = Self::create_pipeline_components("b")?;

        // Add all elements to main pipeline
        main_pipeline.add_many([
            pipeline_a.bin.upcast_ref(),
            pipeline_b.bin.upcast_ref(),
            &audiomixer,
            &master_volume,
            &audio_sink,
        ])?;

        // Link mixer -> master_volume -> audio_sink
        gst::Element::link_many([&audiomixer, &master_volume, &audio_sink])?;

        // Link pipeline A bin to mixer
        let pipeline_a_pad = pipeline_a.bin.static_pad("src")
            .ok_or_else(|| anyhow::anyhow!("No src pad on pipeline A bin"))?;
        let mixer_sink_a = audiomixer.request_pad_simple("sink_%u")
            .ok_or_else(|| anyhow::anyhow!("Failed to get mixer sink pad for pipeline A"))?;
        pipeline_a_pad.link(&mixer_sink_a)?;
        debug!("Linked pipeline A to mixer");

        // Link pipeline B bin to mixer
        let pipeline_b_pad = pipeline_b.bin.static_pad("src")
            .ok_or_else(|| anyhow::anyhow!("No src pad on pipeline B bin"))?;
        let mixer_sink_b = audiomixer.request_pad_simple("sink_%u")
            .ok_or_else(|| anyhow::anyhow!("Failed to get mixer sink pad for pipeline B"))?;
        pipeline_b_pad.link(&mixer_sink_b)?;
        debug!("Linked pipeline B to mixer");

        Ok(Self {
            main_pipeline,
            pipeline_a,
            pipeline_b,
            audiomixer,
            master_volume,
            audio_sink,
            active: Arc::new(RwLock::new(ActivePipeline::A)),
            master_volume_level: Arc::new(RwLock::new(0.75)),
        })
    }

    /// Create components for a single pipeline
    fn create_pipeline_components(name: &str) -> Result<PipelineComponents> {
        // Create bin to hold all elements
        let bin = gst::Bin::new();

        // Create elements
        let filesrc = gst::ElementFactory::make("filesrc")
            .name(&format!("filesrc_{}", name))
            .build()?;

        let decodebin = gst::ElementFactory::make("decodebin")
            .name(&format!("decoder_{}", name))
            .build()?;

        let audioconvert = gst::ElementFactory::make("audioconvert")
            .name(&format!("converter_{}", name))
            .build()?;

        let audioresample = gst::ElementFactory::make("audioresample")
            .name(&format!("resampler_{}", name))
            .build()?;

        let volume = gst::ElementFactory::make("volume")
            .name(&format!("volume_{}", name))
            .property("volume", 1.0f64) // Start at full volume
            .build()?;

        // Add elements to bin
        bin.add_many([
            &filesrc,
            &decodebin,
            &audioconvert,
            &audioresample,
            &volume,
        ])?;

        // Link static elements: filesrc -> decodebin
        filesrc.link(&decodebin)?;

        // Link: audioconvert -> audioresample -> volume
        gst::Element::link_many([&audioconvert, &audioresample, &volume])?;

        // Setup dynamic pad linking for decodebin
        let audioconvert_clone = audioconvert.clone();
        let name_owned = name.to_string(); // Convert to owned String for closure
        decodebin.connect_pad_added(move |_element, pad| {
            let pad_caps = pad.current_caps().unwrap();
            let pad_struct = pad_caps.structure(0).unwrap();
            let pad_name = pad_struct.name();

            if pad_name.starts_with("audio/") {
                let sink_pad = audioconvert_clone.static_pad("sink").unwrap();
                if !sink_pad.is_linked() {
                    if let Err(e) = pad.link(&sink_pad) {
                        warn!("Failed to link decodebin to audioconvert: {}", e);
                    } else {
                        debug!("Linked decodebin to audioconvert for {}", name_owned);
                    }
                }
            }
        });

        // Create ghost pad for output
        let volume_src_pad = volume.static_pad("src")
            .ok_or_else(|| anyhow::anyhow!("No src pad on volume"))?;
        let ghost_pad = gst::GhostPad::with_target(&volume_src_pad)?;
        ghost_pad.set_active(true)?;
        bin.add_pad(&ghost_pad)?;

        Ok(PipelineComponents {
            filesrc,
            decodebin,
            audioconvert,
            audioresample,
            volume,
            volume_level: Arc::new(RwLock::new(1.0)),
            bin,
        })
    }

    /// Load a file into the specified pipeline
    pub async fn load_file(&self, pipeline: ActivePipeline, file_path: &PathBuf) -> Result<()> {
        info!("Loading file {:?} into pipeline {:?}", file_path, pipeline);

        let components = match pipeline {
            ActivePipeline::A => &self.pipeline_a,
            ActivePipeline::B => &self.pipeline_b,
        };

        // Set file location
        components.filesrc.set_property("location", file_path.to_str().unwrap());

        Ok(())
    }

    /// Start playback
    pub fn play(&self) -> Result<()> {
        self.main_pipeline.set_state(gst::State::Playing)?;
        info!("Dual pipeline playing");
        Ok(())
    }

    /// Pause playback
    pub fn pause(&self) -> Result<()> {
        self.main_pipeline.set_state(gst::State::Paused)?;
        info!("Dual pipeline paused");
        Ok(())
    }

    /// Stop playback
    pub fn stop(&self) -> Result<()> {
        self.main_pipeline.set_state(gst::State::Null)?;
        info!("Dual pipeline stopped");
        Ok(())
    }

    /// Get the active pipeline
    pub async fn active(&self) -> ActivePipeline {
        *self.active.read().await
    }

    /// Switch active pipeline
    pub async fn switch_active(&self) {
        let mut active = self.active.write().await;
        *active = active.other();
        info!("Switched active pipeline to {:?}", *active);
    }

    /// Set volume for a specific pipeline (for crossfading)
    pub async fn set_pipeline_volume(&self, pipeline: ActivePipeline, volume: f64) -> Result<()> {
        let components = match pipeline {
            ActivePipeline::A => &self.pipeline_a,
            ActivePipeline::B => &self.pipeline_b,
        };

        let clamped_volume = volume.clamp(0.0, 1.0);
        components.volume.set_property("volume", clamped_volume);
        *components.volume_level.write().await = clamped_volume;

        debug!("Set pipeline {:?} volume to {:.2}", pipeline, clamped_volume);
        Ok(())
    }

    /// Set master volume (0.0 to 1.0)
    pub async fn set_master_volume(&self, volume: f64) -> Result<()> {
        let clamped_volume = volume.clamp(0.0, 1.0);
        self.master_volume.set_property("volume", clamped_volume);
        *self.master_volume_level.write().await = clamped_volume;

        info!("Set master volume to {:.2}", clamped_volume);
        Ok(())
    }

    /// Get current playback position in milliseconds for active pipeline
    pub async fn position_ms(&self) -> Option<i64> {
        self.main_pipeline
            .query_position::<gst::ClockTime>()
            .map(|pos| pos.mseconds() as i64)
    }

    /// Get duration in milliseconds for active pipeline
    pub async fn duration_ms(&self) -> Option<i64> {
        self.main_pipeline
            .query_duration::<gst::ClockTime>()
            .map(|dur| dur.mseconds() as i64)
    }

    /// Seek to position in milliseconds
    pub fn seek_to(&self, position_ms: i64) -> Result<()> {
        let position = gst::ClockTime::from_mseconds(position_ms as u64);

        self.main_pipeline.seek_simple(
            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
            position,
        )?;

        debug!("Seeked to {}ms", position_ms);
        Ok(())
    }

    /// Check if end of stream reached
    pub fn is_eos(&self) -> bool {
        let bus = self.main_pipeline.bus().unwrap();

        while let Some(msg) = bus.pop() {
            use gst::MessageView;
            match msg.view() {
                MessageView::Eos(_) => return true,
                _ => {}
            }
        }

        false
    }

    /// Start crossfade from current pipeline to the other
    ///
    /// This initiates a volume crossfade over the specified duration.
    /// The caller is responsible for calling update_crossfade() periodically
    /// to update the volume levels.
    pub async fn start_crossfade(&self, duration_ms: u64) -> Result<()> {
        let active = self.active().await;
        info!("Starting crossfade from {:?} to {:?} over {}ms",
              active, active.other(), duration_ms);

        // The actual crossfade is performed by periodically calling update_crossfade()
        // from the monitoring task
        Ok(())
    }

    /// Update crossfade progress
    ///
    /// # Arguments
    /// * `progress` - Crossfade progress from 0.0 (start) to 1.0 (complete)
    /// * `curve` - Fade curve to use ("linear", "exponential", or "cosine")
    ///
    /// This should be called periodically during a crossfade to update volumes
    pub async fn update_crossfade(&self, progress: f64, curve: &str) -> Result<()> {
        let progress = progress.clamp(0.0, 1.0);
        let active = self.active().await;

        // Calculate fade-out volume for current pipeline
        let fade_out_volume = Self::calculate_fade_curve(1.0 - progress, curve);

        // Calculate fade-in volume for next pipeline
        let fade_in_volume = Self::calculate_fade_curve(progress, curve);

        // Apply volumes
        self.set_pipeline_volume(active, fade_out_volume).await?;
        self.set_pipeline_volume(active.other(), fade_in_volume).await?;

        debug!("Crossfade progress: {:.2}%, {:?} vol: {:.2}, {:?} vol: {:.2}",
               progress * 100.0, active, fade_out_volume, active.other(), fade_in_volume);

        Ok(())
    }

    /// Calculate volume based on fade curve
    ///
    /// # Arguments
    /// * `t` - Progress from 0.0 to 1.0
    /// * `curve` - "linear", "exponential", or "cosine"
    fn calculate_fade_curve(t: f64, curve: &str) -> f64 {
        let t = t.clamp(0.0, 1.0);

        match curve {
            "exponential" => {
                // Exponential curve: slow start, fast finish
                t * t
            }
            "logarithmic" => {
                // Logarithmic curve: fast start, slow finish
                1.0 - (1.0 - t) * (1.0 - t)
            }
            "cosine" => {
                // Cosine S-curve: smooth acceleration and deceleration
                0.5 * (1.0 - f64::cos(t * std::f64::consts::PI))
            }
            _ => {
                // Linear (default)
                t
            }
        }
    }
}

impl Drop for DualPipeline {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

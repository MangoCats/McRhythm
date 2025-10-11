//! Single GStreamer pipeline for basic playback

use anyhow::{anyhow, Result};
use gstreamer::prelude::*;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Single GStreamer pipeline for audio playback
pub struct SinglePipeline {
    pipeline: gstreamer::Pipeline,
    audio_sink: gstreamer::Element,
}

impl SinglePipeline {
    /// Create a new pipeline for the given file
    pub fn new(file_path: &PathBuf, start_time_ms: i64, end_time_ms: i64) -> Result<Self> {
        info!("Creating pipeline for: {}", file_path.display());

        // Create pipeline
        let pipeline = gstreamer::Pipeline::new();

        // Create elements
        let src = gstreamer::ElementFactory::make("filesrc")
            .name("source")
            .build()?;

        let decodebin = gstreamer::ElementFactory::make("decodebin")
            .name("decoder")
            .build()?;

        let audioconvert = gstreamer::ElementFactory::make("audioconvert")
            .name("converter")
            .build()?;

        let audioresample = gstreamer::ElementFactory::make("audioresample")
            .name("resampler")
            .build()?;

        let volume = gstreamer::ElementFactory::make("volume")
            .name("volume")
            .property("volume", 1.0f64)
            .build()?;

        let audio_sink = gstreamer::ElementFactory::make("autoaudiosink")
            .name("sink")
            .build()?;

        // Set file location
        src.set_property("location", file_path.to_str().ok_or_else(|| anyhow!("Invalid path"))?);

        // Add elements to pipeline
        pipeline.add_many(&[&src, &decodebin, &audioconvert, &audioresample, &volume, &audio_sink])?;

        // Link source to decodebin
        src.link(&decodebin)?;

        // Link the rest (audioconvert -> audioresample -> volume -> sink)
        gstreamer::Element::link_many(&[&audioconvert, &audioresample, &volume, &audio_sink])?;

        // Handle dynamic pad from decodebin
        let audioconvert_clone = audioconvert.clone();
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
                        debug!("Linked decodebin audio pad to audioconvert");
                    }
                }
            }
        });

        Ok(Self {
            pipeline,
            audio_sink,
        })
    }

    /// Start playback
    pub fn play(&self) -> Result<()> {
        debug!("Starting pipeline playback");
        self.pipeline.set_state(gstreamer::State::Playing)?;
        Ok(())
    }

    /// Pause playback
    pub fn pause(&self) -> Result<()> {
        debug!("Pausing pipeline");
        self.pipeline.set_state(gstreamer::State::Paused)?;
        Ok(())
    }

    /// Stop playback
    pub fn stop(&self) -> Result<()> {
        debug!("Stopping pipeline");
        self.pipeline.set_state(gstreamer::State::Null)?;
        Ok(())
    }

    /// Get current position in milliseconds
    pub fn position_ms(&self) -> Option<i64> {
        self.pipeline
            .query_position::<gstreamer::ClockTime>()
            .map(|pos| pos.mseconds() as i64)
    }

    /// Get duration in milliseconds
    pub fn duration_ms(&self) -> Option<i64> {
        self.pipeline
            .query_duration::<gstreamer::ClockTime>()
            .map(|dur| dur.mseconds() as i64)
    }

    /// Seek to position in milliseconds
    pub fn seek_to(&self, position_ms: i64) -> Result<()> {
        let position = gstreamer::ClockTime::from_mseconds(position_ms as u64);
        self.pipeline.seek_simple(
            gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
            position,
        )?;
        Ok(())
    }

    /// Set volume (0.0 to 1.0)
    pub fn set_volume(&self, volume: f64) -> Result<()> {
        let volume_element = self.pipeline
            .by_name("volume")
            .ok_or_else(|| anyhow!("Volume element not found"))?;

        volume_element.set_property("volume", volume.clamp(0.0, 1.0));
        Ok(())
    }

    /// Check if pipeline is in playing state
    pub fn is_playing(&self) -> bool {
        let (_, current, _) = self.pipeline.state(gstreamer::ClockTime::from_mseconds(0));
        current == gstreamer::State::Playing
    }

    /// Check if end of stream
    pub fn is_eos(&self) -> bool {
        // Check for EOS message on bus without blocking
        if let Some(bus) = self.pipeline.bus() {
            if let Some(msg) = bus.peek() {
                if msg.type_() == gstreamer::MessageType::Eos {
                    return true;
                }
            }
        }
        false
    }

    /// Get the underlying pipeline
    pub fn pipeline(&self) -> &gstreamer::Pipeline {
        &self.pipeline
    }
}

impl Drop for SinglePipeline {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

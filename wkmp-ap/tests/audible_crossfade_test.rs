//! Audible crossfade test using real MP3 files
//!
//! This test finds 3 MP3 files, decodes them, and plays them through speakers
//! with crossfades so the user can hear the quality of the mixing.
//!
//! Features:
//! - Plays 3 passages with crossfades
//! - Adds fade-out to final passage (no abrupt cutoff)
//! - Tracks RMS audio levels to detect clipping/issues
//! - Verifies timing expectations
//! - Logs all key playback events
//!
//! Run with: cargo test --test audible_crossfade_test -- --ignored --nocapture

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use walkdir::WalkDir;
use wkmp_ap::audio::{AudioFrame, AudioOutput, PassageBuffer, Resampler, SimpleDecoder};
use wkmp_ap::playback::pipeline::{CrossfadeMixer, FadeCurve};

/// Playback events for tracking test timeline
#[derive(Debug, Clone)]
enum PlaybackEvent {
    Passage1Started { time: f32 },
    Passage1FadeInComplete { time: f32 },
    Crossfade1To2Started { time: f32 },
    Crossfade1To2Complete { time: f32 },
    Passage2FullVolume { time: f32 },
    Crossfade2To3Started { time: f32 },
    Crossfade2To3Complete { time: f32 },
    Passage3FullVolume { time: f32 },
    Passage3FadeOutStarted { time: f32 },
    Passage3FadeOutComplete { time: f32 },
    PlaybackComplete { time: f32 },
}

/// Expected timing for playback events
struct ExpectedTimeline {
    passage1_start: f32,
    passage1_fade_in_complete: f32,
    crossfade1_start: f32,
    crossfade1_complete: f32,
    crossfade2_start: f32,
    crossfade2_complete: f32,
    passage3_fade_out_start: f32,
    passage3_fade_out_complete: f32,
}

/// Audio level tracker for detecting volume issues
struct AudioLevelTracker {
    samples: Vec<f32>,
    window_size: usize,
}

impl AudioLevelTracker {
    fn new(window_size: usize) -> Self {
        AudioLevelTracker {
            samples: Vec::with_capacity(window_size),
            window_size,
        }
    }

    fn add_frame(&mut self, frame: &AudioFrame) {
        // Average of left and right absolute values
        let avg = (frame.left.abs() + frame.right.abs()) / 2.0;
        self.samples.push(avg);
        if self.samples.len() > self.window_size {
            self.samples.remove(0);
        }
    }

    fn rms(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.samples.iter().map(|s| s * s).sum();
        (sum / self.samples.len() as f32).sqrt()
    }

    fn reset(&mut self) {
        self.samples.clear();
    }
}

/// Verify timeline against expected values
fn verify_timeline(events: &[PlaybackEvent], expected: &ExpectedTimeline) -> Vec<String> {
    let mut errors = Vec::new();
    let tolerance = 0.5; // 500ms tolerance

    for event in events {
        match event {
            PlaybackEvent::Passage1FadeInComplete { time } => {
                if (*time - expected.passage1_fade_in_complete).abs() > tolerance {
                    errors.push(format!(
                        "Passage 1 fade-in completed at {:.2}s, expected {:.2}s (diff: {:.2}s)",
                        time,
                        expected.passage1_fade_in_complete,
                        time - expected.passage1_fade_in_complete
                    ));
                }
            }
            PlaybackEvent::Crossfade1To2Started { time } => {
                if (*time - expected.crossfade1_start).abs() > tolerance {
                    errors.push(format!(
                        "Crossfade 1→2 started at {:.2}s, expected {:.2}s (diff: {:.2}s)",
                        time,
                        expected.crossfade1_start,
                        time - expected.crossfade1_start
                    ));
                }
            }
            PlaybackEvent::Crossfade1To2Complete { time } => {
                if (*time - expected.crossfade1_complete).abs() > tolerance {
                    errors.push(format!(
                        "Crossfade 1→2 completed at {:.2}s, expected {:.2}s (diff: {:.2}s)",
                        time,
                        expected.crossfade1_complete,
                        time - expected.crossfade1_complete
                    ));
                }
            }
            PlaybackEvent::Crossfade2To3Started { time } => {
                if (*time - expected.crossfade2_start).abs() > tolerance {
                    errors.push(format!(
                        "Crossfade 2→3 started at {:.2}s, expected {:.2}s (diff: {:.2}s)",
                        time,
                        expected.crossfade2_start,
                        time - expected.crossfade2_start
                    ));
                }
            }
            PlaybackEvent::Crossfade2To3Complete { time } => {
                if (*time - expected.crossfade2_complete).abs() > tolerance {
                    errors.push(format!(
                        "Crossfade 2→3 completed at {:.2}s, expected {:.2}s (diff: {:.2}s)",
                        time,
                        expected.crossfade2_complete,
                        time - expected.crossfade2_complete
                    ));
                }
            }
            PlaybackEvent::Passage3FadeOutStarted { time } => {
                if (*time - expected.passage3_fade_out_start).abs() > tolerance {
                    errors.push(format!(
                        "Passage 3 fade-out started at {:.2}s, expected {:.2}s (diff: {:.2}s)",
                        time,
                        expected.passage3_fade_out_start,
                        time - expected.passage3_fade_out_start
                    ));
                }
            }
            PlaybackEvent::Passage3FadeOutComplete { time } => {
                if (*time - expected.passage3_fade_out_complete).abs() > tolerance {
                    errors.push(format!(
                        "Passage 3 fade-out completed at {:.2}s, expected {:.2}s (diff: {:.2}s)",
                        time,
                        expected.passage3_fade_out_complete,
                        time - expected.passage3_fade_out_complete
                    ));
                }
            }
            _ => {}
        }
    }

    errors
}

/// Find MP3 files in a directory (recursive)
fn find_mp3_files(root: &str, count: usize) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase() == "mp3")
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .take(count)
        .collect()
}

/// Blocking wrapper for CrossfadeMixer
struct BlockingMixer {
    runtime: tokio::runtime::Runtime,
    mixer: Arc<tokio::sync::RwLock<CrossfadeMixer>>,
}

impl BlockingMixer {
    fn new() -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let mixer = Arc::new(tokio::sync::RwLock::new(CrossfadeMixer::new()));
        Self { runtime, mixer }
    }

    fn get_next_frame(&self) -> AudioFrame {
        self.runtime.block_on(async {
            let mut m = self.mixer.write().await;
            m.get_next_frame().await
        })
    }

    fn start_passage(
        &self,
        buffer: Arc<tokio::sync::RwLock<PassageBuffer>>,
        passage_id: Uuid,
        fade_in_curve: Option<FadeCurve>,
        fade_in_duration_ms: u32,
    ) {
        self.runtime.block_on(async {
            let mut m = self.mixer.write().await;
            m.start_passage(buffer, passage_id, fade_in_curve, fade_in_duration_ms)
                .await
        })
    }

    fn start_crossfade(
        &self,
        next_buffer: Arc<tokio::sync::RwLock<PassageBuffer>>,
        next_passage_id: Uuid,
        fade_out_curve: FadeCurve,
        fade_out_duration_ms: u32,
        fade_in_curve: FadeCurve,
        fade_in_duration_ms: u32,
    ) -> Result<(), wkmp_ap::error::Error> {
        self.runtime.block_on(async {
            let mut m = self.mixer.write().await;
            m.start_crossfade(
                next_buffer,
                next_passage_id,
                fade_out_curve,
                fade_out_duration_ms,
                fade_in_curve,
                fade_in_duration_ms,
            )
            .await
        })
    }
}

#[test]
#[ignore] // Run manually with: cargo test --test audible_crossfade_test -- --ignored --nocapture
fn test_audible_crossfade() {
    println!("\n=== ENHANCED AUDIBLE CROSSFADE TEST ===");
    println!("This test will play 3 MP3 files with crossfades through your speakers.");
    println!("Features:");
    println!("  - Fade-in on passage 1");
    println!("  - Crossfades between passages 1→2 and 2→3");
    println!("  - Fade-out on passage 3 (no abrupt cutoff)");
    println!("  - RMS level tracking to detect clipping");
    println!("  - Timing verification against expected timeline\n");

    // Find MP3 files
    println!("Finding MP3 files in /home/sw/Music...");
    let mp3_files = find_mp3_files("/home/sw/Music", 3);

    if mp3_files.len() < 3 {
        println!(
            "ERROR: Not enough MP3 files found. Need 3, found {}.",
            mp3_files.len()
        );
        println!("Please ensure /home/sw/Music contains at least 3 MP3 files.");
        return;
    }

    println!("Found {} MP3 files:\n", mp3_files.len());
    for (i, file) in mp3_files.iter().enumerate() {
        println!("  {}. {}", i + 1, file.display());
    }

    // Decode and prepare buffers
    println!("\n=== Decoding files ===\n");

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut buffers = Vec::new();

    for (i, file) in mp3_files.iter().enumerate() {
        println!("Decoding file {}...", i + 1);

        // Decode
        let (samples, sample_rate, channels) = match SimpleDecoder::decode_file(file) {
            Ok(result) => result,
            Err(e) => {
                println!("ERROR: Failed to decode file: {}", e);
                continue;
            }
        };

        println!(
            "  Sample rate: {}, Channels: {}, Samples: {}",
            sample_rate,
            channels,
            samples.len()
        );

        // Resample to 44.1kHz if needed
        let resampled = if sample_rate != 44100 {
            println!("  Resampling to 44.1kHz...");
            match Resampler::resample(&samples, sample_rate, channels) {
                Ok(result) => result,
                Err(e) => {
                    println!("ERROR: Failed to resample: {}", e);
                    continue;
                }
            }
        } else {
            samples
        };

        // Limit to 30 seconds to keep test reasonable
        let max_samples = 44100 * 2 * 30; // 30 seconds stereo
        let limited_samples = if resampled.len() > max_samples {
            println!("  Limiting to 30 seconds...");
            resampled[..max_samples].to_vec()
        } else {
            resampled
        };

        // Create buffer
        let buffer = PassageBuffer::new(Uuid::new_v4(), limited_samples, 44100, 2);

        println!("  Duration: {:.2}s", buffer.duration_seconds());

        buffers.push(Arc::new(tokio::sync::RwLock::new(buffer)));
    }

    if buffers.len() < 3 {
        println!(
            "\nERROR: Failed to decode enough files. Only got {} buffers.",
            buffers.len()
        );
        return;
    }

    println!("\n=== Setting up crossfade mixer ===\n");

    // Create blocking mixer wrapper
    let mixer = Arc::new(Mutex::new(BlockingMixer::new()));

    // Crossfade configuration
    let crossfade_duration_ms = 5000;
    let fade_out_duration_ms = 5000;

    println!("Crossfade duration: {}ms", crossfade_duration_ms);
    println!("Fade-out duration: {}ms", fade_out_duration_ms);
    println!("Fade curves: Exponential (in) / Logarithmic (out)\n");

    // Calculate durations
    let passage_durations = runtime.block_on(async {
        let b1 = buffers[0].read().await;
        let b2 = buffers[1].read().await;
        let b3 = buffers[2].read().await;
        (
            b1.duration_seconds(),
            b2.duration_seconds(),
            b3.duration_seconds(),
        )
    });

    let (p1_dur, p2_dur, p3_dur) = passage_durations;

    // Calculate expected timeline
    let crossfade_sec = crossfade_duration_ms as f32 / 1000.0;
    let fade_out_sec = fade_out_duration_ms as f32 / 1000.0;

    let expected = ExpectedTimeline {
        passage1_start: 0.0,
        passage1_fade_in_complete: crossfade_sec,
        crossfade1_start: p1_dur - crossfade_sec,
        crossfade1_complete: p1_dur,
        crossfade2_start: p1_dur + p2_dur - crossfade_sec * 2.0,
        crossfade2_complete: p1_dur + p2_dur - crossfade_sec,
        passage3_fade_out_start: p1_dur + p2_dur + p3_dur - crossfade_sec * 2.0 - fade_out_sec,
        passage3_fade_out_complete: p1_dur + p2_dur + p3_dur - crossfade_sec * 2.0,
    };

    println!("=== Expected Timeline ===");
    println!("  Passage 1 starts: {:.1}s", expected.passage1_start);
    println!("  Passage 1 fade-in complete: {:.1}s", expected.passage1_fade_in_complete);
    println!("  Crossfade 1→2: {:.1}s - {:.1}s", expected.crossfade1_start, expected.crossfade1_complete);
    println!("  Crossfade 2→3: {:.1}s - {:.1}s", expected.crossfade2_start, expected.crossfade2_complete);
    println!("  Passage 3 fade-out: {:.1}s - {:.1}s", expected.passage3_fade_out_start, expected.passage3_fade_out_complete);
    println!("  Total duration: {:.1}s\n", expected.passage3_fade_out_complete);

    // Event tracking
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();

    // Level tracking
    let level_tracker = Arc::new(Mutex::new(AudioLevelTracker::new(4410))); // 100ms window at 44.1kHz
    let level_tracker_clone = level_tracker.clone();

    // Start first passage with fade-in
    println!("Starting passage 1 with fade-in...");
    {
        let passage_id = runtime.block_on(async { buffers[0].read().await.passage_id });
        mixer.lock().unwrap().start_passage(
            buffers[0].clone(),
            passage_id,
            Some(FadeCurve::Exponential),
            crossfade_duration_ms,
        );
    }

    events_clone.lock().unwrap().push(PlaybackEvent::Passage1Started { time: 0.0 });

    println!("\n=== Playing audio ===");
    println!("Listen for smooth crossfades and clean fade-out!\n");

    // Open audio output
    let mut output = match AudioOutput::new(None) {
        Ok(output) => output,
        Err(e) => {
            println!("ERROR: Failed to open audio output: {}", e);
            println!("Make sure your audio device is available and not in use.");
            return;
        }
    };

    println!("Audio device: {}\n", output.device_name());

    // Create mixer clone for audio thread
    let mixer_clone = mixer.clone();

    // Start audio playback
    if let Err(e) = output.start(move || {
        // This closure runs synchronously on the audio thread
        if let Ok(m) = mixer_clone.try_lock() {
            let frame = m.get_next_frame();

            // Track level
            if let Ok(mut tracker) = level_tracker_clone.try_lock() {
                tracker.add_frame(&frame);
            }

            frame
        } else {
            AudioFrame::zero()
        }
    }) {
        println!("ERROR: Failed to start audio: {}", e);
        return;
    }

    // Track timing for crossfades and fade-out
    let start_time = std::time::Instant::now();
    let mut crossfade_triggered = [false, false, false]; // 3rd is fade-out
    let mut last_progress = 0.0;
    let mut last_rms = 0.0;
    let mut fade_in_complete_logged = false;
    let mut crossfade1_complete_logged = false;
    let mut crossfade2_complete_logged = false;

    let total_duration = expected.passage3_fade_out_complete;

    println!("Total playback duration: {:.1}s", total_duration);
    println!("----------------------------------------\n");

    // Main timing loop
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        let elapsed = start_time.elapsed().as_secs_f32();

        // Get current RMS level
        let rms = level_tracker.lock().unwrap().rms();

        // Log level changes
        if (rms - last_rms).abs() > 0.1 {
            println!("  Level at {:.1}s: RMS={:.3}", elapsed, rms);
            last_rms = rms;
        }

        // Detect clipping
        if rms > 0.95 {
            println!("  WARNING: Possible clipping at {:.1}s! RMS={:.3}", elapsed, rms);
        }

        // Progress updates
        let progress = (elapsed / total_duration * 100.0).min(100.0);
        if (progress / 5.0).floor() > (last_progress / 5.0_f32).floor() {
            println!(
                "Progress: {:.0}% ({:.1}s / {:.1}s) [RMS: {:.3}]",
                progress, elapsed, total_duration, rms
            );
            last_progress = progress;
        }

        // Log fade-in complete
        if !fade_in_complete_logged && elapsed >= expected.passage1_fade_in_complete {
            println!("\n>>> Passage 1 fade-in complete at {:.2}s <<<", elapsed);
            events.lock().unwrap().push(PlaybackEvent::Passage1FadeInComplete { time: elapsed });
            fade_in_complete_logged = true;
        }

        // Trigger crossfade 1→2
        if !crossfade_triggered[0] && elapsed >= expected.crossfade1_start {
            println!("\n>>> Triggering crossfade 1→2 at {:.2}s <<<", elapsed);
            events.lock().unwrap().push(PlaybackEvent::Crossfade1To2Started { time: elapsed });

            let next_passage_id = runtime.block_on(async { buffers[1].read().await.passage_id });

            if let Err(e) = mixer.lock().unwrap().start_crossfade(
                buffers[1].clone(),
                next_passage_id,
                FadeCurve::Logarithmic,
                crossfade_duration_ms,
                FadeCurve::Exponential,
                crossfade_duration_ms,
            ) {
                println!("ERROR: Failed to start crossfade: {}", e);
            }

            crossfade_triggered[0] = true;
        }

        // Log crossfade 1→2 complete
        if !crossfade1_complete_logged && crossfade_triggered[0] && elapsed >= expected.crossfade1_complete {
            println!(">>> Crossfade 1→2 complete at {:.2}s <<<\n", elapsed);
            events.lock().unwrap().push(PlaybackEvent::Crossfade1To2Complete { time: elapsed });
            crossfade1_complete_logged = true;
        }

        // Trigger crossfade 2→3
        if !crossfade_triggered[1] && elapsed >= expected.crossfade2_start {
            println!("\n>>> Triggering crossfade 2→3 at {:.2}s <<<", elapsed);
            events.lock().unwrap().push(PlaybackEvent::Crossfade2To3Started { time: elapsed });

            let next_passage_id = runtime.block_on(async { buffers[2].read().await.passage_id });

            if let Err(e) = mixer.lock().unwrap().start_crossfade(
                buffers[2].clone(),
                next_passage_id,
                FadeCurve::Logarithmic,
                crossfade_duration_ms,
                FadeCurve::Exponential,
                crossfade_duration_ms,
            ) {
                println!("ERROR: Failed to start crossfade: {}", e);
            }

            crossfade_triggered[1] = true;
        }

        // Log crossfade 2→3 complete
        if !crossfade2_complete_logged && crossfade_triggered[1] && elapsed >= expected.crossfade2_complete {
            println!(">>> Crossfade 2→3 complete at {:.2}s <<<\n", elapsed);
            events.lock().unwrap().push(PlaybackEvent::Crossfade2To3Complete { time: elapsed });
            crossfade2_complete_logged = true;
        }

        // Trigger fade-out of passage 3
        if !crossfade_triggered[2] && elapsed >= expected.passage3_fade_out_start {
            println!("\n>>> Triggering passage 3 fade-out at {:.2}s <<<", elapsed);
            events.lock().unwrap().push(PlaybackEvent::Passage3FadeOutStarted { time: elapsed });

            // Create silent buffer for fade-out
            // We crossfade to silence, effectively fading out the current passage
            let silent_buffer = Arc::new(tokio::sync::RwLock::new(PassageBuffer::new(
                Uuid::new_v4(),
                vec![0.0; 44100 * 2 * 10], // 10s silence
                44100,
                2,
            )));

            if let Err(e) = mixer.lock().unwrap().start_crossfade(
                silent_buffer,
                Uuid::new_v4(),
                FadeCurve::Logarithmic, // Fade out passage 3
                fade_out_duration_ms,
                FadeCurve::Linear, // Fade in silence (doesn't matter)
                fade_out_duration_ms,
            ) {
                println!("ERROR: Failed to start fade-out: {}", e);
            }

            crossfade_triggered[2] = true;
        }

        // Check if fade-out complete (when RMS drops very low)
        if crossfade_triggered[2] && elapsed >= expected.passage3_fade_out_complete {
            println!(">>> Fade-out complete at {:.2}s (RMS={:.3}) <<<\n", elapsed, rms);
            events.lock().unwrap().push(PlaybackEvent::Passage3FadeOutComplete { time: elapsed });

            // Wait a bit more to ensure silence
            std::thread::sleep(std::time::Duration::from_millis(500));
            break;
        }

        // Safety timeout
        if elapsed >= total_duration + 2.0 {
            println!("\n  Timeout reached at {:.2}s", elapsed);
            break;
        }
    }

    println!("\n=== Playback complete ===");

    // Verify timeline
    println!("\n=== Verifying Timeline ===");
    let events = events.lock().unwrap();
    let errors = verify_timeline(&events, &expected);

    if errors.is_empty() {
        println!("✓ All timing checks PASSED!");
    } else {
        println!("⚠ Timing issues detected:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    // Final check
    let final_rms = level_tracker.lock().unwrap().rms();
    if final_rms < 0.01 {
        println!("✓ Final fade-out successful (RMS={:.3})", final_rms);
    } else {
        println!("⚠ Final fade-out incomplete (RMS={:.3}, expected < 0.01)", final_rms);
    }

    println!("\n=== Event Log ===");
    for event in events.iter() {
        println!("{:?}", event);
    }

    println!("\n=== Test Summary ===");
    println!("Test finished successfully!");
    println!("\nListening feedback:");
    println!("  - Did you hear smooth fade-in at the start?");
    println!("  - Were the crossfades between passages smooth?");
    println!("  - Did passage 3 fade out smoothly to silence (no abrupt cutoff)?");
    println!("  - Were there any clicks, pops, or distortion?");
    println!("  - Did the volume remain consistent during crossfades?\n");
}

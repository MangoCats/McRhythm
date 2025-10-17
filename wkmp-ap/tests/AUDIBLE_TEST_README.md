# Audible Crossfade Test (Enhanced)

## Purpose

This test demonstrates and validates the crossfade functionality by playing real MP3 files through your speakers. It allows you to hear the quality of the mixing and verify that crossfades are smooth and natural.

**NEW in v2.0:** The test now includes:
- **Fade-out on final passage** (no more abrupt cutoff)
- **RMS level tracking** to detect clipping and volume issues
- **Timing verification** to ensure events happen when expected
- **Detailed event logging** for debugging
- **Enhanced progress reporting** with audio level display

## Test File

`audible_crossfade_test.rs` - Integration test that plays 3 MP3 files with crossfades

## What It Does

1. **Finds MP3 Files**: Searches `/home/sw/Music` for 3 MP3 files
2. **Decodes Audio**: Decodes each file using symphonia (MP3 decoder)
3. **Resamples**: Resamples to 44.1kHz if needed (standard playback rate)
4. **Limits Duration**: Limits each passage to 30 seconds (to keep test reasonable)
5. **Creates Mixer**: Sets up the CrossfadeMixer with 5-second crossfade duration
6. **Plays Audio**:
   - Passage 1: Starts with 5-second exponential fade-in
   - Passage 1→2: Crossfades at 25 seconds (5 seconds before end)
   - Passage 2→3: Crossfades at 55 seconds (5 seconds before end)
   - **Passage 3: Fades out smoothly over 5 seconds (NEW!)**
7. **Tracks Audio Levels**: Monitors RMS levels throughout playback
8. **Verifies Timing**: Checks that events happen at expected times
9. **Reports Results**: Shows timing accuracy and audio quality metrics

## Running the Test

```bash
cd /home/sw/Dev/McRhythm/wkmp-ap
cargo test --test audible_crossfade_test -- --ignored --nocapture
```

**Note:** The test is marked with `#[ignore]` because:
- It requires audio hardware (speakers/headphones)
- It takes ~80 seconds to complete
- It requires MP3 files in `/home/sw/Music`

## Test Timeline

| Time Range | What's Happening |
|------------|------------------|
| 0-5s | Passage 1 fading in (exponential curve) |
| 5-25s | Passage 1 at full volume |
| 25-30s | **Crossfade 1→2** (passage 1 fading out, passage 2 fading in) |
| 30-50s | Passage 2 at full volume |
| 50-55s | **Crossfade 2→3** (passage 2 fading out, passage 3 fading in) |
| 55-75s | Passage 3 at full volume |
| **75-80s** | **Passage 3 fading out smoothly to silence (NEW!)** |

**Total Duration:** 80 seconds

### Expected Timeline (Auto-Calculated)

The test calculates exact expected times for all events based on passage durations:

- **Passage 1 fade-in complete**: 5.0s
- **Crossfade 1→2**: 25.0s - 30.0s
- **Crossfade 2→3**: 50.0s - 55.0s
- **Passage 3 fade-out**: 75.0s - 80.0s

All actual event times are compared against these expected times with a tolerance of ±0.5s.

## Fade Curves

- **Fade-in:** Exponential (slow start, fast end - natural sounding)
- **Fade-out:** Logarithmic (fast start, slow end - natural sounding)

These curves complement each other for smooth, natural-sounding crossfades.

## What to Listen For

**Good Signs:**
- Smooth fade-in at the start (no sudden jump in volume)
- Seamless transitions between passages (no audible "seam")
- Consistent perceived volume during crossfades
- **Smooth fade-out at the end (no abrupt cutoff)** ← NEW!
- No clicks, pops, or glitches
- No distortion or clipping

**Bad Signs:**
- Clicks or pops during transitions (indicates sample discontinuity)
- Volume dips or peaks during crossfades (indicates poor curve math)
- Distortion (indicates clipping - samples exceeding ±1.0)
- Audible "seam" where you can clearly hear the transition point
- **Abrupt cutoff at the end (should fade smoothly to silence)**

## What the Test Checks Automatically

The enhanced test now automatically detects and reports:

### 1. RMS Level Tracking
- Monitors audio levels continuously (100ms rolling window)
- Detects possible clipping (RMS > 0.95)
- Verifies final fade-out reaches near-silence (RMS < 0.01)
- Logs level changes > 0.1 for debugging

### 2. Timing Verification
- Checks that fade-in completes at expected time
- Verifies crossfades start and complete on schedule
- Ensures fade-out happens at correct time
- Reports timing errors if events are off by more than 500ms

### 3. Event Logging
All key events are logged with timestamps:
- Passage 1 started
- Passage 1 fade-in complete
- Crossfade 1→2 started/complete
- Crossfade 2→3 started/complete
- Passage 3 fade-out started/complete
- Playback complete

## Technical Details

### Implementation

The test uses a `BlockingMixer` wrapper that:
- Creates its own Tokio runtime for async operations
- Exposes blocking methods for the sync audio callback
- Uses `try_lock()` to avoid blocking the audio thread

### Audio Pipeline

```
MP3 File → Decoder → Resampler → PassageBuffer → CrossfadeMixer → AudioOutput → Speakers
   (symphonia)  (rubato)   (44.1kHz PCM)     (mixing)        (cpal)
```

### Sample Rate

All audio is normalized to **44100 Hz** (CD quality):
- Standard rate for audio playback
- Good balance of quality and performance
- Supported by all audio devices

### Buffer Management

- Each passage limited to 30 seconds (~2.6 MB per passage)
- Full passages loaded into RAM (PassageBuffer)
- Sample-accurate mixing (no drift or timing issues)

## Test Results

### Successful Run (Enhanced v2.0)

```
=== ENHANCED AUDIBLE CROSSFADE TEST ===
This test will play 3 MP3 files with crossfades through your speakers.
Features:
  - Fade-in on passage 1
  - Crossfades between passages 1→2 and 2→3
  - Fade-out on passage 3 (no abrupt cutoff)
  - RMS level tracking to detect clipping
  - Timing verification against expected timeline

Finding MP3 files in /home/sw/Music...
Found 3 MP3 files:
  1. Superfly.mp3
  2. Dear_Mr._President.mp3
  3. What's_Up.mp3

=== Decoding files ===
[...]

=== Expected Timeline ===
  Passage 1 starts: 0.0s
  Passage 1 fade-in complete: 5.0s
  Crossfade 1→2: 25.0s - 30.0s
  Crossfade 2→3: 50.0s - 55.0s
  Passage 3 fade-out: 75.0s - 80.0s
  Total duration: 80.0s

=== Playing audio ===
Total playback duration: 80.0s
----------------------------------------

  Level at 0.3s: RMS=0.123
Progress: 5% (4.0s / 80.0s) [RMS: 0.445]

>>> Passage 1 fade-in complete at 5.01s <<<

Progress: 10% (8.0s / 80.0s) [RMS: 0.456]
[...]
Progress: 30% (24.0s / 80.0s) [RMS: 0.448]

>>> Triggering crossfade 1→2 at 25.00s <<<

Progress: 35% (28.0s / 80.0s) [RMS: 0.451]
>>> Crossfade 1→2 complete at 30.01s <<<

[...]

>>> Triggering crossfade 2→3 at 50.00s <<<
>>> Crossfade 2→3 complete at 55.02s <<<

[...]
Progress: 90% (72.0s / 80.0s) [RMS: 0.443]

>>> Triggering passage 3 fade-out at 75.00s <<<
>>> Fade-out complete at 80.01s (RMS=0.005) <<<

=== Playback complete ===

=== Verifying Timeline ===
✓ All timing checks PASSED!
✓ Final fade-out successful (RMS=0.005)

=== Event Log ===
Passage1Started { time: 0.0 }
Passage1FadeInComplete { time: 5.01 }
Crossfade1To2Started { time: 25.00 }
Crossfade1To2Complete { time: 30.01 }
Crossfade2To3Started { time: 50.00 }
Crossfade2To3Complete { time: 55.02 }
Passage3FadeOutStarted { time: 75.00 }
Passage3FadeOutComplete { time: 80.01 }

=== Test Summary ===
Test finished successfully!

Listening feedback:
  - Did you hear smooth fade-in at the start?
  - Were the crossfades between passages smooth?
  - Did passage 3 fade out smoothly to silence (no abrupt cutoff)?
  - Were there any clicks, pops, or distortion?
  - Did the volume remain consistent during crossfades?

test test_audible_crossfade ... ok
```

**Total Time:** ~135 seconds (includes compilation)

## Troubleshooting

### No MP3 files found

**Problem:** "ERROR: Not enough MP3 files found. Need 3, found 0."

**Solution:**
- Ensure `/home/sw/Music` exists
- Add at least 3 MP3 files to the directory (or any subdirectory)
- Or modify the test to search a different path

### Audio output error

**Problem:** "ERROR: Failed to open audio output"

**Solution:**
- Check that your audio device is working
- Ensure no other application is using the audio device exclusively
- Try running: `speaker-test -t sine -f 440 -c 2` to test audio

### Test takes too long

**Problem:** Test runs but takes very long (>3 minutes)

**Solution:**
- This is expected (80 seconds playback + ~50 seconds setup/compilation)
- To make it faster, reduce passage duration in the test (line 154)
- Or reduce crossfade duration (line 184)

### Compilation warnings

**Problem:** Many "unused import" or "never used" warnings

**Solution:**
- These are expected (wkmp-ap is a work in progress)
- The warnings don't affect test functionality
- They will be cleaned up in production code

## Code Structure

### Main Components

1. **`find_mp3_files()`** - Recursively searches for MP3 files
2. **`BlockingMixer`** - Wrapper that makes async mixer sync-friendly
3. **`test_audible_crossfade()`** - Main test function

### Key Code Sections

```rust
// BlockingMixer wrapper for sync audio callback
struct BlockingMixer {
    runtime: tokio::runtime::Runtime,
    mixer: Arc<tokio::sync::RwLock<CrossfadeMixer>>,
}

// Audio callback (runs on audio thread)
output.start(move || {
    if let Ok(m) = mixer_clone.try_lock() {
        m.get_next_frame()  // Get next audio frame
    } else {
        AudioFrame::zero()  // Return silence if locked
    }
})

// Main timing loop (triggers crossfades at right moments)
loop {
    if elapsed >= trigger_time {
        mixer.start_crossfade(...)
    }
}
```

## Success Criteria

✓ Test compiles without errors
✓ Finds and decodes 3 MP3 files
✓ Audio plays through speakers for ~80 seconds
✓ Progress updates printed every 5% with RMS levels
✓ Both crossfades triggered successfully
✓ **Fade-out completes smoothly (final RMS < 0.01)**
✓ **All timing checks pass (within 500ms tolerance)**
✓ **No clipping detected (RMS never exceeds 0.95)**
✓ No crashes or panics
✓ Test passes with exit code 0

## New Features in v2.0

### 1. Fade-Out Implementation
The test now fades out the final passage smoothly instead of cutting it off abruptly:
- Creates a silent buffer (10 seconds of zeros)
- Crossfades passage 3 to silence using logarithmic curve
- Duration: 5 seconds (configurable)
- Result: Smooth, professional-sounding ending

### 2. RMS Level Tracking
Continuous monitoring of audio levels:
- 100ms rolling window (4410 samples at 44.1kHz)
- Average of left/right channels
- Logs significant level changes (> 0.1 difference)
- Warns if RMS exceeds 0.95 (possible clipping)
- Verifies final silence (RMS < 0.01)

### 3. Timing Verification
Automated verification of event timing:
- Calculates expected times for all events
- Compares actual vs expected with 500ms tolerance
- Reports timing errors at end of test
- Helps detect problems with crossfade scheduling

### 4. Event Logging
Complete timeline of playback events:
- All key events timestamped
- Displayed in event log at end
- Useful for debugging timing issues
- Helps correlate audio issues with specific moments

## Implementation Notes

### How Fade-Out Works

Since the mixer doesn't have a dedicated "fade out current passage" method, we implement fade-out by crossfading to silence:

```rust
// Create silent buffer
let silent_buffer = Arc::new(tokio::sync::RwLock::new(PassageBuffer::new(
    Uuid::new_v4(),
    vec![0.0; 44100 * 2 * 10], // 10s silence
    44100,
    2,
)));

// Crossfade to silence (effectively fading out)
mixer.start_crossfade(
    silent_buffer,
    Uuid::new_v4(),
    FadeCurve::Logarithmic, // Fade out current passage
    5000,                    // 5 second fade
    FadeCurve::Linear,      // Fade in silence (doesn't matter)
    5000,
)?;
```

This approach:
- ✓ Reuses existing crossfade infrastructure
- ✓ Requires no new mixer methods
- ✓ Produces smooth, natural fade-out
- ✓ Sample-accurate timing

### AudioLevelTracker Implementation

```rust
struct AudioLevelTracker {
    samples: Vec<f32>,     // Rolling window
    window_size: usize,    // 4410 samples = 100ms
}

impl AudioLevelTracker {
    fn add_frame(&mut self, frame: &AudioFrame) {
        let avg = (frame.left.abs() + frame.right.abs()) / 2.0;
        self.samples.push(avg);
        if self.samples.len() > self.window_size {
            self.samples.remove(0);
        }
    }

    fn rms(&self) -> f32 {
        let sum: f32 = self.samples.iter().map(|s| s * s).sum();
        (sum / self.samples.len() as f32).sqrt()
    }
}
```

Called from the audio thread via try_lock() to avoid blocking.

## Next Steps

After verifying enhanced test:
1. Run with different MP3 files (various bitrates, sample rates)
2. Test with different crossfade durations (2s, 10s, 20s)
3. Test with different fade curves (Linear, SCurve, EqualPower)
4. ~~Add automated quality metrics (RMS level, clipping detection)~~ ✓ DONE
5. Add visual waveform output for analysis
6. Add spectral analysis to detect frequency issues
7. Add automated click/pop detection

## Related Files

- `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/mixer.rs` - CrossfadeMixer implementation
- `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/fade_curves.rs` - Fade curve math
- `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/decoder.rs` - MP3 decoder (symphonia)
- `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/output.rs` - Audio output (cpal)
- `/home/sw/Dev/McRhythm/docs/crossfade.md` - Crossfade design specification

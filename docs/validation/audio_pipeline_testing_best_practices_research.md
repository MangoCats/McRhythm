# Audio Pipeline Testing Best Practices Research Report

**Document Type:** Research Report
**Created:** 2025-10-22
**Purpose:** Document best practices for testing real-time audio processing pipelines
**Target Architecture:** WKMP SPEC016 decoder-buffer-mixer lock-free architecture
**Status:** Research Complete - Ready for Implementation Planning

---

## Executive Summary

This report synthesizes industry best practices, academic research, and practical methodologies for testing real-time audio processing pipelines. The recommendations are specifically tailored to the WKMP Audio Player's lock-free architecture (SPEC016) featuring:

- **Single-threaded decoder worker** processing multiple decoder chains serially
- **Lock-free SPSC ring buffers** (HeapRb) for producer-consumer communication
- **Atomic coordination** between decoder and mixer threads
- **Sample-accurate crossfading** and timing requirements

**Key Findings:**
1. **Sample integrity verification** requires deterministic test patterns + frame accounting + checksum validation
2. **Lock-free correctness** demands stress testing under concurrent load with eventual consistency verification
3. **Real-time performance** requires latency profiling, jitter measurement, and adaptive buffer sizing
4. **Hardware adaptation** (Raspberry Pi) needs CPU governor tuning, service minimization, and USB audio interfaces

---

## 1. Sample Integrity Testing

### 1.1 Frame Counting and Sample Accounting

**Industry Standard Approach:**
- **Total Sample Accounting:** Track cumulative samples at each pipeline stage (decode → resample → fade → buffer → mixer → output)
- **Conservation Law:** `samples_decoded - samples_dropped = samples_output` (within resampling ratio tolerance)
- **Frame-Level Precision:** Audio frames (not samples) are the atomic unit; verify `frame_count * channels_per_frame = total_samples`

**Recommended Implementation for WKMP:**

```rust
// Add to each pipeline stage (DecoderChain, PlayoutProducer, PlayoutConsumer, Mixer)
struct SampleAccountingMetrics {
    total_frames_in: AtomicU64,      // Frames entering this stage
    total_frames_out: AtomicU64,     // Frames leaving this stage
    total_frames_dropped: AtomicU64, // Frames intentionally dropped (underrun, overflow)
    last_update_timestamp: AtomicU64, // For rate calculation
}

impl SampleAccountingMetrics {
    fn verify_conservation(&self, expected_ratio: f64) -> Result<(), IntegrityError> {
        let frames_in = self.total_frames_in.load(Ordering::Relaxed);
        let frames_out = self.total_frames_out.load(Ordering::Relaxed);
        let frames_dropped = self.total_frames_dropped.load(Ordering::Relaxed);

        let expected_out = (frames_in as f64 * expected_ratio) as u64;
        let actual_out = frames_out + frames_dropped;

        // Allow 1% tolerance for resampling rounding
        let tolerance = (expected_out as f64 * 0.01) as u64;

        if actual_out.abs_diff(expected_out) > tolerance {
            return Err(IntegrityError::SampleLoss {
                stage: "decoder",
                expected: expected_out,
                actual: actual_out,
            });
        }

        Ok(())
    }
}
```

**Test Pattern:**
1. Decode known-length test file (e.g., 10 seconds @ 44.1kHz = 441,000 frames)
2. Verify `decoder.total_frames_decoded == 441,000 ± resampling_ratio`
3. Verify `buffer.total_frames_written == decoder.total_frames_output`
4. Verify `mixer.total_frames_consumed == buffer.total_frames_read`
5. Verify `output.total_frames_sent == mixer.total_frames_output`

**Tolerance Thresholds:**
- **PCM Passthrough (44.1kHz → 44.1kHz):** 0 frame loss tolerance
- **Resampling (e.g., 48kHz → 44.1kHz):** ±1 frame per 1000 frames (0.1%) due to rubato rounding
- **Fade boundaries:** Verify fade_in/fade_out sample counts match passage metadata

---

### 1.2 Underrun and Overrun Detection

**Buffer Underrun Characteristics:**
- **Symptom:** Mixer requests frame, buffer returns cached `last_frame` instead of new data
- **Audible Effect:** Repeated sample ("Max Headroom" stutter) or silence (if last_frame = zero)
- **Root Cause:** Decoder not keeping pace with mixer consumption rate

**Detection Methods:**

**Method 1: Event-Based Detection (WKMP Current Implementation)**
```rust
// In PlayoutConsumer::pop_frame()
pub fn pop_frame(&self) -> Result<AudioFrame, BufferEmptyError> {
    match self.consumer.try_pop() {
        Some(frame) => Ok(frame),
        None => {
            // UNDERRUN DETECTED - log and emit event
            warn!("Buffer underrun detected for passage {:?}", self.passage_id);
            self.underrun_count.fetch_add(1, Ordering::Relaxed);

            // Return cached last_frame to prevent silence
            Err(BufferEmptyError {
                last_frame: self.load_last_frame(),
                underrun_count: self.underrun_count.load(Ordering::Relaxed),
            })
        }
    }
}
```

**Method 2: Statistical Monitoring**
```rust
struct UnderrunStatistics {
    underrun_count: AtomicU64,
    last_underrun_timestamp: AtomicU64,
    consecutive_underruns: AtomicU64,
    max_consecutive_underruns: AtomicU64,
}

impl UnderrunStatistics {
    fn record_underrun(&self, timestamp_ns: u64) {
        let count = self.underrun_count.fetch_add(1, Ordering::Relaxed);
        let last_time = self.last_underrun_timestamp.swap(timestamp_ns, Ordering::Relaxed);

        // Consecutive if within 100ms
        if timestamp_ns - last_time < 100_000_000 {
            let consecutive = self.consecutive_underruns.fetch_add(1, Ordering::Relaxed) + 1;
            self.max_consecutive_underruns.fetch_max(consecutive, Ordering::Relaxed);
        } else {
            self.consecutive_underruns.store(1, Ordering::Relaxed);
        }
    }

    fn health_check(&self) -> HealthStatus {
        let consecutive = self.consecutive_underruns.load(Ordering::Relaxed);

        if consecutive > 10 {
            HealthStatus::Critical // Likely systematic problem
        } else if self.underrun_count.load(Ordering::Relaxed) > 0 {
            HealthStatus::Degraded // Transient underruns
        } else {
            HealthStatus::Healthy
        }
    }
}
```

**Method 3: Latency Threshold Monitoring**
```rust
// Monitor buffer fill level and warn before underrun occurs
fn check_buffer_health(fill_level: usize, capacity: usize, mixer_min_start_level: usize) -> BufferHealth {
    let fill_percent = (fill_level as f64 / capacity as f64) * 100.0;

    if fill_level == 0 {
        BufferHealth::Underrun
    } else if fill_level < (mixer_min_start_level / 2) {
        BufferHealth::CriticallyLow // < 0.5 seconds remaining
    } else if fill_level < mixer_min_start_level {
        BufferHealth::Low // < 1.0 seconds remaining
    } else {
        BufferHealth::Normal
    }
}
```

**Buffer Overrun Detection:**
```rust
// In PlayoutProducer::push_frame()
pub fn push_frame(&self, frame: AudioFrame) -> Result<(), BufferFullError> {
    match self.producer.try_push(frame) {
        Ok(()) => { /* normal case */ },
        Err(_) => {
            // OVERRUN DETECTED - decoder producing faster than mixer consuming
            warn!("Buffer overrun for passage {:?}", self.passage_id);
            self.overrun_count.fetch_add(1, Ordering::Relaxed);

            return Err(BufferFullError {
                capacity: self.capacity,
                occupied: self.fill_level.load(Ordering::Relaxed),
                overrun_count: self.overrun_count.load(Ordering::Relaxed),
            });
        }
    }
}
```

**Recommended Metrics:**
- **Underrun Rate:** `underruns_per_hour = (underrun_count / playback_hours)` - Target: <0.1/hour
- **Max Consecutive Underruns:** Target: <3 (indicates brief transient, not systematic issue)
- **Time to Recovery:** Measure time from underrun to buffer refill ≥ `mixer_min_start_level` - Target: <500ms

---

### 1.3 Checksum-Based Integrity Verification

**Audio-Specific Checksum Methods:**

**Method 1: Frame-Level CRC32 (Fast, Good for Testing)**
```rust
use crc32fast::Hasher;

struct AudioChecksumValidator {
    hasher: Hasher,
    frame_count: u64,
}

impl AudioChecksumValidator {
    fn feed_frame(&mut self, frame: AudioFrame) {
        // Hash raw f32 bits to avoid floating-point comparison issues
        self.hasher.update(&frame.left.to_bits().to_le_bytes());
        self.hasher.update(&frame.right.to_bits().to_le_bytes());
        self.frame_count += 1;
    }

    fn finalize(self) -> AudioChecksum {
        AudioChecksum {
            crc32: self.hasher.finalize(),
            frame_count: self.frame_count,
        }
    }
}
```

**Method 2: Adler-32 (Streaming-Friendly, FFmpeg Compatible)**
```rust
// Use adler crate
use adler::Adler32;

// Adler-32 is faster than CRC32 and sufficient for integrity checking
// FFmpeg uses Adler-32 for framecrc output format
let mut adler = Adler32::new();
for frame in audio_stream {
    adler.write_slice(&frame.left.to_bits().to_le_bytes());
    adler.write_slice(&frame.right.to_bits().to_le_bytes());
}
let checksum = adler.checksum();
```

**Method 3: MD5 Audio-Only Hash (Industry Standard for Audio Verification)**
```rust
use md5::{Md5, Digest};

struct AudioMD5Generator {
    hasher: Md5,
}

impl AudioMD5Generator {
    fn process_chunk(&mut self, frames: &[AudioFrame]) {
        for frame in frames {
            // Hash raw PCM data, not container/metadata
            self.hasher.update(&frame.left.to_bits().to_le_bytes());
            self.hasher.update(&frame.right.to_bits().to_le_bytes());
        }
    }

    fn finalize(self) -> [u8; 16] {
        self.hasher.finalize().into()
    }
}
```

**Testing Strategy with Checksums:**

1. **Golden Reference Generation:**
   - Decode test file with FFmpeg: `ffmpeg -i test.mp3 -f framemd5 reference.md5`
   - Or use Symphonia's built-in validation: `symphonia-check test.mp3 --verify md5`
   - Store golden checksums in test fixtures

2. **Pipeline Validation:**
   ```rust
   #[test]
   fn test_decode_integrity_matches_reference() {
       let test_file = "tests/fixtures/sine_440hz_10s_44100.flac";
       let expected_md5 = load_golden_checksum("sine_440hz_10s_44100.md5");

       // Decode through WKMP pipeline
       let decoder = DecoderChain::new(test_file, /* ... */);
       let mut hasher = AudioMD5Generator::new();

       while let Some(frame) = decoder.decode_next_frame() {
           hasher.process_frame(frame);
       }

       let actual_md5 = hasher.finalize();
       assert_eq!(actual_md5, expected_md5, "Audio integrity mismatch");
   }
   ```

3. **Streaming Verification:**
   ```rust
   // For long files, use incremental checksums per chunk
   struct ChunkedAudioVerifier {
       chunk_size: usize,
       current_hasher: Adler32,
       chunk_checksums: Vec<u32>,
   }

   // Verify each 1-second chunk matches expected sequence
   // Allows pinpointing exactly where corruption occurs
   ```

**Checksum Use Cases:**
- **Regression Testing:** Verify decoder output matches known-good baseline
- **Resampler Validation:** Compare input/output checksums (accounting for sample rate conversion)
- **Crossfade Correctness:** Hash output during crossfade, verify against pre-computed reference
- **Round-Trip Testing:** Play audio → capture → checksum, verify matches original

---

### 1.4 Gap and Discontinuity Detection

**Timestamp-Based Gap Detection:**
```rust
struct TimestampContinuityChecker {
    last_timestamp: Option<Duration>,
    expected_frame_duration: Duration,
    gap_count: u64,
    max_gap_duration: Duration,
}

impl TimestampContinuityChecker {
    fn check_frame(&mut self, timestamp: Duration) -> Option<GapDetection> {
        if let Some(last) = self.last_timestamp {
            let gap = timestamp.saturating_sub(last);
            let expected = self.expected_frame_duration;

            // Allow ±1 sample tolerance for jitter
            if gap > expected + Duration::from_micros(23) { // ~1 sample @ 44.1kHz
                self.gap_count += 1;
                self.max_gap_duration = self.max_gap_duration.max(gap);

                return Some(GapDetection {
                    expected_timestamp: last + expected,
                    actual_timestamp: timestamp,
                    gap_duration: gap - expected,
                });
            }
        }

        self.last_timestamp = Some(timestamp);
        None
    }
}
```

**Sample Value Discontinuity Detection (Click/Pop Detection):**
```rust
fn detect_discontinuities(frames: &[AudioFrame], threshold: f32) -> Vec<usize> {
    let mut discontinuities = Vec::new();

    for i in 1..frames.len() {
        let delta_left = (frames[i].left - frames[i-1].left).abs();
        let delta_right = (frames[i].right - frames[i-1].right).abs();

        // Threshold for "click" detection (e.g., 0.5 = 50% full scale jump)
        if delta_left > threshold || delta_right > threshold {
            discontinuities.push(i);
        }
    }

    discontinuities
}

// Test crossfade smoothness
#[test]
fn test_crossfade_no_clicks() {
    let crossfade_output = generate_crossfade_test(/* ... */);
    let clicks = detect_discontinuities(&crossfade_output, 0.3);

    assert_eq!(clicks.len(), 0, "Detected clicks at sample indices: {:?}", clicks);
}
```

---

## 2. Timing and Latency Testing

### 2.1 Round-Trip Latency Measurement

**Standard RTL Method (Hardware Loopback):**

1. **Hardware Setup:**
   - Connect audio output (DAC) to audio input (ADC) via 3.5mm cable or USB loopback adapter
   - For Raspberry Pi: Use USB sound card, not built-in 3.5mm jack (poor quality)

2. **Test Signal Generation:**
   ```rust
   // Generate impulse or sine sweep
   fn generate_test_impulse(sample_rate: u32) -> Vec<f32> {
       let mut samples = vec![0.0f32; sample_rate as usize]; // 1 second of silence
       samples[0] = 1.0; // Single impulse at t=0
       samples
   }

   fn generate_sine_sweep(sample_rate: u32, f_start: f32, f_end: f32, duration_s: f32) -> Vec<f32> {
       let num_samples = (sample_rate as f32 * duration_s) as usize;
       (0..num_samples)
           .map(|i| {
               let t = i as f32 / sample_rate as f32;
               let freq = f_start + (f_end - f_start) * t / duration_s;
               let phase = 2.0 * std::f32::consts::PI * freq * t;
               (phase.sin() * 0.5) // 50% amplitude to avoid clipping
           })
           .collect()
   }
   ```

3. **Cross-Correlation Detection:**
   ```rust
   // Find time offset between output and input signals
   fn measure_latency_cross_correlation(
       output_signal: &[f32],
       input_recording: &[f32],
       sample_rate: u32,
   ) -> Duration {
       let max_lag = output_signal.len();
       let mut max_correlation = f32::MIN;
       let mut best_lag = 0;

       for lag in 0..max_lag {
           let correlation = calculate_correlation(output_signal, input_recording, lag);
           if correlation > max_correlation {
               max_correlation = correlation;
               best_lag = lag;
           }
       }

       // Convert lag samples to time
       Duration::from_secs_f64(best_lag as f64 / sample_rate as f64)
   }

   fn calculate_correlation(signal_a: &[f32], signal_b: &[f32], lag: usize) -> f32 {
       let len = signal_a.len().min(signal_b.len() - lag);
       let mut sum = 0.0f32;

       for i in 0..len {
           sum += signal_a[i] * signal_b[i + lag];
       }

       sum / len as f32
   }
   ```

4. **Expected Latency Ranges:**
   - **USB Audio Interface:** 10-50ms typical (depends on buffer size and ASIO/ALSA settings)
   - **Built-in Audio (Raspberry Pi):** 50-200ms (avoid for professional use)
   - **JACK Audio:** 5-15ms (low-latency mode)
   - **Target for WKMP:** <50ms play command → first audio output

---

### 2.2 End-to-End Pipeline Latency

**Component-Level Latency Breakdown:**

```rust
struct LatencyProfiler {
    stage_timings: HashMap<String, Vec<Duration>>,
}

impl LatencyProfiler {
    fn profile_stage<F, R>(&mut self, stage_name: &str, operation: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = operation();
        let elapsed = start.elapsed();

        self.stage_timings
            .entry(stage_name.to_string())
            .or_insert_with(Vec::new)
            .push(elapsed);

        result
    }

    fn report(&self) {
        for (stage, timings) in &self.stage_timings {
            let mean = timings.iter().sum::<Duration>() / timings.len() as u32;
            let max = *timings.iter().max().unwrap();
            let p99 = percentile(timings, 0.99);

            println!("{}: mean={:?}, p99={:?}, max={:?}", stage, mean, p99, max);
        }
    }
}

// Example usage
#[test]
fn profile_decode_latency() {
    let mut profiler = LatencyProfiler::new();

    let frame = profiler.profile_stage("decode", || decoder.decode_frame());
    let resampled = profiler.profile_stage("resample", || resampler.process(frame));
    let faded = profiler.profile_stage("fade", || fader.apply(resampled));

    profiler.report();
}
```

**Expected Latency Budget (WKMP Architecture):**

| Stage | Expected Latency | Notes |
|-------|------------------|-------|
| Decoder (symphonia) | 0.5-2ms per chunk | MP3 slower than FLAC |
| Resampler (rubato) | 0.1-0.5ms per chunk | Depends on quality setting |
| Fade calculation | <0.01ms per frame | Simple multiplication |
| Buffer push | <0.001ms | Lock-free atomic operation |
| Mixer calculation | 0.1-1ms per refill | Crossfade math, volume |
| Output ring buffer | <0.001ms | Lock-free write |
| CPAL output callback | 5-20ms | Hardware-dependent |
| **Total Pipeline** | **10-50ms** | **Acceptable for music playback** |

**Latency Testing Strategy:**

```rust
#[test]
fn test_play_command_latency() {
    let engine = PlaybackEngine::new();
    let test_file = "tests/fixtures/short_test.mp3";

    // Measure time from enqueue to first audio output
    let start = Instant::now();
    engine.enqueue_file(test_file).await.unwrap();
    engine.play().await.unwrap();

    // Wait for first frame to be consumed by mixer
    while engine.mixer_stats().total_frames_consumed == 0 {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    let latency = start.elapsed();

    // Should start playback within 50ms for good UX
    assert!(latency < Duration::from_millis(50),
            "Play latency too high: {:?}", latency);
}
```

---

### 2.3 Jitter and Timing Consistency

**Mixer Callback Jitter Measurement:**

```rust
struct JitterAnalyzer {
    last_callback_time: Option<Instant>,
    inter_callback_intervals: Vec<Duration>,
    expected_interval: Duration,
}

impl JitterAnalyzer {
    fn new(sample_rate: u32, buffer_size: u32) -> Self {
        let expected_interval = Duration::from_secs_f64(
            buffer_size as f64 / sample_rate as f64
        );

        Self {
            last_callback_time: None,
            inter_callback_intervals: Vec::new(),
            expected_interval,
        }
    }

    fn record_callback(&mut self) {
        let now = Instant::now();

        if let Some(last) = self.last_callback_time {
            let interval = now.duration_since(last);
            self.inter_callback_intervals.push(interval);
        }

        self.last_callback_time = Some(now);
    }

    fn analyze(&self) -> JitterStats {
        let intervals = &self.inter_callback_intervals;
        let expected = self.expected_interval;

        let mean = intervals.iter().sum::<Duration>() / intervals.len() as u32;
        let max_jitter = intervals.iter()
            .map(|&interval| interval.abs_diff(expected))
            .max()
            .unwrap();

        let p99_jitter = percentile(
            &intervals.iter()
                .map(|&interval| interval.abs_diff(expected))
                .collect::<Vec<_>>(),
            0.99
        );

        JitterStats {
            mean_interval: mean,
            expected_interval: expected,
            max_jitter,
            p99_jitter,
            callback_count: intervals.len(),
        }
    }
}

// Acceptable jitter thresholds
const MAX_ACCEPTABLE_JITTER_MS: u64 = 1; // 1ms p99 jitter is acceptable
const MAX_ABSOLUTE_JITTER_MS: u64 = 5;   // 5ms absolute max before audio glitches
```

**Real-Time Performance Validation:**

```rust
#[test]
fn test_mixer_thread_scheduling_jitter() {
    let engine = PlaybackEngine::new();
    let mut jitter_analyzer = JitterAnalyzer::new(44100, 512);

    // Play 10 seconds of audio
    engine.play_test_file("tests/fixtures/10s_test.flac").await;

    // Record callback timings
    for _ in 0..1000 {
        tokio::time::sleep(Duration::from_millis(10)).await;
        jitter_analyzer.record_callback();
    }

    let stats = jitter_analyzer.analyze();

    // Verify real-time performance
    assert!(stats.p99_jitter.as_millis() < MAX_ACCEPTABLE_JITTER_MS as u128,
            "Mixer jitter too high: {:?}", stats);
}
```

---

## 3. Deterministic Test Patterns

### 3.1 Generated Test Signals

**Sine Wave Generator (Pure Tone Testing):**

```rust
struct SineWaveGenerator {
    frequency: f32,
    sample_rate: u32,
    phase: f32,
}

impl SineWaveGenerator {
    fn new(frequency: f32, sample_rate: u32) -> Self {
        Self {
            frequency,
            sample_rate,
            phase: 0.0,
        }
    }

    fn generate_stereo(&mut self, num_frames: usize) -> Vec<AudioFrame> {
        let phase_increment = 2.0 * std::f32::consts::PI * self.frequency / self.sample_rate as f32;

        (0..num_frames)
            .map(|_| {
                let sample = self.phase.sin() * 0.5; // 50% amplitude
                self.phase += phase_increment;

                // Keep phase in [0, 2π] to avoid precision loss
                if self.phase >= 2.0 * std::f32::consts::PI {
                    self.phase -= 2.0 * std::f32::consts::PI;
                }

                AudioFrame::from_stereo(sample, sample)
            })
            .collect()
    }
}

// Test usage
#[test]
fn test_pipeline_with_sine_440hz() {
    let mut generator = SineWaveGenerator::new(440.0, 44100);
    let test_signal = generator.generate_stereo(44100); // 1 second

    // Feed through pipeline and verify output matches input
    let output = process_through_pipeline(test_signal);

    // Verify frequency content using FFT (simplified)
    let peak_frequency = find_dominant_frequency(&output, 44100);
    assert!((peak_frequency - 440.0).abs() < 1.0, "Frequency shift detected");
}
```

**Impulse Response Generator:**

```rust
fn generate_impulse(sample_rate: u32) -> Vec<AudioFrame> {
    let mut frames = vec![AudioFrame::zero(); sample_rate as usize];
    frames[0] = AudioFrame::from_stereo(1.0, 1.0); // δ(0) = 1
    frames
}

#[test]
fn test_impulse_response() {
    let impulse = generate_impulse(44100);
    let output = process_through_pipeline(impulse);

    // Impulse response should show system's frequency response
    // All frequencies present in output (within Nyquist limit)
    verify_flat_frequency_response(&output, 44100);
}
```

**White Noise Generator (Stochastic Testing):**

```rust
use rand::Rng;

fn generate_white_noise(duration_frames: usize, amplitude: f32) -> Vec<AudioFrame> {
    let mut rng = rand::thread_rng();

    (0..duration_frames)
        .map(|_| {
            let left = rng.gen::<f32>() * 2.0 - 1.0; // Range [-1, 1]
            let right = rng.gen::<f32>() * 2.0 - 1.0;
            AudioFrame::from_stereo(left * amplitude, right * amplitude)
        })
        .collect()
}

#[test]
fn test_noise_robustness() {
    // Verify pipeline handles random data without crashes or NaN
    let noise = generate_white_noise(44100, 0.3);
    let output = process_through_pipeline(noise);

    // No samples should be NaN or Inf
    for frame in output {
        assert!(frame.left.is_finite());
        assert!(frame.right.is_finite());
    }
}
```

**Silence (Zero) Generator:**

```rust
fn generate_silence(duration_frames: usize) -> Vec<AudioFrame> {
    vec![AudioFrame::zero(); duration_frames]
}

#[test]
fn test_silence_handling() {
    let silence = generate_silence(44100);
    let output = process_through_pipeline(silence);

    // Output should be bit-identical to input for silence
    for frame in output {
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }
}
```

---

### 3.2 Known-Good Test Files

**Recommended Test File Suite:**

1. **Sine Tones (Frequency Response Testing):**
   - `sine_20hz_44100.flac` - Low frequency boundary test
   - `sine_440hz_44100.flac` - A4 reference tone (standard tuning)
   - `sine_1000hz_44100.flac` - 1kHz reference (calibration standard)
   - `sine_10000hz_44100.flac` - High frequency test
   - `sine_20000hz_44100.flac` - Near Nyquist limit (humans can't hear, but tests aliasing)

2. **Stepped Sine Sweep (THD+N Testing):**
   - `sine_sweep_20_20000_44100.flac` - Logarithmic sweep for frequency response analysis
   - Expected output: FFT should show clean single peak moving through spectrum

3. **Impulse Files (Transient Response):**
   - `impulse_44100.flac` - Single 1.0 sample followed by zeros
   - `click_train_10hz_44100.flac` - 10Hz click train for testing transient handling

4. **Silence Files:**
   - `silence_1s_44100.flac` - Verify zero output with zero input
   - `silence_10s_44100.flac` - Test long-duration silence handling

5. **Sample Rate Conversion Test Files:**
   - `sine_440hz_48000.flac` - Test 48kHz → 44.1kHz resampling
   - `sine_440hz_96000.flac` - Test 96kHz → 44.1kHz resampling
   - `sine_440hz_22050.flac` - Test 22.05kHz → 44.1kHz upsampling

6. **Edge Cases:**
   - `full_scale_sine_440hz_44100.flac` - Amplitude = ±1.0 (clipping boundary)
   - `dc_offset_0.5_44100.flac` - Constant 0.5 DC value (tests DC blocking)
   - `alternating_polarity_44100.flac` - +1, -1, +1, -1... (Nyquist frequency square wave)

7. **Duration Tests:**
   - `sine_440hz_0.1s_44100.flac` - Very short file (4410 frames)
   - `sine_440hz_60s_44100.flac` - 1 minute duration
   - `sine_440hz_600s_44100.flac` - 10 minute duration (stress test)

**Generating Test Files:**

```bash
# Using FFmpeg
ffmpeg -f lavfi -i "sine=frequency=440:duration=10:sample_rate=44100" -c:a flac sine_440hz_44100.flac

# Using Sox
sox -n -r 44100 -c 2 sine_440hz_44100.flac synth 10 sine 440

# Generate silence
ffmpeg -f lavfi -i "anullsrc=r=44100:cl=stereo" -t 10 -c:a flac silence_10s_44100.flac
```

**Golden Checksum Generation:**

```bash
# Generate MD5 reference
ffmpeg -i sine_440hz_44100.flac -f framemd5 - > sine_440hz_44100.md5

# Generate CRC reference (per-packet checksum)
ffmpeg -i sine_440hz_44100.flac -f framecrc - > sine_440hz_44100.crc
```

**Test File Organization:**

```
tests/
  fixtures/
    audio/
      sine/
        sine_440hz_44100.flac
        sine_440hz_44100.md5
      impulse/
        impulse_44100.flac
      silence/
        silence_10s_44100.flac
      sample_rates/
        sine_440hz_48000.flac
        sine_440hz_96000.flac
      edge_cases/
        full_scale_sine_440hz_44100.flac
```

---

### 3.3 Crossfade Validation Testing

**Crossfade Correctness Verification:**

```rust
#[test]
fn test_crossfade_output_matches_reference() {
    // Load two passages with known crossfade parameters
    let passage_a = load_test_passage("sine_440hz.flac", 0.0, 5.0, 4.5, 5.0); // fade_out 4.5-5.0
    let passage_b = load_test_passage("sine_880hz.flac", 0.0, 5.0, 0.0, 0.5); // fade_in 0.0-0.5

    // Generate crossfade
    let crossfade_output = engine.process_crossfade(passage_a, passage_b);

    // Verify envelope shape matches fade curve (e.g., S-curve)
    verify_fade_curve(&crossfade_output, FadeCurve::SCurve);

    // Verify no clicks/pops (sample delta threshold)
    let clicks = detect_discontinuities(&crossfade_output, 0.3);
    assert_eq!(clicks.len(), 0);

    // Verify total energy conservation (RMS should be roughly constant)
    verify_energy_conservation(&crossfade_output);

    // Compare against golden reference
    let expected_checksum = load_golden_checksum("crossfade_440_880_scurve.md5");
    let actual_checksum = calculate_md5(&crossfade_output);
    assert_eq!(actual_checksum, expected_checksum);
}

fn verify_fade_curve(frames: &[AudioFrame], curve_type: FadeCurve) {
    // Extract envelope (RMS over small windows)
    let envelope = extract_rms_envelope(frames, 441); // 10ms windows

    match curve_type {
        FadeCurve::SCurve => {
            // Should be smooth sigmoid shape
            for i in 1..envelope.len() {
                let slope = envelope[i] - envelope[i-1];
                // Slope should gradually decrease (no sudden jumps)
                assert!(slope.abs() < 0.1, "Abrupt envelope change at index {}", i);
            }
        },
        FadeCurve::Linear => {
            // Constant slope
            verify_constant_slope(&envelope, 0.01);
        },
        // ... other curve types
    }
}
```

---

## 4. Performance Profiling

### 4.1 CPU Usage Monitoring

**Thread-Specific CPU Monitoring:**

```rust
use sysinfo::{System, SystemExt, ProcessExt};

struct CpuProfiler {
    system: System,
    target_pid: u32,
    samples: Vec<CpuSample>,
}

struct CpuSample {
    timestamp: Instant,
    cpu_usage_percent: f32,
    thread_breakdowns: HashMap<String, f32>,
}

impl CpuProfiler {
    fn sample(&mut self) {
        self.system.refresh_processes();

        if let Some(process) = self.system.process(sysinfo::Pid::from_u32(self.target_pid)) {
            let sample = CpuSample {
                timestamp: Instant::now(),
                cpu_usage_percent: process.cpu_usage(),
                thread_breakdowns: HashMap::new(), // Requires thread-level profiling
            };

            self.samples.push(sample);
        }
    }

    fn report(&self) -> CpuStats {
        let mean_cpu = self.samples.iter()
            .map(|s| s.cpu_usage_percent)
            .sum::<f32>() / self.samples.len() as f32;

        let max_cpu = self.samples.iter()
            .map(|s| s.cpu_usage_percent)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        CpuStats {
            mean_cpu_percent: mean_cpu,
            max_cpu_percent: *max_cpu,
            sample_count: self.samples.len(),
        }
    }
}

// Target CPU usage for audio applications
const MAX_ACCEPTABLE_CPU_PERCENT: f32 = 25.0; // Should not exceed 25% on modern hardware
const MAX_SPIKE_CPU_PERCENT: f32 = 50.0;      // Brief spikes up to 50% acceptable
```

**Rust-Specific Profiling Tools:**

1. **perf (Linux):**
   ```bash
   # Profile WKMP audio player
   perf record -F 999 -g ./target/release/wkmp-ap
   perf report

   # Identify hot spots in decoder/mixer
   perf top -p $(pidof wkmp-ap)
   ```

2. **cargo-flamegraph:**
   ```bash
   cargo install flamegraph
   cargo flamegraph --bin wkmp-ap -- --test-mode
   # Generates flamegraph.svg showing CPU time distribution
   ```

3. **Inline Performance Counters (Hardware Counters):**
   ```rust
   #[cfg(target_arch = "x86_64")]
   fn read_cpu_cycles() -> u64 {
       unsafe { core::arch::x86_64::_rdtsc() }
   }

   // Measure decode cycle cost
   let start_cycles = read_cpu_cycles();
   decoder.decode_chunk();
   let cycles = read_cpu_cycles() - start_cycles;
   ```

---

### 4.2 Memory Allocation Profiling

**Heap Allocation Tracking:**

```rust
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrackingAllocator;

static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        ALLOCATED_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

#[test]
fn test_decoder_allocation_stability() {
    let initial_allocs = ALLOCATION_COUNT.load(Ordering::Relaxed);

    // Decode 10 seconds of audio
    decoder.decode_duration(Duration::from_secs(10));

    let final_allocs = ALLOCATION_COUNT.load(Ordering::Relaxed);
    let allocs_per_second = (final_allocs - initial_allocs) / 10;

    // Decoder should stabilize to ~0 allocations after initial buffers allocated
    assert!(allocs_per_second < 10, "Excessive allocations in hot path");
}
```

**Zero-Allocation Verification for Hot Paths:**

```rust
#[test]
fn test_mixer_callback_zero_alloc() {
    // Mixer callback must NEVER allocate (real-time constraint)
    let allocs_before = ALLOCATION_COUNT.load(Ordering::Relaxed);

    mixer.process_next_frame();

    let allocs_after = ALLOCATION_COUNT.load(Ordering::Relaxed);

    assert_eq!(allocs_before, allocs_after,
               "Mixer callback allocated memory (FORBIDDEN in real-time context)");
}
```

---

### 4.3 Thread Scheduling Analysis

**Real-Time Thread Priority Verification:**

```rust
use nix::sys::pthread::pthread_setschedparam;
use nix::sched::{sched_getscheduler, SchedPolicy};

fn verify_audio_thread_priority() -> Result<(), String> {
    let thread_id = std::thread::current().id();
    let policy = sched_getscheduler(nix::unistd::Pid::from_raw(0))
        .map_err(|e| format!("Failed to get scheduler: {}", e))?;

    match policy {
        SchedPolicy::SCHED_FIFO | SchedPolicy::SCHED_RR => {
            // Good - real-time scheduling policy
            Ok(())
        },
        _ => {
            Err(format!("Audio thread not using real-time scheduler: {:?}", policy))
        }
    }
}

#[test]
fn test_mixer_thread_priority() {
    // Mixer thread should be SCHED_FIFO or SCHED_RR with high priority
    verify_audio_thread_priority().expect("Mixer thread not real-time");
}
```

---

## 5. Hardware-Specific Optimization (Raspberry Pi)

### 5.1 CPU Frequency Scaling

**Set CPU Governor to Performance Mode:**

```bash
# Prevent CPU frequency scaling during audio playback
sudo cpufreq-set -g performance

# Or persist across reboots
echo "performance" | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
```

**Verify CPU Frequency:**

```bash
# Check current CPU frequency
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq

# Monitor frequency during playback
watch -n 1 'cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq'
```

**Rust Implementation (Runtime Check):**

```rust
fn check_cpu_governor() -> Result<String, std::io::Error> {
    std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
}

fn warn_if_not_performance() {
    if let Ok(governor) = check_cpu_governor() {
        if !governor.trim().eq_ignore_ascii_case("performance") {
            eprintln!("WARNING: CPU governor is '{}', audio performance may be degraded. Set to 'performance' for best results.", governor.trim());
        }
    }
}
```

---

### 5.2 Service Minimization

**Disable Unnecessary Services:**

```bash
# Stop services that interfere with real-time audio
sudo systemctl stop ntp
sudo systemctl stop triggerhappy
sudo systemctl stop bluetooth
sudo systemctl stop cups

# Disable on boot
sudo systemctl disable ntp triggerhappy bluetooth cups
```

**Recommended Patchbox OS Settings:**

- Patchbox OS is a pre-configured Raspberry Pi distribution for audio projects
- Comes with optimized kernel parameters and disabled unnecessary services
- Includes JACK audio server pre-configured for low latency

---

### 5.3 USB Audio Interface Recommendations

**Why USB Audio?**

- Raspberry Pi's built-in 3.5mm jack uses PWM (poor quality, high noise floor)
- USB audio codecs are far more linear and have better SNR
- Lower latency achievable with USB Audio Class 2.0 devices

**Recommended USB Audio Interfaces for Raspberry Pi:**

1. **Behringer UCA202** (~$30) - Entry-level, reliable
2. **FocusRite Scarlett Solo** (~$120) - Professional quality
3. **HiFiBerry DAC+ ADC** (HAT) - GPIO-based, low latency

**ALSA Configuration for USB Audio:**

```bash
# Set USB device as default
cat > ~/.asoundrc <<EOF
pcm.!default {
    type hw
    card 1  # USB audio is typically card 1
}
ctl.!default {
    type hw
    card 1
}
EOF
```

---

### 5.4 Adaptive Buffer Sizing

**Dynamic Buffer Adjustment Based on CPU Load:**

```rust
struct AdaptiveBufferManager {
    current_buffer_size: usize,
    min_buffer_size: usize,
    max_buffer_size: usize,
    underrun_history: VecDeque<Instant>,
    cpu_load_threshold: f32,
}

impl AdaptiveBufferManager {
    fn adjust_buffer_size(&mut self, cpu_load: f32, underrun_occurred: bool) {
        if underrun_occurred {
            self.underrun_history.push_back(Instant::now());

            // Remove underruns older than 60s
            while let Some(time) = self.underrun_history.front() {
                if time.elapsed() > Duration::from_secs(60) {
                    self.underrun_history.pop_front();
                } else {
                    break;
                }
            }

            // Increase buffer if underruns frequent
            if self.underrun_history.len() > 5 {
                self.increase_buffer_size();
            }
        } else if cpu_load < self.cpu_load_threshold {
            // CPU headroom available, try smaller buffer for lower latency
            self.decrease_buffer_size();
        }
    }

    fn increase_buffer_size(&mut self) {
        let new_size = (self.current_buffer_size * 3 / 2).min(self.max_buffer_size);
        if new_size != self.current_buffer_size {
            info!("Increasing buffer size {} -> {} due to underruns",
                  self.current_buffer_size, new_size);
            self.current_buffer_size = new_size;
        }
    }

    fn decrease_buffer_size(&mut self) {
        let new_size = (self.current_buffer_size * 2 / 3).max(self.min_buffer_size);
        if new_size != self.current_buffer_size {
            info!("Decreasing buffer size {} -> {} (CPU headroom available)",
                  self.current_buffer_size, new_size);
            self.current_buffer_size = new_size;
        }
    }
}
```

**Platform-Specific Defaults:**

```rust
fn get_optimal_buffer_size() -> usize {
    // Detect platform
    #[cfg(target_arch = "aarch64")]
    {
        // Raspberry Pi / ARM - larger buffers
        if is_raspberry_pi() {
            88200 // 2 seconds @ 44.1kHz
        } else {
            44100 // 1 second for other ARM devices
        }
    }

    #[cfg(target_arch = "x86_64")]
    {
        // Desktop/server - smaller buffers for lower latency
        22050 // 0.5 seconds @ 44.1kHz
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        44100 // Default fallback
    }
}

fn is_raspberry_pi() -> bool {
    std::fs::read_to_string("/proc/device-tree/model")
        .map(|model| model.contains("Raspberry Pi"))
        .unwrap_or(false)
}
```

---

## 6. Lock-Free Architecture Testing

### 6.1 Concurrent Push/Pop Stress Testing

**Multi-Threaded Race Condition Detection:**

```rust
#[test]
fn test_concurrent_producer_consumer_correctness() {
    const NUM_FRAMES: usize = 1_000_000;

    let buffer = PlayoutRingBuffer::new(Some(100_000), Some(1000), None, None);
    let (producer, consumer) = buffer.split();

    let producer_arc = Arc::new(producer);
    let consumer_arc = Arc::new(consumer);

    // Producer thread
    let producer_handle = {
        let producer = Arc::clone(&producer_arc);
        std::thread::spawn(move || {
            for i in 0..NUM_FRAMES {
                let frame = AudioFrame::from_stereo(i as f32, i as f32);

                // Retry on full (expected with bounded buffer)
                while let Err(_) = producer.push_frame(frame) {
                    std::thread::yield_now();
                }
            }
        })
    };

    // Consumer thread
    let consumer_handle = {
        let consumer = Arc::clone(&consumer_arc);
        std::thread::spawn(move || {
            let mut received = Vec::new();

            for _ in 0..NUM_FRAMES {
                // Retry on empty (expected if consumer faster than producer)
                loop {
                    match consumer.pop_frame() {
                        Ok(frame) => {
                            received.push(frame);
                            break;
                        },
                        Err(_) => {
                            std::thread::yield_now();
                        }
                    }
                }
            }

            received
        })
    };

    // Wait for completion
    producer_handle.join().unwrap();
    let received_frames = consumer_handle.join().unwrap();

    // Verify correctness
    assert_eq!(received_frames.len(), NUM_FRAMES);

    for (i, frame) in received_frames.iter().enumerate() {
        assert_eq!(frame.left, i as f32, "Frame {} corrupted", i);
        assert_eq!(frame.right, i as f32, "Frame {} corrupted", i);
    }
}
```

**ABA Problem Detection:**

```rust
#[test]
fn test_no_aba_problem() {
    // ABA problem: Consumer reads index, producer wraps around, consumer writes stale index
    // HeapRb should handle this correctly with wrapping arithmetic

    let buffer = PlayoutRingBuffer::new(Some(100), Some(10), None, None);
    let (producer, consumer) = buffer.split();

    // Rapidly wrap around buffer multiple times
    for iteration in 0..1000 {
        // Fill buffer
        for i in 0..90 {
            let frame = AudioFrame::from_stereo(
                (iteration * 100 + i) as f32,
                (iteration * 100 + i) as f32
            );
            producer.push_frame(frame).unwrap();
        }

        // Drain buffer
        for _ in 0..90 {
            consumer.pop_frame().unwrap();
        }
    }

    // If ABA problem exists, buffer would be corrupted by now
    // Fill and verify one more time
    for i in 0..90 {
        producer.push_frame(AudioFrame::from_stereo(i as f32, i as f32)).unwrap();
    }

    for i in 0..90 {
        let frame = consumer.pop_frame().unwrap();
        assert_eq!(frame.left, i as f32);
    }
}
```

---

### 6.2 Atomic Ordering Verification

**Memory Ordering Correctness Testing:**

```rust
#[test]
fn test_pause_resume_memory_ordering() {
    // Verify Release/Acquire pairing for decoder_should_pause flag

    let buffer = PlayoutRingBuffer::new(Some(100), Some(10), None, None);
    let (producer, consumer) = buffer.split();

    let producer_arc = Arc::new(producer);
    let consumer_arc = Arc::new(consumer);

    // Producer: Fill buffer to trigger pause
    let producer_handle = {
        let producer = Arc::clone(&producer_arc);
        std::thread::spawn(move || {
            for _ in 0..95 {
                producer.push_frame(AudioFrame::zero()).unwrap();
            }

            // Pause flag should now be set (verified with Acquire load)
            assert!(producer.should_decoder_pause());
        })
    };

    // Consumer: Drain buffer to trigger resume
    let consumer_handle = {
        let consumer = Arc::clone(&consumer_arc);
        std::thread::spawn(move || {
            // Wait for buffer to fill
            std::thread::sleep(Duration::from_millis(10));

            // Drain below resume threshold
            for _ in 0..50 {
                consumer.pop_frame().unwrap();
            }
        })
    };

    producer_handle.join().unwrap();
    consumer_handle.join().unwrap();

    // Verify pause flag cleared (memory ordering ensures visibility)
    assert!(!producer_arc.should_decoder_pause());
}
```

**Data Race Detection with Thread Sanitizer:**

```bash
# Build with thread sanitizer (Linux only)
RUSTFLAGS="-Z sanitizer=thread" cargo test --target x86_64-unknown-linux-gnu

# Thread sanitizer will catch:
# - Data races (non-atomic concurrent access)
# - Incorrect memory ordering (Relaxed where Acquire needed)
# - Lock ordering violations
```

---

### 6.3 Eventual Consistency Verification

**Fill Level Accuracy Testing:**

```rust
#[test]
fn test_fill_level_eventual_consistency() {
    let buffer = PlayoutRingBuffer::new(Some(1000), Some(100), None, None);
    let (producer, consumer) = buffer.split();

    // Push 500 frames
    for _ in 0..500 {
        producer.push_frame(AudioFrame::zero()).unwrap();
    }

    // Pop 200 frames
    for _ in 0..200 {
        consumer.pop_frame().unwrap();
    }

    // Fill level should be ~300 (allow off-by-one due to Relaxed ordering)
    let producer_fill = producer.stats().occupied;
    let consumer_fill = consumer.stats().occupied;

    assert!((producer_fill as i64 - 300).abs() <= 2, "Producer fill level: {}", producer_fill);
    assert!((consumer_fill as i64 - 300).abs() <= 2, "Consumer fill level: {}", consumer_fill);

    // Producer and consumer views may differ slightly (Relaxed)
    assert!((producer_fill as i64 - consumer_fill as i64).abs() <= 2);
}
```

---

### 6.4 Last Frame Cache Tearing Tolerance

**Verify Tearing Doesn't Cause Safety Violations:**

```rust
#[test]
fn test_last_frame_cache_tearing_safe() {
    let buffer = PlayoutRingBuffer::new(Some(100), Some(10), None, None);
    let (producer, consumer) = buffer.split();

    let producer_arc = Arc::new(producer);
    let consumer_arc = Arc::new(consumer);

    // Rapidly update last_frame from producer
    let producer_handle = {
        let producer = Arc::clone(&producer_arc);
        std::thread::spawn(move || {
            for i in 0..10000 {
                let frame = AudioFrame::from_stereo(i as f32, i as f32);
                let _ = producer.push_frame(frame);
            }
        })
    };

    // Simultaneously read last_frame from consumer (triggering underruns)
    let consumer_handle = {
        let consumer = Arc::clone(&consumer_arc);
        std::thread::spawn(move || {
            for _ in 0..10000 {
                // Force underruns by popping from empty buffer
                let _ = consumer.pop_frame();
            }
        })
    };

    producer_handle.join().unwrap();
    consumer_handle.join().unwrap();

    // If tearing causes safety violation (NaN, Inf), test would panic
    // Success = no panic, even if left/right channels occasionally mismatched
}
```

---

## 7. Recommended Testing Framework

### 7.1 Unit Test Structure

```rust
// wkmp-ap/src/playback/tests/mod.rs
mod ring_buffer_tests {
    use super::*;

    #[test]
    fn test_basic_push_pop() { /* ... */ }

    #[test]
    fn test_underrun_returns_cached_frame() { /* ... */ }

    #[test]
    fn test_pause_threshold_triggers() { /* ... */ }

    #[test]
    fn test_atomic_ordering_correctness() { /* ... */ }
}

mod decoder_tests {
    #[test]
    fn test_decode_chunk_sample_count() { /* ... */ }

    #[test]
    fn test_decode_checksum_matches_reference() { /* ... */ }

    #[test]
    fn test_resampler_output_length() { /* ... */ }
}

mod integration_tests {
    #[tokio::test]
    async fn test_end_to_end_playback() { /* ... */ }

    #[tokio::test]
    async fn test_concurrent_decode_mix() { /* ... */ }
}
```

---

### 7.2 Continuous Integration Testing

**GitHub Actions Workflow:**

```yaml
name: Audio Pipeline Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libasound2-dev

      - name: Run unit tests
        run: cargo test --lib

      - name: Run integration tests
        run: cargo test --test '*'

      - name: Run benchmarks (comparison only)
        run: cargo bench --no-run

      - name: Thread sanitizer tests (Linux only)
        run: |
          RUSTFLAGS="-Z sanitizer=thread" cargo test --target x86_64-unknown-linux-gnu
```

---

### 7.3 Benchmark Suite (Criterion)

```rust
// benches/audio_pipeline.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_decode_chunk(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");

    for codec in &["mp3", "flac", "aac"] {
        group.bench_with_input(BenchmarkId::new("decode_chunk", codec), codec, |b, &codec| {
            let decoder = setup_decoder(codec);
            b.iter(|| {
                decoder.decode_chunk()
            });
        });
    }

    group.finish();
}

fn bench_ring_buffer_push_pop(c: &mut Criterion) {
    let (producer, consumer) = create_test_buffer();

    c.bench_function("ring_buffer_push", |b| {
        b.iter(|| {
            producer.push_frame(black_box(AudioFrame::zero()))
        });
    });

    c.bench_function("ring_buffer_pop", |b| {
        b.iter(|| {
            consumer.pop_frame()
        });
    });
}

criterion_group!(benches, bench_decode_chunk, bench_ring_buffer_push_pop);
criterion_main!(benches);
```

---

## 8. Implementation Roadmap

### Phase 1: Basic Integrity Testing (Week 1)
- [ ] Implement frame counting for each pipeline stage
- [ ] Add sample accounting metrics (total_in, total_out, total_dropped)
- [ ] Create deterministic test signal generators (sine, impulse, silence)
- [ ] Implement CRC32 checksumming for regression testing

### Phase 2: Underrun/Overrun Detection (Week 1)
- [ ] Add underrun statistics tracking
- [ ] Implement event-based underrun logging
- [ ] Create health check system (Normal/Low/Critical/Underrun)
- [ ] Add overrun detection and decoder pause verification

### Phase 3: Latency and Timing (Week 2)
- [ ] Implement latency profiler for each pipeline stage
- [ ] Create jitter analyzer for mixer callbacks
- [ ] Add end-to-end latency measurement (enqueue → first output)
- [ ] Implement timestamp continuity checker

### Phase 4: Lock-Free Verification (Week 2-3)
- [ ] Create concurrent push/pop stress tests (1M+ frames)
- [ ] Implement ABA problem detection test
- [ ] Add memory ordering verification tests
- [ ] Test with Thread Sanitizer on CI

### Phase 5: Hardware Optimization (Week 3)
- [ ] Implement adaptive buffer sizing based on CPU load
- [ ] Add CPU governor detection and warning
- [ ] Create platform-specific buffer size defaults
- [ ] Document Raspberry Pi optimization guide

### Phase 6: Test File Suite (Week 4)
- [ ] Generate golden reference test files (sine tones, impulses, silence)
- [ ] Create MD5 golden checksums for all test files
- [ ] Organize test fixtures directory structure
- [ ] Add crossfade validation test suite

### Phase 7: Integration and CI (Week 4)
- [ ] Set up Criterion benchmark suite
- [ ] Configure GitHub Actions for automated testing
- [ ] Add performance regression detection
- [ ] Create dashboard for test metrics

---

## 9. Rust-Specific Tools and Libraries

### Recommended Crates for Testing

```toml
[dev-dependencies]
# Testing frameworks
criterion = "0.5"           # Benchmarking
proptest = "1.0"            # Property-based testing

# Audio analysis
dasp = "0.11"               # DSP utilities
rustfft = "6.0"             # FFT for frequency analysis
hound = "3.5"               # WAV file reading/writing

# Checksumming
crc32fast = "1.3"           # Fast CRC32
adler = "1.0"               # Adler-32 checksum
md5 = "0.7"                 # MD5 hashing

# System monitoring
sysinfo = "0.29"            # CPU/memory monitoring
perf-event = "0.4"          # Linux perf counters

# Concurrency testing
loom = "0.5"                # Concurrency model checking
```

### Symphonia-Specific Testing

```rust
// Use symphonia-check for golden reference generation
use symphonia::core::codecs::Decoder;
use symphonia::core::formats::FormatReader;

fn validate_decode_output(file_path: &str) -> Result<AudioChecksum, Error> {
    let mut format = symphonia::default::get_probe()
        .format(file_path, Default::default(), &Default::default())?;

    let track = format.default_track().unwrap();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())?;

    let mut hasher = AudioMD5Generator::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };

        let decoded = decoder.decode(&packet)?;

        // Convert to AudioFrame and hash
        for sample_plane in decoded.planes().planes() {
            for &sample in sample_plane {
                hasher.process_sample(sample);
            }
        }
    }

    Ok(hasher.finalize())
}
```

---

## 10. Key Takeaways and Recommendations

### Critical Success Factors

1. **Sample Integrity is Non-Negotiable:**
   - Implement frame accounting at every pipeline stage
   - Use checksumming for regression testing
   - Zero tolerance for dropped frames (except intentional underrun recovery)

2. **Lock-Free Correctness Requires Rigorous Testing:**
   - Stress test with 1M+ concurrent operations
   - Use Thread Sanitizer in CI
   - Verify eventual consistency of atomic counters
   - Test memory ordering with race condition scenarios

3. **Real-Time Performance Demands Predictability:**
   - Measure jitter, not just average latency
   - Target <1ms p99 jitter for mixer thread
   - Zero allocations in audio callback
   - CPU governor must be "performance" on embedded devices

4. **Hardware Adaptation is Essential:**
   - Raspberry Pi needs 2x buffer sizes vs. desktop
   - USB audio interface mandatory (not built-in 3.5mm)
   - Disable unnecessary services (ntp, bluetooth, etc.)
   - Monitor CPU frequency scaling during playback

### Recommended Testing Priorities

**Priority 1 (MVP - Must Have):**
- Frame counting and sample accounting
- Underrun/overrun detection and logging
- Concurrent push/pop correctness tests
- Basic latency profiling

**Priority 2 (Production - Should Have):**
- Checksumming and golden reference validation
- Jitter measurement and monitoring
- Test file suite with known patterns
- CPU/memory profiling

**Priority 3 (Optimization - Nice to Have):**
- Adaptive buffer sizing
- Platform-specific tuning
- Advanced FFT-based analysis
- Crossfade quality metrics

### Common Pitfalls to Avoid

❌ **Don't:** Rely solely on manual listening tests for quality verification
✅ **Do:** Combine automated checksumming with selective manual validation

❌ **Don't:** Test with only short audio files (<10s)
✅ **Do:** Include long-duration tests (10+ minutes) to catch memory leaks and drift

❌ **Don't:** Assume lock-free = correct
✅ **Do:** Stress test with concurrent operations and Thread Sanitizer

❌ **Don't:** Use default buffer sizes across all hardware
✅ **Do:** Detect platform and adjust buffer sizes accordingly

❌ **Don't:** Ignore underruns as "transient issues"
✅ **Do:** Track underrun statistics and investigate root causes

---

## 11. References and Further Reading

### Industry Standards and Best Practices

1. **ITU-R BS.1284-2** - General methods for subjective assessment of sound quality
2. **FFmpeg Testing Documentation** - framecrc, framemd5 output formats
3. **BWF MetaEdit** - Audio-only MD5 checksum standards (Broadcast Wave Format)
4. **JACK Audio Connection Kit** - Real-time audio architecture patterns
5. **Oblique Audio RTL Utility** - Round-trip latency measurement methodology

### Academic Papers

1. **"Using Locks in Real-Time Audio Processing, Safely"** - timur.audio (2023)
2. **"Lock-Free Ring Buffers for Embedded Systems"** - QuantumLeaps (2019)
3. **"Sample-Accurate Audio Timing in Digital Systems"** - AES Convention (2015)

### Rust-Specific Resources

1. **RustAudio Project** - github.com/RustAudio (cpal, rodio, dasp)
2. **Symphonia Documentation** - github.com/pdeljanov/Symphonia
3. **Lock-Free Programming in Rust** - Crossbeam and Atomic documentation

### Hardware Optimization Guides

1. **Linux Audio Wiki - Raspberry Pi** - wiki.linuxaudio.org/wiki/raspberrypi
2. **Patchbox OS Documentation** - blokas.io/patchbox-os (pre-configured audio RPi)
3. **Raspberry Pi Audio Optimization** - RuneAudio forum threads

---

## Document End

**Status:** Research Complete
**Next Action:** Implement Phase 1 testing (frame counting + underrun detection)
**Owner:** WKMP Development Team
**Review Date:** 2025-11-01

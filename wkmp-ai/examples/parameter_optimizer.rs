use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
/// Parameter Optimizer for Album Matching
///
/// For each album in training_set.txt:
/// 1. Decode MP3
/// 2. Search MusicBrainz for ground truth track durations
/// 3. Run parameter sweep to find optimal silence detection parameters
/// 4. Output results showing which parameters work best for each album
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tokio::time::sleep;

// MusicBrainz API structures
#[derive(Debug, Deserialize)]
struct MBSearchResponse {
    releases: Vec<MBRelease>,
}

#[derive(Debug, Deserialize)]
struct MBRelease {
    id: String,
    title: String,
    #[serde(rename = "artist-credit")]
    artist_credit: Option<Vec<MBArtistCredit>>,
}

#[derive(Debug, Deserialize)]
struct MBArtistCredit {
    artist: Option<MBArtist>,
}

#[derive(Debug, Deserialize)]
struct MBArtist {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MBReleaseDetails {
    id: String,
    title: String,
    media: Vec<MBMedia>,
}

#[derive(Debug, Deserialize)]
struct MBMedia {
    tracks: Vec<MBTrack>,
}

#[derive(Debug, Deserialize)]
struct MBTrack {
    length: Option<u32>, // in milliseconds
}

// Rate limiter for MusicBrainz API (1 request per second)
struct RateLimiter {
    last_request: Arc<Mutex<std::time::Instant>>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            last_request: Arc::new(Mutex::new(
                std::time::Instant::now() - Duration::from_secs(2),
            )),
        }
    }

    async fn wait(&self) {
        let elapsed = {
            let last = self.last_request.lock();
            last.elapsed()
        };

        if elapsed < Duration::from_secs(1) {
            let wait_time = Duration::from_secs(1) - elapsed;
            sleep(wait_time).await;
        }

        *self.last_request.lock() = std::time::Instant::now();
    }
}

#[derive(Debug, Clone, Serialize)]
struct OptimizationResult {
    album_path: String,
    artist: String,
    album: String,
    expected_tracks: usize,
    best_threshold_db: f32,
    best_min_duration_secs: f32,
    best_detected_tracks: usize,
    best_score: f64,
    mbid: String,
}

#[derive(Debug, Clone)]
struct ParameterTest {
    threshold_db: f32,
    min_duration_secs: f32,
}

/// Decode MP3 file to PCM samples
fn decode_mp3(path: &Path) -> Result<(Vec<f32>, u32), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed =
        symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
    let mut format = probed.format;

    let track = format.default_track().ok_or("No default track")?;
    let track_id = track.id;

    let sample_rate = track.codec_params.sample_rate.ok_or("No sample rate")?;

    let decoder_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts)?;

    let mut samples = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => match decoded {
                AudioBufferRef::F32(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample);
                    }
                }
                AudioBufferRef::U8(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push((sample as f32 - 128.0) / 128.0);
                    }
                }
                AudioBufferRef::U16(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push((sample as f32 - 32768.0) / 32768.0);
                    }
                }
                AudioBufferRef::U24(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push((sample.inner() as f32 - 8388608.0) / 8388608.0);
                    }
                }
                AudioBufferRef::U32(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push((sample as f32 - 2147483648.0) / 2147483648.0);
                    }
                }
                AudioBufferRef::S8(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample as f32 / 128.0);
                    }
                }
                AudioBufferRef::S16(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample as f32 / 32768.0);
                    }
                }
                AudioBufferRef::S24(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample.inner() as f32 / 8388608.0);
                    }
                }
                AudioBufferRef::S32(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample as f32 / 2147483648.0);
                    }
                }
                AudioBufferRef::F64(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample as f32);
                    }
                }
            },
            Err(_e) => {
                continue;
            }
        }
    }

    Ok((samples, sample_rate))
}

/// Calculate RMS amplitude in decibels
fn calculate_db(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return -100.0;
    }

    let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

    if rms < 1e-10 {
        -100.0
    } else {
        20.0 * rms.log10()
    }
}

/// Detect silence regions with given parameters
fn detect_silence(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f32,
    min_duration_secs: f32,
) -> Vec<(usize, usize)> {
    let window_size = (sample_rate as f32 * 0.1) as usize; // 100ms
    let window_step = (sample_rate as f32 * 0.05) as usize; // 50ms
    let min_silence_samples = (sample_rate as f32 * min_duration_secs) as usize;

    let mut is_silent = Vec::new();

    for window_start in (0..samples.len()).step_by(window_step) {
        let window_end = (window_start + window_size).min(samples.len());
        let db = calculate_db(&samples[window_start..window_end]);
        is_silent.push(db < threshold_db);
    }

    // Find continuous silence regions
    let mut silence_regions = Vec::new();
    let mut in_silence = false;
    let mut silence_start = 0;

    for (i, &silent) in is_silent.iter().enumerate() {
        if silent && !in_silence {
            silence_start = i * window_step;
            in_silence = true;
        } else if !silent && in_silence {
            let silence_end = i * window_step;
            let duration = silence_end - silence_start;

            if duration >= min_silence_samples {
                silence_regions.push((silence_start, silence_end));
            }
            in_silence = false;
        }
    }

    silence_regions
}

/// Count tracks detected with given parameters
fn count_tracks_detected(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f32,
    min_duration_secs: f32,
) -> usize {
    let silence_regions = detect_silence(samples, sample_rate, threshold_db, min_duration_secs);

    if silence_regions.is_empty() {
        return 1; // Entire file is one track
    }

    silence_regions.len() + 1 // Number of gaps + 1
}

/// Get track durations for given parameters
fn get_track_durations(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f32,
    min_duration_secs: f32,
) -> Vec<f64> {
    let silence_regions = detect_silence(samples, sample_rate, threshold_db, min_duration_secs);

    let mut tracks = Vec::new();
    let mut current_start = 0;

    for (silence_start, silence_end) in silence_regions {
        if silence_start > current_start {
            let duration_secs = (silence_start - current_start) as f64 / sample_rate as f64;
            tracks.push(duration_secs);
        }
        current_start = silence_end;
    }

    // Add final segment
    if current_start < samples.len() {
        let duration_secs = (samples.len() - current_start) as f64 / sample_rate as f64;
        tracks.push(duration_secs);
    }

    tracks
}

/// Calculate score for detected tracks vs expected
fn calculate_score(detected: &[f64], expected: &[u32]) -> f64 {
    if detected.len() != expected.len() {
        // Heavy penalty for track count mismatch
        let count_diff = (detected.len() as i32 - expected.len() as i32).abs();
        return 1000.0 + (count_diff as f64 * 100.0);
    }

    // Mean absolute error for matching track counts
    let mut total_error = 0.0;
    for (det, exp) in detected.iter().zip(expected.iter()) {
        total_error += (det - *exp as f64).abs();
    }

    total_error / detected.len() as f64
}

/// Search MusicBrainz for album and get expected track durations
async fn get_expected_durations(
    artist: &str,
    album: &str,
    rate_limiter: &RateLimiter,
) -> Result<(Vec<u32>, String), Box<dyn std::error::Error>> {
    let query = format!("artist:{} AND release:{}", artist, album);
    let encoded_query = urlencoding::encode(&query);

    let search_url = format!(
        "https://musicbrainz.org/ws/2/release/?query={}&fmt=json&limit=5",
        encoded_query
    );

    rate_limiter.wait().await;

    let client = reqwest::Client::builder()
        .user_agent("WKMP-ParameterOptimizer/0.1 (https://github.com/yourusername/wkmp)")
        .timeout(Duration::from_secs(30))
        .build()?;

    let search_response = client
        .get(&search_url)
        .send()
        .await?
        .json::<MBSearchResponse>()
        .await?;

    if search_response.releases.is_empty() {
        return Err("No releases found".into());
    }

    // Get first release details
    let release = &search_response.releases[0];
    rate_limiter.wait().await;

    let details_url = format!(
        "https://musicbrainz.org/ws/2/release/{}?inc=recordings&fmt=json",
        release.id
    );

    let details = client
        .get(&details_url)
        .send()
        .await?
        .json::<MBReleaseDetails>()
        .await?;

    // Extract track durations
    let mut durations = Vec::new();
    for medium in &details.media {
        for track in &medium.tracks {
            if let Some(length_ms) = track.length {
                durations.push(length_ms / 1000); // Convert to seconds
            }
        }
    }

    Ok((durations, release.id.clone()))
}

/// Extract artist and album from path
fn extract_metadata_from_path(path: &Path) -> (String, String) {
    let path_str = path.to_string_lossy();

    // Extract from pattern: "Music\Artist\Album.mp3"
    let parts: Vec<&str> = path_str.split('\\').collect();

    if parts.len() >= 3 {
        let artist = parts[parts.len() - 2].replace(", ", " ").to_string();
        let album_with_ext = parts[parts.len() - 1];
        let album = album_with_ext
            .trim_end_matches(".mp3")
            .replace(", ", " ")
            .to_string();

        (artist, album)
    } else {
        ("Unknown".to_string(), "Unknown".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read training set
    let training_set_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\training_set.txt");

    let training_content = std::fs::read_to_string(training_set_path)?;
    let mut training_files = Vec::new();

    for line in training_content.lines() {
        if let Some(path_start) = line.find("] ") {
            let path_str = &line[path_start + 2..];
            training_files.push(PathBuf::from(path_str));
        }
    }

    println!("=== Album Parameter Optimization ===");
    println!(
        "Processing {} albums from training set\n",
        training_files.len()
    );

    // Parameter ranges to test
    let thresholds = vec![-50.0, -52.0, -55.0, -57.0, -60.0];
    let min_durations = vec![0.3, 0.4, 0.5, 0.8, 1.0, 1.5, 2.0];

    println!(
        "Testing {} parameter combinations per album",
        thresholds.len() * min_durations.len()
    );
    println!("  Thresholds: {:?} dB", thresholds);
    println!("  Min durations: {:?} seconds\n", min_durations);

    let rate_limiter = RateLimiter::new();
    let mut results = Vec::new();

    for (idx, file_path) in training_files.iter().enumerate() {
        println!("=== Album {}/{} ===", idx + 1, training_files.len());
        println!("File: {}", file_path.display());

        if !file_path.exists() {
            println!("  ERROR: File not found\n");
            continue;
        }

        let (artist, album) = extract_metadata_from_path(file_path);
        println!("  Artist: {}", artist);
        println!("  Album: {}", album);

        // Get expected durations from MusicBrainz
        print!("  Fetching MusicBrainz data... ");
        let (expected_durations, mbid) =
            match get_expected_durations(&artist, &album, &rate_limiter).await {
                Ok(data) => {
                    println!("Found {} tracks", data.0.len());
                    data
                }
                Err(e) => {
                    println!("FAILED: {}\n", e);
                    continue;
                }
            };

        // Decode MP3
        print!("  Decoding... ");
        let (samples, sample_rate) = match decode_mp3(file_path) {
            Ok((s, sr)) => {
                let duration_mins = s.len() as f64 / sr as f64 / 60.0;
                println!(
                    "Done! {} samples at {} Hz ({:.2} mins)",
                    s.len(),
                    sr,
                    duration_mins
                );
                (s, sr)
            }
            Err(e) => {
                println!("FAILED: {}\n", e);
                continue;
            }
        };

        // Run parameter sweep
        println!(
            "  Testing {} parameter combinations...",
            thresholds.len() * min_durations.len()
        );

        let mut best_score = f64::MAX;
        let mut best_threshold = -60.0;
        let mut best_min_duration = 2.0;
        let mut best_detected_tracks = 0;

        for &threshold in &thresholds {
            for &min_duration in &min_durations {
                let detected_durations =
                    get_track_durations(&samples, sample_rate, threshold, min_duration);
                let score = calculate_score(&detected_durations, &expected_durations);

                if score < best_score {
                    best_score = score;
                    best_threshold = threshold;
                    best_min_duration = min_duration;
                    best_detected_tracks = detected_durations.len();
                }
            }
        }

        println!(
            "  BEST: threshold={} dB, min_duration={} s, detected={} tracks, score={:.2}",
            best_threshold, best_min_duration, best_detected_tracks, best_score
        );

        results.push(OptimizationResult {
            album_path: file_path.to_string_lossy().to_string(),
            artist: artist.clone(),
            album: album.clone(),
            expected_tracks: expected_durations.len(),
            best_threshold_db: best_threshold,
            best_min_duration_secs: best_min_duration,
            best_detected_tracks,
            best_score,
            mbid,
        });

        println!();
    }

    // Write results
    let output_path =
        Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\parameter_optimization_results.json");
    println!("\n=== Writing Results ===");
    println!("Output: {}", output_path.display());

    let json = serde_json::to_string_pretty(&results)?;
    std::fs::write(output_path, json)?;

    // Analysis
    println!("\n=== ANALYSIS ===");
    println!("Total albums optimized: {}", results.len());

    if !results.is_empty() {
        // Threshold distribution
        let mut threshold_counts = std::collections::HashMap::new();
        for result in &results {
            *threshold_counts
                .entry(result.best_threshold_db as i32)
                .or_insert(0) += 1;
        }

        println!("\nOptimal Threshold Distribution:");
        let mut threshold_vec: Vec<_> = threshold_counts.iter().collect();
        threshold_vec.sort_by_key(|(k, _)| *k);
        for (threshold, count) in threshold_vec {
            println!(
                "  {} dB: {} albums ({:.1}%)",
                threshold,
                count,
                (*count as f64 / results.len() as f64) * 100.0
            );
        }

        // Min duration distribution
        let mut duration_counts = std::collections::HashMap::new();
        for result in &results {
            let duration_key = (result.best_min_duration_secs * 10.0) as i32; // 0.1s precision
            *duration_counts.entry(duration_key).or_insert(0) += 1;
        }

        println!("\nOptimal Min Duration Distribution:");
        let mut duration_vec: Vec<_> = duration_counts.iter().collect();
        duration_vec.sort_by_key(|(k, _)| *k);
        for (duration_key, count) in duration_vec {
            let duration = *duration_key as f32 / 10.0;
            println!(
                "  {:.1} s: {} albums ({:.1}%)",
                duration,
                count,
                (*count as f64 / results.len() as f64) * 100.0
            );
        }

        // Score statistics
        let scores: Vec<f64> = results.iter().map(|r| r.best_score).collect();
        let avg_score = scores.iter().sum::<f64>() / scores.len() as f64;
        let perfect_matches = results
            .iter()
            .filter(|r| r.best_detected_tracks == r.expected_tracks)
            .count();

        println!("\nScore Statistics:");
        println!("  Average best score: {:.2}", avg_score);
        println!(
            "  Perfect track count matches: {}/{} ({:.1}%)",
            perfect_matches,
            results.len(),
            (perfect_matches as f64 / results.len() as f64) * 100.0
        );
    }

    println!("\nDone!");
    Ok(())
}

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
/// Parameter Validation for Album Matching
///
/// Tests optimal parameters (-57dB, 0.9s) on all training set albums
/// and analyzes track-by-track matching quality
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
    country: Option<String>,
    status: Option<String>,
    packaging: Option<String>,
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
    format: Option<String>,
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
struct TrackMatch {
    detected_duration: f64,
    expected_duration: u32,
    error: f64,
    matches: bool, // Within 10s tolerance
}

#[derive(Debug, Clone, Serialize)]
struct ValidationResult {
    album_path: String,
    artist: String,
    album: String,
    mbid: String,
    expected_track_count: usize,
    detected_track_count: usize,
    perfect_count_match: bool,
    track_matches: Vec<TrackMatch>,
    matched_tracks_count: usize, // Tracks within 10s tolerance
    match_percentage: f64,       // % of expected tracks that matched
    mean_error: f64,
    status: String,
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

/// Compare detected tracks with expected, analyzing match quality
fn analyze_track_matching(
    detected: &[f64],
    expected: &[u32],
    tolerance_secs: f64,
) -> (Vec<TrackMatch>, usize, f64) {
    let mut matches = Vec::new();
    let mut matched_count = 0;

    // Match track-by-track up to the minimum count
    let min_count = detected.len().min(expected.len());

    for i in 0..min_count {
        let error = (detected[i] - expected[i] as f64).abs();
        let is_match = error <= tolerance_secs;

        if is_match {
            matched_count += 1;
        }

        matches.push(TrackMatch {
            detected_duration: detected[i],
            expected_duration: expected[i],
            error,
            matches: is_match,
        });
    }

    // Calculate match percentage based on expected count
    let match_percentage = if expected.is_empty() {
        0.0
    } else {
        (matched_count as f64 / expected.len() as f64) * 100.0
    };

    (matches, matched_count, match_percentage)
}

/// Insert spaces before capital letters in CamelCase strings
fn split_camel_case(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_uppercase() && chars[i - 1].is_lowercase() {
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

/// Apply wildcard to handle common misspellings
fn apply_wildcard_fixes(text: &str) -> Option<String> {
    // Handle "Lizzie" vs "Lizzy" (Thin Lizzie -> Thin Lizz?)
    if text.contains("Lizzie") {
        return Some(text.replace("Lizzie", "Lizz?"));
    }
    None
}

/// Generate search query variants to try in sequence
fn generate_search_queries(artist: &str, album: &str) -> Vec<String> {
    let mut queries = Vec::new();

    // Strategy 1: Original query with type:album filter
    queries.push(format!(
        "type:album AND artist:{} AND release:{}",
        artist, album
    ));

    // Strategy 2: CamelCase split (most effective per test results)
    let album_spaced = split_camel_case(album);
    if album_spaced != album {
        queries.push(format!(
            "type:album AND artist:{} AND release:\"{}\"",
            artist, album_spaced
        ));
    }

    // Strategy 3: Fuzzy matching (catches punctuation differences like "Funk49" -> "Funk #49")
    queries.push(format!(
        "type:album AND artist:{}~ AND release:{}~",
        artist, album
    ));

    // Strategy 4: Targeted wildcard for common misspellings (e.g., "Lizzie" -> "Lizz?")
    if let Some(artist_wildcard) = apply_wildcard_fixes(artist) {
        let album_variant = apply_wildcard_fixes(album).unwrap_or_else(|| album.to_string());
        queries.push(format!(
            "type:album AND artist:{} AND release:{}",
            artist_wildcard, album_variant
        ));
    } else if let Some(album_wildcard) = apply_wildcard_fixes(album) {
        queries.push(format!(
            "type:album AND artist:{} AND release:{}",
            artist, album_wildcard
        ));
    }

    queries
}

/// Search MusicBrainz for album and get expected track durations
/// Multi-stage filtering: 1) Total duration, 2) Track count, 3) Duration pattern
/// Uses multiple search strategies: original, CamelCase split, fuzzy matching
async fn get_expected_durations(
    artist: &str,
    album: &str,
    detected_track_count: usize,
    detected_total_duration: f64,
    rate_limiter: &RateLimiter,
) -> Result<(Vec<u32>, String), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .user_agent("WKMP-ParameterValidator/0.1 (https://github.com/yourusername/wkmp)")
        .timeout(Duration::from_secs(30))
        .build()?;

    // Try multiple search strategies until one succeeds
    let search_queries = generate_search_queries(artist, album);
    let mut search_response = None;

    for (i, query) in search_queries.iter().enumerate() {
        let encoded_query = urlencoding::encode(query);
        let search_url = format!(
            "https://musicbrainz.org/ws/2/release/?query={}&fmt=json&limit=100",
            encoded_query
        );

        rate_limiter.wait().await;

        let response = client
            .get(&search_url)
            .send()
            .await?
            .json::<MBSearchResponse>()
            .await?;

        if !response.releases.is_empty() {
            println!(
                "  MusicBrainz: Found {} results with search strategy {}/{}",
                response.releases.len(),
                i + 1,
                search_queries.len()
            );
            search_response = Some(response);
            break;
        }
    }

    let search_response = search_response.ok_or("No releases found with any search strategy")?;

    // Fetch details for all candidates
    #[derive(Debug)]
    struct Candidate {
        durations: Vec<u32>,
        mbid: String,
        track_count: usize,
        total_duration: u32,
        duration_diff: f64,
        count_diff: i32,
        // Metadata for prioritization
        country: Option<String>,
        status: Option<String>,
        packaging: Option<String>,
        is_cd: bool,
    }

    let mut candidates: Vec<Candidate> = Vec::new();

    for release in search_response.releases.iter().take(30) {
        rate_limiter.wait().await;

        let details_url = format!(
            "https://musicbrainz.org/ws/2/release/{}?inc=recordings&fmt=json",
            release.id
        );

        let details = match client.get(&details_url).send().await {
            Ok(response) => match response.json::<MBReleaseDetails>().await {
                Ok(d) => d,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        // Extract track durations and check for CD format
        let mut durations = Vec::new();
        let mut is_cd = false;
        for medium in &details.media {
            for track in &medium.tracks {
                if let Some(length_ms) = track.length {
                    durations.push(length_ms / 1000); // Convert to seconds
                }
            }
            // Check if this medium is a CD
            if let Some(ref format) = medium.format {
                if format == "CD" {
                    is_cd = true;
                }
            }
        }

        let track_count = durations.len();
        let total_duration: u32 = durations.iter().sum();

        // Calculate fitness scores
        let duration_diff = (total_duration as f64 - detected_total_duration).abs();
        let count_diff = (track_count as i32 - detected_track_count as i32).abs();

        candidates.push(Candidate {
            durations,
            mbid: release.id.clone(),
            track_count,
            total_duration,
            duration_diff,
            count_diff,
            country: release.country.clone(),
            status: release.status.clone(),
            packaging: release.packaging.clone(),
            is_cd,
        });
    }

    if candidates.is_empty() {
        return Err("No valid releases found".into());
    }

    // Stage 1: Filter by total duration (keep candidates within 10% of detected duration)
    let duration_tolerance = detected_total_duration * 0.10; // 10% tolerance
    let duration_filtered: Vec<_> = candidates
        .iter()
        .filter(|c| c.duration_diff <= duration_tolerance)
        .collect();

    // If no candidates within 10%, expand to 20%
    let candidates_to_consider = if duration_filtered.is_empty() {
        let duration_tolerance = detected_total_duration * 0.20;
        candidates
            .iter()
            .filter(|c| c.duration_diff <= duration_tolerance)
            .collect()
    } else {
        duration_filtered
    };

    // If still no candidates, just use all
    let candidates_to_consider: Vec<&Candidate> = if candidates_to_consider.is_empty() {
        candidates.iter().collect()
    } else {
        candidates_to_consider
    };

    // Stage 2: Among duration-matched candidates, sort by composite score
    // Base score = duration_diff (seconds) + count_diff * 60 (penalize track count mismatch heavily)
    // Then apply metadata-based prioritization (subtract to improve score)
    let mut scored: Vec<_> = candidates_to_consider
        .into_iter()
        .map(|c| {
            let mut score = c.duration_diff + (c.count_diff as f64 * 60.0);

            // Prioritize CD releases (-50 points)
            if c.is_cd {
                score -= 50.0;
            }

            // Prioritize US releases (-30 points)
            if c.country.as_deref() == Some("US") {
                score -= 30.0;
            }

            // Prioritize Official status (-40 points)
            if c.status.as_deref() == Some("Official") {
                score -= 40.0;
            }

            (c, score)
        })
        .collect();

    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Return best match
    let (best, _score) = scored.into_iter().next().unwrap();
    Ok((best.durations.clone(), best.mbid.clone()))
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

    println!("=== Parameter Validation ===");
    println!(
        "Testing optimal parameters on {} albums from training set\n",
        training_files.len()
    );

    // Use optimal parameters from analysis
    let threshold_db = -57.0;
    let min_duration_secs = 0.9;
    let match_tolerance_secs = 10.0;

    println!("Parameters:");
    println!("  Threshold: {} dB", threshold_db);
    println!("  Min duration: {} seconds", min_duration_secs);
    println!("  Match tolerance: {} seconds\n", match_tolerance_secs);

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

        // Decode MP3 first to get detected track count for better MusicBrainz matching
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
                println!("FAILED: {}", e);
                results.push(ValidationResult {
                    album_path: file_path.to_string_lossy().to_string(),
                    artist: artist.clone(),
                    album: album.clone(),
                    mbid: String::new(),
                    expected_track_count: 0,
                    detected_track_count: 0,
                    perfect_count_match: false,
                    track_matches: Vec::new(),
                    matched_tracks_count: 0,
                    match_percentage: 0.0,
                    mean_error: 0.0,
                    status: format!("Decode failed: {}", e),
                });
                println!();
                continue;
            }
        };

        // Detect tracks
        print!("  Detecting tracks... ");
        let detected_durations =
            get_track_durations(&samples, sample_rate, threshold_db, min_duration_secs);
        let detected_total: f64 = detected_durations.iter().sum();
        println!(
            "Found {} tracks ({:.1} mins total)",
            detected_durations.len(),
            detected_total / 60.0
        );

        // Get expected durations from MusicBrainz (using total duration + track count for better matching)
        print!("  Fetching MusicBrainz data... ");
        let (expected_durations, mbid) = match get_expected_durations(
            &artist,
            &album,
            detected_durations.len(),
            detected_total,
            &rate_limiter,
        )
        .await
        {
            Ok(data) => {
                println!(
                    "Found {} tracks (best match from {} candidates)",
                    data.0.len(),
                    "multiple"
                );
                data
            }
            Err(e) => {
                println!("FAILED: {}", e);
                results.push(ValidationResult {
                    album_path: file_path.to_string_lossy().to_string(),
                    artist: artist.clone(),
                    album: album.clone(),
                    mbid: String::new(),
                    expected_track_count: 0,
                    detected_track_count: detected_durations.len(),
                    perfect_count_match: false,
                    track_matches: Vec::new(),
                    matched_tracks_count: 0,
                    match_percentage: 0.0,
                    mean_error: 0.0,
                    status: format!("MusicBrainz lookup failed: {}", e),
                });
                println!();
                continue;
            }
        };

        // Analyze matching
        let (track_matches, matched_count, match_percentage) = analyze_track_matching(
            &detected_durations,
            &expected_durations,
            match_tolerance_secs,
        );

        let perfect_count = detected_durations.len() == expected_durations.len();

        let mean_error = if !track_matches.is_empty() {
            track_matches.iter().map(|m| m.error).sum::<f64>() / track_matches.len() as f64
        } else {
            0.0
        };

        println!(
            "  Track count: {}/{} {}",
            detected_durations.len(),
            expected_durations.len(),
            if perfect_count { "✓" } else { "✗" }
        );
        println!(
            "  Matched tracks: {}/{} ({:.1}%)",
            matched_count,
            expected_durations.len(),
            match_percentage
        );
        println!("  Mean error: {:.2}s", mean_error);

        // Show first few track matches
        if !track_matches.is_empty() {
            println!("  First 5 tracks:");
            for (i, tm) in track_matches.iter().enumerate().take(5) {
                let status = if tm.matches { "✓" } else { "✗" };
                println!(
                    "    {}. {:6.1}s vs {:6}s  error={:5.1}s  {}",
                    i + 1,
                    tm.detected_duration,
                    tm.expected_duration,
                    tm.error,
                    status
                );
            }
        }

        results.push(ValidationResult {
            album_path: file_path.to_string_lossy().to_string(),
            artist,
            album,
            mbid,
            expected_track_count: expected_durations.len(),
            detected_track_count: detected_durations.len(),
            perfect_count_match: perfect_count,
            track_matches,
            matched_tracks_count: matched_count,
            match_percentage,
            mean_error,
            status: "Success".to_string(),
        });

        println!();
    }

    // Write results
    let output_path =
        Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\parameter_validation_results.json");
    println!("\n=== Writing Results ===");
    println!("Output: {}", output_path.display());

    let json = serde_json::to_string_pretty(&results)?;
    std::fs::write(output_path, json)?;

    // Analysis
    println!("\n=== VALIDATION ANALYSIS ===");

    let successful = results.iter().filter(|r| r.status == "Success").count();
    println!("Total albums processed: {}", results.len());
    println!("Successfully analyzed: {}", successful);

    if successful > 0 {
        let perfect_counts = results.iter().filter(|r| r.perfect_count_match).count();
        let excellent = results
            .iter()
            .filter(|r| r.match_percentage >= 80.0)
            .count();
        let good = results
            .iter()
            .filter(|r| r.match_percentage >= 60.0 && r.match_percentage < 80.0)
            .count();
        let fair = results
            .iter()
            .filter(|r| r.match_percentage >= 40.0 && r.match_percentage < 60.0)
            .count();
        let poor = results
            .iter()
            .filter(|r| r.match_percentage < 40.0 && r.status == "Success")
            .count();

        println!("\nTrack Count Matches:");
        println!(
            "  Perfect: {}/{} ({:.1}%)",
            perfect_counts,
            successful,
            (perfect_counts as f64 / successful as f64) * 100.0
        );

        println!("\nMatch Quality Distribution:");
        println!("  Excellent (≥80%): {} albums", excellent);
        println!("  Good (60-79%):    {} albums", good);
        println!("  Fair (40-59%):    {} albums", fair);
        println!("  Poor (<40%):      {} albums", poor);

        // Average statistics
        let avg_match_pct = results
            .iter()
            .filter(|r| r.status == "Success")
            .map(|r| r.match_percentage)
            .sum::<f64>()
            / successful as f64;

        let avg_error = results
            .iter()
            .filter(|r| r.status == "Success" && r.mean_error > 0.0)
            .map(|r| r.mean_error)
            .sum::<f64>()
            / successful as f64;

        println!("\nAverage Statistics:");
        println!("  Match percentage: {:.1}%", avg_match_pct);
        println!("  Mean error: {:.2}s", avg_error);

        // Show best and worst
        let mut success_results: Vec<_> =
            results.iter().filter(|r| r.status == "Success").collect();
        success_results
            .sort_by(|a, b| b.match_percentage.partial_cmp(&a.match_percentage).unwrap());

        println!("\nBest 5 Albums:");
        for (i, result) in success_results.iter().take(5).enumerate() {
            println!(
                "  {}. {} - {} ({:.1}%, {}/{} tracks)",
                i + 1,
                result.artist,
                result.album,
                result.match_percentage,
                result.matched_tracks_count,
                result.expected_track_count
            );
        }

        println!("\nWorst 5 Albums:");
        for (i, result) in success_results.iter().rev().take(5).enumerate() {
            println!(
                "  {}. {} - {} ({:.1}%, {}/{} tracks)",
                i + 1,
                result.artist,
                result.album,
                result.match_percentage,
                result.matched_tracks_count,
                result.expected_track_count
            );
        }
    }

    println!("\nDone!");
    Ok(())
}

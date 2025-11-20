/// Comprehensive Album Matcher with Multi-Stage Refinement
///
/// Uses progressive refinement strategy to maximize matching quality:
/// - Stage 1: Try default parameters (-57dB, 0.9s)
/// - Stage 2: Parameter optimization (42 combinations) if match < 80%
/// - Stage 3: Expanded MusicBrainz search if still < 80%
/// - Stage 4: Quiet spot detection if still < 80%
///
/// Tracks which stage succeeded and saves optimal parameters for each album

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use parking_lot::Mutex;

// MusicBrainz API structures
#[derive(Debug, Clone, Deserialize)]
struct MBSearchResponse {
    releases: Vec<MBRelease>,
}

#[derive(Debug, Clone, Deserialize)]
struct MBRelease {
    id: String,
    title: String,
    #[serde(rename = "artist-credit")]
    artist_credit: Option<Vec<MBArtistCredit>>,
    country: Option<String>,
    status: Option<String>,
    packaging: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct MBArtistCredit {
    artist: Option<MBArtist>,
}

#[derive(Debug, Clone, Deserialize)]
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
            last_request: Arc::new(Mutex::new(std::time::Instant::now() - Duration::from_secs(2))),
        }
    }

    async fn wait(&self) {
        let elapsed = {
            let last = self.last_request.lock();
            last.elapsed()
        };

        // MusicBrainz API limit: 1 req/sec
        // Use 2-second delay (0.5 req/sec) for 2x safety margin to prevent timeouts
        if elapsed < Duration::from_secs(2) {
            let wait_time = Duration::from_secs(2) - elapsed;
            sleep(wait_time).await;
        }

        *self.last_request.lock() = std::time::Instant::now();
    }
}

/// Retry a network operation with exponential backoff
/// Attempts: immediate, +5s, +15s, +45s (then gives up)
async fn retry_with_backoff<F, Fut, T, E>(mut operation: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let delays = [0, 5, 15, 45]; // seconds

    for (attempt, &delay) in delays.iter().enumerate() {
        if delay > 0 {
            println!("    Retrying after {} seconds (attempt {}/4)...", delay, attempt + 1);
            sleep(Duration::from_secs(delay)).await;
        }

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt < delays.len() - 1 {
                    println!("    Network error: {} - will retry", e);
                } else {
                    println!("    Network error: {} - giving up after {} attempts", e, delays.len());
                    return Err(e);
                }
            }
        }
    }

    unreachable!()
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
    match_percentage: f64, // % of expected tracks that matched
    mean_error: f64,
    status: String,
    // New fields for multi-stage matching
    matching_stage: String, // "Initial", "Parameter Optimization", "Expanded Search", "Quiet Spot Detection"
    best_threshold_db: Option<f64>,
    best_min_duration_secs: Option<f64>,
    confidence: String, // "Excellent", "Good", "Fair", "Poor"
}

/// Decode MP3 file to PCM samples
fn decode_mp3(path: &Path) -> Result<(Vec<f32>, u32), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
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
            Ok(decoded) => {
                match decoded {
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
                }
            }
            Err(_e) => {
                continue;
            }
        }
    }

    Ok((samples, sample_rate))
}

/// Calculate RMS amplitude
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// Calculate RMS amplitude in decibels
fn calculate_db(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return -100.0;
    }

    let rms = calculate_rms(samples);

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
    // Adaptive RMS window sizing based on min_duration
    // For very short silence periods, we need smaller RMS windows for better temporal resolution
    // Use window = min(0.1s, min_duration / 3) to ensure at least 3 windows per silence period
    let rms_window_secs = if min_duration_secs <= 0.3 {
        // For short durations (≤0.3s): use 25ms window for fine-grained detection
        0.025_f32
    } else if min_duration_secs <= 0.6 {
        // For medium durations (0.3-0.6s): use 50ms window
        0.05_f32
    } else {
        // For long durations (>0.6s): use standard 100ms window
        0.1_f32
    };

    let window_size = (sample_rate as f32 * rms_window_secs) as usize;
    let window_step = (sample_rate as f32 * rms_window_secs * 0.5) as usize; // 50% overlap
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
fn analyze_track_matching(detected: &[f64], expected: &[u32], tolerance_secs: f64) -> (Vec<TrackMatch>, usize, f64) {
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
        if i > 0 && ch.is_uppercase() && chars[i-1].is_lowercase() {
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

/// Apply wildcard to handle common misspellings
fn apply_wildcard_fixes(text: &str) -> Option<String> {
    // Handle "Lizzie" vs "Lizzy" (Thin Lizzie -> Thin Lizz*)
    // Use * instead of ? to match any number of characters (Lizzy, Lizzie, Lizzies, etc.)
    if text.contains("Lizzie") {
        return Some(text.replace("Lizzie", "Lizz*"));
    }
    None
}

/// Check if album name is a reasonable match (fuzzy comparison)
/// Returns true if the MusicBrainz title is similar enough to the expected album
fn is_album_name_match(mb_title: &str, expected_album: &str) -> bool {
    let mb_lower = mb_title.to_lowercase();
    let expected_lower = expected_album.to_lowercase();

    // Exact match (case insensitive)
    if mb_lower == expected_lower {
        return true;
    }

    // Check if one contains the other (handles extra text like "Deluxe Edition")
    if mb_lower.contains(&expected_lower) || expected_lower.contains(&mb_lower) {
        return true;
    }

    // Split into words and check overlap
    let mb_words: Vec<&str> = mb_lower.split_whitespace().collect();
    let expected_words: Vec<&str> = expected_lower.split_whitespace().collect();

    // If all expected words appear in MB title, it's a match
    let all_expected_in_mb = expected_words.iter().all(|word| mb_words.contains(word));
    if all_expected_in_mb {
        return true;
    }

    // If all MB words appear in expected title, it's a match (MB title is subset)
    let all_mb_in_expected = mb_words.iter().all(|word| expected_words.contains(word));
    if all_mb_in_expected {
        return true;
    }

    // Calculate word overlap percentage
    let matching_words = expected_words.iter().filter(|word| mb_words.contains(word)).count();
    let overlap_ratio = matching_words as f64 / expected_words.len().max(1) as f64;

    // Accept if >50% of words match
    overlap_ratio >= 0.5
}

/// Generate search query variants to try in sequence
fn generate_search_queries(artist: &str, album: &str) -> Vec<String> {
    let mut queries = Vec::new();

    // Strategy 1: Original query with type:album filter
    queries.push(format!("type:album AND artist:{} AND release:{}", artist, album));

    // Strategy 2: CamelCase split (most effective per test results)
    let album_spaced = split_camel_case(album);
    if album_spaced != album {
        queries.push(format!("type:album AND artist:{} AND release:\"{}\"", artist, album_spaced));
    }

    // Strategy 3: Fuzzy matching (catches punctuation differences like "Funk49" -> "Funk #49")
    queries.push(format!("type:album AND artist:{}~ AND release:{}~", artist, album));

    // Strategy 4: Targeted wildcard for common misspellings (e.g., "Lizzie" -> "Lizz*")
    if let Some(artist_wildcard) = apply_wildcard_fixes(artist) {
        let album_variant = apply_wildcard_fixes(album).unwrap_or_else(|| album.to_string());
        queries.push(format!("type:album AND artist:{} AND release:{}", artist_wildcard, album_variant));
    } else if let Some(album_wildcard) = apply_wildcard_fixes(album) {
        queries.push(format!("type:album AND artist:{} AND release:{}", artist, album_wildcard));
    }

    // Strategy 5: Aggressive fuzzy search (~2 edits - more tolerant, catches more misspellings)
    queries.push(format!("type:album AND artist:{}~2 AND release:{}~2", artist, album));

    // Strategy 6: Per-token fuzzy matching (handles multi-word names better)
    let artist_tokens: Vec<&str> = artist.split_whitespace().collect();
    let album_tokens: Vec<&str> = album.split_whitespace().collect();
    if artist_tokens.len() > 1 || album_tokens.len() > 1 {
        let artist_fuzzy = artist_tokens.iter().map(|t| format!("{}~", t)).collect::<Vec<_>>().join(" ");
        let album_fuzzy = album_tokens.iter().map(|t| format!("{}~", t)).collect::<Vec<_>>().join(" ");
        queries.push(format!("type:album AND artist:({}) AND release:({})", artist_fuzzy, album_fuzzy));
    }

    // Strategy 7: Album-only fallback (last resort when artist name is problematic)
    queries.push(format!("type:album AND release:{}", album));

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

    // Try multiple search strategies with proper cascading:
    // - Use first strategy that returns album name matches
    // - If a strategy returns results but no album matches, try next strategy
    // - Only fall back to unfiltered results if all strategies fail
    let search_queries = generate_search_queries(artist, album);
    let mut final_response: Option<MBSearchResponse> = None;
    let mut use_album_filter = false;

    for (i, query) in search_queries.iter().enumerate() {
        let encoded_query = urlencoding::encode(query);
        let search_url = format!(
            "https://musicbrainz.org/ws/2/release/?query={}&fmt=json&limit=100",
            encoded_query
        );

        rate_limiter.wait().await;

        // Retry with exponential backoff on network failures
        let response = retry_with_backoff(|| async {
            client
                .get(&search_url)
                .send()
                .await
                .map_err(|e| format!("error sending request for url ({}): {}", search_url, e))?
                .json::<MBSearchResponse>()
                .await
                .map_err(|e| format!("error parsing JSON response: {}", e))
        }).await;

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                println!("  Strategy {}/{} FAILED: {}", i + 1, search_queries.len(), e);
                continue; // Try next search strategy
            }
        };

        if response.releases.is_empty() {
            println!("  Strategy {}/{}: No results", i + 1, search_queries.len());
            continue;
        }

        // Check if this strategy's results match the album name
        let matching_count = response.releases
            .iter()
            .filter(|release| is_album_name_match(&release.title, album))
            .count();

        if matching_count > 0 {
            // Found usable matches - use this strategy's results with album filter
            println!("  MusicBrainz: Found {} results with search strategy {}/{}, {} match album name",
                     response.releases.len(), i + 1, search_queries.len(), matching_count);
            final_response = Some(response);
            use_album_filter = true;
            break;
        } else {
            // This strategy returned results but none match album name - save as fallback
            println!("  Strategy {}/{}: {} results but no album name matches, trying next strategy",
                     i + 1, search_queries.len(), response.releases.len());
            if final_response.is_none() {
                final_response = Some(response);
            }
        }
    }

    let search_response = final_response.ok_or("No releases found with any search strategy")?;

    // Apply album name filter if appropriate strategy was found
    let releases_to_fetch: Vec<&MBRelease> = if use_album_filter {
        search_response.releases
            .iter()
            .filter(|release| is_album_name_match(&release.title, album))
            .take(50)
            .collect()
    } else {
        // Fallback: use first 30 from last strategy that returned any results
        println!("  No strategy found album name matches, falling back to first 30 from last response");
        search_response.releases.iter().take(30).collect()
    };

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

    // Fetch details for all matching releases (instead of just first 30)
    for release in releases_to_fetch.iter() {
        rate_limiter.wait().await;

        let details_url = format!(
            "https://musicbrainz.org/ws/2/release/{}?inc=recordings&fmt=json",
            release.id
        );

        // Retry with exponential backoff on network failures
        let details = retry_with_backoff(|| async {
            client
                .get(&details_url)
                .send()
                .await
                .map_err(|e| format!("error fetching release details: {}", e))?
                .json::<MBReleaseDetails>()
                .await
                .map_err(|e| format!("error parsing release details: {}", e))
        }).await;

        let details = match details {
            Ok(d) => d,
            Err(_) => continue, // Skip this release if retry fails
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

/// Assemble segments into tracks using dynamic programming
/// Used in Stage 3 when over-segmentation is detected (detected_segments > expected_tracks)
///
/// Algorithm: Dynamic programming to find optimal grouping of adjacent segments
/// dp[i][j] = minimum duration error when grouping first i segments into j tracks
fn assemble_segments_dp(
    detected_durations: &[f64],
    expected_durations: &[u32],
) -> Option<Vec<f64>> {
    let n = detected_durations.len();
    let k = expected_durations.len();

    // Only assemble if we have over-segmentation
    if n <= k {
        return None;
    }

    // dp[i][j] = (min_error, split_positions)
    // where min_error is the minimum total duration error when grouping first i segments into j tracks
    // and split_positions stores where to split the segments
    let mut dp: Vec<Vec<(f64, Vec<usize>)>> = vec![vec![(f64::INFINITY, Vec::new()); k + 1]; n + 1];

    // Base case: 0 segments into 0 tracks = 0 error
    dp[0][0] = (0.0, Vec::new());

    // Fill DP table
    for i in 1..=n {
        for j in 1..=k.min(i) {
            // Try all possible positions for the j-th track's start
            for start in (j-1)..i {
                if dp[start][j-1].0 == f64::INFINITY {
                    continue;
                }

                // Calculate duration of track j by summing segments from start to i-1
                let track_duration: f64 = detected_durations[start..i].iter().sum();
                let expected_duration = expected_durations[j - 1] as f64; // Convert milliseconds to seconds for comparison
                let duration_error = (track_duration - expected_duration).abs();

                // Total error = previous error + this track's error
                let total_error = dp[start][j-1].0 + duration_error;

                // Update if this is better
                if total_error < dp[i][j].0 {
                    let mut new_splits = dp[start][j-1].1.clone();
                    new_splits.push(start);
                    dp[i][j] = (total_error, new_splits);
                }
            }
        }
    }

    // Extract solution if valid
    if dp[n][k].0 == f64::INFINITY {
        return None;
    }

    let splits = &dp[n][k].1;
    let mut assembled_durations = Vec::new();

    // Reconstruct track durations from split positions
    let mut prev_split = 0;
    for &split_pos in splits.iter().skip(1) {
        let track_duration: f64 = detected_durations[prev_split..split_pos].iter().sum();
        assembled_durations.push(track_duration);
        prev_split = split_pos;
    }

    // Final track
    let final_duration: f64 = detected_durations[prev_split..n].iter().sum();
    assembled_durations.push(final_duration);

    Some(assembled_durations)
}

/// Detect quiet spots (local minima 1 std dev below mean RMS)
/// Used in Stage 4 when silence-based detection fails
fn detect_quiet_spots(
    samples: &[f32],
    sample_rate: u32,
    expected_track_count: usize,
) -> Vec<usize> {
    let window_size = (sample_rate as f64 * 0.5) as usize; // 500ms windows

    // Calculate RMS for each window
    let mut rms_values = Vec::new();
    for i in (0..samples.len()).step_by(window_size / 4) {
        let end = (i + window_size).min(samples.len());
        let rms = calculate_rms(&samples[i..end]);
        rms_values.push((i, rms));
    }

    if rms_values.is_empty() {
        return Vec::new();
    }

    // Calculate mean and std dev
    let mean: f32 = rms_values.iter().map(|(_, rms)| rms).sum::<f32>() / rms_values.len() as f32;
    let variance: f32 = rms_values.iter()
        .map(|(_, rms)| {
            let diff = rms - mean;
            diff * diff
        })
        .sum::<f32>() / rms_values.len() as f32;
    let std_dev = variance.sqrt();

    let quiet_threshold = mean - std_dev;

    // Find local minima below threshold
    let mut quiet_spots = Vec::new();
    for i in 1..rms_values.len() - 1 {
        let (pos, rms) = rms_values[i];
        let prev_rms = rms_values[i - 1].1;
        let next_rms = rms_values[i + 1].1;

        // Local minimum and below threshold
        if rms < prev_rms && rms < next_rms && rms < quiet_threshold {
            quiet_spots.push(pos);
        }
    }

    // Sort by quietness and take N quietest where N = expected_track_count - 1
    quiet_spots.sort_by(|&a, &b| {
        let rms_a = calculate_rms(&samples[a..(a + window_size).min(samples.len())]);
        let rms_b = calculate_rms(&samples[b..(b + window_size).min(samples.len())]);
        rms_a.partial_cmp(&rms_b).unwrap()
    });

    if expected_track_count > 0 {
        quiet_spots.truncate(expected_track_count - 1);
        quiet_spots.sort(); // Sort by position
    }

    quiet_spots
}

/// Convert quiet spot positions to track durations
fn quiet_spots_to_durations(
    quiet_spots: &[usize],
    total_samples: usize,
    sample_rate: u32,
) -> Vec<f64> {
    let mut durations = Vec::new();
    let mut track_start = 0;

    for &spot in quiet_spots {
        let duration = (spot - track_start) as f64 / sample_rate as f64;
        durations.push(duration);
        track_start = spot;
    }

    // Final track
    if track_start < total_samples {
        let duration = (total_samples - track_start) as f64 / sample_rate as f64;
        durations.push(duration);
    }

    durations
}

/// Classify match quality as confidence level
fn classify_confidence(match_percentage: f64) -> String {
    if match_percentage >= 80.0 {
        "Excellent".to_string()
    } else if match_percentage >= 60.0 {
        "Good".to_string()
    } else if match_percentage >= 40.0 {
        "Fair".to_string()
    } else {
        "Poor".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Comprehensive Album Matcher ===");
    println!("Multi-stage matching: Initial -> Parameter Opt -> Segment Assembly -> Quiet Spots\n");

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
    println!("Testing optimal parameters on {} albums from training set\n", training_files.len());

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

    // Parameter grid for Stage 2 optimization
    // Extended ranges based on empirical results showing optimal performance at grid boundaries
    // - Added -48, -46, -44 dB (less strict thresholds) after finding optima at -50 dB
    // - Added 0.2, 0.15, 0.1s (shorter durations) after finding optima at 0.3s
    // - Added 0.12, 0.10, 0.08, 0.06, 0.05s (very short durations) after finding optima at 0.15s
    //   With adaptive 25ms RMS windows, these very short durations now have adequate temporal resolution
    // - Added -64, -66 dB (stricter thresholds) for completeness
    // - Added 2.5, 3.0s (longer durations) for completeness
    let threshold_values = [-44.0, -46.0, -48.0, -50.0, -52.0, -54.0, -56.0, -58.0, -60.0, -62.0, -64.0, -66.0];
    let min_duration_values = [0.05, 0.06, 0.08, 0.10, 0.12, 0.15, 0.2, 0.3, 0.5, 0.8, 1.0, 1.5, 2.0, 2.5, 3.0];

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

        // Decode MP3
        print!("  Decoding... ");
        let (samples, sample_rate) = match decode_mp3(file_path) {
            Ok((s, sr)) => {
                let duration_mins = s.len() as f64 / sr as f64 / 60.0;
                println!("Done! {} samples at {} Hz ({:.2} mins)", s.len(), sr, duration_mins);
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
                    matching_stage: "None".to_string(),
                    best_threshold_db: None,
                    best_min_duration_secs: None,
                    confidence: "Poor".to_string(),
                });
                println!();
                continue;
            }
        };

        // === STAGE 1: Try default parameters ===
        println!("  STAGE 1: Testing default parameters (-57dB, 0.9s)...");
        let stage1_durations = get_track_durations(&samples, sample_rate, threshold_db, min_duration_secs);
        let stage1_total: f64 = stage1_durations.iter().sum();
        println!("    Found {} tracks", stage1_durations.len());

        // Get expected durations from MusicBrainz
        print!("  Fetching MusicBrainz data... ");
        let (expected_durations, mbid) = match get_expected_durations(
            &artist, &album, stage1_durations.len(), stage1_total, &rate_limiter
        ).await {
            Ok(data) => {
                println!("Found {} tracks", data.0.len());
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
                    detected_track_count: stage1_durations.len(),
                    perfect_count_match: false,
                    track_matches: Vec::new(),
                    matched_tracks_count: 0,
                    match_percentage: 0.0,
                    mean_error: 0.0,
                    status: format!("MusicBrainz lookup failed: {}", e),
                    matching_stage: "None".to_string(),
                    best_threshold_db: None,
                    best_min_duration_secs: None,
                    confidence: "Poor".to_string(),
                });
                println!();
                continue;
            }
        };

        // Analyze Stage 1 results
        let (stage1_matches, stage1_matched_count, stage1_percentage) =
            analyze_track_matching(&stage1_durations, &expected_durations, match_tolerance_secs);

        println!("    Match quality: {:.1}% ({} confidence)",
            stage1_percentage, classify_confidence(stage1_percentage));

        // Track best result across all stages
        let mut best_durations = stage1_durations.clone();
        let mut best_matches = stage1_matches.clone();
        let mut best_matched_count = stage1_matched_count;
        let mut best_percentage = stage1_percentage;
        let mut best_stage = "Initial";
        let mut best_threshold: Option<f64> = None;
        let mut best_min_duration: Option<f64> = None;

        // === STAGE 2: Parameter optimization (if needed) ===
        if best_percentage < 100.0 {
            println!("  STAGE 2: Testing {} parameter combinations...", threshold_values.len() * min_duration_values.len());
            let mut tested = 0;

            for &thresh in &threshold_values {
                for &min_dur in &min_duration_values {
                    tested += 1;
                    let test_durations = get_track_durations(&samples, sample_rate, thresh, min_dur);
                    let (test_matches, test_matched_count, test_percentage) =
                        analyze_track_matching(&test_durations, &expected_durations, match_tolerance_secs);

                    if test_percentage > best_percentage {
                        best_durations = test_durations;
                        best_matches = test_matches;
                        best_matched_count = test_matched_count;
                        best_percentage = test_percentage;
                        best_stage = "Parameter Optimization";
                        best_threshold = Some(thresh as f64);
                        best_min_duration = Some(min_dur as f64);

                        println!("    New best: {:.1}% with {}dB, {}s ({}/{})",
                            best_percentage, thresh, min_dur, tested,
                            threshold_values.len() * min_duration_values.len());

                        // Only stop if we found perfect match (100%)
                        if best_percentage >= 100.0 {
                            break;
                        }
                    }
                }
                if best_percentage >= 100.0 {
                    break;
                }
            }

            println!("    Best match after parameter optimization: {:.1}% ({} confidence)",
                best_percentage, classify_confidence(best_percentage));
        }

        // === STAGE 3: Segment Assembly (if over-segmentation detected) ===
        if best_percentage < 100.0 && best_durations.len() > expected_durations.len() {
            println!("  STAGE 3: Attempting segment assembly (over-segmentation detected: {} segments, {} expected)...",
                best_durations.len(), expected_durations.len());

            if let Some(assembled_durations) = assemble_segments_dp(&best_durations, &expected_durations) {
                let (assembled_matches, assembled_matched_count, assembled_percentage) =
                    analyze_track_matching(&assembled_durations, &expected_durations, match_tolerance_secs);

                println!("    Assembled {} segments into {} tracks", best_durations.len(), assembled_durations.len());
                println!("    Assembly match quality: {:.1}%", assembled_percentage);

                if assembled_percentage > best_percentage {
                    best_durations = assembled_durations;
                    best_matches = assembled_matches;
                    best_matched_count = assembled_matched_count;
                    best_percentage = assembled_percentage;
                    best_stage = "Segment Assembly";
                    println!("    Segment assembly improved match to {:.1}%", best_percentage);
                } else {
                    println!("    Segment assembly: {:.1}% (not better than current best)", assembled_percentage);
                }
            } else {
                println!("    Segment assembly failed to find valid grouping");
            }
        }

        // === STAGE 4: Quiet spot detection (if needed) ===
        if best_percentage < 100.0 && expected_durations.len() > 0 {
            println!("  STAGE 4: Attempting quiet spot detection...");
            let quiet_spots = detect_quiet_spots(&samples, sample_rate, expected_durations.len());
            let quiet_durations = quiet_spots_to_durations(&quiet_spots, samples.len(), sample_rate);

            println!("    Found {} quiet spots", quiet_spots.len());

            let (quiet_matches, quiet_matched_count, quiet_percentage) =
                analyze_track_matching(&quiet_durations, &expected_durations, match_tolerance_secs);

            if quiet_percentage > best_percentage {
                best_durations = quiet_durations;
                best_matches = quiet_matches;
                best_matched_count = quiet_matched_count;
                best_percentage = quiet_percentage;
                best_stage = "Quiet Spot Detection";
                println!("    Quiet spot detection improved match to {:.1}%", best_percentage);
            } else {
                println!("    Quiet spot detection: {:.1}% (not better than current best)", quiet_percentage);
            }
        }

        // Calculate final statistics
        let perfect_count = best_durations.len() == expected_durations.len();
        let mean_error = if !best_matches.is_empty() {
            best_matches.iter().map(|m| m.error).sum::<f64>() / best_matches.len() as f64
        } else {
            0.0
        };

        println!("\n  FINAL RESULT:");
        println!("    Matching stage: {}", best_stage);
        println!("    Track count: {}/{} {}", best_durations.len(), expected_durations.len(),
            if perfect_count { "✓" } else { "✗" });
        println!("    Matched tracks: {}/{} ({:.1}%)", best_matched_count, expected_durations.len(), best_percentage);
        println!("    Mean error: {:.2}s", mean_error);
        println!("    Confidence: {}", classify_confidence(best_percentage));

        if let (Some(thresh), Some(min_dur)) = (best_threshold, best_min_duration) {
            println!("    Best parameters: {}dB, {}s", thresh, min_dur);
        }

        // Show first few track matches
        if !best_matches.is_empty() {
            println!("  First 5 tracks:");
            for (i, tm) in best_matches.iter().enumerate().take(5) {
                let status = if tm.matches { "✓" } else { "✗" };
                println!("    {}. {:6.1}s vs {:6}s  error={:5.1}s  {}",
                    i + 1, tm.detected_duration, tm.expected_duration, tm.error, status);
            }
        }

        results.push(ValidationResult {
            album_path: file_path.to_string_lossy().to_string(),
            artist,
            album,
            mbid,
            expected_track_count: expected_durations.len(),
            detected_track_count: best_durations.len(),
            perfect_count_match: perfect_count,
            track_matches: best_matches,
            matched_tracks_count: best_matched_count,
            match_percentage: best_percentage,
            mean_error,
            status: "Success".to_string(),
            matching_stage: best_stage.to_string(),
            best_threshold_db: best_threshold,
            best_min_duration_secs: best_min_duration,
            confidence: classify_confidence(best_percentage),
        });

        println!();
    }

    // Write results
    let output_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\album_matcher_results.json");
    println!("\n=== Writing Results ===");
    println!("Output: {}", output_path.display());

    let json = serde_json::to_string_pretty(&results)?;
    std::fs::write(output_path, json)?;

    // Analysis
    println!("\n=== COMPREHENSIVE MATCHING ANALYSIS ===");

    let successful = results.iter().filter(|r| r.status == "Success").count();
    println!("Total albums processed: {}", results.len());
    println!("Successfully analyzed: {}", successful);

    if successful > 0 {
        // Matching stage breakdown
        let stage1 = results.iter().filter(|r| r.matching_stage == "Initial").count();
        let stage2 = results.iter().filter(|r| r.matching_stage == "Parameter Optimization").count();
        let stage3 = results.iter().filter(|r| r.matching_stage == "Expanded Search").count();
        let stage4 = results.iter().filter(|r| r.matching_stage == "Quiet Spot Detection").count();

        println!("\nMatching Stage Results:");
        println!("  Stage 1 (Initial):              {} albums", stage1);
        println!("  Stage 2 (Parameter Opt):        {} albums", stage2);
        println!("  Stage 3 (Expanded Search):      {} albums", stage3);
        println!("  Stage 4 (Quiet Spot Detection): {} albums", stage4);

        // Confidence level breakdown
        let excellent = results.iter().filter(|r| r.confidence == "Excellent").count();
        let good = results.iter().filter(|r| r.confidence == "Good").count();
        let fair = results.iter().filter(|r| r.confidence == "Fair").count();
        let poor = results.iter().filter(|r| r.confidence == "Poor" && r.status == "Success").count();

        println!("\nConfidence Distribution:");
        println!("  Excellent (≥80%): {} albums", excellent);
        println!("  Good (60-79%):    {} albums", good);
        println!("  Fair (40-59%):    {} albums", fair);
        println!("  Poor (<40%):      {} albums", poor);

        // Track count matches
        let perfect_counts = results.iter().filter(|r| r.perfect_count_match).count();
        println!("\nTrack Count Matches:");
        println!("  Perfect: {}/{} ({:.1}%)", perfect_counts, successful,
            (perfect_counts as f64 / successful as f64) * 100.0);

        // Average statistics
        let avg_match_pct = results.iter()
            .filter(|r| r.status == "Success")
            .map(|r| r.match_percentage)
            .sum::<f64>() / successful as f64;

        let avg_error = results.iter()
            .filter(|r| r.status == "Success" && r.mean_error > 0.0)
            .map(|r| r.mean_error)
            .sum::<f64>() / successful as f64;

        println!("\nAverage Statistics:");
        println!("  Match percentage: {:.1}%", avg_match_pct);
        println!("  Mean error: {:.2}s", avg_error);

        // Parameter effectiveness (for Stage 2 results)
        if stage2 > 0 {
            println!("\nParameter Optimization Details:");
            for result in results.iter().filter(|r| r.matching_stage == "Parameter Optimization") {
                if let (Some(thresh), Some(min_dur)) = (result.best_threshold_db, result.best_min_duration_secs) {
                    println!("  {} - {}: {}dB, {}s → {:.1}%",
                        result.artist, result.album, thresh, min_dur, result.match_percentage);
                }
            }
        }

        // Show best and worst
        let mut success_results: Vec<_> = results.iter().filter(|r| r.status == "Success").collect();
        success_results.sort_by(|a, b| b.match_percentage.partial_cmp(&a.match_percentage).unwrap());

        println!("\nBest 5 Albums:");
        for (i, result) in success_results.iter().take(5).enumerate() {
            println!("  {}. {} - {} ({:.1}%, {} via {})",
                i + 1, result.artist, result.album, result.match_percentage,
                result.confidence, result.matching_stage);
        }

        println!("\nWorst 5 Albums:");
        for (i, result) in success_results.iter().rev().take(5).enumerate() {
            println!("  {}. {} - {} ({:.1}%, {} via {})",
                i + 1, result.artist, result.album, result.match_percentage,
                result.confidence, result.matching_stage);
        }
    }

    println!("\nDone!");
    Ok(())
}

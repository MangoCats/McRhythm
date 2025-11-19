use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use parking_lot::Mutex;

/// Detected track segment
#[derive(Debug, Clone)]
struct Track {
    start_sample: u64,
    end_sample: u64,
    duration_secs: f64,
}

/// MusicBrainz release candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseCandidate {
    mbid: String,
    title: String,
    artist: String,
    track_count: usize,
    track_durations: Vec<u32>, // in seconds
    total_duration: u32,
}

// MusicBrainz API response structures
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

/// Rate limiter for MusicBrainz API (1 request per second)
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

        if elapsed < Duration::from_secs(1) {
            let wait_time = Duration::from_secs(1) - elapsed;
            sleep(wait_time).await;
        }

        *self.last_request.lock() = std::time::Instant::now();
    }
}

/// Match result for an album
#[derive(Debug, Clone, Serialize)]
struct MatchResult {
    file_path: String,
    detected_tracks: usize,
    detected_durations: Vec<f64>,
    total_detected_duration: f64,
    best_match_mbid: Option<String>,
    best_match_title: Option<String>,
    best_match_artist: Option<String>,
    best_match_track_count: Option<usize>,
    mean_duration_error: Option<f64>,
    confidence: String, // "Excellent", "Good", "Fair", "Poor", "No Match"
    status: String, // "Success", "Failed to decode", "No candidates", etc.
}

/// Calculate RMS (root mean square) amplitude and convert to dB
fn calculate_db(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return -100.0;
    }

    let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

    if rms > 0.0 {
        20.0 * rms.log10()
    } else {
        -100.0
    }
}

/// Decode MP3 file into mono samples
fn decode_mp3(file_path: &Path) -> Result<(Vec<f32>, u32), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(file_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = file_path.extension() {
        hint.with_extension(ext.to_str().unwrap_or("mp3"));
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
    let mut format = probed.format;

    let track = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("No audio track found")?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.ok_or("No sample rate")?;

    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet)? {
            AudioBufferRef::F32(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx];
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::U8(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        let sample = (buf.chan(ch)[frame_idx] as f32 - 128.0) / 128.0;
                        sum += sample;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::U16(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        let sample = (buf.chan(ch)[frame_idx] as f32 - 32768.0) / 32768.0;
                        sum += sample;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::U24(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        let sample = (buf.chan(ch)[frame_idx].inner() as f32 - 8388608.0) / 8388608.0;
                        sum += sample;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::U32(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        let sample = (buf.chan(ch)[frame_idx] as f32 - 2147483648.0) / 2147483648.0;
                        sum += sample;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::S8(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx] as f32 / 128.0;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::S16(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx] as f32 / 32768.0;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::S24(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx].inner() as f32 / 8388608.0;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::S32(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx] as f32 / 2147483648.0;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
            AudioBufferRef::F64(buf) => {
                let channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += buf.chan(ch)[frame_idx] as f32;
                    }
                    all_samples.push(sum / channels as f32);
                }
            }
        }
    }

    Ok((all_samples, sample_rate))
}

/// Detect silence and extract track boundaries
fn detect_tracks(
    samples: &[f32],
    sample_rate: u32,
) -> Result<Vec<Track>, Box<dyn std::error::Error>> {
    // Use empirically-optimized parameters from silence analysis
    // Middle-ground parameters that work for both compilations and studio albums
    let threshold_db = -55.0;  // More sensitive than -60dB for studio albums
    let min_duration_secs = 1.0;  // Between 0.4s (studio) and 2.0s (compilation)
    let window_size = (sample_rate as f32 * 0.1) as usize; // 100ms
    let window_step = (sample_rate as f32 * 0.05) as usize; // 50ms (50% overlap)
    let min_silence_samples = (sample_rate as f32 * min_duration_secs) as usize;

    let mut is_silent = Vec::new();

    for window_start in (0..samples.len()).step_by(window_step) {
        let window_end = (window_start + window_size).min(samples.len());
        let db = calculate_db(&samples[window_start..window_end]);
        is_silent.push(db < threshold_db);
    }

    // Find continuous silence regions
    let mut silence_regions: Vec<(usize, usize)> = Vec::new();
    let mut silence_start = None;

    for (idx, &silent) in is_silent.iter().enumerate() {
        match (silent, silence_start) {
            (true, None) => silence_start = Some(idx),
            (false, Some(start)) => {
                let duration_windows = idx - start;
                let duration_samples = duration_windows * window_step;
                if duration_samples >= min_silence_samples {
                    silence_regions.push((start * window_step, idx * window_step));
                }
                silence_start = None;
            }
            _ => {}
        }
    }

    // Handle case where silence extends to end
    if let Some(start) = silence_start {
        let idx = is_silent.len();
        let duration_windows = idx - start;
        let duration_samples = duration_windows * window_step;
        if duration_samples >= min_silence_samples {
            silence_regions.push((start * window_step, idx * window_step));
        }
    }

    // Convert silence regions to track segments
    let mut tracks = Vec::new();
    let mut current_start = 0u64;

    for (silence_start, silence_end) in silence_regions {
        if silence_start > current_start as usize {
            tracks.push(Track {
                start_sample: current_start,
                end_sample: silence_start as u64,
                duration_secs: (silence_start as u64 - current_start) as f64 / sample_rate as f64,
            });
        }
        current_start = silence_end as u64;
    }

    // Add final segment if exists
    if (current_start as usize) < samples.len() {
        tracks.push(Track {
            start_sample: current_start,
            end_sample: samples.len() as u64,
            duration_secs: (samples.len() as u64 - current_start) as f64 / sample_rate as f64,
        });
    }

    Ok(tracks)
}

/// Extract artist and album name from file path
fn extract_metadata_from_path(file_path: &Path) -> (String, String) {
    let path_str = file_path.to_string_lossy();
    let components: Vec<&str> = path_str.split('\\').collect();

    // Expected format: ...\\Artist\\AlbumName.mp3
    if components.len() >= 2 {
        let artist = components[components.len() - 2].to_string();
        let album = components[components.len() - 1]
            .trim_end_matches(".mp3")
            .trim_end_matches(".MP3")
            .to_string();
        (artist, album)
    } else {
        ("Unknown".to_string(), "Unknown".to_string())
    }
}

/// Search MusicBrainz for matching releases
async fn search_musicbrainz(
    artist: &str,
    album: &str,
    track_count: usize,
    rate_limiter: &RateLimiter,
) -> Result<Vec<ReleaseCandidate>, Box<dyn std::error::Error>> {
    println!("  Searching MusicBrainz for: {} - {} ({} tracks)", artist, album, track_count);

    // Build search query
    let query = format!("artist:{} AND release:{}", artist, album);
    let encoded_query = urlencoding::encode(&query);

    let search_url = format!(
        "https://musicbrainz.org/ws/2/release/?query={}&fmt=json&limit=10",
        encoded_query
    );

    // Rate limit
    rate_limiter.wait().await;

    // Search for releases
    let client = reqwest::Client::builder()
        .user_agent("WKMP-AlbumMatcher/0.1 (https://github.com/yourusername/wkmp)")
        .timeout(Duration::from_secs(30))
        .build()?;

    let search_response = client
        .get(&search_url)
        .send()
        .await?
        .json::<MBSearchResponse>()
        .await?;

    println!("    Found {} potential releases", search_response.releases.len());

    let mut candidates: Vec<ReleaseCandidate> = Vec::new();

    // Fetch details for each release
    for (idx, release) in search_response.releases.iter().enumerate().take(10) {
        // Check first 10 releases for better pattern matching disambiguation
        rate_limiter.wait().await;

        let details_url = format!(
            "https://musicbrainz.org/ws/2/release/{}?inc=recordings&fmt=json",
            release.id
        );

        let details_result = client.get(&details_url).send().await;
        let details = match details_result {
            Ok(response) => {
                match response.json::<MBReleaseDetails>().await {
                    Ok(d) => d,
                    Err(e) => {
                        println!("    WARNING: Failed to parse release {}: {}", idx + 1, e);
                        continue;
                    }
                }
            }
            Err(e) => {
                println!("    WARNING: Failed to fetch release {}: {}", idx + 1, e);
                continue;
            }
        };

        // Extract track durations
        let mut track_durations: Vec<u32> = Vec::new();
        for medium in &details.media {
            for track in &medium.tracks {
                if let Some(length_ms) = track.length {
                    track_durations.push(length_ms / 1000); // Convert ms to seconds
                } else {
                    // Unknown duration - use 0 as placeholder
                    track_durations.push(0);
                }
            }
        }

        let total_tracks = track_durations.len();

        // Filter by track count (allow ±2 tracks tolerance)
        if total_tracks < track_count.saturating_sub(2) || total_tracks > track_count + 2 {
            println!("    Skipping release {} (has {} tracks, expected ~{})", idx + 1, total_tracks, track_count);
            continue;
        }

        let total_duration: u32 = track_durations.iter().sum();

        // Extract artist name
        let artist_name = release
            .artist_credit
            .as_ref()
            .and_then(|credits| credits.first())
            .and_then(|credit| credit.artist.as_ref())
            .map(|artist| artist.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        candidates.push(ReleaseCandidate {
            mbid: release.id.clone(),
            title: details.title.clone(),
            artist: artist_name,
            track_count: total_tracks,
            track_durations,
            total_duration,
        });

        println!("    Added candidate: {} - {} ({} tracks, MBID: {})",
            candidates.last().unwrap().artist,
            candidates.last().unwrap().title,
            total_tracks,
            release.id
        );
    }

    println!("    Retained {} candidates after filtering", candidates.len());
    Ok(candidates)
}

/// Calculate match score between detected tracks and a release candidate
/// Uses multi-metric pattern matching for sophisticated disambiguation
fn calculate_match_score(detected: &[Track], candidate: &ReleaseCandidate) -> (f64, String) {
    // Handle track count mismatch with partial matching penalty
    let track_count_penalty = if detected.len() == candidate.track_count {
        0.0
    } else {
        let diff = (detected.len() as i32 - candidate.track_count as i32).abs();
        100.0 * diff as f64  // 100s penalty per missing/extra track
    };

    // If track counts are very different (>3), apply severe penalty but continue evaluation
    if (detected.len() as i32 - candidate.track_count as i32).abs() > 3 {
        return (10000.0 + track_count_penalty, "Poor".to_string());
    }

    let n = detected.len().min(candidate.track_count);

    // Metric 1: Mean Absolute Error (classic duration matching)
    let mut mae = 0.0;
    for (track, &expected_duration) in detected.iter().zip(candidate.track_durations.iter()) {
        mae += (track.duration_secs - expected_duration as f64).abs();
    }
    mae /= n as f64;

    // Metric 2: Total Duration Error (album-level matching)
    let detected_total: f64 = detected.iter().map(|t| t.duration_secs).sum();
    let candidate_total = candidate.total_duration as f64;
    let total_duration_error = (detected_total - candidate_total).abs();

    // Metric 3: Pattern Correlation (track duration sequence similarity)
    // Use Pearson correlation coefficient to measure pattern shape
    let detected_durations: Vec<f64> = detected.iter().map(|t| t.duration_secs).collect();
    let candidate_durations: Vec<f64> = candidate.track_durations.iter().map(|&d| d as f64).collect();

    let det_mean = detected_durations.iter().sum::<f64>() / n as f64;
    let can_mean = candidate_durations.iter().sum::<f64>() / n as f64;

    let mut numerator = 0.0;
    let mut det_sq_sum = 0.0;
    let mut can_sq_sum = 0.0;

    for i in 0..n {
        let det_diff = detected_durations[i] - det_mean;
        let can_diff = candidate_durations[i] - can_mean;
        numerator += det_diff * can_diff;
        det_sq_sum += det_diff * det_diff;
        can_sq_sum += can_diff * can_diff;
    }

    let correlation = if det_sq_sum > 0.0 && can_sq_sum > 0.0 {
        numerator / (det_sq_sum.sqrt() * can_sq_sum.sqrt())
    } else {
        0.0
    };

    // Convert correlation to error metric (1.0 = perfect, 0.0 = no correlation)
    let correlation_error = (1.0 - correlation) * 100.0; // Scale to comparable range

    // Metric 4: Relative Duration Ratios (track-to-track pattern)
    // Compare ratios between consecutive tracks
    let mut ratio_error = 0.0;
    if n > 1 {
        for i in 1..n {
            let det_ratio = detected_durations[i] / detected_durations[i - 1];
            let can_ratio = candidate_durations[i] / candidate_durations[i - 1];
            ratio_error += (det_ratio - can_ratio).abs();
        }
        ratio_error /= (n - 1) as f64;
        ratio_error *= 10.0; // Scale to comparable range
    }

    // Metric 5: Position-Weighted MAE (longer tracks weighted higher)
    let mut weighted_error = 0.0;
    let mut total_weight = 0.0;
    for (track, &expected_duration) in detected.iter().zip(candidate.track_durations.iter()) {
        let weight = expected_duration as f64; // Longer tracks = higher weight
        weighted_error += (track.duration_secs - expected_duration as f64).abs() * weight;
        total_weight += weight;
    }
    if total_weight > 0.0 {
        weighted_error /= total_weight;
    }

    // Combine metrics with weights
    // MAE: 30%, Total Duration: 15%, Correlation: 25%, Ratio: 15%, Weighted MAE: 15%
    // Plus track count penalty (100s per missing/extra track)
    let composite_score =
        mae * 0.30 +
        total_duration_error * 0.15 +
        correlation_error * 0.25 +
        ratio_error * 0.15 +
        weighted_error * 0.15 +
        track_count_penalty;

    // Classify confidence based on composite score and track count match
    let confidence = if composite_score < 10.0 && correlation > 0.95 && track_count_penalty == 0.0 {
        "Excellent"
    } else if composite_score < 20.0 && correlation > 0.85 && track_count_penalty == 0.0 {
        "Good"
    } else if composite_score < 150.0 && correlation > 0.70 {
        "Fair"  // Allow Fair rating even with 1 missing/extra track
    } else {
        "Poor"
    };

    (composite_score, confidence.to_string())
}

/// Process a single album file
async fn process_album(file_path: &Path, rate_limiter: &RateLimiter) -> MatchResult {
    println!("\nProcessing: {}", file_path.display());

    // Decode audio
    print!("  Decoding... ");
    std::io::Write::flush(&mut std::io::stdout()).ok();

    let (samples, sample_rate) = match decode_mp3(file_path) {
        Ok((s, sr)) => {
            println!("Done! {} samples at {} Hz ({:.2} mins)",
                s.len(), sr, s.len() as f64 / sr as f64 / 60.0);
            (s, sr)
        }
        Err(e) => {
            println!("Failed: {}", e);
            return MatchResult {
                file_path: file_path.to_string_lossy().to_string(),
                detected_tracks: 0,
                detected_durations: Vec::new(),
                total_detected_duration: 0.0,
                best_match_mbid: None,
                best_match_title: None,
                best_match_artist: None,
                best_match_track_count: None,
                mean_duration_error: None,
                confidence: "N/A".to_string(),
                status: format!("Failed to decode: {}", e),
            };
        }
    };

    // Detect tracks
    print!("  Detecting tracks... ");
    std::io::Write::flush(&mut std::io::stdout()).ok();

    let tracks = match detect_tracks(&samples, sample_rate) {
        Ok(t) => {
            println!("Found {} tracks", t.len());
            t
        }
        Err(e) => {
            println!("Failed: {}", e);
            return MatchResult {
                file_path: file_path.to_string_lossy().to_string(),
                detected_tracks: 0,
                detected_durations: Vec::new(),
                total_detected_duration: 0.0,
                best_match_mbid: None,
                best_match_title: None,
                best_match_artist: None,
                best_match_track_count: None,
                mean_duration_error: None,
                confidence: "N/A".to_string(),
                status: format!("Failed to detect tracks: {}", e),
            };
        }
    };

    if tracks.is_empty() {
        return MatchResult {
            file_path: file_path.to_string_lossy().to_string(),
            detected_tracks: 0,
            detected_durations: Vec::new(),
            total_detected_duration: 0.0,
            best_match_mbid: None,
            best_match_title: None,
            best_match_artist: None,
            best_match_track_count: None,
            mean_duration_error: None,
            confidence: "N/A".to_string(),
            status: "No tracks detected (file appears to be continuous)".to_string(),
        };
    }

    let detected_durations: Vec<f64> = tracks.iter().map(|t| t.duration_secs).collect();
    let total_duration: f64 = detected_durations.iter().sum();

    // Extract metadata from path
    let (artist, album) = extract_metadata_from_path(file_path);

    // Search MusicBrainz
    let candidates = match search_musicbrainz(&artist, &album, tracks.len(), rate_limiter).await {
        Ok(c) => c,
        Err(e) => {
            println!("  MusicBrainz search failed: {}", e);
            return MatchResult {
                file_path: file_path.to_string_lossy().to_string(),
                detected_tracks: tracks.len(),
                detected_durations,
                total_detected_duration: total_duration,
                best_match_mbid: None,
                best_match_title: None,
                best_match_artist: None,
                best_match_track_count: None,
                mean_duration_error: None,
                confidence: "N/A".to_string(),
                status: format!("MusicBrainz search failed: {}", e),
            };
        }
    };

    if candidates.is_empty() {
        println!("  No MusicBrainz matches found");
        return MatchResult {
            file_path: file_path.to_string_lossy().to_string(),
            detected_tracks: tracks.len(),
            detected_durations,
            total_detected_duration: total_duration,
            best_match_mbid: None,
            best_match_title: Some(album),
            best_match_artist: Some(artist),
            best_match_track_count: None,
            mean_duration_error: None,
            confidence: "No Match".to_string(),
            status: "No MusicBrainz candidates found".to_string(),
        };
    }

    // Find best match
    println!("  Scoring {} candidates:", candidates.len());
    let mut best_candidate: Option<&ReleaseCandidate> = None;
    let mut best_score = f64::MAX;
    let mut best_confidence = "Poor".to_string();

    for (idx, candidate) in candidates.iter().enumerate() {
        let (score, confidence) = calculate_match_score(&tracks, candidate);
        println!("    {}: {} - {} ({} tracks) → score={:.2}, confidence={}",
            idx + 1, candidate.artist, candidate.title, candidate.track_count, score, confidence);
        if score < best_score {
            best_score = score;
            best_candidate = Some(candidate);
            best_confidence = confidence.clone();
        }
    }

    if let Some(candidate) = best_candidate {
        println!("  Best match: {} - {} (MBID: {})", candidate.artist, candidate.title, candidate.mbid);
        println!("  Confidence: {} (mean error: {:.2}s)", best_confidence, best_score);

        MatchResult {
            file_path: file_path.to_string_lossy().to_string(),
            detected_tracks: tracks.len(),
            detected_durations,
            total_detected_duration: total_duration,
            best_match_mbid: Some(candidate.mbid.clone()),
            best_match_title: Some(candidate.title.clone()),
            best_match_artist: Some(candidate.artist.clone()),
            best_match_track_count: Some(candidate.track_count),
            mean_duration_error: Some(best_score),
            confidence: best_confidence,
            status: "Success".to_string(),
        }
    } else {
        MatchResult {
            file_path: file_path.to_string_lossy().to_string(),
            detected_tracks: tracks.len(),
            detected_durations,
            total_detected_duration: total_duration,
            best_match_mbid: None,
            best_match_title: Some(album),
            best_match_artist: Some(artist),
            best_match_track_count: None,
            mean_duration_error: None,
            confidence: "No Match".to_string(),
            status: "No matching candidates".to_string(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_list_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\long_files_list.txt");
    let output_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\album_matches.json");

    if !file_list_path.exists() {
        eprintln!("File list not found: {}", file_list_path.display());
        return Ok(());
    }

    println!("Reading file list from: {}", file_list_path.display());
    let content = std::fs::read_to_string(file_list_path)?;

    // Parse file list (format: [duration] path)
    let mut file_paths: Vec<PathBuf> = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Extract path after the duration timestamp
        if let Some(path_start) = line.find("] ") {
            let path_str = &line[path_start + 2..];
            file_paths.push(PathBuf::from(path_str));
        }
    }

    println!("Found {} album files to process\n", file_paths.len());

    // Check if training_set.txt exists, otherwise use all files
    let training_set_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\training_set.txt");

    let files_to_process = if training_set_path.exists() {
        println!("NOTE: Processing training set (25 albums)\n");

        let training_content = std::fs::read_to_string(training_set_path)?;
        let mut training_files = Vec::new();

        for line in training_content.lines() {
            if let Some(path_start) = line.find("] ") {
                let path_str = &line[path_start + 2..];
                training_files.push(PathBuf::from(path_str));
            }
        }

        training_files
    } else {
        println!("NOTE: Processing all {} albums\n", file_paths.len());
        file_paths.clone()
    };

    println!("Processing {} albums...\n", files_to_process.len());

    // Create rate limiter for MusicBrainz API
    let rate_limiter = RateLimiter::new();

    let mut results: Vec<MatchResult> = Vec::new();

    // Process all files
    for (idx, file_path) in files_to_process.iter().enumerate() {
        println!("=== Album {}/{} ===", idx + 1, files_to_process.len());

        if !file_path.exists() {
            println!("WARNING: File not found: {}", file_path.display());
            results.push(MatchResult {
                file_path: file_path.to_string_lossy().to_string(),
                detected_tracks: 0,
                detected_durations: Vec::new(),
                total_detected_duration: 0.0,
                best_match_mbid: None,
                best_match_title: None,
                best_match_artist: None,
                best_match_track_count: None,
                mean_duration_error: None,
                confidence: "N/A".to_string(),
                status: "File not found".to_string(),
            });
        } else {
            let result = process_album(&file_path, &rate_limiter).await;
            results.push(result);
        }
    }

    // Write results to JSON
    println!("\n\nWriting results to: {}", output_path.display());
    let json = serde_json::to_string_pretty(&results)?;
    std::fs::write(output_path, json)?;

    println!("Done! Processed {} albums", results.len());

    // Print detailed summary
    let detected_count = results.iter().filter(|r| r.detected_tracks > 0).count();
    let matched_count = results.iter().filter(|r| r.status == "Success").count();

    let excellent = results.iter().filter(|r| r.confidence == "Excellent").count();
    let good = results.iter().filter(|r| r.confidence == "Good").count();
    let fair = results.iter().filter(|r| r.confidence == "Fair").count();
    let poor = results.iter().filter(|r| r.confidence == "Poor" && r.status == "Success").count();
    let no_match = results.iter().filter(|r| r.confidence == "No Match").count();

    println!("\n=== MATCHING SUMMARY ===");
    println!("Total files: {}", results.len());
    println!("Successfully detected tracks: {}", detected_count);
    println!("Successfully matched to MusicBrainz: {}", matched_count);
    println!("\nConfidence Distribution:");
    println!("  Excellent: {}", excellent);
    println!("  Good:      {}", good);
    println!("  Fair:      {}", fair);
    println!("  Poor:      {}", poor);
    println!("  No Match:  {}", no_match);

    if matched_count > 0 {
        let scores: Vec<f64> = results.iter()
            .filter_map(|r| r.mean_duration_error)
            .collect();

        if !scores.is_empty() {
            let avg_score = scores.iter().sum::<f64>() / scores.len() as f64;
            let mut sorted_scores = scores.clone();
            sorted_scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median_score = sorted_scores[sorted_scores.len() / 2];

            println!("\nScoring Statistics:");
            println!("  Average composite score: {:.2}", avg_score);
            println!("  Median composite score:  {:.2}", median_score);
            println!("  Best score:   {:.2}", sorted_scores[0]);
            println!("  Worst score:  {:.2}", sorted_scores[sorted_scores.len() - 1]);
        }
    }

    Ok(())
}

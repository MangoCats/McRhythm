# Increment 13: AlbumMatcher Implementation

**Estimated Effort:** 5 hours
**Dependencies:** Increments 1-12
**Deliverables:** services/album_matcher.rs (full implementation)

---

## Objective

Implement the complete `AlbumMatcher` service that ties together all components: audio decoding, metadata extraction, MusicBrainz search, silence detection, multi-stage matching, and result classification.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| main.rs | ~1137 | Extract core logic |
| orchestration.rs | ~361 | Already in Increment 11 |

---

## Tasks

### 13.1 Implement Full AlbumMatcher

Replace placeholder in `services/album_matcher.rs`:

```rust
//! Album Matcher Service
//!
//! Complete implementation of multi-stage album matching.

use crate::matching::{
    constants::*,
    types::*,
    metadata::extract_metadata,
    silence_detection::precompute_silence_cache,
    single_track::{analyze_pre_decode, analyze_post_decode, SingleTrackThresholds},
    editions::{group_into_editions, filter_and_sort_editions},
    stages::stage4::RmsProfile,
    orchestrator::{run_orchestration, OrchestratorConfig},
    validation::verify_artist_match,
};
use crate::services::MusicBrainzClient;
use std::path::Path;
use tokio::task::spawn_blocking;

/// Album Matcher configuration
#[derive(Debug, Clone)]
pub struct AlbumMatcherConfig {
    /// Minimum match percentage to accept
    pub min_match_percentage: f64,
    /// Track duration tolerance (seconds)
    pub tolerance_secs: f64,
    /// Minimum artist similarity (Jaro-Winkler)
    pub min_artist_similarity: f64,
    /// Maximum editions to test
    pub max_editions: usize,
    /// Enable early exit on 100% match
    pub early_exit: bool,
    /// Silence detection thresholds (dB)
    pub threshold_values: Vec<f64>,
    /// Minimum silence durations (seconds)
    pub min_duration_values: Vec<f64>,
    /// Enable MusicBrainz caching
    pub cache_enabled: bool,
}

impl Default for AlbumMatcherConfig {
    fn default() -> Self {
        Self {
            min_match_percentage: 80.0,
            tolerance_secs: MATCH_TOLERANCE_SECS,
            min_artist_similarity: MIN_ARTIST_SIMILARITY,
            max_editions: 20,
            early_exit: true,
            threshold_values: THRESHOLD_VALUES.to_vec(),
            min_duration_values: MIN_DURATION_VALUES.to_vec(),
            cache_enabled: true,
        }
    }
}

/// Album Matcher service
pub struct AlbumMatcher {
    config: AlbumMatcherConfig,
    mb_client: MusicBrainzClient,
}

impl AlbumMatcher {
    /// Create new AlbumMatcher
    pub fn new(config: AlbumMatcherConfig, mb_client: MusicBrainzClient) -> Self {
        Self { config, mb_client }
    }

    /// Match album file to MusicBrainz release
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    ///
    /// # Returns
    /// Classification result with match details
    pub async fn match_album(&self, file_path: &Path) -> Result<ClassificationResult, AlbumMatchError> {
        // Step 1: Extract metadata
        let metadata = extract_metadata(file_path)?;

        // Step 2: Pre-decode single-track check
        let pre_analysis = analyze_pre_decode(
            file_path,
            metadata.duration_hint,
            &SingleTrackThresholds::default(),
        );

        if pre_analysis.is_single_track && pre_analysis.confidence > 0.7 {
            return Ok(ClassificationResult {
                content_type: ContentType::SingleTrack,
                confidence: MatchConfidence::Medium,
                confidence_value: pre_analysis.confidence,
                recording_mbid: None,
                release_mbid: None,
                match_percentage: None,
                artist_verified: false,
                matching_stage: Some("pre_decode".to_string()),
            });
        }

        // Step 3: Decode audio (CPU-bound, run in blocking task)
        let file_path_owned = file_path.to_path_buf();
        let threshold_values = self.config.threshold_values.clone();
        let min_duration_values = self.config.min_duration_values.clone();

        let (samples, sample_rate, silence_cache, rms_profile) = spawn_blocking(move || {
            // Decode to mono samples
            let (samples, sample_rate) = decode_audio_mono(&file_path_owned)?;

            // Pre-compute silence cache
            let silence_cache = precompute_silence_cache(
                &samples,
                sample_rate,
                &threshold_values,
                &min_duration_values,
            );

            // Compute RMS profile for Stage 4
            let rms_profile = RmsProfile::from_samples(&samples, sample_rate, 50.0);

            Ok::<_, AlbumMatchError>((samples, sample_rate, silence_cache, rms_profile))
        })
        .await
        .map_err(|e| AlbumMatchError::TaskJoinError(e.to_string()))??;

        // Step 4: Post-decode single-track check
        let silence_count = silence_cache.get(
            DEFAULT_THRESHOLD_IDX,
            DEFAULT_MIN_DURATION_IDX,
        ).len().saturating_sub(1);

        let post_analysis = analyze_post_decode(
            &pre_analysis,
            samples.len() as f64 / sample_rate as f64,
            silence_count,
            &SingleTrackThresholds::default(),
        );

        if post_analysis.is_single_track && post_analysis.confidence > 0.8 {
            return Ok(ClassificationResult {
                content_type: ContentType::SingleTrack,
                confidence: MatchConfidence::High,
                confidence_value: post_analysis.confidence,
                recording_mbid: None,
                release_mbid: None,
                match_percentage: None,
                artist_verified: false,
                matching_stage: Some("post_decode".to_string()),
            });
        }

        // Step 5: Search MusicBrainz
        let releases = self.mb_client.comprehensive_search(
            &metadata.artist,
            &metadata.album,
            50,
        ).await?;

        if releases.is_empty() {
            return Ok(ClassificationResult {
                content_type: ContentType::IdentificationFailed,
                confidence: MatchConfidence::Poor,
                confidence_value: 0.0,
                recording_mbid: None,
                release_mbid: None,
                match_percentage: None,
                artist_verified: false,
                matching_stage: Some("no_candidates".to_string()),
            });
        }

        // Step 6: Group into editions and filter
        let editions = group_into_editions(&releases);
        let editions = filter_and_sort_editions(
            editions,
            &metadata.artist,
            &metadata.album,
            self.config.max_editions,
        );

        // Step 7: Run multi-stage matching
        let orchestrator_config = OrchestratorConfig {
            min_match_percentage: self.config.min_match_percentage,
            tolerance_secs: self.config.tolerance_secs,
            early_exit: self.config.early_exit,
            ..Default::default()
        };

        let result = run_orchestration(
            &silence_cache,
            &rms_profile,
            samples.len(),
            &editions,
            &orchestrator_config,
        );

        // Step 8: Verify artist match
        let artist_verified = result.matched_edition.as_ref()
            .map(|e| verify_artist_match(&e.artist, &metadata.artist, self.config.min_artist_similarity))
            .unwrap_or(false);

        // Step 9: Build classification result
        let (content_type, confidence) = classify_result(
            result.match_percentage,
            artist_verified,
            self.config.min_match_percentage,
        );

        Ok(ClassificationResult {
            content_type,
            confidence,
            confidence_value: result.match_percentage / 100.0,
            recording_mbid: result.matched_edition.as_ref()
                .and_then(|e| e.recording_mbids.first().cloned()),
            release_mbid: result.matched_edition.as_ref()
                .map(|e| e.release_mbid.clone()),
            match_percentage: Some(result.match_percentage),
            artist_verified,
            matching_stage: Some(format!("{:?}", result.winning_stage)),
        })
    }
}

/// Classify result into content type and confidence
fn classify_result(
    percentage: f64,
    artist_verified: bool,
    min_percentage: f64,
) -> (ContentType, MatchConfidence) {
    if percentage >= 100.0 && artist_verified {
        (ContentType::IdentifiedAlbum, MatchConfidence::High)
    } else if percentage >= min_percentage && artist_verified {
        (ContentType::IdentifiedAlbum, MatchConfidence::Medium)
    } else if percentage >= min_percentage {
        (ContentType::IdentifiedAlbum, MatchConfidence::Low)
    } else if percentage >= 50.0 {
        (ContentType::PossibleAlbum, MatchConfidence::Low)
    } else {
        (ContentType::IdentificationFailed, MatchConfidence::Poor)
    }
}

/// Decode audio file to mono f32 samples
fn decode_audio_mono(path: &Path) -> Result<(Vec<f32>, u32), AlbumMatchError> {
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path)
        .map_err(|e| AlbumMatchError::IoError(e.to_string()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| AlbumMatchError::DecodeError(e.to_string()))?;

    let mut format = probed.format;
    let track = format.default_track()
        .ok_or(AlbumMatchError::DecodeError("No default track".into()))?;

    let sample_rate = track.codec_params.sample_rate
        .ok_or(AlbumMatchError::DecodeError("No sample rate".into()))?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AlbumMatchError::DecodeError(e.to_string()))?;

    let track_id = track.id;
    let mut samples = Vec::new();

    loop {
        match format.next_packet() {
            Ok(packet) if packet.track_id() == track_id => {
                if let Ok(decoded) = decoder.decode(&packet) {
                    // Convert to mono f32
                    let spec = decoded.spec();
                    let channels = spec.channels.count();

                    // Use audio buffer conversion
                    let mut sample_buf = symphonia::core::audio::SampleBuffer::<f32>::new(
                        decoded.capacity() as u64,
                        *spec,
                    );
                    sample_buf.copy_interleaved_ref(decoded);

                    // Mix to mono
                    for chunk in sample_buf.samples().chunks(channels) {
                        let mono: f32 = chunk.iter().sum::<f32>() / channels as f32;
                        samples.push(mono);
                    }
                }
            }
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(_) => continue,
            _ => continue,
        }
    }

    Ok((samples, sample_rate))
}
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-013-01 | Config defaults | Correct default values |
| TC-U-013-02 | Single-track rejection | Rejects short files |
| TC-U-013-03 | Classification logic | Correct content types |
| TC-U-013-04 | Artist verification | Integrates correctly |
| TC-I-013-01 | Full match pipeline | Real audio matches |
| TC-I-013-02 | No candidates handling | Graceful failure |

---

## Acceptance Criteria

- [ ] AlbumMatcher fully implemented
- [ ] All stages integrated
- [ ] Single-track rejection working
- [ ] Classification correct
- [ ] All 6 tests pass

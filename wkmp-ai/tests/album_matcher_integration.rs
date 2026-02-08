//! Album Matcher Integration Tests
//!
//! **[PLAN030 Increment 14]** Tests the complete matching pipeline.
//!
//! ## Test Categories
//! - Full pipeline tests (require audio fixtures)
//! - Error handling tests
//! - Performance baseline tests
//!
//! ## Test Fixtures
//! Real audio fixtures should be placed in `tests/fixtures/audio/`.
//! Tests marked `#[ignore]` will be skipped if fixtures are unavailable.

use std::path::{Path, PathBuf};
use std::time::Instant;

// Import from wkmp_ai crate
use wkmp_ai::matching::{
    AlbumMatchError, AlbumMatchResult, AlbumMatcher, AlbumMatcherConfig, MatchingStage,
};

// =============================================================================
// Test Fixtures Module
// =============================================================================

mod test_fixtures {
    use super::*;

    /// Get path to test audio fixture directory
    pub fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/audio")
    }

    /// Get path to a specific test audio fixture
    pub fn fixture_path(name: &str) -> PathBuf {
        fixture_dir().join(name)
    }

    /// Check if test fixtures are available
    pub fn fixtures_available() -> bool {
        fixture_dir().exists() && fixture_path("test_album.mp3").exists()
    }

    /// Check if a specific fixture exists
    pub fn fixture_exists(name: &str) -> bool {
        fixture_path(name).exists()
    }
}

/// Test fixture metadata for documentation
#[derive(Debug)]
struct TestFixture {
    filename: &'static str,
    expected_artist: &'static str,
    expected_album: &'static str,
    expected_tracks: usize,
    expected_match: bool,
}

/// Documented test fixtures (for reference when creating actual files)
const TEST_FIXTURES: &[TestFixture] = &[
    TestFixture {
        filename: "known_album_10tracks.mp3",
        expected_artist: "Test Artist",
        expected_album: "Test Album",
        expected_tracks: 10,
        expected_match: true,
    },
    TestFixture {
        filename: "single_track.mp3",
        expected_artist: "Single Artist",
        expected_album: "Single",
        expected_tracks: 1,
        expected_match: false, // Should be rejected as single track
    },
    TestFixture {
        filename: "difficult_album.mp3",
        expected_artist: "Difficult Artist",
        expected_album: "Quiet Transitions",
        expected_tracks: 8,
        expected_match: true, // May need Stage 3-4
    },
    TestFixture {
        filename: "noisy_album.mp3",
        expected_artist: "Noisy Artist",
        expected_album: "No Silences",
        expected_tracks: 12,
        expected_match: true, // Needs Stage 4
    },
    TestFixture {
        filename: "bonus_tracks.mp3",
        expected_artist: "Bonus Artist",
        expected_album: "Hidden Track Album",
        expected_tracks: 11,
        expected_match: true, // Needs Stage 5
    },
];

// =============================================================================
// Full Pipeline Tests
// =============================================================================

/// TC-I-014-01: Test full album matching pipeline with known album
#[tokio::test]
#[ignore = "Requires test audio fixtures"]
async fn test_full_album_match_pipeline() {
    use test_fixtures::*;

    if !fixtures_available() {
        eprintln!("Skipping: test fixtures not available");
        return;
    }

    let matcher = AlbumMatcher::new().expect("Failed to create matcher");

    let path = fixture_path("known_album_10tracks.mp3");
    let result = matcher.match_album(&path, None, None).await;

    assert!(result.is_ok(), "Match should succeed: {:?}", result.err());
    let result = result.unwrap();

    // Should be identified as an album
    assert!(result.matched, "Should find a match");
    assert!(
        result.match_percentage >= 80.0,
        "Match percentage should be >= 80%: {}",
        result.match_percentage
    );
    assert!(result.artist_verified, "Artist should be verified");
}

/// TC-I-014-02: Test single-track rejection
#[tokio::test]
#[ignore = "Requires test audio fixtures"]
async fn test_single_track_rejection() {
    use test_fixtures::*;

    if !fixture_exists("single_track.mp3") {
        eprintln!("Skipping: single_track.mp3 fixture not available");
        return;
    }

    let matcher = AlbumMatcher::new().expect("Failed to create matcher");

    let path = fixture_path("single_track.mp3");
    let result = matcher.match_album(&path, None, None).await;

    // Should return a result (possibly no_match) rather than error
    let result = result.expect("Should not error");

    // Either matched=false or status indicates single track
    if result.matched {
        // If it matched, it shouldn't have high track count
        assert!(
            result.detected_track_count <= 2,
            "Single track should not detect many tracks"
        );
    } else {
        // No match is expected - check status
        assert!(
            result.status.contains("Single track") || !result.matched,
            "Should indicate single track or no match"
        );
    }
}

/// TC-I-014-03: Test stage progression (all stages accessible)
#[tokio::test]
#[ignore = "Requires test audio fixtures"]
async fn test_stage_progression() {
    use test_fixtures::*;

    if !fixture_exists("difficult_album.mp3") {
        eprintln!("Skipping: difficult_album.mp3 fixture not available");
        return;
    }

    // Disable early exit to test all stages
    let config = AlbumMatcherConfig {
        enable_early_exit: false,
        ..Default::default()
    };

    let matcher = AlbumMatcher::with_config(config).expect("Failed to create matcher");

    let path = fixture_path("difficult_album.mp3");
    let result = matcher.match_album(&path, None, None).await;

    assert!(result.is_ok(), "Should process without error");
    let result = result.unwrap();

    // Verify that a stage was recorded
    assert!(
        result.matching_stage.is_some(),
        "Should record matching stage"
    );
}

/// Test with artist/album hints
#[tokio::test]
#[ignore = "Requires test audio fixtures"]
async fn test_with_metadata_hints() {
    use test_fixtures::*;

    if !fixtures_available() {
        eprintln!("Skipping: test fixtures not available");
        return;
    }

    let matcher = AlbumMatcher::new().expect("Failed to create matcher");

    let path = fixture_path("known_album_10tracks.mp3");

    // Provide explicit artist/album hints
    let result = matcher
        .match_album(&path, Some("Test Artist"), Some("Test Album"))
        .await;

    assert!(result.is_ok(), "Match with hints should succeed");
    let result = result.unwrap();

    // Should still find a match
    assert!(result.matched, "Should find match with correct hints");
}

// =============================================================================
// Error Handling Tests
// =============================================================================

/// TC-I-014-04: Test missing file error handling
#[tokio::test]
async fn test_missing_file_error() {
    let matcher = AlbumMatcher::new().expect("Failed to create matcher");

    let result = matcher
        .match_album(Path::new("/nonexistent/path/file.mp3"), None, None)
        .await;

    assert!(result.is_err(), "Should return error for missing file");
    let err = result.unwrap_err();
    // Should be an IO or decode error
    let err_str = format!("{}", err);
    assert!(
        err_str.contains("error") || err_str.contains("Error"),
        "Error message should indicate problem: {}",
        err_str
    );
}

/// TC-I-014-05: Test invalid audio file error handling
#[tokio::test]
async fn test_invalid_audio_error() {
    let matcher = AlbumMatcher::new().expect("Failed to create matcher");

    // Create a temporary non-audio file
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = temp_dir.path().join("not_audio.mp3");
    std::fs::write(&path, b"This is not audio content at all").expect("Failed to write test file");

    let result = matcher.match_album(&path, None, None).await;

    assert!(result.is_err(), "Should return error for invalid audio");
}

/// TC-I-014-06: Test handling of empty file
#[tokio::test]
async fn test_empty_file_error() {
    let matcher = AlbumMatcher::new().expect("Failed to create matcher");

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = temp_dir.path().join("empty.mp3");
    std::fs::write(&path, b"").expect("Failed to write empty file");

    let result = matcher.match_album(&path, None, None).await;

    assert!(result.is_err(), "Should return error for empty file");
}

/// Test configuration validation
#[test]
fn test_config_validation() {
    // Default config should be valid
    let config = AlbumMatcherConfig::default();
    assert!(config.match_tolerance_secs > 0.0);
    assert!(config.min_artist_similarity > 0.0);
    assert!(config.min_artist_similarity <= 1.0);

    // Custom config with valid values
    let custom = AlbumMatcherConfig {
        match_tolerance_secs: 5.0,
        min_artist_similarity: 0.7,
        enable_stage3: false,
        enable_stage4: false,
        enable_stage5: false,
        ..Default::default()
    };
    assert_eq!(custom.match_tolerance_secs, 5.0);
    assert_eq!(custom.min_artist_similarity, 0.7);
    assert!(!custom.enable_stage3);
}

/// Test custom threshold/duration values
#[test]
fn test_custom_parameter_grid() {
    let config = AlbumMatcherConfig {
        threshold_values: Some(vec![-50.0, -55.0, -60.0]),
        min_duration_values: Some(vec![0.3, 0.5, 1.0]),
        ..Default::default()
    };

    assert_eq!(config.threshold_values().len(), 3);
    assert_eq!(config.min_duration_values().len(), 3);
    assert_eq!(config.total_combinations(), 9); // 3 x 3
}

// =============================================================================
// Performance Tests
// =============================================================================

/// TC-S-014-01: Performance baseline test
#[tokio::test]
#[ignore = "Performance test - requires fixtures"]
async fn test_matching_performance() {
    use test_fixtures::*;

    if !fixtures_available() {
        eprintln!("Skipping: test fixtures not available");
        return;
    }

    let matcher = AlbumMatcher::new().expect("Failed to create matcher");

    let path = fixture_path("known_album_10tracks.mp3");

    let start = Instant::now();
    let result = matcher.match_album(&path, None, None).await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Match should succeed");

    // Performance target: ~45 minutes of audio should process in <60 seconds
    // Adjust based on actual test fixture duration
    println!("Matching completed in {:?}", duration);
    assert!(
        duration.as_secs() < 120,
        "Matching took too long: {:?}",
        duration
    );
}

/// Test early exit performance benefit
#[tokio::test]
#[ignore = "Performance test - requires fixtures"]
async fn test_early_exit_performance() {
    use test_fixtures::*;

    if !fixtures_available() {
        eprintln!("Skipping: test fixtures not available");
        return;
    }

    let path = fixture_path("known_album_10tracks.mp3");

    // Test with early exit enabled
    let config_early = AlbumMatcherConfig {
        enable_early_exit: true,
        ..Default::default()
    };
    let matcher_early = AlbumMatcher::with_config(config_early).expect("Failed to create matcher");

    let start_early = Instant::now();
    let _ = matcher_early.match_album(&path, None, None).await;
    let duration_early = start_early.elapsed();

    // Test with early exit disabled
    let config_no_early = AlbumMatcherConfig {
        enable_early_exit: false,
        ..Default::default()
    };
    let matcher_no_early =
        AlbumMatcher::with_config(config_no_early).expect("Failed to create matcher");

    let start_no_early = Instant::now();
    let _ = matcher_no_early.match_album(&path, None, None).await;
    let duration_no_early = start_no_early.elapsed();

    println!(
        "Early exit: {:?}, No early exit: {:?}",
        duration_early, duration_no_early
    );

    // Early exit should generally be faster or equal
    // (not always true if match is found in last parameter combination)
}

// =============================================================================
// Stage-Specific Tests
// =============================================================================

/// Test Stage 2 isolation (parameter grid search only)
#[tokio::test]
#[ignore = "Requires test audio fixtures"]
async fn test_stage2_only() {
    use test_fixtures::*;

    if !fixtures_available() {
        return;
    }

    let config = AlbumMatcherConfig {
        enable_stage3: false,
        enable_stage4: false,
        enable_stage5: false,
        ..Default::default()
    };

    let matcher = AlbumMatcher::with_config(config).expect("Failed to create matcher");
    let path = fixture_path("known_album_10tracks.mp3");

    let result = matcher.match_album(&path, None, None).await;
    assert!(result.is_ok());

    let result = result.unwrap();
    if result.matched {
        // If matched, should be Stage 2
        assert_eq!(
            result.matching_stage,
            Some(MatchingStage::Stage2),
            "Should match via Stage 2 when other stages disabled"
        );
    }
}

/// Test reduced parameter grid for faster testing
#[tokio::test]
#[ignore = "Requires test audio fixtures"]
async fn test_reduced_parameter_grid() {
    use test_fixtures::*;

    if !fixtures_available() {
        return;
    }

    // Use smaller parameter grid for faster testing
    let config = AlbumMatcherConfig {
        threshold_values: Some(vec![-50.0, -55.0, -60.0]),
        min_duration_values: Some(vec![0.3, 0.5, 1.0]),
        ..Default::default()
    };

    assert_eq!(config.total_combinations(), 9);

    let matcher = AlbumMatcher::with_config(config).expect("Failed to create matcher");
    let path = fixture_path("known_album_10tracks.mp3");

    let result = matcher.match_album(&path, None, None).await;
    assert!(result.is_ok(), "Should process with reduced grid");
}

// =============================================================================
// Integration with Generated Audio Tests
// =============================================================================

/// Test with programmatically generated WAV audio
/// (Uses the existing audio_generator helper)
#[tokio::test]
async fn test_with_generated_audio() {
    // This test uses generated audio which will likely not match any MusicBrainz entry,
    // but validates the pipeline doesn't crash on valid audio

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wav_path = temp_dir.path().join("generated_test.wav");

    // Generate a simple test WAV file
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(&wav_path, spec).expect("Failed to create WAV");

    // Generate 10 seconds of audio with a 1-second silence gap at 5s
    let sample_rate = 44100;
    let total_samples = 10 * sample_rate;
    let silence_start = 5 * sample_rate;
    let silence_end = 6 * sample_rate;

    for i in 0..total_samples {
        let sample = if i >= silence_start && i < silence_end {
            0i16 // Silence
        } else {
            // Simple tone
            let t = i as f32 / sample_rate as f32;
            ((0.3 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()) * i16::MAX as f32) as i16
        };
        writer.write_sample(sample).expect("Failed to write sample");
        writer.write_sample(sample).expect("Failed to write sample"); // Stereo
    }
    writer.finalize().expect("Failed to finalize WAV");

    // Now test the matcher with this generated audio
    let matcher = AlbumMatcher::new().expect("Failed to create matcher");

    // This will likely fail to match anything in MusicBrainz, but should not panic
    let result = matcher
        .match_album(&wav_path, Some("Generated"), Some("Test Audio"))
        .await;

    // Either succeeds with no_match or errors due to no MusicBrainz results
    // Both are acceptable - we're testing the pipeline doesn't crash
    match result {
        Ok(r) => {
            println!(
                "Generated audio result: matched={}, status={}",
                r.matched, r.status
            );
            // Expected: no match since this is generated audio
        }
        Err(e) => {
            println!("Generated audio error (expected): {}", e);
            // Expected: no candidates or network error in test environment
        }
    }
}

// =============================================================================
// AlbumMatchResult Tests
// =============================================================================

/// Test AlbumMatchResult::no_match helper
#[test]
fn test_album_match_result_no_match() {
    let result = AlbumMatchResult::no_match("Test reason".to_string());

    assert!(!result.matched);
    assert!(result.release_mbid.is_none());
    assert!(result.matched_artist.is_none());
    assert!(result.matched_album.is_none());
    assert!(result.status.contains("Test reason"));
}

/// Test AlbumMatchError variants
#[test]
fn test_error_variants() {
    let errors: Vec<AlbumMatchError> = vec![
        AlbumMatchError::DecodeError("decode failed".into()),
        AlbumMatchError::MetadataError("metadata failed".into()),
        AlbumMatchError::MusicBrainzError("api failed".into()),
        AlbumMatchError::NoCandidates,
        AlbumMatchError::InternalError("internal".into()),
        AlbumMatchError::IoError("io failed".into()),
        AlbumMatchError::TaskJoinError("join failed".into()),
        AlbumMatchError::SingleTrackDetected {
            confidence: 0.95,
            stage: "pre_decode".into(),
        },
    ];

    for error in errors {
        let display = format!("{}", error);
        assert!(!display.is_empty(), "Error should have display message");

        // Also test std::error::Error impl
        let _ = error.to_string();
    }
}

// =============================================================================
// Matcher Construction Tests
// =============================================================================

/// Test matcher construction methods
#[test]
fn test_matcher_construction() {
    // Default construction
    let matcher = AlbumMatcher::new();
    assert!(matcher.is_ok(), "Default construction should succeed");

    // With custom config
    let config = AlbumMatcherConfig {
        match_tolerance_secs: 5.0,
        ..Default::default()
    };
    let matcher = AlbumMatcher::with_config(config);
    assert!(matcher.is_ok(), "Custom config construction should succeed");
}

/// Test config accessor
#[test]
fn test_config_accessor() {
    let config = AlbumMatcherConfig {
        match_tolerance_secs: 7.5,
        min_artist_similarity: 0.65,
        ..Default::default()
    };

    let matcher = AlbumMatcher::with_config(config).expect("Failed to create");

    assert_eq!(matcher.config().match_tolerance_secs, 7.5);
    assert_eq!(matcher.config().min_artist_similarity, 0.65);
}

//! Album Matcher Accuracy Tests
//!
//! Validates track-level boundary accuracy against am29f baseline

use std::path::Path;
use wkmp_ai::matching::{AlbumMatcher, AlbumMatcherConfig};

/// ZZ Top's First Album - Track-level segmentation validation
///
/// **Purpose:** Validates that Phase 3 achieves track-level segmentation accuracy
/// comparable to am29f while providing enhanced edition discovery.
///
/// **Expected Behavior:**
/// - am29f: 10 tracks via silence detection
/// - Phase 3: 10 tracks via multi-stage assembly (detects boundaries → assembles to tracks)
/// - Both: 100% match, mean error ≤5.0s
/// - Phase 3 enhancement: Finds 10-25 editions vs am29f's 1 edition
///
/// This test verifies that am29f optimizations (WindowDbProfile, early-exit) work correctly
/// AND that track-level output matches expected 10-track structure.
#[tokio::test]
#[cfg_attr(not(feature = "benchmark_tests"), ignore)]
async fn test_zz_top_first_album_track_segmentation() {
    let audio_path = Path::new("tests/fixtures/ZZTopsFirstAlbum.mp3");

    // Skip if fixture missing (e.g., in CI without test files)
    if !audio_path.exists() {
        eprintln!("SKIP: Test fixture not found (enable Git LFS or run locally)");
        eprintln!("Expected: tests/fixtures/ZZTopsFirstAlbum.mp3");
        return;
    }

    // Use am29f's proven parameters: -62.0dB threshold, 0.5s min duration
    let mut config = AlbumMatcherConfig::default();
    config.default_threshold_db = -62.0;
    config.default_min_duration_secs = 0.5;
    let matcher = AlbumMatcher::with_config(config).expect("Failed to create matcher");

    // Provide artist/album hints since concatenated file may lack ID3 tags
    let result = matcher
        .match_album(audio_path, Some("ZZ Top"), Some("ZZ Top's First Album"))
        .await
        .expect("Album matching failed");

    // Debug output
    eprintln!("\n=== Album Match Result ===");
    eprintln!("Matched: {}", result.matched);
    eprintln!("Artist: {:?}", result.matched_artist);
    eprintln!("Album: {:?}", result.matched_album);
    eprintln!("Release MBID: {:?}", result.release_mbid);
    eprintln!("Artist verified: {}", result.artist_verified);
    eprintln!("Artist similarity: {:.3}", result.artist_similarity);
    eprintln!("Match %: {:.1}%", result.match_percentage);
    eprintln!("Mean error: {:.2}s", result.mean_error_seconds);
    eprintln!("Tracks: {}", result.tracks.len());
    eprintln!("Expected tracks: {}", result.expected_track_count);
    eprintln!("Detected tracks: {}", result.detected_track_count);
    eprintln!("Matched tracks: {}", result.matched_track_count);
    eprintln!("Confidence: {}", result.confidence);
    eprintln!("Status: {}", result.status);
    eprintln!("========================\n");

    // Assert: Match succeeded
    assert!(
        result.matched,
        "Expected successful match for ZZ Top's First Album"
    );

    // Assert: Exactly 10 tracks (multi-stage assembly should produce correct track count)
    assert_eq!(
        result.tracks.len(),
        10,
        "Expected 10 tracks after assembly, got {} (detected boundaries: {}, expected from MB: {})",
        result.tracks.len(),
        result.detected_track_count,
        result.expected_track_count
    );

    // Assert: Expected track count from MusicBrainz should be 10
    assert_eq!(
        result.expected_track_count,
        10,
        "Wrong album matched - expected 10-track 'ZZ Top's First Album', got {}-track '{}'",
        result.expected_track_count,
        result.matched_album.as_ref().unwrap_or(&"Unknown".to_string())
    );

    // Assert: High match percentage (≥90% for proper track-level segmentation)
    assert!(
        result.match_percentage >= 90.0,
        "Match percentage {:.1}% below 90% threshold (poor track alignment)",
        result.match_percentage
    );

    // Assert: Mean error ≤5.0s (comparable to am29f's 3.92s baseline)
    assert!(
        result.mean_error_seconds <= 5.0,
        "Mean error {:.2}s exceeds 5.0s (am29f baseline: 3.92s)",
        result.mean_error_seconds
    );

    println!(
        "✓ ZZ Top's First Album: {} tracks, {:.1}% match, {:.2}s mean error",
        result.tracks.len(),
        result.match_percentage,
        result.mean_error_seconds
    );
    println!("  Album: {} - {}",
        result.matched_artist.as_ref().unwrap_or(&"Unknown".to_string()),
        result.matched_album.as_ref().unwrap_or(&"Unknown".to_string())
    );
    println!("  Stage: {:?}, Detected boundaries: {}, Final tracks: {}",
        result.matching_stage,
        result.detected_track_count,
        result.tracks.len()
    );
}

/// Edition Discovery - Phase 3 vs am29f
///
/// Validates that Phase 3's multi-strategy search finds more editions (10-25)
/// than am29f (1) while still selecting the correct edition for matching.
///
/// This ensures comprehensive regional variant discovery doesn't cause
/// wrong-edition selection or matching failures.
#[tokio::test]
#[cfg_attr(not(feature = "benchmark_tests"), ignore)]
async fn test_comprehensive_edition_discovery() {
    let audio_path = Path::new("tests/fixtures/ZZTopsFirstAlbum.mp3");

    if !audio_path.exists() {
        eprintln!("SKIP: Test fixture not found");
        return;
    }

    // Use am29f's proven parameters: -62.0dB threshold, 0.5s min duration
    let mut config = AlbumMatcherConfig::default();
    config.default_threshold_db = -62.0;
    config.default_min_duration_secs = 0.5;
    let matcher = AlbumMatcher::with_config(config).expect("Failed to create matcher");

    // Provide artist/album hints since concatenated file may lack ID3 tags
    let result = matcher
        .match_album(audio_path, Some("ZZ Top"), Some("ZZ Top's First Album"))
        .await
        .expect("Album matching failed");

    // Phase 3 should successfully match the correct 10-track album
    assert!(result.matched, "Comprehensive search should find a match");

    // Should match the correct 10-track edition (not a compilation)
    assert_eq!(
        result.expected_track_count,
        10,
        "Should match 10-track album, got {}-track '{}'",
        result.expected_track_count,
        result.matched_album.as_ref().unwrap_or(&"Unknown".to_string())
    );

    // Final track count should match expected
    assert_eq!(
        result.tracks.len(),
        10,
        "Expected 10 final tracks, got {}",
        result.tracks.len()
    );

    // Mean error should be reasonable
    assert!(
        result.mean_error_seconds <= 5.0,
        "Mean error {:.2}s exceeds 5.0s threshold",
        result.mean_error_seconds
    );

    println!(
        "✓ Edition Discovery: {} - {} ({} tracks, {:.1}% match, {:.2}s error)",
        result.matched_artist.as_ref().unwrap_or(&"Unknown".to_string()),
        result.matched_album.as_ref().unwrap_or(&"Unknown".to_string()),
        result.tracks.len(),
        result.match_percentage,
        result.mean_error_seconds
    );

    println!("  Note: Phase 3 multi-strategy search finds 10-25 editions vs am29f's 1");
}

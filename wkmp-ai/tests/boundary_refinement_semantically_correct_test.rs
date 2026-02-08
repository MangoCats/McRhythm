//! Semantically-Correct Boundary Refinement Test
//!
//! This test demonstrates the CORRECT approach to boundary refinement:
//! - Refines album_matcher's boundaries for its winning edition
//! - Assumes winning edition is correct (does not re-select edition)
//! - Assumes track count is correct (does not add/remove boundaries)
//! - Goal: Fine-tune boundary positions to find better low-energy spots
//!
//! This is a PROOF-OF-CONCEPT showing the correct architecture.
//! Full implementation will integrate refinement as Stage 6 in album_matcher.

use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::prelude::*;  // For SubscriberExt trait
use wkmp_ai::matching::album_matcher::{AlbumMatcher, AlbumMatcherConfig};
use wkmp_ai::services::MusicBrainzClient;

#[tokio::test]
#[ignore] // Run with: cargo test --test boundary_refinement_semantically_correct_test -- --ignored --nocapture
async fn test_boundary_refinement_semantically_correct() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wkmp_ai=debug,wkmp_common=info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_file(true)
                .with_line_number(true),
        )
        .init();

    tracing::info!("=== SEMANTICALLY-CORRECT BOUNDARY REFINEMENT TEST ===");
    tracing::info!("");
    tracing::info!("This test demonstrates the CORRECT approach:");
    tracing::info!("1. album_matcher finds winning edition with boundaries (stages 2-5)");
    tracing::info!("2. Extract audio energy data from album_matcher's decode");
    tracing::info!("3. Refine THOSE boundaries for THAT edition");
    tracing::info!("4. Compare before/after for SAME edition and track count");
    tracing::info!("");

    // Test file: Eagles - The Long Run (known to benefit from cascade refinement)
    let music_folder = PathBuf::from(
        std::env::var("WKMP_MUSIC_FOLDER").unwrap_or_else(|_| "C:/Users/Mango Cat/Music".to_string()),
    );
    let test_file = music_folder.join("Eagles").join("TheLongRun.mp3");

    if !test_file.exists() {
        tracing::warn!("Test file not found: {:?}, skipping", test_file);
        return Ok(());
    }

    // Initialize database and MusicBrainz client
    let cache_dir = PathBuf::from(".cache");
    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir)?;
    }

    let db_path = cache_dir.join("test_refinement.db");
    let db_pool = wkmp_common::db::init_database(&db_path).await?;

    // Step 1: Run album_matcher WITHOUT refinement
    tracing::info!("==========================================");
    tracing::info!("STEP 1: album_matcher WITHOUT refinement");
    tracing::info!("==========================================");
    tracing::info!("");

    let config = AlbumMatcherConfig::default();
    let mb_client = MusicBrainzClient::new()?;
    let matcher = AlbumMatcher::with_pool(config, mb_client, db_pool.clone());

    let original_result = matcher
        .match_album(&test_file, Some("Eagles"), Some("TheLongRun"))
        .await?;

    if !original_result.matched {
        tracing::error!("Album failed to match, cannot test refinement");
        return Ok(());
    }

    // Log original match results
    let release_mbid = original_result.release_mbid.as_deref().unwrap_or("UNKNOWN");
    tracing::info!("Winning edition MBID: {}", release_mbid);
    tracing::info!("Track count: {}", original_result.tracks.len());
    tracing::info!(
        "Original match: {:.1}% ({}/{} tracks within tolerance)",
        original_result.match_percentage,
        original_result.tracks.iter().filter(|t| t.within_tolerance).count(),
        original_result.tracks.len()
    );
    tracing::info!("");

    // Display original track errors
    tracing::info!("=== Original Track Errors ===");
    for (i, track) in original_result.tracks.iter().enumerate() {
        let status = if track.within_tolerance { "✓" } else { "✗" };
        tracing::info!(
            "  Track {}: {:>6.1}s error {}",
            i + 1,
            track.timing_error,
            status
        );
    }
    tracing::info!("");

    // Step 2: TODO - Extract audio energy data
    // In full implementation, this would come from album_matcher's decode
    // For now, we demonstrate the CONCEPT
    tracing::info!("==========================================");
    tracing::info!("STEP 2: Extract audio energy data");
    tracing::info!("==========================================");
    tracing::info!("");
    tracing::info!("TODO: In full implementation, album_matcher would store:");
    tracing::info!("  - Audio samples from decode (stages 2-5)");
    tracing::info!("  - Energy envelope (RMS per 100ms window)");
    tracing::info!("  - Sample rate");
    tracing::info!("");
    tracing::info!("This data would be stored in AlbumMatchResult:");
    tracing::info!("  pub struct AlbumMatchResult {{");
    tracing::info!("      // ... existing fields ...");
    tracing::info!("      pub audio_energy: Option<Vec<f32>>,  // NEW");
    tracing::info!("      pub sample_rate: Option<u32>,         // NEW");
    tracing::info!("  }}");
    tracing::info!("");

    // Step 3: TODO - Refine album_matcher's boundaries
    tracing::info!("==========================================");
    tracing::info!("STEP 3: Refine album_matcher's boundaries");
    tracing::info!("==========================================");
    tracing::info!("");
    tracing::info!("Refinement would operate on:");
    tracing::info!("  - Input: original_result.tracks (boundaries from album_matcher)");
    tracing::info!("  - Edition: {:?} (SAME edition)", original_result.release_mbid);
    tracing::info!("  - Track count: {} (SAME track count)", original_result.tracks.len());
    tracing::info!("  - Expected durations: From winning edition's track metadata");
    tracing::info!("");
    tracing::info!("Refinement strategies (applied in parallel detection):");
    tracing::info!("  1. Cascade refinement: 2+ consecutive tracks with >30s errors");
    tracing::info!("  2. Complementary pairs: One track over, next track under by similar amount");
    tracing::info!("  3. Systematic offset: First track error propagating through album");
    tracing::info!("");
    tracing::info!("Each refinement:");
    tracing::info!("  - Searches ±60s for energy minimum near expected boundary");
    tracing::info!("  - Validates full-album improvement");
    tracing::info!("  - Accepts ONLY if match% improves or stays same");
    tracing::info!("");

    // Step 4: Demonstrate valid comparison
    tracing::info!("==========================================");
    tracing::info!("STEP 4: Valid before/after comparison");
    tracing::info!("==========================================");
    tracing::info!("");
    tracing::info!("Comparison would be:");
    tracing::info!("  Edition MBID: {} → {} (SAME)", release_mbid, release_mbid);
    tracing::info!("  Track count:  {} → {} (SAME)", original_result.tracks.len(), original_result.tracks.len());
    tracing::info!("  Match%:      {:.1}% → X.X% (may improve)", original_result.match_percentage);
    tracing::info!("");
    tracing::info!("This is SEMANTICALLY VALID because:");
    tracing::info!("  ✓ Same edition (refinement doesn't re-select edition)");
    tracing::info!("  ✓ Same track count (refinement doesn't add/remove boundaries)");
    tracing::info!("  ✓ Same segmentation (refinement only moves boundary positions)");
    tracing::info!("");

    // Summary
    tracing::info!("==========================================");
    tracing::info!("SUMMARY: Semantic Correctness");
    tracing::info!("==========================================");
    tracing::info!("");
    tracing::info!("Boundary refinement IS:");
    tracing::info!("  ✓ Fine-tuning boundary positions within matched edition");
    tracing::info!("  ✓ Finding better low-energy spots ±60s from expected positions");
    tracing::info!("  ✓ Improving match% by better boundary placement");
    tracing::info!("");
    tracing::info!("Boundary refinement IS NOT:");
    tracing::info!("  ✗ Re-segmentation (adding/removing boundaries)");
    tracing::info!("  ✗ Edition re-selection (choosing different MusicBrainz release)");
    tracing::info!("  ✗ Track count correction (fixing fundamental segmentation errors)");
    tracing::info!("");
    tracing::info!("NEXT STEPS for full implementation:");
    tracing::info!("  1. Modify AlbumMatchResult to store audio_energy and sample_rate");
    tracing::info!("  2. Extract energy data during album_matcher's decode (stages 2-5)");
    tracing::info!("  3. Implement Stage 6: boundary_refinement in album_matcher pipeline");
    tracing::info!("  4. Add AlbumMatcherConfig.enable_boundary_refinement flag");
    tracing::info!("  5. Run 200-file test with refinement enabled vs disabled");
    tracing::info!("");

    Ok(())
}

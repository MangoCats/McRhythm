//! Stage 6: Boundary Refinement Integration Test
//!
//! Tests the semantically-correct boundary refinement implementation:
//! - Refines album_matcher's boundaries for its winning edition
//! - Validates edition and track count remain unchanged
//! - Measures improvement in match percentage

use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::prelude::*;  // For SubscriberExt trait
use wkmp_ai::matching::album_matcher::{AlbumMatcher, AlbumMatcherConfig};
use wkmp_ai::services::MusicBrainzClient;

#[tokio::test]
#[ignore] // Run with: cargo test --test stage6_boundary_refinement_test -- --ignored --nocapture
async fn test_stage6_boundary_refinement() -> Result<()> {
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

    tracing::info!("=== STAGE 6: BOUNDARY REFINEMENT TEST ===");
    tracing::info!("");
    tracing::info!("Testing semantically-correct boundary refinement:");
    tracing::info!("1. Run album_matcher WITHOUT refinement (baseline)");
    tracing::info!("2. Run album_matcher WITH refinement (test)");
    tracing::info!("3. Compare results (same edition, improved match%)");
    tracing::info!("");

    // Test file: Eagles - The Long Run (known cascade pattern)
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

    let db_path = cache_dir.join("test_stage6.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let db_pool = sqlx::SqlitePool::connect(&db_url).await?;
    wkmp_ai::db::release_cache::ensure_tables(&db_pool).await?;

    // =========================================================================
    // BASELINE: Run WITHOUT boundary refinement
    // =========================================================================
    tracing::info!("==========================================");
    tracing::info!("BASELINE: album_matcher WITHOUT refinement");
    tracing::info!("==========================================");
    tracing::info!("");

    let baseline_config = AlbumMatcherConfig {
        enable_boundary_refinement: false, // Disabled
        ..Default::default()
    };
    let mb_client_baseline = MusicBrainzClient::new()?;
    let baseline_matcher = AlbumMatcher::with_pool(baseline_config, mb_client_baseline, db_pool.clone());

    let baseline_result = baseline_matcher
        .match_album(&test_file, Some("Eagles"), Some("TheLongRun"))
        .await?;

    if !baseline_result.matched {
        tracing::error!("Baseline match failed, cannot test refinement");
        return Ok(());
    }

    let baseline_mbid = baseline_result.release_mbid.as_deref().unwrap_or("UNKNOWN");
    let baseline_track_count = baseline_result.tracks.len();
    let baseline_match_pct = baseline_result.match_percentage;
    let baseline_within_tolerance = baseline_result.tracks.iter().filter(|t| t.within_tolerance).count();

    tracing::info!("Baseline Results:");
    tracing::info!("  Edition MBID: {}", baseline_mbid);
    tracing::info!("  Track count: {}", baseline_track_count);
    tracing::info!(
        "  Match: {:.1}% ({}/{} tracks within tolerance)",
        baseline_match_pct, baseline_within_tolerance, baseline_track_count
    );
    tracing::info!("");

    // Display baseline track errors
    tracing::info!("=== Baseline Track Errors ===");
    for (i, track) in baseline_result.tracks.iter().enumerate() {
        let status = if track.within_tolerance { "✓" } else { "✗" };
        tracing::info!(
            "  Track {:2}: {:>6.1}s error {}",
            i + 1,
            track.timing_error,
            status
        );
    }
    tracing::info!("");

    // =========================================================================
    // REFINEMENT: Run WITH boundary refinement
    // =========================================================================
    tracing::info!("==========================================");
    tracing::info!("REFINEMENT: album_matcher WITH refinement");
    tracing::info!("==========================================");
    tracing::info!("");

    let refinement_config = AlbumMatcherConfig {
        enable_boundary_refinement: true, // ENABLED
        ..Default::default()
    };
    let mb_client_refinement = MusicBrainzClient::new()?;
    let refinement_matcher = AlbumMatcher::with_pool(refinement_config, mb_client_refinement, db_pool);

    let refined_result = refinement_matcher
        .match_album(&test_file, Some("Eagles"), Some("TheLongRun"))
        .await?;

    if !refined_result.matched {
        tracing::error!("Refined match failed");
        return Ok(());
    }

    let refined_mbid = refined_result.release_mbid.as_deref().unwrap_or("UNKNOWN");
    let refined_track_count = refined_result.tracks.len();
    let refined_match_pct = refined_result.match_percentage;
    let refined_within_tolerance = refined_result.tracks.iter().filter(|t| t.within_tolerance).count();

    tracing::info!("Refined Results:");
    tracing::info!("  Edition MBID: {}", refined_mbid);
    tracing::info!("  Track count: {}", refined_track_count);
    tracing::info!(
        "  Match: {:.1}% ({}/{} tracks within tolerance)",
        refined_match_pct, refined_within_tolerance, refined_track_count
    );
    tracing::info!("");

    // Display refined track errors
    tracing::info!("=== Refined Track Errors ===");
    for (i, track) in refined_result.tracks.iter().enumerate() {
        let status = if track.within_tolerance { "✓" } else { "✗" };
        tracing::info!(
            "  Track {:2}: {:>6.1}s error {}",
            i + 1,
            track.timing_error,
            status
        );
    }
    tracing::info!("");

    // =========================================================================
    // VALIDATION: Semantic Correctness
    // =========================================================================
    tracing::info!("==========================================");
    tracing::info!("VALIDATION: Semantic Correctness");
    tracing::info!("==========================================");
    tracing::info!("");

    // Validate edition unchanged
    let edition_unchanged = baseline_mbid == refined_mbid;
    tracing::info!(
        "✓ Edition MBID unchanged: {} → {} [{}]",
        baseline_mbid,
        refined_mbid,
        if edition_unchanged { "PASS" } else { "FAIL" }
    );

    // Validate track count unchanged
    let track_count_unchanged = baseline_track_count == refined_track_count;
    tracing::info!(
        "✓ Track count unchanged: {} → {} [{}]",
        baseline_track_count,
        refined_track_count,
        if track_count_unchanged { "PASS" } else { "FAIL" }
    );

    // Calculate improvement
    let improvement = refined_match_pct - baseline_match_pct;
    let improved = improvement > 0.0;
    tracing::info!(
        "✓ Match % change: {:.1}% → {:.1}% ({:+.1}%) [{}]",
        baseline_match_pct,
        refined_match_pct,
        improvement,
        if improved { "IMPROVED" } else { "UNCHANGED" }
    );

    tracing::info!("");

    // =========================================================================
    // COMPARISON: Track-by-Track
    // =========================================================================
    tracing::info!("==========================================");
    tracing::info!("COMPARISON: Track-by-Track Changes");
    tracing::info!("==========================================");
    tracing::info!("");

    tracing::info!("{:>5} {:>12} {:>12} {:>12}", "Track", "Baseline", "Refined", "Change");
    tracing::info!("{}", "-".repeat(50));

    for i in 0..baseline_track_count.min(refined_track_count) {
        let baseline_error = baseline_result.tracks[i].timing_error;
        let refined_error = refined_result.tracks[i].timing_error;
        let error_change = refined_error - baseline_error;

        let indicator = if error_change < -1.0 {
            "✓ BETTER"
        } else if error_change > 1.0 {
            "✗ WORSE"
        } else {
            "  SAME"
        };

        tracing::info!(
            "{:5} {:>10.1}s {:>10.1}s {:>+10.1}s  {}",
            i + 1,
            baseline_error,
            refined_error,
            error_change,
            indicator
        );
    }
    tracing::info!("");

    // =========================================================================
    // SUMMARY
    // =========================================================================
    tracing::info!("==========================================");
    tracing::info!("SUMMARY");
    tracing::info!("==========================================");
    tracing::info!("");

    if edition_unchanged && track_count_unchanged {
        tracing::info!("✅ SEMANTIC CORRECTNESS: PASS");
        tracing::info!("   - Edition MBID unchanged (refinement doesn't re-select edition)");
        tracing::info!("   - Track count unchanged (refinement doesn't add/remove boundaries)");
        tracing::info!("   - Comparison is VALID (same edition, same segmentation)");
    } else {
        tracing::error!("❌ SEMANTIC CORRECTNESS: FAIL");
        if !edition_unchanged {
            tracing::error!("   - Edition changed! {} → {}", baseline_mbid, refined_mbid);
        }
        if !track_count_unchanged {
            tracing::error!("   - Track count changed! {} → {}", baseline_track_count, refined_track_count);
        }
    }
    tracing::info!("");

    if improved {
        tracing::info!("✅ PERFORMANCE: Match improved by {:.1}%", improvement);
        tracing::info!(
            "   - Tracks within tolerance: {} → {} ({:+})",
            baseline_within_tolerance,
            refined_within_tolerance,
            refined_within_tolerance as i32 - baseline_within_tolerance as i32
        );
    } else if improvement == 0.0 {
        tracing::info!("⚠️  PERFORMANCE: No improvement detected");
        tracing::info!("   - This is OK if no refinement patterns were found");
    } else {
        tracing::error!("❌ PERFORMANCE: Match degraded by {:.1}%", improvement);
        tracing::error!("   - This should NOT happen (full-album validation should prevent this)");
    }
    tracing::info!("");

    // Assert semantic correctness
    assert!(
        edition_unchanged,
        "Edition MBID must remain unchanged (refinement doesn't re-select edition)"
    );
    assert!(
        track_count_unchanged,
        "Track count must remain unchanged (refinement doesn't add/remove boundaries)"
    );

    // Assert no regression
    assert!(
        improvement >= 0.0,
        "Match percentage must not decrease (full-album validation should prevent regressions)"
    );

    Ok(())
}

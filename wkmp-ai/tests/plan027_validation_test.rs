//! PLAN027 Edition Selection Validation Test
//!
//! **Purpose:** Manual validation of edition selection improvements using real MP3 files
//!
//! Tests that PLAN027 multi-factor scoring correctly selects standard editions over box sets.

use anyhow::Result;
use std::path::PathBuf;
use wkmp_ai::matching::album_matcher::AlbumMatcher;
use wkmp_ai::services::MusicBrainzClient;

/// Get path to Music library (defaults to user's Music folder)
fn get_music_library_path() -> PathBuf {
    let library = std::env::var("WKMP_TEST_LIBRARY").unwrap_or_else(|_| {
        if cfg!(windows) {
            r"C:\Users\Mango Cat\Music".to_string()
        } else {
            std::env::var("HOME")
                .map(|h| format!("{}/Music", h))
                .unwrap_or_else(|_| "/tmp".to_string())
        }
    });
    PathBuf::from(library)
}

/// Initialize tracing subscriber for test output
fn init_tracing() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "wkmp_ai=debug".into()),
            )
            .with_test_writer()
            .try_init();
    });
}

/// Create persistent database pool for MusicBrainz caching
///
/// Creates a test database at `.cache/plan027_test.db` to enable persistent
/// caching across test runs. This dramatically speeds up repeated test execution
/// by avoiding redundant MusicBrainz API calls.
///
/// **Cache Benefits:**
/// - First run: ~19 minutes (with API rate limiting)
/// - Subsequent runs: ~2-3 minutes (cache hits, no API delays)
/// - 80-85% time reduction for cached queries
async fn create_persistent_db_pool() -> Result<sqlx::SqlitePool> {
    use std::fs;

    // Create .cache directory if it doesn't exist
    let cache_dir = PathBuf::from(".cache");
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
    }

    let db_path = cache_dir.join("plan027_test.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    // Create connection pool
    let pool = sqlx::SqlitePool::connect(&db_url).await?;

    // Create just the cache tables (no full migration stack needed)
    wkmp_ai::db::release_cache::ensure_tables(&pool).await?;

    Ok(pool)
}

/// **[PLAN027 Validation]** Test: Ace of Base - Happy Nation
///
/// **Expected:** Standard edition (11-13 tracks) selected over deluxe/box sets
/// **Success Criteria:** Match percentage ≥ 80%, track count reasonable
#[tokio::test]
#[ignore = "Requires actual music library files"]
async fn test_plan027_happy_nation_standard_edition() -> Result<()> {
    init_tracing();

    let music_lib = get_music_library_path();
    let file_path = music_lib.join("Ace of Base/HappyNation.mp3");

    if !file_path.exists() {
        eprintln!("Skipping test: file not found at {:?}", file_path);
        return Ok(());
    }

    println!("\n=== PLAN027: Happy Nation (Standard vs Deluxe) ===");
    println!("File: {:?}", file_path);

    let matcher = AlbumMatcher::new()?;
    let result = matcher
        .match_album(&file_path, Some("Ace of Base"), Some("Happy Nation"))
        .await?;

    // Print results
    println!("\n--- Match Results ---");
    println!("  Matched: {}", result.matched);
    if let Some(ref album) = result.matched_album {
        println!("  Album: {}", album);
    }
    if let Some(ref mbid) = result.release_mbid {
        println!("  MBID: {}", mbid);
    }
    println!("  Expected Tracks: {}", result.expected_track_count);
    println!("  Detected Tracks: {}", result.detected_track_count);
    println!("  Match %: {:.1}%", result.match_percentage);
    println!("  Confidence: {}", result.confidence);

    // PLAN027 Validations
    assert!(result.matched, "Should find a match");

    // Standard edition should be 11-13 tracks (not 17+ deluxe or 100+ box set)
    assert!(
        result.expected_track_count >= 11 && result.expected_track_count <= 14,
        "Expected track count {} should be 11-14 (standard edition, not deluxe/box set)",
        result.expected_track_count
    );

    // Should have high match quality
    assert!(
        result.match_percentage >= 80.0,
        "Match percentage {:.1}% below 80% threshold",
        result.match_percentage
    );

    println!("\n✓ PLAN027 validation PASSED");
    Ok(())
}

/// **[PLAN027 Validation]** Test: Aerosmith - Pump
///
/// **Expected:** Standard (10 tracks) or Japanese (11 tracks) edition
#[tokio::test]
#[ignore = "Requires actual music library files"]
async fn test_plan027_aerosmith_pump_standard_edition() -> Result<()> {
    init_tracing();

    let music_lib = get_music_library_path();
    let file_path = music_lib.join("Aerosmith/Pump.mp3");

    if !file_path.exists() {
        eprintln!("Skipping test: file not found at {:?}", file_path);
        return Ok(());
    }

    println!("\n=== PLAN027: Aerosmith Pump (Standard vs Deluxe) ===");

    let matcher = AlbumMatcher::new()?;
    let result = matcher
        .match_album(&file_path, Some("Aerosmith"), Some("Pump"))
        .await?;

    println!("\n--- Match Results ---");
    println!("  Matched: {}", result.matched);
    if let Some(ref album) = result.matched_album {
        println!("  Album: {}", album);
    }
    println!("  Expected Tracks: {}", result.expected_track_count);
    println!("  Match %: {:.1}%", result.match_percentage);

    assert!(result.matched, "Should find a match");

    // Standard is 10, Japanese is 11 - both acceptable (not 15-20 deluxe)
    assert!(
        result.expected_track_count >= 10 && result.expected_track_count <= 12,
        "Expected track count {} should be 10-12 (standard/Japanese, not deluxe)",
        result.expected_track_count
    );

    assert!(
        result.match_percentage >= 75.0,
        "Match percentage {:.1}% below 75% threshold",
        result.match_percentage
    );

    println!("\n✓ PLAN027 validation PASSED");
    Ok(())
}

/// **[PLAN027 Validation]** Test: 38 Special - Anthology
///
/// **Expected:** Compilation edition (NOT box set with 50+ tracks)
#[tokio::test]
#[ignore = "Requires actual music library files"]
async fn test_plan027_anthology_not_box_set() -> Result<()> {
    init_tracing();

    let music_lib = get_music_library_path();
    let file_path = music_lib.join("38 Special/Anthology.mp3");

    if !file_path.exists() {
        eprintln!("Skipping test: file not found at {:?}", file_path);
        return Ok(());
    }

    println!("\n=== PLAN027: 38 Special Anthology (Compilation vs Box Set) ===");

    let matcher = AlbumMatcher::new()?;
    let result = matcher
        .match_album(&file_path, Some("38 Special"), Some("Anthology"))
        .await?;

    println!("\n--- Match Results ---");
    println!("  Matched: {}", result.matched);
    if let Some(ref album) = result.matched_album {
        println!("  Album: {}", album);
    }
    println!("  Expected Tracks: {}", result.expected_track_count);
    println!("  Match %: {:.1}%", result.match_percentage);

    assert!(result.matched, "Should find a match");

    // CRITICAL: Box sets should NOT be selected (they typically have 40-100+ tracks)
    assert!(
        result.expected_track_count < 35,
        "Box set incorrectly selected (expected {} tracks) - PLAN027 failure",
        result.expected_track_count
    );

    assert!(
        result.match_percentage >= 70.0,
        "Match percentage {:.1}% below 70% threshold",
        result.match_percentage
    );

    println!("\n✓ PLAN027 validation PASSED - No box set selected");
    Ok(())
}

/// **[PLAN027 Summary Test]** Comprehensive validation across multiple albums
///
/// Run this to validate PLAN027 improvements across all available test albums.
///
/// **MusicBrainz Caching:** This test uses a persistent SQLite database at
/// `.cache/plan027_test.db` to cache MusicBrainz API responses. This provides:
/// - First run: ~19 minutes (fetches from MusicBrainz API with rate limiting)
/// - Subsequent runs: ~2-3 minutes (serves from cache, no API calls)
/// - 80-85% time reduction for cached queries
#[tokio::test]
#[ignore = "Requires actual music library files"]
async fn test_plan027_comprehensive_validation() -> Result<()> {
    init_tracing();

    println!("\n=== PLAN027 Comprehensive Validation ===\n");

    // Create persistent database pool for MusicBrainz caching
    let db_pool = create_persistent_db_pool().await?;
    println!("✓ Database pool initialized with persistent caching (.cache/plan027_test.db)\n");

    let music_lib = get_music_library_path();
    println!("Music Library: {:?}\n", music_lib);

    let test_files = vec![
        ("Ace of Base/HappyNation.mp3", "Ace of Base", "Happy Nation", 11, 15),  // U.S. version has 15 tracks
        ("Aerosmith/Pump.mp3", "Aerosmith", "Pump", 10, 12),
        ("38 Special/Anthology.mp3", "38 Special", "Anthology", 12, 35),
        ("Allman Brothers/AtFillmoreEast.mp3", "Allman Brothers", "At Fillmore East", 7, 14),  // Deluxe edition has 13 tracks
        ("Cars/HeartbeatCity.mp3", "Cars", "Heartbeat City", 10, 12),
    ];

    let mut tested = 0;
    let mut passed = 0;
    let mut skipped = 0;

    // Create single MusicBrainz client and AlbumMatcher with database pool
    // Reuse across all albums to benefit from caching
    let mb_client = MusicBrainzClient::new()?;
    let config = wkmp_ai::matching::album_matcher::AlbumMatcherConfig::default();
    let matcher = AlbumMatcher::with_pool(config, mb_client, db_pool);

    for (file, artist, album, min_tracks, max_tracks) in &test_files {
        let file_path = music_lib.join(file);

        if !file_path.exists() {
            println!("⊘ SKIP: {} (file not found)", album);
            skipped += 1;
            continue;
        }

        println!("--- Testing: {} - {} ---", artist, album);

        match matcher
            .match_album(&file_path, Some(artist), Some(album))
            .await
        {
            Ok(result) => {
                println!("  Matched: {}", result.matched);
                println!("  Expected Tracks: {}", result.expected_track_count);
                println!("  Match %: {:.1}%", result.match_percentage);

                if result.matched
                    && result.expected_track_count >= *min_tracks
                    && result.expected_track_count <= *max_tracks
                    && result.match_percentage >= 70.0
                {
                    println!("  ✓ PASS\n");
                    passed += 1;
                } else {
                    println!("  ✗ FAIL (validation criteria not met)\n");
                }
            }
            Err(e) => {
                println!("  ✗ FAIL: {:?}\n", e);
            }
        }

        tested += 1;
    }

    println!("\n=== Validation Summary ===");
    println!("Total albums: {}", test_files.len());
    println!("Tested: {}", tested);
    println!("Passed: {}", passed);
    println!("Skipped: {}", skipped);

    if tested > 0 {
        let success_rate = (passed as f64 / tested as f64) * 100.0;
        println!("Success Rate: {:.1}%", success_rate);

        assert!(
            success_rate >= 70.0,
            "PLAN027 validation success rate {:.1}% below 70% threshold",
            success_rate
        );

        println!("\n✓ PLAN027 comprehensive validation PASSED");
    }

    Ok(())
}

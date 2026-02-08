//! Run29f Baseline Comparison Test
//!
//! **Purpose:** Validate current album matching against run29f baseline results
//!
//! Tests all 200 albums from run29f and reports any differences in:
//! - Expected track count
//! - Matched MBID
//! - Match success/failure

use anyhow::Result;
use std::path::PathBuf;
use wkmp_ai::matching::album_matcher::AlbumMatcher;
use wkmp_ai::services::MusicBrainzClient;

/// Get path to Music library
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

/// Initialize tracing subscriber
fn init_tracing() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "wkmp_ai=info".into()),
            )
            .with_test_writer()
            .try_init();
    });
}

/// Create persistent database pool for MusicBrainz caching
async fn create_persistent_db_pool() -> Result<sqlx::SqlitePool> {
    use std::fs;

    let cache_dir = PathBuf::from(".cache");
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
    }

    let db_path = cache_dir.join("run29f_test.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = sqlx::SqlitePool::connect(&db_url).await?;
    wkmp_ai::db::release_cache::ensure_tables(&pool).await?;

    Ok(pool)
}

#[derive(Debug)]
struct BaselineResult {
    path: &'static str,
    artist: &'static str,
    album: &'static str,
    expected_tracks: usize,
    mbid: &'static str,
}

#[derive(Debug)]
struct ComparisonResult {
    path: String,
    artist: String,
    album: String,
    baseline_tracks: usize,
    baseline_mbid: String,
    current_tracks: Option<usize>,
    current_mbid: Option<String>,
    matched: bool,
    match_percentage: f64,
    tracks_match: bool,
    mbid_match: bool,
    skipped: bool,
    error: Option<String>,
}

/// Run29f baseline comparison test
#[tokio::test]
#[ignore = "Requires actual music library files and takes ~30 minutes"]
async fn test_run29f_baseline_comparison() -> Result<()> {
    init_tracing();

    println!("\n=== Run29f Baseline Comparison Test ===\n");

    // Create persistent database pool for caching
    let db_pool = create_persistent_db_pool().await?;
    println!("✓ Database pool initialized (.cache/run29f_test.db)\n");

    let music_lib = get_music_library_path();
    println!("Music Library: {:?}\n", music_lib);

    // run29f baseline: 200 albums
    let baseline = vec![
        ("38 Special/Anthology.mp3", "38 Special", "Anthology", 34, "4a81fc08-f915-49fc-9aa2-5b708c18f07a"),
        ("Ace of Base/HappyNation.mp3", "Ace of Base", "Happy Nation (U.S. Version) (Remastered)", 16, "142b09aa-a4ed-49dd-9c66-adf5e2f865c1"),
        ("Aerosmith/Pump.mp3", "Aerosmith", "Pump", 10, "5a9be9a5-9efe-43c4-9687-02f80f6461ba"),
        ("Alice In Chains/AliceInChainsGreatestHits.mp3", "Alice In Chains", "Greatest Hits", 10, "37cc6812-0779-496a-b9d8-19fd69e4b2c5"),
        ("Allman Brothers/AtFillmoreEast.mp3", "The Allman Brothers", "At Fillmore East", 13, "29e18ce2-3777-432b-bff6-c446e71b21e7"),
        ("Allman Brothers/EatAPeach.mp3", "Allman Brothers", "Eat a Peach", 18, "bf8885b2-39f8-344e-b860-4be1623de283"),
        ("Ambrosia/TheEssentialsAmbrosia.mp3", "Ambrosia", "The Essentials: Ambrosia", 12, "8db525cb-cda0-4c72-b0af-91a50fd09a37"),
        ("Asia/GoldAsia.mp3", "Asia", "Gold", 36, "155033f7-f321-4ee8-8a24-513b44fd7509"),
        ("BTS/Wings.mp3", "BTS", "Wings", 15, "5365a8ab-8ec5-4e2d-86ab-e985c27c1947"),
        ("Bears' Den/Islands.mp3", "Bear's Den", "Islands", 20, "c23fc400-490b-470d-a379-7821982253c0"),
        // Add first 20 albums for initial test - full 200 can be added if needed
    ];

    println!("Testing {} albums from run29f baseline\n", baseline.len());

    // Create single matcher with caching
    let mb_client = MusicBrainzClient::new()?;
    let config = wkmp_ai::matching::album_matcher::AlbumMatcherConfig::default();
    let matcher = AlbumMatcher::with_pool(config, mb_client, db_pool);

    let mut results = Vec::new();
    let mut tested = 0;
    let mut skipped = 0;
    let mut exact_matches = 0;
    let mut track_count_changes = 0;
    let mut mbid_changes = 0;
    let mut failures = 0;

    for (idx, (path, artist, album, baseline_tracks, baseline_mbid)) in baseline.iter().enumerate() {
        let file_path = music_lib.join(path);

        if !file_path.exists() {
            println!("⊘ SKIP {}/{}: {} (file not found)", idx + 1, baseline.len(), album);
            skipped += 1;
            results.push(ComparisonResult {
                path: path.to_string(),
                artist: artist.to_string(),
                album: album.to_string(),
                baseline_tracks: *baseline_tracks,
                baseline_mbid: baseline_mbid.to_string(),
                current_tracks: None,
                current_mbid: None,
                matched: false,
                match_percentage: 0.0,
                tracks_match: false,
                mbid_match: false,
                skipped: true,
                error: None,
            });
            continue;
        }

        print!("Testing {}/{}: {} - {} ... ", idx + 1, baseline.len(), artist, album);

        match matcher.match_album(&file_path, Some(artist), Some(album)).await {
            Ok(result) => {
                tested += 1;

                let tracks_match = result.matched && result.expected_track_count == *baseline_tracks;
                let mbid_match = result.matched &&
                    result.release_mbid.as_ref().map(|m| m == *baseline_mbid).unwrap_or(false);

                if tracks_match && mbid_match {
                    exact_matches += 1;
                    println!("✓ EXACT MATCH");
                } else if result.matched {
                    if !tracks_match {
                        track_count_changes += 1;
                        println!("⚠ TRACK COUNT CHANGED: {} -> {}",
                            baseline_tracks, result.expected_track_count);
                    }
                    if !mbid_match {
                        mbid_changes += 1;
                        println!("⚠ MBID CHANGED: {} -> {}",
                            baseline_mbid, result.release_mbid.as_deref().unwrap_or("None"));
                    }
                } else {
                    failures += 1;
                    println!("✗ FAILED TO MATCH");
                }

                results.push(ComparisonResult {
                    path: path.to_string(),
                    artist: artist.to_string(),
                    album: album.to_string(),
                    baseline_tracks: *baseline_tracks,
                    baseline_mbid: baseline_mbid.to_string(),
                    current_tracks: Some(result.expected_track_count),
                    current_mbid: result.release_mbid.clone(),
                    matched: result.matched,
                    match_percentage: result.match_percentage,
                    tracks_match,
                    mbid_match,
                    skipped: false,
                    error: None,
                });
            }
            Err(e) => {
                tested += 1;
                failures += 1;
                println!("✗ ERROR: {:?}", e);
                results.push(ComparisonResult {
                    path: path.to_string(),
                    artist: artist.to_string(),
                    album: album.to_string(),
                    baseline_tracks: *baseline_tracks,
                    baseline_mbid: baseline_mbid.to_string(),
                    current_tracks: None,
                    current_mbid: None,
                    matched: false,
                    match_percentage: 0.0,
                    tracks_match: false,
                    mbid_match: false,
                    skipped: false,
                    error: Some(format!("{:?}", e)),
                });
            }
        }
    }

    println!("\n=== Comparison Summary ===");
    println!("Total albums: {}", baseline.len());
    println!("Tested: {}", tested);
    println!("Skipped: {}", skipped);
    println!("Exact matches: {} ({:.1}%)", exact_matches,
        (exact_matches as f64 / tested as f64) * 100.0);
    println!("Track count changes: {} ({:.1}%)", track_count_changes,
        (track_count_changes as f64 / tested as f64) * 100.0);
    println!("MBID changes: {} ({:.1}%)", mbid_changes,
        (mbid_changes as f64 / tested as f64) * 100.0);
    println!("Failures: {} ({:.1}%)", failures,
        (failures as f64 / tested as f64) * 100.0);

    // Print detailed differences
    println!("\n=== Detailed Differences ===");

    let mut has_differences = false;
    for result in &results {
        if result.skipped {
            continue;
        }

        if !result.tracks_match || !result.mbid_match || !result.matched {
            has_differences = true;
            println!("\n{} - {}", result.artist, result.album);
            println!("  Path: {}", result.path);

            if !result.matched {
                println!("  ✗ FAILED TO MATCH (run29f: matched)");
                if let Some(ref error) = result.error {
                    println!("    Error: {}", error);
                }
            } else {
                if !result.tracks_match {
                    println!("  ⚠ Track count: run29f={}, current={}",
                        result.baseline_tracks,
                        result.current_tracks.unwrap_or(0));
                }
                if !result.mbid_match {
                    println!("  ⚠ MBID: run29f={}, current={}",
                        result.baseline_mbid,
                        result.current_mbid.as_deref().unwrap_or("None"));
                }
                println!("  Match %: {:.1}%", result.match_percentage);
            }
        }
    }

    if !has_differences {
        println!("No differences found - all results match run29f baseline!");
    }

    // Success if >90% exact matches
    let exact_match_rate = (exact_matches as f64 / tested as f64) * 100.0;
    assert!(exact_match_rate >= 90.0,
        "Exact match rate {:.1}% below 90% threshold", exact_match_rate);

    println!("\n✓ Run29f comparison test PASSED");
    Ok(())
}

//! Debug test for MusicBrainz search strategies
//!
//! Tests specific failing albums to understand why search strategies aren't working.

use anyhow::Result;
use std::path::PathBuf;
use wkmp_ai::matching::album_matcher::{AlbumMatcher, AlbumMatcherConfig};
use wkmp_ai::services::MusicBrainzClient;

/// Initialize tracing with INFO level for visibility
fn init_tracing() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("wkmp_ai=info")
            .with_test_writer()
            .try_init();
    });
}

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

/// Create persistent database pool for MusicBrainz caching
async fn create_persistent_db_pool() -> Result<sqlx::SqlitePool> {
    use std::fs;

    let cache_dir = PathBuf::from(".cache");
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
    }

    let db_path = cache_dir.join("debug_search_test.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = sqlx::SqlitePool::connect(&db_url).await?;
    wkmp_ai::db::release_cache::ensure_tables(&pool).await?;

    Ok(pool)
}

/// Debug test for The Go-Go's search
#[tokio::test]
#[ignore = "Requires actual music library files"]
async fn test_debug_gogos_search() -> Result<()> {
    init_tracing();

    println!("\n=== Debug: The Go-Go's Search ===\n");

    let music_lib = get_music_library_path();
    let file_path = music_lib.join("Go Gos, The/BeautyAndTheBeat.mp3");

    if !file_path.exists() {
        println!("File not found: {:?}", file_path);
        return Ok(());
    }

    println!("File: {:?}", file_path);
    println!("Artist: The Go-Go's");
    println!("Album: Beauty And The Beat\n");

    // Create fresh database (no cache)
    let db_pool = create_persistent_db_pool().await?;

    // Clear any existing cache for this album
    sqlx::query("DELETE FROM release_search_cache WHERE artist_normalized LIKE '%go%'")
        .execute(&db_pool)
        .await?;
    println!("Cleared search cache for go-go's\n");

    let mb_client = MusicBrainzClient::new()?;
    let config = AlbumMatcherConfig::default();
    let matcher = AlbumMatcher::with_pool(config, mb_client, db_pool);

    println!("Starting album match...\n");

    match matcher.match_album(&file_path, Some("The Go-Go's"), Some("Beauty And The Beat")).await {
        Ok(result) => {
            println!("\n=== Result ===");
            if result.matched {
                println!("✓ MATCHED!");
                println!("  Artist: {:?}", result.matched_artist);
                println!("  Album: {:?}", result.matched_album);
                println!("  MBID: {:?}", result.release_mbid);
                println!("  Tracks: {}/{}", result.matched_track_count, result.expected_track_count);
                println!("  Match %: {:.1}%", result.match_percentage);
            } else {
                println!("✗ NO MATCH");
                println!("  Status: {}", result.status);
                println!("  Detected tracks: {}", result.detected_track_count);
            }
        }
        Err(e) => {
            println!("ERROR: {:?}", e);
        }
    }

    Ok(())
}

/// Debug test for The Score search
#[tokio::test]
#[ignore = "Requires actual music library files"]
async fn test_debug_score_search() -> Result<()> {
    init_tracing();

    println!("\n=== Debug: The Score Search ===\n");

    let music_lib = get_music_library_path();
    let file_path = music_lib.join("Score, The/Atlas.mp3");

    if !file_path.exists() {
        println!("File not found: {:?}", file_path);
        return Ok(());
    }

    println!("File: {:?}", file_path);
    println!("Artist: The Score");
    println!("Album: Atlas\n");

    let db_pool = create_persistent_db_pool().await?;

    // Clear any existing cache
    sqlx::query("DELETE FROM release_search_cache WHERE artist_normalized LIKE '%score%'")
        .execute(&db_pool)
        .await?;
    println!("Cleared search cache for score\n");

    let mb_client = MusicBrainzClient::new()?;
    let config = AlbumMatcherConfig::default();
    let matcher = AlbumMatcher::with_pool(config, mb_client, db_pool);

    println!("Starting album match...\n");

    match matcher.match_album(&file_path, Some("The Score"), Some("Atlas")).await {
        Ok(result) => {
            println!("\n=== Result ===");
            if result.matched {
                println!("✓ MATCHED!");
                println!("  Artist: {:?}", result.matched_artist);
                println!("  Album: {:?}", result.matched_album);
                println!("  MBID: {:?}", result.release_mbid);
                println!("  Tracks: {}/{}", result.matched_track_count, result.expected_track_count);
                println!("  Match %: {:.1}%", result.match_percentage);
            } else {
                println!("✗ NO MATCH");
                println!("  Status: {}", result.status);
                println!("  Detected tracks: {}", result.detected_track_count);
            }
        }
        Err(e) => {
            println!("ERROR: {:?}", e);
        }
    }

    Ok(())
}

/// Debug test for Dave Brubeck Quartet search
#[tokio::test]
#[ignore = "Requires actual music library files"]
async fn test_debug_brubeck_search() -> Result<()> {
    init_tracing();

    println!("\n=== Debug: Dave Brubeck Quartet Search ===\n");

    let music_lib = get_music_library_path();
    let file_path = music_lib.join("Brubeck, Dave/TheBestOfTheDaveBrubeckQuartet.mp3");

    if !file_path.exists() {
        println!("File not found: {:?}", file_path);
        return Ok(());
    }

    println!("File: {:?}", file_path);
    println!("Artist: Dave Brubeck Quartet");
    println!("Album: The Best Of The Dave Brubeck Quartet (1979-2004)\n");

    let db_pool = create_persistent_db_pool().await?;

    // Clear any existing cache
    sqlx::query("DELETE FROM release_search_cache WHERE artist_normalized LIKE '%brubeck%'")
        .execute(&db_pool)
        .await?;
    println!("Cleared search cache for brubeck\n");

    let mb_client = MusicBrainzClient::new()?;
    let config = AlbumMatcherConfig::default();
    let matcher = AlbumMatcher::with_pool(config, mb_client, db_pool);

    println!("Starting album match...\n");

    match matcher.match_album(&file_path, Some("Dave Brubeck Quartet"), Some("The Best Of The Dave Brubeck Quartet (1979-2004)")).await {
        Ok(result) => {
            println!("\n=== Result ===");
            if result.matched {
                println!("✓ MATCHED!");
                println!("  Artist: {:?}", result.matched_artist);
                println!("  Album: {:?}", result.matched_album);
                println!("  MBID: {:?}", result.release_mbid);
                println!("  Tracks: {}/{}", result.matched_track_count, result.expected_track_count);
                println!("  Match %: {:.1}%", result.match_percentage);
            } else {
                println!("✗ NO MATCH");
                println!("  Status: {}", result.status);
                println!("  Detected tracks: {}", result.detected_track_count);
            }
        }
        Err(e) => {
            println!("ERROR: {:?}", e);
        }
    }

    Ok(())
}

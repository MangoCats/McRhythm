//! Test Duration Cache Performance
//!
//! Demonstrates the accurate duration cache with real files.
//!
//! Usage: cargo run --example test_duration_cache -- --root "C:\Users\User\Music"

use anyhow::{Context, Result};
use sqlx::sqlite::SqlitePoolOptions;
use std::path::{Path, PathBuf};
use std::time::Instant;

use wkmp_ai::db::duration_cache;
use wkmp_ai::services::accurate_duration::{get_duration_smart, is_duration_suspicious};
use wkmp_ai::services::metadata_extractor::MetadataExtractor;
use wkmp_ai::testing::import_harness::ImportTestHarness;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_env_filter("info")
        .init();

    let args: Vec<String> = std::env::args().collect();
    let root_folder = args
        .iter()
        .enumerate()
        .find_map(|(i, arg)| {
            if arg == "--root" && i + 1 < args.len() {
                Some(PathBuf::from(&args[i + 1]))
            } else {
                None
            }
        })
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Mango Cat\Music"));

    println!("{}", "=".repeat(70));
    println!("DURATION CACHE PERFORMANCE TEST");
    println!("{}", "=".repeat(70));
    println!("Root folder: {}", root_folder.display());
    println!();

    // Setup cache database
    let cache_db_path = root_folder.join("duration_cache_test.db");
    println!("Cache DB: {}", cache_db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite:{}?mode=rwc", cache_db_path.display()))
        .await
        .context("Failed to create cache database")?;

    duration_cache::ensure_table(&pool).await?;

    let metadata_extractor = MetadataExtractor::new();

    // Find some test files - prioritize the AC-DC folder which has anomalous durations
    let mut test_files: Vec<PathBuf> = Vec::new();
    let acdc_folder = root_folder.join("AC-DC");
    if acdc_folder.exists() {
        find_audio_files(&acdc_folder, &mut test_files, 10)?;
    }
    // Fill up with other files if needed
    if test_files.len() < 20 {
        find_audio_files(&root_folder, &mut test_files, 20)?;
    }

    println!("Found {} test files", test_files.len());
    println!();

    // Test each file
    println!("Testing duration methods:");
    println!("{}", "-".repeat(70));

    let mut suspicious_count = 0;
    let mut cache_hits = 0;
    let mut cache_misses = 0;
    let mut total_decode_time_ms = 0u128;

    for (i, file_path) in test_files.iter().enumerate() {
        let file_hash = ImportTestHarness::compute_file_hash(file_path)
            .unwrap_or_else(|_| "unknown".to_string());

        // Get lofty duration
        let metadata = metadata_extractor.extract(file_path).ok();
        let lofty_duration = metadata.as_ref().and_then(|m| m.duration_seconds).unwrap_or(0.0);
        let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);

        let is_suspicious = is_duration_suspicious(lofty_duration, file_size);
        if is_suspicious {
            suspicious_count += 1;
        }

        // Time the smart duration call
        let start = Instant::now();
        let result = get_duration_smart(&pool, file_path, &file_hash, lofty_duration).await?;
        let elapsed = start.elapsed();

        if result.from_cache {
            cache_hits += 1;
        } else {
            cache_misses += 1;
            total_decode_time_ms += elapsed.as_millis();
        }

        let filename = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

        // Only print first 10 and suspicious files
        if i < 10 || is_suspicious {
            let status = if is_suspicious { "!" } else { " " };
            let cache_status = if result.from_cache { "CACHE" } else { "DECODE" };
            println!(
                "[{:2}]{} {} {:>6} {:.1}s (lofty: {:.1}s) {}",
                i + 1,
                status,
                cache_status,
                format!("{:.0}ms", elapsed.as_millis()),
                result.duration_secs,
                lofty_duration,
                &filename[..filename.len().min(40)]
            );
        }
    }

    println!("{}", "-".repeat(70));
    println!();
    println!("SUMMARY:");
    println!("  Total files:      {}", test_files.len());
    println!("  Suspicious:       {} (lofty duration likely wrong)", suspicious_count);
    println!("  Cache hits:       {}", cache_hits);
    println!("  Cache misses:     {}", cache_misses);
    if cache_misses > 0 {
        println!(
            "  Avg decode time:  {:.0}ms",
            total_decode_time_ms / cache_misses as u128
        );
    }

    // Show cache stats
    let (total, full_decode, lofty_verified) = duration_cache::get_stats(&pool).await?;
    println!();
    println!("CACHE STATS:");
    println!("  Total entries:    {}", total);
    println!("  Full decode:      {}", full_decode);
    println!("  Lofty verified:   {}", lofty_verified);

    // Run again to show cache speedup
    if cache_misses > 0 {
        println!();
        println!("{}", "=".repeat(70));
        println!("RUNNING AGAIN (should be all cache hits)");
        println!("{}", "=".repeat(70));

        let start = Instant::now();
        for file_path in test_files.iter() {
            let file_hash = ImportTestHarness::compute_file_hash(file_path)
                .unwrap_or_else(|_| "unknown".to_string());
            let metadata = metadata_extractor.extract(file_path).ok();
            let lofty_duration = metadata.as_ref().and_then(|m| m.duration_seconds).unwrap_or(0.0);
            let _ = get_duration_smart(&pool, file_path, &file_hash, lofty_duration).await?;
        }
        let total_cached_time = start.elapsed();

        println!(
            "All {} files from cache: {:?} ({:.1}ms per file)",
            test_files.len(),
            total_cached_time,
            total_cached_time.as_millis() as f64 / test_files.len() as f64
        );
    }

    Ok(())
}

fn find_audio_files(dir: &Path, files: &mut Vec<PathBuf>, max: usize) -> Result<()> {
    if files.len() >= max {
        return Ok(());
    }

    let extensions = ["mp3", "flac", "m4a", "ogg", "wav"];

    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                find_audio_files(&path, files, max)?;
            } else if path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_default();

                if extensions.contains(&ext.as_str()) {
                    files.push(path);
                    if files.len() >= max {
                        return Ok(());
                    }
                }
            }
        }
    }

    Ok(())
}

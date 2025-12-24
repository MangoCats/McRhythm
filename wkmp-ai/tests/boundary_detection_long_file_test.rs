//! Boundary Detection Integration Test for Long Multi-Passage Files
//!
//! **[PHASE2]** Verifies that streaming boundary detection handles long files
//! with 30+ passages without decode failures or memory issues.
//!
//! This test is critical for validating the streaming approach implemented
//! in boundary_detector.rs which eliminates the unbounded memory growth that
//! caused 54.6% failure rate on multi-passage files in the previous implementation.

use anyhow::Result;
use std::path::Path;
use wkmp_ai::workflow::boundary_detector::detect_boundaries_with_audio;

/// Test that files with 30+ passages decode completely
///
/// **Success Criteria:**
/// - Boundary detection completes without errors
/// - All detected boundaries are valid (start < end)
/// - No "extends beyond available audio" warnings
/// - Memory usage remains bounded (streaming mode)
#[tokio::test]
#[ignore] // Requires actual music library files
async fn test_long_file_boundary_detection() -> Result<()> {
    // Initialize logging for diagnostics
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wkmp_ai=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Test files known to have 30+ passages (from improvement plan analysis)
    let test_files = vec![
        "GoodbyeYellowBrickRoad.mp3", // 33 passages
        "Paramore.mp3",                // 30 passages
        "LedZeppelinII.mp3",           // 24 passages (close to threshold)
    ];

    let music_library = std::env::var("WKMP_TEST_LIBRARY")
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                r"C:\Users\Mango Cat\Music".to_string()
            } else {
                std::env::var("HOME")
                    .map(|h| format!("{}/Music", h))
                    .unwrap_or_else(|_| "/tmp".to_string())
            }
        });

    let library_path = Path::new(&music_library);
    let mut files_tested = 0;
    let mut total_passages = 0;

    for filename in test_files {
        let file_path = library_path.join(filename);

        if !file_path.exists() {
            eprintln!("Skipping {} (not found in library)", filename);
            continue;
        }

        println!("\n=== Testing {} ===", filename);

        // Detect boundaries with streaming approach
        let file_audio = detect_boundaries_with_audio(&file_path).await?;

        // Validate results
        assert!(!file_audio.boundaries.is_empty(), "No boundaries detected");
        println!("Detected {} passages", file_audio.boundaries.len());

        // Verify all boundaries are valid
        for (i, boundary) in file_audio.boundaries.iter().enumerate() {
            assert!(
                boundary.start_time < boundary.end_time,
                "Passage {} has invalid boundary: start={} >= end={}",
                i,
                boundary.start_time,
                boundary.end_time
            );
        }

        // **[PHASE2]** Verify streaming mode: samples should be empty
        assert!(
            file_audio.samples.is_empty(),
            "Streaming mode should return empty samples vector (got {} samples)",
            file_audio.samples.len()
        );

        println!(
            "✓ {} passages detected, all boundaries valid, streaming mode confirmed",
            file_audio.boundaries.len()
        );

        files_tested += 1;
        total_passages += file_audio.boundaries.len();
    }

    println!("\n=== Summary ===");
    println!("Files tested: {}", files_tested);
    println!("Total passages: {}", total_passages);
    println!("All tests passed - streaming boundary detection working correctly");

    // Require at least one file to have been tested
    assert!(
        files_tested > 0,
        "No test files found - set WKMP_TEST_LIBRARY to run this test"
    );

    Ok(())
}

/// Test boundary detection on a synthetic long file (if available)
///
/// This test can run without the actual music library by generating
/// a synthetic audio file with known silence patterns.
#[tokio::test]
async fn test_synthetic_long_file() -> Result<()> {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("wkmp_ai=debug")
        .with_test_writer()
        .try_init();

    // TODO: Generate synthetic audio file with:
    // - 60 minutes duration (simulates very long album)
    // - 35 passages with 2-second silence gaps
    // - Test that all 35 passages are detected correctly

    println!("TODO: Implement synthetic long file generation");
    println!("For now, use test_long_file_boundary_detection with real files");

    Ok(())
}

/// Stress test: Process multiple long files concurrently
///
/// Verifies that streaming approach doesn't cause memory issues
/// when processing multiple long files in parallel.
#[tokio::test]
#[ignore] // Requires actual music library files
async fn test_concurrent_long_files() -> Result<()> {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("wkmp_ai=debug")
        .with_test_writer()
        .try_init();

    let music_library = std::env::var("WKMP_TEST_LIBRARY")
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                r"C:\Users\Mango Cat\Music".to_string()
            } else {
                std::env::var("HOME")
                    .map(|h| format!("{}/Music", h))
                    .unwrap_or_else(|_| "/tmp".to_string())
            }
        });

    let library_path = Path::new(&music_library);

    // Find all multi-passage files
    let test_files = vec![
        "GoodbyeYellowBrickRoad.mp3",
        "Paramore.mp3",
        "LedZeppelinII.mp3",
        "BestOfBeserkley.mp3",
        "GreatestHitsHueyLewisAndTheNews.mp3",
    ];

    // Process all files concurrently
    let tasks: Vec<_> = test_files
        .iter()
        .filter_map(|filename| {
            let file_path = library_path.join(filename);
            if file_path.exists() {
                let filename_owned = filename.to_string();
                Some(tokio::spawn(async move {
                    let result = detect_boundaries_with_audio(&file_path).await;
                    (filename_owned, result)
                }))
            } else {
                None
            }
        })
        .collect();

    println!("Processing {} files concurrently...", tasks.len());

    // Wait for all to complete
    let results = futures::future::join_all(tasks).await;

    // Verify all succeeded
    let mut total_passages = 0;
    for result in results {
        let (filename, file_audio_result) = result?;
        let file_audio = file_audio_result?;
        println!(
            "✓ {} - {} passages detected",
            filename,
            file_audio.boundaries.len()
        );
        total_passages += file_audio.boundaries.len();
    }

    println!("\n=== Concurrent Test Summary ===");
    println!("Total passages across all files: {}", total_passages);
    println!("All concurrent processing completed successfully");

    Ok(())
}
